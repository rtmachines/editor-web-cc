//! The request struct of `POST /machines/{code}/connection` (`api.md` section 2.2, ADR-0154,
//! design D2, D8, and D12).
//!
//! The struct reads the fields of both shapes of the form. An empty text is no value. So a field
//! that the platform does not use arrives as `None`, and a field of another shape with a value
//! reaches the domain and gives `E-PLT-022`. The password stays in a [`SecretField`] until the
//! domain parses it. A text that does not parse gives the line `E-WEB-008` of its input, and the
//! answer lists every such input at once (ADR-0041).

use std::fmt;

use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{
    CommMode, ConnectionFields, PlcAddress, PlcHost, PlcLogin, PlcPassword, Rack, Slot, TcpPort,
};
use serde::{Deserialize, Deserializer};
use zeroize::Zeroizing;

use super::connection::InputTexts;
use crate::error::{FieldError, FieldReason, WebError};
use crate::pages::form_fields::FieldLines;

/// The text of the password field of the connection form.
///
/// The type solves one problem: the typed password must not reach a log line, an error value, or a
/// serialized answer. It has a `Debug` that hides the text, no `Serialize`, no `Clone`, and no
/// `Display`. It clears its buffer on drop, and the assertion below fails the build if it does not.
pub(super) struct SecretField(Zeroizing<String>);

// The buffer of the password must clear itself on drop. A plain `String` field does not
// implement `ZeroizeOnDrop`, so this assertion then fails the build in every profile.
const _: fn(&SecretField) -> &dyn zeroize::ZeroizeOnDrop = |field| &field.0;

impl<'de> Deserialize<'de> for SecretField {
    /// Read the text of the field and move it into the buffer.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer).map(Self::from)
    }
}

impl From<String> for SecretField {
    /// Take the text of the field. The buffer moves in, so no copy stays behind.
    fn from(text: String) -> Self {
        Self(Zeroizing::new(text))
    }
}

impl SecretField {
    /// Return the password text.
    ///
    /// Only the parse of the password calls this accessor. Any other caller leaks the secret.
    pub(super) fn expose(&self) -> &str {
        self.0.as_str()
    }
}

impl fmt::Debug for SecretField {
    /// Write the field without its text.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SecretField(<redacted>)")
    }
}

/// Read the password field: an absent or empty text is `None` (design D2).
fn optional_secret<'de, D>(deserializer: D) -> Result<Option<SecretField>, D::Error>
where
    D: Deserializer<'de>,
{
    let text = Option::<String>::deserialize(deserializer)?;
    Ok(text.filter(|text| !text.is_empty()).map(SecretField::from))
}

/// The fields of the form of `POST /machines/{code}/connection`.
///
/// The type solves one problem: one struct reads the fields of both shapes, so a field of another
/// platform with a value reaches the domain and gives `E-PLT-022` (design D8). An empty text is
/// no value. The `Debug` of the struct hides every text.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ConnectionRequest {
    row_version: String,
    #[serde(default)]
    host: Option<String>,
    #[serde(default)]
    rack: Option<String>,
    #[serde(default)]
    slot: Option<String>,
    #[serde(default)]
    port: Option<String>,
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    comm_mode: Option<String>,
    #[serde(default)]
    login: Option<String>,
    /// The new password. An empty text is `None`, and the stored password stays (design D2).
    #[serde(default, deserialize_with = "optional_secret")]
    pub(super) password: Option<SecretField>,
    /// The session token. The CSRF layer checked it before the handler runs.
    #[serde(default, rename = "csrf_token")]
    _csrf_token: Option<String>,
}

impl fmt::Debug for ConnectionRequest {
    /// Write the request without the password and without the other texts.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ConnectionRequest(<redacted>)")
    }
}

/// Return the text of a field, or `None` when the field is absent or empty.
fn present(field: &Option<String>) -> Option<&str> {
    field.as_deref().filter(|text| !text.is_empty())
}

