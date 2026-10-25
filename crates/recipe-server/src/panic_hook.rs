//! The panic hook of the server (`reliability.md` section 2.4 rule 2).
//!
//! The hook is the only place that sees the message and the location of every panic. The
//! router layers, the job supervisor, and the join of a task catch the panic after the hook
//! runs. The hook ends no process and calls no earlier hook, because the earlier hook of the
//! standard library would print a second text on stderr.

use recipe_app::operations::AlertCounters;

/// The text of a panic whose payload is not a string.
const NON_TEXT_PAYLOAD: &str = "the panic payload is not text";

/// The text of a panic without a source location.
const NO_LOCATION: &str = "unknown";

/// Install the panic hook as the hook of the process.
///
/// The hook counts the panic in `counters` first, because the job `alerts` reads that counter.
/// It then writes one `ERROR` event with the panic message and the source location. The event
/// inherits the open spans of the panicking thread, so a panic in a request gives the
/// `request_id` of that request (`reliability.md` section 3.2).
///
/// The function solves one problem: a panic outside a request has no `E-WEB-010` line, and
/// the operator must still see it and count it.
pub fn install(counters: AlertCounters) {
    std::panic::set_hook(Box::new(move |info| {
        counters.record_panic();
        let message = info.payload_as_str().unwrap_or(NON_TEXT_PAYLOAD);
        let location = info
            .location()
            .map_or_else(|| NO_LOCATION.to_owned(), ToString::to_string);
        tracing::error!(
            panic_message = message,
            location = location,
            "a panic occurred"
        );
    }));
}
