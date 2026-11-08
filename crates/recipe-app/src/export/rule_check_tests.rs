//! Tests of the rule check and the hash compare of the export flow (ADR-0118, ADR-0119).

use std::collections::BTreeSet;

use recipe_domain::audit::{AuditAction, AuditEntry};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{ExportHeader, HashedBody, Sha256};
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, Machine, MemberName, PlatformTypes, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord,
    Primitive, RecordShape, S7Address, TypeMember, TypeName, TypeTrees, UnitId, UnitPath,
};
use recipe_domain::recipe::{Ordinal, Parameter, StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::test_doubles::{RecordingWriter, port};
use super::tests::export_approved;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope};
use crate::memory::fixtures::{Fixture, actor, meta, mid, parameter_on, reference, tid, uid, vid};
use crate::memory::seed_type_id;
use crate::ports::{Isolation, UnitOfWork};

/// The identifier of the `DATE` row of `S7_1500` in the tests of the rule fault.
const DATE_TYPE: u128 = 960;
/// The identifier of the `S5TIME` row of `S7_1500` in the tests of the rule fault.
const S5TIME_TYPE: u128 = 961;
/// A `DATE` count that only the earlier rule set permits: `D#2169-01-17` (ADR-0118).
const EARLIER_DATE: i64 = 72_700;

/// Return the parameter `DUE` of type `DATE` with the stored count `EARLIER_DATE`.
///
/// The seed has no `DATE` row, so the function stores one for `S7_1500`.
fn seed_earlier_date_parameter(fixture: &Fixture) -> StoredParameter {
    let plc_type = fixture.seed_primitive_type(DATE_TYPE, PlcPlatform::S7_1500, Primitive::Date);
    StoredParameter {
        plc_type,
        value: StoredValue::from(json!(EARLIER_DATE)),
        s7_address: Some(S7Address::parse("DB1.DBW0").unwrap()),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_1500, 1, "DUE", uid(102), 0)
    }
}

/// Return the parameter `HOLD` of type `S5TIME` with the stored count 0 and the limit `max` 15.
///
/// The count 15 lies between two `S5TIME` steps, so the stored limit breaks a count rule
/// (ADR-0120 (b)). The seed has no `S5TIME` row, so the function stores one for `S7_1500`.
fn seed_s5time_parameter(fixture: &Fixture) -> StoredParameter {
    let plc_type =
        fixture.seed_primitive_type(S5TIME_TYPE, PlcPlatform::S7_1500, Primitive::S5Time);
    let limit = StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(15.into()),
        max_length: None,
    };
    StoredParameter {
        plc_type,
        value: StoredValue::from(json!(0)),
        s7_address: Some(S7Address::parse("DB1.DBW0").unwrap()),
        limits: vec![limit],
        ..parameter_on(PlcPlatform::S7_1500, 1, "HOLD", uid(102), 0)
    }
}

/// Return the hash of the hashed body that version 1 of `R-1` has in the store.
///
/// The function reads the rows as step 4 of the export flow does. The approval refuses the
/// count `EARLIER_DATE`, so a test cannot take the hash from the entry `VersionApproved`.
async fn body_hash_of_version_one(fixture: &Fixture, machine: &Machine) -> Sha256 {
    let mut tx = fixture.store.begin(Isolation::ReadOnly).await.unwrap();
    let parameters: Vec<Parameter> = tx
        .recipes()
        .version_parameters(vid(101))
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
        reference: reference("R-1"),
        version: 1,
    };
    let hash = HashedBody::build(&header, &parameters, &units, &trees)
        .unwrap()
        .sha256()
        .unwrap();
    // A read transaction ends by drop, as in the use cases.
    drop(tx);
    hash
}

/// Store `hash` as the content hash of version 1.
fn store_hash(fixture: &Fixture, hash: Sha256) {
    fixture.store.update_state(|state, _| {
        if let Some(row) = state.versions.get_mut(&vid(101)) {
            row.content_sha256 = Some(hash);
        }
    });
}

/// The offset of a `DINT` at the end of a data block of `S7_300`. The last byte is 65 537.
const PAST_BLOCK_END: &str = "DB1.DBD65534";
/// A `DATE` count above the end of the earlier rule set. It is a storage defect (ADR-0119 (k)).
const BEYOND_EARLIER_DATE: i64 = 72_900;

/// Return the parameter `SPEED` of type `DINT` at `DB1.DBD65534` for a machine on `S7_300`.
fn seed_past_block_end_parameter() -> StoredParameter {
    StoredParameter {
        plc_type: seed_type_id(Primitive::Dint, None).unwrap(),
        s7_address: Some(S7Address::parse(PAST_BLOCK_END).unwrap()),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_300, 1, "SPEED", uid(102), 0)
    }
}

#[tokio::test]
async fn a_dint_past_the_end_of_the_data_block_on_s7_300_is_not_loadable() {
    // ADR-0119: the address breaks the current rules, so the version is not loadable.
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(101), &seed_past_block_end_parameter());
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let exported = export_approved(&fixture, Role::Operator).await.unwrap();
    assert_eq!(exported.content_sha256, stored);
    assert!(!exported.loadable());
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

