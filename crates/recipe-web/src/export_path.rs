//! The path prefix of the export routes (`api.md` section 2.7, ADR-0109 (q)).

/// The prefix of every path of an export route.
///
/// It holds `/export/v1/` and `/export/v2/` (ADR-0109 (q)). The error layer reads it for the JSON
/// answers of ADR-0053. The `HEAD` rule of ADR-0054 and the
/// guard of ADR-0055 read the same constant. The prefix ends with `/`, so `/exports` is
/// outside it.
pub(crate) const EXPORT_PATH_PREFIX: &str = "/export/";

/// Return true when `path` starts with the prefix of the export routes.
pub(crate) fn is_export_path(path: &str) -> bool {
    path.starts_with(EXPORT_PATH_PREFIX)
}
