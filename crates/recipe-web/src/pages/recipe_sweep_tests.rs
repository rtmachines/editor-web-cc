//! The `hx-*` rule and the CSRF token rule over every answer of the recipe pages (task 14.2, D14).
//!
//! The sweep reads the full `GET` pages of the three areas of design D12, their HTMX fragments, the
//! success answers of the `POST` commands, and the error answers. It runs each one in `pl` and in
//! `en`. The pages come from `sweep_pages()` and the commands from `route_rows()`, so a route that
//! an area adds later joins the sweep.

use std::collections::BTreeSet;

use axum::http::{Method, StatusCode};

use super::page_tests::{LANGS, assert_assets, assert_layout};
use super::test_helpers::{CSP, HxAttribute, assert_clean_html, field_value, hx_attributes};
use super::{parameters, recipes, versions};
use crate::components_tests::assert_fixed_svg_tags;
use crate::csrf::is_mutating;
use crate::routes::matrix::{Endpoint, Principal, Want, fill, world};
use crate::test_support::plant::{HTMX, TARGET_CONTENT};
use crate::test_support::{Reply, TestSession, get_with, post_fields};

/// The value of `hx-headers` of `<body>` is the one value that holds the token (`api.md` section 7.3).
const HX_HEADERS: &str = "hx-headers";
/// The header of an HTMX request that targets a region of the page and not `main`.
const TARGET_REGION: (&str, &str) = ("hx-target", "region");
/// The header of an HTMX request that targets the region of the form of a new parameter.
const TARGET_FORM: (&str, &str) = ("hx-target", "parameter-form");
/// The shapes of a `GET` request: a name and the extra headers.
const GET_SHAPES: [(&str, &[(&str, &str)]); 4] = [
    ("page", &[]),
    ("content", &[HTMX, TARGET_CONTENT]),
    ("region", &[HTMX, TARGET_REGION]),
    ("form", &[HTMX, TARGET_FORM]),
];
/// Pages beyond the last page of a list. They show the link to the first page, with its own `hx-*`.
const AFTER_END_PAGES: [&str; 5] = [
    "/recipes?page=2",
    "/recipes/R-100?page=2",
    "/recipes/R-100/versions?page=2",
    "/recipes/R-100/versions/3?page=2",
    "/recipes/R-100/versions/3/parameters?page=2",
];
/// Every `hx-*` attribute that a recipe answer may hold, with the literals that it may have.
///
/// A name or a value that is not in the list fails the test. The test also fails when a literal of
/// the list appears in no answer, so the list cannot go stale.
const HX_LITERALS: [(&str, &[&str]); 7] = [
    ("hx-boost", &["true"]),
    (
        "hx-target",
        &[
            "#content",
            "#parameter-form",
            "#parameter-rows",
            "#parameter-summary",
            "#parameter-table",
            "#recipe-name",
            "#translations",
            "#versions-table",
            "closest .translation-row",
        ],
    ),
    (
        "hx-swap",
        &[
            "innerHTML",
            "outerHTML",
            "innerHTML show:none focus-scroll:true",
            "beforeend show:none focus-scroll:true",
        ],
    ),
    ("hx-push-url", &["true", "false"]),
    ("hx-sync", &["closest form:drop"]),
    ("hx-swap-oob", &["true", "innerHTML"]),
    ("hx-history", &["false"]),
];
/// Commands that fail: the path template, the body, and the status of the answer.
///
/// Each one gives an error fragment with the out-of-band form or the conflict fragment. The row
/// version 99 of a draft is stale.
const REFUSED: [(&str, &str, StatusCode); 5] = [
    (
        "/recipes",
        "machine_code=M7&reference_number=&name=&change_note=",
        StatusCode::UNPROCESSABLE_ENTITY,
    ),
    (
        "/recipes/{ref}/rename",
        "name=",
        StatusCode::UNPROCESSABLE_ENTITY,
    ),
    (
        "/recipes/{ref}/versions",
        "based_on=x&change_note=",
        StatusCode::UNPROCESSABLE_ENTITY,
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters",
        "row_version=1&tag=",
        StatusCode::UNPROCESSABLE_ENTITY,
    ),
    (
        "/recipes/{ref}/versions/{n}/parameters/{key}/translations/{lang}",
        "row_version=1&name=",
        StatusCode::UNPROCESSABLE_ENTITY,
    ),
];

