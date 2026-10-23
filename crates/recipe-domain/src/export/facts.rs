//! The fields of the export document outside the hash (`api.md` section 6.1, ADR-0019).

use std::collections::BTreeMap;

use serde::Serialize;
use serde::ser::Error as _;
use time::UtcOffset;

use super::{ExportError, Sha256};
use crate::identity::Login;
use crate::recipe::ParameterKey;
use crate::text::Text;
use crate::timestamp::Timestamp;

/// The value of `schema_version` in every export document of this format.
pub const SCHEMA_VERSION: &str = "1";

/// The last year that the time text can write with four digits.
const MAX_YEAR: i32 = 9999;

/// The status of an exported version (`api.md` section 6.1).
///
/// Only an approved or a retired version can be exported. The status serializes as its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize)]
pub enum ExportStatus {
    /// The version is the approved version of its recipe.
    Approved,
    /// The version was retired. A PLC must not load it.
    Retired,
}

impl ExportStatus {
    /// Return `true` when a PLC can load a version with this status.
    #[must_use]
    pub const fn loadable(self) -> bool {
        matches!(self, Self::Approved)
    }
}

/// The fields of the export document that the hash does not cover.
///
/// The type solves one problem: the flow collects these values, and the writer puts them into the
/// document next to the hashed body. A change of these values cannot change `content_sha256`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentFacts {
    /// The instant of the export.
    pub exported_at: Timestamp,
    /// The login of the user that exports the version.
    pub exported_by: Login,
    /// The status of the version.
    pub status: ExportStatus,
    /// The instant of the approval of the version.
    pub approved_at: Timestamp,
    /// The login of the user that approved the version.
    pub approved_by: Login,
    /// The name of the machine.
    pub machine_name: Text,
    /// The name of the recipe.
    pub recipe_name: Text,
    /// The stored hash of the version.
    pub content_sha256: Sha256,
    /// The English name (`name_en`) of each parameter, by its key.
    pub names: BTreeMap<ParameterKey, Text>,
}

impl DocumentFacts {
    /// Return the value of `loadable`. Only an approved version is loadable.
    #[must_use]
    pub const fn loadable(&self) -> bool {
        self.status.loadable()
    }
}

/// Write an instant as UTC text `YYYY-MM-DDTHH:MM:SSZ` with whole seconds (`api.md` section 6.2).
///
/// The function converts the instant to UTC and drops the fraction of a second.
///
/// # Errors
///
/// Returns `E-EXP-004` when the year in UTC is outside 0000 to 9999.
pub fn utc_text(instant: Timestamp) -> Result<String, ExportError> {
    let utc = instant
        .as_offset_date_time()
        .checked_to_offset(UtcOffset::UTC)
        .ok_or_else(|| year_out_of_range(instant))?
        .truncate_to_second();
    let (year, month, day) = utc.to_calendar_date();
    if !(0..=MAX_YEAR).contains(&year) {
        return Err(year_out_of_range(instant));
    }
    let (hour, minute, second) = utc.to_hms();
    let month = u8::from(month);
    Ok(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z"
    ))
}

/// Build the `E-EXP-004` error of an instant whose UTC year has no four-digit text.
fn year_out_of_range(instant: Timestamp) -> ExportError {
    ExportError::Serialize(serde_json::Error::custom(format_args!(
        "the UTC year of {:?} is outside 0000 to 9999",
        instant.as_offset_date_time()
    )))
}

#[cfg(test)]
mod tests;
