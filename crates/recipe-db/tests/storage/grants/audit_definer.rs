//! Tests of the migration `0007_audit_definer_operators.sql` (`data-model.md` section 5.6,
//! ADR-0098 (a), (f), and (g)).
//!
//! The migration qualifies the operators and types of `audit_hash_chain()` and
//! `security_event_purge()`. The scan covers every function that `audit_owner` owns.

use std::collections::BTreeMap;

use super::operator_scan::unqualified_operators;
use crate::support::{Role, TestDatabase, document_sql, migration_sql};

const MIGRATION: &str = "0007_audit_definer_operators.sql";
/// The functions that the languages `sql` and `plpgsql` hold. The scanner reads only these.
const SCANNED_LANGUAGES: [&str; 2] = ["plpgsql", "sql"];
/// The functions of `audit_owner` after `0006`.
const OWNED_FUNCTIONS: [&str; 5] = [
    "audit_canonical",
    "audit_hash_chain",
    "replication_state",
    "security_event_guard",
    "security_event_purge",
];
const SEARCH_PATH: &str = "search_path=pg_catalog, public, pg_temp";
/// The ACL of `audit_hash_chain()` after `0002`: the owner only (`REVOKE ... FROM PUBLIC`).
const HASH_CHAIN_ACL: &str = "{audit_owner=X/audit_owner}";
/// The ACL of `security_event_purge()` after `0003`: the owner and `recipe_app`.
const PURGE_ACL: &str = "{audit_owner=X/audit_owner,recipe_app=X/audit_owner}";

#[test]
fn migration_0007_equals_the_sql_block_of_section_5_6() {
    let document = document_sql(&["5.6"], &[]);
    let file = migration_sql(MIGRATION);

    assert!(document.starts_with("GRANT CREATE ON SCHEMA public TO audit_owner;"));
    assert!(document.ends_with("REVOKE CREATE ON SCHEMA public FROM audit_owner;"));
    assert_eq!(file, document);
}

/// The name, the language, and the `prosrc` of every function that `audit_owner` owns.
async fn owned_functions(database: &TestDatabase) -> Vec<(String, String, String)> {
    let mut admin = database.connect(Role::Superuser).await;
    sqlx::query_as(
        "SELECT p.proname::text, l.lanname::text, p.prosrc FROM pg_proc p \
         JOIN pg_language l ON l.oid = p.prolang \
         WHERE p.proowner = 'audit_owner'::regrole AND p.pronamespace = 'public'::regnamespace \
         ORDER BY p.proname",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap()
}

#[tokio::test]
async fn every_function_of_audit_owner_has_only_qualified_operators() {
    let database = TestDatabase::create().await;

    let functions = owned_functions(&database).await;

    let names: Vec<&str> = functions.iter().map(|(name, _, _)| name.as_str()).collect();
    assert_eq!(names, OWNED_FUNCTIONS, "audit_owner owns another function");
    let mut hits: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    for (name, language, body) in &functions {
        assert!(
            SCANNED_LANGUAGES.contains(&language.as_str()),
            "{name} is written in {language}, which the scanner does not read"
        );
        let found = unqualified_operators(body);
        if !found.is_empty() {
            hits.insert(name, found);
        }
    }
    assert_eq!(
        hits,
        BTreeMap::new(),
        "bare operators in functions of audit_owner"
    );
}

#[tokio::test]
async fn the_two_changed_functions_keep_definer_search_path_owner_and_acl() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let rows: Vec<(String, bool, Vec<String>, String, String)> = sqlx::query_as(
        "SELECT proname::text, prosecdef, proconfig, pg_get_userbyid(proowner)::text, \
                proacl::text \
         FROM pg_proc \
         WHERE proname IN ('audit_hash_chain', 'security_event_purge') \
           AND pronamespace = 'public'::regnamespace \
         ORDER BY proname",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    let expected = |name: &str, acl: &str| {
        (
            name.to_owned(),
            true,
            vec![SEARCH_PATH.to_owned()],
            "audit_owner".to_owned(),
            acl.to_owned(),
        )
    };
    assert_eq!(
        rows,
        [
            expected("audit_hash_chain", HASH_CHAIN_ACL),
            expected("security_event_purge", PURGE_ACL)
        ]
    );
}