/// One answer of the sweep with the statuses that it may have.
pub(super) struct Answer {
    pub(super) what: String,
    pub(super) reply: Reply,
    pub(super) statuses: Vec<StatusCode>,
}

/// Return the pattern and the path of each full `GET` page of the three areas.
pub(super) fn sweep_pages() -> Vec<(&'static str, String)> {
    let mut pages = recipes::route_tests::sweep_pages();
    pages.extend(versions::route_tests::sweep_pages());
    pages.extend(parameters::route_tests::sweep_pages());
    pages
}

/// Return each mutating endpoint of the three areas with the answer of the Administrator.
pub(super) fn post_endpoints() -> Vec<(Endpoint, Want)> {
    let mut rows = recipes::route_tests::route_rows();
    rows.extend(versions::route_tests::route_rows());
    rows.extend(parameters::route_tests::route_rows());
    rows.into_iter()
        .filter(|(endpoint, _wants)| is_mutating(&endpoint.method))
        .map(|(endpoint, wants)| (endpoint, wants[1]))
        .collect()
}

/// Return the statuses that an HTMX command may answer when the matrix expects `want`.
///
/// A command without HTMX redirects. An HTMX command answers `200`, or redirects too.
fn statuses_of(want: Want) -> Vec<StatusCode> {
    match want {
        Want::Conflict(_code) => vec![StatusCode::CONFLICT],
        _success => vec![StatusCode::OK, StatusCode::SEE_OTHER],
    }
}

/// Collect the answers of every `GET` page in every shape for the language `code`.
async fn get_answers(code: &str) -> Vec<Answer> {
    let (fixture, session) = world(Principal::Administrator).await;
    let cookie = format!("{}; recipe_lang={code}", session.unwrap().cookie);
    let mut answers = Vec::new();
    let pages = sweep_pages().into_iter().map(|(_pattern, path)| path);
    for path in pages.chain(AFTER_END_PAGES.map(str::to_owned)) {
        for (shape, extra) in GET_SHAPES {
            let mut headers = vec![("cookie", cookie.as_str())];
            headers.extend_from_slice(extra);
            answers.push(Answer {
                what: format!("{code} {shape} GET {path}"),
                reply: get_with(&fixture.router(), &path, &headers).await,
                statuses: vec![StatusCode::OK],
            });
        }
    }
    answers
}

/// Send the command of `endpoint` with the body `body` as an HTMX request of a fresh Administrator.
async fn send_command(endpoint: &Endpoint, body: &str, code: &str) -> Reply {
    let (fixture, session) = world(Principal::Administrator).await;
    let lang = format!("recipe_lang={code}");
    let headers = [("cookie", lang.as_str()), HTMX, TARGET_REGION];
    let router = fixture.router();
    let path = fill(endpoint.path);
    post_fields(&router, &path, session.as_ref(), &headers, body).await
}

/// Collect the success answers of the commands of `route_rows()` for the language `code`.
async fn command_answers(code: &str) -> Vec<Answer> {
    let mut answers = Vec::new();
    for (endpoint, want) in post_endpoints() {
        let reply = send_command(&endpoint, &fill(endpoint.body), code).await;
        answers.push(Answer {
            what: format!("{code} POST {}", endpoint.path),
            reply,
            statuses: statuses_of(want),
        });
    }
    answers
}

/// Collect the error answers: the bad fields of `REFUSED` and a stale `row_version` per draft route.
async fn error_answers(code: &str) -> Vec<Answer> {
    let mut answers = Vec::new();
    for (template, body, status) in REFUSED {
        let endpoint = crate::routes::matrix::endpoint(Method::POST, template, "");
        let reply = send_command(&endpoint, &fill(body), code).await;
        answers.push(Answer {
            what: format!("{code} POST {template} refused"),
            reply,
            statuses: vec![status],
        });
    }
    for (endpoint, _want) in post_endpoints() {
        if !endpoint.body.contains("row_version=1") {
            continue;
        }
        let stale = fill(endpoint.body).replacen("row_version=1", "row_version=99", 1);
        let reply = send_command(&endpoint, &stale, code).await;
        answers.push(Answer {
            what: format!("{code} POST {} stale", endpoint.path),
            reply,
            statuses: vec![StatusCode::CONFLICT],
        });
    }
    answers
}

