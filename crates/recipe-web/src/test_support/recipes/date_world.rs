//! The recipe `R-400` with a stored `DATE` that breaks the current rules (ADR-0119, ADR-0120).
//!
//! The version page tests and the export route tests need the same data. One module gives it, so
//! no test file repeats the seed code. This module is test code.

use std::collections::BTreeSet;

use recipe_app::ports::{Isolation, NonEmpty, UnitOfWork};
use recipe_domain::audit::{
    AuditContext, AuditEntry, AuditTarget, RequestId, RowAction, RowChange,
};
use recipe_domain::export::{ExportHeader, HashedBody};
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, S7Address, TypeTrees, UnitId,
};
use recipe_domain::recipe::{
    Ordinal, Parameter, ParameterId, ParameterKey, ParameterTag, Recipe, RecipeId, RecipeVersionId,
    ReferenceNumber, StoredParameter, StoredRecipe, StoredValue, StoredVersionHeader, VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use crate::test_support::Fixture;
use crate::test_support::plant::{World, world};

/// The version page of the draft of `R-400` on the `S7_1500` machine `M30`.
pub(crate) const DATE_PAGE: &str = "/recipes/R-400/versions/1";
/// The stored `DATE` count that passes the earlier rule set and breaks the current one.
pub(crate) const EARLIER_DATE_COUNT: i64 = 72_700;
/// The identifier of the type `DATE` of `S7_1500`. The seed has no row of it.
const DATE_TYPE_ID: Uuid = Uuid::from_u128(0x7357_0013_0000_0000_0000_0000_0000_0f02);
/// The identifier and the key of the parameter `DUE`.
const DUE_ID: Uuid = Uuid::from_u128(0x7357_0006_0000_0000_0000_0000_0f00_0040);
/// The identifier of the machine `M30`.
pub(crate) const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0000_001e);
/// The identifier of the version 1 of `R-400`.
pub(crate) const VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0000_0401);

/// Seed `R-400` on the `S7_1500` machine `M30` with its version 1 in the state `state`. The version
/// holds the parameter `DUE` of the type `DATE` and the stored count `count`.
///
/// `S7_1500` is the platform, because `S7_300` keeps the `DATE` end 72683 in both rule sets
/// (ADR-0120).
pub(crate) fn seed_date_world(world: &World, count: i64, state: VersionState) {
    let machine_id = MachineId::from(MACHINE_ID);
    let unit_id = UnitId::from(Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_001e_0001));
    let machine = Machine::create(
        machine_id,
        MachineCode::parse("M30").unwrap(),
        Text::parse("Dating line").unwrap(),
        PlcPlatform::S7_1500,
        (unit_id, Text::parse("Infeed").unwrap()),
    );
    world.fixture.store.seed_machine(&machine);
    let primitive = Primitive::Date;
    let row = PlcType::try_from(PlcTypeRecord {
        id: PlcTypeId::from(DATE_TYPE_ID),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap();
    world.fixture.store.seed_type(&row);
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0400)),
        machine: machine_id,
        reference: ReferenceNumber::parse("R-400").unwrap(),
        name: Text::parse("Dating recipe").unwrap(),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(VERSION_ID),
            number: 1,
            state,
            row_version: 1,
        }],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    world
        .fixture
        .store
        .seed_recipe(&recipe, PlcPlatform::S7_1500, creator);
    let parameter = StoredParameter {
        id: ParameterId::from(DUE_ID),
        key: ParameterKey::from(DUE_ID),
        unit: unit_id,
        tag: ParameterTag::parse("DUE").unwrap(),
        name_en: Text::parse("due").unwrap(),
        description_en: LongText::parse("A seeded date").unwrap(),
        translations: Vec::new(),
        plc_type: PlcTypeId::from(DATE_TYPE_ID),
        s7_address: Some(S7Address::parse("DB1.DBW0").unwrap()),
        limits: Vec::new(),
        value: StoredValue::from(json!(count)),
        ordinal: Ordinal::parse(0).unwrap(),
    };
    world
        .fixture
        .store
        .seed_parameter(RecipeVersionId::from(VERSION_ID), &parameter);
}

/// Build the plant world with the approved version 1 of `R-400` and the stored count
/// `EARLIER_DATE_COUNT`.
///
/// The version breaks the current rules, so every export route marks its document not loadable.
pub(crate) async fn affected_approved_world() -> World {
    let world = world();
    seed_date_world(&world, EARLIER_DATE_COUNT, VersionState::LiveDraft);
    approve_with_real_hash(&world.fixture).await;
    world
}

/// Approve the live draft of `seed_date_world` with the real content hash of its rows.
///
/// The function solves one problem: the approval use case refuses the count `EARLIER_DATE_COUNT`,
/// and the memory seed stores a zero hash. An export route checks the stored hash first, so a test
/// of an approved version with this count needs the real hash. The function builds the hashed
/// body as step 4 of the export flow does. Then it approves the version through the port.
pub(crate) async fn approve_with_real_hash(fixture: &Fixture) {
    let version = RecipeVersionId::from(VERSION_ID);
    let machine = fixture
        .store
        .state()
        .machine(MachineId::from(MACHINE_ID))
        .unwrap();
    let hash = {
        let mut tx = fixture.store.begin(Isolation::ReadOnly).await.unwrap();
        let parameters: Vec<Parameter> = tx
            .recipes()
            .version_parameters(version)
            .await
            .unwrap()
            .into_iter()
            .map(Parameter::from)
            .collect();
        let units = tx.units().live_units(machine.id()).await.unwrap().unwrap();
        let rows = tx.types().type_rows(machine.id()).await.unwrap();
        let types = PlatformTypes::collect(machine.id(), machine.platform(), rows).unwrap();
        let trees = TypeTrees::resolve(parameters.iter().map(Parameter::plc_type), &types);
        let header = ExportHeader {
            machine: machine.code(),
            platform: machine.platform(),
            reference: ReferenceNumber::parse("R-400").unwrap(),
            version: 1,
        };
        HashedBody::build(&header, &parameters, &units, &trees)
            .unwrap()
            .sha256()
            .unwrap()
    };
    let approver = UserId::from(Uuid::from_u128(1));
    let mut tx = fixture.store.begin(Isolation::ReadCommitted).await.unwrap();
    tx.recipes()
        .approve_version(version, approver, hash)
        .await
        .unwrap();
    let context = AuditContext {
        actor: Some(approver),
        request_id: RequestId::from(Uuid::nil()),
        client_ip: None,
        session_ref: None,
    };
    let entry = AuditEntry::row(
        RowAction::VersionApproved,
        AuditTarget::recipe_version(
            RecipeId::from(Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0000_0400)),
            version,
        ),
        RowChange::Create { after: json!({}) },
        context,
    );
    let store: &dyn UnitOfWork = &fixture.store;
    store.commit(tx, NonEmpty::new(entry)).await.unwrap();
}
