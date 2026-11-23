//! The count of the calls of `GridView::build` on the current thread. This module is test code.
//!
//! The module solves one problem: a route test must show how many grids a request builds. A route
//! test runs the router on its own thread, so a thread-local count does not mix with other tests.
use std::cell::Cell;

thread_local! {
    /// The calls of `GridView::build` since the last `reset` on this thread.
    static BUILDS: Cell<usize> = const { Cell::new(0) };
}

/// Count one call of `GridView::build`.
pub(super) fn record() {
    BUILDS.with(|builds| builds.set(builds.get().saturating_add(1)));
}

/// Set the count of this thread to 0. A test calls it before the request.
pub(crate) fn reset() {
    BUILDS.with(|builds| builds.set(0));
}

/// Return the number of the calls since the last `reset` on this thread.
pub(crate) fn builds() -> usize {
    BUILDS.with(Cell::get)
}
