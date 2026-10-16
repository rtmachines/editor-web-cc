//! Tests of `RecipeReads::parameter_page`, `unit_counts`, and `parameter_by_key` in memory (design D5).

use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::{Ordinal, ParameterTag, StoredParameter, VersionState};

use super::support::{begin, seed_live};
use crate::error::StorageError;
use crate::memory::MemoryStore;
use crate::memory::fixtures::{machine, param, pkey, store, text, uid, vid};
use crate::ports::{Isolation, ParameterRow, Transaction};

const TWO_VERSIONS: &[(u128, VersionState)] =
    &[(11, VersionState::Approved), (12, VersionState::LiveDraft)];

/// A parameter on the unit `unit` with the given tag and ordinal.
fn placed(n: u128, unit: u128, (tag, ordinal): (&str, u32)) -> StoredParameter {
    StoredParameter {
        ordinal: Ordinal::parse(ordinal).unwrap(),
        ..param(n, unit, tag)
    }
}

/// A store with the units U1 (101), U1.2 (102), and U1.3 (103) of M1, and U1.2 (202) and U1.10 (210) of M2.
fn store_with_units() -> MemoryStore {
    let store = store();
    let mut second = machine(2, "M2");
    let root = UnitPath::parse("U1").unwrap();
    second
        .add_unit(uid(202), Some(&root), "2", text("Two"))
        .unwrap();
    second
        .add_unit(uid(210), Some(&root), "10", text("Ten"))
        .unwrap();
    store.seed_machine(&second);
    seed_live(&store, (1, "R-1", "One"), TWO_VERSIONS);
    store
}

fn keys(rows: &[ParameterRow]) -> Vec<recipe_domain::recipe::ParameterKey> {
    rows.iter().map(|row| row.key).collect()
}

fn seed_all(store: &MemoryStore, version: u128, parameters: &[StoredParameter]) {
    for parameter in parameters {
        store.seed_parameter(vid(version), parameter);
    }
}

#[tokio::test]
async fn parameter_page_orders_by_the_numeric_unit_path_then_ordinal_then_tag() {
    let store = store_with_units();
    seed_all(
        &store,
        12,
        &[
            placed(1, 102, ("B", 0)),
            placed(2, 101, ("Z", 2)),
            placed(3, 101, ("A", 2)),
            placed(4, 101, ("M", 1)),
            placed(5, 210, ("A", 0)),
        ],
    );
    let mut tx = begin(&store, Isolation::RepeatableRead);

    let rows = tx
        .recipe_reads()
        .parameter_page(vid(12), None, 0, 10)
        .await
        .unwrap();

    assert_eq!(keys(&rows), [pkey(4), pkey(3), pkey(2), pkey(1), pkey(5)]);
}

#[tokio::test]
async fn parameter_page_returns_the_columns_of_the_table_row() {
    let store = store_with_units();
    seed_all(&store, 12, &[placed(1, 102, ("B", 3))]);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let rows = tx
        .recipe_reads()
        .parameter_page(vid(12), None, 0, 10)
        .await
        .unwrap();

    let row = rows.first().unwrap();
    assert_eq!(row.key, pkey(1));
    assert_eq!(row.unit, uid(102));
    assert_eq!(row.unit_path, UnitPath::parse("U1.2").unwrap());
    assert_eq!(row.tag, ParameterTag::parse("B").unwrap());
    assert_eq!(row.name_en, text("Name"));
    assert_eq!(row.ordinal, Ordinal::parse(3).unwrap());
    assert_eq!(row.plc_type, param(1, 102, "B").plc_type);
    assert_eq!(row.s7_address, None);
}

#[tokio::test]
async fn parameter_page_keeps_only_the_unit_and_the_version_that_it_names() {
    let store = store_with_units();
    seed_all(
        &store,
        12,
        &[placed(1, 101, ("A", 0)), placed(2, 102, ("A", 0))],
    );
    seed_all(&store, 11, &[placed(3, 101, ("A", 0))]);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let in_unit = tx
        .recipe_reads()
        .parameter_page(vid(12), Some(uid(101)), 0, 10)
        .await
        .unwrap();
    let empty_unit = tx
        .recipe_reads()
        .parameter_page(vid(12), Some(uid(103)), 0, 10)
        .await
        .unwrap();
    let unknown_version = tx
        .recipe_reads()
        .parameter_page(vid(99), None, 0, 10)
        .await
        .unwrap();

    assert_eq!(keys(&in_unit), [pkey(1)]);
    assert!(empty_unit.is_empty());
    assert!(unknown_version.is_empty());
}

