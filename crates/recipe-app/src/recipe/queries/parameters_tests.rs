//! Tests of `ListParameters` (design D4, ADR-0042, ADR-0043 (e) and (j)).

use std::collections::{BTreeMap, BTreeSet};

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::{
    ArrayDimensions, MachineCode, PlcPlatform, PlcType, PlcTypeRecord, Primitive, RecordShape,
    StringKind, StringLength, TypeExpr, UnitPath,
};
use recipe_domain::recipe::{Ordinal, ParameterKey, StoredParameter, StoredValue};
use serde_json::json;

use super::parameters::{ListParameters, ListParametersInput, ParameterList};
use super::test_world::seed_users;
use crate::auth::Operation;
use crate::blocking::{AppSettings, PageSizes};
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::JournalEntry;
use crate::memory::fixtures::{
    Fixture, actor, meta, mid, parameter_on, pkey, reference, scenarios, tid, uid, vid,
};
use crate::memory::seed_type_id;
use crate::ports::Isolation;

/// The platform of the test machine.
const PLATFORM: PlcPlatform = PlcPlatform::TwinCat3;

fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
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

/// The draft `R-1` (version 101) of `M1` with four parameters on three units.
///
/// `U1` holds `SPEED` (key 1, ordinal 0) and `FLOW` (key 2, ordinal 1). `U1.2` holds `TEMP`
/// (key 3). `U1.2.1` holds `HOLD` (key 4).
fn world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    let placed = [
        (1, "SPEED", 101, 0),
        (2, "FLOW", 101, 1),
        (3, "TEMP", 102, 0),
        (4, "HOLD", 103, 0),
    ];
    for (n, tag, unit, ordinal) in placed {
        let parameter = StoredParameter {
            ordinal: Ordinal::parse(ordinal).unwrap(),
            ..parameter_on(PLATFORM, n, tag, uid(unit), 0)
        };
        fixture.store.seed_parameter(vid(101), &parameter);
    }
    fixture
}

/// The recipe `R-1` with the approved version 1 (version 101) that holds the four parameters of
/// `world`, and the live draft 2 (version 102) without a parameter.
fn approved_world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PLATFORM, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let placed = [
        (1, "SPEED", 101, 0),
        (2, "FLOW", 101, 1),
        (3, "TEMP", 102, 0),
        (4, "HOLD", 103, 0),
    ];
    for (n, tag, unit, ordinal) in placed {
        let parameter = StoredParameter {
            ordinal: Ordinal::parse(ordinal).unwrap(),
            ..parameter_on(PLATFORM, n, tag, uid(unit), 0)
        };
        fixture.store.seed_parameter(vid(101), &parameter);
    }
    fixture
}

/// A machine row of `M1` on `PLATFORM` with `shape`.
fn machine_row(id: u128, shape: RecordShape) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id: tid(id),
        machine: Some(mid(1)),
        platforms: BTreeSet::from([PLATFORM]),
        shape,
    })
    .unwrap()
}

/// The rows `ARRAY[1..4] OF INT` (type 950) and `STRING(20)` (type 951) of `M1`.
fn derived_rows() -> [PlcType; 2] {
    let array = expr_dimensions("ARRAY[1..4] OF INT");
    let length = StringLength::parse(StringKind::String, PLATFORM, 20).unwrap();
    let string = RecordShape::Primitive {
        primitive: Primitive::String,
        leaf_class: Primitive::String.leaf_class(),
        size_bytes: length.size_bytes(StringKind::String).unwrap(),
        string_length: Some(length.value()),
    };
    let int = seed_type_id(Primitive::Int, None).unwrap();
    [
        machine_row(
            950,
            RecordShape::Array {
                element: int,
                dimensions: array,
            },
        ),
        machine_row(951, string),
    ]
}

fn expr_dimensions(text: &str) -> ArrayDimensions {
    let expr = TypeExpr::parse(text).unwrap();
    expr.arrays().first().unwrap().clone()
}

