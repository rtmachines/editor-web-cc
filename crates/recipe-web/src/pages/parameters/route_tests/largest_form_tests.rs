//! Tests of the largest edit form that the budgets of A26 allow (ADR-0211 (d) to (j)).

use std::collections::BTreeSet;

use axum::http::StatusCode;
use recipe_app::memory::{MemoryState, seed_type_id};
use recipe_domain::identity::UserId;
use recipe_domain::plant::{
    IecLiteral, LeafClass, Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform, PlcType,
    PlcTypeId, PlcTypeRecord, Primitive, RecordShape, RuleSet, StringEncoding, TimeType, TypeTree,
    UnitId,
};
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, ParameterValue, Recipe, RecipeId,
    RecipeVersionId, ReferenceNumber, StoredParameter, StoredRecipe, StoredValue,
    StoredVersionHeader, VersionState,
};
use recipe_domain::text::{LongText, Text};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::pages::parameters::test_kit::form;
use crate::pages::test_helpers::encode;
use crate::settings::WebSettings;
use crate::test_support::get_bounded;
use crate::test_support::plant::{ENGLISH, World, world};

/// The number of array levels of the type. It equals `json.max_depth`.
const LEVELS: usize = 32;
/// The number of outer levels with two elements. Their product is `budget.leaves_per_parameter`.
pub(super) const WIDE_LEVELS: usize = 14;
/// The number of leaves: 2 to the power of `WIDE_LEVELS`.
pub(super) const LEAVES: usize = 1 << WIDE_LEVELS;
/// The first index of every level. Both indexes of a level have two digits.
const FIRST_INDEX: usize = 10;
/// The second index of an outer level.
const SECOND_INDEX: usize = 11;
/// The longest canonical `LDT` text. It has 33 bytes and encodes to 39 bytes.
pub(super) const MAX_TEXT: &str = "LDT#2262-04-11-23:47:16.854775807";
/// The lowest `LDT` text with a fraction of nine digits.
pub(super) const MIN_TEXT: &str = "LDT#1970-01-01-00:00:00.000000001";
/// The longest encoded text of a leaf of a form (ADR-0211 (f)).
const MAX_ENCODED_TEXT: usize = 40;
/// The size of the old limit of the value routes: 4 MiB (ADR-0039 (g)).
const OLD_LIMIT: usize = 4 * 1024 * 1024;
/// The reference number of the recipe of the world.
pub(super) const REFERENCE: &str = "R-440";
/// The key of the parameter `BIG`.
pub(super) const KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0f44_0001);
/// The identifier of the draft of the world.
pub(super) const VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f44_0001);
/// The identifier of the machine `M44`.
const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f44_0001);
/// The identifier of the unit `U1` of `M44`.
const UNIT_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f44_0001);
/// The identifier of the recipe `R-440`.
const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f44_0001);
/// The first field of the identifier of a primitive row of the text check.
const ROW_BASE: u128 = 0x7357_0013_0000_0000_0000_0000_0f44_0000;

/// Return the indexes of the level `level`: 10 and 11 for each of the first `wide` levels, else 10.
fn indexes(level: usize, wide: usize) -> std::ops::RangeInclusive<usize> {
    let last = if level < wide {
        SECOND_INDEX
    } else {
        FIRST_INDEX
    };
    FIRST_INDEX..=last
}

/// Return the leaf path of the leaf number `leaf` with `WIDE_LEVELS` wide levels.
pub(super) fn leaf_path(leaf: usize) -> String {
    leaf_path_of(leaf, WIDE_LEVELS)
}

/// Return the concrete leaf path of the leaf number `leaf`, for example `[10][11]...[10]`.
///
/// The binary digits of `leaf` pick the index of each of the `wide` outer levels. The other
/// levels have one index, so every path has 32 segments of 4 bytes.
pub(super) fn leaf_path_of(leaf: usize, wide: usize) -> String {
    let bits = format!("{leaf:0wide$b}");
    (0..LEVELS)
        .map(|level| {
            let second = bits.as_bytes().get(level) == Some(&b'1');
            format!("[{}]", if second { SECOND_INDEX } else { FIRST_INDEX })
        })
        .collect()
}

/// Return the text of the type: 32 `ARRAY` prefixes and the base `LDT`.
fn type_text(wide: usize) -> String {
    let prefixes: String = (0..LEVELS)
        .map(|level| format!("ARRAY[{}..{}] OF ", FIRST_INDEX, indexes(level, wide).end()))
        .collect();
    format!("{prefixes}LDT")
}

