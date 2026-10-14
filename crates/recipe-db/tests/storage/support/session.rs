//! Plain connections to the test container.

use sqlx::postgres::{PgConnectOptions, PgSslMode};
use sqlx::{ConnectOptions as _, PgConnection};

use super::container::{Role, Server};

/// Build the connection options of a role for one database of the container.
pub fn options(server: &Server, role: Role, database: &str) -> PgConnectOptions {
    PgConnectOptions::new()
        .host(&server.host)
        .port(server.port)
        .username(role.name())
        .password(role.password())
        .database(database)
        .ssl_mode(PgSslMode::Disable)
}

/// Open a normal session as a role. The call fails the test when the server rejects it.
pub async fn connect(server: &Server, role: Role, database: &str) -> PgConnection {
    options(server, role, database)
        .connect()
        .await
        .map_err(|error| format!("{} cannot connect to {database}: {error}", role.name()))
        .expect("the session opens")
}

/// Try to open a normal session and return the SQLSTATE of the rejection.
///
/// The call fails the test when the connection succeeds or when the error has no SQLSTATE.
pub async fn connect_error(server: &Server, role: Role, database: &str) -> String {
    let result = options(server, role, database).connect().await;
    let error = result.expect_err("the server rejects the session");
    let sqlx::Error::Database(database_error) = error else {
        return format!("not a database error: {error}");
    };
    database_error
        .code()
        .map(|code| code.into_owned())
        .unwrap_or_default()
}
