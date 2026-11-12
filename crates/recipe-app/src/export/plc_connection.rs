//! The use case `ExportPlcConnection` (ADR-0154, design D9).

use std::sync::Arc;

use recipe_domain::audit::{AuditEntry, AuditTarget, StateAction};
use recipe_domain::plant::{
    DataBlockNumber, Machine, MachineCode, MachineId, PlantError, PlcConnection, PlcConnectionId,
    PlcPassword, PlcPlatform,
};
use recipe_domain::recipe::ReferenceNumber;
use serde_json::json;

use crate::auth::{CurrentUser, Operation, RequestMeta, authorize};
use crate::context::{AppContext, UseCase, audit_context};
use crate::error::{AppError, Missing};
use crate::ports::{CredentialCipher, Isolation, NonEmpty, StoredConnection, Transaction};

/// The `schema_version` of the connection document (`api.md` section 6.5).
const CONNECTION_SCHEMA_VERSION: &str = "1";

/// The parsed request of `ExportPlcConnection`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExportPlcConnectionInput {
    /// The code of the machine in the URL.
    pub code: MachineCode,
    /// The reference number of the recipe in the query value `recipe`.
    pub recipe: ReferenceNumber,
}

/// The connection record of a machine with its plain password.
///
/// The type has no `Clone` and no `PartialEq`. [`PlcPassword`] hides its text in `Debug`.
#[derive(Debug)]
pub struct ExportedPlcConnection {
    /// The identifier of the machine.
    pub machine: MachineId,
    /// The code of the machine.
    pub code: MachineCode,
    /// The platform of the machine.
    pub platform: PlcPlatform,
    /// The reference number of the recipe that the request named.
    pub recipe: ReferenceNumber,
    /// The `row_version` of the record.
    pub row_version: i64,
    /// The data block number of the machine. It is `None` on an IEC platform (ADR-0156 (g)).
    ///
    /// The field `connection.db_number` of the document takes this value, not the copy in the record.
    pub db_number: Option<DataBlockNumber>,
    /// The settings of the record.
    pub connection: PlcConnection,
    /// The opened password, or `None` when the record holds none.
    pub password: Option<PlcPassword>,
}

/// Build the exported record from the locked machine, the stored record, and the opened password.
fn build_exported(
    machine: MachineId,
    state: &Machine,
    recipe: ReferenceNumber,
    stored: StoredConnection,
    password: Option<PlcPassword>,
) -> ExportedPlcConnection {
    ExportedPlcConnection {
        machine,
        code: state.code(),
        platform: state.platform(),
        recipe,
        row_version: stored.row_version,
        db_number: state.db_number(),
        connection: stored.connection,
        password,
    }
}

/// The result of the timed part: the open transaction, the id of the record row, and the record.
struct Prepared {
    tx: Box<dyn Transaction>,
    id: PlcConnectionId,
    exported: ExportedPlcConnection,
}

/// Build the entry `PlcConnectionExported` (`data-model.md` section 10).
///
/// The function solves one problem: the entry must name the read and never hold the password.
/// The state holds only the machine, the `row_version`, the schema version, and the recipe.
fn exported_entry(
    id: PlcConnectionId,
    exported: &ExportedPlcConnection,
    (user, meta): (&CurrentUser, &RequestMeta),
) -> AuditEntry {
    let after = json!({
        "machine_id": exported.machine.to_string(),
        "row_version": exported.row_version,
        "schema_version": CONNECTION_SCHEMA_VERSION,
        "reference_number": exported.recipe.to_string(),
    });
    AuditEntry::state(
        StateAction::PlcConnectionExported,
        AuditTarget::plc_connection(id, exported.machine),
        after,
        audit_context(user, meta),
    )
}

/// Reads the connection record of a machine with its password.
///
/// The use case solves one problem: the transfer client must load the connection of the machine
/// that a recipe belongs to, and the password must never leave through any other path
/// (ADR-0154, design D9). The permission check is the first statement (ADR-0013). The use case
/// takes an `export.permits` permit. Then it resolves the machine by the `{code}` rule of
/// `api.md` section 2.7 in a `ReadOnly` transaction. Then one `ReadCommitted` transaction locks
/// `machine FOR SHARE`, reads the recipe, and locks `plc_connection FOR SHARE`. The checks run in
/// a fixed order: archive state, recipe machine, record. Then the cipher opens the password.
/// `timeout.export` covers the permit, the reads, the locks, and the `open`. The use case then
/// commits the entry `PlcConnectionExported` and returns the record. The use case runs in
/// read-only mode (A24).
#[derive(Clone)]
pub struct ExportPlcConnection {
    ctx: AppContext,
    cipher: Arc<dyn CredentialCipher>,
}

