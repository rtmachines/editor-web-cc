//! Tests of the log filter, the JSON subscriber, and the fallback logger.

use std::io;
use std::sync::LazyLock;
use std::sync::mpsc::{self, Sender};

use tracing::Dispatch;
use tracing::subscriber::NoSubscriber;

use super::{DEFAULT_FILTER, build_filter, fallback_error, subscriber};

/// A registered dispatch that no thread installs. It lives until the test process ends.
///
/// It solves one problem of `tracing-core` 0.1.36 (tokio-rs/tracing#3611). With one live
/// dispatch, a callsite takes its interest from the thread that hits it first. A test thread
/// without a subscriber then caches `Interest::never` for all threads.
static KEEPALIVE: LazyLock<Dispatch> = LazyLock::new(|| Dispatch::new(NoSubscriber::default()));

/// A writer that sends each write to a channel, so a test reads the log output without a lock.
struct ChannelWriter(Sender<Vec<u8>>);

impl io::Write for ChannelWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.0.send(buf.to_vec()).map_err(io::Error::other)?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// Run `emit` under the subscriber of `directives` and return the lines that it wrote.
fn lines_of(directives: Option<&str>, emit: impl FnOnce()) -> Vec<String> {
    let (sender, receiver) = mpsc::channel();
    let filter = build_filter(directives).unwrap();
    let subscriber = subscriber(filter, move || ChannelWriter(sender.clone()));
    LazyLock::force(&KEEPALIVE);
    tracing::subscriber::with_default(subscriber, emit);
    let bytes: Vec<u8> = receiver.try_iter().flatten().collect();
    String::from_utf8(bytes)
        .unwrap()
        .lines()
        .map(str::to_owned)
        .collect()
}

#[test]
fn the_default_filter_text_is_the_text_of_reliability_3_3() {
    assert_eq!(DEFAULT_FILTER, "info,sqlx=warn,hyper=warn");
}

#[test]
fn a_missing_variable_gives_info_for_the_application_and_warn_for_sqlx_and_hyper() {
    let lines = lines_of(None, || {
        tracing::debug!(target: "recipe_server", "app debug");
        tracing::info!(target: "recipe_server", "app info");
        tracing::info!(target: "sqlx::query", "sqlx info");
        tracing::warn!(target: "sqlx::query", "sqlx warn");
        tracing::info!(target: "hyper::proto", "hyper info");
        tracing::warn!(target: "hyper::proto", "hyper warn");
    });

    let text = lines.join("\n");
    assert_eq!(lines.len(), 3, "{text}");
    assert!(text.contains("app info"), "{text}");
    assert!(text.contains("sqlx warn"), "{text}");
    assert!(text.contains("hyper warn"), "{text}");
}

#[test]
fn an_empty_variable_gives_the_default_and_not_a_silent_server() {
    let lines = lines_of(Some("  "), || {
        tracing::info!(target: "recipe_server", "app info");
    });

    assert_eq!(lines.len(), 1, "{lines:?}");
}

#[test]
fn a_text_without_a_directive_gives_the_default() {
    for text in [",", " , ", ",,", " ,  , "] {
        let lines = lines_of(Some(text), || {
            tracing::debug!(target: "recipe_server", "app debug");
            tracing::info!(target: "recipe_server", "app info");
            tracing::info!(target: "sqlx::query", "sqlx info");
        });

        assert_eq!(lines.len(), 1, "text: {text:?}, lines: {lines:?}");
        assert!(lines[0].contains("app info"), "text: {text:?}");
    }
}

#[test]
fn a_space_after_a_comma_does_not_change_a_directive() {
    let lines = lines_of(Some("info, debug"), || {
        tracing::debug!(target: "recipe_server", "app debug");
    });
    assert_eq!(lines.len(), 1, "{lines:?}");

    let spaced = build_filter(Some("info, sqlx=warn")).unwrap();
    let plain = build_filter(Some("info,sqlx=warn")).unwrap();
    assert_eq!(spaced.to_string(), plain.to_string());
    let lines = lines_of(Some("info, sqlx=warn"), || {
        tracing::info!(target: "recipe_server", "app info");
        tracing::info!(target: "sqlx::query", "sqlx info");
    });
    assert_eq!(lines.len(), 1, "{lines:?}");
}

