//! Tests of `type_text`: the text of a type row parses and resolves to the same row again.

use std::collections::BTreeSet;

use recipe_domain::identity::Role;
use recipe_domain::plant::{
    MemberName, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, RuleSet,
    TypeExpr, TypeMember, TypeName,
};

use super::type_text::{expr_names_row, type_text};
use crate::context::audit_context;
use crate::memory::fixtures::{Fixture, actor, meta, mid, tid};
use crate::memory::{MemoryTransaction, seed_type_id};
use crate::plant::DerivedTypes;
use crate::ports::{Isolation, Transaction};

/// The platform of the machine of the tests.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// The expressions of the tests. Each one is a canonical text, so `Display` gives it back.
const EXPRESSIONS: [&str; 14] = [
    "INT",
    "REAL",
    "STRING",
    "WSTRING",
    "STRING(16)",
    "STRING(80)",
    "WSTRING(20)",
    "ST_Zone@1",
    "ST_Zone@2",
    "ARRAY[1..4] OF STRING(16)",
    "ARRAY[0..9] OF STRING",
    "ARRAY[1..2,1..3] OF INT",
    "ARRAY[1..2] OF ARRAY[1..3] OF DINT",
    "ARRAY[1..4] OF ST_Zone@2",
];

fn zone(id: u128, revision: u32) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Zone").unwrap(),
            revision,
            supersedes: None,
            members: vec![TypeMember {
                name: MemberName::parse("setpoint").unwrap(),
                type_id: seed_type_id(Primitive::Real, None).unwrap(),
            }],
        },
    })
    .unwrap()
}

/// A stored struct `Int@1` of the machine 1. The row passes `RuleSet::Earlier`, so its name is
/// the name of a primitive.
fn int_struct(id: u128) -> PlcType {
    PlcType::from_stored(
        PlcTypeRecord {
            id: tid(id),
            machine: Some(mid(1)),
            platforms: BTreeSet::from([PLATFORM]),
            shape: RecordShape::Struct {
                name: TypeName::from_stored("Int", RuleSet::Earlier).unwrap(),
                revision: 1,
                supersedes: None,
                members: vec![TypeMember {
                    name: MemberName::parse("setpoint").unwrap(),
                    type_id: seed_type_id(Primitive::Real, None).unwrap(),
                }],
            },
        },
        RuleSet::Earlier,
    )
    .unwrap()
}

/// One open transaction over the machine 1 with the struct `ST_Zone` in two revisions.
async fn open() -> (Fixture, MemoryTransaction, DerivedTypes) {
    open_with(&[]).await
}

/// Like [`open`], with the rows `extra` stored before the transaction reads the type rows.
async fn open_with(extra: &[PlcType]) -> (Fixture, MemoryTransaction, DerivedTypes) {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.store.seed_type(&zone(901, 1));
    fixture.store.seed_type(&zone(902, 2));
    for row in extra {
        fixture.store.seed_type(row);
    }
    let mut tx = fixture
        .store
        .begin_memory(Isolation::ReadCommitted)
        .unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    let context = audit_context(&actor(Role::ProcessEngineer), &meta());
    let types = DerivedTypes::new(mid(1), PLATFORM, rows, context);
    (fixture, tx, types)
}

fn expr(text: &str) -> TypeExpr {
    TypeExpr::parse(text).unwrap()
}

#[tokio::test]
async fn the_text_of_a_row_is_its_expression() {
    let (_fixture, mut tx, mut types) = open().await;
    let mut ids: Vec<PlcTypeId> = Vec::new();
    for text in EXPRESSIONS {
        ids.push(types.ensure_expr(tx.types(), &expr(text)).await.unwrap());
    }
    let platform_types = types.platform_types().unwrap();

    for (text, id) in EXPRESSIONS.into_iter().zip(ids) {
        assert_eq!(type_text(&platform_types, id), Some(expr(text).to_string()));
    }
}

