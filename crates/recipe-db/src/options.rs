//! Connection options shared by the migrator and the pool.

use sqlx::ConnectOptions as _;
use sqlx::postgres::{PgConnectOptions, PgSslMode};

use crate::settings::Connection;

/// The name that every connection of the server shows in `pg_stat_activity`.
const APPLICATION_NAME: &str = "recipe-server";

/// Build the connection options of a target.
///
/// The function solves two problems. The migrator and the pool must never log the text of a
/// statement, because a statement in a log line can hold a secret. The options therefore disable
/// statement logging. A connection must also come only from the configuration and the secret
/// files. The options skip the pgpass file of libpq. Code sets the host, the port, the database,
/// the user, the password, the TLS mode, and the application name. TLS is off, because the
/// connection stays inside the Compose network (ADR-0069 (a)).
///
/// The driver still reads the `PG*` values of the environment into the fields that code does not
/// set, and no setter clears `PGOPTIONS`. The subcommands therefore refuse a `PG*` variable
/// before they build any options (ADR-0069 (b)). That refusal covers the rest.
pub(crate) fn connect_options(target: &Connection) -> PgConnectOptions {
    PgConnectOptions::new_without_pgpass()
        .application_name(APPLICATION_NAME)
        .host(&target.host)
        .port(target.port)
        .database(&target.database)
        .username(&target.user)
        .password(target.password.expose())
        .ssl_mode(PgSslMode::Disable)
        .disable_statement_logging()
}

#[cfg(test)]
mod tests {
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
        let source = include_str!("options.rs");
        let (production, _tests) = source.split_once("#[cfg(test)]").unwrap();
        let reading_constructor = concat!("PgConnectOptions::", "new()");

        assert!(production.contains("new_without_pgpass()"));
        assert!(!production.contains(reading_constructor));
    }
}
