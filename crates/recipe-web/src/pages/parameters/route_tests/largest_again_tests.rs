//! Measurement of the notice of a new type and of a refused command (task 5.6, design D2 and D6,
//! ADR-0223).
//!
//! The tests start after `nested_form_world`: a limit for each element at each array level and the
//! committed `row_version` 2. They send the edit form of the fixture to `/definition` as the
//! ProcessEngineer in `pl`. The first answer is the notice of a new type: `prepare` shows the form
//! again and no command runs. The second answer is a refused command: the `tag` of a second
//! parameter of the draft gives `RecipeError::DuplicateTag`, and `failed` builds the form with
//! the line. Each test pins the answer of the code after groups 3 and 4, so a pin is a regression
//! check and not a red test. Run the tests alone: `RUSTFLAGS="--cfg heap_measure"
//! CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_again --
//! --ignored --test-threads=1`.

use std::time::{Duration, Instant};

use axum::body::to_bytes;
use axum::http::StatusCode;
use recipe_app::memory::seed_type_id;
use recipe_app::recipe::{RetypeInput, UpdateParameterDefinition, UpdateParameterDefinitionInput};
use recipe_domain::plant::Primitive;
use recipe_domain::recipe::{
    Ordinal, ParameterId, ParameterKey, ParameterTag, RecipeVersionId, StoredParameter,
    StoredValue, ValueInput,
};
use recipe_domain::text::{LongText, Text};
use serde_json::json;
use uuid::Uuid;

use super::largest_form_tests::{
    KEY, LEAVES, TYPE_LEVEL_MODE_ROWS, VERSION_ID, form_pairs, nested_form_world,
};
use super::measure_kit::{
    ANSWER_LIMIT, ENGINEER_SESSION, Pin, Rendered, change_a_sum, check_status, count,
    definition_request, headers_of, loaded_of_post, report,
};
use crate::error::{ErrorContext, answer};
use crate::pages::parameters::definition::check;
use crate::pages::parameters::fields::{FieldSet, read_body};
use crate::pages::parameters::reply::{command_lines, load_shown, refuse, sent_shown, show_form};
use crate::pages::parameters::test_kit::form;
use crate::test_support::call_bounded;
use crate::test_support::heap::{BodyDigest, peak_in, peak_of};
use crate::test_support::plant::{HTMX, POLISH, World};

/// The row version of the draft after `nested_form_world`: the seed has 1 and the save adds 1.
const SAVED_ROW_VERSION: i64 = 2;
/// The mark of the leaf rows of the grid.
const LEAF_ROW: &str = "<tr><th scope=\"row\"";
/// The mark of the mode rows of the grid.
const MODE_ROW: &str = "<tr><td colspan=\"6\"><span class=\"grid-note\">";
/// The base type of the stored type. The notice test sends `LTIME` in its place.
const OLD_BASE: &str = "LDT";
/// The base type of the new type. It keeps the 16 384 leaves and the 32 type levels.
const NEW_BASE: &str = "LTIME";
/// The text of the notice in `pl` (`value-type-notice`).
const NOTICE_PL: &str = "Typ się zmienił. Wpisz wartości nowego typu.";
/// The tag of the second parameter of the draft.
const SECOND_TAG: &str = "SMALL";
/// The key of the second parameter of the draft.
const SECOND_KEY: Uuid = Uuid::from_u128(0x7357_0007_0000_0000_0000_0000_0f44_0002);
/// The code of the line of a refused command.
const DUPLICATE_CODE: &str = "E-RCP-014";
/// The start of the value text of each leaf in the body.
const VALUE_TEXT: &str = "LDT#2262-04-11-23:47:16.854775";

/// The answer that the test sends.
#[derive(Clone, Copy)]
enum Again {
    /// A `type` that differs from `value_type`: the notice of a new type.
    Notice,
    /// The `tag` of the second parameter: the command refuses.
    Refused,
}

/// Return the type text of the notice: the stored type with `NEW_BASE` as base.
fn new_type(old: &str) -> String {
    let outer = old.strip_suffix(OLD_BASE).unwrap();
    format!("{outer}{NEW_BASE}")
}

