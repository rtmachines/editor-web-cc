//! Tests of `preview.rs` (design D4): the preview tree equals the tree after `EnsureDerivedType`.

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    DerivedTypeReason, LeafClass, MemberName, PlantError, PlcPlatform, PlcType, PlcTypeRecord,
    Primitive, RecordShape, TypeExpr, TypeLimit, TypeMember, TypeName, TypeTree,
};

use super::preview::preview_tree;
use super::test_world::seed_users;
use crate::context::audit_context;
use crate::error::AppError;
use crate::memory::fixtures::{Fixture, actor, meta, mid, scenarios, tid};
use crate::memory::seed_type_id;
use crate::plant::DerivedTypes;
use crate::ports::{Isolation, Transaction};

/// The platform of the machine `M1` of the zone draft.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

/// The type expressions of the equality test (design D4, D16).
const EXPRESSIONS: [&str; 5] = [
    "ARRAY[1..8] OF INT",
    "ARRAY[0..1,1..2] OF REAL",
    "STRING(20)",
    "ST_Zone@2",
    "ARRAY[1..4] OF ST_Zone@2",
];

/// One leaf without its row identifier, because a preview row has a preview identifier.
type LeafShape = (String, Primitive, LeafClass, u32, Option<u16>);

fn shapes(tree: &TypeTree) -> Vec<LeafShape> {
    tree.leaves()
        .iter()
        .map(|leaf| {
            let target = leaf.target();
            (
                leaf.path().to_string(),
                target.primitive(),
                target.leaf_class(),
                target.size_bytes(),
                target.string_length(),
            )
        })
        .collect()
}

fn expr(text: &str) -> TypeExpr {
    TypeExpr::parse(text).unwrap()
}

/// The type rows of `M1` as a query reads them.
async fn rows_of(fixture: &Fixture) -> Vec<PlcType> {
    let mut tx = fixture.store.begin_memory(Isolation::ReadOnly).unwrap();
    tx.types().type_rows(mid(1)).await.unwrap()
}

/// The tree after `EnsureDerivedType` in a command transaction that never commits.
async fn ensured_tree(fixture: &Fixture, text: &str) -> TypeTree {
    ensured(fixture, PLATFORM, text).await.unwrap()
}

/// Run `EnsureDerivedType` in a command transaction that never commits, then resolve the tree.
///
/// `ensure_expr` resolves the tree only for an array. A command resolves the tree of every type
/// after it, so the last step here gives the error of a struct or a string above a limit.
async fn ensured(
    fixture: &Fixture,
    platform: PlcPlatform,
    text: &str,
) -> Result<TypeTree, AppError> {
    let mut tx = fixture
        .store
        .begin_memory(Isolation::ReadCommitted)
        .unwrap();
    let rows = tx.types().type_rows(mid(1)).await.unwrap();
    let context = audit_context(&actor(Role::ProcessEngineer), &meta());
    let mut derived = DerivedTypes::new(mid(1), platform, rows, context);
    let id = derived.ensure_expr(tx.types(), &expr(text)).await?;
    Ok(TypeTree::resolve(id, &derived.platform_types()?)?)
}

#[tokio::test]
async fn the_preview_leaves_equal_the_leaves_after_ensure_derived_type() {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);
    let rows = rows_of(&fixture).await;

    for text in EXPRESSIONS {
        let expected = ensured_tree(&fixture, text).await;
        let preview = preview_tree(mid(1), PLATFORM, rows.clone(), &expr(text)).unwrap();

        assert_eq!(shapes(&preview), shapes(&expected), "{text}");
        assert_eq!(preview.leaf_count(), expected.leaf_count(), "{text}");
        assert_eq!(preview.depth(), expected.depth(), "{text}");
    }
}

#[tokio::test]
async fn the_preview_adds_no_row_to_the_store() {
    let fixture = scenarios::zone_draft();
    let rows = rows_of(&fixture).await;

    preview_tree(
        mid(1),
        PLATFORM,
        rows.clone(),
        &expr("ARRAY[1..4] OF STRING(20)"),
    )
    .unwrap();

    assert_eq!(rows_of(&fixture).await.len(), rows.len());
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn an_unknown_struct_revision_gives_plt_013() {
    let fixture = scenarios::zone_draft();
    let rows = rows_of(&fixture).await;

    let error = preview_tree(mid(1), PLATFORM, rows, &expr("ST_Zone@9")).unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_013);
}

#[tokio::test]
async fn a_type_that_the_platform_does_not_hold_gives_plt_013() {
    let fixture = Fixture::new();
    fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    let rows = rows_of(&fixture).await;

    let wstring = preview_tree(
        mid(1),
        PlcPlatform::S7_300,
        rows.clone(),
        &expr("WSTRING(10)"),
    );
    let lreal = preview_tree(mid(1), PlcPlatform::S7_300, rows, &expr("LREAL"));

    assert_eq!(wstring.unwrap_err().code(), ErrorCode::PLT_013);
    assert_eq!(lreal.unwrap_err().code(), ErrorCode::PLT_013);
}