#[tokio::test]
async fn a_written_text_parses_and_resolves_to_the_same_row_without_a_new_row() {
    let (_fixture, mut tx, mut types) = open().await;
    let mut ids: Vec<PlcTypeId> = Vec::new();
    for text in EXPRESSIONS {
        ids.push(types.ensure_expr(tx.types(), &expr(text)).await.unwrap());
    }
    let created = types.entries().len();
    let platform_types = types.platform_types().unwrap();

    for id in ids {
        let written = type_text(&platform_types, id).unwrap();
        let again = types
            .ensure_expr(tx.types(), &TypeExpr::parse(&written).unwrap())
            .await
            .unwrap();
        assert_eq!(again, id, "{written}");
    }

    assert_eq!(types.entries().len(), created);
}

#[tokio::test]
async fn the_global_string_and_the_machine_string_of_one_length_have_different_texts() {
    let (_fixture, mut tx, mut types) = open().await;
    let global = types
        .ensure_expr(tx.types(), &expr("STRING"))
        .await
        .unwrap();
    let own = types
        .ensure_expr(tx.types(), &expr("STRING(80)"))
        .await
        .unwrap();
    let platform_types = types.platform_types().unwrap();

    assert_ne!(global, own);
    assert_eq!(
        type_text(&platform_types, global).as_deref(),
        Some("STRING")
    );
    assert_eq!(
        type_text(&platform_types, own).as_deref(),
        Some("STRING(80)")
    );
}

#[tokio::test]
async fn an_unknown_row_has_no_text() {
    let (_fixture, _tx, types) = open().await;
    let platform_types = types.platform_types().unwrap();

    assert_eq!(type_text(&platform_types, tid(9_999)), None);
}

#[tokio::test]
async fn expr_names_row_is_true_exactly_when_ensure_expr_resolves_to_the_row() {
    let (_fixture, mut tx, mut types) = open().await;
    for text in EXPRESSIONS {
        types.ensure_expr(tx.types(), &expr(text)).await.unwrap();
    }
    let platform_types = types.platform_types().unwrap();

    for text in EXPRESSIONS {
        let parsed = expr(text);
        let resolved = types.ensure_expr(tx.types(), &parsed).await.unwrap();
        for row in platform_types.rows() {
            assert_eq!(
                expr_names_row(&platform_types, row.id(), &parsed),
                row.id() == resolved,
                "{text} against {:?}",
                row.id()
            );
        }
    }
}

#[tokio::test]
async fn the_global_string_names_the_seed_row_and_the_sized_string_names_the_row_of_the_machine() {
    let (_fixture, mut tx, mut types) = open().await;
    let global = types
        .ensure_expr(tx.types(), &expr("STRING"))
        .await
        .unwrap();
    let own = types
        .ensure_expr(tx.types(), &expr("STRING(80)"))
        .await
        .unwrap();
    let platform_types = types.platform_types().unwrap();

    assert!(expr_names_row(&platform_types, global, &expr("STRING")));
    assert!(!expr_names_row(&platform_types, own, &expr("STRING")));
    assert!(expr_names_row(&platform_types, own, &expr("STRING(80)")));
    assert!(!expr_names_row(
        &platform_types,
        global,
        &expr("STRING(80)")
    ));
}

#[tokio::test]
async fn a_stored_struct_with_a_primitive_name_names_its_row_only_with_the_exact_name() {
    let (_fixture, _tx, types) = open_with(&[int_struct(903)]).await;
    let platform_types = types.platform_types().unwrap();
    let stored = TypeExpr::from_stored("Int@1", RuleSet::Earlier).unwrap();
    let lower = TypeExpr::from_stored("int@1", RuleSet::Earlier).unwrap();

    assert!(expr_names_row(&platform_types, tid(903), &stored));
    assert!(!expr_names_row(&platform_types, tid(903), &lower));
    assert!(
        platform_types
            .rows()
            .all(|row| !expr_names_row(&platform_types, row.id(), &lower))
    );
}

#[tokio::test]
async fn an_identifier_that_the_types_do_not_hold_names_no_row() {
    let (_fixture, _tx, types) = open().await;
    let platform_types = types.platform_types().unwrap();

    for text in EXPRESSIONS {
        assert!(!expr_names_row(&platform_types, tid(9_999), &expr(text)));
    }
}
