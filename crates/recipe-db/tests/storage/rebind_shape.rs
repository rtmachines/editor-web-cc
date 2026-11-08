//! Rebind across the seed row of `STRING` on PostgreSQL (ADR-0146 (c), (g), (h), design D5).
//!
//! A struct revision 1 holds a member `label` of type `STRING(80)`, which is a row of the
//! machine. Revision 2 holds `label` of type `STRING`, which is the seed row. The two rows have one
//! type shape, so a rebind without new member values keeps the value and the limits of the
//! parameter. The rebind writes the limit rows again, so the test does not compare `created_at`.

use std::collections::BTreeMap;

use recipe_app::plant::{DefineUdt, DefineUdtInput, DefineUdtMode};
use recipe_app::recipe::{
    AddParameter, AddParameterInput, RebindTypeRevision, RebindTypeRevisionInput,
};
use recipe_domain::plant::{LeafPath, MemberName, TypeExpr, TypeName, UnitPath};
use recipe_domain::recipe::{
    LimitInput, LimitList, Ordinal, ParameterTag, ReferenceNumber, ValueInput,
};
use recipe_domain::text::{LongText, Text};
use uuid::Uuid;

use crate::support::app::{App, Seeded};
use crate::support::fixtures;

const CODESYS: &str = "CodesysIec";
/// The seed row of `STRING` (`0004_seed_plc_types.sql`).
const SEED_STRING: Uuid = Uuid::from_u128(0x5eed_0000_0000_4000_8000_0000_0000_001e);

/// The row of one member type of a struct revision.
const MEMBER_TYPE: &str = "SELECT member_type_id FROM plc_type_member \
                           WHERE struct_type_id = $1 AND member_name = 'label'";
/// The machine and the string length of a type row.
const TYPE_ROW: &str = "SELECT machine_id, string_length FROM plc_type WHERE id = $1";
/// The parameter row of the tag `LABEL_1` in a draft: type row and value text.
const PARAMETER: &str = "SELECT plc_type_id, value::text FROM parameter \
                         WHERE recipe_version_id = $1 AND tag = 'LABEL_1'";
/// The limit rows of the parameter `LABEL_1`, without `created_at`.
const LIMITS: &str = "SELECT l.leaf_path, l.min_value::text, l.max_value::text, l.max_length \
                      FROM parameter_limit l JOIN parameter p ON p.id = l.parameter_id \
                      WHERE p.recipe_version_id = $1 AND p.tag = 'LABEL_1' \
                      ORDER BY l.leaf_path";

type LimitRow = (String, Option<String>, Option<String>, Option<i16>);

fn text(value: &str) -> Text {
    Text::parse(value).unwrap()
}

/// A `CodesysIec` machine with the unit `U1`, a recipe, and its live draft.
struct Draft {
    seed: Seeded,
    version: Uuid,
    reference: ReferenceNumber,
}

async fn draft(app: &mut App) -> Draft {
    let seed = app.seed_machine(CODESYS, &["U1"]).await;
    let creator = *app.user.user().as_uuid();
    let recipe = fixtures::recipe(app.migrator(), seed.id, CODESYS, creator).await;
    let version = fixtures::draft_version(app.migrator(), recipe, CODESYS, 1, creator).await;
    let reference: String = sqlx::query_scalar("SELECT reference_number FROM recipe WHERE id = $1")
        .bind(recipe)
        .fetch_one(app.migrator())
        .await
        .unwrap();
    Draft {
        seed,
        version,
        reference: ReferenceNumber::parse(&reference).unwrap(),
    }
}

/// Run `DefineUdt` for `ST_Text` with the member `label` of the listed type.
async fn define_text(app: &mut App, seed: &Seeded, label: &str, mode: DefineUdtMode) -> Uuid {
    let input = DefineUdtInput {
        code: seed.code,
        name: TypeName::parse("ST_Text").unwrap(),
        members: vec![(
            MemberName::parse("label").unwrap(),
            TypeExpr::parse(label).unwrap(),
        )],
        mode,
    };
    let output = DefineUdt::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    *output.type_id.as_uuid()
}