/// The code of an error with its reason (`E-PLT-013`) or its limit (`E-PLT-018`).
type Failure = (ErrorCode, Option<DerivedTypeReason>, Option<TypeLimit>);

fn failure(error: &AppError) -> Failure {
    match error {
        AppError::Plant(PlantError::DerivedTypeInvalid { reason, .. }) => {
            (error.code(), Some(*reason), None)
        }
        AppError::Plant(PlantError::TypeTooLarge { limit, .. }) => {
            (error.code(), None, Some(*limit))
        }
        other => (other.code(), None, None),
    }
}

/// Add the struct `ST_Big@1` to `fixture`. Its member `values` is `ARRAY[1..16385] OF INT`.
///
/// No command can store such a struct. The rows test a struct whose tree is above the leaf
/// limit, because `ensure_expr` resolves no tree for an expression without an array.
fn seed_big_struct(fixture: &Fixture) {
    let array_expr = expr("ARRAY[1..16385] OF INT");
    let array = PlcType::try_from(PlcTypeRecord {
        id: tid(950),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Array {
            element: seed_type_id(Primitive::Int, None).unwrap(),
            dimensions: array_expr.arrays().first().unwrap().clone(),
        },
    })
    .unwrap();
    let member = TypeMember {
        name: MemberName::parse("values").unwrap(),
        type_id: tid(950),
    };
    let big = PlcType::try_from(PlcTypeRecord {
        id: tid(951),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape: RecordShape::Struct {
            name: TypeName::parse("ST_Big").unwrap(),
            revision: 1,
            supersedes: None,
            members: vec![member],
        },
    })
    .unwrap();
    fixture.store.seed_type(&array);
    fixture.store.seed_type(&big);
}

/// The machine `M1` on `platform`. On `TwinCAT3` it holds `ST_Zone@1`, `ST_Zone@2`, and `ST_Big@1`.
fn bad_world(platform: PlcPlatform) -> Fixture {
    if platform == PLATFORM {
        let fixture = scenarios::zone_draft();
        seed_big_struct(&fixture);
        return fixture;
    }
    let fixture = Fixture::new();
    fixture.seed_machine_on(platform, 1, "M1");
    fixture
}

/// The bad expressions of the parity test with their platform and their expected code.
fn bad_cases() -> Vec<(PlcPlatform, String, ErrorCode)> {
    let deep = format!("{}ST_Zone@2", "ARRAY[1..1] OF ".repeat(32));
    let cases = [
        (PLATFORM, "ST_Zone@9", ErrorCode::PLT_013),
        (PLATFORM, "ARRAY[1..2] OF ST_Zone@9", ErrorCode::PLT_013),
        (PlcPlatform::S7_300, "WSTRING(10)", ErrorCode::PLT_013),
        (PlcPlatform::S7_300, "LREAL", ErrorCode::PLT_013),
        (PLATFORM, "STRING(300)", ErrorCode::PLT_013),
        (PlcPlatform::S7_300, "STRING(300)", ErrorCode::PLT_013),
        (PLATFORM, "ARRAY[1..16385] OF INT", ErrorCode::PLT_018),
        (
            PLATFORM,
            "ARRAY[1..200] OF ARRAY[1..100] OF INT",
            ErrorCode::PLT_018,
        ),
        (PLATFORM, deep.as_str(), ErrorCode::PLT_018),
        (PLATFORM, "ST_Big@1", ErrorCode::PLT_018),
        (PLATFORM, "ARRAY[1..2] OF ST_Big@1", ErrorCode::PLT_018),
    ];
    cases
        .into_iter()
        .map(|(platform, text, code)| (platform, text.to_owned(), code))
        .collect()
}

#[tokio::test]
async fn a_bad_expression_gives_the_same_code_and_reason_as_ensure_derived_type() {
    for (platform, text, code) in bad_cases() {
        let fixture = bad_world(platform);
        let rows = rows_of(&fixture).await;

        let preview = preview_tree(mid(1), platform, rows, &expr(&text)).unwrap_err();
        let ensured = ensured(&fixture, platform, &text).await.unwrap_err();

        assert_eq!(preview.code(), code, "{text} on {platform:?}: {preview:?}");
        assert_eq!(
            failure(&preview),
            failure(&ensured),
            "{text} on {platform:?}"
        );
    }
}

#[tokio::test]
async fn an_array_above_the_leaf_limit_gives_plt_018() {
    let fixture = scenarios::zone_draft();
    let rows = rows_of(&fixture).await;

    let error: AppError =
        preview_tree(mid(1), PLATFORM, rows, &expr("ARRAY[1..16385] OF INT")).unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_018);
}
