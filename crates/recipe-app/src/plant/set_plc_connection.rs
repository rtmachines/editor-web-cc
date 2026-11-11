//! The use case `SetPlcConnection` (`domain-model.md` section 5.1, ADR-0154, design D6).

use std::sync::Arc;

use recipe_domain::audit::{AuditEntry, AuditTarget, RowAction, RowChange};
use recipe_domain::plant::{
    ConnectionFields, MachineCode, MachineId, PasswordPlan, PasswordState, PlantError,
    PlcConnection, PlcConnectionId, PlcPlatform,
};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{
    CredentialCipher, Isolation, Locked, NonEmpty, PasswordWrite, RowImage, SealedSecret,
    StoredConnection, Transaction, seal_password,
};

/// The parsed request of `SetPlcConnection`.
///
/// The type has no `Clone` and no `PartialEq`, because [`ConnectionFields`] holds a password.
#[derive(Debug)]
pub struct SetPlcConnectionInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
    /// The `row_version` of the record that the form showed. A machine without a record counts as 0.
    pub expected_row_version: i64,
    /// The parsed fields of the form. An empty field is `None`.
    pub fields: ConnectionFields,
}

/// The result of `SetPlcConnection`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SetPlcConnectionOutput {
    /// The identifier of the machine that owns the record.
    pub machine: MachineId,
    /// The `row_version` of the record after the save. The first save gives 1.
    pub row_version: i64,
}

/// Creates or changes the PLC connection record of a machine.
///
/// The use case solves one problem: the connection settings of a machine need one audited
/// write path with an optimistic lock and a sealed password. It checks the permission first
/// (ADR-0013), reads the platform without a lock, and checks the fields before it opens the
/// command transaction. The transaction locks the machine `FOR NO KEY UPDATE` and then the
/// record `FOR UPDATE` (ADR-0015). The machine lock serializes two first saves and a save with
/// `ArchiveMachine` (design D6).
#[derive(Clone)]
pub struct SetPlcConnection {
    ctx: AppContext,
    cipher: Arc<dyn CredentialCipher>,
}

impl UseCase for SetPlcConnection {
    const OPERATION: Operation = Operation::SetPlcConnection;
}

/// The facts that step 2 of design D6 reads without a lock.
struct Unlocked {
    machine: MachineId,
    code: MachineCode,
    platform: PlcPlatform,
    password: PasswordState,
}

/// Return the `row_version` of an image that a write method returned.
fn row_version_of(image: &RowImage) -> Result<i64, StorageError> {
    image
        .as_value()
        .get("row_version")
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| StorageError::invariant("the row image has no row version"))
}

/// Return the id of the record row of an image that `insert_connection` returned.
fn connection_id_of(image: &RowImage) -> Result<PlcConnectionId, StorageError> {
    image
        .as_value()
        .get("id")
        .and_then(Value::as_str)
        .and_then(|text| Uuid::parse_str(text).ok())
        .map(PlcConnectionId::from)
        .ok_or_else(|| StorageError::invariant("the row image has no id"))
}

impl SetPlcConnection {
    /// Build the use case over a context and the cipher that seals a new password.
    #[must_use]
    pub const fn new(ctx: AppContext, cipher: Arc<dyn CredentialCipher>) -> Self {
        Self { ctx, cipher }
    }

    /// Save the connection record of the machine.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `AppError::Plants` (`E-PLT-022` to `E-PLT-026`, `E-PLT-028`) for each field fault.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived under the lock.
    /// Returns `ConnectionConcurrentEdit` (`E-WEB-013`) when the `row_version` is stale.
    /// Returns `AppError::Storage` or `AppError::AuditWriteFailed` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: SetPlcConnectionInput,
    ) -> Result<SetPlcConnectionOutput, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let unlocked = self.read_unlocked(input.code).await?;
        let plan = PlcConnection::for_platform(
            unlocked.code,
            unlocked.platform,
            input.fields,
            unlocked.password,
        )?;
        let password = self.password_write(unlocked.machine, plan.password)?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let machine = tx
            .plant()
            .lock_machine_for_no_key_update(unlocked.machine)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        if !machine.state.is_live() {
            return Err(PlantError::MachineArchived { code: input.code }.into());
        }
        let locked = tx
            .plc_connections()
            .lock_connection_for_update(unlocked.machine)
            .await?;
        let actual = locked.as_ref().map_or(0, |held| held.state.row_version);
        if actual != input.expected_row_version {
            return Err(AppError::ConnectionConcurrentEdit {
                code: input.code,
                expected: input.expected_row_version,
                actual,
            });
        }
        let (connection_id, row_version, change) = write_record(
            tx.as_mut(),
            unlocked.machine,
            &plan.connection,
            &password,
            locked,
        )
        .await?;
        let entries = NonEmpty::new(AuditEntry::row(
            RowAction::PlcConnectionSet,
            AuditTarget::plc_connection(connection_id, unlocked.machine),
            change,
            audit_context(user, meta),
        ));
        self.ctx.uow.commit(tx, entries).await?;
        Ok(SetPlcConnectionOutput {
            machine: unlocked.machine,
            row_version,
        })
    }

    /// Step 3 of design D6: turn the password plan into the write of the column `password_sealed`.
    ///
    /// The method seals a new password here, before the command transaction opens, because the
    /// seal must not run while a lock is held (`reliability.md` section 7.4). An S7 record has no
    /// plan and keeps the column unchanged.
    ///
    /// # Errors
    ///
    /// Returns `AppError::TaskFailed { stage: Token }` when the seal fails.
    fn password_write(
        &self,
        machine: MachineId,
        plan: Option<PasswordPlan>,
    ) -> Result<PasswordWrite, AppError> {
        match plan {
            None | Some(PasswordPlan::Keep) => Ok(PasswordWrite::Keep),
            Some(PasswordPlan::Clear) => Ok(PasswordWrite::Clear),
            Some(PasswordPlan::Replace(password)) => {
                seal_password(self.cipher.as_ref(), machine, &password).map(PasswordWrite::Set)
            }
        }
    }

    /// Step 2 of design D6: read the live machine and the password state without a lock.
    ///
    /// The transaction is `ReadOnly` and ends before the function returns. The platform of a
    /// machine never changes, and the `row_version` compare of step 5 proves the password state.
    async fn read_unlocked(&self, code: MachineCode) -> Result<Unlocked, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = tx
            .plant()
            .machine_by_code(code)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(code),
            })?;
        let stored = tx
            .plc_connections()
            .connection_by_machine(machine.id())
            .await?;
        Ok(Unlocked {
            machine: machine.id(),
            code,
            platform: machine.platform(),
            password: stored.map_or(PasswordState::Absent, |record| record.password),
        })
    }
}

