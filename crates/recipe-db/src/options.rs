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
mod tests;