#[test]
fn a_non_ascii_directive_after_a_comma_and_a_space_does_not_panic() {
    assert!(build_filter(Some("info, \u{17c}=warn")).is_ok());
}

#[test]
fn a_good_directive_replaces_the_default() {
    let lines = lines_of(Some("debug,tower_http=error"), || {
        tracing::debug!(target: "recipe_server", "app debug");
        tracing::info!(target: "tower_http::trace", "tower info");
        tracing::info!(target: "sqlx::query", "sqlx info");
    });

    let text = lines.join("\n");
    assert_eq!(lines.len(), 2, "{text}");
    assert!(text.contains("app debug"), "{text}");
    assert!(text.contains("sqlx info"), "{text}");
}

#[test]
fn a_bad_directive_returns_an_error_that_names_the_variable_and_the_text() {
    for text in ["info,sqlx=loud", "info,=", "info,x=warn=error"] {
        let error = build_filter(Some(text)).unwrap_err();

        let chain = format!("{error:#}");
        assert!(chain.contains("RUST_LOG"), "{chain}");
        assert!(chain.contains(text), "{chain}");
    }
}

/// ADR-0061 (c): a field value in a directive is literal text, not a regular expression.
///
/// The literal match compares the `Debug` text of a string value, so the directive quotes the
/// value. A regular expression would look for the quote characters in the plain value and
/// would match neither span.
#[test]
fn a_field_value_in_a_directive_matches_as_literal_text() {
    let directives = Some("info,[probe{key=\"a.c\"}]=debug");

    let lines = lines_of(directives, || {
        let regex_only = tracing::info_span!("probe", key = "abc");
        regex_only.in_scope(|| tracing::debug!("wrong span"));
        let literal = tracing::info_span!("probe", key = "a.c");
        literal.in_scope(|| tracing::debug!("right span"));
    });

    let text = lines.join("\n");
    assert_eq!(lines.len(), 1, "{text}");
    assert!(text.contains("right span"), "{text}");
}

#[test]
fn the_subscriber_writes_one_json_object_for_each_event() {
    let lines = lines_of(None, || {
        let span = tracing::info_span!("http.request", request_id = "req-1");
        span.in_scope(|| tracing::info!(count = 3, "a request"));
    });

    assert_eq!(lines.len(), 1, "{lines:?}");
    let line = &lines[0];
    assert!(line.starts_with('{') && line.ends_with('}'), "{line}");
    assert!(line.contains("\"level\":\"INFO\""), "{line}");
    assert!(line.contains("\"message\":\"a request\""), "{line}");
    assert!(line.contains("\"request_id\":\"req-1\""), "{line}");
    assert!(line.contains("\"spans\":["), "{line}");
}

/// Emit one event at TRACE, DEBUG, and INFO for each target that can log the webhook host.
///
/// `hyper-util` 0.1.21 logs the host, the address, the pool key, or the URI in these targets.
/// The target of a `tracing` event must be a literal, so a macro writes the calls.
macro_rules! emit_webhook_targets {
    ($($target:literal),+ $(,)?) => {
        $(
            tracing::trace!(target: $target, "trace line");
            tracing::debug!(target: $target, "debug line");
            tracing::info!(target: $target, "info line");
        )+
    };
}

fn emit_hyper_util_client_targets() {
    emit_webhook_targets!(
        "hyper_util::client::legacy::connect::http",
        "hyper_util::client::legacy::connect::dns",
        "hyper_util::client::legacy::pool",
        "hyper_util::client::legacy::client",
    );
}

