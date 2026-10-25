//! The webhook file of `serve` (ADR-0063 (a), (b), (f), (g), design D7).
//!
//! The secret file of `alerts.webhook_url_file` holds the URL of the webhook. The URL is a
//! secret. No error, log line, `Display` text, or `Debug` text in this module holds a part of it.

use std::fmt;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Result, anyhow};
use hyper::Uri;
use hyper::http::uri::Scheme;
use recipe_app::operations::WebhookUrlText;

use super::{SecretFault, secret_line};

/// The URL of the webhook, after the checks of ADR-0063 (b).
///
/// The type solves one problem: the URL holds a token, so a derived `Debug` or a log field must
/// not print it. `Debug` is written by hand and redacts the URL. The type has no `Display`.
#[derive(Clone)]
pub struct WebhookUrl {
    uri: Uri,
    text: String,
}

impl WebhookUrl {
    /// Return the parsed URL. The caller must never log it.
    #[must_use]
    pub const fn uri(&self) -> &Uri {
        &self.uri
    }

    /// Return the text of the file line for the fingerprint of `app_setting` (ADR-0063 (c)).
    #[must_use]
    pub fn to_url_text(&self) -> WebhookUrlText {
        WebhookUrlText::from(self.text.clone())
    }
}

impl fmt::Debug for WebhookUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WebhookUrl(<redacted>)")
    }
}

/// Why the text of a webhook file is not one valid HTTPS URL (ADR-0063 (b)).
///
/// This is a reason type without `code()` (ADR-0025). No variant holds a part of the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookFileFault {
    /// The file holds a carriage return anywhere.
    CarriageReturn,
    /// The file holds a second line.
    SeveralLines,
    /// The URL has a scheme other than `https`.
    NotHttps,
    /// The text does not parse as a URL with a scheme and a host.
    NotAnAbsoluteUrl,
}

impl fmt::Display for WebhookFileFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::CarriageReturn => "holds a carriage return",
            Self::SeveralLines => "holds more than one line",
            Self::NotHttps => "does not use the scheme https",
            Self::NotAnAbsoluteUrl => "holds a text that is not an absolute URL",
        })
    }
}

/// Parse the text of a webhook file.
///
/// An empty file, or a file with one line feed only, means no webhook. One trailing line feed is
/// allowed.
///
/// # Errors
///
/// Returns a [`WebhookFileFault`] for a carriage return, a second line, a text that is not an
/// absolute URL, or a scheme other than `https`.
pub fn parse_webhook_text(text: &str) -> Result<Option<WebhookUrl>, WebhookFileFault> {
    let line = match secret_line(text) {
        Ok(line) => line,
        Err(SecretFault::Empty) => return Ok(None),
        Err(SecretFault::CarriageReturn) => return Err(WebhookFileFault::CarriageReturn),
        Err(SecretFault::SeveralLines) => return Err(WebhookFileFault::SeveralLines),
    };
    let uri: Uri = line
        .parse()
        .map_err(|_bad| WebhookFileFault::NotAnAbsoluteUrl)?;
    match uri.scheme() {
        None => return Err(WebhookFileFault::NotAnAbsoluteUrl),
        Some(scheme) if *scheme != Scheme::HTTPS => return Err(WebhookFileFault::NotHttps),
        Some(_https) => {}
    }
    if uri.host().is_none() {
        return Err(WebhookFileFault::NotAnAbsoluteUrl);
    }
    Ok(Some(WebhookUrl {
        uri,
        text: line.to_owned(),
    }))
}

/// How the path of the webhook file got its value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WebhookSource {
    /// The operator set the key. A missing file stops the start (ADR-0063 (a)).
    Set,
    /// The key has its default path. A missing file means no webhook.
    Default,
}

/// Read the webhook file of the configuration key `key`.
///
/// An empty file gives `None` and one `INFO` line that names the key. The error names the key.
/// It never shows the path or a part of the URL.
///
/// # Errors
///
/// Returns an error when a file at a set path cannot be read, or when its text breaks the rules
/// of [`parse_webhook_text`].
pub fn read_webhook_file(
    key: &str,
    path: &Path,
    source: WebhookSource,
) -> Result<Option<WebhookUrl>> {
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(fault) if fault.kind() == ErrorKind::NotFound && source == WebhookSource::Default => {
            tracing::info!(
                key,
                "no webhook file at the default path: alerts have no webhook"
            );
            return Ok(None);
        }
        Err(fault) => {
            return Err(anyhow!(
                "the file of {key} cannot be read: {}",
                fault.kind()
            ));
        }
    };
    let url = parse_webhook_text(&text).map_err(|fault| anyhow!("the file of {key} {fault}"))?;
    if url.is_none() {
        tracing::info!(key, "the webhook file is empty: alerts have no webhook");
    }
    Ok(url)
}

#[cfg(test)]
mod tests;
