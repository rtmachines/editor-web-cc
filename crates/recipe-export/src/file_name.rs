//! File name of an export download (`api.md` section 6.3).

use recipe_domain::export::ExportStatus;
use recipe_domain::recipe::ReferenceNumber;

/// The prefix of the file name of a retired version.
const RETIRED_PREFIX: &str = "RETIRED_";

/// Return the file name of the export of one version.
///
/// The name is `recipe_{ref}_v{n}.json` for an approved version. A retired version gets the
/// prefix `RETIRED_`, so a person sees the status in the download list. The grammar of
/// `ReferenceNumber` permits only `A-Z`, `0-9`, `_`, and `-`. The name therefore needs no escape
/// and is safe in a `Content-Disposition` header.
#[must_use]
pub fn file_name(reference: &ReferenceNumber, number: u32, status: ExportStatus) -> String {
    let prefix = match status {
        ExportStatus::Approved => "",
        ExportStatus::Retired => RETIRED_PREFIX,
    };
    format!("{prefix}recipe_{reference}_v{number}.json")
}

#[cfg(test)]
mod tests;
