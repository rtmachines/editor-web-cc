//! Tests of the answer of `SetParameterValue` and of the memory of its body (task 4.7, design D3,
//! ADR-0223).

use axum::http::StatusCode;
use axum::response::Html;
use recipe_domain::audit::RequestId;
use uuid::Uuid;

use recipe_domain::plant::{LeafPath, Primitive};

use super::big_grid_kit::MIN_BYTES;
use super::grid::{GridSource, GridValuesFragment, GridView, LimitTexts, ValueTexts};
use super::remove::toast_html;
use super::test_kit::{parameter_path, post_htmx};
use super::tree_kit::array_tree;
use super::value::values_body;
use crate::i18n::Lang;
use crate::pages::Texts;
use crate::pages::recipes::test_world::recipe_world;
use crate::test_support::heap::{BodyDigest, peak_in};
use crate::test_support::recipes::SPEED_KEY;
use crate::test_support::state;

/// The SHA-256 of the HTMX answer of `SetParameterValue` for `SPEED`, as hex.
///
/// The bytes are those of the code before task 4.7. The hash input replaces each copy of the
/// request id. Task 4.7 changes how the body joins its parts and must keep this value.
const VALUES_ANSWER_SHA256: &str =
    "4bbd002bad09f09181efab45d1121384234552766ddab35737c7dd359af3f070";

#[tokio::test]
async fn values_answer_pin_of_the_htmx_success_answer() {
    let world = recipe_world();
    let path = parameter_path(SPEED_KEY, "/value");
    let reply = post_htmx(&world, &path, &[("row_version", "1"), ("value", "1500")]).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
    assert!(
        reply.body.contains("<td class=\"data\">1500</td>"),
        "{}",
        reply.body
    );
    assert!(reply.body.contains("id=\"toast\""), "{}", reply.body);
    let mut digest = BodyDigest::new(&reply.header("x-request-id"));
    digest.push(reply.body.as_bytes());
    let (_len, sha256) = digest.finish();
    assert_eq!(sha256, VALUES_ANSWER_SHA256, "{}", reply.body);
}

/// The count of elements of the grid. The read-only grid is smaller than the form, so 1 MiB of
/// answer needs more elements than `big_grid_kit` has.
const LEAVES: i32 = 7_000;
/// The size of a group of the grid, as `form.array_chunk` of the tests.
const CHUNK: usize = 100;

fn element(index: i32) -> LeafPath {
    LeafPath::parse(&format!("[{index}]")).unwrap()
}

/// Build the grid `ARRAY[1..7000] OF INT` with a value and a limit on each element.
fn values_grid(t: Texts<'_>) -> GridView {
    let tree = array_tree(Primitive::Int, &[(1, LEAVES)]);
    let values: Vec<(LeafPath, String)> = (1..=LEAVES)
        .map(|index| (element(index), index.to_string()))
        .collect();
    let limits: Vec<LimitTexts> = (1..=LEAVES)
        .map(|index| LimitTexts {
            path: element(index),
            min: "0".to_owned(),
            max: "32767".to_owned(),
            max_length: String::new(),
        })
        .collect();
    let source = GridSource {
        tree: &tree,
        values: ValueTexts::from_sent(&values),
        limits,
        toggle: None,
        chunk: CHUNK,
        root_label: "BIG".to_owned(),
    };
    GridView::build(source, t)
}

/// The slack that the peak may add to the length of the body.
const SLACK: usize = 64 * 1024;
/// The sentence of the toast of the tests.
const SENTENCE: &str = "Value of BIG is saved.";

fn request_id() -> RequestId {
    RequestId::from(Uuid::nil())
}

#[test]
fn values_answer_body_of_a_grid_of_one_mebibyte_has_a_capacity_equal_to_its_length() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let grid = values_grid(t);
    let toast = toast_html(SENTENCE.to_owned(), true, request_id()).unwrap();

    let Html(body) =
        values_body(&GridValuesFragment { t, view: &grid }, &toast, request_id()).unwrap();

    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    assert!(body.ends_with(&toast), "{}", &body[body.len() - 400..]);
    assert_eq!(
        body.capacity(),
        body.len(),
        "capacity {} differs from length {}",
        body.capacity(),
        body.len()
    );
}

#[test]
#[ignore = "measurement test: run alone with --ignored --test-threads=1"]
fn values_body_peak_stays_within_the_body_for_a_grid_of_one_mebibyte() {
    let state = state();
    let t = Texts::new(state.bundles(), Lang::En);
    let grid = values_grid(t);
    let toast = toast_html(SENTENCE.to_owned(), true, request_id()).unwrap();
    let fragment = GridValuesFragment { t, view: &grid };

    let (body, peak) = peak_in(|| values_body(&fragment, &toast, request_id()).unwrap());

    let Html(body) = body;
    assert!(body.len() >= MIN_BYTES, "body length {}", body.len());
    let bound = body.len() + SLACK;
    assert!(
        peak <= bound,
        "peak {peak} exceeds bound {bound} (body length {} + {SLACK}, capacity {})",
        body.len(),
        body.capacity()
    );
}
