//! Tests of the copy of D2 against `budget.limit_rows_per_version` (ADR-0223, design D2).
//!
//! The parameter `WIDE` has the type `ARRAY[1..2] OF ARRAY[1..8192] OF INT`. It stores a limit on
//! `[1][*]` and on each of `[2][1]` to `[2][8192]`, so it holds 8 193 limit rows. The edit form
//! copies the limit of `[1][*]` to the 8 192 elements of the first array level. A save without a
//! change then sends 16 384 limit rows for `WIDE`. The other parameters of the draft add to that.

use axum::http::StatusCode;
use recipe_domain::recipe::{ParameterKey, RecipeVersionId, StoredLimit};

use super::test_kit::form;
use crate::test_support::plant::{ENGLISH, World, world};
use crate::test_support::{Reply, get_bounded, post_fields_bounded};

/// The path of the draft of `R-200`. It is a draft of the `TwinCat3` machine `M20`.
const DRAFT: &str = "/recipes/R-200/versions/1";
/// The largest answer that the tests read. The page of `WIDE` has about 23 MB.
const PAGE_LIMIT: usize = 256 * 1024 * 1024;
/// The elements of the second array level of `WIDE`.
const ELEMENTS: usize = 8192;
/// The limit rows of the parameter `OTHER`: one for each of its leaves.
const OTHER_ROWS: usize = 16_384;
/// The text of the type of `WIDE`.
const WIDE_TYPE: &str = "ARRAY[1..2] OF ARRAY[1..8192] OF INT";
/// The text of the type of `OTHER`.
const OTHER_TYPE: &str = "ARRAY[1..16384] OF INT";
/// The bounds of the limit on `[1][*]` of `WIDE`.
const ALL_BOUNDS: (&str, &str) = ("5", "50");
/// The bounds of every other limit row.
const ELEMENT_BOUNDS: (&str, &str) = ("0", "100");
/// The count of limit rows of the draft that `budget.limit_rows_per_version` allows.
const BUDGET: usize = 32_768;

/// One limit row of a sent form: a leaf path and its bounds.
type Row = (String, (&'static str, &'static str));

/// The pairs of a draft form for a parameter: the fields, the leaf values, and the limit rows.
type Pairs = Vec<(String, String)>;

/// Return the pairs of the add form of the parameter `tag`, without its `row_version`.
///
/// Each leaf of `leaves` gets the value 7. Each row of `rows` gets its bounds.
fn add_pairs(tag: &str, type_text: &str, leaves: &[String], rows: &[Row]) -> Pairs {
    let fixed = [
        ("unit_path", "U1"),
        ("tag", tag),
        ("name_en", tag),
        ("description_en", "A wide array"),
        ("type", type_text),
        ("value_type", type_text),
        ("ordinal", "20"),
    ];
    let mut pairs: Pairs = fixed
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value.to_owned()))
        .collect();
    for leaf in leaves {
        pairs.push((format!("value{leaf}"), "7".to_owned()));
    }
    for (index, (path, (min, max))) in rows.iter().enumerate() {
        pairs.push((format!("limits[{index}].leaf_path"), path.clone()));
        pairs.push((format!("limits[{index}].min"), (*min).to_owned()));
        pairs.push((format!("limits[{index}].max"), (*max).to_owned()));
    }
    pairs
}

/// Send the form `pairs` to `path` as the ProcessEngineer, as a request without HTMX.
async fn send(world: &World, path: &str, pairs: &[(String, String)]) -> Reply {
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect();
    let router = world.fixture.router();
    let body = form(&refs);
    let session = Some(&world.engineer);
    post_fields_bounded(&router, path, session, &[ENGLISH], &body, PAGE_LIMIT).await
}

/// Return the current `row_version` of the draft `version`.
fn row_version_of(world: &World, version: RecipeVersionId) -> i64 {
    world
        .fixture
        .store
        .state()
        .version_row_version(version)
        .unwrap()
}

