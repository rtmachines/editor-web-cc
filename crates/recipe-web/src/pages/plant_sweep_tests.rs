//! The `hx-*` rule and the CSRF token rule over every answer of the plant pages (task 11.2, D14).
//!
//! The sweep reads the full `GET` pages of the three areas, the HTMX fragments, the content answers,
//! and the success answers of the `POST` commands. It runs each one in `pl` and in `en`.

use axum::http::StatusCode;

use super::page_tests::{LANGS, administrator, assert_assets};
use super::test_helpers::{CSP, HxAttribute, assert_clean_html, field_value, hx_attributes};
use super::{machines, types, units};
use crate::components_tests::assert_fixed_svg_tags;
use crate::test_support::plant::{HTMX, TARGET_CONTENT};
use crate::test_support::{Fixture, Reply, TestSession, get_with, post_fields};

/// The header of an HTMX request that targets the unit form.
const HTMX_UNIT_FORM: (&str, &str) = ("hx-target", "unit-form");
/// The value of `hx-headers` of `<body>` is the one value that holds the token (`api.md` section 7.3).
const HX_HEADERS: &str = "hx-headers";
/// Every `hx-*` attribute that a plant answer may hold, with the literals that it may have.
///
/// A name or a value that is not in the list fails the test. The test also fails when a literal of
/// the list appears in no answer, so the list cannot go stale.
const HX_LITERALS: [(&str, &[&str]); 7] = [
    ("hx-boost", &["true"]),
    (
        "hx-target",
        &[
            "#content",
            "#unit-form",
            "#unit-tree",
            "#machine-name",
            "#unit-name",
        ],
    ),
    ("hx-swap", &["innerHTML"]),
    ("hx-push-url", &["true", "false"]),
    ("hx-sync", &["closest form:drop"]),
    ("hx-swap-oob", &["true", "innerHTML"]),
    ("hx-history", &["false"]),
];
/// The commands of the success answers: path, body, and the target of the HTMX request.
const COMMANDS: [(&str, &str, &str); 5] = [
    ("/machines/M7/rename", "name=Renamed", "machine-name"),
    (
        "/machines/M7/units/U1.2/rename",
        "name=Renamed",
        "unit-name",
    ),
    (
        "/machines/M7/units",
        "parent_path=U1.2&segment=5&name=Added",
        "unit-tree",
    ),
    (
        "/machines/M7/types",
        "name=ST_P&members[0].name=a&members[0].type=INT&add=1",
        "content",
    ),
    (
        "/machines/M7/types/ST_Zone/revisions",
        "members[0].name=a&members[0].type=INT&add=1",
        "content",
    ),
];

/// One answer of the sweep with the status that it must have.
struct Answer {
    what: String,
    reply: Reply,
    want: StatusCode,
    /// True for a full page. A fragment has no frame and no script.
    full: bool,
}

/// The state of one language run: the fixture, the session, and the language code.
struct Run<'a> {
    fixture: &'a Fixture,
    session: &'a TestSession,
    code: &'a str,
}

impl Run<'_> {
    /// Send `GET path` with the extra headers.
    async fn get(&self, path: &str, extra: &[(&str, &str)]) -> Reply {
        let cookie = format!("{}; recipe_lang={}", self.session.cookie, self.code);
        let mut headers = vec![("cookie", cookie.as_str())];
        headers.extend_from_slice(extra);
        get_with(&self.fixture.router(), path, &headers).await
    }

    /// Send the form `body` to `path` as an HTMX request with the target `target`.
    async fn post(&self, path: &str, body: &str, target: &str) -> Reply {
        let lang = format!("recipe_lang={}", self.code);
        let headers = [("cookie", lang.as_str()), HTMX, ("hx-target", target)];
        let router = self.fixture.router();
        post_fields(&router, path, Some(self.session), &headers, body).await
    }

    /// Return the answer of a request that must give `200`.
    fn answer(&self, what: String, reply: Reply, full: bool) -> Answer {
        let want = StatusCode::OK;
        Answer {
            what: format!("{} {what}", self.code),
            reply,
            want,
            full,
        }
    }
}

/// Return the pattern and the path of each full `GET` page of the three areas.
fn sweep_pages() -> Vec<(&'static str, String)> {
    let mut pages = machines::route_tests::sweep_pages();
    pages.extend(units::route_tests::sweep_pages());
    pages.extend(types::route_tests::sweep_pages());
    pages
}

/// Collect the full pages, the content answers, and the two fragments of the `GET` routes.
async fn page_answers(run: &Run<'_>) -> Vec<Answer> {
    let mut answers = Vec::new();
    for (_pattern, path) in sweep_pages() {
        let full = run.get(&path, &[]).await;
        answers.push(run.answer(format!("GET {path}"), full, true));
        let content = run.get(&path, &[HTMX, TARGET_CONTENT]).await;
        answers.push(run.answer(format!("GET {path} content"), content, false));
    }
    let tree = run.get("/machines/M7/units", &[HTMX]).await;
    answers.push(run.answer("GET units fragment".to_owned(), tree, false));
    let path = "/machines/M7/units/new?parent_path=U1.2";
    let form = run.get(path, &[HTMX, HTMX_UNIT_FORM]).await;
    answers.push(run.answer(format!("GET {path} fragment"), form, false));
    answers
}

