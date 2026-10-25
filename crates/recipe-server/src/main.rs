//! Binary of the recipe editor.
//!
//! The binary parses the arguments and calls the library `recipe_server`.
//! It runs the subcommands `serve`, `migrate`, `create-admin`, and `healthcheck`.
//! The subcommand `serve` runs the start, waits for the stop signal, and stops the server.
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/reliability.md` section 2.

#![forbid(unsafe_code)]

use anyhow::Result;
use recipe_app::operations::AlertCounters;
use recipe_server::config::args::{self, Command};
use recipe_server::{create_admin, logging, migrate, panic_hook, pg_env, probe, serve};

/// Runs the subcommand of the command line.
///
/// The function starts the JSON logger once. When `RUST_LOG` is not valid, it writes exactly one
/// `ERROR` event through the fallback logger. When the subcommand fails, it writes exactly one
/// `ERROR` event with the chain of causes. In both cases it returns the error, so the process
/// exits with code 1. The chain never holds a secret.
///
/// # Errors
///
/// Returns the error of the logger, of the argument parser, or of the subcommand.
fn main() -> Result<()> {
    if let Err(error) = logging::init_logging() {
        logging::fallback_error(std::io::stdout, &format!("{error:#}"));
        return Err(error);
    }
    let counters = AlertCounters::default();
    panic_hook::install(counters.clone());
    let result = run(counters);
    if let Err(error) = &result {
        let chain = format!("{error:#}");
        tracing::error!(error = chain.as_str(), "the command failed");
    }
    result
}

/// Parse the arguments and run the subcommand.
fn run(counters: AlertCounters) -> Result<()> {
    let arguments = std::env::args_os()
        .skip(1)
        .map(|argument| {
            argument
                .into_string()
                .map_err(|_| anyhow::anyhow!("an argument is not valid UTF-8"))
        })
        .collect::<Result<Vec<String>>>()?;
    let command = args::parse(&arguments)?;
    refuse_environment_values(&command)?;
    match command {
        Command::Migrate { config } => migrate::run(config.as_deref()),
        Command::Serve => serve::run(counters),
        Command::CreateAdmin {
            login,
            password_file,
            config,
        } => create_admin::run(login, password_file.as_deref(), config.as_deref()),
        Command::Healthcheck { path } => probe::run_healthcheck(&path),
    }
}

/// Refuse a `PG*` variable for each subcommand that opens a database connection (ADR-0069 (b)).
///
/// The check runs directly after the logger starts and before every other step. The match has no
/// wildcard arm, so a new subcommand must say whether it opens a connection. `healthcheck` opens
/// none and gets no check (ADR-0069 (c)).
fn refuse_environment_values(command: &Command) -> Result<()> {
    match command {
        Command::Migrate { .. } | Command::Serve | Command::CreateAdmin { .. } => {
            pg_env::refuse_pg_variables()
        }
        Command::Healthcheck { .. } => Ok(()),
    }
}
