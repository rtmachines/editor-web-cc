//! Contract cases of `lock_draft`, `read_draft`, and `touch_draft`.

use recipe_domain::plant::{PlcPlatform, PlcTypeId, StringEncoding};
use recipe_domain::recipe::{ReferenceNumber, VersionState};
use uuid::Uuid;

use super::fixtures::{commit_draft, full_parameter, live_draft, read_back};
use crate::contract::harness::{Harness, begin};
use crate::contract::images::{field, text};
use crate::contract::machine::codesys_machine;
use crate::contract::postgres_harness::PgHarness;
use crate::contract::recipe_seed::{
    Plant, approve, create_recipe, describe, parameter, seed, version_id,
};
use crate::contract::seed::insert;
use crate::support::Role;

async fn lock_draft_reads_the_header_the_editors_and_the_parameters(h: &dyn Harness) {
    let d = live_draft(h, 1).await;
    let beta = full_parameter(&d.plant, 1, "BETA");
    let alpha = parameter(&d.plant, 2, "ALPHA");
    seed(h, d.version, &beta).await;
    seed(h, d.version, &alpha).await;
    let mut tx = begin(h).await;

    let draft = tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    assert_eq!(draft.reference, ReferenceNumber::parse("REF-1").unwrap());
    assert_eq!(draft.machine, d.plant.machine);
    assert_eq!(draft.platform, PlcPlatform::TwinCat3);
    assert_eq!(draft.version, d.version);
    assert_eq!(draft.number, 1);
    assert_eq!(draft.state, VersionState::LiveDraft);
    assert_eq!(draft.row_version, 1);
    assert_eq!(draft.created_by, h.user());
    assert!(draft.editors.is_empty());
    assert_eq!(describe(&draft.parameters), describe(&[alpha, beta]));
}

async fn lock_draft_gives_a_version_in_any_state(h: &dyn Harness) {
    let d = live_draft(h, 2).await;
    approve(h, d.version).await;
    let mut tx = begin(h).await;

    let draft = tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    assert_eq!(draft.state, VersionState::Approved);
    assert_eq!(draft.row_version, 2);
}

async fn a_missing_version_gives_none_from_lock_draft_and_read_draft(h: &dyn Harness) {
    let mut tx = begin(h).await;

    let locked = tx.drafts().lock_draft(version_id(98)).await.unwrap();
    let read = tx.drafts().read_draft(version_id(98)).await.unwrap();

    assert!(locked.is_none());
    assert!(read.is_none());
}

async fn read_draft_gives_the_content_of_lock_draft(h: &dyn Harness) {
    let d = live_draft(h, 3).await;
    seed(h, d.version, &full_parameter(&d.plant, 1, "FULL")).await;
    let mut tx = begin(h).await;

    let locked = tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    let read = tx.drafts().read_draft(d.version).await.unwrap().unwrap();

    assert_eq!(format!("{locked:?}"), format!("{read:?}"));
}

async fn a_touch_adds_one_to_the_row_version_and_the_editor_once(h: &dyn Harness) {
    let d = live_draft(h, 4).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();

    let first = tx.drafts().touch_draft(d.version, h.user()).await.unwrap();
    let second = tx.drafts().touch_draft(d.version, h.user()).await.unwrap();
    commit_draft(h, &d, tx).await;

    assert_eq!(first, 2);
    assert_eq!(second, 3);
    let draft = read_back(h, d.version).await;
    assert_eq!(draft.row_version, 3);
    assert_eq!(draft.editors, vec![h.user()]);
}

async fn a_status_write_after_a_lock_and_a_touch_passes_the_guard(h: &dyn Harness) {
    let d = live_draft(h, 5).await;
    let mut tx = begin(h).await;
    tx.drafts().lock_draft(d.version).await.unwrap().unwrap();
    tx.drafts().touch_draft(d.version, h.user()).await.unwrap();

    let image = tx.recipes().discard_version(d.version).await.unwrap();
    commit_draft(h, &d, tx).await;

    assert_eq!(field(&image, "row_version").as_i64(), Some(3));
    assert!(!field(&image, "deleted_at").is_null());
    assert_eq!(text(&image, "status"), "draft");
}

