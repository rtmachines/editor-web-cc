//! The XSS corpus through each field, field name, and path value of the recipe pages (task 14.1,
//! design D14).
//!
//! Each field and path value gets every payload of `tests/web/xss-corpus.txt` and one NUL payload.
//! A value that the server echoes shows as escaped text. A value that the server refuses gives its
//! error code. No answer holds the raw payload, no `hx-*` attribute holds it, and each `<svg` of an
//! answer is the fixed tag of its icon. The routes come from `route_rows()` and `sweep_pages()`.

use axum::http::StatusCode;

use super::recipe_sweep_tests::{attribute_values, post_endpoints, sweep_pages};
use super::recipes::test_world::recipe_world;
use super::xss_tests::with_query;
use crate::components_tests::assert_fixed_svg_tags;
use crate::pages::test_helpers::{
    askama_escape, assert_clean_html, assert_no_user_text_in_hx_attributes, encode, field_value,
    payloads,
};
use crate::routes::matrix::{Endpoint, fill};
use crate::test_support::plant::{HTMX, World};
use crate::test_support::{Reply, post_pre_session};

/// The most `<script>` tags that an answer may hold. They are the three files of `base.html`.
const MAX_SCRIPT_TAGS: usize = 3;
/// The path of the draft 3 of `R-100`.
const DRAFT: &str = "/recipes/R-100/versions/3";
/// The query values that no payload can satisfy: a template with `{}` and the code of the answer.
const REFUSED_QUERIES: [(&str, &str); 15] = [
    ("/recipes?machine={}", "E-WEB-005"),
    ("/recipes?status={}", "E-WEB-008"),
    ("/recipes?page={}", "E-WEB-008"),
    ("/recipes?show_deleted={}", "E-WEB-008"),
    ("/recipes?csrf_token={}", "E-WEB-008"),
    ("/recipes/{ref}?page={}", "E-WEB-008"),
    ("/recipes/{ref}/versions?page={}", "E-WEB-008"),
    ("/recipes/{ref}/versions/{n}?page={}", "E-WEB-008"),
    ("/recipes/{ref}/versions/{n}?unit={}", "E-PLT-017"),
    (
        "/recipes/{ref}/versions/{n}/parameters?page={}",
        "E-WEB-008",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters?unit={}",
        "E-PLT-017",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/new?unit={}",
        "E-PLT-017",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}?x={}",
        "E-WEB-008",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/limits?x={}",
        "E-WEB-008",
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/translations?x={}",
        "E-WEB-008",
    ),
];
/// The path values and the code of the answer for a payload in each of them.
const HOLES: [(&str, &str); 4] = [
    ("{ref}", "E-WEB-005"),
    ("{n}", "E-WEB-005"),
    ("{key}", "E-WEB-005"),
    ("{lang}", "E-RCP-027"),
];
/// The forms of a field name. `{}` is the encoded payload. Each pair sends the value 1.
const FIELD_NAMES: [&str; 5] = [
    "{}",
    "value.{}",
    "value[{}]",
    "limits[{}].min",
    "new_member_values[{}].x",
];

/// Return true when `Text::parse` refuses the payload, because it holds a control character.
fn has_control(payload: &str) -> bool {
    payload.chars().any(char::is_control)
}

/// Return true when escaping changes the payload, so the raw payload must not appear in an answer.
fn escapes(payload: &str) -> bool {
    askama_escape(payload) != payload
}

/// Assert that an answer holds no markup of `payload` (design D14).
fn assert_safe(reply: &Reply, payload: &str, what: &str) {
    let body = &reply.body;
    assert_clean_html(body);
    assert_fixed_svg_tags(body);
    assert_no_user_text_in_hx_attributes(body, payload);
    assert!(!body.contains("<img"), "{what}: <img in {body}");
    let scripts = body.matches("<script").count();
    assert!(scripts <= MAX_SCRIPT_TAGS, "{what}: {scripts} scripts");
    if escapes(payload) {
        assert!(!body.contains(payload), "{what}: raw payload in {body}");
    }
}

