//! The query `GetPlcConnectionForm` (ADR-0154, design D8).
//!
//! The specs are "Connection of a machine" and "Secrecy of the connection password".

use recipe_domain::plant::{MachineCode, MachineId, PasswordState, PlcConnection, PlcPlatform};

use super::live_machine;
use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase};
use crate::error::AppError;
use crate::ports::Isolation;

/// The parsed request of `GetPlcConnectionForm`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PlcConnectionFormInput {
    /// The code of the live machine in the URL.
    pub code: MachineCode,
}

/// The data of the page that edits the connection of a machine.
///
/// The type has no password and no ciphertext, so a template cannot show either one.
/// The type has no `Clone` and no `PartialEq`, because [`PlcConnection`] has none.
#[derive(Debug)]
pub struct PlcConnectionForm {
    /// The identifier of the machine.
    pub machine: MachineId,
    /// The code of the machine.
    pub code: MachineCode,
    /// The platform of the machine. It decides which fields the form shows.
    pub platform: PlcPlatform,
    /// The stored settings, or `None` when the machine has no record.
    pub connection: Option<PlcConnection>,
    /// The `row_version` that the hidden field of the form sends back. A machine without a record gives 0.
    pub row_version: i64,
    /// Whether the record holds a sealed password. The form shows this state and never the password.
    pub password_set: bool,
}

/// Gives the data of the form that edits the connection of a machine.
///
/// The query solves one problem: the form needs the platform, the stored settings, the lock
/// token, and the state of the password, but never the password itself (R31, A47). The query
/// reads the machine and the record in one `ReadOnly` transaction and takes no lock. The first
/// statement checks `MachineManage` (ADR-0013).
#[derive(Clone)]
pub struct GetPlcConnectionForm {
    ctx: AppContext,
}

impl UseCase for GetPlcConnectionForm {
    const OPERATION: Operation = Operation::GetPlcConnectionForm;
}

impl GetPlcConnectionForm {
    /// Build the query over a context.
    #[must_use]
    pub const fn new(ctx: AppContext) -> Self {
        Self { ctx }
    }

    /// Return the form data of the live machine with the code of the request.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks `MachineManage`.
    /// Returns `NotFound` (`E-WEB-005`) when no live machine has the code.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: PlcConnectionFormInput,
    ) -> Result<PlcConnectionForm, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = live_machine(tx.plant(), input.code).await?;
        let stored = tx
            .plc_connections()
            .connection_by_machine(machine.id())
            .await?;
        let (connection, row_version, password_set) = match stored {
            Some(record) => (
                Some(record.connection),
                record.row_version,
                record.password == PasswordState::Stored,
            ),
            None => (None, 0, false),
        };
        Ok(PlcConnectionForm {
            machine: machine.id(),
            code: machine.code(),
            platform: machine.platform(),
            connection,
            row_version,
            password_set,
        })
    }
}
