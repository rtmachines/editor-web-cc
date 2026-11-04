//! Steps 4 and 5 of the export flow: read the rows of the body, then build, check, and write it.
//!
//! See design D6 (c) to (e) and (h) of the change `export-v1`.

use std::sync::Arc;

use recipe_domain::export::{
    DocumentFacts, ExportError, ExportHeader, ExportStatus, HashedBody, ScopeBody, ScopeFacts,
    Sha256, check_version_parameters,
};
use recipe_domain::identity::Login;
use recipe_domain::plant::{LiveUnits, Machine, PlantError, PlatformTypes, TypeTrees, UnitPath};
use recipe_domain::recipe::{Parameter, RecipeVersionId, ReferenceNumber};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

use super::scope::{ExportScope, ScopeAudit};
use crate::blocking::{Blocking, ExportPermit};
use crate::error::{AppError, Missing, StorageError};
use crate::ports::{DocumentWriter, Transaction};

/// The rows of the locked version that the hashed body needs (step 4).
pub(super) struct BodyRows {
    header: ExportHeader,
    parameters: Vec<Parameter>,
    units: LiveUnits,
    trees: TypeTrees,
}

impl BodyRows {
    /// Check that `path` names a unit that the export can use (ADR-0109 (ja)).
    ///
    /// On a live machine, an archived unit counts as no unit. On an archived machine, every
    /// unit counts. The check reads the units of step 4, so it runs before the blocking task.
    ///
    /// # Errors
    ///
    /// Returns `NotFound` (`E-WEB-005`) when no unit has `path` or the unit is archived on a live
    /// machine.
    pub(super) fn require_unit(&self, path: &UnitPath) -> Result<(), AppError> {
        let usable = match self.units.require(path) {
            Ok(_live) => true,
            Err(PlantError::UnitArchived { .. }) => self.units.is_machine_archived(),
            Err(_missing) => false,
        };
        if usable {
            return Ok(());
        }
        Err(AppError::NotFound {
            entity: Missing::Unit {
                code: self.units.machine_code(),
                path: path.clone(),
            },
        })
    }
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
    /// Add the checked hash, the result of the rule check, and the English names of the parameters.
    ///
    /// `has_rule_fault` is `true` when a stored part breaks the current rules (ADR-0119).
    fn complete(
        self,
        rows: &BodyRows,
        content_sha256: Sha256,
        has_rule_fault: bool,
    ) -> DocumentFacts {
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
            has_rule_fault,
        }
    }
}

/// The result of the blocking task: the bytes of the document and the checked hash.
pub(super) struct Written {
    /// The bytes that the port `DocumentWriter` wrote.
    pub(super) document: Vec<u8>,
    /// The hash of the body. It equals the stored hash.
    pub(super) hash: Sha256,
    /// The unit and the hash of the scope body, or `None` for an export of the whole version.
    pub(super) scope: Option<ScopeAudit>,
    /// `true` when the rule check found a fault in the whole version.
    pub(super) has_rule_fault: bool,
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

/// Write the document of `scope` from the checked whole body (step 5, after the hash check).
///
/// The function solves one problem: the scope filter must run after the hash check of the whole
/// version (ADR-0109 (w)). The caller has compared the hash already.
///
/// # Errors
///
/// Returns `E-EXP-005` when the scope holds no parameter, and `E-EXP-004` when the writer fails.
fn write_scope(
    writer: &dyn DocumentWriter,
    (body, facts): (&HashedBody, DocumentFacts),
    (scope, header): (&ExportScope, &ExportHeader),
) -> Result<(Vec<u8>, Option<ScopeAudit>), ExportError> {
    let ExportScope::Unit(path) = scope else {
        return Ok((writer.write(body, &facts)?, None));
    };
    let scoped = ScopeBody::build(body, path);
    if scoped.is_empty() {
        return Err(ExportError::EmptyScope {
            reference: header.reference.clone(),
            number: header.version,
            path: path.clone(),
        });
    }
    let sha256 = scoped.sha256()?;
    let document = writer.write_scoped(
        &scoped,
        &ScopeFacts {
            document: facts,
            scope_sha256: sha256,
        },
    )?;
    let audit = ScopeAudit {
        unit_path: path.clone(),
        sha256,
    };
    Ok((document, Some(audit)))
}

/// Build the body, compare its hash with `stored`, and write the document in one blocking task.
///
/// The function solves one problem: the build, the hash, and the writer use the CPU, so they
/// run in `spawn_blocking`, and the limit `timeout.export` can stop the wait (step 5). The task
/// holds `permit` of the export gate until it ends (ADR-0056 (c)). The task compares the hash of
/// the whole version before it filters the scope (ADR-0109 (w)). Then it checks the whole version
/// against the current rule set, also for a unit export (ADR-0119 (f)).
///
/// # Errors
///
/// Returns `E-EXP-002` when the body cannot be built or its hash differs from `stored`.
/// Returns `E-DB-008` when a stored part breaks the earlier rule set (ADR-0119 (k)).
/// Returns `E-EXP-005` when `scope` holds no parameter.
/// Returns `E-EXP-004` when the writer fails, and `E-WEB-011` when the task panics.
pub(super) async fn write_document(
    (blocking, permit): (Blocking, ExportPermit),
    writer: Arc<dyn DocumentWriter>,
    (rows, facts): (BodyRows, KnownFacts),
    (stored, scope): (Option<Sha256>, ExportScope),
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
            let found =
                check_version_parameters(&rows.parameters, &rows.trees).map_err(|broken| {
                    let (key, fault) = broken.into_parts();
                    StorageError::broken_stored_parameter(key, fault)
                })?;
            let has_rule_fault = !found.is_empty();
            let facts = facts.complete(&rows, hash, has_rule_fault);
            let (document, scope) =
                write_scope(writer.as_ref(), (&body, facts), (&scope, &rows.header))?;
            Ok(Written {
                document,
                hash,
                scope,
                has_rule_fault,
            })
        })
        .await?
}