/// Assert a refused request: a client error status, the error code, and a safe body.
fn assert_refused(reply: &Reply, code: &str, payload: &str, what: &str) {
    assert!(
        reply.status.is_client_error(),
        "{what}: {} {}",
        reply.status,
        reply.body
    );
    assert!(
        reply.body.contains(code),
        "{what}: no {code} in {}",
        reply.body
    );
    assert_safe(reply, payload, what);
}

/// Assert that an HTMX answer sends its error into `#error-panel` and keeps the URL.
fn assert_error_panel(reply: &Reply, what: &str) {
    assert_eq!(reply.header("hx-retarget"), "#error-panel", "{what}");
    assert_eq!(reply.header("hx-push-url"), "false", "{what}");
}

/// Return the tag of the element with the `id`, up to the end of the tag.
fn tag_of<'a>(body: &'a str, id: &str) -> &'a str {
    let marker = format!("id=\"{id}\"");
    let (_before, rest) = body.split_once(&marker).unwrap();
    rest.split_once('>').unwrap().0
}

/// Assert that the input `id` of the form shows the escaped payload as its value.
fn assert_input_value(reply: &Reply, id: &str, payload: &str, what: &str) {
    let value = format!("value=\"{}\"", askama_escape(payload));
    assert!(tag_of(&reply.body, id).contains(&value), "{what} {id}");
}

/// Send the form `body` to `path` as an HTMX request of the Administrator.
async fn post(world: &World, path: &str, body: &str) -> Reply {
    world.post_with(&world.admin, path, body, &[HTMX]).await
}

/// Return the template path with `{}` replaced by the encoded payload, and the placeholders filled.
fn fill_payload(template: &str, payload: &str) -> String {
    fill(&template.replace("{}", &encode(payload)))
}

/// Return the template with the placeholder `hole` replaced by the encoded payload.
fn with_payload(template: &str, hole: &str, payload: &str) -> String {
    fill(&template.replace(hole, &encode(payload)))
}

// --- Path values and query values ------------------------------------------------------------------------

#[tokio::test]
async fn recipe_xss_path_values_of_get_pages_give_their_error_code() {
    let world = recipe_world();
    for payload in payloads() {
        for (pattern, _path) in sweep_pages() {
            for (hole, code) in HOLES.iter().filter(|(hole, _code)| pattern.contains(hole)) {
                let path = with_payload(pattern, hole, &payload);
                let reply = world.get(&world.admin, &path, &[]).await;
                assert_refused(&reply, code, &payload, &format!("GET {path}"));
            }
        }
    }
}

#[tokio::test]
async fn recipe_xss_path_values_of_post_routes_give_their_error_code() {
    let world = recipe_world();
    let mut holes_seen = 0_usize;
    for payload in payloads() {
        for (endpoint, _want) in post_endpoints() {
            for (hole, code) in HOLES
                .iter()
                .filter(|(hole, _code)| endpoint.path.contains(hole))
            {
                let path = with_payload(endpoint.path, hole, &payload);
                let reply = post(&world, &path, &fill(endpoint.body)).await;
                let what = format!("POST {path}");
                assert_refused(&reply, code, &payload, &what);
                assert_error_panel(&reply, &what);
                holes_seen = holes_seen.checked_add(1).unwrap();
            }
        }
    }
    assert!(holes_seen > 100, "{holes_seen}");
}

#[tokio::test]
async fn recipe_xss_query_values_of_get_pages_give_their_error_code() {
    let world = recipe_world();
    for payload in payloads() {
        for (template, code) in REFUSED_QUERIES {
            let path = fill_payload(template, &payload);
            let reply = world.get(&world.admin, &path, &[]).await;
            assert_refused(&reply, code, &payload, &format!("GET {path}"));
        }
    }
}

