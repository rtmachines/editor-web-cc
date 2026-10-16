//! Tests of `GetVersion` (design D4, ADR-0042, ADR-0043 (m)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::VersionState;
use time::Duration;

use super::outdated::OutdatedParameter;
use super::test_world::seed_users;
use super::versions::{GetVersion, GetVersionInput, LastChange, VersionView};
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{
    Fixture, actor, meta, pkey, recipe_with, reference, rid, scenarios, second_engineer, tid, user,
    vid,
};
use crate::ports::Isolation;
use crate::recipe::AddParameter;

async fn get(
    fixture: &Fixture,
    role: Role,
    (name, number): (&str, u32),
) -> Result<VersionView, AppError> {
    GetVersion::new(fixture.context())
        .execute(
            &actor(role),
            &meta(),
            GetVersionInput {
                reference: reference(name),
                number,
                rebind_bound: 16_384,
            },
        )
        .await
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(GetVersion::OPERATION, Operation::GetVersion);
}

#[tokio::test]
async fn get_version_returns_the_header_the_logins_and_the_machine() {
    let fixture = scenarios::approved_and_draft();
    seed_users(&fixture);

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    let detail = &view.detail;
    assert_eq!(detail.id, vid(101));
    assert_eq!(detail.recipe, rid(1));
    assert_eq!((detail.number, detail.state), (1, VersionState::Approved));
    assert_eq!(detail.created_by.as_str(), "jkowalski");
    assert_eq!(
        detail.approved_by.as_ref().map(Login::as_str),
        Some("jkowalski")
    );
    assert_eq!(detail.row_version, 1);
    assert_eq!(view.recipe.reference().to_string(), "R-1");
    assert_eq!(view.machine.platform(), PlcPlatform::TwinCat3);
    assert_eq!(view.last_change, None);
    assert!(view.outdated.is_empty());
}

#[tokio::test]
async fn get_version_reads_in_one_repeatable_read_transaction_and_writes_nothing() {
    let fixture = scenarios::draft();
    seed_users(&fixture);

    get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn last_change_names_the_actor_of_the_last_draft_command_and_its_time() {
    let fixture = scenarios::draft();
    seed_users(&fixture);
    fixture.clock.advance(Duration::seconds(60));
    AddParameter::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            scenarios::add_parameter_input(),
        )
        .await
        .unwrap();
    fixture.clock.advance(Duration::seconds(60));

    let view = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap();

    assert_eq!(view.detail.row_version, 2);
    assert_eq!(
        view.last_change,
        Some(LastChange {
            login: Login::parse("anowak").unwrap(),
            at: view.detail.updated_at,
        })
    );
}

#[tokio::test]
async fn a_draft_with_an_old_struct_revision_lists_the_outdated_parameter() {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);

    let view = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(view.outdated.len(), 1);
    let found: &OutdatedParameter = view.outdated.first().unwrap();
    assert_eq!(found.key, pkey(11));
    assert_eq!(found.tag.to_string(), "ZONE1");
    assert_eq!(found.unit_path.to_string(), "U1");
    assert_eq!(found.type_name.to_string(), "ST_Zone");
    assert_eq!((found.current, found.newest), (1, 2));
    assert_eq!((found.from, found.to), (tid(901), tid(902)));
}

#[tokio::test]
async fn a_version_that_is_not_a_live_draft_lists_no_outdated_parameter() {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);
    let approved = recipe_with((1, 1), "R-1", &[(101, VersionState::Approved)]);
    fixture
        .store
        .seed_recipe(&approved, PlcPlatform::TwinCat3, user(2));

    let view = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(view.detail.state, VersionState::Approved);
    assert!(view.outdated.is_empty());
}

#[tokio::test]
async fn an_unknown_recipe_or_version_is_not_found() {
    let fixture = scenarios::draft();
    seed_users(&fixture);

    let recipe = get(&fixture, Role::Operator, ("R-9", 1)).await.unwrap_err();
    let version = get(&fixture, Role::Operator, ("R-1", 7)).await.unwrap_err();

    assert!(matches!(
        recipe,
        AppError::NotFound {
            entity: Missing::Recipe(_)
        }
    ));
    assert!(matches!(
        version,
        AppError::NotFound {
            entity: Missing::Version { number: 7, .. }
        }
    ));
}

#[tokio::test]
async fn a_version_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);

    let operator = get(&fixture, Role::Operator, ("R-1", 1)).await.unwrap_err();
    let engineer = get(&fixture, Role::ProcessEngineer, ("R-1", 1))
        .await
        .unwrap_err();
    let administrator = get(&fixture, Role::Administrator, ("R-1", 1))
        .await
        .unwrap();

    assert_eq!(operator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert_eq!(administrator.detail.state, VersionState::Discarded);
    assert!(administrator.recipe.is_deleted());
}
