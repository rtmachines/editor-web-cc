//! The matrix rows and the sweep pages of the parameter pages (design D12).
//!
//! `routes::tests` and `pages::sweep_tests` collect both lists. The agents of wave 5 add the rows
//! of the parameter routes here. The placeholder `{key}` names the parameter `LINE` of `R-100`.

use axum::http::Method;

use crate::routes::matrix::{Endpoint, Want, endpoint};

/// The body of a valid `AddParameter` on the draft 3 of `R-100`. The draft has the `row_version` 1.
const ADD_BODY: &str = "row_version=1&unit_path=U1&tag=MATRIX_P&name_en=Matrix&description_en=Matrix+row&type=INT&value_type=INT&s7_address=DB9.DBW0&ordinal=0&value=1";
/// The body of `SetParameterValue` for `LINE` (`ST_Line@1`): four zones with the value 0 in each leaf.
const VALUE_BODY: &str = "row_version=1&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0";
/// The body of a valid save of the edit form of `LINE` without a change.
///
/// It sends the value fields and the limit rows as a pair (ADR-0213 (i)). The limit rows are blank.
/// The rows have the indexes 1 and 2, so the XSS test can add the row 0 without a duplicate key.
const DEFINITION_BODY: &str = "row_version=1&unit_path=U1&tag=LINE&name_en=line&description_en=The+zones+of+the+press+line&type=ST_Line%401&value_type=ST_Line%401&s7_address=DB1.DBB0&ordinal=0&value.zones%5B1%5D.speed=0&value.zones%5B1%5D.temp_max=0&value.zones%5B2%5D.speed=0&value.zones%5B2%5D.temp_max=0&value.zones%5B3%5D.speed=0&value.zones%5B3%5D.temp_max=0&value.zones%5B4%5D.speed=0&value.zones%5B4%5D.temp_max=0&limits%5B1%5D.leaf_path=.zones%5B*%5D.speed&limits%5B2%5D.leaf_path=.zones%5B*%5D.temp_max";
/// The path of the page of `LINE`. A save of the edit form without HTMX goes to it.
const LINE_PAGE: &str = "/recipes/R-100/versions/3/parameters/73570007-0000-0000-0000-000000000001";

/// Return the matrix rows of the parameter routes.
///
/// A row holds an endpoint and the answer for each of the six principals: no session, the
/// Administrator, the ProcessEngineer, the Operator, the user who must change the password, and the
/// deactivated user (`api.md` section 3.3).
pub(crate) fn route_rows() -> Vec<(Endpoint, [Want; 6])> {
    let mut rows = new_rows();
    rows.extend(parameter_rows());
    rows.extend(table_and_translation_rows());
    rows
}

/// The answers of a route that every role reads.
const VIEW: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Page,
    Want::Password,
    Want::Login,
];
/// The answers of a route that needs `ParameterEdit`.
const EDIT: [Want; 6] = [
    Want::Login,
    Want::Page,
    Want::Page,
    Want::Forbidden,
    Want::Password,
    Want::Login,
];

/// Return the rows of the form of a new parameter and of `AddParameter`.
fn new_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let added = Redirect("/recipes/R-100/versions/3");
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/new",
                "",
            ),
            EDIT,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters",
                ADD_BODY,
            ),
            [Login, added, added, Forbidden, Password, Login],
        ),
    ]
}

/// Return the rows of the page, the edit form, the value, and the limits of `LINE`.
fn parameter_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let (view, edit) = (VIEW, EDIT);
    let saved = Redirect(LINE_PAGE);
    vec![
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/definition",
                DEFINITION_BODY,
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/value",
                VALUE_BODY,
            ),
            edit,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "",
            ),
            view,
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
                "row_version=1",
            ),
            edit,
        ),
    ]
}