/// The draft `R-1` with one parameter of each kind of type on `U1`.
///
/// `SPEED` (key 1) is `INT`, `VALUES` (key 2) is `ARRAY[1..4] OF INT`, `LABEL` (key 3) is
/// `STRING(20)`, and `ZONE1` (key 11) is `ST_Zone@1` (type 901).
pub(super) fn typed_world() -> Fixture {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);
    let [array, string] = derived_rows();
    fixture.store.seed_type(&array);
    fixture.store.seed_type(&string);
    let placed = [
        (1, "SPEED", None, json!(120)),
        (2, "VALUES", Some(950), json!([1, 2, 3, 4])),
        (3, "LABEL", Some(951), json!("abc")),
    ];
    for (n, tag, plc_type, value) in placed {
        let base = parameter_on(PLATFORM, n, tag, uid(101), 0);
        let parameter = StoredParameter {
            plc_type: plc_type.map_or(base.plc_type, tid),
            value: StoredValue::from(value),
            limits: Vec::new(),
            ordinal: Ordinal::parse(u32::try_from(n).unwrap()).unwrap(),
            ..base
        };
        fixture.store.seed_parameter(vid(101), &parameter);
    }
    fixture
}

/// The type text of each parameter key of `typed_world`.
pub(super) const TYPE_TEXTS: [(u128, &str); 4] = [
    (1, "INT"),
    (2, "ARRAY[1..4] OF INT"),
    (3, "STRING(20)"),
    (11, "ST_Zone@1"),
];

async fn list_with(
    fixture: &Fixture,
    settings: AppSettings,
    role: Role,
    (unit, page): (Option<&str>, u32),
) -> Result<ParameterList, AppError> {
    ListParameters::new(fixture.context_with(settings))
        .execute(
            &actor(role),
            &meta(),
            ListParametersInput {
                reference: reference("R-1"),
                number: 1,
                unit: unit.map(path),
                page,
            },
        )
        .await
}

async fn list(fixture: &Fixture, unit: Option<&str>, page: u32) -> ParameterList {
    list_with(
        fixture,
        AppSettings::default(),
        Role::Operator,
        (unit, page),
    )
    .await
    .unwrap()
}

fn keys(list: &ParameterList) -> Vec<ParameterKey> {
    list.rows.iter().map(|row| row.key).collect()
}

fn counts(list: &ParameterList) -> Vec<(String, u32)> {
    list.units
        .iter()
        .map(|count| (count.path.to_string(), count.count))
        .collect()
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(ListParameters::OPERATION, Operation::ListParameters);
}

#[tokio::test]
async fn a_small_version_gives_every_row_in_unit_order_with_the_counts_and_the_row_version() {
    let fixture = world();

    let result = list(&fixture, None, 0).await;

    assert_eq!(keys(&result), [pkey(1), pkey(2), pkey(3), pkey(4)]);
    assert_eq!(result.unit, None);
    assert_eq!(
        counts(&result),
        [
            ("U1".to_owned(), 2),
            ("U1.2".to_owned(), 1),
            ("U1.2.1".to_owned(), 1)
        ]
    );
    assert_eq!(result.detail.row_version, 1);
    assert_eq!(result.recipe.reference().to_string(), "R-1");
    assert_eq!((result.page, result.has_next), (0, false));
}

#[tokio::test]
async fn the_list_gives_the_type_text_of_each_type_on_the_page() {
    let fixture = typed_world();

    let result = list(&fixture, None, 0).await;

    let by_key: BTreeMap<ParameterKey, Option<&str>> = result
        .rows
        .iter()
        .map(|row| {
            (
                row.key,
                result.type_texts.get(&row.plc_type).map(String::as_str),
            )
        })
        .collect();
    let expected: BTreeMap<ParameterKey, Option<&str>> = TYPE_TEXTS
        .iter()
        .map(|(key, text)| (pkey(*key), Some(*text)))
        .collect();
    assert_eq!(by_key, expected);
    assert_eq!(result.type_texts.len(), 4, "one text per type on the page");
}

#[tokio::test]
async fn the_unit_filter_keeps_the_rows_of_that_unit() {
    let fixture = world();

    let result = list(&fixture, Some("U1"), 0).await;

    assert_eq!(keys(&result), [pkey(1), pkey(2)]);
    assert_eq!(result.unit, Some(path("U1")));
    assert_eq!(result.units.len(), 3);
}

