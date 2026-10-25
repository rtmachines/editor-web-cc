//! The log filter, the JSON subscriber, and the fallback logger (`reliability.md` section 3.1).
//!
//! The server writes one JSON object per line to stdout. Docker collects the lines. The filter
//! comes from the variable `RUST_LOG` (ADR-0061 (c), (d)). A bad directive stops the start. The
//! caller then writes one `ERROR` line through the fallback logger and returns the error, so the
//! process exits with code 1.
//!
//! A line holds the current span and the list of all open spans. The spans of
//! `reliability.md` section 3.2 therefore give their fields, for example `request_id`, to every
//! event inside them.

use std::env::VarError;

use anyhow::{Context, Result, anyhow, bail};
use tracing::{Level, Metadata, Subscriber};
use tracing_subscriber::filter::{FilterExt, filter_fn};
use tracing_subscriber::fmt::MakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer};

/// The filter text of a `RUST_LOG` without a directive (ADR-0080 (a)).
pub const DEFAULT_FILTER: &str = "info,sqlx=warn,hyper=warn";

/// The name of the environment variable that holds the filter directives.
const FILTER_VARIABLE: &str = "RUST_LOG";

/// The targets of `hyper-util` that log the webhook host, the address, the pool key, or the URI.
///
/// The webhook URL is a secret (ADR-0063 (f)). A target matches when it starts with one entry.
/// The first entry covers every module of `connect`. `subscriber` rejects each event and span of
/// these targets below `info`, whatever the text of `RUST_LOG` is.
const WEBHOOK_PROTECTED_TARGETS: [&str; 3] = [
    "hyper_util::client::legacy::connect",
    "hyper_util::client::legacy::pool",
    "hyper_util::client::legacy::client",
];

/// The directives that keep the events of the protected targets at `info` and above.
///
/// `build_filter` appends them after the pieces of `RUST_LOG`. A directive for the same target
/// replaces an earlier one, so a lower level of `RUST_LOG` or a missing directive does not hide
/// an `info` event of a protected target. A longer target beats a shorter one, so the exact
/// targets are listed. These directives do not stop a span directive. `permits_webhook_targets`
/// stops it.
const WEBHOOK_INFO_DIRECTIVES: [&str; 5] = [
    "hyper_util::client::legacy::connect=info",
    "hyper_util::client::legacy::connect::http=info",
    "hyper_util::client::legacy::connect::dns=info",
    "hyper_util::client::legacy::pool=info",
    "hyper_util::client::legacy::client=info",
];

/// Tell whether the callsite described by `metadata` may reach the log.
///
/// The function solves one problem: a span directive of `RUST_LOG`, for example `[job]=trace`,
/// enables every event inside the span before a target directive can lower it. The function
/// ignores `RUST_LOG`. It rejects a protected target below `info` and permits all else.
fn permits_webhook_targets(metadata: &Metadata<'_>) -> bool {
    let is_protected = WEBHOOK_PROTECTED_TARGETS
        .iter()
        .any(|target| metadata.target().starts_with(target));
    !is_protected || *metadata.level() <= Level::INFO
}

/// Build the event filter from the text of `RUST_LOG`.
///
/// The function splits the text on commas, trims each piece, and drops the empty pieces. A text
/// without a piece gives `DEFAULT_FILTER`. That covers a missing, empty, or blank variable and a
/// text of commas. A field value in a directive is literal text and not a regular expression
/// (ADR-0061 (c)).
///
/// The function solves one problem: an operator value must not hide an `info` event of the
/// webhook client. It therefore appends `WEBHOOK_INFO_DIRECTIVES` after the pieces. The function
/// cannot stop a span directive such as `[job]=trace`. `subscriber` adds the mandatory filter
/// that stops it.
///
/// # Errors
///
/// Returns an error that names `RUST_LOG` and the text when a directive is not valid.
pub fn build_filter(directives: Option<&str>) -> Result<EnvFilter> {
    let pieces: Vec<&str> = directives
        .into_iter()
        .flat_map(|text| text.split(','))
        .map(str::trim)
        .filter(|piece| !piece.is_empty())
        .collect();
    let text = if pieces.is_empty() {
        DEFAULT_FILTER.to_owned()
    } else {
        pieces.join(",")
    };
    let floor = WEBHOOK_INFO_DIRECTIVES.join(",");
    EnvFilter::builder()
        .with_regex(false)
        .parse(format!("{text},{floor}"))
        .with_context(|| format!("{FILTER_VARIABLE} \"{text}\" is not a valid log filter"))
}

/// Build the JSON subscriber that writes to `writer` and keeps the events that `filter` permits.
///
/// The function solves two problems. The server and its tests must share one line format. Each
/// line holds the fields `timestamp`, `level`, `fields`, and `target`. The fields `span` and
/// `spans` occur only inside a span.
///
/// The webhook URL is a secret (ADR-0063 (f)), and the logs of `hyper-util` show its host. The
/// subscriber therefore joins `filter` with `WEBHOOK_PROTECTED_TARGETS` by a logical and. No
/// `RUST_LOG` value, with or without a span or field directive, shows an event of those targets
/// below `info`.
pub fn subscriber<W>(filter: EnvFilter, writer: W) -> impl Subscriber + Send + Sync + 'static
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_current_span(true)
        .with_span_list(true)
        .with_writer(writer)
        .with_filter(filter.and(filter_fn(permits_webhook_targets)));
    tracing_subscriber::registry().with(layer)
}

/// Install the JSON subscriber with the filter of `RUST_LOG` as the global default.
///
/// Output goes to stdout.
///
/// # Errors
///
/// Returns an error when `RUST_LOG` is not valid UTF-8, when a directive is not valid, or when
/// another subscriber is already installed.
pub fn init_logging() -> Result<()> {
    let value = match std::env::var(FILTER_VARIABLE) {
        Ok(value) => Some(value),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => bail!("{FILTER_VARIABLE} is not valid UTF-8"),
    };
    let filter = build_filter(value.as_deref())?;
    // `try_init` also forwards the records of the `log` crate to `tracing` (feature `tracing-log`).
    subscriber(filter, std::io::stdout)
        .try_init()
        .map_err(|error| anyhow!("cannot start the logger: {error}"))
}

/// Write one `ERROR` line to `writer` with a subscriber that keeps `ERROR` and higher.
///
/// The function solves one problem: when `init_logging` fails, no global subscriber exists, and
/// the failure must still give exactly one JSON line. The subscriber lives only for this call.
pub fn fallback_error<W>(writer: W, message: &str)
where
    W: for<'writer> MakeWriter<'writer> + Send + Sync + 'static,
{
    let fallback = tracing_subscriber::fmt()
        .json()
        .with_max_level(Level::ERROR)
        .with_writer(writer)
        .finish();
    tracing::subscriber::with_default(fallback, || {
        tracing::error!(error = message, "the logger cannot start");
    });
}

#[cfg(test)]
mod tests;