/// Return the rows of the table, the translations, and the removal of `LINE`.
///
/// The translation `pl` of `LINE` exists, so its removal succeeds. A command without HTMX goes to
/// the parameter page or, after the removal, to the version page.
fn table_and_translation_rows() -> Vec<(Endpoint, [Want; 6])> {
    use Want::{Forbidden, Login, Password, Redirect};
    let saved = Redirect(LINE_PAGE);
    let removed = Redirect("/recipes/R-100/versions/3");
    let translation = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";
    vec![
        (
            endpoint(Method::GET, "/recipes/{ref}/versions/{n}/parameters", ""),
            VIEW,
        ),
        (
            endpoint(
                Method::GET,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
                "",
            ),
            VIEW,
        ),
        (
            endpoint(
                Method::POST,
                translation,
                "row_version=1&name=Linia&description=",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete",
                "row_version=1",
            ),
            [Login, saved, saved, Forbidden, Password, Login],
        ),
        (
            endpoint(
                Method::POST,
                "/recipes/{ref}/versions/{n}/parameters/{key}/delete",
                "row_version=1",
            ),
            [Login, removed, removed, Forbidden, Password, Login],
        ),
    ]
}

/// Return the pattern and the request path of each full `GET` page of the parameter pages.
///
/// The sweep requests each page in both languages. Without `HX-Request`, the table answers the
/// version page, and the limit rows and the translation rows answer the parameter page.
pub(crate) fn sweep_pages() -> Vec<(&'static str, String)> {
    vec![
        (
            "/recipes/{ref}/versions/{n}/parameters",
            "/recipes/R-100/versions/3/parameters".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/translations",
            format!("{LINE_PAGE}/translations"),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/new",
            "/recipes/R-100/versions/3/parameters/new".to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}",
            LINE_PAGE.to_owned(),
        ),
        (
            "/recipes/{ref}/versions/{n}/parameters/{key}/limits",
            format!("{LINE_PAGE}/limits"),
        ),
    ]
}

/// Tests of the table of a draft with a parameter whose type does not resolve (ADR-0195 (ad)).
#[cfg(test)]
mod unresolved_type_tests {
    use axum::http::StatusCode;
    use recipe_domain::plant::PlcTypeId;
    use serde_json::json;
    use uuid::Uuid;

    use crate::i18n::Lang;
    use crate::pages::recipes::test_world::recipe_world;
    use crate::test_support::plant::{ENGLISH, HTMX, World, main_of, seed_cycle, struct_id};
    use crate::test_support::recipes::earlier_name_world::{ZONE_KEY, seed_zone_of_type};
    use crate::test_support::text;

    /// The page of the draft 3 of `R-100`. It holds the parameter table.
    const DRAFT_PAGE: &str = "/recipes/R-100/versions/3";

    /// Return the table row that links to the parameter `ZONE`.
    fn zone_row(body: &str) -> String {
        let anchor = format!("/parameters/{ZONE_KEY}");
        let (head, tail) = body.split_once(&anchor).unwrap();
        let start = head.rfind("<tr>").unwrap();
        let end = tail.find("</tr>").unwrap();
        format!(
            "{}{anchor}{}",
            head.get(start..).unwrap(),
            tail.get(..end).unwrap()
        )
    }

    /// The named route of the parameter table. A request with `HX-Request` gets the fragment.
    const TABLE_ROUTE: &str = "/recipes/R-100/versions/3/parameters";

    /// Seed a parameter `ZONE` whose root type is in a struct cycle.
    fn seed_cycle_zone(world: &World) {
        seed_cycle(world);
        seed_zone_of_type(
            &world.fixture,
            world.ids.draft,
            struct_id(1),
            json!({"beta": {}}).into(),
        );
    }

    /// Seed a parameter `ZONE` whose root type has no row.
    fn seed_missing_root_zone(world: &World) {
        let missing = PlcTypeId::from(Uuid::from_u128(0x7357_0e00_0000_0000_0000_0000_0000_0f99));
        seed_zone_of_type(&world.fixture, world.ids.draft, missing, json!(5).into());
    }

    /// Assert the row of a struct cycle: the type name and the composite word.
    fn assert_cycle_row(body: &str) {
        let row = zone_row(body);
        let row = row.as_str();
        let word = text(Lang::En, "parameters-value-composite");
        assert!(row.contains("ST_Alpha@1"), "{row}");
        assert!(
            row.contains(&format!("<td class=\"value-note\">{word}</td>")),
            "{row}"
        );
    }

