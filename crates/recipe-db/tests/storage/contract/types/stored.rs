//! Contract cases of a stored struct with a name that the current rule refuses (D3).
//!
//! The earlier rule set keeps a stored struct such as `Int@1` readable. The read of the type
//! rows and the read of the audit label use `RuleSet::Earlier`.

use recipe_app::ports::{AggregateId, EntityLabel, Isolation};
use recipe_domain::audit::AggregateType;
use recipe_domain::plant::{PlcType, PlcTypeRecord, Primitive, RecordShape, RuleSet, TypeName};

use crate::contract::harness::{Harness, begin, commit};
use crate::contract::seed::{insert, machine, seed_type, struct_record, type_id};

/// The name that the current rule refuses and that a stored row can hold.
const STORED_NAME: &str = "Int";

fn stored_name() -> TypeName {
    TypeName::from_stored(STORED_NAME, RuleSet::Earlier).unwrap()
}

/// Build the struct `Int@1` with the earlier rule set.
///
/// `struct_record` parses the name with the current rule, so the case replaces the name.
fn stored_int(record: PlcTypeRecord) -> PlcType {
    let (revision, supersedes, members) = match record.shape {
        RecordShape::Struct {
            revision,
            supersedes,
            members,
            ..
        } => Some((revision, supersedes, members)),
        _ => None,
    }
    .expect("the record is a struct");
    let shape = RecordShape::Struct {
        name: stored_name(),
        revision,
        supersedes,
        members,
    };
    let record = PlcTypeRecord { shape, ..record };
    PlcType::from_stored(record, RuleSet::Earlier).unwrap()
}

/// Insert the struct `Int@1` of a new machine as `recipe_app`.
async fn seed_stored_int(h: &dyn Harness, number: u128, code: &str) -> PlcType {
    let built = machine(number, code);
    insert(h, &built).await;
    let dint = seed_type(h, built.id(), Primitive::Dint).await;
    let record = struct_record(&built, type_id(number), "Pump", 1, None, &[("speed", dint)]);
    let row = stored_int(record);
    let mut tx = begin(h).await;
    tx.types().insert_struct(&row).await.unwrap();
    commit(h, tx, built.id()).await;
    row
}

async fn a_stored_struct_with_a_reserved_name_is_read_with_its_members(h: &dyn Harness) {
    let row = seed_stored_int(h, 160, "M160").await;
    let machine_id = row.machine().unwrap();
    let mut tx = begin(h).await;

    let rows = tx.types().type_rows(machine_id).await.unwrap();

    let stored = rows
        .iter()
        .find(|candidate| candidate.id() == row.id())
        .expect("the stored struct is in the rows of the machine");
    assert_eq!(stored, &row);
    assert!(
        matches!(stored.shape(), RecordShape::Struct { name, members, .. }
            if name.as_str() == STORED_NAME && members.len() == 1),
        "the struct keeps its name and its member"
    );
}

contract_case!(
    a_stored_struct_with_a_reserved_name_is_read_with_its_members,
    memory_a_stored_struct_with_a_reserved_name_is_read_with_its_members,
    postgres_a_stored_struct_with_a_reserved_name_is_read_with_its_members
);

async fn the_audit_label_of_a_stored_struct_with_a_reserved_name_reads(h: &dyn Harness) {
    let row = seed_stored_int(h, 161, "M161").await;
    let mut tx = h.unit_of_work().begin(Isolation::ReadOnly).await.unwrap();

    let label = tx
        .audit_reads()
        .aggregate_label(
            AggregateType::PlcType,
            AggregateId::from(*row.id().as_uuid()),
        )
        .await
        .unwrap();

    assert_eq!(
        label,
        EntityLabel::Type {
            name: stored_name(),
            revision: 1,
        }
    );
}

contract_case!(
    the_audit_label_of_a_stored_struct_with_a_reserved_name_reads,
    memory_the_audit_label_of_a_stored_struct_with_a_reserved_name_reads,
    postgres_the_audit_label_of_a_stored_struct_with_a_reserved_name_reads
);
