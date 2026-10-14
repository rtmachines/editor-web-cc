//! The command line and the configuration of the binary (ADR-0031).
//!
//! `figment` merges the optional TOML file and the environment values. The prefix is `RECIPE_`
//! and a double underscore separates the parts of a key, for example `RECIPE_DB__HOST`. An
//! environment value overrides a file value. The defaults equal `reliability.md` section 7.11.
//!
//! The settings `migrate.*` do not come from the file in this step. The subcommand uses
//! `MigrateSettings::default()`, which equals the table of section 7.11.

pub mod args;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use figment::providers::{Env, Format, Toml};
use figment::{Figment, Provider};
use recipe_db::settings::{Connection, Password};
use serde::Deserialize;

const DEFAULT_HOST: &str = "db";
const DEFAULT_PORT: u16 = 5432;
const DEFAULT_DATABASE: &str = "recipe";
const DEFAULT_MIGRATOR_USER: &str = "recipe_migrator";
const DEFAULT_APP_USER: &str = "recipe_app";
const DEFAULT_MIGRATOR_PASSWORD_FILE: &str = "/run/secrets/migrator_password";
const DEFAULT_APP_PASSWORD_FILE: &str = "/run/secrets/app_password";
const ENV_PREFIX: &str = "RECIPE_";
const ENV_SEPARATOR: &str = "__";
/// The environment names without the prefix, one for each key of the settings.
const ENV_KEYS: [&str; 7] = [
    "db__host",
    "db__port",
    "db__name",
    "db__migrator_user",
    "db__app_user",
    "secrets__migrator_password_file",
    "secrets__app_password_file",
];

/// The keys of the section `db`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbSettings {
    /// The host name of the database server (`db.host`).
    pub host: String,
    /// The TCP port of the database server (`db.port`).
    pub port: u16,
    /// The name of the database (`db.name`).
    pub name: String,
    /// The login role of the migrator (`db.migrator_user`).
    pub migrator_user: String,
    /// The login role of the application and of `create-admin` (`db.app_user`).
    pub app_user: String,
}

impl Default for DbSettings {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_owned(),
            port: DEFAULT_PORT,
            name: DEFAULT_DATABASE.to_owned(),
            migrator_user: DEFAULT_MIGRATOR_USER.to_owned(),
            app_user: DEFAULT_APP_USER.to_owned(),
        }
    }
}

/// The keys of the section `secrets`. A key holds a path and never a password.
#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct SecretSettings {
    /// The file with the password of the migrator (`secrets.migrator_password_file`).
    pub migrator_password_file: PathBuf,
    /// The file with the password of the application role (`secrets.app_password_file`).
    pub app_password_file: PathBuf,
}

impl Default for SecretSettings {
    fn default() -> Self {
        Self {
            migrator_password_file: PathBuf::from(DEFAULT_MIGRATOR_PASSWORD_FILE),
            app_password_file: PathBuf::from(DEFAULT_APP_PASSWORD_FILE),
        }
    }
}

/// The merged configuration of the binary.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    /// The keys `db.*`.
    pub db: DbSettings,
    /// The keys `secrets.*`.
    pub secrets: SecretSettings,
}

/// Read the optional TOML file and merge the environment values over it.
///
/// A missing or unreadable file at an explicit path is an error.
///
/// # Errors
///
/// Returns an error when the file cannot be read, when it is not valid TOML, or when a value
/// has the wrong type.
pub fn load(file: Option<&Path>) -> Result<Settings> {
    load_with(file, environment())
}

/// Build the environment provider. It reads only the known keys.
///
/// The structs of the settings reject an unknown key. Without this filter, any other variable
/// with the prefix `RECIPE_` would stop the program. `only` compares the name before the
/// split, so it lists the names with the separator.
fn environment() -> Env {
    Env::prefixed(ENV_PREFIX)
        .only(&ENV_KEYS)
        .split(ENV_SEPARATOR)
}

/// Merge the optional TOML file and then `overrides`, which wins over the file.
///
/// `load` passes the environment as `overrides`. A test passes another provider, because the
/// environment of a process is global and parallel tests would share it.
///
/// # Errors
///
/// See [`load`].
fn load_with(file: Option<&Path>, overrides: impl Provider) -> Result<Settings> {
    let mut figment = Figment::new();
    if let Some(path) = file {
        let text = std::fs::read_to_string(path)
            .with_context(|| format!("cannot read the configuration file {}", path.display()))?;
        figment = figment.merge(Toml::string(&text));
    }
    figment
        .merge(overrides)
        .extract()
        .context("the configuration is not valid")
}

/// Build the connection of the migrator and read its password from the secret file.
///
/// The function reads the file before any connection exists. The password never appears in
/// an error. The error names the key and the path only.
///
/// # Errors
///
/// Returns an error when the secret file cannot be read, is not UTF-8, or breaks the secret rule
/// of ADR-0031: it is empty, holds a carriage return, or holds more than one line.
pub fn migrator_connection(settings: &Settings) -> Result<Connection> {
    connection(
        settings,
        &settings.db.migrator_user,
        &settings.secrets.migrator_password_file,
        "secrets.migrator_password_file",
    )
}

/// Build the connection of the application role and read its password from the secret file.
///
/// `create-admin` uses it. The rules are those of [`migrator_connection`], with the key
/// `secrets.app_password_file`.
///
/// # Errors
///
/// See [`migrator_connection`].
pub fn app_connection(settings: &Settings) -> Result<Connection> {
    connection(
        settings,
        &settings.db.app_user,
        &settings.secrets.app_password_file,
        "secrets.app_password_file",
    )
}

/// Read one secret file and build the connection of `user`. `key` names the setting in errors.
fn connection(settings: &Settings, user: &str, path: &Path, key: &str) -> Result<Connection> {
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read the file of {key} {}", path.display()))?;
    let secret = secret_line(&text)
        .map_err(|fault| anyhow!("the file of {key} {} {}", path.display(), fault.describe()))?;
    Ok(Connection {
        host: settings.db.host.clone(),
        port: settings.db.port,
        database: settings.db.name.clone(),
        user: user.to_owned(),
        password: Password::from(secret.to_owned()),
    })
}

/// The reason why the text of a secret file is not one valid line (ADR-0031).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretFault {
    /// The file holds no characters, or only one line feed.
    Empty,
    /// The file holds a carriage return anywhere.
    CarriageReturn,
    /// The file holds a line feed before its last character.
    SeveralLines,
}

impl SecretFault {
    /// Return the words that end the error message. They never show the content of the file.
    pub const fn describe(self) -> &'static str {
        match self {
            Self::Empty => "is empty",
            Self::CarriageReturn => "holds a carriage return",
            Self::SeveralLines => "holds more than one line",
        }
    }
}

/// Return the one line of a secret file.
///
/// The rule of ADR-0031: the file holds exactly one non-empty line and no carriage return.
/// One trailing line feed is allowed and removed. `01-roles.sh` reads the same rule.
pub fn secret_line(text: &str) -> Result<&str, SecretFault> {
    if text.contains('\r') {
        return Err(SecretFault::CarriageReturn);
    }
    let line = text.strip_suffix('\n').unwrap_or(text);
    if line.contains('\n') {
        return Err(SecretFault::SeveralLines);
    }
    if line.is_empty() {
        return Err(SecretFault::Empty);
    }
    Ok(line)
}

#[cfg(test)]
mod tests;
