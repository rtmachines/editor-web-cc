//! Tests of the summary of `field-errors.html` (task 7.2, design D9, ADR-0041 (d)).
//!
//! The answers come from a router with test routes, so the tests read the real markup.

use axum::Router;
use axum::extract::Path;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::{MethodRouter, get};
use recipe_app::error::AppError;
use recipe_app::ports::NonEmpty;
use recipe_domain::plant::{LeafPath, MachineCode, PlantError};
use recipe_domain::recipe::{ParameterTag, RecipeError, RecipeErrors, SnapshotError, SnapshotItem};

use super::WebError;
use super::lists::leaf_lines;
use crate::WebState;
use crate::i18n::Lang;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{PEER, Reply, capture_logs, get_with, icon_tag, state};

/// The number of lines that the summary shows (ADR-0041 (d)).
const SHOWN: usize = 20;

fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

/// Return `count` faults `E-RCP-004`, one per leaf `[1]` to `[count]`.
fn faults(count: u16) -> RecipeErrors {
    let entries = (1..=count)
        .map(|index| RecipeError::ValueOutOfLimit {
            tag: tag("SPEED_TABLE"),
            leaf: LeafPath::parse(&format!("[{index}]")).unwrap(),
            value: "3500".to_owned(),
            min: "0".to_owned(),
            max: "3000".to_owned(),
        })
        .collect();
    RecipeErrors::from_vec(entries).unwrap()
}

/// The input id of a leaf `[n]`: `value-n`.
fn value_input(path: &LeafPath) -> Option<String> {
    let text = path.to_string();
    let index = text.strip_prefix('[')?.strip_suffix(']')?;
    Some(format!("value-{index}"))
}

async fn fields(Path(count): Path<u16>) -> Result<&'static str, WebError> {
    let lines = leaf_lines(&faults(count), &value_input);
    Err(WebError::Fields {
        lines: NonEmpty::try_from(lines).unwrap(),
        form: None,
        failure: None,
    })
}

async fn validation() -> Result<&'static str, WebError> {
    let items = vec![
        SnapshotItem::Parameter {
            tag: tag("SPEED"),
            error: SnapshotError::Recipe(RecipeError::LimitMissing {
                tag: tag("SPEED"),
                leaf: LeafPath::parse("[2]").unwrap(),
            }),
        },
        SnapshotItem::Version {
            error: SnapshotError::Plant(PlantError::MachineArchived {
                code: MachineCode::parse("M7").unwrap(),
            }),
        },
    ];
    Err(WebError::App(AppError::Recipe(
        RecipeError::ValidationFailed { items },
    )))
}

async fn recipes(Path(count): Path<u16>) -> Result<&'static str, WebError> {
    Err(WebError::App(AppError::Recipes(faults(count))))
}

