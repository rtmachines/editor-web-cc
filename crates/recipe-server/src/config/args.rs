//! The hand-written parser of the command line (design D15).
//!
//! Three subcommands with at most one option need no parser crate.

use std::path::PathBuf;

use anyhow::{Result, bail};

/// The usage text that an argument error appends to its message.
const USAGE: &str = "usage: recipe-server migrate [--config <path>]";

/// One subcommand of the binary with its options.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    /// Apply the migrations of `recipe-db`. The file is optional because the environment can
    /// hold every value.
    Migrate {
        /// The path of the TOML file (`--config`).
        config: Option<PathBuf>,
    },
    /// Start the web server. This build does not run it.
    Serve,
    /// Create the first administrator. This build does not run it.
    CreateAdmin,
}

/// Parse the arguments after the program name.
///
/// # Errors
///
/// Returns an error for a missing subcommand, an unknown subcommand, an unknown option, an
/// option without a value, and a repeated option.
pub fn parse(args: &[String]) -> Result<Command> {
    let mut words = args.iter();
    let Some(subcommand) = words.next() else {
        bail!("missing subcommand. {USAGE}");
    };
    match subcommand.as_str() {
        "migrate" => parse_migrate(words),
        "serve" => no_options(words, Command::Serve),
        "create-admin" => no_options(words, Command::CreateAdmin),
        other => bail!("unknown subcommand {other}. {USAGE}"),
    }
}

fn no_options<'a>(
    mut words: impl Iterator<Item = &'a String>,
    command: Command,
) -> Result<Command> {
    match words.next() {
        Some(option) => bail!("unknown option {option}. {USAGE}"),
        None => Ok(command),
    }
}

fn parse_migrate<'a>(mut words: impl Iterator<Item = &'a String>) -> Result<Command> {
    let mut config = None;
    while let Some(word) = words.next() {
        if word != "--config" {
            bail!("unknown option {word}. {USAGE}");
        }
        let Some(path) = words.next() else {
            bail!("the option --config needs a path. {USAGE}");
        };
        if config.replace(PathBuf::from(path)).is_some() {
            bail!("the option --config appears twice. {USAGE}");
        }
    }
    Ok(Command::Migrate { config })
}