/// Return the value text of the leaf number `leaf`: a time with a fraction of nine digits.
pub(super) fn value_text(leaf: usize) -> String {
    format!("LDT#2262-04-11-23:47:16.854775{:03}", leaf % 800)
}

/// Return the count of an `LDT` text on `TwinCat3`.
pub(super) fn ldt_count(text: &str) -> i64 {
    let time_type = TimeType::of(Primitive::Ldt, PlcPlatform::TwinCat3).unwrap();
    i64::try_from(IecLiteral::parse(time_type, text).unwrap().count()).unwrap()
}

/// Return the nested JSON that opens the level `level`. `next` is the number of the next leaf.
pub(super) fn nested(level: usize, next: &mut usize, wide: usize) -> Value {
    if level == LEVELS {
        let count = ldt_count(&value_text(*next));
        *next = next.checked_add(1).unwrap();
        return json!(count);
    }
    let deeper = level.checked_add(1).unwrap();
    Value::Array(
        indexes(level, wide)
            .map(|_index| nested(deeper, next, wide))
            .collect(),
    )
}

/// Return the pairs of the edit form of `BIG` with `WIDE_LEVELS` wide levels.
pub(super) fn form_pairs() -> Vec<(String, String)> {
    form_pairs_of(WIDE_LEVELS)
}

/// Return every pair of the edit form of `BIG` with the new type, the values, and the limits.
///
/// The type has `wide` array levels with two elements. The form has `2^wide` leaves.
pub(super) fn form_pairs_of(wide: usize) -> Vec<(String, String)> {
    let text = type_text(wide);
    let mut pairs: Vec<(String, String)> = [
        ("row_version", "1"),
        ("unit_path", "U1"),
        ("tag", "BIG"),
        ("name_en", "Big"),
        ("description_en", "The largest edit form"),
        ("s7_address", ""),
        ("ordinal", "0"),
    ]
    .iter()
    .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
    .collect();
    pairs.push(("type".to_owned(), text.clone()));
    pairs.push(("value_type".to_owned(), text));
    for leaf in 0..(1_usize << wide) {
        let path = leaf_path_of(leaf, wide);
        pairs.push((format!("value{path}"), value_text(leaf)));
        pairs.push((format!("limits[{leaf}].leaf_path"), path));
        pairs.push((format!("limits[{leaf}].min"), MIN_TEXT.to_owned()));
        pairs.push((format!("limits[{leaf}].max"), MAX_TEXT.to_owned()));
    }
    pairs
}

/// Return the global row of the primitive `primitive` on `TwinCat3` with the identifier `id`.
fn primitive_row(id: PlcTypeId, primitive: Primitive) -> PlcType {
    PlcType::try_from(PlcTypeRecord {
        id,
        machine: None,
        platforms: BTreeSet::from([PlcPlatform::TwinCat3]),
        shape: RecordShape::Primitive {
            primitive,
            leaf_class: primitive.leaf_class(),
            size_bytes: primitive.fixed_size_bytes().unwrap(),
            string_length: None,
        },
    })
    .unwrap()
}

/// Return the canonical texts of the values `candidates` of the primitive `primitive`.
///
/// A candidate that the primitive does not accept gives no text. The `0` is always accepted.
fn canonical_texts(primitive: Primitive, index: u128, candidates: &[&str]) -> Vec<String> {
    let id = PlcTypeId::from(Uuid::from_u128(ROW_BASE | index));
    let machine = MachineId::from(MACHINE_ID);
    let types = PlatformTypes::collect(
        machine,
        PlcPlatform::TwinCat3,
        StringEncoding::Latin1,
        vec![primitive_row(id, primitive)],
    )
    .unwrap();
    let tree = TypeTree::resolve(id, &types).unwrap();
    candidates
        .iter()
        .filter_map(|json| ParameterValue::parse(json, &tree).ok())
        .flat_map(|value| value.texts(&tree).unwrap())
        .map(|leaf| leaf.text)
        .collect()
}

