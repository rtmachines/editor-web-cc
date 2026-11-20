//! Tests of the query of the runbook section "Roll back on the newer schema" (ADR-0202 (q3)).
//!
//! The reader takes the first SQL block after the heading from `docs/ops/runbook.md`. Each case
//! runs that query as the superuser, as the runbook does with `-u postgres`, in its own migrated
//! database.

use recipe_app::auth::CurrentUser;
use recipe_app::plant::{SetMachineStringEncoding, SetMachineStringEncodingInput};
use recipe_app::recipe::{ApproveVersion, ApproveVersionInput, CreateRecipe, CreateRecipeInput};
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::StringEncoding;
use recipe_domain::recipe::ReferenceNumber;

use crate::atomic::world::{long_text, text};
use crate::stored_rules::{
    Case, ENGINEER, add_case_parameter, length_limit, new_meta, row_version,
};
use crate::support::app::App;
use crate::support::identity::PASSWORD;

/// The runbook, from the root of the crate.
const RUNBOOK: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/ops/runbook.md");
/// The heading of the section with the query.
const HEADING: &str = "#### Roll back on the newer schema";
/// The text that ends the shell line before the SQL of a block.
const BLOCK_START: &str = "<<'SQL'";
/// The line that ends a block.
const BLOCK_END: &str = "SQL";

/// The first SQL block after the heading of the section.
fn rollback_query() -> String {
    let runbook = std::fs::read_to_string(RUNBOOK).expect("the runbook is readable");
    let mut lines = runbook.lines().skip_while(|line| *line != HEADING);
    assert!(
        lines.next().is_some(),
        "the runbook has no heading {HEADING:?}"
    );
    let mut block = lines.skip_while(|line| !line.contains(BLOCK_START)).skip(1);
    let query: Vec<&str> = block
        .by_ref()
        .take_while(|line| *line != BLOCK_END)
        .collect();
    assert!(
        !query.is_empty(),
        "the runbook has no SQL block after {HEADING:?}"
    );
    query.join("\n")
}

/// The two counts of the query.
async fn counts(app: &mut App) -> (i64, i64) {
    sqlx::query_as(&rollback_query())
        .fetch_one(app.admin())
        .await
        .unwrap()
}

/// Set the encoding of a machine through `SetMachineStringEncoding`.
async fn set_encoding(app: &App, code: recipe_domain::plant::MachineCode, to: StringEncoding) {
    SetMachineStringEncoding::new(app.context())
        .execute(
            &app.user,
            &app.meta,
            SetMachineStringEncodingInput {
                code,
                string_encoding: to,
            },
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn adr_0202_rollback_query_counts_utf8_machines_and_encoding_entries_on_a_fresh_database() {
    let mut app = App::create().await;

    assert_eq!(counts(&mut app).await, (0, 0));
}

#[tokio::test]
async fn adr_0202_rollback_query_counts_utf8_machines_and_encoding_entries_for_a_stored_utf8_value()
{
    let mut app = App::create().await;
    let id = app
        .seed_user(ENGINEER, Role::ProcessEngineer, PASSWORD)
        .await;
    let engineer = CurrentUser::new(id, Login::parse(ENGINEER).unwrap(), Role::ProcessEngineer);
    let seed = app.seed_machine("CodesysIec", &["U1"]).await;
    set_encoding(&app, seed.code, StringEncoding::Utf8).await;
    let case = Case {
        reference: "RB-UTF8",
        platform: "CodesysIec",
        structs: Vec::new(),
        type_text: "STRING(4)",
        address: None,
        value: "\"é\"".to_owned(),
        limits: vec![length_limit("", 4)],
    };
    let reference = ReferenceNumber::parse(case.reference).unwrap();
    let created = CreateRecipeInput {
        machine_code: seed.code,
        reference: reference.clone(),
        name: text(case.reference),
        change_note: long_text("Stored UTF-8 row"),
    };
    CreateRecipe::new(app.context())
        .execute(&engineer, &new_meta(), created)
        .await
        .unwrap();
    add_case_parameter(&app, &engineer, &reference, &case).await;
    let approval = ApproveVersionInput {
        reference: reference.clone(),
        number: 1,
        expected_row_version: row_version(&app, &reference).await,
        allow_outdated: false,
    };
    ApproveVersion::new(app.context())
        .execute(&app.user, &new_meta(), approval)
        .await
        .unwrap();

    let bytes: i32 = sqlx::query_scalar(
        "SELECT octet_length(convert_to(p.value #>> '{}', 'UTF8')) FROM parameter p \
         JOIN recipe_version v ON v.id = p.recipe_version_id \
         JOIN recipe r ON r.id = v.recipe_id WHERE r.reference_number = $1",
    )
    .bind(case.reference)
    .fetch_one(app.admin())
    .await
    .unwrap();
    assert_eq!(bytes, 2);
    assert_eq!(counts(&mut app).await, (1, 1));
}

#[tokio::test]
async fn adr_0202_rollback_query_counts_utf8_machines_and_encoding_entries_after_a_change_back() {
    let mut app = App::create().await;
    let seed = app.seed_machine("CodesysIec", &["U1"]).await;

    set_encoding(&app, seed.code, StringEncoding::Utf8).await;
    set_encoding(&app, seed.code, StringEncoding::Latin1).await;

    assert_eq!(counts(&mut app).await, (0, 2));
}