    /// Assert the row of a missing root row: empty type cell and empty value cell.
    fn assert_missing_root_row(body: &str) {
        let row = zone_row(body);
        let row = row.as_str();
        let word = text(Lang::En, "parameters-value-composite");
        assert!(!row.contains(&word), "{row}");
        assert!(
            row.contains("<td class=\"data\"></td><td class=\"value-note\"></td>"),
            "{row}"
        );
    }

    #[tokio::test]
    async fn the_table_shows_the_composite_word_for_a_parameter_of_a_struct_cycle() {
        let world = recipe_world();
        seed_cycle_zone(&world);

        let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_cycle_row(main_of(&reply.body));
    }

    #[tokio::test]
    async fn the_table_shows_an_empty_type_and_an_empty_value_for_a_missing_root_row() {
        let world = recipe_world();
        seed_missing_root_zone(&world);

        let reply = world.get_en(&world.engineer, DRAFT_PAGE).await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert_missing_root_row(main_of(&reply.body));
    }

    #[tokio::test]
    async fn the_table_fragment_shows_the_composite_word_for_a_parameter_of_a_struct_cycle() {
        let world = recipe_world();
        seed_cycle_zone(&world);

        let reply = world
            .get(&world.engineer, TABLE_ROUTE, &[ENGLISH, HTMX])
            .await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("<main"), "{}", reply.body);
        assert_cycle_row(&reply.body);
    }

    #[tokio::test]
    async fn the_table_fragment_shows_an_empty_type_and_an_empty_value_for_a_missing_root_row() {
        let world = recipe_world();
        seed_missing_root_zone(&world);

        let reply = world
            .get(&world.engineer, TABLE_ROUTE, &[ENGLISH, HTMX])
            .await;

        assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
        assert!(!reply.body.contains("<main"), "{}", reply.body);
        assert_missing_root_row(&reply.body);
    }
}

/// Tests of the largest edit form that the budgets of A26 allow (ADR-0211 (d) to (j)).
#[cfg(test)]
mod largest_form_tests {
    use std::collections::BTreeSet;

    use axum::http::StatusCode;
    use recipe_app::memory::seed_type_id;
    use recipe_domain::identity::UserId;
    use recipe_domain::plant::{
        IecLiteral, LeafClass, Machine, MachineCode, MachineId, PlatformTypes, PlcPlatform,
        PlcType, PlcTypeId, PlcTypeRecord, Primitive, RecordShape, RuleSet, StringEncoding,
        TimeType, TypeTree, UnitId,
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
    use crate::test_support::plant::{World, world};

    /// The number of array levels of the type. It equals `json.max_depth`.
    const LEVELS: usize = 32;
    /// The number of outer levels with two elements. Their product is `budget.leaves_per_parameter`.
    const WIDE_LEVELS: usize = 14;
    /// The number of leaves: 2 to the power of `WIDE_LEVELS`.
    const LEAVES: usize = 1 << WIDE_LEVELS;
    /// The first index of every level. Both indexes of a level have two digits.
    const FIRST_INDEX: usize = 10;
    /// The second index of an outer level.
    const SECOND_INDEX: usize = 11;
    /// The longest canonical `LDT` text. It has 33 bytes and encodes to 39 bytes.
    const MAX_TEXT: &str = "LDT#2262-04-11-23:47:16.854775807";
    /// The lowest `LDT` text with a fraction of nine digits.
    const MIN_TEXT: &str = "LDT#1970-01-01-00:00:00.000000001";
    /// The longest encoded text of a leaf of a form (ADR-0211 (f)).
    const MAX_ENCODED_TEXT: usize = 40;
    /// The size of the old limit of the value routes: 4 MiB (ADR-0039 (g)).
    const OLD_LIMIT: usize = 4 * 1024 * 1024;
    /// The reference number of the recipe of the world.
    const REFERENCE: &str = "R-440";
    /// The key of the parameter `BIG`.
    const KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0f44_0001);
    /// The identifier of the draft of the world.
    const VERSION_ID: Uuid = Uuid::from_u128(0x7357_0005_0000_0000_0000_0000_0f44_0001);
    /// The identifier of the machine `M44`.
    const MACHINE_ID: Uuid = Uuid::from_u128(0x7357_0001_0000_0000_0000_0000_0f44_0001);
    /// The identifier of the unit `U1` of `M44`.
    const UNIT_ID: Uuid = Uuid::from_u128(0x7357_0012_0000_0000_0000_0000_0f44_0001);
    /// The identifier of the recipe `R-440`.
    const RECIPE_ID: Uuid = Uuid::from_u128(0x7357_0004_0000_0000_0000_0000_0f44_0001);
    /// The first field of the identifier of a primitive row of the text check.
    const ROW_BASE: u128 = 0x7357_0013_0000_0000_0000_0000_0f44_0000;