impl UseCase for ExportPlcConnection {
    const OPERATION: Operation = Operation::ExportPlcConnection;
}

impl ExportPlcConnection {
    /// Build the use case over a context and the cipher that opens the password.
    #[must_use]
    pub const fn new(ctx: AppContext, cipher: Arc<dyn CredentialCipher>) -> Self {
        Self { ctx, cipher }
    }

    /// Read the record, open its password, and commit the entry `PlcConnectionExported`.
    ///
    /// The method follows the flow of design D9. The commit runs after the limit `timeout.export`,
    /// so a timeout never hides a committed entry. The method returns the record only after the
    /// commit.
    ///
    /// # Errors
    ///
    /// Returns `Forbidden` (`E-WEB-002`) when the role lacks `VersionExport`.
    /// Returns `NotFound` (`E-WEB-005`) when no machine has the code or no recipe has the reference.
    /// Returns `PlantError::MachineArchived` (`E-PLT-010`) when the machine is archived.
    /// Returns `PlantError::ConnectionRecipeMismatch` (`E-PLT-029`) when the recipe belongs to another machine.
    /// Returns `PlantError::ConnectionNotConfigured` (`E-PLT-027`) when the machine has no record.
    /// Returns `SecretUnreadable` (`E-WEB-014`) when the cipher cannot open the stored password.
    /// Returns `Timeout` (`E-WEB-009`) when the limit `timeout.export` ends first.
    /// Returns `AuditWriteFailed` (`E-AUD-001`) when the audit insert fails.
    /// Returns `AppError::Storage` when the database fails.
    #[tracing::instrument(name = "use_case", skip_all, fields(operation = %Self::OPERATION))]
    pub async fn execute(
        &self,
        user: &CurrentUser,
        meta: &RequestMeta,
        input: ExportPlcConnectionInput,
    ) -> Result<ExportedPlcConnection, AppError> {
        authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?;
        let Prepared { tx, id, exported } = self
            .ctx
            .settings
            .blocking
            .export(async {
                let _permit = self.ctx.export_gate.acquire().await?;
                self.read(input).await
            })
            .await?;
        let entry = exported_entry(id, &exported, (user, meta));
        self.ctx.uow.commit(tx, NonEmpty::new(entry)).await?;
        Ok(exported)
    }

    /// Run the reads, the locks, the checks, and the `open` in one `ReadCommitted` transaction.
    ///
    /// The method returns the open transaction. The caller commits it after the limit
    /// `timeout.export`. An error or a timeout drops the transaction, which rolls it back.
    async fn read(&self, input: ExportPlcConnectionInput) -> Result<Prepared, AppError> {
        let machine = self.resolve_machine(input.code).await?;
        let mut tx = self.ctx.uow.begin(Isolation::ReadCommitted).await?;
        let locked = tx
            .plant()
            .lock_machine_shared(machine)
            .await?
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(input.code),
            })?;
        let recipe = tx
            .recipes()
            .recipe_by_reference(&input.recipe)
            .await?
            .ok_or_else(|| AppError::NotFound {
                entity: Missing::Recipe(input.recipe.clone()),
            })?;
        let shared = tx.plc_connections().lock_connection_shared(machine).await?;
        let code = locked.state.code();
        if !locked.state.is_live() {
            return Err(PlantError::MachineArchived { code }.into());
        }
        if recipe.machine() != machine {
            return Err(PlantError::ConnectionRecipeMismatch {
                code,
                reference: input.recipe,
            }
            .into());
        }
        let shared = shared.ok_or(PlantError::ConnectionNotConfigured { code })?;
        let password = shared
            .sealed
            .map(|sealed| self.cipher.open(machine, &sealed))
            .transpose()
            .map_err(|_fault| AppError::SecretUnreadable { code })?;
        let id = shared.stored.id;
        let exported = build_exported(
            machine,
            &locked.state,
            input.recipe,
            shared.stored,
            password,
        );
        Ok(Prepared { tx, id, exported })
    }

    /// Resolve the machine of the URL by the `{code}` rule of `api.md` section 2.7.
    ///
    /// The live machine with the code wins. Without one, the archived machine with the latest
    /// `archived_at` wins. The transaction takes no lock and ends before the function returns.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` (`E-WEB-005`) when no machine has the code.
    async fn resolve_machine(&self, code: MachineCode) -> Result<MachineId, AppError> {
        let mut tx = self.ctx.uow.begin(Isolation::ReadOnly).await?;
        let machine = tx.plant().export_machine_by_code(code).await?;
        machine
            .map(|machine| machine.id())
            .ok_or(AppError::NotFound {
                entity: Missing::Machine(code),
            })
    }
}