/// ADR-0063 (f): no `RUST_LOG` value lowers the targets of the webhook client below `info`.
#[test]
fn no_variable_value_shows_the_webhook_host_in_the_client_targets() {
    let exact = "hyper_util::client::legacy::connect::http=trace,\
                 hyper_util::client::legacy::connect::dns=trace,\
                 hyper_util::client::legacy::pool=trace,\
                 hyper_util::client::legacy::client=trace";
    let values = [
        "hyper_util=trace",
        "hyper_util::client=trace",
        "hyper_util::client::legacy::connect=trace",
        "trace",
        "debug,hyper_util=debug",
        exact,
    ];
    for value in values {
        let lines = lines_of(Some(value), emit_hyper_util_client_targets);

        let text = lines.join("\n");
        assert_eq!(lines.len(), 4, "value: {value:?}, lines: {text}");
        assert!(
            lines.iter().all(|line| line.contains("info line")),
            "value: {value:?}, lines: {text}"
        );
    }
}

/// Run `emit` inside a span named `job`, the name of the span of the supervisor.
fn in_job_span(emit: impl FnOnce()) {
    let span = tracing::info_span!("job", job = "alerts");
    span.in_scope(emit);
}

/// A span or field directive of `RUST_LOG` must not lower a protected target below `info`.
///
/// The span `job` is the span of the supervisor. A directive such as `[job]=trace` enables every
/// event inside that span, and a filter of the target directives alone cannot stop it.
#[test]
fn no_variable_value_shows_the_webhook_host_inside_the_span_job() {
    let values = [
        "info,[job]=trace",
        "[job]=trace",
        "recipe_server::supervisor[job]=trace",
        "hyper_util=trace",
        "trace",
        "debug,hyper_util::client::legacy::connect::http[{host}]=trace",
        "info,hyper_util::client::legacy::connect::http[job]=trace",
        "info,[job{job=\"alerts\"}]=trace",
    ];
    for value in values {
        let lines = lines_of(Some(value), || in_job_span(emit_hyper_util_client_targets));

        let text = lines.join("\n");
        assert_eq!(lines.len(), 4, "value: {value:?}, lines: {text}");
        assert!(
            lines.iter().all(|line| line.contains("info line")),
            "value: {value:?}, lines: {text}"
        );
    }
}

#[test]
fn an_unrelated_target_keeps_the_level_of_the_variable_inside_the_span_job() {
    let cases = [
        ("info,[job]=trace", 6),
        ("[job]=trace", 6),
        ("info,hyper_util=trace", 4),
        ("info", 2),
    ];
    for (value, expected) in cases {
        let lines = lines_of(Some(value), || {
            in_job_span(|| {
                emit_webhook_targets!("hyper_util::server::graceful", "recipe_server");
            });
        });

        let text = lines.join("\n");
        assert_eq!(lines.len(), expected, "value: {value:?}, lines: {text}");
    }
}

#[test]
fn the_exact_target_of_the_connector_stays_at_info_for_hyper_util_trace() {
    let lines = lines_of(Some("hyper_util=trace"), || {
        emit_webhook_targets!("hyper_util::client::legacy::connect::http");
    });

    let text = lines.join("\n");
    assert_eq!(lines.len(), 1, "{text}");
    assert!(text.contains("info line"), "{text}");
}

#[test]
fn an_unrelated_target_keeps_the_level_of_the_variable() {
    let lines = lines_of(Some("info,hyper_util=trace"), || {
        emit_webhook_targets!("hyper_util::server::graceful", "recipe_server");
    });

    let text = lines.join("\n");
    assert_eq!(lines.len(), 4, "{text}");
    assert_eq!(text.matches("trace line").count(), 1, "{text}");
    assert_eq!(text.matches("debug line").count(), 1, "{text}");
}

#[test]
fn the_fallback_logger_writes_one_error_line() {
    let (sender, receiver) = mpsc::channel();

    fallback_error(
        move || ChannelWriter(sender.clone()),
        "RUST_LOG is not valid",
    );

    let bytes: Vec<u8> = receiver.try_iter().flatten().collect();
    let text = String::from_utf8(bytes).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 1, "{text}");
    assert!(lines[0].contains("\"level\":\"ERROR\""), "{text}");
    assert!(lines[0].contains("RUST_LOG is not valid"), "{text}");
}
