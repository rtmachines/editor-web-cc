//! Tests of the error map on the database (task 8.4, design D2 and D4).
//!
//! The map is private, so every test reaches a failure through a port adapter. This table lists
//! each rule of `reliability.md` section 1.1 and the test that covers it on the database.
//!
//! | Rule of the map | Test |
//! |---|---|
//! | Trigger code `E-RCP-031` | `contract::draft::insert::an_insert_into_an_approved_version_gives_e_rcp_031` |
//! | Trigger code `E-RCP-033` | `contract::recipe_write::status::the_discard_of_an_approved_version_gives_e_rcp_033` |
//! | Trigger code `E-PLT-030..049` | No port reaches one without a domain refusal first. The unit tests of `error` cover the range, and `triggers::*` cover each trigger in SQL |
//! | `23514` without a trigger code, `E-DB-003` | `a_check_constraint_without_a_trigger_code_gives_e_db_003` here. The contract cases `a_struct_record_gives_the_database_rule_error` and `a_row_that_is_not_a_struct_gives_the_database_rule_error` cover the refusal of the adapter, not a CHECK |
//! | A code outside the trigger ranges | The unit test `a_code_outside_the_trigger_ranges_gives_e_db_003` of `error`. No trigger of the schema raises such a code |
//! | `23505`, `Duplicate` | `contract::machine::a_live_duplicate_code_gives_the_constraint_of_the_live_index` and the other duplicate cases of `contract` |
//! | `23503`, `MissingReference` | `contract::unit::insert_unit_with_a_missing_parent_gives_missing_reference` and the other reference cases |
//! | A constraint name that a use case does not know, `E-DB-006` and `E-DB-007` | The name reaches the use case from a real server in `an_unknown_unique_constraint_gives_duplicate_with_its_name` and `an_unknown_foreign_key_gives_missing_reference_with_its_name` here. The conversion is `map_storage` of `recipe-app`: `error::mapping::tests`, `plant::add_unit::tests`, `recipe::add_parameter::tests`, `recipe::update_definition::tests` |
//! | `40P01`, `Deadlock`, `E-DB-001` | `two_transactions_that_lock_two_machines_in_opposite_order_give_one_deadlock` here |
//! | `40001`, `Deadlock` with the cause `Serialization` | The unit test of `error`. The retry belongs to P2.5 (`SetUserRole`, `DeactivateUser`) |
//! | `57014`, `StatementTimeout`, `E-DB-002` | `pool::a_statement_that_runs_too_long_gives_statement_timeout` |
//! | `55P03`, `Deadlock`, `E-DB-001` | `pool::a_lock_wait_that_passes_the_lock_timeout_gives_deadlock` |
//! | Pool acquire timeout, `E-DB-004` | `pool::a_pool_without_a_free_connection_gives_pool_timeout` |
//! | Connection failure, `E-DB-005` | `pool::connect_to_a_closed_port_gives_unavailable` |
//! | Any other code, `Other` with the SQLSTATE | `a_raise_without_a_trigger_code_gives_other_with_the_sqlstate` here |

use std::sync::Arc;

use recipe_app::error::{LockFailure, StorageError};
use recipe_app::ports::{Isolation, UnitOfWork};
use recipe_db::Database;
use recipe_db::settings::PoolSettings;
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::UserId;
use recipe_domain::plant::{Machine, MachineCode, MachineId, PlcPlatform, UnitId};
use recipe_domain::recipe::{ParameterKey, RecipeVersionId, StoredValue};
use recipe_domain::text::Text;
use serde_json::json;
use std::time::Duration;
use uuid::Uuid;

use crate::support::audit::{audit_count, entries};
use crate::support::fixtures::{self, Draft};
use crate::support::probe;
use crate::support::{Role, TestDatabase};

/// The lock wait of the deadlock test. It stays far above the 1 s of `deadlock_timeout`.
const LONG_LOCK_TIMEOUT: Duration = Duration::from_secs(20);
/// The size of a JSON string that breaks `parameter_value_size` (1 MiB of column size).
const OVERSIZE_VALUE_BYTES: usize = 1_100_000;
/// The trigger body of an error without a trigger code and without a special SQLSTATE.
const PLAIN_RAISE: &str = "RAISE EXCEPTION 'injected fault';";

/// Open the pool of `recipe_app` on a test database.
async fn connect(database: &TestDatabase, settings: &PoolSettings) -> Arc<dyn UnitOfWork> {
    Database::connect(&database.target(Role::App), settings)
        .await
        .unwrap()
        .unit_of_work()
}

/// Build a new machine for `insert_machine`.
fn new_machine() -> Machine {
    Machine::create(
        MachineId::from(Uuid::new_v4()),
        MachineCode::parse("M77").unwrap(),
        Text::parse("Press").unwrap(),
        PlcPlatform::TwinCat3,
        (UnitId::from(Uuid::new_v4()), Text::parse("Feeder").unwrap()),
    )
}

