//! Test of design D16: `ListParameters` reads a fixed number of statements (A19, A26).
//!
//! A counting unit of work wraps the memory store. Its transaction counts every method call of
//! `RecipeReads`, `PlantRepository`, and `TypeCatalog`, and every accessor call of the other
//! ports. So a loop that reuses one borrow of a port still adds to the count. The count for a
//! draft with 40 parameters equals the count for a draft with 4 000 parameters. The count for
//! 4 units equals the count for 40 units.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use async_trait::async_trait;
use parking_lot::Mutex;
use recipe_domain::audit::AuditEntry;
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Unit, UnitId,
    UnitPath, UnitUsage,
};
use recipe_domain::recipe::{
    LangCode, Ordinal, ParameterKey, RecipeId, RecipeVersionId, StoredParameter, StoredValue,
};
use recipe_domain::text::Text;
use recipe_domain::timestamp::Timestamp;

use super::parameters::{ListParameters, ListParametersInput, ParameterList};
use super::parameters_tests::typed_world;
use super::test_world::seed_users;
use super::versions::{GetVersion, GetVersionInput};
use crate::blocking::{AppSettings, PageSizes};
use crate::context::AppContext;
use crate::error::StorageError;
use crate::memory::MemoryStore;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, scenarios, text, uid, vid,
};
use crate::ports::{
    AttemptRepository, AuditReads, AuditWritten, DerivedTypeLock, DiffReads, DraftRepository,
    Ensured, ExportableRecipes, InsertedMachine, Isolation, Locked, MachineFilter, MachineSummary,
    NonEmpty, ParameterRow, PlantRepository, PlcConnections, RecipeFilter, RecipeReads,
    RecipeRepository, RecipeSummary, RowImage, SessionRepository, SettingRepository, Transaction,
    TypeCatalog, TypeUsageRow, UnitCount, UnitLookup, UnitOfWork, UserRepository, VersionDetail,
    VersionSummary,
};

/// The platform of the test machine.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;
/// The number of main units of the large machine.
const UNITS: u128 = 40;
/// The port calls of `ListParameters`: the recipe, the machine, the version detail, the unit
/// counts, the page of rows, the type rows, and the values of the primitive rows.
const READS: usize = 7;

/// What the counting transactions record: the count of port calls and the `keys` argument of
/// each `parameter_values` call.
#[derive(Default)]
struct Probe {
    calls: AtomicUsize,
    value_keys: Mutex<Vec<Vec<ParameterKey>>>,
}

/// A unit of work that counts the port calls of every transaction that it opens.
struct CountingStore {
    inner: MemoryStore,
    probe: Arc<Probe>,
}

#[async_trait]
impl UnitOfWork for CountingStore {
    async fn begin(&self, isolation: Isolation) -> Result<Box<dyn Transaction>, StorageError> {
        let inner = self.inner.begin(isolation).await?;
        Ok(Box::new(CountingTransaction {
            inner,
            probe: Arc::clone(&self.probe),
        }))
    }
}

/// A transaction that counts each `RecipeReads` call and each accessor call of another port.
struct CountingTransaction {
    inner: Box<dyn Transaction>,
    probe: Arc<Probe>,
}

impl CountingTransaction {
    fn count(&self) {
        self.probe.calls.fetch_add(1, Ordering::Relaxed);
    }
}

#[async_trait]
impl Transaction for CountingTransaction {
    fn recipes(&mut self) -> &mut dyn RecipeRepository {
        self.count();
        self.inner.recipes()
    }

    fn recipe_reads(&mut self) -> &mut dyn RecipeReads {
        self
    }

    fn audit_reads(&mut self) -> &mut dyn AuditReads {
        self.count();
        self.inner.audit_reads()
    }

    fn diff_reads(&mut self) -> &mut dyn DiffReads {
        self.count();
        self.inner.diff_reads()
    }

    fn drafts(&mut self) -> &mut dyn DraftRepository {
        self.count();
        self.inner.drafts()
    }

    fn plant(&mut self) -> &mut dyn PlantRepository {
        self
    }

    fn plc_connections(&mut self) -> &mut dyn PlcConnections {
        self.count();
        self.inner.plc_connections()
    }

    fn units(&mut self) -> &mut dyn UnitLookup {
        self.count();
        self.inner.units()
    }

    fn types(&mut self) -> &mut dyn TypeCatalog {
        self
    }

    fn users(&mut self) -> &mut dyn UserRepository {
        self.count();
        self.inner.users()
    }

    fn sessions(&mut self) -> &mut dyn SessionRepository {
        self.count();
        self.inner.sessions()
    }

    fn attempts(&mut self) -> &mut dyn AttemptRepository {
        self.count();
        self.inner.attempts()
    }