async fn row_version(app: &mut App, version: Uuid) -> i64 {
    app.count(
        "SELECT row_version FROM recipe_version WHERE id = $1",
        version,
    )
    .await
}

/// Run `AddParameter` for `LABEL_1` of type `ST_Text@1` with the value `"ABC"` and the limit
/// `max_length` 40 on `.label`.
async fn add_label(app: &mut App, draft: &Draft) {
    let expected = row_version(app, draft.version).await;
    let limit = LimitInput {
        leaf_path: LeafPath::parse(".label").unwrap(),
        min: None,
        max: None,
        max_length: Some(40),
    };
    let input = AddParameterInput {
        reference: draft.reference.clone(),
        number: 1,
        expected_row_version: expected,
        unit_path: UnitPath::parse("U1").unwrap(),
        tag: ParameterTag::parse("LABEL_1").unwrap(),
        name_en: text("Label"),
        description_en: LongText::parse("Label of the text").unwrap(),
        type_expr: TypeExpr::parse("ST_Text@1").unwrap(),
        s7_address: None,
        ordinal: Ordinal::parse(1).unwrap(),
        value: ValueInput::Json("{\"label\": \"ABC\"}".to_owned()),
        limits: LimitList::parse(vec![limit]).unwrap(),
    };
    AddParameter::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
}

async fn member_type(app: &mut App, struct_type: Uuid) -> Uuid {
    sqlx::query_scalar(MEMBER_TYPE)
        .bind(struct_type)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

async fn parameter(app: &mut App, version: Uuid) -> (Uuid, String) {
    sqlx::query_as(PARAMETER)
        .bind(version)
        .fetch_one(app.admin())
        .await
        .unwrap()
}

async fn limit_rows(app: &mut App, version: Uuid) -> Vec<LimitRow> {
    sqlx::query_as(LIMITS)
        .bind(version)
        .fetch_all(app.admin())
        .await
        .unwrap()
}

#[tokio::test]
async fn rebind_keeps_the_value_and_the_limits_across_the_seed_row_of_string() {
    let mut app = App::create().await;
    let draft = draft(&mut app).await;
    let revision_one = define_text(&mut app, &draft.seed, "STRING(80)", DefineUdtMode::New).await;
    let revision_two = define_text(&mut app, &draft.seed, "STRING", DefineUdtMode::Revision).await;

    // The two member rows are two rows with one shape.
    assert_eq!(member_type(&mut app, revision_two).await, SEED_STRING);
    let machine_row = member_type(&mut app, revision_one).await;
    let (machine, length): (Option<Uuid>, Option<i16>) = sqlx::query_as(TYPE_ROW)
        .bind(machine_row)
        .fetch_one(app.admin())
        .await
        .unwrap();
    assert_eq!(machine, Some(draft.seed.id));
    assert_eq!(length, Some(80));

    add_label(&mut app, &draft).await;
    let (type_before, value_before) = parameter(&mut app, draft.version).await;
    assert_eq!(type_before, revision_one);
    let limits_before = limit_rows(&mut app, draft.version).await;
    assert_eq!(limits_before.len(), 1);
    let expected = row_version(&mut app, draft.version).await;
    let head = app.head().await;

    let input = RebindTypeRevisionInput {
        reference: draft.reference.clone(),
        number: 1,
        expected_row_version: expected,
        from: revision_one.into(),
        to: revision_two.into(),
        new_member_values: BTreeMap::new(),
    };
    RebindTypeRevision::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();

    let (type_after, value_after) = parameter(&mut app, draft.version).await;
    assert_eq!(type_after, revision_two);
    assert_eq!(value_after, value_before);
    assert_eq!(limit_rows(&mut app, draft.version).await, limits_before);
    app.assert_chain_moved(&head, &["TypeRevisionRebound"])
        .await;
}