/// Return the sealed password that a first save stores. `Keep` and `Clear` store none.
const fn sealed_for_insert(password: &PasswordWrite) -> Option<&SealedSecret> {
    match password {
        PasswordWrite::Set(sealed) => Some(sealed),
        PasswordWrite::Keep | PasswordWrite::Clear => None,
    }
}

/// The two flags that `PlcConnectionSet` adds to the row image (design D7, ADR-0154 (h)).
#[derive(Clone, Copy, Debug)]
struct PasswordFlags {
    /// Whether the row holds a sealed password.
    set: bool,
    /// Whether the command wrote a new sealed password or removed the stored one.
    changed: bool,
}

impl PasswordFlags {
    /// Derive the flags of the row after the command from the stored state and the write.
    ///
    /// A `Clear` of a record without a password changes nothing, so it gives `changed` false.
    const fn after(stored: PasswordState, write: &PasswordWrite) -> Self {
        let was_set = matches!(stored, PasswordState::Stored);
        match write {
            PasswordWrite::Set(_) => Self {
                set: true,
                changed: true,
            },
            PasswordWrite::Keep => Self {
                set: was_set,
                changed: false,
            },
            PasswordWrite::Clear => Self {
                set: false,
                changed: was_set,
            },
        }
    }
}

/// Add the key `password_set` and, when `flags` is given, the key `password_changed` to an image.
///
/// The image never holds `password_sealed`, so the audit entry holds no ciphertext. A value that
/// is not a JSON object stays unchanged.
fn with_password_flags(image: RowImage, set: bool, changed: Option<bool>) -> Value {
    let mut value = image.into_value();
    if let Value::Object(object) = &mut value {
        object.insert("password_set".to_owned(), Value::Bool(set));
        if let Some(changed) = changed {
            object.insert("password_changed".to_owned(), Value::Bool(changed));
        }
    }
    value
}

/// Step 6 of design D6: insert the first record or update the existing one.
///
/// The function returns the id of the row, the new `row_version`, and the change for the audit entry. An insert
/// stores a sealed password only for `PasswordWrite::Set`. An update applies `password` to the
/// column `password_sealed`. The change holds the row images and the flags `password_set` and
/// `password_changed` (design D7). A create holds only the image after the change.
async fn write_record(
    tx: &mut dyn Transaction,
    machine: MachineId,
    connection: &PlcConnection,
    password: &PasswordWrite,
    locked: Option<Locked<StoredConnection>>,
) -> Result<(PlcConnectionId, i64, RowChange), AppError> {
    match locked {
        None => {
            let after = tx
                .plc_connections()
                .insert_connection(machine, connection, sealed_for_insert(password))
                .await?;
            let connection_id = connection_id_of(&after)?;
            let row_version = row_version_of(&after)?;
            let flags = PasswordFlags::after(PasswordState::Absent, password);
            let change = RowChange::Create {
                after: with_password_flags(after, flags.set, Some(flags.changed)),
            };
            Ok((connection_id, row_version, change))
        }
        Some(held) => {
            let after = tx
                .plc_connections()
                .update_connection(machine, connection, password)
                .await?;
            let connection_id = held.state.id;
            let row_version = row_version_of(&after)?;
            let was_set = matches!(held.state.password, PasswordState::Stored);
            let flags = PasswordFlags::after(held.state.password, password);
            let change = RowChange::Update {
                before: with_password_flags(held.before, was_set, None),
                after: with_password_flags(after, flags.set, Some(flags.changed)),
            };
            Ok((connection_id, row_version, change))
        }
    }
}

#[cfg(test)]
mod tests;
