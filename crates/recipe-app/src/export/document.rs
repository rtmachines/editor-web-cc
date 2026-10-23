//! Steps 4 and 5 of the export flow: read the rows of the body, then build, check, and write it.
//!
//! See design D6 (c) to (e) and (h) of the change `export-v1`.

use std::sync::Arc;

use recipe_domain::export::{
    DocumentFacts, ExportError, ExportHeader, ExportStatus, HashedBody, Sha256,
};
use recipe_domain::identity::Login;
use recipe_domain::plant::{LiveUnits, Machine, PlatformTypes, TypeTrees};
use recipe_domain::recipe::{Parameter, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

use crate::blocking::{Blocking, ExportPermit};
use crate::error::{AppError, Missing};
use crate::ports::{DocumentWriter, Transaction};

/// The rows of the locked version that the hashed body needs (step 4).
pub(super) struct BodyRows {
    header: ExportHeader,
    parameters: Vec<Parameter>,
    units: LiveUnits,
    trees: TypeTrees,
}

/// The facts of the document that the flow knows before the blocking task.
///
/// The type solves one problem: the blocking task computes the hash, so `content_sha256` and the
/// names of the parameters join the facts there.
pub(super) struct KnownFacts {
    /// The instant of the export. The flow reads the clock once before the task.
    pub(super) exported_at: Timestamp,
    /// The login of the user that exports the version.
    pub(super) exported_by: Login,
    /// The status of the version after the state check.
    pub(super) status: ExportStatus,
    /// The instant of the approval of the version.
    pub(super) approved_at: Timestamp,
    /// The login of the user that approved the version.
    pub(super) approved_by: Login,
    /// The name of the machine.
    pub(super) machine_name: Text,
    /// The name of the recipe under the lock of the version.
    pub(super) recipe_name: Text,
}

impl KnownFacts {
    /// Add the checked hash and the English names of the parameters of `rows`.
    fn complete(self, rows: &BodyRows, content_sha256: Sha256) -> DocumentFacts {
        DocumentFacts {
            exported_at: self.exported_at,
            exported_by: self.exported_by,
            status: self.status,
            approved_at: self.approved_at,
            approved_by: self.approved_by,
            machine_name: self.machine_name,
            recipe_name: self.recipe_name,
            content_sha256,
            names: rows
                .parameters
                .iter()
                .map(|parameter| (parameter.key(), parameter.name_en().clone()))
                .collect(),
        }
    }
}

/// The result of the blocking task: the bytes of the document and the checked hash.
pub(super) struct Written {
    /// The bytes that the port `DocumentWriter` wrote.
    pub(super) document: Vec<u8>,
    /// The hash of the body. It equals the stored hash.
    pub(super) hash: Sha256,
}

/// Read the parameters, the units, and the types of the version (step 4).
///
/// # Errors
///
/// Returns `NotFound` when the machine has no units, and `Storage` or `Plant` errors.
pub(super) async fn read_rows(
    tx: &mut dyn Transaction,
    machine: &Machine,
    (reference, version, number): (&ReferenceNumber, RecipeVersionId, u32),
) -> Result<BodyRows, AppError> {
    let machine_id = machine.id();
    let parameters: Vec<Parameter> = tx
        .recipes()
        .version_parameters(version)
        .await?
        .into_iter()
        .map(Parameter::from)
        .collect();
    let units = tx
        .units()
        .live_units(machine_id)
        .await?
        .ok_or_else(|| AppError::NotFound {
            entity: Missing::MachineId(machine_id),
        })?;
    let rows = tx.types().type_rows(machine_id).await?;
    let types = PlatformTypes::collect(machine_id, machine.platform(), rows)?;
    let trees = TypeTrees::resolve(parameters.iter().map(Parameter::plc_type), &types);
    let header = ExportHeader {
        machine: machine.code(),
        platform: machine.platform(),
        reference: reference.clone(),
        version: number,
    };
    Ok(BodyRows {
        header,
        parameters,
        units,
        trees,
    })
}

/// Build the body, compare its hash with `stored`, and write the document in one blocking task.
///
/// The function solves one problem: the build, the hash, and the writer use the CPU, so they
/// run in `spawn_blocking`, and the limit `timeout.export` can stop the wait (step 5). The task
/// holds `permit` of the export gate until it ends (ADR-0056 (c)).
///
/// # Errors
///
/// Returns `E-EXP-002` when the body cannot be built or its hash differs from `stored`.
/// Returns `E-EXP-004` when the writer fails, and `E-WEB-011` when the task panics.
pub(super) async fn write_document(
    (blocking, permit): (Blocking, ExportPermit),
    writer: Arc<dyn DocumentWriter>,
    (rows, facts): (BodyRows, KnownFacts),
    stored: Option<Sha256>,
) -> Result<Written, AppError> {
    blocking
        .rebuild_export(permit, move || -> Result<Written, AppError> {
            let mismatch = || ExportError::ContentHashMismatch {
                reference: rows.header.reference.clone(),
                number: rows.header.version,
            };
            let body = HashedBody::build(&rows.header, &rows.parameters, &rows.units, &rows.trees)
                .map_err(|_build_error| mismatch())?;
            let hash = body.sha256()?;
            if stored != Some(hash) {
                return Err(mismatch().into());
            }
            let facts = facts.complete(&rows, hash);
            let document = writer.write(&body, &facts)?;
            Ok(Written { document, hash })
        })
        .await?
}