/// Add the parameter `tag` to the draft `version` and require a `303`.
async fn add(world: &World, version: RecipeVersionId, tag: &str, type_text: &str, rows: &[Row]) {
    let leaves = leaves_of(type_text);
    let mut pairs = vec![(
        "row_version".to_owned(),
        row_version_of(world, version).to_string(),
    )];
    pairs.extend(add_pairs(tag, type_text, &leaves, rows));

    let reply = send(world, &format!("{DRAFT}/parameters"), &pairs).await;

    let shown: String = reply.body.chars().take(300).collect();
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{tag}: {shown}");
}

/// Return the leaf paths of `WIDE_TYPE` or `OTHER_TYPE`, and no path for a scalar.
fn leaves_of(type_text: &str) -> Vec<String> {
    if type_text == WIDE_TYPE {
        return (1..=2)
            .flat_map(|outer| (1..=ELEMENTS).map(move |inner| format!("[{outer}][{inner}]")))
            .collect();
    }
    if type_text == OTHER_TYPE {
        return (1..=OTHER_ROWS).map(|index| format!("[{index}]")).collect();
    }
    vec![String::new()]
}

/// Return the limit rows of `OTHER`: a limit on each element.
fn other_rows() -> Vec<Row> {
    leaves_of(OTHER_TYPE)
        .into_iter()
        .map(|path| (path, ELEMENT_BOUNDS))
        .collect()
}

/// Return the limit rows of `WIDE`: a limit on `[1][*]` and on each element of `[2]`.
fn wide_rows() -> Vec<Row> {
    let mut rows = vec![("[1][*]".to_owned(), ALL_BOUNDS)];
    rows.extend((1..=ELEMENTS).map(|index| (format!("[2][{index}]"), ELEMENT_BOUNDS)));
    rows
}

/// Return the draft `R-200` with `OTHER`, `WIDE`, and `extra` more parameters of one limit row.
///
/// The draft holds `OTHER_ROWS` and 8 193 limit rows, and one more row for each extra parameter.
async fn wide_world(extra: usize) -> (World, RecipeVersionId, ParameterKey) {
    let world = world();
    let version = world.fixture.seed_recipes().r200_draft;
    add(&world, version, "OTHER", OTHER_TYPE, &other_rows()).await;
    add(&world, version, "WIDE", WIDE_TYPE, &wide_rows()).await;
    for index in 0..extra {
        let rows = [(String::new(), ELEMENT_BOUNDS)];
        add(&world, version, &format!("ONE_{index}"), "INT", &rows).await;
    }
    let stored = world.fixture.store.state().parameters_of(version);
    let wide = stored
        .iter()
        .find(|row| row.tag.as_str() == "WIDE")
        .unwrap();
    let key = wide.key;
    (world, version, key)
}

/// What a refused save must leave as it was: the limits, the audit entries, and the `row_version`.
#[derive(PartialEq)]
struct Written {
    /// The limits of each parameter of the draft, in the order of the keys.
    limits: Vec<(ParameterKey, Vec<StoredLimit>)>,
    /// The count of the committed audit entries.
    audit: usize,
    /// The `row_version` of the draft.
    row_version: i64,
}

/// Read what a save may write from the store of `world`.
fn written(world: &World, version: RecipeVersionId) -> Written {
    let mut limits: Vec<(ParameterKey, Vec<StoredLimit>)> = world
        .fixture
        .store
        .state()
        .parameters_of(version)
        .into_iter()
        .map(|row| (row.key, row.limits))
        .collect();
    limits.sort_by_key(|(key, _rows)| *key);
    Written {
        limits,
        audit: world.audit_count(),
        row_version: row_version_of(world, version),
    }
}

/// Return the value of the attribute `name` of the tag `tag`, or `None` when it has none.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let (_before, rest) = tag.split_once(&format!(" {name}=\""))?;
    rest.split('"').next()
}

