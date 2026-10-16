//! A test adapter that returns one row image less than the port contract promises.

use std::sync::Arc;

use async_trait::async_trait;
use recipe_domain::audit::AuditEntry;
use recipe_domain::plant::{Machine, MachineCode, MachineId, Unit, UnitId, UnitPath, UnitUsage};
use recipe_domain::text::Text;

use crate::context::AppContext;
use crate::error::StorageError;
use crate::memory::fixtures::Fixture;
use crate::ports::{
    AttemptRepository, AuditReads, AuditWritten, DiffReads, DraftRepository, InsertedMachine,
    Isolation, Locked, MachineFilter, MachineSummary, NonEmpty, PlantRepository, RecipeReads,
    RecipeRepository, RowImage, SessionRepository, Transaction, TypeCatalog, UnitLookup,
    UnitOfWork, UserRepository,
};

/// A unit of work whose transactions drop the last row image of `insert_machine` and `archive_units`.
struct ShortRowsStore {
    inner: Arc<dyn UnitOfWork>,
}

#[async_trait]
impl UnitOfWork for ShortRowsStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        let inner = self.inner.begin(isolation).await?;
        Ok(Box::new(ShortRowsTransaction { inner }))
    }
}

/// The transaction of [`ShortRowsStore`]. It forwards every call to the inner transaction.
struct ShortRowsTransaction {
    inner: Box<dyn Transaction>,
}

#[async_trait]
impl Transaction for ShortRowsTransaction {
    fn recipes(&mut self) -> &mut dyn RecipeRepository {
        self.inner.recipes()
    }

    fn recipe_reads(&mut self) -> &mut dyn RecipeReads {
        self.inner.recipe_reads()
    }

    fn audit_reads(&mut self) -> &mut dyn AuditReads {
        self.inner.audit_reads()
    }

    fn diff_reads(&mut self) -> &mut dyn DiffReads {
        self.inner.diff_reads()
    }

    fn drafts(&mut self) -> &mut dyn DraftRepository {
        self.inner.drafts()
    }

    fn plant(&mut self) -> &mut dyn PlantRepository {
        self
    }

    fn units(&mut self) -> &mut dyn UnitLookup {
        self.inner.units()
    }

    fn types(&mut self) -> &mut dyn TypeCatalog {
        self.inner.types()
    }

    fn users(&mut self) -> &mut dyn UserRepository {
        self.inner.users()
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self.inner.sessions()
    }

    fn attempts(&mut self) -> &mut dyn AttemptRepository {
        self.inner.attempts()
    }

    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError> {
        self.inner.append_audit(entries).await
    }

    async fn finish(self: Box<Self>, written: AuditWritten) -> Result<(), StorageError> {
        self.inner.finish(written).await
    }
}

#[async_trait]
impl PlantRepository for ShortRowsTransaction {
    async fn machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        self.inner.plant().machine_by_code(code).await
    }

    async fn archived_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        self.inner.plant().archived_machine_by_code(code).await
    }

    async fn machine_by_id(&mut self, id: MachineId) -> Result<Option<Machine>, StorageError> {
        self.inner.plant().machine_by_id(id).await
    }

    async fn list_machines(
        &mut self,
        filter: &MachineFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<MachineSummary>, StorageError> {
        self.inner
            .plant()
            .list_machines(filter, offset, limit)
            .await
    }

    async fn lock_machine_shared(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.inner.plant().lock_machine_shared(id).await
    }

    async fn lock_machine_for_no_key_update(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.inner.plant().lock_machine_for_no_key_update(id).await
    }

    async fn insert_machine(&mut self, machine: &Machine) -> Result<InsertedMachine, StorageError> {
        let mut inserted = self.inner.plant().insert_machine(machine).await?;
        inserted.units.pop();
        Ok(inserted)
    }

    async fn rename_machine(
        &mut self,
        id: MachineId,
        name: &Text,
    ) -> Result<RowImage, StorageError> {
        self.inner.plant().rename_machine(id, name).await
    }

    async fn archive_machine(&mut self, id: MachineId) -> Result<RowImage, StorageError> {
        self.inner.plant().archive_machine(id).await
    }

    async fn insert_unit(
        &mut self,
        machine: MachineId,
        unit: &Unit,
    ) -> Result<RowImage, StorageError> {
        self.inner.plant().insert_unit(machine, unit).await
    }

    async fn lock_unit_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Locked<Unit>>, StorageError> {
        self.inner.plant().lock_unit_for_update(machine, path).await
    }

    async fn rename_unit(&mut self, unit: UnitId, name: &Text) -> Result<RowImage, StorageError> {
        self.inner.plant().rename_unit(unit, name).await
    }

    async fn lock_unit_tree_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Vec<Locked<Unit>>, StorageError> {
        self.inner
            .plant()
            .lock_unit_tree_for_update(machine, path)
            .await
    }

    async fn archive_units(&mut self, units: &[UnitId]) -> Result<Vec<RowImage>, StorageError> {
        let mut images = self.inner.plant().archive_units(units).await?;
        images.pop();
        Ok(images)
    }

    async fn unit_usage(
        &mut self,
        machine: MachineId,
        units: &[UnitId],
    ) -> Result<UnitUsage, StorageError> {
        self.inner.plant().unit_usage(machine, units).await
    }
}

/// Return a context over `fixture` whose plant adapter returns one row image too few.
pub(super) fn short_rows_context(fixture: &Fixture) -> AppContext {
    let base = fixture.context();
    AppContext::new(
        Arc::new(ShortRowsStore {
            inner: Arc::clone(&base.uow),
        }),
        base.security,
        base.clock,
        base.settings,
    )
}
