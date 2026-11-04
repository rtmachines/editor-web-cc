//! Tests of `GetParameterForm` (design D4, ADR-0039 (h), ADR-0043 (e)).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use std::collections::BTreeSet;

use recipe_domain::plant::{
    LeafPath, MachineCode, PlcPlatform, PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape,
    S7Address, TypeExpr, UnitPath,
};
use recipe_domain::recipe::{
    Ordinal, RecipeError, RuleField, StoredLimit, StoredParameter, StoredValue, VersionState,
    VersionStatus,
};
use serde_json::json;

use super::parameter_form::{GetParameterForm, GetParameterFormInput, ParameterForm};
use super::parameters_tests::{TYPE_TEXTS, typed_world};
use super::test_world::seed_users;
use crate::auth::Operation;
use crate::context::UseCase;
use crate::error::{AppError, Missing};
use crate::memory::fixtures::{
    Fixture, actor, meta, parameter_on, pkey, recipe_with, reference, scenarios, tid, uid, user,
    vid,
};
use crate::memory::{JournalEntry, seed_type_id};
use crate::plant::{GetUdtForm, UdtFormInput};
use crate::ports::Isolation;

/// The draft `R-1` with the parameter `ZONE1` (key 11) of `ST_Zone@1`, and `ST_Zone@2`.
fn world() -> Fixture {
    let fixture = scenarios::zone_draft();
    seed_users(&fixture);
    fixture
}

/// The draft `R-1` on the `S7_300` machine `M1` without a parameter.
fn s7_world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PlcPlatform::S7_300, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture
}

fn input(key: Option<u128>, type_text: Option<&str>, unit: Option<&str>) -> GetParameterFormInput {
    GetParameterFormInput {
        purpose: crate::auth::ParameterFormPurpose::AddParameter,
        reference: reference("R-1"),
        number: 1,
        key: key.map(pkey),
        type_expr: type_text.map(|text| TypeExpr::parse(text).unwrap()),
        unit: unit.map(|text| UnitPath::parse(text).unwrap()),
    }
}

async fn form_as(
    fixture: &Fixture,
    role: Role,
    input: GetParameterFormInput,
) -> Result<ParameterForm, AppError> {
    GetParameterForm::new(fixture.context())
        .execute(&actor(role), &meta(), input)
        .await
}

async fn form(fixture: &Fixture, input: GetParameterFormInput) -> Result<ParameterForm, AppError> {
    form_as(fixture, Role::ProcessEngineer, input).await
}

/// The number of type rows of the store.
fn type_rows(fixture: &Fixture) -> usize {
    fixture.store.state().types.len()
}