/// Collect every answer of the sweep in both languages. Each command gets a fresh world.
pub(super) async fn answers() -> Vec<Answer> {
    let mut all = Vec::new();
    for (_lang, code) in LANGS {
        all.extend(get_answers(code).await);
        all.extend(command_answers(code).await);
        all.extend(error_answers(code).await);
    }
    all
}

// --- Reading the markup -------------------------------------------------------------------------------

/// Return the values of the attribute `name` of every tag of `body`, written as ` name="..."`.
pub(super) fn attribute_values<'a>(body: &'a str, name: &str) -> Vec<&'a str> {
    let marker = format!(" {name}=\"");
    body.match_indices(marker.as_str())
        .filter_map(|(at, _)| body.get(at..)?.strip_prefix(marker.as_str()))
        .filter_map(|rest| rest.split('"').next())
        .collect()
}

/// Return the start tag and the markup up to `</form>` of every `<form` in `body`.
fn forms(body: &str) -> Vec<(&str, &str)> {
    body.match_indices("<form")
        .filter_map(|(at, _)| body.get(at..))
        .map(|rest| {
            let tag_end = rest.find('>').unwrap_or(rest.len());
            let block_end = rest.find("</form>").unwrap_or(rest.len());
            (
                rest.get(..tag_end).unwrap_or_default(),
                rest.get(..block_end).unwrap_or_default(),
            )
        })
        .collect()
}

/// Return true when the answer is a full page and not a fragment.
fn is_full(reply: &Reply) -> bool {
    reply.body.contains("<html")
}

// --- The rules ----------------------------------------------------------------------------------------

/// Assert that `attribute` is a literal of the list, or `hx-headers` of `<body>`.
fn assert_literal(attribute: &HxAttribute, what: &str) {
    let HxAttribute {
        name,
        value,
        element,
    } = attribute;
    if name == HX_HEADERS {
        assert_eq!(element, "body", "{what}: hx-headers outside <body>");
        return;
    }
    let known = HX_LITERALS.iter().find(|(known, _values)| known == name);
    assert!(
        known.is_some(),
        "{what}: unknown attribute {name}={value} on <{element}>"
    );
    assert!(
        known.is_some_and(|(_name, values)| values.contains(&value.as_str())),
        "{what}: {name}={value:?} is no literal of the list"
    );
}

/// Assert that none of the URLs of an answer holds the CSRF token or its field name.
fn assert_urls_without_token(answer: &Answer, token: &str) {
    let Answer { what, reply, .. } = answer;
    let mut urls: Vec<String> = ["href", "action"]
        .iter()
        .flat_map(|name| attribute_values(&reply.body, name))
        .map(str::to_owned)
        .collect();
    for header in ["location", "hx-push-url", "hx-redirect", "hx-location"] {
        urls.push(reply.header(header));
    }
    if reply.body.contains("name=\"next\"") {
        urls.push(field_value(&reply.body, "next"));
    }
    for url in urls {
        assert!(!url.contains("csrf_token"), "{what}: {url}");
        assert!(!url.contains(token), "{what}: token in {url}");
    }
}

/// Return the token of the Administrator of a world. The session token is fixed for `user1`.
async fn admin_token() -> String {
    let (_fixture, session) = world(Principal::Administrator).await;
    let session: TestSession = session.unwrap();
    session.csrf
}

#[test]
fn recipe_sweep_the_pages_are_every_get_page_of_the_three_areas() {
    let mut rows = recipes::rows();
    rows.extend(versions::rows());
    rows.extend(parameters::rows());
    let routes: BTreeSet<&str> = rows
        .iter()
        .filter(|row| row.method == Method::GET)
        .map(|row| row.path)
        .collect();
    let listed: BTreeSet<&str> = sweep_pages()
        .iter()
        .map(|(pattern, _path)| *pattern)
        .collect();
    assert_eq!(listed, routes);
}

