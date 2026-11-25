#![cfg(target_env = "musl")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Unwind proof with the allocator of the shipped image (ADR-0224 (q), design D7).
//!
//! The musl build of `recipe-server` uses `mimalloc` as its global allocator. The gnu tests use the
//! system allocator. No other test proves the unwind with the allocator of the shipped image.
//! This test declares the same static as `main.rs` and lets a route panic.
//! The route allocates a buffer and holds a value with a `Drop`. `CatchPanicLayer` wraps the route.
//! The route must answer `500`, the `Drop` must run, and a second route must answer `200`.
//!
//! The file compiles to an empty crate on a gnu host. The mode `mimalloc-unwind` of
//! `deploy/ci/rust-tests.sh` runs it on musl in the dev profile and with `--release`.

use std::future::poll_fn;
use std::hint::black_box;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use tower::Service;
use tower_http::catch_panic::CatchPanicLayer;

/// The allocator of the shipped image. It is the same static as the one in `main.rs`.
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// The size of the buffer of the panicking route. The size is large, so the buffer is a big block.
const BUFFER_BYTES: usize = 1024 * 1024;

/// A value whose `Drop` sets a flag. The flag proves that the unwind ran the drop code.
struct DropFlag(Arc<AtomicBool>);

impl Drop for DropFlag {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

/// A route that allocates a buffer, holds a `DropFlag`, and then panics.
async fn allocate_and_panic(State(dropped): State<Arc<AtomicBool>>) -> &'static str {
    let buffer = black_box(vec![1_u8; BUFFER_BYTES]);
    let _guard = DropFlag(dropped);
    // A failed parse stands for any bug of a handler that panics.
    let _number: u8 = "not a number".parse().unwrap();
    black_box(buffer.len());
    "not reached"
}

/// Send one `GET` request to the router and return the status of the answer.
async fn get_status(router: &mut Router, path: &str) -> StatusCode {
    poll_fn(|context| Service::<Request<Body>>::poll_ready(router, context))
        .await
        .unwrap();
    let request = Request::builder().uri(path).body(Body::empty()).unwrap();
    router.call(request).await.unwrap().status()
}

#[tokio::test]
async fn catch_panic_layer_answers_500_and_runs_drop_after_a_panic() {
    let dropped = Arc::new(AtomicBool::new(false));
    let mut router = Router::new()
        .route("/panic", get(allocate_and_panic))
        .route("/ok", get(|| async { "ok" }))
        .with_state(Arc::clone(&dropped))
        .layer(CatchPanicLayer::new());

    let panic_status = get_status(&mut router, "/panic").await;
    assert_eq!(panic_status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        dropped.load(Ordering::SeqCst),
        "the unwind must run the Drop of the guard"
    );

    let ok_status = get_status(&mut router, "/ok").await;
    assert_eq!(ok_status, StatusCode::OK);
}
