use sqlx::postgres::PgSslMode;

use super::connect_options;
use crate::settings::{Connection, Password};

fn target() -> Connection {
    Connection {
        host: "db.internal".to_owned(),
        port: 5433,
        database: "recipe".to_owned(),
        user: "recipe_app".to_owned(),
        password: Password::from("not-a-real-secret".to_owned()),
    }
}

#[test]
fn options_carry_the_fields_of_the_target_and_disable_tls() {
    let options = connect_options(&target());

    assert_eq!(options.get_host(), "db.internal");
    assert_eq!(options.get_port(), 5433);
    assert_eq!(options.get_username(), "recipe_app");
    assert_eq!(options.get_database(), Some("recipe"));
    assert!(matches!(options.get_ssl_mode(), PgSslMode::Disable));
}

#[test]
fn options_set_the_application_name_of_the_server() {
    let options = connect_options(&target());

    assert_eq!(options.get_application_name(), Some("recipe-server"));
}

/// The pgpass lookup of `sqlx` leaves no trace in the options, because code sets the password
/// afterwards. The source text is therefore the only place where the rule is visible.
#[test]
fn options_are_built_without_the_pgpass_file() {
    let source = include_str!("../options.rs");
    let (production, _tests) = source.split_once("#[cfg(test)]").unwrap();
    let reading_constructor = concat!("PgConnectOptions::", "new()");

    assert!(production.contains("new_without_pgpass()"));
    assert!(!production.contains(reading_constructor));
}
