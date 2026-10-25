//! The internal operation `CompareSettings` (`reliability.md` section 7.12, ADR-0079).
//!
//! The server calls the operation in step 7 of the start order (ADR-0061). The operation
//! compares the 18 configuration values with the rows of `app_setting`. It writes one
//! `SettingChanged` entry for each difference. It has no permission check and no row of the
//! matrix (ADR-0060). It also runs in read-only mode (ADR-0061 (e)).

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use recipe_domain::audit::{AuditEntry, RequestId};
use recipe_domain::setting::{SettingKey, SettingValue};

use crate::error::{AppError, StorageError};
use crate::ports::{Isolation, Locked, NonEmpty, SettingRepository, SettingRow, UnitOfWork};

/// The text of the webhook URL that the server read from its secret file.
///
/// The type solves one problem: the URL is a secret, and a derived `Debug` of
/// [`ConfiguredSettings`] must not print it. The type has no `Display` and no accessor. Only the
/// comparison reads it, and it stores the fingerprint of the text (ADR-0063 (c), (d)).
#[derive(Clone)]
pub struct WebhookUrlText(String);

impl From<String> for WebhookUrlText {
    fn from(text: String) -> Self {
        Self(text)
    }
}

impl fmt::Debug for WebhookUrlText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WebhookUrlText(<redacted>)")
    }
}

/// The 18 values that `serve` records in `app_setting` (ADR-0062 (a), design D7).
///
/// The type solves one problem: the canonical text of each value must have one definition.
/// `recipe-server` fills the fields from its checked configuration. [`ConfiguredSettings::canonical`]
/// makes the text. A path is the text as the operator gave it. A `None` file and a `None` URL
/// mean that the operator set no value.
#[derive(Clone, Debug)]
pub struct ConfiguredSettings {
    /// `db.host`.
    pub db_host: String,
    /// `db.port`.
    pub db_port: u16,
    /// `db.name`.
    pub db_name: String,
    /// `db.app_user`.
    pub db_app_user: String,
    /// `secrets.app_password_file`.
    pub app_password_file: String,
    /// `security.login_hmac_key_file`.
    pub login_hmac_key_file: String,
    /// `security.csrf_signing_key_file`.
    pub csrf_signing_key_file: String,
    /// `http.trusted_proxies`. The CIDR texts in the given order.
    pub trusted_proxies: Vec<String>,
    /// `http.public_port`.
    pub public_port: u16,
    /// `http.internal_port`.
    pub internal_port: u16,
    /// `approval.separation_of_duties`.
    pub separation_of_duties: bool,
    /// `alerts.webhook_url_file`.
    pub webhook_url_file: Option<String>,
    /// `alerts.webhook_ca_file`.
    pub webhook_ca_file: Option<String>,
    /// The URL in the webhook file. The row `alerts.webhook_url` holds its fingerprint.
    /// An empty text means no URL.
    pub webhook_url: Option<WebhookUrlText>,
    /// `session.secure`.
    pub session_secure: bool,
    /// `session.idle_timeout`. The text keeps whole seconds only.
    pub idle_timeout: Duration,
    /// `session.absolute_timeout`. The text keeps whole seconds only.
    pub absolute_timeout: Duration,
    /// `i18n.translation_langs`. The language tags in the given order.
    pub translation_langs: Vec<String>,
}