/// Return the body of the edit form of the fixture with the committed `row_version`.
///
/// It differs from `form_pairs` in `row_version` and in one more field. The notice sends `type`
/// with `NEW_BASE` and keeps `value_type`. The refused command sends `tag` `SMALL`.
fn again_body(again: Again) -> String {
    let pairs: Vec<(String, String)> = form_pairs()
        .into_iter()
        .map(|(name, text)| {
            if name == "row_version" {
                (name, SAVED_ROW_VERSION.to_string())
            } else if name == "type" && matches!(again, Again::Notice) {
                let new = new_type(&text);
                (name, new)
            } else if name == "tag" && matches!(again, Again::Refused) {
                (name, SECOND_TAG.to_owned())
            } else {
                (name, text)
            }
        })
        .collect();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(name, text)| (name.as_str(), text.as_str()))
        .collect();
    let body = form(&refs);
    let mut digest = BodyDigest::new("");
    digest.push(body.as_bytes());
    let (len, sha256) = digest.finish();
    report(&format!("again body: len {len} sha256 {sha256}"));
    body
}

/// Add the second, small parameter to the draft of `world`.
///
/// The parameter has the type `LDT`, the value 1, and no limit. It stands in the unit of `BIG`.
/// The seed does not change the `row_version` of the draft.
fn seed_second_parameter(world: &World) {
    let store = &world.fixture.store;
    let version = RecipeVersionId::from(VERSION_ID);
    let unit = store.state().parameters_of(version).first().unwrap().unit;
    store.seed_parameter(
        version,
        &StoredParameter {
            id: ParameterId::from(SECOND_KEY),
            key: ParameterKey::from(SECOND_KEY),
            unit,
            tag: ParameterTag::parse(SECOND_TAG).unwrap(),
            name_en: Text::parse("Small").unwrap(),
            description_en: LongText::parse("The second parameter").unwrap(),
            translations: Vec::new(),
            plc_type: seed_type_id(Primitive::Ldt, None).unwrap(),
            s7_address: None,
            limits: Vec::new(),
            value: StoredValue::from(json!(1)),
            ordinal: Ordinal::parse(1).unwrap(),
        },
    );
    assert_eq!(store.state().parameters_of(version).len(), 2);
}

/// Assert the answer: the status, the notice or the line, the leaf rows, and the mode rows.
///
/// Both answers show the grid of 16 384 leaf rows with 32 mode rows. The notice answer names the
/// new type in `value_type`. The refused answer has the code and one line under `tag`.
async fn assert_again(world: &World, headers: &[(&str, &str)], body: &str, again: Again) {
    let request = definition_request(headers, &world.engineer.csrf, body);
    let reply = call_bounded(&world.fixture.router(), request, ANSWER_LIMIT).await;
    let page = &reply.body;
    let shown = |needle: &str| count(page, needle);
    let new_value_type = format!(
        "name=\"value_type\" value=\"{}\"",
        new_type(&stored_type_text())
    );
    report(&format!(
        "again answer: status {} len {} leaf rows {} mode buttons {} mode rows {} oob {} \
         notice {} code {DUPLICATE_CODE} {} tag line {} new value_type {} sent value texts {} \
         form rows {}",
        reply.status,
        page.len(),
        shown(LEAF_ROW),
        shown("name=\"limit_mode\""),
        shown(MODE_ROW),
        shown("hx-swap-oob=\"true\""),
        shown(NOTICE_PL),
        shown(DUPLICATE_CODE),
        shown("id=\"tag-error\""),
        shown(&new_value_type),
        shown(VALUE_TEXT),
        shown("<form"),
    ));
    assert_eq!(shown(LEAF_ROW), LEAVES);
    assert_eq!(shown("name=\"limit_mode\""), TYPE_LEVEL_MODE_ROWS);
    assert_eq!(shown(MODE_ROW), TYPE_LEVEL_MODE_ROWS);
    match again {
        Again::Notice => {
            assert_eq!(reply.status, StatusCode::OK);
            assert_eq!(shown(NOTICE_PL), 1);
            assert_eq!(shown(&new_value_type), 1);
            assert_eq!(shown(DUPLICATE_CODE), 0);
            assert_eq!(shown("id=\"tag-error\""), 0);
        }
        Again::Refused => {
            assert_eq!(reply.status, StatusCode::CONFLICT);
            assert!(shown(DUPLICATE_CODE) >= 1);
            assert_eq!(shown("id=\"tag-error\""), 1);
            assert_eq!(shown(NOTICE_PL), 0);
            assert_eq!(shown(&new_value_type), 0);
        }
    }
}