/// The identifier of the struct `ST_Bytes@1` in the tests of the overlap.
const BYTES_STRUCT: u128 = 970;
/// The first identifier of the global rows `BYTE`, `CHAR`, `SINT`, and `USINT`.
const BYTE_FAMILY_FIRST: u128 = 971;
/// The primitive types of the members `a` to `d` of `ST_Bytes@1`, in order.
const BYTES_MEMBERS: [(&str, Primitive); 4] = [
    ("a", Primitive::Byte),
    ("b", Primitive::Char),
    ("c", Primitive::Sint),
    ("d", Primitive::Usint),
];

/// Return the platforms of the global row of `primitive` (`0004_seed_plc_types.sql`).
///
/// `SINT` and `USINT` do not exist on `S7_300`. `CHAR` does not exist on the IEC platforms.
fn byte_family_platforms(primitive: Primitive) -> BTreeSet<PlcPlatform> {
    let all = [
        PlcPlatform::CodesysIec,
        PlcPlatform::TwinCat3,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
        PlcPlatform::S7_300,
    ];
    all.into_iter()
        .filter(|platform| match primitive {
            Primitive::Sint | Primitive::Usint => *platform != PlcPlatform::S7_300,
            Primitive::Char => matches!(
                platform,
                PlcPlatform::S7_1200 | PlcPlatform::S7_1500 | PlcPlatform::S7_300
            ),
            _ => true,
        })
        .collect()
}

/// Store the global row `id` of `primitive` with the platforms of the seed migration.
fn seed_byte_family_row(fixture: &Fixture, id: u128, primitive: Primitive) -> PlcTypeId {
    let row = PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: None,
        platforms: byte_family_platforms(primitive),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap();
    fixture.store.seed_type(&row);
    tid(id)
}

/// Seed `ST_Bytes@1` and return the identifiers of the struct and of the `BYTE` row.
///
/// The members are `a` of type `BYTE`, `b` of type `CHAR`, `c` of type `SINT`, and `d` of type
/// `USINT`. The seed of the memory store has none of the four primitive rows.
fn seed_bytes_struct(fixture: &Fixture) -> (PlcTypeId, PlcTypeId) {
    let mut members = Vec::new();
    let mut byte = None;
    for (offset, (name, primitive)) in (0_u128..).zip(BYTES_MEMBERS) {
        let id = BYTE_FAMILY_FIRST.checked_add(offset).unwrap();
        let type_id = seed_byte_family_row(fixture, id, primitive);
        byte = byte.or((primitive == Primitive::Byte).then_some(type_id));
        members.push(TypeMember {
            name: MemberName::parse(name).unwrap(),
            type_id,
        });
    }
    let row = PlcType::try_from(PlcTypeRecord {
        id: tid(BYTES_STRUCT),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Bytes").unwrap(),
            revision: 1,
            supersedes: None,
            members,
        },
    })
    .unwrap();
    fixture.store.seed_type(&row);
    (tid(BYTES_STRUCT), byte.unwrap())
}

/// Seed the approved version 1 of `R-1` on `S7_1500` with the overlap of the current layout rules.
///
/// `BYTES` of type `ST_Bytes@1` stands at `DB1.DBB0` in `U1.2`. The earlier layout gave the struct
/// the size 4. The current layout gives the size 6 (ADR-0147). `MODE` of type `BYTE` stands at
/// `DB1.DBB4` in the unit `mode_unit`. The struct and `MODE` then share the byte 4 (ADR-0149).
fn seed_overlap_version(fixture: &Fixture, mode_unit: UnitId) -> Machine {
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let (bytes, byte) = seed_bytes_struct(fixture);
    let address = |text: &str| Some(S7Address::parse(text).unwrap());
    let bytes_parameter = StoredParameter {
        plc_type: bytes,
        value: StoredValue::from(json!({"a": 0, "b": "A", "c": 0, "d": 0})),
        s7_address: address("DB1.DBB0"),
        limits: Vec::new(),
        ..parameter_on(PlcPlatform::S7_1500, 1, "BYTES", uid(102), 0)
    };
    let mode_parameter = StoredParameter {
        plc_type: byte,
        value: StoredValue::from(json!(0)),
        s7_address: address("DB1.DBB4"),
        limits: Vec::new(),
        ordinal: Ordinal::parse(1).unwrap(),
        ..parameter_on(PlcPlatform::S7_1500, 2, "MODE", mode_unit, 0)
    };
    fixture.store.seed_parameter(vid(101), &bytes_parameter);
    fixture.store.seed_parameter(vid(101), &mode_parameter);
    machine
}

#[tokio::test]
async fn an_approved_version_with_an_overlap_of_the_current_layout_is_not_loadable() {
    // ADR-0149 (e) to (g): the struct of size 6 at DB1.DBB0 covers MODE at DB1.DBB4. The earlier
    // rules gave the size 4, so the version was clean when it was approved. It exports with the
    // stored hash. The audit entry stays. The document marks it as not loadable (D10).
    let fixture = Fixture::new();
    let machine = seed_overlap_version(&fixture, uid(102));
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);

    let exported = export_approved(&fixture, Role::Operator).await.unwrap();

    assert_eq!(exported.content_sha256, stored);
    assert!(!exported.loadable());
    assert_eq!(fixture.store.audit_entries().len(), 1);
}

