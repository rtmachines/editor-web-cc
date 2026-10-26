//! The operator keys of `serve` (ADR-0062, design D7).
//!
//! `serve` reads 17 keys from the environment. The 18th key, `alerts.webhook_url`, is derived
//! from the webhook file (ADR-0063). The function [`load`] reads and checks every key, both key
//! files, and the webhook file. It does this before the server opens a port. A refusal
//! for a key file or the webhook file names the key only. It never shows a secret value,
//! a file content, a path, or the webhook URL.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, anyhow, bail};
use figment::providers::{Env, Serialized};
use figment::{Figment, Provider};
use recipe_app::operations::ConfiguredSettings;
use recipe_db::settings::Connection;
use recipe_domain::identity::KeyRing;
use recipe_domain::recipe::LangCode;
use recipe_domain::setting::SettingValue;
use recipe_web::CidrList;
use serde::Deserialize;

use super::key_file::read_key_file;
use super::webhook::{WebhookSource, WebhookUrl, read_webhook_file};
use super::{DbSettings, ENV_PREFIX, ENV_SEPARATOR, SecretSettings, Settings, app_connection};
use crate::port::{DEFAULT_INTERNAL_PORT, deserialize_port};

/// The two keys whose environment text must reach `parse_port` unchanged.
const PORT_KEYS: [&str; 2] = ["http.public_port", "http.internal_port"];
const KEY_LOGIN_HMAC: &str = "security.login_hmac_key_file";
const KEY_CSRF_SIGNING: &str = "security.csrf_signing_key_file";
const KEY_WEBHOOK_URL_FILE: &str = "alerts.webhook_url_file";
const KEY_WEBHOOK_CA_FILE: &str = "alerts.webhook_ca_file";
const KEY_TRUSTED_PROXIES: &str = "http.trusted_proxies";
const KEY_TRANSLATION_LANGS: &str = "i18n.translation_langs";
const KEY_IDLE_TIMEOUT: &str = "session.idle_timeout";
const KEY_ABSOLUTE_TIMEOUT: &str = "session.absolute_timeout";

const DEFAULT_LOGIN_HMAC_KEY_FILE: &str = "/run/secrets/login_hmac_key";
const DEFAULT_CSRF_SIGNING_KEY_FILE: &str = "/run/secrets/csrf_signing_key";
const DEFAULT_WEBHOOK_URL_FILE: &str = "/run/secrets/alerts_webhook_url";
const DEFAULT_TRUSTED_PROXIES: &str = "172.28.0.2/32";
const DEFAULT_PUBLIC_PORT: u16 = 8080;
const DEFAULT_IDLE_TIMEOUT_SECONDS: u64 = 30 * 60;
const DEFAULT_ABSOLUTE_TIMEOUT_SECONDS: u64 = 12 * 60 * 60;
const DEFAULT_TRANSLATION_LANGS: &str = "pl";

/// Whether this build is a development build.
///
/// Only a build with the feature `dev` is a development build (ADR-0062 (b)).
/// Without the feature, `session.secure=false` stops the start.
const DEV_BUILD: bool = cfg!(feature = "dev");

/// The environment names without the prefix, one for each key that an operator can set.
///
/// The key `alerts.webhook_url` has no name, because ADR-0063 derives it from the file.
pub const SERVE_ENV_KEYS: [&str; 17] = [
    "db__host",
    "db__port",
    "db__name",
    "db__app_user",
    "secrets__app_password_file",
    "security__login_hmac_key_file",
    "security__csrf_signing_key_file",
    "http__trusted_proxies",
    "http__public_port",
    "http__internal_port",
    "approval__separation_of_duties",
    "alerts__webhook_url_file",
    "alerts__webhook_ca_file",
    "session__secure",
    "session__idle_timeout",
    "session__absolute_timeout",
    "i18n__translation_langs",
];

/// The unchecked keys of `serve`, as the environment gives them.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawServe {
    db: RawDb,
    secrets: RawSecrets,
    security: RawSecurity,
    http: RawHttp,
    approval: RawApproval,
    alerts: RawAlerts,
    session: RawSession,
    i18n: RawI18n,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawDb {
    host: String,
    port: u16,
    name: String,
    app_user: String,
}