/// Install a trigger with `body` on `machine` and return the error of `insert_machine`.
async fn insert_machine_error(body: &str) -> StorageError {
    let database = TestDatabase::create().await;
    probe::install(&database, "machine", "BEFORE", "INSERT", body).await;
    let unit_of_work = connect(&database, &PoolSettings::default()).await;
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();

    tx.plant()
        .insert_machine(&new_machine())
        .await
        .err()
        .unwrap()
}

#[tokio::test]
async fn two_transactions_that_lock_two_machines_in_opposite_order_give_one_deadlock() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;
    let user = fixtures::user(&mut migrator).await;
    let mut ids = Vec::new();
    for _ in 0..2 {
        let machine = fixtures::machine(&mut migrator, fixtures::TWIN_CAT).await;
        fixtures::unit(&mut migrator, machine, "U1").await;
        ids.push(MachineId::from(machine));
    }
    let (first, second) = (ids[0], ids[1]);
    let settings = PoolSettings {
        lock_timeout: LONG_LOCK_TIMEOUT,
        ..PoolSettings::default()
    };
    let unit_of_work = connect(&database, &settings).await;
    let mut left = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();
    let mut right = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();
    left.plant()
        .lock_machine_for_no_key_update(first)
        .await
        .unwrap();
    right
        .plant()
        .lock_machine_for_no_key_update(second)
        .await
        .unwrap();

    let (left_result, right_result) = tokio::join!(
        async {
            left.plant()
                .lock_machine_for_no_key_update(second)
                .await
                .map(|_| ())
        },
        async {
            right
                .plant()
                .lock_machine_for_no_key_update(first)
                .await
                .map(|_| ())
        },
    );

    let left_failed = left_result.is_err();
    assert_ne!(
        left_failed,
        right_result.is_err(),
        "exactly one transaction fails"
    );
    let (winner, loser_error) = if left_failed {
        (right, left_result.err().unwrap())
    } else {
        (left, right_result.err().unwrap())
    };
    assert!(
        matches!(
            loser_error,
            StorageError::Deadlock {
                cause: LockFailure::Deadlock
            }
        ),
        "{loser_error:?}"
    );
    assert_eq!(loser_error.code(), ErrorCode::DB_001);
    let (list, _) = entries(Some(user), 1);
    unit_of_work.commit(winner, list).await.unwrap();
    let mut admin = database.connect(Role::Superuser).await;
    assert_eq!(audit_count(&mut admin).await, 1);
}

#[tokio::test]
async fn a_raise_without_a_trigger_code_gives_other_with_the_sqlstate() {
    let error = insert_machine_error(PLAIN_RAISE).await;

    assert!(
        matches!(&error, StorageError::Other { sqlstate, .. } if sqlstate.as_deref() == Some("P0001")),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_unknown_unique_constraint_gives_duplicate_with_its_name() {
    let error = insert_machine_error(
        "RAISE EXCEPTION 'injected' USING ERRCODE = '23505', CONSTRAINT = 'test_unknown_unique';",
    )
    .await;

    assert!(
        matches!(&error, StorageError::Duplicate { constraint } if constraint == "test_unknown_unique"),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_006);
}

#[tokio::test]
async fn an_unknown_foreign_key_gives_missing_reference_with_its_name() {
    let error = insert_machine_error(
        "RAISE EXCEPTION 'injected' USING ERRCODE = '23503', CONSTRAINT = 'test_unknown_fkey';",
    )
    .await;

    assert!(
        matches!(&error, StorageError::MissingReference { constraint } if constraint == "test_unknown_fkey"),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_007);
}

#[tokio::test]
async fn a_check_constraint_without_a_trigger_code_gives_e_db_003() {
    let database = TestDatabase::create().await;
    let mut migrator = database.connect(Role::Migrator).await;
    let draft = Draft::create(&mut migrator).await;
    let parameter = draft.parameter(&mut migrator, "BIG").await;
    let key: Uuid = sqlx::query_scalar("SELECT parameter_key FROM parameter WHERE id = $1")
        .bind(parameter)
        .fetch_one(&mut migrator)
        .await
        .unwrap();
    let version = RecipeVersionId::from(draft.version);
    let unit_of_work = connect(&database, &PoolSettings::default()).await;
    let mut tx = unit_of_work.begin(Isolation::ReadCommitted).await.unwrap();
    tx.drafts().lock_draft(version).await.unwrap().unwrap();
    tx.drafts()
        .touch_draft(version, UserId::from(draft.user))
        .await
        .unwrap();
    let oversize = StoredValue::from(json!("x".repeat(OVERSIZE_VALUE_BYTES)));

    let error = tx
        .drafts()
        .set_value(version, ParameterKey::from(key), &oversize)
        .await
        .unwrap_err();

    assert!(
        matches!(&error, StorageError::Constraint { code, message }
            if *code == ErrorCode::DB_003
                && message.as_str() == "The change violates a database rule."),
        "{error:?}"
    );
}