#[tokio::test]
async fn recipe_sweep_every_page_has_the_layout_in_both_languages() {
    for (lang, code) in LANGS {
        for answer in get_answers(code)
            .await
            .iter()
            .filter(|a| a.what.contains(" page "))
        {
            let Answer { what, reply, .. } = answer;
            assert_eq!(reply.status, StatusCode::OK, "{what}");
            assert_layout(&reply.body, lang, code);
            assert_assets(&reply.body);
        }
    }
}

#[tokio::test]
async fn recipe_sweep_every_answer_has_the_csp_and_clean_markup() {
    for answer in answers().await {
        let Answer {
            what,
            reply,
            statuses,
        } = &answer;
        assert!(statuses.contains(&reply.status), "{what}: {}", reply.status);
        assert_eq!(reply.header("content-security-policy"), CSP, "{what}");
        assert_clean_html(&reply.body);
        assert_fixed_svg_tags(&reply.body);
        assert!(!reply.body.contains("<style"), "{what}");
        if is_full(reply) {
            assert_assets(&reply.body);
        } else {
            assert!(!reply.body.contains("<script"), "{what}: script");
        }
    }
}

#[tokio::test]
async fn recipe_sweep_each_hx_value_is_a_literal_of_the_list() {
    let mut seen: Vec<(String, String)> = Vec::new();
    for answer in answers().await {
        for attribute in hx_attributes(&answer.reply.body) {
            assert_literal(&attribute, &answer.what);
            seen.push((attribute.name, attribute.value));
        }
    }
    let stale: Vec<String> = HX_LITERALS
        .iter()
        .flat_map(|(name, values)| values.iter().map(move |value| (*name, *value)))
        .filter(|(name, value)| !seen.contains(&((*name).to_owned(), (*value).to_owned())))
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    assert!(stale.is_empty(), "no answer holds: {stale:?}");
}

#[tokio::test]
async fn recipe_sweep_no_url_and_no_next_field_holds_the_csrf_token() {
    let token = admin_token().await;
    for answer in answers().await {
        assert_urls_without_token(&answer, &token);
    }
}

#[tokio::test]
async fn recipe_sweep_each_form_with_a_token_sends_post_and_each_get_form_holds_none() {
    let mut with_token = 0_usize;
    let mut sends_get = 0_usize;
    for answer in answers().await {
        for (tag, block) in forms(&answer.reply.body) {
            if block.contains("name=\"csrf_token\"") {
                with_token = with_token.checked_add(1).unwrap();
                assert!(tag.contains(" method=\"post\""), "{}: {tag}", answer.what);
            } else if tag.contains(" method=\"get\"") {
                sends_get = sends_get.checked_add(1).unwrap();
                assert!(!tag.contains("csrf"), "{}: {tag}", answer.what);
            }
        }
    }
    assert!(with_token > 0, "no form holds a token");
    assert!(sends_get > 0, "no form sends GET");
}

#[tokio::test]
async fn recipe_sweep_redirects_hold_no_token_and_send_the_csp() {
    let (fixture, session) = world(Principal::Administrator).await;
    let session = session.unwrap();
    let commands = [
        (
            "/recipes",
            "machine_code=M7&reference_number=R-501&name=A&change_note=B",
        ),
        ("/recipes/R-100/rename", "name=Renamed"),
    ];
    for (path, body) in commands {
        let reply = post_fields(&fixture.router(), path, Some(&session), &[], body).await;
        assert_eq!(
            reply.status,
            StatusCode::SEE_OTHER,
            "{path}: {}",
            reply.body
        );
        assert_eq!(reply.header("content-security-policy"), CSP, "{path}");
        let location = reply.header("location");
        assert!(location.starts_with('/'), "{path}: {location}");
        assert!(!location.contains("csrf_token"), "{path}: {location}");
        assert!(!location.contains(&session.csrf), "{path}: {location}");
    }
}
