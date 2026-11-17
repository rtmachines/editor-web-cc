//! A row lock in a `READ ONLY` transaction (ADR-0042 (a), design D7).
//!
//! PostgreSQL refuses `SELECT ... FOR UPDATE` and `FOR SHARE` in a read-only transaction with
//! SQLSTATE `25006`, also when no row matches. The memory adapter gives the same error at both read-only levels.
//! These cases run on both adapters with the same assertions.

use recipe_app::error::StorageError;
use recipe_app::ports::{Isolation, Transaction};
use recipe_domain::error_codes::ErrorCode;

use crate::contract::harness::Harness;
use crate::contract::recipe_seed::{create_recipe, plant, recipe_id, version_id};
use crate::contract::seed::machine_id;

/// The two levels that run `READ ONLY` (ADR-0042 (a), ADR-0050 (f)).
const READ_ONLY_LEVELS: [Isolation; 2] = [Isolation::ReadOnly, Isolation::RepeatableRead];

/// Assert the error of a row lock in a read-only transaction: `25006` and `E-DB-008`.
fn assert_read_only_refusal<T: std::fmt::Debug>(result: Result<T, StorageError>) {
    let error = result.unwrap_err();
    assert!(
        matches!(&error, StorageError::Other { sqlstate: Some(state), .. } if state == "25006"),
        "{error:?}"
    );
    assert_eq!(error.code(), ErrorCode::DB_008);
}

/// Open a read-only transaction. An error aborts a PostgreSQL transaction, so each lock gets its own.
async fn begin_at(h: &dyn Harness, level: Isolation) -> Box<dyn Transaction> {
    h.unit_of_work().begin(level).await.unwrap()
}

async fn a_row_lock_of_an_existing_row_in_a_read_only_transaction_gives_25006(h: &dyn Harness) {
    let plant = plant(h, 21, "M21").await;
    let created = create_recipe(h, &plant, 21, "REF-21").await;
    let draft = created.versions()[0].id();

    for level in READ_ONLY_LEVELS {
        let result = begin_at(h, level).await.drafts().lock_draft(draft).await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level).await.recipes().lock_version(draft).await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_versions(created.id())
            .await;
        assert_read_only_refusal(result);
    }
}

async fn a_row_lock_of_a_missing_row_in_a_read_only_transaction_gives_25006(h: &dyn Harness) {
    for level in READ_ONLY_LEVELS {
        let result = begin_at(h, level)
            .await
            .drafts()
            .lock_draft(version_id(900))
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_version(version_id(900))
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_versions(recipe_id(900))
            .await;
        assert_read_only_refusal(result);
    }
}

async fn a_recipe_lock_in_a_read_only_transaction_gives_25006(h: &dyn Harness) {
    let plant = plant(h, 22, "M22").await;
    let created = create_recipe(h, &plant, 22, "REF-22").await;

    for level in READ_ONLY_LEVELS {
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_recipe(created.id())
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_recipe(recipe_id(900))
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_recipes_of_machine(plant.machine)
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_recipes_of_machine(machine_id(900))
            .await;
        assert_read_only_refusal(result);
    }
}

async fn a_shared_version_lock_in_a_read_only_transaction_gives_25006(h: &dyn Harness) {
    let plant = plant(h, 23, "M23").await;
    let created = create_recipe(h, &plant, 23, "REF-23").await;
    let draft = created.versions()[0].id();

    for level in READ_ONLY_LEVELS {
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_version_shared(draft)
            .await;
        assert_read_only_refusal(result);
        let result = begin_at(h, level)
            .await
            .recipes()
            .lock_version_shared(version_id(900))
            .await;
        assert_read_only_refusal(result);
    }
}

contract_case!(
    a_row_lock_of_an_existing_row_in_a_read_only_transaction_gives_25006,
    a_row_lock_of_an_existing_row_in_a_read_only_transaction_gives_25006_memory,
    a_row_lock_of_an_existing_row_in_a_read_only_transaction_gives_25006_postgres
);
contract_case!(
    a_row_lock_of_a_missing_row_in_a_read_only_transaction_gives_25006,
    a_row_lock_of_a_missing_row_in_a_read_only_transaction_gives_25006_memory,
    a_row_lock_of_a_missing_row_in_a_read_only_transaction_gives_25006_postgres
);
contract_case!(
    a_recipe_lock_in_a_read_only_transaction_gives_25006,
    a_recipe_lock_in_a_read_only_transaction_gives_25006_memory,
    a_recipe_lock_in_a_read_only_transaction_gives_25006_postgres
);
contract_case!(
    a_shared_version_lock_in_a_read_only_transaction_gives_25006,
    a_shared_version_lock_in_a_read_only_transaction_gives_25006_memory,
    a_shared_version_lock_in_a_read_only_transaction_gives_25006_postgres
);
