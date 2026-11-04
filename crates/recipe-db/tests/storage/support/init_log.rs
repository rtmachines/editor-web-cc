//! The server log of a fresh container that logs every statement (ADR-0123, design D1).
//!
//! The container has its own label value, so the cleanup of the harness never removes it.
//! The harness container migrates its template database once per binary and cannot serve here.

use sqlx::ConnectOptions as _;

use super::container::remove_old_containers;
use super::container::{
    LOG_EVERY_STATEMENT, LOG_LABEL_VALUE, Role, start_container, wait_for_login,
};
use super::session::options;

/// What the log test reads from the container after the init ended.
#[derive(Debug)]
pub struct InitLog {
    /// The names of the roles from the list that exist in `pg_roles`, sorted.
    pub roles: Vec<String>,
    /// The stdout and the stderr of the container as text.
    pub text: String,
    /// The stdout of the container only. The temporary server of the init writes its log there.
    pub init_text: String,
}

/// The five roles that `01-roles.sh` and the image create.
pub fn expected_roles() -> Vec<&'static str> {
    let mut names: Vec<&'static str> = Role::WITH_SECRET_FILE
        .iter()
        .map(|role| role.name())
        .collect();
    names.push(Role::Superuser.name());
    names.sort_unstable();
    names
}

/// Start a fresh container, wait until the init ends, and read its log.
///
/// The call removes the container before it returns, also when a step fails.
pub async fn read_init_log() -> Result<InitLog, String> {
    remove_old_containers(LOG_LABEL_VALUE);
    let (server, container) = start_container(LOG_LABEL_VALUE, &LOG_EVERY_STATEMENT).await?;
    let outcome = async {
        // The temporary server of the init accepts no TCP login, so this login proves that the
        // init scripts are done.
        wait_for_login(&server, &container).await?;
        let mut admin = options(&server, Role::Superuser, "postgres")
            .connect()
            .await
            .map_err(|error| format!("The superuser cannot connect: {error}"))?;
        let roles: Vec<String> =
            sqlx::query_scalar("SELECT rolname FROM pg_roles WHERE rolname = ANY($1) ORDER BY 1")
                .bind(expected_roles())
                .fetch_all(&mut admin)
                .await
                .map_err(|error| format!("Cannot read pg_roles: {error}"))?;
        // The temporary server of the init writes its log to the stdout of the container. The
        // final server writes it to the stderr. The text holds both streams.
        let stdout = container
            .stdout_to_vec()
            .await
            .map_err(|error| format!("Cannot read the stdout of the container: {error}"))?;
        let stderr = container
            .stderr_to_vec()
            .await
            .map_err(|error| format!("Cannot read the stderr of the container: {error}"))?;
        let init_text = String::from_utf8_lossy(&stdout).into_owned();
        Ok(InitLog {
            roles,
            text: format!(
                "{init_text}
{}",
                String::from_utf8_lossy(&stderr)
            ),
            init_text,
        })
    }
    .await;
    // A failed removal leaves a container with the label. The next run removes it.
    let _ = container.rm().await;
    outcome
}