fn leaf_paths(form: &ParameterForm) -> Vec<String> {
    form.preview
        .as_ref()
        .map(|tree| {
            tree.leaves()
                .iter()
                .map(|leaf| leaf.path().to_string())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_query_declares_its_operation() {
    assert_eq!(GetParameterForm::OPERATION, Operation::GetParameterForm);
}

#[tokio::test]
async fn the_form_without_a_key_gives_the_live_units_the_platform_and_the_proposals() {
    let fixture = world();

    let result = form(&fixture, input(None, None, None)).await.unwrap();
    let udt_form = GetUdtForm::new(fixture.context())
        .execute(
            &actor(Role::ProcessEngineer),
            &meta(),
            UdtFormInput {
                purpose: crate::auth::UdtFormPurpose::DefineUdt,
                code: MachineCode::parse("M1").unwrap(),
                from: None,
            },
        )
        .await
        .unwrap();

    let units: Vec<String> = result
        .units
        .iter()
        .map(|unit| unit.path().to_string())
        .collect();
    assert_eq!(units, ["U1", "U1.2", "U1.2.1"]);
    assert_eq!(result.machine.platform(), PlcPlatform::TwinCat3);
    assert_eq!(result.choices, udt_form.choices);
    assert!(result.choices.iter().any(|choice| choice == "ST_Zone@2"));
    assert!(!result.choices.iter().any(|choice| choice == "ST_Zone@1"));
    assert!(result.parameter.is_none());
    assert!(result.preview.is_none());
    assert_eq!(result.unit, None);
    assert_eq!(result.detail.row_version, 1);
}

#[tokio::test]
async fn the_form_with_a_key_and_a_unit_gives_the_parameter_and_keeps_the_unit() {
    let fixture = world();

    let result = form(&fixture, input(Some(11), None, Some("U1.2")))
        .await
        .unwrap();

    assert_eq!(
        result.parameter.as_ref().map(|found| found.tag.to_string()),
        Some("ZONE1".to_owned())
    );
    assert_eq!(result.unit, Some(UnitPath::parse("U1.2").unwrap()));
}

#[tokio::test]
async fn the_form_with_a_type_gives_the_preview_tree_and_writes_nothing() {
    let fixture = world();
    let rows_before = type_rows(&fixture);

    let result = form(
        &fixture,
        input(None, Some("ARRAY[1..2] OF ST_Zone@2"), None),
    )
    .await
    .unwrap();

    assert_eq!(
        leaf_paths(&result),
        ["[1].setpoint", "[1].extra", "[2].setpoint", "[2].extra"]
    );
    assert_eq!(type_rows(&fixture), rows_before, "no new type row");
    assert_eq!(
        fixture.store.journal(),
        [JournalEntry::Begin(Isolation::RepeatableRead)]
    );
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_type_that_the_platform_does_not_hold_gives_plt_013() {
    let fixture = s7_world();

    let error = form(&fixture, input(None, Some("WSTRING(10)"), None))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_type_above_the_leaf_limit_gives_plt_018() {
    let fixture = world();
    let rows_before = type_rows(&fixture);

    let error = form(&fixture, input(None, Some("ARRAY[1..16385] OF INT"), None))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::PLT_018);
    assert_eq!(type_rows(&fixture), rows_before);
}

#[tokio::test]
async fn a_unit_that_is_not_a_live_unit_of_the_machine_is_not_found() {
    let fixture = world();

    let error = form(&fixture, input(None, None, Some("U9")))
        .await
        .unwrap_err();

    let expected = Missing::Unit {
        code: MachineCode::parse("M1").unwrap(),
        path: UnitPath::parse("U9").unwrap(),
    };
    assert_eq!(error.code(), ErrorCode::WEB_005);
    assert!(
        matches!(&error, AppError::NotFound { entity } if entity == &expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_key_that_the_version_does_not_hold_is_not_found() {
    let fixture = world();

    let error = form(&fixture, input(Some(9), None, None))
        .await
        .unwrap_err();

    let expected = Missing::Parameter {
        reference: reference("R-1"),
        number: 1,
        key: pkey(9),
    };
    assert!(
        matches!(&error, AppError::NotFound { entity } if entity == &expected),
        "{error:?}"
    );
}

#[tokio::test]
async fn the_form_of_a_deleted_recipe_is_refused_for_every_role() {
    let fixture = scenarios::deleted();
    seed_users(&fixture);

    let administrator = form_as(&fixture, Role::Administrator, input(None, None, None))
        .await
        .unwrap_err();
    let engineer = form_as(&fixture, Role::ProcessEngineer, input(None, None, None))
        .await
        .unwrap_err();

    assert_eq!(administrator.code(), ErrorCode::RCP_012);
    assert_eq!(engineer.code(), ErrorCode::RCP_012);
}

#[tokio::test]
async fn the_edit_form_gives_the_type_text_of_each_kind_of_type() {
    let fixture = typed_world();

    for (key, text) in TYPE_TEXTS {
        let result = form(&fixture, input(Some(key), None, None)).await.unwrap();

        assert_eq!(result.type_text.as_deref(), Some(text), "key {key}");
    }
    let new = form(&fixture, input(None, None, None)).await.unwrap();
    assert_eq!(new.type_text, None);
}

/// Run the form query as a process engineer on version `number` of `R-1`.
async fn form_of(fixture: &Fixture, number: u32) -> Result<ParameterForm, AppError> {
    let target = GetParameterFormInput {
        number,
        ..input(None, None, None)
    };
    form(fixture, target).await
}

/// Tell whether `error` is `E-RCP-002` for version `number` of `R-1` with `status`.
fn is_not_editable(error: &AppError, number: u32, status: VersionStatus) -> bool {
    let fields = matches!(
        error,
        AppError::Recipe(RecipeError::VersionNotEditable {
            reference: found,
            number: found_number,
            status: found_status,
        }) if *found == reference("R-1") && *found_number == number && *found_status == status
    );
    error.code() == ErrorCode::RCP_002 && fields
}

#[tokio::test]
async fn an_approved_version_gives_rcp_002_and_the_live_draft_gives_the_form() {
    let fixture = scenarios::approved_and_draft();
    seed_users(&fixture);

    let approved = form_of(&fixture, 1).await.unwrap_err();
    let draft = form_of(&fixture, 2).await.unwrap();

    assert!(
        is_not_editable(&approved, 1, VersionStatus::Approved),
        "{approved:?}"
    );
    assert_eq!(draft.detail.number, 2);
}

#[tokio::test]
async fn a_retired_version_gives_rcp_002() {
    let fixture = scenarios::retired();
    seed_users(&fixture);

    let error = form_of(&fixture, 1).await.unwrap_err();

    assert!(
        is_not_editable(&error, 1, VersionStatus::Retired),
        "{error:?}"
    );
}

#[tokio::test]
async fn a_discarded_draft_gives_rcp_002() {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PlcPlatform::TwinCat3, 1, "M1");
    let recipe = recipe_with((1, 1), "R-1", &[(101, VersionState::Discarded)]);
    fixture
        .store
        .seed_recipe(&recipe, machine.platform(), user(2));

    let error = form_of(&fixture, 1).await.unwrap_err();

    assert!(
        is_not_editable(&error, 1, VersionStatus::Draft),
        "{error:?}"
    );
}

#[tokio::test]
async fn an_operator_is_refused_and_the_store_stays_unread() {
    let fixture = world();

    let error = form_as(&fixture, Role::Operator, input(None, None, None))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_002);
    assert!(fixture.store.journal().is_empty());
    assert_eq!(fixture.sink.events().len(), 1);
}

/// The key of the `REAL` parameter `RATE` of `rule_fault_world`.
pub(super) const RATE_KEY: u128 = 1;

/// The key of the `DATE` parameter `DUE` of `rule_fault_world`.
pub(super) const DUE_KEY: u128 = 2;

/// The key of the `DINT` parameter `OFFSET` of `rule_fault_world`.
pub(super) const OFFSET_KEY: u128 = 3;

/// The key of the `DATE` parameter `LATE` of `rule_fault_world`. Its stored value is a defect.
pub(super) const LATE_KEY: u128 = 4;

/// A stored limit on the whole value of a parameter.
fn whole_limit(max: i64) -> StoredLimit {
    StoredLimit {
        leaf_path: LeafPath::parse("").unwrap(),
        min: None,
        max: Some(max.to_string().parse().unwrap()),
        max_length: None,
    }
}

/// The identifier of a seeded primitive type.
fn seeded(primitive: Primitive) -> PlcTypeId {
    seed_type_id(primitive, None).unwrap()
}

/// The global `DATE` type on `S7_1500`. The in-memory seed has no `DATE` row.
fn date_type() -> PlcType {
    let shape = RecordShape::Primitive {
        primitive: Primitive::Date,
        leaf_class: Primitive::Date.leaf_class(),
        size_bytes: Primitive::Date.fixed_size_bytes().unwrap(),
        string_length: None,
    };
    PlcType::try_from(PlcTypeRecord {
        id: tid(960),
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::S7_1500]),
        shape,
    })
    .unwrap()
}

/// A parameter `tag` of `plc_type` on `U1` with the stored `value`, `limits`, and `address`.
fn stored_primitive(
    (key, tag, plc_type): (u128, &str, PlcTypeId),
    (value, limits, address): (serde_json::Value, Vec<StoredLimit>, Option<&str>),
) -> StoredParameter {
    StoredParameter {
        plc_type,
        s7_address: address.map(|text| S7Address::parse(text).unwrap()),
        limits,
        value: StoredValue::from(value),
        ordinal: Ordinal::parse(u32::try_from(key).unwrap()).unwrap(),
        ..parameter_on(PlcPlatform::S7_1500, key, tag, uid(101), 0)
    }
}

/// The draft `R-1` on an `S7_1500` machine with parameters that break the current rules only.
///
/// One parameter has one type, so one parameter cannot hold the three faults. A `REAL` value
/// gives `E-RCP-055`. Only a `DATE` limit gives `E-RCP-006`, because the `DATE` range is the one
/// that the rule set changes. An address gives `E-RCP-056`. So the world has one parameter for
/// each fault. `LATE` breaks the earlier rules too, so it is a structure fault.
pub(super) fn rule_fault_world() -> Fixture {
    let fixture = Fixture::new();
    seed_users(&fixture);
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_draft(&machine, 1, "R-1");
    fixture.store.seed_type(&date_type());
    let date = date_type().id();
    let parameters = [
        stored_primitive(
            (RATE_KEY, "RATE", seeded(Primitive::Real)),
            (json!(16_777_217), Vec::new(), None),
        ),
        stored_primitive(
            (DUE_KEY, "DUE", date),
            (json!(72_683), vec![whole_limit(72_700)], None),
        ),
        stored_primitive(
            (OFFSET_KEY, "OFFSET", seeded(Primitive::Dint)),
            (json!(0), Vec::new(), Some("DB1.DBD65534")),
        ),
        stored_primitive((LATE_KEY, "LATE", date), (json!(72_900), Vec::new(), None)),
    ];
    for parameter in &parameters {
        fixture.store.seed_parameter(vid(101), parameter);
    }
    fixture
}

#[tokio::test]
async fn a_stored_parameter_gives_its_value_limit_and_address_rule_faults_with_their_fields() {
    let fixture = rule_fault_world();
    let mut found = Vec::new();

    for key in [RATE_KEY, DUE_KEY, OFFSET_KEY] {
        let result = form(&fixture, input(Some(key), None, None)).await.unwrap();
        for fault in result.rule_faults.iter() {
            found.push((key, fault.code(), fault.field().clone()));
        }
    }

    assert_eq!(
        found,
        [
            (
                RATE_KEY,
                ErrorCode::RCP_055,
                RuleField::Value(LeafPath::parse("").unwrap())
            ),
            (
                DUE_KEY,
                ErrorCode::RCP_006,
                RuleField::LimitMax(LeafPath::parse("").unwrap())
            ),
            (OFFSET_KEY, ErrorCode::RCP_056, RuleField::Address),
        ]
    );
}

#[tokio::test]
async fn a_stored_parameter_that_breaks_the_earlier_rules_keeps_db_008() {
    let fixture = rule_fault_world();

    let error = form(&fixture, input(Some(LATE_KEY), None, None))
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::DB_008);
    assert!(fixture.store.audit_entries().is_empty());
}

#[tokio::test]
async fn a_form_without_a_key_and_a_clean_parameter_give_no_rule_fault() {
    let fixture = world();

    let new = form(&fixture, input(None, None, None)).await.unwrap();
    let edit = form(&fixture, input(Some(11), None, None)).await.unwrap();

    assert!(new.rule_faults.is_empty());
    assert!(edit.rule_faults.is_empty());
}
