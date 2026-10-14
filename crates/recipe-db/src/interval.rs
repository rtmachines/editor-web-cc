//! The bind of a `Duration` for a statement of the identity ports.

use std::time::Duration;

use recipe_app::error::StorageError;

use crate::error::corrupt;

/// Return the duration in whole microseconds, the unit of `PostgreSQL` time.
///
/// The statements multiply the number by `interval '1 microsecond'`, so the result is exact.
/// The function solves one problem: a bind of `f64` seconds would round a long timeout.
///
/// # Errors
///
/// Returns a defect (`E-DB-008`) when the duration does not fit in a `bigint`.
pub(crate) fn micros(table: &'static str, span: Duration) -> Result<i64, StorageError> {
    i64::try_from(span.as_micros()).map_err(|error| corrupt(table, error))
}