/// Assert that each canonical text of a numeric, bool, duration, or instant leaf encodes to at
/// most 40 bytes (ADR-0211 (f)). The extreme texts of every platform and rule set are in it.
fn assert_canonical_texts_fit() {
    let mut texts: Vec<String> = Vec::new();
    for primitive in Primitive::ALL {
        for platform in PlcPlatform::ALL {
            let Some(time_type) = TimeType::of(primitive, platform) else {
                continue;
            };
            for rules in [RuleSet::Current, RuleSet::Earlier] {
                let Some((low, high)) = time_type.range_under(rules) else {
                    continue;
                };
                // A count that is no step or day of its type gives no text.
                let ends = [
                    Some(low),
                    low.checked_add(1),
                    high.checked_sub(1),
                    Some(high),
                ];
                for count in ends.into_iter().flatten() {
                    if let Ok(literal) = IecLiteral::from_count(time_type, count, rules) {
                        texts.push(literal.to_string());
                    }
                }
            }
        }
    }
    let candidates = [
        "0",
        "true",
        "-9223372036854775808",
        "18446744073709551615",
        "-79228162514264337593543950335",
        "79228162514264337593543950335",
        "-0.9999999999999999999999999999",
        "-7.922816251426433759354395033",
        "-0.0000000000000000000000000001",
    ];
    let plain = [LeafClass::Numeric, LeafClass::Bool];
    for (index, primitive) in Primitive::ALL.into_iter().enumerate() {
        if plain.contains(&primitive.leaf_class()) {
            let index = u128::try_from(index).unwrap();
            let found = canonical_texts(primitive, index, &candidates);
            assert!(!found.is_empty(), "{primitive:?}: no candidate parsed");
            texts.extend(found);
        }
    }
    assert!(texts.len() > 100, "{}", texts.len());
    for text in texts {
        let bytes = encode(&text).len();
        assert!(bytes <= MAX_ENCODED_TEXT, "{text}: {bytes} bytes");
    }
    assert_eq!(encode(MAX_TEXT).len(), 39);
}

/// Build the plant world and add the `TwinCat3` machine `M44` with the draft of `R-440`.
///
/// The parameter `BIG` has the type `LDT` and the value 1. The test saves its edit form with the
/// type of 32 array levels. `TwinCat3` has no S7 address (I5).
pub(super) fn largest_form_world() -> World {
    let world = world();
    let store = &world.fixture.store;
    store.seed_time_primitive_types();
    let machine = Machine::create(
        MachineId::from(MACHINE_ID),
        MachineCode::parse("M44").unwrap(),
        Text::parse("Big line").unwrap(),
        PlcPlatform::TwinCat3,
        None,
        StringEncoding::Latin1,
        (UnitId::from(UNIT_ID), Text::parse("Infeed").unwrap()),
    )
    .unwrap();
    store.seed_machine(&machine);
    let recipe = Recipe::try_from(StoredRecipe {
        id: RecipeId::from(RECIPE_ID),
        machine: MachineId::from(MACHINE_ID),
        reference: ReferenceNumber::parse(REFERENCE).unwrap(),
        name: Text::parse("Big recipe").unwrap(),
        deleted_at: None,
        versions: vec![StoredVersionHeader {
            id: RecipeVersionId::from(VERSION_ID),
            number: 1,
            state: VersionState::LiveDraft,
            row_version: 1,
        }],
    })
    .unwrap();
    let creator = UserId::from(Uuid::from_u128(1));
    store.seed_recipe(&recipe, PlcPlatform::TwinCat3, creator);
    store.seed_parameter(
        RecipeVersionId::from(VERSION_ID),
        &StoredParameter {
            id: ParameterId::from(KEY),
            key: ParameterKey::from(KEY),
            unit: UnitId::from(UNIT_ID),
            tag: ParameterTag::parse("BIG").unwrap(),
            name_en: Text::parse("Big").unwrap(),
            description_en: LongText::parse("The largest edit form").unwrap(),
            translations: Vec::new(),
            plc_type: seed_type_id(Primitive::Ldt, None).unwrap(),
            s7_address: None,
            limits: Vec::new(),
            value: StoredValue::from(json!(1)),
            ordinal: Ordinal::parse(0).unwrap(),
        },
    );
    world
}

/// Return the form body of the edit form of `BIG` with the new type, the values, and the
/// limits.
fn largest_form_body() -> String {
    let pairs = form_pairs();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    form(&refs)
}

/// Return the path of the route `.../definition` of `BIG`, which saves the edit form.
fn definition_path() -> String {
    format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}/definition")
}

