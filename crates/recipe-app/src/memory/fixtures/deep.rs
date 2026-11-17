//! The chain of nested structs that the tests of the S7 nesting limit use (ADR-0179).

use std::collections::BTreeSet;

use recipe_domain::plant::{
    LeafPath, MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape,
    RuleSet, TypeMember, TypeName,
};
use recipe_domain::recipe::{StoredLimit, StoredParameter, StoredValue};
use serde_json::{Value, json};

use super::{Fixture, mid, parameter_on, tid, uid};
use crate::memory::seed_type_id;

/// The number of the row of the struct with one level. The struct with level `n` has the row
/// `DEEP_FIRST + n - 1`.
const DEEP_FIRST: u128 = 980;

/// Store a chain of `levels` structs on `platform` and return the identifier of the top struct.
///
/// The struct of level 1 has the `INT` member `leaf`. The struct of level `n` has the member
/// `inner` of the struct of level `n - 1`. The rows are `ST_L1@1` to `ST_L<levels - 1>@1`, and
/// the top struct has the name `top`. Each row passes `RuleSet::Earlier`, as a row that an
/// earlier release stored, so no check of the nesting limit runs on it.
pub(crate) fn seed_struct_chain(
    fixture: &Fixture,
    platform: PlcPlatform,
    (top, levels): (&str, u32),
) -> PlcTypeId {
    let mut member_type = seed_type_id(Primitive::Int, None).unwrap();
    let mut member = "leaf";
    for level in 1..=levels {
        let name = if level == levels {
            top.to_owned()
        } else {
            format!("ST_L{level}")
        };
        let id = DEEP_FIRST.checked_add(u128::from(level)).unwrap();
        let row = PlcType::from_stored(
            PlcTypeRecord {
                id: tid(id),
                machine: Some(mid(1)),
                platforms: BTreeSet::from([platform]),
                shape: RecordShape::Struct {
                    name: TypeName::from_stored(&name, RuleSet::Earlier).unwrap(),
                    revision: 1,
                    supersedes: None,
                    members: vec![TypeMember {
                        name: MemberName::parse(member).unwrap(),
                        type_id: member_type,
                    }],
                },
            },
            RuleSet::Earlier,
        )
        .unwrap();
        fixture.store.seed_type(&row);
        member_type = tid(id);
        member = "inner";
    }
    member_type
}

/// The value of a struct of the chain with `levels` levels: `leaf` is 0 at the bottom.
pub(crate) fn struct_chain_value(levels: u32) -> Value {
    let mut value = json!({"leaf": 0});
    for _ in 1..levels {
        value = json!({"inner": value});
    }
    value
}

/// The parameter `TOP` of the type `top` with nine levels on `S7_1500`, with no S7 address.
///
/// The parameter is a stored parameter from before ADR-0179. It has the key number 5, sits on
/// the unit 101, and has the limit 0 to 500 on its one leaf.
pub(crate) fn over_limit_top_parameter(top: PlcTypeId) -> StoredParameter {
    let path = format!("{}.leaf", ".inner".repeat(8));
    let limit = StoredLimit {
        leaf_path: LeafPath::parse(&path).unwrap(),
        min: Some("0".parse().unwrap()),
        max: Some("500".parse().unwrap()),
        max_length: None,
    };
    StoredParameter {
        plc_type: top,
        value: StoredValue::from(struct_chain_value(9)),
        limits: vec![limit],
        ..parameter_on(PlcPlatform::S7_1500, 5, "TOP", uid(101), 40)
    }
}
