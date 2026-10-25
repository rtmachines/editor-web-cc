//! The subcommand `create-admin`: the password source, the input, and the run (ADR-0035 (h)).
//!
//! The password comes from `--password-file` or from one line of stdin. Both sources follow the
//! secret-file rules of ADR-0031. The text then goes through `NewPassword::parse`, so a policy
//! failure gives `E-IDN-006`.

use std::fmt;
use std::io::{BufRead, IsTerminal, Read};
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow};
use recipe_app::identity::{
    Argon2Hasher, BootstrapOutcome, HashGate, IdentitySettings, bootstrap_administrator,
    new_request_id,
};
use recipe_db::Database;
use recipe_db::settings::{Connection, PoolSettings};
use recipe_domain::identity::{IdentityError, Login, NewPassword};

use crate::coded::coded;
use crate::config::{self, secret_line};

/// The most bytes that the subcommand reads from stdin. A password has at most 128 characters.
const STDIN_LIMIT_BYTES: u64 = 1024;

/// The stdin of the process with the answer to "is this a terminal".
///
/// The type solves one problem: a test must inject the text of stdin and the terminal check.
/// The binary passes the locked stdin and `IsTerminal::is_terminal`.
pub struct StdinInput<R> {
    reader: R,
    is_terminal: bool,
}

impl<R: BufRead> StdinInput<R> {
    /// Wrap a reader and the result of the terminal check.
    pub const fn new(reader: R, is_terminal: bool) -> Self {
        Self {
            reader,
            is_terminal,
        }
    }

    /// Read all of stdin. The subcommand refuses a terminal, because the password would show.
    fn read_text(&mut self) -> Result<String> {
        if self.is_terminal {
            return Err(anyhow!(
                "stdin is a terminal. Pipe the password into the command or use --password-file"
            ));
        }
        read_bounded(&mut self.reader, "stdin")
    }
}

/// Read at most [`STDIN_LIMIT_BYTES`] bytes of `source` as text.
///
/// The function solves one problem: a source that is too long must not fill the memory. It reads
/// one byte more than the limit, so it can tell "exactly the limit" from "too long".
fn read_bounded(source: impl Read, name: &str) -> Result<String> {
    let mut text = String::new();
    source
        .take(STDIN_LIMIT_BYTES.saturating_add(1))
        .read_to_string(&mut text)
        .with_context(|| format!("cannot read the password from {name}"))?;
    let length = u64::try_from(text.len()).unwrap_or(u64::MAX);
    if length > STDIN_LIMIT_BYTES {
        return Err(anyhow!(
            "{name} holds more than {STDIN_LIMIT_BYTES} bytes. That is too long for a password"
        ));
    }
    Ok(text)
}

/// The checked input of `create-admin`.
///
/// `Debug` shows the login and hides the password.
pub struct CreateAdminInput {
    /// The login of the new administrator.
    pub login: Login,
    /// The password that meets the policy of Assumption A23.
    pub password: NewPassword,
}

impl fmt::Debug for CreateAdminInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CreateAdminInput")
            .field("login", &self.login)
            .field("password", &format_args!("[redacted]"))
            .finish()
    }
}

/// Read the password from `file`, else from stdin, and check it against the policy.
///
/// # Errors
///
/// Returns an error when the source cannot be read, when stdin is a terminal, when the text
/// breaks the secret rule of ADR-0031, or when it breaks the policy (`E-IDN-006`). No error
/// holds the text.
pub fn read_password(
    file: Option<&Path>,
    stdin: &mut StdinInput<impl BufRead>,
) -> Result<NewPassword> {
    let (text, origin) = match file {
        Some(path) => {
            let origin = format!("the password file {}", path.display());
            let opened = std::fs::File::open(path)
                .with_context(|| format!("cannot read the password file {}", path.display()))?;
            (read_bounded(opened, &origin)?, origin)
        }
        None => (stdin.read_text()?, "stdin".to_owned()),
    };
    let line = secret_line(&text).map_err(|fault| anyhow!("{origin} {}", fault.describe()))?;
    NewPassword::parse(line).map_err(|reason| coded(IdentityError::from(reason)))
}

/// Connect as the application role and create the first administrator.
///
/// The hash runs under a `HashGate` with the defaults of `reliability.md` section 7.11. Every
/// error carries its error code in the message, because `main` writes it as the one `ERROR` line.
///
/// # Errors
///
/// Returns the error of the connection, the hash, or the transaction, with its code.
pub async fn create(target: &Connection, input: CreateAdminInput) -> Result<BootstrapOutcome> {
    let settings = IdentitySettings::default();
    let hasher = Argon2Hasher::new(settings.argon2).map_err(coded)?;
    let gate = HashGate::new(
        settings.argon2_permits,
        settings.argon2_acquire_timeout,
        Arc::new(hasher),
    );
    let database = Database::connect(target, &PoolSettings::default())
        .await
        .map_err(coded)?;
    let request_id = new_request_id().map_err(coded)?;
    bootstrap_administrator(
        database.unit_of_work().as_ref(),
        &gate,
        input.login,
        input.password,
        request_id,
    )
    .await
    .map_err(coded)
}

/// Read the configuration and the secrets for `create-admin`, then create the administrator.
///
/// The function reads the settings, the password, and the secret file of the connection before it
/// starts the runtime and opens a connection.
///
/// # Errors
///
/// Returns the error of the configuration, of the password source, of the runtime, or of the
/// creation.
pub fn run(login: Login, password_file: Option<&Path>, config_file: Option<&Path>) -> Result<()> {
    let settings = config::load(config_file)?;
    let stdin = std::io::stdin();
    let mut input = StdinInput::new(stdin.lock(), stdin.is_terminal());
    let password = read_password(password_file, &mut input)?;
    let target = config::app_connection(&settings)?;
    let input = CreateAdminInput { login, password };
    let login = input.login.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("cannot start the async runtime")?;
    match runtime.block_on(create(&target, input))? {
        BootstrapOutcome::Created => {
            tracing::info!(login = %login, "the administrator is created");
        }
        BootstrapOutcome::AdministratorExists => {
            tracing::info!("an active administrator exists, nothing is created");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