#[tokio::test]
async fn the_page_cuts_the_rows_and_tells_whether_a_next_page_exists() {
    let fixture = world();

    let first = list_with(&fixture, pages_of(1), Role::Operator, (Some("U1"), 0))
        .await
        .unwrap();
    let second = list_with(&fixture, pages_of(1), Role::Operator, (Some("U1"), 1))
        .await
        .unwrap();

    assert_eq!(keys(&first), [pkey(1)]);
    assert_eq!((first.page, first.has_next), (0, true));
    assert_eq!(keys(&second), [pkey(2)]);
    assert_eq!((second.page, second.has_next), (1, false));
}

#[tokio::test]
async fn a_large_version_without_a_unit_shows_its_first_unit_and_every_count() {
    let fixture = world();

    let result = list_with(&fixture, pages_of(2), Role::Operator, (None, 0))
        .await
        .unwrap();

    assert_eq!(keys(&result), [pkey(1), pkey(2)]);
    assert_eq!(result.unit, Some(path("U1")));
    assert_eq!(
        counts(&result),
        [
            ("U1".to_owned(), 2),
            ("U1.2".to_owned(), 1),
            ("U1.2.1".to_owned(), 1)
        ]
    );
    assert!(!result.has_next);
}

#[tokio::test]
async fn a_large_version_that_is_not_a_draft_pages_across_the_units() {
    let fixture = approved_world();

    let result = list_with(&fixture, pages_of(3), Role::Operator, (None, 0))
        .await
        .unwrap();

    assert_eq!(keys(&result), [pkey(1), pkey(2), pkey(3)]);
    assert_eq!(result.unit, None);
    assert_eq!(result.units.len(), 3);
    assert!(result.has_next);
}

#[tokio::test]
async fn a_unit_that_the_version_does_not_use_is_not_found() {
    let fixture = world();

    let error = list_with(
        &fixture,
        AppSettings::default(),
        Role::Operator,
        (Some("U7"), 0),
    )
    .await
    .unwrap_err();

    let expected = Missing::Unit {
        code: MachineCode::parse("M1").unwrap(),
        path: path("U7"),
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity } if entity == &expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn list_parameters_reads_in_one_repeatable_read_transaction_and_writes_nothing() {
    let fixture = world();

    list(&fixture, None, 0).await;

    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert!(fixture.store.audit_entries().is_empty());
    assert!(fixture.sink.events().is_empty());
}

#[tokio::test]
async fn an_unknown_recipe_or_version_is_not_found() {
    let fixture = world();
    let query = ListParameters::new(fixture.context());
    let unknown_recipe = ListParametersInput {
        reference: reference("R-9"),
        number: 1,
        unit: None,
        page: 0,
    };
    let unknown_version = ListParametersInput {
        number: 7,
        ..unknown_recipe.clone()
    };
    let unknown_version = ListParametersInput {
        reference: reference("R-1"),
        ..unknown_version
    };

    let recipe = query
        .execute(&actor(Role::Operator), &meta(), unknown_recipe)
        .await
        .unwrap_err();
    let version = query
        .execute(&actor(Role::Operator), &meta(), unknown_version)
        .await
        .unwrap_err();

    assert!(matches!(
        recipe,
        AppError::NotFound {
            entity: Missing::Recipe(_)
        }
    ));
    assert!(matches!(
        version,
        AppError::NotFound {
            entity: Missing::Version { number: 7, .. }
        }
    ));
}

#[tokio::test]
async fn a_version_of_a_deleted_recipe_is_refused_except_for_the_role_that_restores() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);

    let operator = list_with(&fixture, AppSettings::default(), Role::Operator, (None, 0))
        .await
        .unwrap_err();
    let engineer = list_with(
        &fixture,
        AppSettings::default(),
        Role::ProcessEngineer,
        (None, 0),
    )
    .await
    .unwrap_err();
    let administrator = list_with(
        &fixture,
        AppSettings::default(),
        Role::Administrator,
        (None, 0),
    )
    .await
    .unwrap();

    assert_eq!(operator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
    assert!(administrator.recipe.is_deleted());
    assert!(administrator.rows.is_empty());
}
