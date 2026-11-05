//! The bind values of a page window (design D4, U21).
//!
//! `machine_list`, `type_usage`, and the user list bind an offset and a limit as `bigint`.
//! This module holds the one conversion and the one error of a page offset that does not fit.

use recipe_app::error::StorageError;

/// The cause of the storage error for a page offset that `bigint` cannot hold.
#[derive(Debug, thiserror::Error)]
#[error("the page offset {offset} does not fit the type bigint")]
struct OffsetOutOfRange {
    offset: u64,
}

/// The cause of the storage error for a page number and a page size whose product has no `u64`.
#[derive(Debug, thiserror::Error)]
#[error("the page {page} times the page size {size} does not fit the type u64")]
struct OffsetOverflow {
    page: u32,
    size: u32,
}

/// Compute the row offset of a page with `checked_mul`.
///
/// The product of two `u32` values always fits `u64`, so the error is a guard and never happens.
/// It gives `StorageError::Other` without a SQLSTATE (`E-DB-008`).
pub(super) fn page_offset(page: u32, size: u32) -> Result<u64, StorageError> {
    u64::from(page)
        .checked_mul(u64::from(size))
        .ok_or_else(|| StorageError::Other {
            sqlstate: None,
            source: Box::new(OffsetOverflow { page, size }),
        })
}

/// Convert a page offset to the `bigint` value that `OFFSET` binds.
///
/// An offset above `i64::MAX` is a defect of the caller. It gives `StorageError::Other` without a
/// SQLSTATE, as any other fault of the adapter does (`E-DB-008`). The memory adapters return the
/// same variant for the same offset.
pub(super) fn bind_offset(offset: u64) -> Result<i64, StorageError> {
    i64::try_from(offset).map_err(|_| StorageError::Other {
        sqlstate: None,
        source: Box::new(OffsetOutOfRange { offset }),
    })
}

/// Convert the offset and the limit of a page to the `bigint` values that `OFFSET` and `LIMIT` bind.
///
/// A limit of `u32` always fits, so only the offset can fail.
pub(super) fn bind_window(offset: u64, limit: u32) -> Result<(i64, i64), StorageError> {
    Ok((bind_offset(offset)?, i64::from(limit)))
}

#[cfg(test)]
mod tests;