/// Return the action and the pairs of the edit form in `page`: its inputs, a unit, and a text.
///
/// The browser sends the same fields. The unit is a `select` and the description is a `textarea`,
/// so the helper adds them with the values of `WIDE`.
fn edit_form_of(page: &str) -> (String, Pairs) {
    let (_before, rest) = page.split_once("<form id=\"parameter-edit-form\"").unwrap();
    let (html, _after) = rest.split_once("</form>").unwrap();
    let action = attribute(html, "action").unwrap().to_owned();
    let mut pairs: Pairs = Vec::new();
    for tag in html.split("<input ").skip(1) {
        let tag = tag.split('>').next().unwrap();
        let (Some(name), Some(value)) = (attribute(tag, "name"), attribute(tag, "value")) else {
            continue;
        };
        assert!(!value.contains('&'), "{name}: {value}");
        pairs.push((name.to_owned(), value.to_owned()));
    }
    pairs.push(("unit_path".to_owned(), "U1".to_owned()));
    pairs.push(("description_en".to_owned(), "A wide array".to_owned()));
    (action, pairs)
}

/// Read the edit page of `WIDE` and return the pairs that an unchanged save sends.
async fn unchanged_save(world: &World, key: ParameterKey) -> (String, Pairs) {
    let path = format!("{DRAFT}/parameters/{key}");
    let cookie = world.engineer.cookie.as_str();
    let router = world.fixture.router();

    let page = get_bounded(&router, &path, &[("cookie", cookie), ENGLISH], PAGE_LIMIT).await;

    assert_eq!(page.status, StatusCode::OK);
    let (action, pairs) = edit_form_of(&page.body);
    let paths: Vec<&str> = pairs
        .iter()
        .filter(|(name, _value)| name.ends_with(".leaf_path"))
        .map(|(_name, value)| value.as_str())
        .collect();
    assert_eq!(paths.len(), 2 * ELEMENTS, "the form holds the copied rows");
    assert!(paths.iter().all(|path| !path.contains("[*]")));
    (action, pairs)
}

#[tokio::test]
async fn mixed_budget_save_at_the_limit_row_budget_stores_the_copied_limits() {
    let (world, version, key) = wide_world(0).await;
    let before = written(&world, version);
    let rows: usize = before.limits.iter().map(|(_key, rows)| rows.len()).sum();
    assert_eq!(rows, OTHER_ROWS + ELEMENTS + 1);
    let (action, pairs) = unchanged_save(&world, key).await;

    let reply = send(&world, &action, &pairs).await;

    let shown: String = reply.body.chars().take(300).collect();
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{shown}");
    let after = written(&world, version);
    let total: usize = after.limits.iter().map(|(_key, rows)| rows.len()).sum();
    assert_eq!(total, BUDGET);
    let (_key, wide) = after
        .limits
        .iter()
        .find(|(row, _rows)| *row == key)
        .unwrap();
    assert_eq!(wide.len(), 2 * ELEMENTS);
    for limit in wide {
        let path = limit.leaf_path.to_string();
        let (min, max) = if path.starts_with("[1]") {
            ALL_BOUNDS
        } else {
            ELEMENT_BOUNDS
        };
        assert!(!path.contains("[*]"), "{path}");
        let bounds = (
            limit.min.map(|bound| bound.to_string()),
            limit.max.map(|bound| bound.to_string()),
        );
        assert_eq!(
            bounds,
            (Some(min.to_owned()), Some(max.to_owned())),
            "{path}"
        );
    }
    assert_eq!(
        wide.iter()
            .filter(|row| row.leaf_path.to_string().starts_with("[1]"))
            .count(),
        ELEMENTS
    );
}

#[tokio::test]
async fn mixed_budget_save_above_the_limit_row_budget_gives_e_rcp_021_and_writes_nothing() {
    let (world, version, key) = wide_world(1).await;
    let before = written(&world, version);
    let (action, pairs) = unchanged_save(&world, key).await;

    let reply = send(&world, &action, &pairs).await;

    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(reply.body.contains("E-RCP-021"));
    let line = "The version exceeds the limit of 32768 limit rows. Current count: 32769.";
    assert!(
        reply.body.contains(line),
        "{:?}",
        reply.body.find("E-RCP-021")
    );
    assert!(written(&world, version) == before, "the refused save wrote");
}
