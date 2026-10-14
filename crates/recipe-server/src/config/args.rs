//! The hand-written parser of the command line (design D15, D17).
//!
//! Three subcommands with at most three options need no parser crate.

use std::path::PathBuf;

use anyhow::{Result, anyhow, bail};
use recipe_domain::identity::Login;

/// The usage text that an argument error appends to its message.
const USAGE: &str = "usage: recipe-server migrate [--config <path>] | recipe-server serve | \
recipe-server create-admin --login <login> [--password-file <path>] [--config <path>]";

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
    /// Create the first administrator.
    CreateAdmin {
        /// The login of the new administrator (`--login`).
        login: Login,
        /// The file with the password (`--password-file`). Without it, the password comes from stdin.
        password_file: Option<PathBuf>,
        /// The path of the TOML file (`--config`).
        config: Option<PathBuf>,
    },
}

/// Parse the arguments after the program name.
///
/// # Errors
///
/// Returns an error for a missing subcommand, an unknown subcommand, an unknown option, an
/// option without a value, a repeated option, a missing `--login`, and a login that breaks the
/// grammar.
pub fn parse(args: &[String]) -> Result<Command> {
    let mut words = args.iter().zip(1_usize..);
    let Some((subcommand, position)) = words.next() else {
        bail!("missing subcommand. {USAGE}");
    };
    match subcommand.as_str() {
        "migrate" => parse_migrate(words),
        "serve" => no_options(words, Command::Serve),
        "create-admin" => parse_create_admin(words),
        _unknown => Err(not_known(position)),
    }
}

/// The error for an argument that the parser does not know.
///
/// The text names only the position. An unknown argument can be a password that the operator
/// typed by mistake, and `main` writes the error to the log (`reliability.md` section 3.4).
fn not_known(position: usize) -> anyhow::Error {
    anyhow!("argument {position} is not known. {USAGE}")
}

/// The arguments after the subcommand, each with its 1-based position.
type Words<'a> = std::iter::Zip<std::slice::Iter<'a, String>, std::ops::RangeFrom<usize>>;

fn no_options(mut words: Words<'_>, command: Command) -> Result<Command> {
    match words.next() {
        Some((_option, position)) => Err(not_known(position)),
        None => Ok(command),
    }
}

/// Store the value of a named option once.
fn take_value(name: &str, slot: &mut Option<String>, words: &mut Words<'_>) -> Result<()> {
    let Some((value, _position)) = words.next() else {
        bail!("the option {name} needs a value. {USAGE}");
    };
    if slot.replace(value.clone()).is_some() {
        bail!("the option {name} appears twice. {USAGE}");
    }
    Ok(())
}

fn parse_migrate(mut words: Words<'_>) -> Result<Command> {
    let mut config = None;
    while let Some((word, position)) = words.next() {
        if word != "--config" {
            return Err(not_known(position));
        }
        take_value("--config", &mut config, &mut words)?;
    }
    Ok(Command::Migrate {
        config: config.map(PathBuf::from),
    })
}

fn parse_create_admin(mut words: Words<'_>) -> Result<Command> {
    let mut login = None;
    let mut password_file = None;
    let mut config = None;
    while let Some((word, position)) = words.next() {
        match word.as_str() {
            "--login" => take_value("--login", &mut login, &mut words)?,
            "--password-file" => take_value("--password-file", &mut password_file, &mut words)?,
            "--config" => take_value("--config", &mut config, &mut words)?,
            _unknown => return Err(not_known(position)),
        }
    }
    let Some(login) = login else {
        bail!("the option --login is required. {USAGE}");
    };
    let login = Login::parse(&login)
        .map_err(|reason| anyhow!("the option --login is not valid: {reason}. {USAGE}"))?;
    Ok(Command::CreateAdmin {
        login,
        password_file: password_file.map(PathBuf::from),
        config: config.map(PathBuf::from),
    })
}
