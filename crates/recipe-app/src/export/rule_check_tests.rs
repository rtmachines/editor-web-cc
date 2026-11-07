//! Tests of the rule check and the hash compare of the export flow (ADR-0118, ADR-0119).

use recipe_domain::audit::{AuditAction, AuditEntry};
use recipe_domain::error_codes::ErrorCode;
use recipe_domain::export::{ExportHeader, HashedBody, Sha256};
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    LeafPath, Machine, PlatformTypes, PlcPlatform, Primitive, S7Address, TypeTrees, UnitPath,
};
use recipe_domain::recipe::{Ordinal, Parameter, StoredLimit, StoredParameter, StoredValue};
use serde_json::json;

use super::test_doubles::{RecordingWriter, port};
use super::tests::export_approved;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope};
use crate::memory::fixtures::{Fixture, actor, meta, parameter_on, reference, uid, vid};
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