#[tokio::test]
async fn a_unit_document_whose_scope_holds_only_the_struct_of_an_overlap_is_not_loadable() {
    // ADR-0149 (i): MODE sits in U1, outside the scope U1.2. The scope holds only BYTES. The rule
    // check still covers the whole version, so the unit document is not loadable.
    let fixture = Fixture::new();
    let machine = seed_overlap_version(&fixture, uid(101));
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let writer = RecordingWriter::shared();
    let input = ExportApprovedInput {
        reference: reference("R-1"),
        machine: None,
        scope: ExportScope::Unit(UnitPath::parse("U1.2").unwrap()),
    };

    let exported = ExportApprovedVersion::new(fixture.context(), port(&writer))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap();

    assert!(!exported.loadable());
    let calls = writer.scoped_calls();
    let (body, facts) = calls.last().unwrap();
    assert_eq!(body.parameters().len(), 1);
    assert!(!facts.document.loadable());
}

#[tokio::test]
async fn a_changed_hash_gives_exp_002_before_the_rule_check() {
    // The version has a storage defect (E-DB-008) and a wrong hash. The hash compare runs first,
    // so the answer is E-EXP-002. If the rule check ran first, the answer would be E-DB-008.
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let broken = StoredParameter {
        value: StoredValue::from(json!(BEYOND_EARLIER_DATE)),
        ..seed_earlier_date_parameter(&fixture)
    };
    fixture.store.seed_parameter(vid(101), &broken);
    let correct = body_hash_of_version_one(&fixture, &machine).await;
    let wrong = Sha256::from([7_u8; 32]);
    assert_ne!(wrong, correct);
    store_hash(&fixture, wrong);
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_stored_date_beyond_the_earlier_rule_set_gives_db_008_and_no_entry() {
    // ADR-0119 (k): the count 72900 is a storage defect, not a rule fault.
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let broken = StoredParameter {
        value: StoredValue::from(json!(BEYOND_EARLIER_DATE)),
        ..seed_earlier_date_parameter(&fixture)
    };
    fixture.store.seed_parameter(vid(101), &broken);
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_stored_s5time_limit_between_two_steps_gives_db_008_and_no_entry() {
    // ADR-0120 (b): the stored bound 15 breaks the step rule, so it is a storage defect.
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(101), &seed_s5time_parameter(&fixture));
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let error = export_approved(&fixture, Role::Operator).await.unwrap_err();
    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_approved_version_with_a_date_of_the_earlier_rule_set_is_not_loadable() {
    // ADR-0119: the count 72700 breaks the current rules, so the version is not loadable.
    // The hash and the audit entry stay as for any other export (D8).
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    fixture
        .store
        .seed_parameter(vid(101), &seed_earlier_date_parameter(&fixture));
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let exported = export_approved(&fixture, Role::Operator).await.unwrap();
    let entries = fixture.store.audit_entries();
    let exports: Vec<&AuditEntry> = entries
        .iter()
        .filter(|entry| entry.action() == AuditAction::VersionExported)
        .collect();
    assert_eq!(exported.content_sha256, stored);
    assert_eq!(exports.len(), 1);
    assert_eq!(entries.len(), 1);
    assert!(!exported.loadable());
}

#[tokio::test]
async fn a_unit_export_is_not_loadable_when_a_parameter_outside_the_unit_breaks_a_rule() {
    // ADR-0119 (f): the rule check covers the whole version, also for a unit export.
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    // The faulty parameter sits in U1. The clean parameter sits in U1.2, the scope of the export.
    let faulty = StoredParameter {
        unit: uid(101),
        ..seed_earlier_date_parameter(&fixture)
    };
    let clean = StoredParameter {
        s7_address: Some(S7Address::parse("DB1.DBW2").unwrap()),
        limits: Vec::new(),
        ordinal: Ordinal::parse(1).unwrap(),
        ..parameter_on(PlcPlatform::S7_1500, 2, "CLEAN", uid(102), 0)
    };
    fixture.store.seed_parameter(vid(101), &faulty);
    fixture.store.seed_parameter(vid(101), &clean);
    let stored = body_hash_of_version_one(&fixture, &machine).await;
    store_hash(&fixture, stored);
    let writer = RecordingWriter::shared();
    let input = ExportApprovedInput {
        reference: reference("R-1"),
        machine: None,
        scope: ExportScope::Unit(UnitPath::parse("U1.2").unwrap()),
    };

    let exported = ExportApprovedVersion::new(fixture.context(), port(&writer))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap();

    assert!(!exported.loadable());
    let calls = writer.scoped_calls();
    let (body, facts) = calls.last().unwrap();
    assert_eq!(body.parameters().len(), 1);
    assert!(!facts.document.loadable());
}
