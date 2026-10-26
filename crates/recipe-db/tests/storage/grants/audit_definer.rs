//! Tests of the migration `0007_audit_definer_operators.sql` (`data-model.md` section 5.6,
//! ADR-0098 (a), (f), and (g)).
//!
//! The migration qualifies the operators and types of `audit_hash_chain()`,
//! `security_event_purge()`, and `security_event_guard()`. The scan covers every function that
//! `audit_owner` owns.

use std::collections::BTreeMap;

use super::operator_scan::unqualified_operators;
use crate::support::{Role, TestDatabase, assert_trigger, document_sql, migration_sql};

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
/// The ACL of `security_event_guard()` after `0005`: the owner only (`REVOKE ... FROM PUBLIC`).
const GUARD_ACL: &str = "{audit_owner=X/audit_owner}";

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
async fn the_three_changed_functions_keep_security_mode_search_path_owner_and_acl() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let rows: Vec<(String, bool, Vec<String>, String, String)> = sqlx::query_as(
        "SELECT proname::text, prosecdef, proconfig, pg_get_userbyid(proowner)::text, \
                proacl::text \
         FROM pg_proc \
         WHERE proname IN ('audit_hash_chain', 'security_event_guard', 'security_event_purge') \
           AND pronamespace = 'public'::regnamespace \
         ORDER BY proname",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    let expected = |name: &str, is_definer: bool, acl: &str| {
        (
            name.to_owned(),
            is_definer,
            vec![SEARCH_PATH.to_owned()],
            "audit_owner".to_owned(),
            acl.to_owned(),
        )
    };
    assert_eq!(
        rows,
        [
            expected("audit_hash_chain", true, HASH_CHAIN_ACL),
            expected("security_event_guard", false, GUARD_ACL),
            expected("security_event_purge", true, PURGE_ACL)
        ]
    );
}

#[tokio::test]
async fn the_guard_trigger_stays_bound_to_security_event_before_update() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;

    let triggers: Vec<(String, String, bool, i16)> = sqlx::query_as(
        "SELECT t.tgname::text, t.tgrelid::regclass::text, t.tgenabled = 'O', t.tgtype \
         FROM pg_trigger t WHERE t.tgfoid = 'public.security_event_guard()'::regprocedure",
    )
    .fetch_all(&mut admin)
    .await
    .unwrap();

    // tgtype 19 = ROW (1) + BEFORE (2) + UPDATE (16).
    let bound = (
        "security_event_guard".to_owned(),
        "security_event".to_owned(),
        true,
        19,
    );
    assert_eq!(triggers, [bound]);
}

/// A table with the shape that the guard reads: `id`, a nullable `count`, and one more column.
/// A nullable `count` reaches the cases NULL/NULL and NULL/value, which `security_event` cannot.
const PROBE_TABLE: &str = "\
CREATE TABLE guard_probe (id INTEGER, count INTEGER, detail JSONB); \
CREATE TRIGGER guard_probe_guard BEFORE UPDATE ON guard_probe \
    FOR EACH ROW EXECUTE FUNCTION public.security_event_guard()";
const PROBE_COUNTS: [Option<i32>; 4] = [None, Some(1), Some(2), Some(3)];
const PROBE_DETAILS: [Option<&str>; 3] = [None, Some(r#"{"a":1}"#), Some(r#"{"a":2}"#)];
/// The decision of the old text of `0005`: its two `IS DISTINCT FROM` expressions.
const OLD_DECISION: &str = "SELECT $1::integer IS DISTINCT FROM ($2::integer + 1) \
    OR $3::text::jsonb IS DISTINCT FROM $4::text::jsonb";

#[tokio::test]
async fn the_guard_decides_like_is_distinct_from_for_null_and_for_values() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;
    sqlx::raw_sql(PROBE_TABLE)
        .execute(&mut admin)
        .await
        .unwrap();
    let (mut rejected, mut passed) = (0_u32, 0_u32);

    for old_count in PROBE_COUNTS {
        for new_count in PROBE_COUNTS {
            for old_detail in PROBE_DETAILS {
                for new_detail in PROBE_DETAILS {
                    sqlx::query("DELETE FROM guard_probe")
                        .execute(&mut admin)
                        .await
                        .unwrap();
                    sqlx::query("INSERT INTO guard_probe VALUES (1, $1, $2::text::jsonb)")
                        .bind(old_count)
                        .bind(old_detail)
                        .execute(&mut admin)
                        .await
                        .unwrap();
                    let is_rejected: bool = sqlx::query_scalar(OLD_DECISION)
                        .bind(new_count)
                        .bind(old_count)
                        .bind(new_detail)
                        .bind(old_detail)
                        .fetch_one(&mut admin)
                        .await
                        .unwrap();

                    let result =
                        sqlx::query("UPDATE guard_probe SET count = $1, detail = $2::text::jsonb")
                            .bind(new_count)
                            .bind(new_detail)
                            .execute(&mut admin)
                            .await;

                    if is_rejected {
                        assert_trigger(result, "E-AUD-003");
                        rejected += 1;
                    } else {
                        let case = (old_count, new_count, old_detail, new_detail);
                        assert!(result.is_ok(), "the guard rejects {case:?}: {result:?}");
                        passed += 1;
                    }
                }
            }
        }
    }
    assert!(
        rejected > 0 && passed > 0,
        "{rejected} rejected, {passed} passed"
    );
}

/// The form of `a IS DISTINCT FROM b` that the guard uses, for the columns `a` and `b`.
const QUALIFIED_FORM: &str = "((a IS NULL) OPERATOR(pg_catalog.<>) (b IS NULL)) \
    OR COALESCE(a OPERATOR(pg_catalog.<>) b, false)";

#[tokio::test]
async fn the_qualified_form_equals_is_distinct_from_for_integer_and_jsonb() {
    let database = TestDatabase::create().await;
    let mut admin = database.connect(Role::Superuser).await;
    let grids = [
        ("integer", ["NULL", "1", "2"]),
        ("jsonb", ["NULL", r#"'{"a":1}'"#, r#"'{"a":2}'"#]),
    ];

    for (type_name, values) in grids {
        let list = values
            .iter()
            .map(|value| format!("(CAST({value} AS {type_name}))"))
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT count(*), count(*) FILTER \
                 (WHERE ({QUALIFIED_FORM}) IS DISTINCT FROM (a IS DISTINCT FROM b)) \
             FROM (VALUES {list}) AS x(a), (VALUES {list}) AS y(b)"
        );

        let (cases, different): (i64, i64) =
            sqlx::query_as(&sql).fetch_one(&mut admin).await.unwrap();

        assert_eq!((type_name, cases, different), (type_name, 9, 0));
    }
}
