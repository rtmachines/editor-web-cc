//! In-memory `SettingRepository` (task 3.2).

use async_trait::async_trait;
use recipe_domain::setting::{SettingId, SettingKey, SettingValue};
use serde_json::Value;

use super::super::state::StoredSetting;
use super::MemoryTransaction;
use crate::error::StorageError;
use crate::memory::LockMode;
use crate::memory::images::{object, time_value};
use crate::ports::{
    Locked, RowImage, STATE_SETTING_KEYS, SettingRepository, SettingRow, SettingWritten,
};

/// The table name of the journal for a setting lock.
const SETTING_TABLE: &str = "app_setting";

/// Return the image of an `app_setting` row. `updated_by` is always `NULL`.
fn setting_image(key: &SettingKey, row: &StoredSetting) -> RowImage {
    RowImage::new(object(vec![
        ("id", Value::String(row.id.to_string())),
        ("key", Value::String(key.to_string())),
        ("value", Value::String(row.value.to_string())),
        ("updated_by", Value::Null),
        ("created_at", time_value(row.created_at)),
        ("updated_at", time_value(row.updated_at)),
    ]))
}

#[async_trait]
impl SettingRepository for MemoryTransaction {
    async fn lock_rows(&mut self) -> Result<Vec<Locked<SettingRow>>, StorageError> {
        let rows: Vec<(SettingKey, StoredSetting)> = self
            .state()
            .settings
            .iter()
            .filter(|(key, _)| !STATE_SETTING_KEYS.contains(&key.as_str()))
            .map(|(key, row)| (key.clone(), row.clone()))
            .collect();
        // PostgreSQL locks the rows in the order of `id`, so the journal does the same.
        let mut ids: Vec<SettingId> = rows.iter().map(|(_, row)| row.id).collect();
        ids.sort_unstable();
        for id in ids {
            self.lock(SETTING_TABLE, LockMode::ForNoKeyUpdate, &id.to_string());
        }
        Ok(rows
            .into_iter()
            .map(|(key, row)| Locked {
                before: setting_image(&key, &row),
                state: SettingRow {
                    id: row.id,
                    key,
                    value: row.value,
                },
            })
            .collect())
    }

    async fn upsert(
        &mut self,
        key: &SettingKey,
        value: &SettingValue,
    ) -> Result<SettingWritten, StorageError> {
        if STATE_SETTING_KEYS.contains(&key.as_str()) {
            return Err(StorageError::invariant("the upsert names a state key"));
        }
        let now = self.now();
        let row = self.state_mut().put_setting(key, value, now)?;
        Ok(SettingWritten {
            id: row.id,
            after: setting_image(key, &row),
        })
    }
}

#[cfg(test)]
mod tests;