impl ConfiguredSettings {
    /// Return the 18 pairs of key and canonical text, in the order of design D7.
    ///
    /// A number is decimal text. A flag is `true` or `false`. A timeout is whole seconds and `s`.
    /// A list is its items joined by `,`. An absent file is the empty text. The webhook row is the
    /// fingerprint of the URL, or the empty text with no URL.
    /// An empty URL text counts as no URL.
    ///
    /// # Errors
    ///
    /// Returns a defect error (`E-DB-008`) when a text is not a valid setting value, for example
    /// a text with more than 1000 characters. The caller checks the values before, so this is a defect.
    pub fn canonical(&self) -> Result<Vec<(SettingKey, SettingValue)>, StorageError> {
        let webhook = self
            .webhook_url
            .as_ref()
            .filter(|url| !url.0.is_empty())
            .map_or_else(String::new, |url| {
                SettingValue::fingerprint(&url.0).to_string()
            });
        let texts: [(&'static str, String); 18] = [
            ("db.host", self.db_host.clone()),
            ("db.port", self.db_port.to_string()),
            ("db.name", self.db_name.clone()),
            ("db.app_user", self.db_app_user.clone()),
            ("secrets.app_password_file", self.app_password_file.clone()),
            (
                "security.login_hmac_key_file",
                self.login_hmac_key_file.clone(),
            ),
            (
                "security.csrf_signing_key_file",
                self.csrf_signing_key_file.clone(),
            ),
            ("http.trusted_proxies", self.trusted_proxies.join(",")),
            ("http.public_port", self.public_port.to_string()),
            ("http.internal_port", self.internal_port.to_string()),
            (
                "approval.separation_of_duties",
                self.separation_of_duties.to_string(),
            ),
            (
                "alerts.webhook_url_file",
                self.webhook_url_file.clone().unwrap_or_default(),
            ),
            (
                "alerts.webhook_ca_file",
                self.webhook_ca_file.clone().unwrap_or_default(),
            ),
            ("alerts.webhook_url", webhook),
            ("session.secure", self.session_secure.to_string()),
            ("session.idle_timeout", seconds(self.idle_timeout)),
            ("session.absolute_timeout", seconds(self.absolute_timeout)),
            ("i18n.translation_langs", self.translation_langs.join(",")),
        ];
        texts
            .into_iter()
            .map(|(name, text)| parse_pair(name, &text))
            .collect()
    }
}

/// Return the canonical text of a timeout: whole seconds and `s`.
fn seconds(timeout: Duration) -> String {
    format!("{}s", timeout.as_secs())
}

/// Parse one key and one text into the newtypes of the table.
fn parse_pair(name: &'static str, text: &str) -> Result<(SettingKey, SettingValue), StorageError> {
    let key = SettingKey::parse(name)
        .map_err(|_| StorageError::invariant("a key of the settings comparison is not valid"))?;
    let value = SettingValue::parse(text)
        .map_err(|_| StorageError::invariant("a configured value is not a valid setting value"))?;
    Ok((key, value))
}

/// The startup comparison of the configuration with `app_setting`.
///
/// The type solves one problem: every change of an operator setting between two starts needs an
/// audit entry (A34), and a secret must never reach the table. The operation runs one
/// `READ COMMITTED` transaction. It locks every configuration row first (ADR-0079). It upserts each
/// changed or missing row and builds one `SettingChanged` entry for it. All entries share the
/// request identifier of the start. The entries and the rows commit together or not at all.
#[derive(Clone)]
pub struct CompareSettings {
    uow: Arc<dyn UnitOfWork>,
}

impl CompareSettings {
    /// Build the operation on a unit of work.
    #[must_use]
    pub fn new(uow: Arc<dyn UnitOfWork>) -> Self {
        Self { uow }
    }

    /// Compare `configured` with the stored rows and write the differences.
    ///
    /// A stored row without a configured key stays unchanged. With no difference the operation
    /// writes nothing and commits nothing, because a commit needs at least one entry. It returns the
    /// number of rows that it wrote.
    ///
    /// # Errors
    ///
    /// Returns a defect error (`E-DB-008`) for a value that is not a valid setting value.
    /// Returns `AppError::AuditWriteFailed` (`E-AUD-001`) when the entries are not written.
    /// Returns `AppError::Storage` for every other failure of the transaction.
    pub async fn run(
        &self,
        configured: &ConfiguredSettings,
        request_id: RequestId,
    ) -> Result<usize, AppError> {
        let wanted = configured.canonical()?;
        let mut tx = self.uow.begin(Isolation::ReadCommitted).await?;
        let mut stored: HashMap<SettingKey, Locked<SettingRow>> = tx
            .settings()
            .lock_rows()
            .await?
            .into_iter()
            .map(|locked| (locked.state.key.clone(), locked))
            .collect();
        let mut entries = Vec::new();
        for (key, value) in &wanted {
            let old = stored.remove(key);
            let written = write_difference(tx.settings(), key, value, old, request_id).await?;
            entries.extend(written);
        }
        let changed = entries.len();
        // An empty list ends the call here. The drop of `tx` rolls back and frees the locks.
        let Ok(entries) = NonEmpty::try_from(entries) else {
            return Ok(0);
        };
        self.uow.commit(tx, entries).await?;
        Ok(changed)
    }
}

/// Upsert the row of `key` when it differs from `old`, and return the entry of the change.
async fn write_difference(
    repository: &mut dyn SettingRepository,
    key: &SettingKey,
    value: &SettingValue,
    old: Option<Locked<SettingRow>>,
    request_id: RequestId,
) -> Result<Option<AuditEntry>, StorageError> {
    if old
        .as_ref()
        .is_some_and(|locked| locked.state.value == *value)
    {
        return Ok(None);
    }
    let written = repository.upsert(key, value).await?;
    let before = old.map(|locked| locked.before.into_value());
    Ok(Some(AuditEntry::setting_changed(
        written.id,
        before,
        written.after.into_value(),
        request_id,
    )))
}
