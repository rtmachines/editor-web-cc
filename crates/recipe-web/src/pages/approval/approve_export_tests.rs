//! Tests of the error answer of `E-EXP-004`, the code that `POST /recipes/{ref}/versions/{n}/approve`
//! returns when the serializer cannot write the export document (design D8 and D9).
//!
//! The text of the code holds no `{ $request_id }`. The error answer adds the key
//! `error-reference` once. A message that needs the reference would fail to format, and the answer
//! would then show the English `Display` text, also in `pl`.

use axum::Router;
use axum::extract::connect_info::MockConnectInfo;
use axum::http::{Method, StatusCode};
use axum::routing::get;
use recipe_app::error::AppError;
use recipe_domain::export::ExportError;

use crate::error::WebError;
use crate::i18n::Lang;
use crate::routes::{Access, RouteEntry, assemble};
use crate::test_support::{PEER, Reply, get_with, state};

/// The English sentence of `error-E-EXP-004`.
const SENTENCE_EN: &str =
    "The server cannot create the export document. Contact the administrator.";
/// The Polish sentence of `error-E-EXP-004`.
const SENTENCE_PL: &str = "Serwer nie mo\u{17c}e utworzy\u{107} dokumentu eksportu. Skontaktuj si\u{119} z administratorem.";

/// Raise the error that the approval returns when the serializer fails.
async fn raise() -> Result<&'static str, WebError> {
    let failure = ExportError::Serialize(serde::ser::Error::custom("the serializer failed"));
    Err(WebError::App(AppError::Export(failure)))
}

/// Build the router with the one test route and the layer of the error answer.
fn router() -> Router {
    let rows = vec![RouteEntry {
        method: Method::GET,
        path: "/test/export",
        access: Access::Static,
        handler: get(raise),
    }];
    assemble(state(), rows, Router::new()).layer(MockConnectInfo(PEER))
}

/// Send the request of an HTMX form in `lang` to the test route.
async fn ask(lang: Lang) -> Reply {
    let cookie = format!("recipe_lang={lang}");
    let headers = [("cookie", cookie.as_str()), ("hx-request", "true")];
    get_with(&router(), "/test/export", &headers).await
}

/// Assert the answer: the status, the sentence, the code, and one reference number.
fn assert_answer(reply: &Reply, sentence: &str, label: &str) {
    let body = &reply.body;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR, "{body}");
    assert!(body.contains(sentence), "{body}");
    assert!(body.contains("E-EXP-004"), "{body}");
    assert_eq!(body.matches(label).count(), 1, "{body}");
    let id = reply.header("x-request-id");
    assert!(!id.is_empty(), "{body}");
    assert_eq!(body.matches(id.as_str()).count(), 1, "{body}");
    let span = format!("<span class=\"alert-ref\">{label} {id}.</span>");
    assert!(body.contains(&span), "{body}");
}

#[tokio::test]
async fn approve_export_failure_answers_the_polish_sentence_with_one_reference() {
    let reply = ask(Lang::Pl).await;
    assert_answer(&reply, SENTENCE_PL, "Numer referencyjny:");
    assert!(!reply.body.contains("Referencja:"), "{}", reply.body);
}

#[tokio::test]
async fn approve_export_failure_answers_the_english_sentence_with_one_reference() {
    let reply = ask(Lang::En).await;
    assert_answer(&reply, SENTENCE_EN, "Reference:");
}
