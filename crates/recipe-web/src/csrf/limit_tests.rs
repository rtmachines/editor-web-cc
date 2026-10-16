//! Tests of the body limit in the buffer of the CSRF check (task 8.2, design D11).
//!
//! The buffer uses the limit of the route. A chunked body announces no length, so only the buffer
//! can refuse it. A body above the limit gives `413 E-WEB-003` and never `403 E-WEB-007`.

use axum::body::{Body, Bytes};
use axum::http::StatusCode;

use super::buffer_body;
use crate::error::WebError;
use crate::limits::route_tests::{
    FORM_PATHS, TRANSLATION_PATH, VALUE_PATHS, chunked, limit_router, send_with,
};

const KIB: usize = 1024;

#[tokio::test]
async fn csrf_buffer_refuses_a_chunked_body_above_the_limit() {
    let error = buffer_body(chunked(11), 10).await.unwrap_err();
    assert!(
        matches!(error, WebError::PayloadTooLarge { .. }),
        "{error:?}"
    );
}

#[tokio::test]
async fn csrf_buffer_keeps_a_chunked_body_of_exactly_the_limit() {
    let kept: Bytes = buffer_body(chunked(10), 10).await.unwrap();
    assert_eq!(kept.len(), 10);
}

#[tokio::test]
async fn csrf_buffer_names_the_limit_that_it_was_given() {
    let error = buffer_body(Body::from(vec![b'a'; 65]), 64)
        .await
        .unwrap_err();
    assert!(
        matches!(&error, WebError::PayloadTooLarge { limit } if limit.to_string() == "64 B"),
        "{error:?}"
    );
}

#[tokio::test]
async fn csrf_layer_reads_the_token_field_from_a_body_above_the_form_limit_on_a_value_route() {
    let (router, session) = limit_router();
    let padding = "a".repeat(300 * KIB);
    let body = format!("csrf_token={}&value={padding}", session.csrf);
    let headers = [("cookie", session.cookie.as_str())];
    let reply = send_with(&router, VALUE_PATHS[0], (&headers, Body::from(body))).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.body);
}

#[tokio::test]
async fn csrf_layer_gives_413_and_not_403_for_a_body_above_the_limit_with_a_wrong_token() {
    let (router, session) = limit_router();
    let headers = [
        ("cookie", session.cookie.as_str()),
        ("x-csrf-token", "wrong"),
    ];
    let cases = [
        (VALUE_PATHS[2], 4 * KIB * KIB + 1),
        (TRANSLATION_PATH, 64 * KIB + 1),
        (FORM_PATHS[1], 256 * KIB + 1),
    ];
    for (path, length) in cases {
        let reply = send_with(&router, path, (&headers, chunked(length))).await;
        assert_eq!(reply.status, StatusCode::PAYLOAD_TOO_LARGE, "{path}");
        assert!(reply.body.contains("E-WEB-003"), "{path}: {}", reply.body);
    }
}

#[tokio::test]
async fn csrf_layer_gives_403_for_a_wrong_token_inside_the_limit_of_a_value_route() {
    let (router, session) = limit_router();
    let headers = [
        ("cookie", session.cookie.as_str()),
        ("x-csrf-token", "wrong"),
    ];
    let reply = send_with(&router, VALUE_PATHS[0], (&headers, chunked(300 * KIB))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN, "{}", reply.body);
    assert!(reply.body.contains("E-WEB-007"), "{}", reply.body);
}