/// Build the router with the test routes. None of them needs a session.
fn router() -> Router {
    let route = |path: &'static str, handler: MethodRouter<WebState>| RouteEntry {
        method: Method::GET,
        path,
        access: Access::Static,
        handler,
    };
    let rows = vec![
        route("/test/fields/{count}", get(fields)),
        route("/test/validation", get(validation)),
        route("/test/recipes/{count}", get(recipes)),
    ];
    assemble(state(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

async fn ask(path: &str, lang: Lang, htmx: bool) -> Reply {
    let cookie = format!("recipe_lang={lang}");
    let mut headers = vec![("cookie", cookie.as_str())];
    if htmx {
        headers.push(("hx-request", "true"));
    }
    get_with(&router(), path, &headers).await
}

/// Return the element with `role="alert"` of the body, from its tag to its end tag.
fn alert_element(body: &str) -> String {
    let start = body
        .find("<div class=\"alert-head\" role=\"alert\">")
        .unwrap();
    let rest = body.get(start..).unwrap();
    let end = rest.find("</div>").unwrap();
    rest.get(..end).unwrap().to_owned()
}

/// The line of the leaf `[index]` with a link to its input.
fn linked_line(index: u16) -> String {
    format!(
        "<li><a class=\"target-link\" href=\"#value-{index}\"><span class=\"alert-code\">E-RCP-004</span> <span>Value 3500 of SPEED_TABLE[{index}] is outside the process limit 0 to 3000.</span></a></li>"
    )
}

#[tokio::test]
async fn summary_the_alert_holds_only_the_icon_the_code_and_the_count() {
    let reply = ask("/test/fields/2", Lang::En, true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let expected = format!(
        "<div class=\"alert-head\" role=\"alert\">{}",
        icon_tag("alert-error")
    );
    assert!(reply.body.contains(&expected), "{}", reply.body);
    let alert = alert_element(&reply.body);
    assert!(alert.ends_with(
        "<span class=\"alert-code\">E-RCP-004</span><span>Errors: 2. Change the marked fields.</span>"
    ), "{alert}");
    assert!(!alert.contains("SPEED_TABLE"), "{alert}");
    assert_eq!(reply.body.matches("role=\"alert\"").count(), 1);
}

#[tokio::test]
async fn summary_each_line_shows_its_code_its_message_and_a_link_to_its_input() {
    let reply = ask("/test/fields/2", Lang::En, true).await;
    let list = format!(
        "<ul class=\"field-lines\">{}{}</ul>",
        linked_line(1),
        linked_line(2)
    );
    assert!(reply.body.contains(&list), "{}", reply.body);
    let reference = format!(
        "<span class=\"alert-ref\">Reference: {}.</span></div>",
        reply.header("x-request-id")
    );
    assert!(reply.body.contains(&reference), "{}", reply.body);
}

#[tokio::test]
async fn summary_an_htmx_answer_has_the_error_headers() {
    let reply = ask("/test/fields/2", Lang::En, true).await;
    assert_eq!(reply.header("hx-retarget"), "#error-panel");
    assert_eq!(reply.header("hx-reswap"), "innerHTML show:#error-panel:top");
    assert_eq!(reply.header("hx-push-url"), "false");
}

#[tokio::test]
async fn summary_shows_at_most_20_lines_and_then_the_count_of_the_others() {
    let reply = ask("/test/fields/25", Lang::En, true).await;
    let alert = alert_element(&reply.body);
    assert!(
        alert.contains("Errors: 25. Change the marked fields."),
        "{alert}"
    );
    assert_eq!(
        reply.body.matches("<li><a ").count(),
        SHOWN,
        "{}",
        reply.body
    );
    assert!(reply.body.contains(&linked_line(20)), "{}", reply.body);
    assert!(!reply.body.contains("SPEED_TABLE[21]"), "{}", reply.body);
    assert!(
        reply
            .body
            .contains("<li>Other errors: 5. Each one stands under its field.</li></ul>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn summary_with_exactly_20_lines_has_no_line_of_other_errors() {
    let reply = ask("/test/fields/20", Lang::En, true).await;
    assert_eq!(reply.body.matches("<li><a ").count(), SHOWN);
    assert!(!reply.body.contains("Other errors"), "{}", reply.body);
}

#[tokio::test]
async fn summary_follows_the_language_of_the_request() {
    let reply = ask("/test/fields/21", Lang::Pl, true).await;
    let alert = alert_element(&reply.body);
    assert!(
        alert.contains("Liczba b\u{142}\u{119}d\u{f3}w: 21. Popraw oznaczone pola."),
        "{alert}"
    );
    assert!(
        reply.body.contains(
            "<li>Pozosta\u{142}e b\u{142}\u{119}dy: 1. Ka\u{17c}dy stoi pod swoim polem.</li>"
        ),
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .contains("Warto\u{15b}\u{107} 3500 parametru SPEED_TABLE[1]")
    );
}

#[tokio::test]
async fn summary_the_full_error_page_shows_the_lines_without_links() {
    let reply = ask("/test/fields/2", Lang::En, false).await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        reply.body.contains(
            "<li><span class=\"alert-code\">E-RCP-004</span> <span>Value 3500 of SPEED_TABLE[1] is outside the process limit 0 to 3000.</span></li>"
        ),
        "{}",
        reply.body
    );
    assert!(!reply.body.contains("href=\"#value-1\""), "{}", reply.body);
}

#[tokio::test]
async fn summary_an_answer_without_a_form_holds_no_form() {
    let reply = ask("/test/fields/2", Lang::En, true).await;
    assert!(!reply.body.contains("<form"), "{}", reply.body);
    // The only element out of band is the empty status region (design-system.md 10.7).
    assert_eq!(
        reply.body.matches("hx-swap-oob").count(),
        1,
        "{}",
        reply.body
    );
    assert!(
        reply
            .body
            .ends_with("<div id=\"toast\" hx-swap-oob=\"innerHTML\"></div>"),
        "{}",
        reply.body
    );
}

#[tokio::test]
async fn summary_the_items_of_e_rcp_020_give_one_line_each_under_the_code_e_rcp_020() {
    let reply = ask("/test/validation", Lang::En, true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    let alert = alert_element(&reply.body);
    assert!(
        alert.contains("<span class=\"alert-code\">E-RCP-020</span>"),
        "{alert}"
    );
    assert!(
        alert.contains("Errors: 2. Change the marked fields."),
        "{alert}"
    );
    assert!(reply.body.contains(
        "<li><span class=\"alert-code\">E-RCP-016</span> <span>SPEED[2] has no process limit. Set a limit before approval.</span></li>"
    ), "{}", reply.body);
    assert!(reply.body.contains(
        "<li><span class=\"alert-code\">E-PLT-010</span> <span>Machine M7 is archived.</span></li>"
    ), "{}", reply.body);
}

#[tokio::test]
async fn summary_a_list_of_recipe_errors_gives_one_line_per_entry() {
    let reply = ask("/test/recipes/2", Lang::En, true).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.body
    );
    assert_eq!(reply.body.matches("<li>").count(), 2, "{}", reply.body);
    assert!(reply.body.contains("SPEED_TABLE[2]"), "{}", reply.body);
}

#[tokio::test]
async fn summary_the_log_line_of_a_list_of_100_holds_the_count_and_20_lines() {
    for path in ["/test/recipes/100", "/test/fields/100"] {
        let (logs, _guard) = capture_logs();
        ask(path, Lang::En, true).await;
        let text = logs.text();
        assert_eq!(text.matches("request failed").count(), 1, "{text}");
        assert!(
            text.contains("100 errors. The first 20 follow."),
            "{path}: {text}"
        );
        assert!(text.contains("SPEED_TABLE[20] "), "{path}: {text}");
        assert!(!text.contains("SPEED_TABLE[21] "), "{path}: {text}");
    }
}
