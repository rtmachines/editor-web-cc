// reliability.md section 2.3: test code may use `unwrap`, `expect`, and indexing.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

//! HTTP tests of `recipe-web`. The tests drive the router with `tower::ServiceExt::oneshot`.

mod headers;
mod lang;
mod static_files;
mod support;
