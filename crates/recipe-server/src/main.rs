//! Binary of the recipe editor.
//!
//! This crate loads the configuration and connects the adapters to the use cases.
//! It runs the subcommands `serve`, `migrate`, and `create-admin`.
//! It is the only crate that uses `anyhow` (Assumption A37).
//! This build runs `migrate` only.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/reliability.md` section 2.

#![forbid(unsafe_code)]

mod config;
mod create_admin;

use anyhow::{Context, Result, bail};
use std::io::IsTerminal;

use config::args::{self, Command};
use create_admin::{CreateAdminInput, StdinInput, read_password};
use recipe_app::identity::BootstrapOutcome;
use recipe_db::settings::MigrateSettings;
use recipe_domain::identity::Login;

/// Runs the subcommand of the command line.
///
/// The function starts the JSON logger once. When the subcommand fails, it writes exactly one
/// `ERROR` event with the chain of causes and returns the error, so the process exits with
/// code 1. The chain never holds a secret.
///
/// # Errors
///
/// Returns the error of the logger, of the argument parser, or of the subcommand.
fn main() -> Result<()> {
    init_logging()?;
    let result = run();
    if let Err(error) = &result {
        let chain = format!("{error:#}");
        tracing::error!(error = chain.as_str(), "the command failed");
    }
    result
}

/// Install the JSON formatter of `tracing-subscriber`. Output goes to stdout (`reliability.md` 3.1).
fn init_logging() -> Result<()> {
    tracing_subscriber::fmt()
        .json()
        .with_max_level(tracing::Level::INFO)
        .with_writer(std::io::stdout)
        .try_init()
        .map_err(|error| anyhow::anyhow!("cannot start the logger: {error}"))
}

/// Parse the arguments and run the subcommand.
fn run() -> Result<()> {
    let arguments = std::env::args_os()
        .skip(1)
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| anyhow::anyhow!("an argument is not valid UTF-8"))
        })
        .collect::<Result<Vec<String>>>()?;
    match args::parse(&arguments)? {
        Command::Migrate { config } => migrate(config.as_deref()),
        Command::Serve => bail!("the subcommand serve is not available in this build"),
        Command::CreateAdmin {
            login,
            password_file,
            config,
        } => create_admin(login, password_file.as_deref(), config.as_deref()),
    }
}

/// Read the configuration and the secrets for `create-admin`, then create the administrator.
///
/// The settings, the password, and the secret file are read before the runtime and before any
/// connection.
fn create_admin(
    login: Login,
    password_file: Option<&std::path::Path>,
    config_file: Option<&std::path::Path>,
) -> Result<()> {
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
    match runtime.block_on(create_admin::create(&target, input))? {
        BootstrapOutcome::Created => {
            tracing::info!(login = %login, "the administrator is created");
        }
        BootstrapOutcome::AdministratorExists => {
            tracing::info!("an active administrator exists, nothing is created");
        }
    }
    Ok(())
}

/// Apply the migrations of `recipe-db` as `recipe_migrator`.
///
/// The settings and the secret file are read before the runtime and before any connection.
fn migrate(file: Option<&std::path::Path>) -> Result<()> {
    let settings = config::load(file)?;
    let target = config::migrator_connection(&settings)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot start the async runtime")?;
    runtime
        .block_on(recipe_db::migrate(&target, &MigrateSettings::default()))
        .context("the migration failed")?;
    tracing::info!("the migrations are applied");
    Ok(())
}
