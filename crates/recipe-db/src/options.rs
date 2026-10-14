//! Connection options shared by the migrator and the pool.

use sqlx::ConnectOptions as _;
use sqlx::postgres::PgConnectOptions;

use crate::settings::Connection;

/// Build the connection options of a target.
///
/// The function solves one problem: the migrator and the pool must never log the text of a
/// statement. A statement in a log line can hold a secret. The options therefore disable
/// statement logging.
pub(crate) fn connect_options(target: &Connection) -> PgConnectOptions {
    PgConnectOptions::new()
        .host(&target.host)
        .port(target.port)
        .database(&target.database)
        .username(&target.user)
        .password(target.password.expose())
        .disable_statement_logging()
}