impl Default for RawDb {
    fn default() -> Self {
        let db = DbSettings::default();
        Self {
            host: db.host,
            port: db.port,
            name: db.name,
            app_user: db.app_user,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawSecrets {
    app_password_file: String,
}

impl Default for RawSecrets {
    fn default() -> Self {
        let secrets = SecretSettings::default();
        Self {
            app_password_file: secrets.app_password_file.to_string_lossy().into_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawSecurity {
    login_hmac_key_file: String,
    csrf_signing_key_file: String,
}

impl Default for RawSecurity {
    fn default() -> Self {
        Self {
            login_hmac_key_file: DEFAULT_LOGIN_HMAC_KEY_FILE.to_owned(),
            csrf_signing_key_file: DEFAULT_CSRF_SIGNING_KEY_FILE.to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawHttp {
    trusted_proxies: String,
    #[serde(deserialize_with = "deserialize_port")]
    public_port: u16,
    #[serde(deserialize_with = "deserialize_port")]
    internal_port: u16,
}

impl Default for RawHttp {
    fn default() -> Self {
        Self {
            trusted_proxies: DEFAULT_TRUSTED_PROXIES.to_owned(),
            public_port: DEFAULT_PUBLIC_PORT,
            internal_port: DEFAULT_INTERNAL_PORT,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawApproval {
    separation_of_duties: bool,
}

impl Default for RawApproval {
    fn default() -> Self {
        Self {
            separation_of_duties: true,
        }
    }
}

/// The two file keys of the alerts. `None` means that the operator did not set the key.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawAlerts {
    webhook_url_file: Option<String>,
    webhook_ca_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawSession {
    secure: bool,
    idle_timeout: u64,
    absolute_timeout: u64,
}

impl Default for RawSession {
    fn default() -> Self {
        Self {
            secure: true,
            idle_timeout: DEFAULT_IDLE_TIMEOUT_SECONDS,
            absolute_timeout: DEFAULT_ABSOLUTE_TIMEOUT_SECONDS,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct RawI18n {
    translation_langs: String,
}

impl Default for RawI18n {
    fn default() -> Self {
        Self {
            translation_langs: DEFAULT_TRANSLATION_LANGS.to_owned(),
        }
    }
}

/// The checked configuration of `serve`.
///
/// The type solves one problem: `serve` must hold only values that passed every check, so the
/// later steps of the start order need no second check. `Debug` shows paths and numbers. It
/// shows no key, no password, and no webhook URL, because each of those types redacts itself.
#[derive(Debug, Clone)]
pub struct ServeConfig {
    configured: ConfiguredSettings,
    trusted_proxies: CidrList,
    translation_langs: Vec<LangCode>,
    login_keys: KeyRing,
    csrf_keys: KeyRing,
    webhook: Option<WebhookUrl>,
    connection_settings: Settings,
}

impl ServeConfig {
    /// Return the 18 values that `CompareSettings` records in `app_setting`.
    #[must_use]
    pub const fn configured(&self) -> &ConfiguredSettings {
        &self.configured
    }

    /// Return the networks whose `X-Forwarded-For` header counts (`http.trusted_proxies`).
    #[must_use]
    pub const fn trusted_proxies(&self) -> &CidrList {
        &self.trusted_proxies
    }

    /// Return the languages of the translation rows (`i18n.translation_langs`).
    #[must_use]
    pub fn translation_langs(&self) -> &[LangCode] {
        &self.translation_langs
    }

    /// Return the keys of the login HMAC. The first key is the newest.
    #[must_use]
    pub const fn login_keys(&self) -> &KeyRing {
        &self.login_keys
    }

    /// Return the signing keys of the CSRF cookie. The first key is the newest.
    #[must_use]
    pub const fn csrf_keys(&self) -> &KeyRing {
        &self.csrf_keys
    }

    /// Return the webhook URL, or `None` when the operator configured no webhook.
    #[must_use]
    pub const fn webhook_url(&self) -> Option<&WebhookUrl> {
        self.webhook.as_ref()
    }

    /// Read the password file of the application role and build its connection.
    ///
    /// # Errors
    ///
    /// Returns an error that names `secrets.app_password_file` when the file breaks the secret
    /// rule of ADR-0031. The error never shows the content.
    pub fn app_connection(&self) -> Result<Connection> {
        app_connection(&self.connection_settings)
    }
}

/// Read the keys from the environment, check them, and read the files.
///
/// # Errors
///
/// Returns an error that names the key of the first value that breaks a rule.
pub fn load() -> Result<ServeConfig> {
    let environment = Env::prefixed(ENV_PREFIX)
        .only(&SERVE_ENV_KEYS)
        .split(ENV_SEPARATOR);
    let ports = raw_port_texts(&environment);
    load_with(Figment::from(environment).merge(ports), DEV_BUILD)
}

/// Give the two port keys to the loader as the raw text of the environment.
///
/// The environment provider of figment trims a value and reads it as a number. Then `" 9000"`
/// and `"+9000"` would pass as port 9000, and `healthcheck` would refuse the same text. This
/// provider keeps the text, so `deserialize_port` and `parse_port` judge the exact characters.
fn raw_port_texts(environment: &Env) -> Figment {
    environment
        .iter()
        .filter(|(key, _text)| PORT_KEYS.contains(&key.as_str()))
        .fold(Figment::new(), |figment, (key, text)| {
            figment.merge(Serialized::default(key.as_str(), text))
        })
}

/// Check the configuration that `overrides` gives.
///
/// A test passes another provider, because the environment of a process is global.
fn load_with(overrides: impl Provider, dev_build: bool) -> Result<ServeConfig> {
    let raw: RawServe = Figment::new()
        .merge(overrides)
        .extract()
        .context("the configuration is not valid")?;
    check_secure(raw.session.secure, dev_build)?;
    check_texts(&raw)?;
    let (proxy_texts, trusted_proxies) = parse_proxies(&raw.http.trusted_proxies)?;
    let (lang_texts, translation_langs) = parse_langs(&raw.i18n.translation_langs)?;
    let idle = seconds(KEY_IDLE_TIMEOUT, raw.session.idle_timeout)?;
    let absolute = seconds(KEY_ABSOLUTE_TIMEOUT, raw.session.absolute_timeout)?;
    let login_keys = read_key_file(KEY_LOGIN_HMAC, Path::new(&raw.security.login_hmac_key_file))?;
    let csrf_keys = read_key_file(
        KEY_CSRF_SIGNING,
        Path::new(&raw.security.csrf_signing_key_file),
    )?;
    let (webhook_file, webhook) = load_webhook(&raw.alerts)?;
    check_ca_file(&raw.alerts)?;
    let connection_settings = connection_settings(&raw);
    let configured = ConfiguredSettings {
        db_host: raw.db.host,
        db_port: raw.db.port,
        db_name: raw.db.name,
        db_app_user: raw.db.app_user,
        app_password_file: raw.secrets.app_password_file,
        login_hmac_key_file: raw.security.login_hmac_key_file,
        csrf_signing_key_file: raw.security.csrf_signing_key_file,
        trusted_proxies: proxy_texts,
        public_port: raw.http.public_port,
        internal_port: raw.http.internal_port,
        separation_of_duties: raw.approval.separation_of_duties,
        webhook_url_file: webhook_file,
        webhook_ca_file: raw.alerts.webhook_ca_file.filter(|path| !path.is_empty()),
        webhook_url: webhook.as_ref().map(WebhookUrl::to_url_text),
        session_secure: raw.session.secure,
        idle_timeout: idle,
        absolute_timeout: absolute,
        translation_langs: lang_texts,
    };
    Ok(ServeConfig {
        configured,
        trusted_proxies,
        translation_langs,
        login_keys,
        csrf_keys,
        webhook,
        connection_settings,
    })
}

/// Refuse `session.secure = false` in a build without the feature `dev` (ADR-0062 (b)).
fn check_secure(secure: bool, dev_build: bool) -> Result<()> {
    if !secure && !dev_build {
        bail!("the value of session.secure is false, and only a development build accepts it");
    }
    Ok(())
}

/// Refuse a text that the column `app_setting.value` cannot hold (1000 characters, no NUL).
fn check_text(key: &str, text: &str) -> Result<()> {
    SettingValue::parse(text)
        .map(|_value| ())
        .map_err(|reason| anyhow!("the value of {key} is not valid: {reason}"))
}

/// Check the length of every text key before the code reads a file.
fn check_texts(raw: &RawServe) -> Result<()> {
    let optional = |text: &Option<String>| text.clone().unwrap_or_default();
    let pairs = [
        ("db.host", raw.db.host.clone()),
        ("db.name", raw.db.name.clone()),
        ("db.app_user", raw.db.app_user.clone()),
        (
            "secrets.app_password_file",
            raw.secrets.app_password_file.clone(),
        ),
        (KEY_LOGIN_HMAC, raw.security.login_hmac_key_file.clone()),
        (KEY_CSRF_SIGNING, raw.security.csrf_signing_key_file.clone()),
        (KEY_TRUSTED_PROXIES, raw.http.trusted_proxies.clone()),
        (KEY_WEBHOOK_URL_FILE, optional(&raw.alerts.webhook_url_file)),
        (KEY_WEBHOOK_CA_FILE, optional(&raw.alerts.webhook_ca_file)),
        (KEY_TRANSLATION_LANGS, raw.i18n.translation_langs.clone()),
    ];
    pairs
        .iter()
        .try_for_each(|(key, text)| check_text(key, text))
}

/// Split the comma list of networks. The texts keep their order.
fn parse_proxies(text: &str) -> Result<(Vec<String>, CidrList)> {
    let items: Vec<String> = text.split(',').map(str::to_owned).collect();
    let list = CidrList::parse_all(&items).map_err(|reason| {
        anyhow!("the value of {KEY_TRUSTED_PROXIES} is not a comma list of networks: {reason}")
    })?;
    Ok((items, list))
}

/// Split the comma list of language tags. A tag appears once.
fn parse_langs(text: &str) -> Result<(Vec<String>, Vec<LangCode>)> {
    let bad =
        || anyhow!("the value of {KEY_TRANSLATION_LANGS} is not a comma list of language tags");
    let mut codes: Vec<LangCode> = Vec::new();
    for item in text.split(',') {
        let code = LangCode::parse(item).map_err(|_reason| bad())?;
        if codes.contains(&code) {
            return Err(bad());
        }
        codes.push(code);
    }
    let items = codes.iter().map(ToString::to_string).collect();
    Ok((items, codes))
}

/// Turn whole seconds into a duration. A timeout of zero ends every session at once.
fn seconds(key: &str, value: u64) -> Result<Duration> {
    if value == 0 {
        bail!("the value of {key} must be at least 1 second");
    }
    Ok(Duration::from_secs(value))
}

/// Resolve the webhook file key and read the file.
///
/// Returns the canonical file path of the key and the URL. An unset key has the default path.
/// A missing file at the default path means no webhook. An empty key means no webhook.
fn load_webhook(alerts: &RawAlerts) -> Result<(Option<String>, Option<WebhookUrl>)> {
    let (path, source) = match alerts.webhook_url_file.as_deref() {
        None => (DEFAULT_WEBHOOK_URL_FILE, WebhookSource::Default),
        Some("") => {
            tracing::info!(
                key = KEY_WEBHOOK_URL_FILE,
                "the key is empty: alerts have no webhook"
            );
            return Ok((None, None));
        }
        Some(path) => (path, WebhookSource::Set),
    };
    let url = read_webhook_file(KEY_WEBHOOK_URL_FILE, Path::new(path), source)?;
    Ok((Some(path.to_owned()), url))
}

/// Check that a set CA file can be read. The connector of task 7.7 parses the certificates.
fn check_ca_file(alerts: &RawAlerts) -> Result<()> {
    let Some(path) = alerts
        .webhook_ca_file
        .as_deref()
        .filter(|path| !path.is_empty())
    else {
        return Ok(());
    };
    std::fs::read(path).map(|_bytes| ()).map_err(|fault| {
        anyhow!(
            "the file of {KEY_WEBHOOK_CA_FILE} cannot be read: {}",
            fault.kind()
        )
    })
}

/// Build the settings that `app_connection` of the parent module needs.
fn connection_settings(raw: &RawServe) -> Settings {
    Settings {
        db: DbSettings {
            host: raw.db.host.clone(),
            port: raw.db.port,
            name: raw.db.name.clone(),
            app_user: raw.db.app_user.clone(),
            ..DbSettings::default()
        },
        secrets: SecretSettings {
            app_password_file: PathBuf::from(&raw.secrets.app_password_file),
            ..SecretSettings::default()
        },
    }
}

#[cfg(test)]
mod file_tests;
#[cfg(test)]
mod log_tests;
#[cfg(test)]
mod tests;