/// Return the world of `largest_form_world` after a save of its largest edit form.
///
/// Solves the problem that the tests of the grid answers need a stored parameter with the
/// nested type of 32 type levels and a limit for each element, which only a save can make.
/// The helper saves `form_pairs` to `/definition` as the ProcessEngineer. It requires `303`,
/// a stored type of 32 array levels over an `LDT` leaf, and `LEAVES` limits with no `[*]`.
/// A limit per element then exists at each array level.
///
/// The second value is the `row_version` of the draft after the commit. The seed has 1 and
/// the save adds 1, so it is 2. A later save sends it as the field `row_version`, because a
/// draft command compares that field with the stored value (ADR-0008, `api.md` section 4).
pub(super) async fn nested_form_world() -> (World, i64) {
    let world = largest_form_world();
    let body = largest_form_body();

    let reply = world
        .post(&world.engineer, &definition_path(), &body, false)
        .await;

    let code_at = reply.body.find("E-").unwrap_or(0);
    let shown: String = reply.body.chars().skip(code_at).take(300).collect();
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{shown}");
    let state = world.fixture.store.state();
    let version = RecipeVersionId::from(VERSION_ID);
    let stored = state.parameters_of(version);
    let parameter = stored.first().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(array_levels(&state, parameter.plc_type), LEVELS);
    assert_eq!(parameter.limits.len(), LEAVES);
    assert!(
        parameter
            .limits
            .iter()
            .all(|limit| !limit.leaf_path.to_string().contains("[*]")),
        "a limit holds a row [*]"
    );
    let row_version = state.version_row_version(version).unwrap();
    (world, row_version)
}

/// Return the number of array rows from `id` down to the first row that is no array.
pub(super) fn array_levels(state: &MemoryState, id: PlcTypeId) -> usize {
    let mut levels = 0_usize;
    let mut current = id;
    while let Some(RecordShape::Array { element, .. }) = state.plc_type(current).map(PlcType::shape)
    {
        levels = levels.checked_add(1).unwrap();
        current = *element;
    }
    levels
}

#[tokio::test]
async fn largest_edit_form_fits_the_value_limit() {
    assert_canonical_texts_fit();
    let world = largest_form_world();
    let body = largest_form_body();
    assert!(body.len() > OLD_LIMIT, "{} bytes", body.len());
    let path = definition_path();

    let reply = world.post(&world.engineer, &path, &body, false).await;

    let code_at = reply.body.find("E-").unwrap_or(0);
    let shown: String = reply.body.chars().skip(code_at).take(300).collect();
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{shown}");
    let limit = WebSettings::default().body_limit_value;
    assert!(body.len() <= limit, "{} bytes, limit {limit}", body.len());
    let stored = world
        .fixture
        .store
        .state()
        .parameters_of(RecipeVersionId::from(VERSION_ID));
    assert_eq!(stored.len(), 1);
    let parameter = stored.first().unwrap();
    assert_eq!(parameter.value.as_json(), &nested(0, &mut 0, WIDE_LEVELS));
    assert_eq!(parameter.limits.len(), LEAVES);
}

/// The largest page that the guard test reads: 256 MiB. The page of the base has about 177 MB.
pub(super) const PAGE_LIMIT: usize = 256 * 1024 * 1024;
/// The count of the mode rows after ADR-0223: one for each of the 32 type levels.
pub(super) const TYPE_LEVEL_MODE_ROWS: usize = 32;
/// Guard of D2: the edit page of `BIG` has 16 384 leaf rows and a mode row for each type level.
///
/// The base `1551311` had 311 295 mode rows, one for each array level with a limit per element.
#[tokio::test]
#[ignore = "builds a page of about 23 MB: run alone with --ignored --test-threads=1"]
async fn largest_mode_rows() {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, 2);
    let path = format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}");
    let headers = [("cookie", world.engineer.cookie.as_str()), ENGLISH];

    let reply = get_bounded(&world.fixture.router(), &path, &headers, PAGE_LIMIT).await;

    assert_eq!(reply.status, StatusCode::OK);
    let leaf_rows = reply.body.matches("<tr><th scope=\"row\"").count();
    let mode_buttons = reply.body.matches("name=\"limit_mode\"").count();
    let mode_rows = reply
        .body
        .matches("<tr><td colspan=\"6\"><span class=\"grid-note\">")
        .count();
    assert_eq!(leaf_rows, LEAVES, "{} bytes", reply.body.len());
    assert_eq!(mode_buttons, TYPE_LEVEL_MODE_ROWS);
    assert_eq!(mode_rows, TYPE_LEVEL_MODE_ROWS);
}