/// Collect the success answers of the commands and one error answer of an HTMX request.
async fn command_answers(run: &Run<'_>) -> Vec<Answer> {
    let mut answers = Vec::new();
    for (path, body, target) in COMMANDS {
        let reply = run.post(path, body, target).await;
        answers.push(run.answer(format!("POST {path}"), reply, false));
    }
    let path = "/machines/M7/rename";
    let refused = run.post(path, "name=%00", "machine-name").await;
    let mut answer = run.answer(format!("POST {path} refused"), refused, false);
    answer.want = StatusCode::UNPROCESSABLE_ENTITY;
    answers.push(answer);
    answers
}

/// Collect every answer of the sweep in both languages. Each language gets a fresh fixture.
async fn answers() -> Vec<Answer> {
    let mut all = Vec::new();
    for (_lang, code) in LANGS {
        let (fixture, session) = administrator();
        fixture.seed_plant();
        let run = Run {
            fixture: &fixture,
            session: &session,
            code,
        };
        all.extend(page_answers(&run).await);
        all.extend(command_answers(&run).await);
    }
    all
}

// --- Reading the markup -------------------------------------------------------------------------------

/// Return the values of the attribute `name` of every tag of `body`, written as ` name="..."`.
fn attribute_values<'a>(body: &'a str, name: &str) -> Vec<&'a str> {
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
    urls.push(reply.header("location"));
    urls.push(reply.header("hx-push-url"));
    urls.push(reply.header("hx-redirect"));
    if reply.body.contains("name=\"next\"") {
        urls.push(field_value(&reply.body, "next"));
    }
    for url in urls {
        assert!(!url.contains("csrf_token"), "{what}: {url}");
        assert!(!url.contains(token), "{what}: token in {url}");
    }
}

#[tokio::test]
async fn plant_sweep_every_answer_has_the_csp_and_clean_markup() {
    for answer in answers().await {
        let Answer {
            what,
            reply,
            want,
            full,
        } = &answer;
        assert_eq!(reply.status, *want, "{what}: {}", reply.body);
        assert_eq!(reply.header("content-security-policy"), CSP, "{what}");
        assert_clean_html(&reply.body);
        assert_fixed_svg_tags(&reply.body);
        assert!(!reply.body.contains("<style"), "{what}");
        if *full {
            assert_assets(&reply.body);
        } else {
            assert!(!reply.body.contains("<script"), "{what}: script");
        }
    }
}

#[tokio::test]
async fn plant_sweep_each_hx_value_is_a_literal_of_the_list() {
    let mut seen: Vec<(String, String)> = Vec::new();
    for answer in answers().await {
        for attribute in hx_attributes(&answer.reply.body) {
            assert_literal(&attribute, &answer.what);
            seen.push((attribute.name, attribute.value));
        }
    }
    for (name, values) in HX_LITERALS {
        for value in values {
            let pair = (name.to_owned(), (*value).to_owned());
            assert!(seen.contains(&pair), "no answer holds {name}={value}");
        }
    }
}

#[tokio::test]
async fn plant_sweep_no_url_and_no_next_field_holds_the_csrf_token() {
    let (_fixture, session) = administrator();
    for answer in answers().await {
        assert_urls_without_token(&answer, &session.csrf);
    }
}

#[tokio::test]
async fn plant_sweep_each_form_with_a_token_sends_post() {
    let mut with_token = 0_usize;
    for answer in answers().await {
        for (tag, block) in forms(&answer.reply.body) {
            let holds_token = block.contains("name=\"csrf_token\"");
            if holds_token {
                with_token = with_token.checked_add(1).unwrap();
                assert!(tag.contains(" method=\"post\""), "{}: {tag}", answer.what);
            } else {
                assert!(!tag.contains("csrf"), "{}: {tag}", answer.what);
            }
        }
    }
    assert!(with_token > 0, "no form holds a token");
}

#[tokio::test]
async fn plant_sweep_redirects_hold_no_token_and_send_the_csp() {
    let (fixture, session) = administrator();
    fixture.seed_plant();
    let commands = [
        (
            "/machines",
            "code=M99&name=C&plc_platform=S7_300&main_unit_name=I",
        ),
        ("/machines/M7/rename", "name=Renamed"),
        (
            "/machines/M7/units",
            "parent_path=U1.2&segment=5&name=Added",
        ),
        ("/machines/M7/units/U1.2/rename", "name=Renamed"),
        ("/machines/M7/units/U1.2/archive", ""),
        (
            "/machines/M7/types",
            "name=ST_P&members[0].name=a&members[0].type=INT",
        ),
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
