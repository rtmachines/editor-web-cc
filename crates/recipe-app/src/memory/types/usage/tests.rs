//! Tests of the in-memory `type_usage` that the contract suite cannot run.
//!
//! The ports cannot build a cycle of type rows, because a struct row needs the rows of its
//! members first. `MemoryStore::seed_type` writes without a check, so a test can seed one.

use std::collections::BTreeSet;

use recipe_domain::plant::{
    MemberName, PlcPlatform, PlcType, PlcTypeRecord, RecordShape, TypeMember, TypeName,
};
use recipe_domain::recipe::{ParameterTag, StoredParameter, VersionStatus};

use crate::error::StorageError;
use crate::memory::fixtures::{Fixture, mid, parameter_on, reference, tid, uid, vid};
use crate::ports::{Isolation, TypeCatalog, TypeUsageRow};

/// The platform of the machine of the fixture.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// A struct of the machine `M1` with one member `link` of the type `member`.
fn linked_struct(id: u128, name: &str, member: u128) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse(name).unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("link").unwrap(),
                type_id: tid(member),
            }],
        },
    })
    .unwrap()
}

#[tokio::test]
async fn an_offset_above_i64_max_is_a_storage_error_as_in_the_sql_adapter() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PLATFORM, 1, "M1");
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    let largest = u64::try_from(i64::MAX).unwrap();

    let at_limit = tx.type_usage(mid(1), tid(901), largest, 10).await;
    let beyond = tx
        .type_usage(mid(1), tid(901), largest.checked_add(1).unwrap(), 10)
        .await;

    assert_eq!(at_limit.unwrap(), []);
    assert!(matches!(
        beyond.unwrap_err(),
        StorageError::Other { sqlstate: None, .. }
    ));
}

#[tokio::test]
async fn a_cycle_of_two_structs_ends_and_lists_the_parameter_once() {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_type(&linked_struct(901, "ST_A", 902));
    fixture.store.seed_type(&linked_struct(902, "ST_B", 901));
    let parameter = StoredParameter {
        plc_type: tid(902),
        ..parameter_on(PLATFORM, 1, "CYCLE", uid(101), 0)
    };
    fixture.store.seed_parameter(vid(101), &parameter);
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();

    let rows = tx.type_usage(mid(1), tid(901), 0, 10).await.unwrap();

    let expected = TypeUsageRow {
        reference: reference("R-1"),
        number: 1,
        status: VersionStatus::Draft,
        tag: ParameterTag::parse("CYCLE").unwrap(),
        parameter_type: tid(902),
    };
    assert_eq!(rows, [expected]);
}
