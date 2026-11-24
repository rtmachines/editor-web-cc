//! Tests of the bounded readers of `test_support`.

use axum::Router;
use axum::body::Body;
use axum::routing::{get, post};

use super::{Method, call, call_bounded, get_bounded, post_fields_bounded, request};

/// The size of the answer in the tests: twice the limit of `call`.
const BIG: usize = 2 * 1024 * 1024;

/// Build a router that answers `BIG` bytes on `GET /big` and on `POST /big`.
fn big_router() -> Router {
    Router::new()
        .route("/big", get(|| async { "a".repeat(BIG) }))
        .route("/big", post(|| async { "b".repeat(BIG) }))
}

#[tokio::test]
async fn call_bounded_reads_an_answer_above_the_default_limit() {
    let req = request(Method::GET, "/big").body(Body::empty()).unwrap();
    let reply = call_bounded(&big_router(), req, BIG).await;
    assert_eq!(reply.body.len(), BIG);
}

#[tokio::test]
async fn get_bounded_reads_an_answer_above_the_default_limit() {
    let reply = get_bounded(&big_router(), "/big", &[], BIG).await;
    assert_eq!(reply.body.len(), BIG);
}

#[tokio::test]
async fn post_fields_bounded_reads_an_answer_above_the_default_limit() {
    let reply = post_fields_bounded(&big_router(), "/big", None, &[], "a=b", BIG).await;
    assert_eq!(reply.body.len(), BIG);
}

#[tokio::test]
#[should_panic(expected = "LengthLimitError")]
async fn call_still_refuses_an_answer_above_one_mebibyte() {
    let req = request(Method::GET, "/big").body(Body::empty()).unwrap();
    let _ = call(&big_router(), req).await;
}
