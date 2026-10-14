//! `POST /lang` (`api.md` section 8, design D12 item o) and the baseline language-cookie requirements.

use axum::http::{Method, StatusCode};

use crate::support::{Harness, body_bytes, header, post_form, post_lang, send};

/// The attributes of the language cookie. The `cookie` crate writes them in its own order.
const LANG_COOKIE_ATTRIBUTES: [&str; 4] =
    ["Max-Age=31536000", "Secure", "SameSite=Strict", "Path=/"];

/// Assert that the `Set-Cookie` text sets `recipe_lang` to `lang` with the expected attributes.
///
/// The test compares the attributes as a set, so the order of the `cookie` crate does not matter.
fn assert_lang_cookie(text: &str, lang: &str) {
    let pair = format!("recipe_lang={lang}");
    let mut parts = text.split("; ");
    assert_eq!(parts.next(), Some(pair.as_str()), "{text}");
    let mut attributes: Vec<&str> = parts.collect();
    attributes.sort_unstable();
    let mut expected = LANG_COOKIE_ATTRIBUTES.to_vec();
    expected.sort_unstable();
    assert_eq!(attributes, expected, "{text}");
}

/// Percent-encode the characters that matter in a `next` value of these tests.
fn encode(next: &str) -> String {
    let mut out = String::new();
    for byte in next.bytes() {
        if byte.is_ascii_alphanumeric() || b"-._~/".contains(&byte) {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

async fn switch(lang: &str, next: &str) -> axum::response::Response {
    let router = Harness::new().router();
    let fields = format!("lang={lang}&next={}", encode(next));
    post_lang(&router, &fields, &[]).await
}

#[tokio::test]
async fn lang_external_address_goes_to_recipes() {
    let response = switch("en", "https://other.example/page").await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header(&response, "location"), "/recipes");
    assert_lang_cookie(&header(&response, "set-cookie"), "en");
}

#[tokio::test]
async fn lang_unsafe_next_values_go_to_recipes() {
    let payloads = [
        "//evil.example",
        "/\\evil.example",
        "/\t/evil.example",
        "\\\\evil.example",
        "\\evil.example",
        "evil.example",
        "javascript:alert(1)",
        "/a\\b",
        "/a b",
        "/a\nb",
        "/a\rb",
        "/a\u{85}b",
        "/a\u{a0}b",
        "",
    ];
    for payload in payloads {
        let response = switch("en", payload).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{payload:?}");
        assert_eq!(header(&response, "location"), "/recipes", "{payload:?}");
    }
}

#[tokio::test]
async fn lang_same_origin_path_is_kept() {
    for next in [
        "/recipes/R-100/versions/3",
        "/",
        "/users?q=anna&page=2",
        "/a%5Cb",
    ] {
        let response = switch("pl", next).await;
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{next}");
        assert_eq!(header(&response, "location"), next, "{next}");
        assert_lang_cookie(&header(&response, "set-cookie"), "pl");
    }
}

#[tokio::test]
async fn lang_non_ascii_path_is_sent_percent_encoded() {
    let response = switch("pl", "/receptury/\u{141}\u{f3}d\u{17a}").await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(
        header(&response, "location"),
        "/receptury/%C5%81%C3%B3d%C5%BA"
    );
}

#[tokio::test]
async fn lang_missing_next_goes_to_recipes() {
    let router = Harness::new().router();
    let response = post_lang(&router, "lang=en", &[]).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header(&response, "location"), "/recipes");
}

#[tokio::test]
async fn lang_never_reads_the_referer() {
    let router = Harness::new().router();
    let headers = [("referer", "https://other.example/page?x=1")];
    let response = post_lang(&router, "lang=en&next=%2F%2Fevil.example", &headers).await;
    assert_eq!(header(&response, "location"), "/recipes");
    let response = post_lang(&router, "lang=en", &headers).await;
    assert_eq!(header(&response, "location"), "/recipes");
}

#[tokio::test]
async fn lang_unknown_language_is_refused_and_sets_no_cookie() {
    for lang in ["de", "", "EN", "en-US"] {
        let router = Harness::new().router();
        let response = post_lang(&router, &format!("lang={lang}&next=%2F"), &[]).await;
        assert_eq!(
            response.status(),
            StatusCode::UNPROCESSABLE_ENTITY,
            "{lang:?}"
        );
        assert_eq!(header(&response, "set-cookie"), "", "{lang:?}");
    }
}

#[tokio::test]
async fn lang_unknown_language_shows_the_field_error_fragment() {
    let router = Harness::new().router();
    let headers = [("hx-request", "true"), ("cookie", "recipe_lang=en")];
    let response = post_lang(&router, "lang=de", &headers).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(header(&response, "hx-retarget"), "#error-panel");
    let request_id = header(&response, "x-request-id");
    let body = String::from_utf8(body_bytes(response).await).unwrap();
    assert!(body.contains("E-WEB-008"), "{body}");
    assert!(body.contains("Field lang is not valid: the language must be pl or en."));
    assert!(body.contains(&request_id));
}

#[tokio::test]
async fn lang_form_with_the_wrong_content_type_is_stopped_by_the_csrf_layer() {
    // A body that is not an urlencoded form carries no token, so the layer answers before the
    // handler can answer E-WEB-008.
    let harness = Harness::new();
    let router = harness.router();
    let request = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/lang")
        .header("content-type", "application/json")
        .header("cookie", "recipe_lang=en")
        .header("sec-fetch-site", "same-origin")
        .header("hx-request", "true")
        .body(axum::body::Body::from(r#"{"lang":"en"}"#))
        .unwrap();
    let response = tower::ServiceExt::oneshot(router, request).await.unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(header(&response, "set-cookie"), "");
    let body = String::from_utf8(body_bytes(response).await).unwrap();
    assert!(body.contains("E-WEB-007"), "{body}");
    assert_eq!(harness.sink.events().len(), 1);
}

#[tokio::test]
async fn lang_missing_language_is_refused() {
    let router = Harness::new().router();
    let response = post_lang(&router, "next=%2F", &[]).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn lang_unknown_field_is_refused() {
    let router = Harness::new().router();
    let response = post_lang(&router, "lang=en&admin=1", &[]).await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(header(&response, "set-cookie"), "");
}

#[tokio::test]
async fn lang_session_token_field_without_a_session_is_refused() {
    let harness = Harness::new();
    let router = harness.router();
    let headers = [("sec-fetch-site", "same-origin")];
    let response = post_form(&router, "/lang", "lang=en&csrf_token=abc", &headers).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(header(&response, "set-cookie"), "");
    assert_eq!(harness.sink.events().len(), 1);
}

#[tokio::test]
async fn lang_without_a_token_or_an_origin_is_refused() {
    let harness = Harness::new();
    let router = harness.router();
    let response = post_form(&router, "/lang", "lang=en", &[]).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(harness.sink.events().len(), 1);
}

#[tokio::test]
async fn lang_get_is_not_a_route() {
    let router = Harness::new().router();
    let response = send(&router, Method::GET, "/lang").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_eq!(header(&response, "set-cookie"), "");
}

#[tokio::test]
async fn lang_works_in_read_only_mode_and_writes_no_row() {
    let harness = Harness::new();
    harness.state.read_only().set(true);
    let router = harness.router();
    let response = post_lang(&router, "lang=en&next=%2Fusers", &[]).await;
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    assert_eq!(header(&response, "location"), "/users");
    assert!(harness.store.journal().is_empty());
    assert!(harness.sink.events().is_empty());
}

#[tokio::test]
async fn lang_cookie_ignores_a_cookie_that_the_request_sends() {
    let router = Harness::new().router();
    let headers = [("cookie", "recipe_lang=de"), ("accept-language", "en-US")];
    let response = post_lang(&router, "lang=pl", &headers).await;
    assert_lang_cookie(&header(&response, "set-cookie"), "pl");
}
