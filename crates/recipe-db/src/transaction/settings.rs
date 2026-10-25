//! `SettingRepository` on the transaction (task 3.2, `data-model.md` sections 3.17 and 6).
//!
//! The port covers the configuration keys only. The read skips the state keys, which
//! `MaintenanceStore` owns, and the upsert refuses a state key before any write. The role
//! `recipe_app` has `SELECT`, `INSERT`, and `UPDATE (value, updated_by, updated_at)` on the table,
//! and the statements use nothing else.

use async_trait::async_trait;
use recipe_app::error::StorageError;
use recipe_app::ports::{
    Locked, RowImage, STATE_SETTING_KEYS, SettingRepository, SettingRow, SettingWritten,
};
use recipe_domain::setting::{SettingId, SettingKey, SettingValue};
use serde_json::Value;
use uuid::Uuid;

use crate::error::{corrupt, map_error, reserved_key};
use crate::transaction::PgTransaction;

/// The table name in a `Corrupt` error and in a `ReservedKey` error.
const TABLE: &str = "app_setting";

/// `app_setting FOR NO KEY UPDATE` (ADR-0015). The rows lock in the order of `id`, as the rule
/// for several rows of one table demands. The image is the row before any change.
/// The lock needs the `UPDATE` grant on the columns `value`, `updated_by`, and `updated_at`.
const LOCK_ROWS: &str = "\
SELECT id, key, value, to_jsonb(s) AS image \
FROM app_setting s \
WHERE key <> ALL($1::text[]) \
ORDER BY id \
FOR NO KEY UPDATE";

/// Insert the row, or set the value of the row of the same key. The key and `id` stay.
/// The trigger `app_setting_set_updated_at` sets `updated_at` on the update path.
const UPSERT: &str = "\
INSERT INTO app_setting (key, value) VALUES ($1, $2) \
ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_by = NULL \
RETURNING id, to_jsonb(app_setting) AS image";

#[derive(sqlx::FromRow)]
struct LockedRecord {
    id: Uuid,
    key: String,
    value: String,
    image: Value,
}

#[derive(sqlx::FromRow)]
struct WrittenRecord {
    id: Uuid,
    image: Value,
}

impl TryFrom<LockedRecord> for Locked<SettingRow> {
    type Error = StorageError;

    fn try_from(record: LockedRecord) -> Result<Self, StorageError> {
        Ok(Self {
            state: SettingRow {
                id: SettingId::from(record.id),
                key: SettingKey::parse(&record.key).map_err(|error| corrupt(TABLE, error))?,
                value: SettingValue::parse(&record.value).map_err(|error| corrupt(TABLE, error))?,
            },
            before: RowImage::new(record.image),
        })
    }
}

impl From<WrittenRecord> for SettingWritten {
    fn from(record: WrittenRecord) -> Self {
        Self {
            id: SettingId::from(record.id),
            after: RowImage::new(record.image),
        }
    }
}

#[async_trait]
impl SettingRepository for PgTransaction {
    async fn lock_rows(&mut self) -> Result<Vec<Locked<SettingRow>>, StorageError> {
        let records = sqlx::query_as::<_, LockedRecord>(LOCK_ROWS)
            .bind(STATE_SETTING_KEYS.as_slice())
            .fetch_all(&mut *self.tx)
            .await
            .map_err(map_error)?;
        let mut rows = records
            .into_iter()
            .map(Locked::<SettingRow>::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        rows.sort_by(|left, right| left.state.key.cmp(&right.state.key));
        Ok(rows)
    }

    async fn upsert(
        &mut self,
        key: &SettingKey,
        value: &SettingValue,
    ) -> Result<SettingWritten, StorageError> {
        if STATE_SETTING_KEYS.contains(&key.as_str()) {
            return Err(reserved_key(TABLE, key.as_str()));
        }
        sqlx::query_as::<_, WrittenRecord>(UPSERT)
            .bind(key.as_str())
            .bind(value.as_str())
            .fetch_one(&mut *self.tx)
            .await
            .map(SettingWritten::from)
            .map_err(map_error)
    }
}