#[tokio::test]
async fn parameter_page_returns_the_page_that_offset_and_limit_name() {
    let store = store_with_units();
    seed_all(
        &store,
        12,
        &[
            placed(1, 101, ("A", 0)),
            placed(2, 101, ("B", 0)),
            placed(3, 101, ("C", 0)),
        ],
    );
    let mut tx = begin(&store, Isolation::ReadOnly);

    let page = tx
        .recipe_reads()
        .parameter_page(vid(12), None, 1, 1)
        .await
        .unwrap();
    let beyond = u64::try_from(i64::MAX).unwrap().checked_add(1).unwrap();
    let error = tx
        .recipe_reads()
        .parameter_page(vid(12), None, beyond, 1)
        .await;

    assert_eq!(keys(&page), [pkey(2)]);
    assert!(matches!(
        error.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

#[tokio::test]
async fn unit_counts_count_the_parameters_of_each_unit_in_the_numeric_order_of_the_path() {
    let store = store_with_units();
    seed_all(
        &store,
        12,
        &[
            placed(1, 102, ("A", 0)),
            placed(2, 101, ("A", 0)),
            placed(3, 101, ("B", 0)),
            placed(4, 210, ("A", 0)),
        ],
    );
    seed_all(&store, 11, &[placed(5, 103, ("A", 0))]);
    let mut tx = begin(&store, Isolation::RepeatableRead);

    let counts = tx.recipe_reads().unit_counts(vid(12)).await.unwrap();

    let seen: Vec<_> = counts
        .iter()
        .map(|count| (count.unit, count.path.to_string(), count.count))
        .collect();
    assert_eq!(
        seen,
        [
            (uid(101), "U1".to_owned(), 2),
            (uid(102), "U1.2".to_owned(), 1),
            (uid(210), "U1.10".to_owned(), 1),
        ]
    );
}

#[tokio::test]
async fn unit_counts_of_a_version_without_parameters_are_empty() {
    let store = store_with_units();
    let mut tx = begin(&store, Isolation::ReadOnly);

    let counts = tx.recipe_reads().unit_counts(vid(12)).await.unwrap();

    assert!(counts.is_empty());
}

#[tokio::test]
async fn parameter_by_key_returns_the_parameter_with_limits_and_translations() {
    let store = store_with_units();
    let wanted = placed(1, 102, ("B", 4));
    seed_all(&store, 12, &[wanted.clone(), placed(2, 101, ("A", 0))]);
    let mut tx = begin(&store, Isolation::RepeatableRead);

    let found = tx
        .recipe_reads()
        .parameter_by_key(vid(12), pkey(1))
        .await
        .unwrap()
        .unwrap();

    assert_eq!(found.id, wanted.id);
    assert_eq!(found.key, wanted.key);
    assert_eq!(found.unit, wanted.unit);
    assert_eq!(found.tag, wanted.tag);
    assert_eq!(found.ordinal, wanted.ordinal);
    assert_eq!(found.limits.len(), 1);
    assert_eq!(found.translations.len(), 1);
    assert_eq!(
        found.translations.first().unwrap().lang,
        wanted.translations.first().unwrap().lang
    );
}

#[tokio::test]
async fn parameter_by_key_looks_only_into_the_version_that_it_names() {
    let store = store_with_units();
    seed_all(&store, 11, &[placed(1, 101, ("A", 0))]);
    let mut tx = begin(&store, Isolation::ReadOnly);

    let in_other_version = tx
        .recipe_reads()
        .parameter_by_key(vid(12), pkey(1))
        .await
        .unwrap();
    let unknown_key = tx
        .recipe_reads()
        .parameter_by_key(vid(11), pkey(9))
        .await
        .unwrap();

    assert!(in_other_version.is_none());
    assert!(unknown_key.is_none());
}