#[tokio::test]
async fn recipe_xss_search_text_of_the_recipe_list_is_echoed_escaped() {
    let world = recipe_world();
    for payload in payloads() {
        let path = format!("/recipes?q={}", encode(&payload));
        let reply = world.get(&world.admin, &path, &[]).await;
        let what = format!("GET {path}");
        if has_control(&payload) {
            assert_refused(&reply, "E-WEB-008", &payload, &what);
            continue;
        }
        assert_eq!(reply.status, StatusCode::OK, "{what}: {}", reply.body);
        assert_input_value(&reply, "recipe_q", &payload, &what);
        assert_safe(&reply, &payload, &what);
        assert_eq!(field_value(&reply.body, "next"), askama_escape(&path));
    }
}

#[tokio::test]
async fn recipe_xss_search_text_is_encoded_in_the_link_to_the_first_page() {
    // A page after the last page has no row and links to the first page with the kept filters.
    let world = recipe_world();
    for payload in payloads().iter().filter(|payload| !has_control(payload)) {
        let query = encode(payload);
        let path = format!("/recipes?q={query}&page=2");
        let reply = world.get(&world.admin, &path, &[]).await;
        assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.body);
        let link = format!("href=\"/recipes?q={query}&#38;page=1\"");
        assert!(reply.body.contains(&link), "{path}: {}", reply.body);
        assert_safe(&reply, payload, &path);
    }
}

#[tokio::test]
async fn recipe_xss_next_field_of_each_page_holds_the_escaped_uri_or_the_page_refuses_the_key() {
    let world = recipe_world();
    let mut shown = 0_usize;
    for payload in payloads() {
        for (pattern, page) in sweep_pages() {
            let path = with_query(&page, "x", &encode(&payload));
            let reply = world.get(&world.admin, &path, &[]).await;
            let what = format!("GET {pattern} {path}");
            assert!(
                [StatusCode::OK, StatusCode::UNPROCESSABLE_ENTITY].contains(&reply.status),
                "{what}: {}",
                reply.status
            );
            if reply.status == StatusCode::OK {
                assert_safe(&reply, &payload, &what);
                assert_eq!(field_value(&reply.body, "next"), askama_escape(&path));
                shown = shown.checked_add(1).unwrap();
            } else {
                assert_refused(&reply, "E-WEB-008", &payload, &what);
            }
        }
    }
    assert!(shown > 0, "no page shows the field next");
}

#[tokio::test]
async fn recipe_xss_next_field_of_the_language_switch_sends_a_relative_path_back() {
    for payload in payloads() {
        let world = recipe_world();
        for (pattern, page) in sweep_pages() {
            let next = with_query(&page, "x", &encode(&payload));
            let fields = format!("lang=en&next={}", encode(&next));
            let reply = post_pre_session(&world.fixture, "/lang", &fields, &[]).await;
            let what = format!("POST /lang {pattern}");
            assert_eq!(reply.status, StatusCode::SEE_OTHER, "{what}");
            assert_safe(&reply, &payload, &what);
            let location = reply.header("location");
            assert!(
                location == next || location == "/recipes",
                "{what}: {location}"
            );
        }
    }
}

// --- Fields -----------------------------------------------------------------------------------------