    /// Return the indexes of the level `level`: 10 and 11 for an outer level, and 10 for the rest.
    fn indexes(level: usize) -> std::ops::RangeInclusive<usize> {
        let last = if level < WIDE_LEVELS {
            SECOND_INDEX
        } else {
            FIRST_INDEX
        };
        FIRST_INDEX..=last
    }

    /// Return the concrete leaf path of the leaf number `leaf`, for example `[10][11]...[10]`.
    ///
    /// The binary digits of `leaf` pick the index of each outer level. The other levels have one
    /// index, so every path has 32 segments of 4 bytes.
    fn leaf_path(leaf: usize) -> String {
        let bits = format!("{leaf:0WIDE_LEVELS$b}");
        (0..LEVELS)
            .map(|level| {
                let second = bits.as_bytes().get(level) == Some(&b'1');
                format!("[{}]", if second { SECOND_INDEX } else { FIRST_INDEX })
            })
            .collect()
    }

    /// Return the text of the type: 32 `ARRAY` prefixes and the base `LDT`.
    fn type_text() -> String {
        let prefixes: String = (0..LEVELS)
            .map(|level| format!("ARRAY[{}..{}] OF ", FIRST_INDEX, indexes(level).end()))
            .collect();
        format!("{prefixes}LDT")
    }

    /// Return the value text of the leaf number `leaf`: a time with a fraction of nine digits.
    fn value_text(leaf: usize) -> String {
        format!("LDT#2262-04-11-23:47:16.854775{:03}", leaf % 800)
    }

    /// Return the count of an `LDT` text on `TwinCat3`.
    fn ldt_count(text: &str) -> i64 {
        let time_type = TimeType::of(Primitive::Ldt, PlcPlatform::TwinCat3).unwrap();
        i64::try_from(IecLiteral::parse(time_type, text).unwrap().count()).unwrap()
    }

    /// Return the nested JSON that opens the level `level`. `next` is the number of the next leaf.
    fn nested(level: usize, next: &mut usize) -> Value {
        if level == LEVELS {
            let count = ldt_count(&value_text(*next));
            *next = next.checked_add(1).unwrap();
            return json!(count);
        }
        let deeper = level.checked_add(1).unwrap();
        Value::Array(indexes(level).map(|_index| nested(deeper, next)).collect())
    }

    /// Return every pair of the edit form of `BIG` with the new type, the values, and the limits.
    fn form_pairs() -> Vec<(String, String)> {
        let text = type_text();
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
        for leaf in 0..LEAVES {
            let path = leaf_path(leaf);
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
    fn largest_form_world() -> World {
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

    #[tokio::test]
    async fn largest_edit_form_fits_the_value_limit() {
        assert_canonical_texts_fit();
        let world = largest_form_world();
        let pairs = form_pairs();
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_str()))
            .collect();
        let body = form(&refs);
        assert!(body.len() > OLD_LIMIT, "{} bytes", body.len());
        let path = format!("/recipes/{REFERENCE}/versions/1/parameters/{KEY}/definition");

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
        assert_eq!(parameter.value.as_json(), &nested(0, &mut 0));
        assert_eq!(parameter.limits.len(), LEAVES);
    }
}
