//! Look up the names of a migration in `pg_catalog`.

use sqlx::PgConnection;

use super::document::CreatedNames;

const TABLE_QUERY: &str = "SELECT count(*) FROM pg_class \
     WHERE relname = $1 AND relkind IN ('r', 'p') AND relnamespace = 'public'::regnamespace";
const CONSTRAINT_QUERY: &str = "SELECT count(*) FROM pg_constraint \
     WHERE conname = $1 AND connamespace = 'public'::regnamespace";
const INDEX_QUERY: &str = "SELECT count(*) FROM pg_class \
     WHERE relname = $1 AND relkind IN ('i', 'I') AND relnamespace = 'public'::regnamespace";
const TRIGGER_QUERY: &str =
    "SELECT count(*) FROM pg_trigger WHERE tgname = $1 AND NOT tgisinternal";
const FUNCTION_QUERY: &str = "SELECT count(*) FROM pg_proc \
     WHERE proname = $1 AND pronamespace = 'public'::regnamespace";

async fn exists(conn: &mut PgConnection, query: &str, name: &str) -> bool {
    let count: i64 = sqlx::query_scalar(query)
        .bind(name)
        .fetch_one(&mut *conn)
        .await
        .unwrap();
    count > 0
}

/// Return `kind name` for each created name that `pg_catalog` does not hold.
pub async fn missing_names(conn: &mut PgConnection, names: &CreatedNames) -> Vec<String> {
    let groups: [(&str, &str, &Vec<String>); 5] = [
        ("table", TABLE_QUERY, &names.tables),
        ("constraint", CONSTRAINT_QUERY, &names.constraints),
        ("index", INDEX_QUERY, &names.indexes),
        ("trigger", TRIGGER_QUERY, &names.triggers),
        ("function", FUNCTION_QUERY, &names.functions),
    ];
    let mut missing = Vec::new();
    for (kind, query, list) in groups {
        for name in list {
            if !exists(conn, query, name).await {
                missing.push(format!("{kind} {name}"));
            }
        }
    }
    missing
}
