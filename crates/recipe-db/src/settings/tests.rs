use std::time::Duration;

use super::{Connection, MigrateSettings, Password, PoolSettings};

const SECRET: &str = "s3cret-value-1234";
const RELIABILITY_MD: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// Return the value cell of the key in the table of `reliability.md` section 7.11.
///
/// A row can hold several keys and several values, separated by ` / `.
fn table_value(table: &str, key: &str) -> String {
    table
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let keys = cells.get(1)?;
            let values = cells.get(2)?;
            let position = keys
                .split(" / ")
                .position(|name| name.trim_matches('`') == key)?;
            values.split(" / ").nth(position).map(str::to_owned)
        })
        .next()
        .unwrap_or_else(|| String::from("<key not found>"))
}

/// Read a duration such as `10 min`. A cell can continue with a sentence after the unit.
fn duration_of(text: &str) -> Duration {
    let mut words = text.split_whitespace();
    let number: u64 = words.next().unwrap().parse().unwrap();
    let unit = words.next().unwrap().trim_end_matches('.');
    let seconds = match unit {
        "s" => Some(number),
        "min" => number.checked_mul(60),
        _ => None,
    };
    Duration::from_secs(seconds.expect("the unit is `s` or `min`"))
}

fn section_7_11() -> String {
    let document = std::fs::read_to_string(RELIABILITY_MD).unwrap();
    let (_, table) = document.split_once("### 7.11 Settings").unwrap();
    table.to_owned()
}

#[test]
fn password_debug_shows_placeholder_and_never_the_secret() {
    let password = Password::from(SECRET.to_owned());

    let shown = format!("{password:?}");

    assert!(shown.contains("[redacted]"));
    assert!(!shown.contains(SECRET));
}

#[test]
fn connection_debug_never_shows_the_password() {
    let connection = Connection {
        host: "db".to_owned(),
        port: 5432,
        database: "recipe".to_owned(),
        user: "recipe_migrator".to_owned(),
        password: Password::from(SECRET.to_owned()),
    };

    let shown = format!("{connection:?}");

    assert!(shown.contains("recipe_migrator"));
    assert!(shown.contains("[redacted]"));
    assert!(!shown.contains(SECRET));
}

#[test]
fn pool_defaults_equal_the_settings_table() {
    let table = section_7_11();
    let settings = PoolSettings::default();

    let duration = |key: &str| duration_of(&table_value(&table, key));
    assert_eq!(settings.statement_timeout, duration("db.statement_timeout"));
    assert_eq!(settings.lock_timeout, duration("db.lock_timeout"));
    assert_eq!(
        settings.idle_in_transaction_timeout,
        duration("db.idle_in_transaction_timeout")
    );
    assert_eq!(
        settings.acquire_timeout,
        duration("db.pool_acquire_timeout")
    );
    assert_eq!(settings.idle_timeout, duration("db.pool_idle_timeout"));
    assert_eq!(settings.max_lifetime, duration("db.pool_max_lifetime"));
    let size: u32 = table_value(&table, "db.pool_size").parse().unwrap();
    assert_eq!(settings.size, size);
    assert_eq!(settings.wait_warn, duration("db.pool_wait_warn"));
}

#[test]
fn migrate_defaults_equal_the_settings_table() {
    let table = section_7_11();
    let settings = MigrateSettings::default();

    assert_eq!(
        settings.lock_timeout,
        duration_of(&table_value(&table, "migrate.lock_timeout"))
    );
    let retries: u32 = table_value(&table, "migrate.retries").parse().unwrap();
    assert_eq!(settings.retries, retries);
    assert_eq!(
        settings.retry_pause,
        duration_of(&table_value(&table, "migrate.retry_pause"))
    );
    assert_eq!(
        settings.connect_timeout,
        duration_of(&table_value(&table, "migrate.connect_timeout"))
    );
}

#[test]
fn table_parser_reads_a_row_with_several_keys() {
    let table = "| `a.one` / `a.two` | 10 / 2 | 7.5 |\n";

    assert_eq!(table_value(table, "a.one"), "10");
    assert_eq!(table_value(table, "a.two"), "2");
}