/// Return the stored type text: the `type` field of `form_pairs`.
fn stored_type_text() -> String {
    form_pairs()
        .into_iter()
        .find(|(name, _text)| name == "type")
        .map(|(_name, text)| text)
        .unwrap()
}

/// Measure the sync build of the notice answer on a loaded view.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function reads the body and runs the form query of the new type outside the interval, as
/// `again` of `prepare.rs` does. Inside `peak_in` it repeats the sync tail of `again`:
/// `sent_shown`, the notice, and `show_form`. The caller compares the length and the hash with
/// the answer of the router, so a drift between this copy and `prepare.rs` fails the test.
async fn measure_notice_render(world: &World, headers: &[(&str, &str)], body: &str) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of_post(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    let wanted = check(&sent).type_expr.unwrap();
    assert_ne!(sent.value_type(), Some(&wanted));
    let view = load_shown(ctx, Some(wanted), sent.value_type().cloned())
        .await
        .unwrap();
    assert!(view.line.is_none());

    let started = Instant::now();
    let (response, peak) = peak_in(|| {
        let mut shown = sent_shown(ctx, &view.form, &sent, view.previewed.as_ref());
        shown.notice = ctx.texts().get("value-type-notice");
        show_form(ctx, &view.form, shown)
    });
    let time = started.elapsed();
    let response = response.map_err(|_error| "not shown").unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let bytes = to_bytes(response.into_body(), ANSWER_LIMIT).await.unwrap();
    let (len, sha256) = loaded.digest_of(std::str::from_utf8(&bytes).unwrap());
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Measure the sync build of the answer of a refused command on a loaded view.
///
/// Solves the problem that the handler `save` has no sync step that a test can call alone. The
/// function repeats `save` up to the command outside the interval, and runs the command that
/// refuses. Inside `peak_in` it repeats the sync tail of `failed` in `reply.rs`: the sent grid,
/// the lines of the command, and `refuse`. It then calls `answer` of the layer `render_errors`.
/// The caller compares the length and the hash with the answer of the router, so a drift fails.
async fn measure_refused_render(world: &World, headers: &[(&str, &str)], body: &str) -> Rendered {
    let state = &world.fixture.state;
    let loaded = loaded_of_post(world, ENGINEER_SESSION, headers).await;
    let ctx = loaded.ctx(state);
    let sent = read_body(body.as_bytes(), FieldSet::Parameter).unwrap();
    let checked = check(&sent);
    assert!(checked.lines.is_empty());
    let type_expr = checked.type_expr.clone().unwrap();
    assert_eq!(sent.value_type(), Some(&type_expr));
    let definition = checked.definition.unwrap();
    let view = load_shown(ctx, Some(type_expr), None).await.unwrap();
    assert!(view.line.is_none());
    let tree = view.form.preview.as_ref().unwrap();
    let value = sent.leaf_texts(tree).unwrap();
    let limits = sent.limit_list(None).unwrap();
    let input = UpdateParameterDefinitionInput {
        reference: loaded.target.reference.clone(),
        number: loaded.target.number,
        expected_row_version: definition.row_version,
        key: ParameterKey::from(KEY),
        unit_path: definition.unit_path,
        tag: definition.tag,
        name_en: definition.name_en,
        description_en: definition.description_en,
        type_expr: definition.type_expr,
        s7_address: definition.s7_address,
        ordinal: definition.ordinal,
        retype: Some(RetypeInput {
            value: ValueInput::Texts(value),
            limits,
        }),
    };
    let user = loaded
        .request
        .auth()
        .map_err(|_error| "no session")
        .unwrap()
        .user();
    let error = UpdateParameterDefinition::new(state.identity().app().clone())
        .execute(user, &loaded.request.meta(), input)
        .await
        .unwrap_err();
    let errors = ErrorContext {
        request_id: ctx.request.request_id(),
        lang: ctx.request.lang(),
        htmx: ctx.request.is_htmx(),
        retry: None,
        state,
    };

    let started = Instant::now();
    let (response, peak) = peak_in(|| {
        let refused = {
            let shown = sent_shown(ctx, &view.form, &sent, view.previewed.as_ref());
            let lines = {
                let grid = shown.grid.as_ref();
                command_lines(&error, &|fault| grid.and_then(|grid| grid.input_of(fault)))
            };
            refuse(ctx, &view.form, shown, lines.unwrap())
        };
        answer(&refused, &errors)
    });
    let time = started.elapsed();
    drop(error);

    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = to_bytes(response.into_body(), ANSWER_LIMIT).await.unwrap();
    let (len, sha256) = loaded.digest_of(std::str::from_utf8(&bytes).unwrap());
    Rendered {
        len,
        sha256,
        peak,
        time,
    }
}

/// Send the answer of `again` to `BIG` and check it against `pin`.
async fn measure_again(pin: &Pin, again: Again, extra: &[(&str, &str)]) {
    let (world, row_version) = nested_form_world().await;
    assert_eq!(row_version, SAVED_ROW_VERSION);
    if matches!(again, Again::Refused) {
        seed_second_parameter(&world);
    }
    let body = again_body(again);
    let headers = headers_of(&world.engineer.cookie, POLISH, extra);
    assert_again(&world, &headers, &body, again).await;
    let before = world.fixture.snapshot();
    let version = RecipeVersionId::from(VERSION_ID);
    let stored_before = world.fixture.store.state().version_row_version(version);
    assert_eq!(stored_before, Some(SAVED_ROW_VERSION));

    let started = Instant::now();
    let measured = peak_of(&world.fixture.router(), || {
        definition_request(&headers, &world.engineer.csrf, &body)
    })
    .await;
    let request_time: Duration = started.elapsed();
    assert_eq!(world.fixture.snapshot(), before);
    assert_eq!(
        world.fixture.store.state().version_row_version(version),
        stored_before
    );
    let (rendered, status) = match again {
        Again::Notice => (
            measure_notice_render(&world, &headers, &body).await,
            StatusCode::OK,
        ),
        Again::Refused => (
            measure_refused_render(&world, &headers, &body).await,
            StatusCode::CONFLICT,
        ),
    };

    report(&format!(
        "{}: request peak {} | render peak {} | allowance {} | render time {} ms | \
         sum of change A {}",
        pin.name,
        measured.peak,
        rendered.peak,
        measured.peak.saturating_sub(rendered.peak),
        rendered.time.as_millis(),
        change_a_sum(rendered.peak),
    ));
    check_status(pin, status, &measured, &rendered, request_time);
}

/// The notice of a new type, page without `HX-Request`.
const NOTICE_PAGE_PL: Pin = Pin {
    name: "largest_again_notice_page_pl",
    len: 22_453_500,
    sha256: "229d28e748294ef94505dab2fa4fb01d17759db6a5406ae1116e0416c806b53f",
    request_peak: 218_483_358,
    render_peak: 106_203_308,
};

/// The notice of a new type, HTMX answer.
const NOTICE_HTMX_PL: Pin = Pin {
    name: "largest_again_notice_htmx_pl",
    len: 22_451_039,
    sha256: "bd55cf17389113f1200fa507bc6cb67439fba38701b21f9a0dc67e45307f6a84",
    request_peak: 218_483_358,
    render_peak: 106_203_308,
};

/// The refused command, page without `HX-Request`.
const REFUSED_PAGE_PL: Pin = Pin {
    name: "largest_again_refused_page_pl",
    len: 22_519_615,
    sha256: "38fd94dbd42025ec30ba5ba1a60f7cec6cfe14b614c9fc14ba71d8c620fa334f",
    request_peak: 243_761_195,
    render_peak: 106_203_310,
};

/// The refused command, HTMX answer.
const REFUSED_HTMX_PL: Pin = Pin {
    name: "largest_again_refused_htmx_pl",
    len: 22_517_431,
    sha256: "0434661750169f37bf7969d1f7d1c39f5fa494f83862a9c3231b5ead6630948e",
    request_peak: 243_763_059,
    render_peak: 106_203_310,
};

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_again_notice_page_pl() {
    measure_again(&NOTICE_PAGE_PL, Again::Notice, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 32 MB: run alone with --ignored"]
async fn largest_again_notice_htmx_pl() {
    measure_again(&NOTICE_HTMX_PL, Again::Notice, &[HTMX]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 33 MB: run alone with --ignored"]
async fn largest_again_refused_page_pl() {
    measure_again(&REFUSED_PAGE_PL, Again::Refused, &[]).await;
}

#[tokio::test]
#[ignore = "builds a body of 12 MB and an answer of about 33 MB: run alone with --ignored"]
async fn largest_again_refused_htmx_pl() {
    measure_again(&REFUSED_HTMX_PL, Again::Refused, &[HTMX]).await;
}