/// What the server does with a payload in a field.
#[derive(Clone, Copy, Debug)]
enum Fate {
    /// Every payload gives this code.
    Refused(&'static str),
    /// A payload without a control character is a valid text. A control character gives `E-WEB-008`.
    Text(Outcome),
}

/// The answer to a valid text.
#[derive(Clone, Copy, Debug)]
enum Outcome {
    /// The command succeeds.
    Accepted,
    /// The use case refuses the command with this code.
    Code(&'static str),
}

/// A field of a `POST` route: the route, the key in the body, its fate, and the pairs it needs.
type Case = (&'static str, &'static str, Fate, &'static str);

const RECIPES: &str = "/recipes";
const RENAME: &str = "/recipes/{ref}/rename";
const VERSIONS: &str = "/recipes/{ref}/versions";
const REBIND: &str = "/recipes/{ref}/versions/{n}/rebind";
const ADD: &str = "/recipes/{ref}/versions/{n}/parameters";
const DEFINITION: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/definition";
const VALUE: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/value";
const LIMITS: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/limits";
const TRANSLATION: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}";
const TRANSLATION_DELETE: &str =
    "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}/delete";
const REMOVE: &str = "/recipes/{ref}/versions/{n}/parameters/{key}/delete";
/// The path of the parameter `LINE` of the draft 3 of `R-100`.
const LINE: &str = "/recipes/{ref}/versions/{n}/parameters/{key}";
/// The key of the value of `LINE` that the body of the definition sends.
const LINE_VALUE: &str = "value.zones%5B1%5D.speed";
/// The key of the value of the new leaf of `ZONES` that the body of the rebind sends.
const ZONES_VALUE: &str = "new_member_values[73570007-0000-0000-0000-000000000017][1].temp_min";
/// A limit row on the root of an `INT`, without a bound.
const ROOT_LIMIT: &str = "limits%5B0%5D.leaf_path=";
/// A limit row on the leaf `.zones[1].speed` of `LINE`, without a bound.
const SPEED_LIMIT: &str = "limits%5B0%5D.leaf_path=.zones%5B1%5D.speed";

/// Return the fields of the forms of the recipe and of the version.
fn recipe_cases() -> Vec<Case> {
    let web = Fate::Refused("E-WEB-008");
    let text = Fate::Text(Outcome::Accepted);
    vec![
        (RECIPES, "machine_code", web, ""),
        (RECIPES, "reference_number", web, ""),
        (RECIPES, "name", text, ""),
        (RECIPES, "change_note", text, ""),
        (RENAME, "name", text, ""),
        (VERSIONS, "based_on", web, ""),
        (
            VERSIONS,
            "change_note",
            Fate::Text(Outcome::Code("E-RCP-011")),
            "",
        ),
    ]
}

/// Return the fields of a form that sends the definition of a parameter.
fn definition_cases(
    route: &'static str,
    value: &'static str,
    limit_row: &'static str,
) -> Vec<Case> {
    let web = Fate::Refused("E-WEB-008");
    let text = Fate::Text(Outcome::Accepted);
    let bound = Fate::Refused("E-RCP-006");
    vec![
        (route, "row_version", web, ""),
        (route, "unit_path", Fate::Refused("E-PLT-017"), ""),
        (route, "tag", web, ""),
        (route, "name_en", text, ""),
        (route, "description_en", text, ""),
        (route, "type", Fate::Refused("E-PLT-013"), ""),
        (route, "value_type", web, ""),
        (route, "s7_address", web, ""),
        (route, "ordinal", web, ""),
        (route, "fields", web, ""),
        (route, "limit_mode", web, ""),
        (route, value, Fate::Refused("E-RCP-005"), ""),
        (route, "limits%5B0%5D.leaf_path", web, ""),
        (route, "limits%5B0%5D.min", bound, limit_row),
        (route, "limits%5B0%5D.max", bound, limit_row),
        (route, "limits%5B0%5D.max_length", web, limit_row),
    ]
}

/// Return the fields of the value, limits, translation, removal, and rebind routes.
fn command_cases() -> Vec<Case> {
    let web = Fate::Refused("E-WEB-008");
    let text = Fate::Text(Outcome::Accepted);
    vec![
        (VALUE, "row_version", web, ""),
        (VALUE, LINE_VALUE, Fate::Refused("E-RCP-005"), ""),
        (LIMITS, "row_version", web, ""),
        (LIMITS, "limits%5B0%5D.leaf_path", web, ""),
        (
            LIMITS,
            "limits%5B0%5D.min",
            Fate::Refused("E-RCP-006"),
            SPEED_LIMIT,
        ),
        (
            LIMITS,
            "limits%5B0%5D.max",
            Fate::Refused("E-RCP-006"),
            SPEED_LIMIT,
        ),
        (LIMITS, "limits%5B0%5D.max_length", web, SPEED_LIMIT),
        (TRANSLATION, "row_version", web, ""),
        (TRANSLATION, "name", text, ""),
        (TRANSLATION, "description", text, ""),
        (TRANSLATION_DELETE, "row_version", web, ""),
        (REMOVE, "row_version", web, ""),
        (REBIND, "row_version", web, ""),
        (REBIND, "from_type_id", web, ""),
        (REBIND, "to_type_id", web, ""),
        (REBIND, ZONES_VALUE, Fate::Refused("E-RCP-005"), ""),
    ]
}

/// Return every field of every `POST` route of the three areas.
fn cases() -> Vec<Case> {
    let mut all = recipe_cases();
    all.extend(definition_cases(ADD, "value", ROOT_LIMIT));
    all.extend(definition_cases(DEFINITION, LINE_VALUE, SPEED_LIMIT));
    all.extend(command_cases());
    all
}

/// Return the endpoint of the route `template`.
fn endpoint_of(template: &str) -> Endpoint {
    post_endpoints()
        .into_iter()
        .map(|(endpoint, _want)| endpoint)
        .find(|endpoint| endpoint.path == template)
        .unwrap()
}

/// Return `body` with the value of the pair `key` replaced by the encoded payload.
///
/// A body that lacks the pair gets it at the end, after the pairs of `extra`.
fn with_value(body: &str, key: &str, payload: &str, extra: &str) -> String {
    let mut found = false;
    let mut pairs: Vec<String> = body
        .split('&')
        .map(|pair| match pair.split_once('=') {
            Some((name, _old)) if name == key => {
                found = true;
                format!("{name}={}", encode(payload))
            }
            _other => pair.to_owned(),
        })
        .collect();
    if !found {
        pairs.extend(
            extra
                .split('&')
                .filter(|pair| !pair.is_empty())
                .map(str::to_owned),
        );
        pairs.push(format!("{key}={}", encode(payload)));
    }
    pairs.join("&")
}

/// Send the payload in the field of `case` and return the answer with the form body.
async fn send_case(world: &World, case: &Case, payload: &str) -> Reply {
    let (route, key, _fate, extra) = case;
    let body = with_value(&fill(endpoint_of(route).body), key, payload, extra);
    post(world, &fill(route), &body).await
}

/// Assert the answer of a field that must refuse every payload.
fn assert_fate(reply: &Reply, fate: Fate, payload: &str, what: &str) {
    match fate {
        Fate::Refused(code) => {
            assert_refused(reply, code, payload, what);
            assert_error_panel(reply, what);
        }
        Fate::Text(_) if has_control(payload) => {
            assert_refused(reply, "E-WEB-008", payload, what);
            assert_error_panel(reply, what);
        }
        Fate::Text(Outcome::Code(code)) => {
            assert_refused(reply, code, payload, what);
            assert_error_panel(reply, what);
        }
        Fate::Text(Outcome::Accepted) => {
            let status = reply.status;
            assert!(
                status.is_success() || status.is_redirection(),
                "{what}: {status}"
            );
            assert_safe(reply, payload, what);
        }
    }
}

#[test]
fn recipe_xss_the_field_table_names_only_routes_of_the_endpoint_table() {
    let routes: Vec<&str> = post_endpoints().iter().map(|(e, _want)| e.path).collect();
    for (route, key, _fate, _extra) in cases() {
        assert!(routes.contains(&route), "{route} {key}");
    }
    for route in &routes {
        assert!(
            cases().iter().any(|case| case.0 == *route),
            "no field of {route}"
        );
    }
}

#[tokio::test]
async fn recipe_xss_fields_that_the_server_refuses_give_their_error_code() {
    let shared = recipe_world();
    for payload in payloads() {
        for case in cases()
            .iter()
            .filter(|case| matches!(case.2, Fate::Refused(_)))
        {
            let reply = send_case(&shared, case, &payload).await;
            let what = format!("POST {} {}", case.0, case.1);
            assert_fate(&reply, case.2, &payload, &what);
        }
    }
}

#[tokio::test]
async fn recipe_xss_text_fields_accept_a_valid_text_and_refuse_a_control_character() {
    for payload in payloads() {
        for case in cases()
            .iter()
            .filter(|case| matches!(case.2, Fate::Text(_)))
        {
            let world = recipe_world();
            let reply = send_case(&world, case, &payload).await;
            let what = format!("POST {} {}", case.0, case.1);
            assert_fate(&reply, case.2, &payload, &what);
        }
    }
}

#[tokio::test]
async fn recipe_xss_the_payload_in_a_field_name_gives_one_line_e_web_008() {
    let world = recipe_world();
    for payload in payloads() {
        for (endpoint, _want) in post_endpoints() {
            for name in FIELD_NAMES {
                let pair = format!("{}=1", name.replace("{}", &encode(&payload)));
                let body = format!("{}&{pair}", fill(endpoint.body));
                let reply = post(&world, &fill(endpoint.path), &body).await;
                let what = format!("POST {} {name}", endpoint.path);
                assert_refused(&reply, "E-WEB-008", &payload, &what);
                assert_error_panel(&reply, &what);
            }
        }
    }
}

// --- Texts that the server shows again -------------------------------------------------------------------

#[tokio::test]
async fn recipe_xss_sent_texts_of_a_refused_recipe_form_are_echoed_escaped() {
    // The reference number is bad, so the answer swaps the form out of band with the sent texts.
    for payload in payloads() {
        let world = recipe_world();
        let text = encode(&payload);
        let body =
            format!("machine_code=M7&reference_number={text}&name={text}&change_note={text}");
        let reply = post(&world, "/recipes", &body).await;
        assert_refused(&reply, "E-WEB-008", &payload, "POST /recipes");
        for id in ["reference_number", "recipe_name", "change_note"] {
            assert_input_value(&reply, id, &payload, "POST /recipes");
        }
    }
}

#[tokio::test]
async fn recipe_xss_sent_name_of_a_refused_rename_is_echoed_escaped() {
    // Only a control character makes a name bad, so only the NUL payload reaches the form.
    for payload in payloads().iter().filter(|payload| has_control(payload)) {
        let world = recipe_world();
        let body = format!("name={}", encode(payload));
        let reply = post(&world, &fill(RENAME), &body).await;
        assert_refused(&reply, "E-WEB-008", payload, "POST rename");
        assert_input_value(&reply, "recipe_name", payload, "POST rename");
    }
}

#[tokio::test]
async fn recipe_xss_sent_texts_of_the_version_and_translation_forms_are_echoed_escaped() {
    for payload in payloads() {
        let world = recipe_world();
        let version = format!("based_on=x&change_note={}", encode(&payload));
        let reply = post(&world, &fill(VERSIONS), &version).await;
        assert_refused(&reply, "E-WEB-008", &payload, "POST versions");
        assert_input_value(&reply, "change_note", &payload, "POST versions");
        let text = encode(&payload);
        let body = format!("row_version=x&name={text}&description={text}");
        let reply = post(&world, &fill(TRANSLATION), &body).await;
        assert_refused(&reply, "E-WEB-008", &payload, "POST translation");
        for id in ["translation-pl-name", "translation-pl-description"] {
            assert_input_value(&reply, id, &payload, "POST translation");
        }
    }
}

#[tokio::test]
async fn recipe_xss_sent_texts_of_a_refused_parameter_form_are_echoed_escaped() {
    let fields = [
        "tag",
        "name_en",
        "description_en",
        "type",
        "s7_address",
        "ordinal",
    ];
    for route in [ADD, DEFINITION] {
        for payload in payloads() {
            let world = recipe_world();
            let mut body = fill(endpoint_of(route).body);
            for key in fields {
                body = with_value(&body, key, &payload, "");
            }
            let reply = post(&world, &fill(route), &body).await;
            let what = format!("POST {route} all fields");
            assert_refused(&reply, "E-WEB-008", &payload, &what);
            for id in fields {
                assert_input_value(&reply, id, &payload, &what);
            }
        }
    }
}

// --- Texts that the server stores and shows --------------------------------------------------------------

/// Return the key of the new parameter from the link of its row.
fn key_of_row(row: &str) -> String {
    let href = attribute_values(row, "href")
        .into_iter()
        .find(|href| href.contains("/parameters/"))
        .unwrap();
    href.rsplit('/').next().unwrap().to_owned()
}

#[tokio::test]
async fn recipe_xss_recipe_texts_are_stored_and_shown_escaped() {
    for payload in payloads().iter().filter(|payload| !has_control(payload)) {
        let world = recipe_world();
        let text = encode(payload);
        let body = format!("machine_code=M7&reference_number=R-500&name={text}&change_note={text}");
        let created = post(&world, "/recipes", &body).await;
        assert_eq!(created.status, StatusCode::SEE_OTHER, "{}", created.body);
        let renamed = post(&world, "/recipes/R-100/rename", &format!("name={text}")).await;
        assert_eq!(renamed.status, StatusCode::OK, "{}", renamed.body);
        assert!(renamed.body.contains(&askama_escape(payload)));
        assert_safe(&renamed, payload, "POST rename");
        for page in [
            "/recipes",
            "/recipes/R-500",
            "/recipes/R-100",
            "/recipes/R-500/versions/1",
        ] {
            let shown = world.get(&world.admin, page, &[]).await;
            assert_eq!(shown.status, StatusCode::OK, "{page}");
            assert!(shown.body.contains(&askama_escape(payload)), "{page}");
            assert_safe(&shown, payload, page);
        }
    }
}

#[tokio::test]
async fn recipe_xss_parameter_texts_are_stored_and_shown_escaped() {
    for payload in payloads().iter().filter(|payload| !has_control(payload)) {
        let world = recipe_world();
        let mut body = fill(endpoint_of(ADD).body);
        body = with_value(&body, "name_en", payload, "");
        body = with_value(&body, "description_en", payload, "");
        let added = post(&world, &fill(ADD), &body).await;
        assert_eq!(added.status, StatusCode::OK, "{}", added.body);
        assert!(added.body.contains(&askama_escape(payload)));
        assert_safe(&added, payload, "POST parameters");
        let page = format!("{DRAFT}/parameters/{}", key_of_row(&added.body));
        let shown = world.get(&world.admin, &page, &[]).await;
        assert_eq!(shown.status, StatusCode::OK, "{page}");
        assert!(shown.body.contains(&askama_escape(payload)), "{page}");
        assert_safe(&shown, payload, &page);
    }
}

#[tokio::test]
async fn recipe_xss_translation_texts_are_stored_and_shown_escaped() {
    for payload in payloads().iter().filter(|payload| !has_control(payload)) {
        let world = recipe_world();
        let text = encode(payload);
        let body = format!("row_version=1&name={text}&description={text}");
        let saved = post(&world, &fill(TRANSLATION), &body).await;
        assert_eq!(saved.status, StatusCode::OK, "{}", saved.body);
        assert_input_value(&saved, "translation-pl-name", payload, "POST translation");
        assert_safe(&saved, payload, "POST translation");
        let rows = world
            .get(&world.admin, &fill(&format!("{LINE}/translations")), &[])
            .await;
        assert!(rows.body.contains(&askama_escape(payload)), "{}", rows.body);
        assert_safe(&rows, payload, "GET translations");
    }
}
