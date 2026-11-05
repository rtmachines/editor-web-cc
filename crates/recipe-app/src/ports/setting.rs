//! Port of the configuration rows of `app_setting` (`data-model.md` section 3.17, design D6).

use async_trait::async_trait;
use recipe_domain::setting::{SettingId, SettingKey, SettingValue};

use super::state_key::STATE_KEYS;
use super::{Locked, RowImage};
use crate::error::StorageError;

/// The three keys of `app_setting` that hold state and not configuration.
///
/// `MaintenanceStore` and `OperationsReads` own these keys. `SettingRepository` never reads them,
/// because the startup comparison skips them (`reliability.md` section 7.12). Its `upsert`
/// refuses them. The literals have one definition in `state_key.rs`.
pub const STATE_SETTING_KEYS: [&str; 3] = STATE_KEYS;

/// A configuration row of `app_setting` as the startup comparison reads it.
///
/// The type solves one problem: the comparison needs the stored text of each key and nothing
/// else. The row image of the audit entry carries the other columns.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingRow {
    /// The primary key.
    pub id: SettingId,
    /// The key of the setting.
    pub key: SettingKey,
    /// The stored text of the value.
    pub value: SettingValue,
}

/// The result of one upsert of a configuration row.
///
/// The type solves one problem: the audit entry of a new row needs the identifier that the
/// database chose, and the entry needs the row image after the write.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SettingWritten {
    /// The primary key of the row. An update keeps the key of the row.
    pub id: SettingId,
    /// The row image after the write.
    pub after: RowImage,
}

/// Port of the configuration rows of `app_setting` in one transaction.
///
/// The port solves one problem: the operation `CompareSettings` must read the rows and write
/// the changed rows in its own transaction. The rows and the `SettingChanged` entries then
/// commit together or not at all (ADR-0015). The port covers the configuration keys only. It
/// skips the keys of [`STATE_SETTING_KEYS`] when it reads and refuses them when it writes.
#[async_trait]
pub trait SettingRepository: Send {
    /// Read every configuration row and lock it until the end of the transaction.
    ///
    /// The result is in the byte order of the keys. Each row comes with its image before any
    /// change. An empty table gives an empty list.
    async fn lock_rows(&mut self) -> Result<Vec<Locked<SettingRow>>, StorageError>;

    /// Insert the row of `key`, or set its value when the row exists.
    ///
    /// The write sets `updated_by` to `NULL`, because the startup comparison has no actor. It
    /// returns the identifier of the row and the row image after the write.
    ///
    /// # Errors
    ///
    /// A key of [`STATE_SETTING_KEYS`] gives a defect error (`E-DB-008`) before any write.
    async fn upsert(
        &mut self,
        key: &SettingKey,
        value: &SettingValue,
    ) -> Result<SettingWritten, StorageError>;
}

#[cfg(test)]
mod tests;