    fn settings(&mut self) -> &mut dyn SettingRepository {
        self.count();
        self.inner.settings()
    }

    async fn append_audit(&mut self, entries: &NonEmpty<AuditEntry>) -> Result<(), StorageError> {
        self.count();
        self.inner.append_audit(entries).await
    }

    async fn finish(self: Box<Self>, written: AuditWritten) -> Result<(), StorageError> {
        self.inner.finish(written).await
    }
}

#[async_trait]
impl RecipeReads for CountingTransaction {
    async fn list_recipes(
        &mut self,
        filter: &RecipeFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<RecipeSummary>, StorageError> {
        self.count();
        self.inner
            .recipe_reads()
            .list_recipes(filter, offset, limit)
            .await
    }

    async fn version_page(
        &mut self,
        recipe: RecipeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<VersionSummary>, StorageError> {
        self.count();
        self.inner
            .recipe_reads()
            .version_page(recipe, offset, limit)
            .await
    }

    async fn version_detail(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Option<VersionDetail>, StorageError> {
        self.count();
        self.inner.recipe_reads().version_detail(version).await
    }

    async fn last_change(
        &mut self,
        recipe: RecipeId,
        at: Timestamp,
    ) -> Result<Option<Login>, StorageError> {
        self.count();
        self.inner.recipe_reads().last_change(recipe, at).await
    }

    async fn parameter_page(
        &mut self,
        version: RecipeVersionId,
        unit: Option<UnitId>,
        lang: Option<&LangCode>,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<ParameterRow>, StorageError> {
        self.count();
        self.inner
            .recipe_reads()
            .parameter_page(version, unit, lang, offset, limit)
            .await
    }

    async fn parameter_values(
        &mut self,
        version: RecipeVersionId,
        keys: &[ParameterKey],
    ) -> Result<BTreeMap<ParameterKey, StoredValue>, StorageError> {
        self.count();
        self.probe.value_keys.lock().push(keys.to_vec());
        self.inner
            .recipe_reads()
            .parameter_values(version, keys)
            .await
    }

    async fn unit_counts(
        &mut self,
        version: RecipeVersionId,
    ) -> Result<Vec<UnitCount>, StorageError> {
        self.count();
        self.inner.recipe_reads().unit_counts(version).await
    }

    async fn parameter_by_key(
        &mut self,
        version: RecipeVersionId,
        key: ParameterKey,
    ) -> Result<Option<StoredParameterAlias>, StorageError> {
        self.count();
        self.inner
            .recipe_reads()
            .parameter_by_key(version, key)
            .await
    }

    async fn exportable_recipes(
        &mut self,
        machine: MachineCode,
    ) -> Result<Option<ExportableRecipes>, StorageError> {
        self.count();
        self.inner.recipe_reads().exportable_recipes(machine).await
    }
}

/// The stored parameter type, named here so the forwarding signature stays on one line.
type StoredParameterAlias = recipe_domain::recipe::StoredParameter;

#[async_trait]
impl PlantRepository for CountingTransaction {
    async fn machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        self.count();
        self.inner.plant().machine_by_code(code).await
    }

    async fn archived_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        self.count();
        self.inner.plant().archived_machine_by_code(code).await
    }

    async fn export_machine_by_code(
        &mut self,
        code: MachineCode,
    ) -> Result<Option<Machine>, StorageError> {
        self.count();
        self.inner.plant().export_machine_by_code(code).await
    }

    async fn machine_by_id(&mut self, id: MachineId) -> Result<Option<Machine>, StorageError> {
        self.count();
        self.inner.plant().machine_by_id(id).await
    }

    async fn list_machines(
        &mut self,
        filter: &MachineFilter,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<MachineSummary>, StorageError> {
        self.count();
        self.inner
            .plant()
            .list_machines(filter, offset, limit)
            .await
    }

    async fn lock_machine_shared(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.count();
        self.inner.plant().lock_machine_shared(id).await
    }

    async fn lock_machine_for_no_key_update(
        &mut self,
        id: MachineId,
    ) -> Result<Option<Locked<Machine>>, StorageError> {
        self.count();
        self.inner.plant().lock_machine_for_no_key_update(id).await
    }

    async fn insert_machine(&mut self, machine: &Machine) -> Result<InsertedMachine, StorageError> {
        self.count();
        self.inner.plant().insert_machine(machine).await
    }

    async fn rename_machine(
        &mut self,
        id: MachineId,
        name: &Text,
    ) -> Result<RowImage, StorageError> {
        self.count();
        self.inner.plant().rename_machine(id, name).await
    }

    async fn archive_machine(&mut self, id: MachineId) -> Result<RowImage, StorageError> {
        self.count();
        self.inner.plant().archive_machine(id).await
    }

    async fn insert_unit(
        &mut self,
        machine: MachineId,
        unit: &Unit,
    ) -> Result<RowImage, StorageError> {
        self.count();
        self.inner.plant().insert_unit(machine, unit).await
    }

    async fn lock_unit_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Option<Locked<Unit>>, StorageError> {
        self.count();
        self.inner.plant().lock_unit_for_update(machine, path).await
    }

    async fn rename_unit(&mut self, unit: UnitId, name: &Text) -> Result<RowImage, StorageError> {
        self.count();
        self.inner.plant().rename_unit(unit, name).await
    }

    async fn lock_unit_tree_for_update(
        &mut self,
        machine: MachineId,
        path: &UnitPath,
    ) -> Result<Vec<Locked<Unit>>, StorageError> {
        self.count();
        self.inner
            .plant()
            .lock_unit_tree_for_update(machine, path)
            .await
    }

    async fn archive_units(&mut self, units: &[UnitId]) -> Result<Vec<RowImage>, StorageError> {
        self.count();
        self.inner.plant().archive_units(units).await
    }

    async fn unit_usage(
        &mut self,
        machine: MachineId,
        units: &[UnitId],
    ) -> Result<UnitUsage, StorageError> {
        self.count();
        self.inner.plant().unit_usage(machine, units).await
    }
}

#[async_trait]
impl TypeCatalog for CountingTransaction {
    async fn type_rows(&mut self, machine: MachineId) -> Result<Vec<PlcType>, StorageError> {
        self.count();
        self.inner.types().type_rows(machine).await
    }

    async fn type_usage(
        &mut self,
        machine: MachineId,
        type_id: PlcTypeId,
        offset: u64,
        limit: u32,
    ) -> Result<Vec<TypeUsageRow>, StorageError> {
        self.count();
        self.inner
            .types()
            .type_usage(machine, type_id, offset, limit)
            .await
    }

    async fn lock_derived_types(
        &mut self,
        machine: MachineId,
        keys: &[DerivedTypeLock],
    ) -> Result<(), StorageError> {
        self.count();
        self.inner.types().lock_derived_types(machine, keys).await
    }

    async fn ensure_derived(&mut self, record: PlcTypeRecord) -> Result<Ensured, StorageError> {
        self.count();
        self.inner.types().ensure_derived(record).await
    }

    async fn insert_struct(&mut self, row: &PlcType) -> Result<RowImage, StorageError> {
        self.count();
        self.inner.types().insert_struct(row).await
    }
}

/// A machine `M1` with the main units `U1` to `U<units>`. The unit `Uk` has the identifier `100 + k`.
fn machine_with(units: u128) -> Machine {
    let mut machine = Machine::create(
        mid(1),
        MachineCode::parse("M1").unwrap(),
        text("Press"),
        PLATFORM,
        (uid(101), text("Main")),
    );
    for k in 2..=units {
        machine
            .add_unit(
                uid(100_u128.checked_add(k).unwrap()),
                None,
                &k.to_string(),
                text(&format!("Unit {k}")),
            )
            .unwrap();
    }
    machine
}

/// The draft `R-1` of the large machine with `per_unit` parameters on each of its 40 units.
fn large_draft(per_unit: u128) -> Fixture {
    draft_on_units(UNITS, per_unit)
}

/// The draft `R-1` of a machine with `units` main units and `per_unit` parameters on each unit.
fn draft_on_units(units: u128, per_unit: u128) -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = machine_with(units);
    fixture.store.seed_machine(&machine);
    fixture.seed_draft(&machine, 1, "R-1");
    for k in 1..=units {
        for i in 0..per_unit {
            let n = k.checked_mul(1_000).unwrap().checked_add(i).unwrap();
            let offset = u16::try_from(i.checked_mul(2).unwrap()).unwrap();
            let parameter = parameter_on(
                PLATFORM,
                n,
                &format!("P{n}"),
                uid(100_u128.checked_add(k).unwrap()),
                offset,
            );
            fixture.store.seed_parameter(vid(101), &parameter);
        }
    }
    fixture
}

/// The context of `fixture` over a counting unit of work, and the probe that the unit of work fills.
fn counting_context(fixture: &Fixture, settings: AppSettings) -> (AppContext, Arc<Probe>) {
    let probe = Arc::new(Probe::default());
    let base = fixture.context_with(settings);
    let ctx = AppContext::new(
        Arc::new(CountingStore {
            inner: fixture.store.clone(),
            probe: Arc::clone(&probe),
        }),
        base.security,
        base.clock,
        base.settings,
    );
    (ctx, probe)
}

/// Run `ListParameters` of `R-1` without a unit and return the result and the probe.
async fn probed_list(fixture: &Fixture, settings: AppSettings) -> (ParameterList, Arc<Probe>) {
    let (ctx, probe) = counting_context(fixture, settings);
    let result = ListParameters::new(ctx)
        .execute(
            &actor(Role::Operator),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: None,
                page: 0,
                lang: None,
            },
        )
        .await
        .unwrap();
    (result, probe)
}

/// Run `ListParameters` without a unit on `fixture` and return the result and the port calls.
async fn counted_list(fixture: &Fixture) -> (ParameterList, usize) {
    let (result, probe) = probed_list(fixture, AppSettings::default()).await;
    (result, probe.calls.load(Ordering::Relaxed))
}

#[tokio::test]
async fn the_count_of_port_calls_does_not_grow_with_the_count_of_parameters() {
    let small = large_draft(1);
    let large = large_draft(100);

    let (small_list, small_calls) = counted_list(&small).await;
    let (large_list, large_calls) = counted_list(&large).await;

    assert_eq!(small_list.rows.len(), 40);
    assert_eq!(small_list.units.len(), 40);
    assert_eq!(large_list.rows.len(), 100);
    assert_eq!(large_list.units.len(), 40);
    assert_eq!(
        large_list.unit.as_ref().map(ToString::to_string),
        Some("U1".to_owned())
    );
    assert_eq!(small_calls, READS, "the query reads through the ports");
    assert_eq!(
        small_calls, large_calls,
        "40 parameters: {small_calls} calls, 4 000: {large_calls}"
    );
}

#[tokio::test]
async fn the_count_of_port_calls_does_not_grow_with_the_count_of_units() {
    let few = draft_on_units(4, 1);
    let many = draft_on_units(UNITS, 1);

    let (few_list, few_calls) = counted_list(&few).await;
    let (many_list, many_calls) = counted_list(&many).await;

    assert_eq!((few_list.rows.len(), few_list.units.len()), (4, 4));
    assert_eq!((many_list.rows.len(), many_list.units.len()), (40, 40));
    assert_eq!(few_calls, READS, "the query reads through the ports");
    assert_eq!(
        few_calls, many_calls,
        "4 units: {few_calls} calls, 40 units: {many_calls}"
    );
}

/// The settings with `size` rows on a page of the parameter table.
fn pages_of(size: u32) -> AppSettings {
    AppSettings {
        pages: PageSizes {
            parameters: size,
            ..PageSizes::default()
        },
        ..AppSettings::default()
    }
}

/// `typed_world` with the `INT` parameter `RATE` (key 4) after the four rows of the page.
///
/// The row uses a type that the page uses too. So only the page cut keeps its key out.
///
/// The rows of `U1` in table order are `ZONE1` (struct, key 11), `SPEED` (`INT`, key 1),
/// `VALUES` (array, key 2), `LABEL` (`STRING(20)`, key 3), and `RATE` (`INT`, key 4).
fn world_with_a_row_past_the_page() -> Fixture {
    let fixture = typed_world();
    let parameter = StoredParameter {
        ordinal: Ordinal::parse(4).unwrap(),
        ..parameter_on(PLATFORM, 4, "RATE", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(101), &parameter);
    fixture
}

#[tokio::test]
async fn list_parameters_reads_the_values_of_every_row_of_the_cut_page_only() {
    let fixture = world_with_a_row_past_the_page();

    let (list, probe) = probed_list(&fixture, pages_of(4)).await;

    let shown: Vec<ParameterKey> = list.rows.iter().map(|row| row.key).collect();
    assert_eq!(
        shown,
        [pkey(11), pkey(1), pkey(2), pkey(3)],
        "the page holds a struct, an INT, an array, and a STRING"
    );
    assert!(list.has_next, "the INT row 4 lies past the page");
    let mut calls = probe.value_keys.lock().clone();
    assert_eq!(calls.len(), 1, "one call of parameter_values");
    let mut keys = calls.remove(0);
    keys.sort();
    assert_eq!(
        keys,
        [pkey(1), pkey(2), pkey(3), pkey(11)],
        "every row of the cut page, because the address rules need the composite rows too"
    );
}

#[tokio::test]
async fn get_version_does_not_read_parameter_values() {
    let fixture = scenarios::approved_and_draft();
    seed_users(&fixture);
    for number in [1, 2] {
        let (ctx, probe) = counting_context(&fixture, AppSettings::default());

        GetVersion::new(ctx)
            .execute(
                &actor(Role::Operator),
                &meta(),
                GetVersionInput {
                    reference: reference("R-1"),
                    number,
                    rebind_bound: 16_384,
                },
            )
            .await
            .unwrap();

        assert!(
            probe.value_keys.lock().is_empty(),
            "version {number}: GetVersion calls parameter_values"
        );
    }
}