/// Parse a text when it is present. A bad text becomes the line `E-WEB-008` of its input.
///
/// The parser error holds the typed text, so the line keeps only the field and the reason. No
/// typed value reaches a message or a log line (ADR-0041).
fn read<T, R>(
    lines: &mut FieldLines,
    text: Option<&str>,
    parse: impl FnOnce(&str) -> Result<T, R>,
    field: &'static str,
    reason: FieldReason,
) -> Option<T> {
    text.and_then(|text| lines.field(parse(text), field, reason))
}

impl ConnectionRequest {
    /// Read the `row_version` that the form showed. It is a whole number from 0.
    pub(super) fn expected_row_version(&self) -> Result<i64, WebError> {
        let text = self.row_version.as_str();
        let digits = !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit());
        digits
            .then(|| text.parse::<i64>().ok())
            .flatten()
            .ok_or(WebError::BadRequest {
                field: "row_version",
                reason: FieldReason::Reload,
            })
    }

    /// Parse every present field into the fields of the use case.
    ///
    /// The function reads every field, so one answer lists every bad text (ADR-0041 (a)).
    ///
    /// # Errors
    ///
    /// Returns one line `E-WEB-008` under each input whose text does not parse, in the order of
    /// the form.
    pub(super) fn fields(&self) -> Result<ConnectionFields, NonEmpty<FieldError>> {
        let mut lines = FieldLines::default();
        let fields = self.s7_fields(&mut lines, ConnectionFields::default());
        let fields = self.iec_fields(&mut lines, fields);
        let fields = self.credential_fields(&mut lines, fields);
        lines.finish(Some(fields))
    }

    /// Parse the host and the S7 fields: `host`, `rack`, and `slot`.
    fn s7_fields(&self, lines: &mut FieldLines, base: ConnectionFields) -> ConnectionFields {
        ConnectionFields {
            host: read(
                lines,
                present(&self.host),
                PlcHost::parse,
                "host",
                FieldReason::ConnectionHost,
            ),
            rack: read(
                lines,
                present(&self.rack),
                Rack::parse,
                "rack",
                FieldReason::ConnectionRack,
            ),
            slot: read(
                lines,
                present(&self.slot),
                Slot::parse,
                "slot",
                FieldReason::ConnectionSlot,
            ),
            ..base
        }
    }

    /// Parse the three IEC fields: `port`, `address`, and `comm_mode`.
    fn iec_fields(&self, lines: &mut FieldLines, base: ConnectionFields) -> ConnectionFields {
        ConnectionFields {
            port: read(
                lines,
                present(&self.port),
                TcpPort::parse,
                "port",
                FieldReason::ConnectionPort,
            ),
            address: read(
                lines,
                present(&self.address),
                PlcAddress::parse,
                "address",
                FieldReason::ConnectionAddress,
            ),
            mode: read(
                lines,
                present(&self.comm_mode),
                CommMode::parse,
                "comm_mode",
                FieldReason::ConnectionMode,
            ),
            ..base
        }
    }

    /// Parse the credential fields: `login` and `password`.
    fn credential_fields(
        &self,
        lines: &mut FieldLines,
        base: ConnectionFields,
    ) -> ConnectionFields {
        ConnectionFields {
            login: read(
                lines,
                present(&self.login),
                PlcLogin::parse,
                "login",
                FieldReason::ConnectionLogin,
            ),
            password: read(
                lines,
                self.password.as_ref().map(SecretField::expose),
                PlcPassword::parse,
                "password",
                FieldReason::PasswordLength,
            ),
            ..base
        }
    }

    /// Return the texts that the form sent, for the form of a refused answer.
    ///
    /// The result has no password, so the refused form cannot show one.
    pub(super) fn input_texts(&self) -> InputTexts {
        let text = |field: &Option<String>| field.clone().unwrap_or_default();
        InputTexts {
            host: text(&self.host),
            rack: text(&self.rack),
            slot: text(&self.slot),
            port: text(&self.port),
            address: text(&self.address),
            mode: text(&self.comm_mode),
            login: text(&self.login),
        }
    }
}