contract_case!(
    lock_draft_reads_the_header_the_editors_and_the_parameters,
    lock_draft_reads_the_header_the_editors_and_the_parameters_memory,
    lock_draft_reads_the_header_the_editors_and_the_parameters_postgres
);
contract_case!(
    lock_draft_gives_a_version_in_any_state,
    lock_draft_gives_a_version_in_any_state_memory,
    lock_draft_gives_a_version_in_any_state_postgres
);
contract_case!(
    a_missing_version_gives_none_from_lock_draft_and_read_draft,
    a_missing_version_gives_none_from_lock_draft_and_read_draft_memory,
    a_missing_version_gives_none_from_lock_draft_and_read_draft_postgres
);
contract_case!(
    read_draft_gives_the_content_of_lock_draft,
    read_draft_gives_the_content_of_lock_draft_memory,
    read_draft_gives_the_content_of_lock_draft_postgres
);
contract_case!(
    a_touch_adds_one_to_the_row_version_and_the_editor_once,
    a_touch_adds_one_to_the_row_version_and_the_editor_once_memory,
    a_touch_adds_one_to_the_row_version_and_the_editor_once_postgres
);
// MemoryStore keeps no guard record, so this case proves the guard only on `Database`. On
// `MemoryStore` it proves that the sequence lock, touch, and status write works.
contract_case!(
    a_status_write_after_a_lock_and_a_touch_passes_the_guard,
    a_status_write_after_a_lock_and_a_touch_passes_the_guard_memory,
    a_status_write_after_a_lock_and_a_touch_passes_the_guard_postgres
);

/// Postgres only: the read path and the lock path give the encoding of the machine (ADR-0202).
///
/// `Machine::create` stores `Latin1` until task 11.11, so the case sets `Utf8` with SQL as
/// `recipe_app`, which holds the column grant.
#[tokio::test]
async fn adr_0202_stored_draft_reads_the_string_encoding() {
    let h = PgHarness::create().await;
    let latin1 = codesys_machine(70, "M70");
    let utf8 = codesys_machine(71, "M71");
    insert(&h, &latin1).await;
    insert(&h, &utf8).await;
    let mut app = h.database.connect(Role::App).await;
    sqlx::query("UPDATE machine SET string_encoding = 'Utf8' WHERE id = $1")
        .bind(*utf8.id().as_uuid())
        .execute(&mut app)
        .await
        .unwrap();
    let latin1_recipe = create_recipe(&h, &plant_of(&latin1), 70, "REF-70").await;
    let utf8_recipe = create_recipe(&h, &plant_of(&utf8), 71, "REF-71").await;
    let latin1_version = latin1_recipe.versions()[0].id();
    let utf8_version = utf8_recipe.versions()[0].id();
    let mut tx = begin(&h).await;

    let read_utf8 = tx.drafts().read_draft(utf8_version).await.unwrap().unwrap();
    let read_latin1 = tx
        .drafts()
        .read_draft(latin1_version)
        .await
        .unwrap()
        .unwrap();
    let locked_utf8 = tx.drafts().lock_draft(utf8_version).await.unwrap().unwrap();
    let locked_latin1 = tx
        .drafts()
        .lock_draft(latin1_version)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(read_utf8.string_encoding, StringEncoding::Utf8);
    assert_eq!(read_latin1.string_encoding, StringEncoding::Latin1);
    assert_eq!(locked_utf8.string_encoding, StringEncoding::Utf8);
    assert_eq!(locked_latin1.string_encoding, StringEncoding::Latin1);
}

/// The plant of a machine with no type. The case reads no type.
fn plant_of(machine: &recipe_domain::plant::Machine) -> Plant {
    Plant {
        machine: machine.id(),
        unit: machine.units().next().unwrap().id(),
        dint: PlcTypeId::from(Uuid::nil()),
    }
}
