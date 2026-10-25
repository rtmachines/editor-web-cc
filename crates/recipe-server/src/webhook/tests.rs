//! Tests of the webhook adapter (design D14 row T27, ADR-0063).
//!
//! The tests with a real TLS socket cannot use paused time, because the runtime would skip the
//! timeout while the socket still works. They use short timeouts of 0.3 s to 1 s. The tests of
//! `run_bounded` use paused time and no socket.

mod cancel;
mod pki;
mod server;
mod shared_timeout;
mod span;

use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use hyper::Uri;
use recipe_app::ports::{AlertCondition, AlertWebhook, WebhookFault};
use tokio::time::Instant;

use self::server::{Reply, Script, TlsServer, always, silent};
use super::{KEY_WEBHOOK_CA_FILE, NoWebhook, WEBHOOK_TIMEOUT, WebhookClient, run_bounded};
use crate::config::webhook::parse_webhook_text;

/// A recognisable token in the path of the test URL.
const SECRET_PATH: &str = "/hooks/TOKEN-9f3c1a7e5b";

/// A recognisable token in the query of the test URL.
const SECRET_QUERY: &str = "key=QUERY-4d2b8c";

/// A timeout that is long enough for a local call on a loaded machine.
const AMPLE: Duration = Duration::from_secs(5);

type Call = Pin<Box<dyn Future<Output = Result<(), WebhookFault>> + Send>>;

static NEXT_NUMBER: AtomicU32 = AtomicU32::new(0);

/// A temporary directory of one test. The drop removes it.
struct Sandbox(PathBuf);

impl Sandbox {
    fn new() -> Self {
        let number = NEXT_NUMBER.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "recipe-server-webhook-{}-{number}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Build a client for the server with the secret path and query. `ca` is the text of a CA file.
fn client_for(server: &TlsServer, ca: Option<&str>, timeout: Duration) -> WebhookClient {
    let text = server.url(&format!("{SECRET_PATH}?{SECRET_QUERY}"));
    let url = parse_webhook_text(&text).unwrap().unwrap();
    let sandbox = Sandbox::new();
    let ca_path = ca.map(|pem| sandbox.write("ca.pem", pem));
    WebhookClient::new(&url, ca_path.as_deref(), timeout).unwrap()
}

/// Send one run to the server and return the results.
async fn send(
    server: &TlsServer,
    ca: Option<&str>,
    timeout: Duration,
    conditions: &[AlertCondition],
) -> Vec<Result<(), WebhookFault>> {
    client_for(server, ca, timeout).send_all(conditions).await
}

fn assert_holds_no_url_part(text: &str, address: std::net::SocketAddr) {
    let port = address.port().to_string();
    for part in [
        "TOKEN-9f3c1a7e5b",
        "QUERY-4d2b8c",
        "hooks",
        "127.0.0.1",
        &port,
    ] {
        assert!(!text.contains(part), "`{text}` holds the URL part `{part}`");
    }
}

#[test]
fn webhook_timeout_default_equals_the_row_of_the_table_of_numbers() {
    // Arrange, act, assert: reliability.md section 7.11 gives 10 s for `alerts.webhook_timeout`.
    assert_eq!(WEBHOOK_TIMEOUT, Duration::from_secs(10));
}

#[tokio::test]
async fn webhook_posts_the_alert_name_as_json_to_a_tls_listener_with_a_test_ca() {
    let server = TlsServer::start(always(204));

    let results = send(&server, Some(pki::CA_A), AMPLE, &[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Ok(())]);
    let seen = server.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].method, "POST");
    assert_eq!(seen[0].target, format!("{SECRET_PATH}?{SECRET_QUERY}"));
    assert_eq!(seen[0].content_type.as_deref(), Some("application/json"));
    assert_eq!(seen[0].body, r#"{"alert":"panics"}"#);
}

#[tokio::test]
async fn webhook_sends_one_post_for_each_condition_and_answers_in_the_order_of_the_argument() {
    let script: Script = std::sync::Arc::new(|seen| {
        if seen.body.contains("panics") {
            Reply::Status(500)
        } else {
            Reply::Status(204)
        }
    });
    let server = TlsServer::start(script);
    let conditions = [
        AlertCondition::Panics,
        AlertCondition::LowDisk,
        AlertCondition::Panics,
    ];

    let results = send(&server, Some(pki::CA_A), AMPLE, &conditions).await;

    let failure = Err(WebhookFault::Status { code: 500 });
    assert_eq!(results, vec![failure, Ok(()), failure]);
    assert_eq!(server.seen().len(), 3);
}

#[tokio::test]
async fn webhook_gives_no_result_and_no_connection_for_no_condition() {
    let server = TlsServer::start(always(204));

    let results = send(&server, Some(pki::CA_A), AMPLE, &[]).await;

    assert!(results.is_empty());
    assert_eq!(server.accepted(), 0);
}

#[tokio::test]
async fn webhook_refuses_an_http_url_without_a_connection_and_without_url_text() {
    let server = TlsServer::start(always(204));
    let uri: Uri = format!("http://{}{SECRET_PATH}?{SECRET_QUERY}", server.address())
        .parse()
        .unwrap();
    let client = WebhookClient::from_uri(uri, None, AMPLE).unwrap();

    let results = client.send_all(&[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Err(WebhookFault::Connection)]);
    assert_eq!(server.accepted(), 0, "https_only must stop the call");
    let fault = results[0].unwrap_err();
    assert_holds_no_url_part(&format!("{fault} {fault:?}"), server.address());
}

#[tokio::test]
async fn webhook_gives_a_status_fault_for_a_status_outside_2xx_and_follows_no_redirect() {
    let moved: Script = std::sync::Arc::new(|seen| {
        if seen.target.starts_with("/moved") {
            Reply::Status(200)
        } else {
            Reply::Redirect("/moved".to_owned())
        }
    });
    let cases = [
        (always(500), Err(WebhookFault::Status { code: 500 })),
        (always(404), Err(WebhookFault::Status { code: 404 })),
        (always(204), Ok(())),
        (moved, Err(WebhookFault::Status { code: 301 })),
    ];
    for (script, expected) in cases {
        let server = TlsServer::start(script);

        let results = send(&server, Some(pki::CA_A), AMPLE, &[AlertCondition::LowDisk]).await;

        assert_eq!(results, vec![expected]);
        assert_eq!(
            server.seen().len(),
            1,
            "no second request after {expected:?}"
        );
    }
}

#[tokio::test]
async fn webhook_gives_a_connection_fault_when_nothing_listens() {
    let closed = TlsServer::start(always(204));
    let dead = closed.address();
    drop(closed);
    let uri: Uri = format!("https://{dead}{SECRET_PATH}").parse().unwrap();
    let client = WebhookClient::from_uri(uri, None, AMPLE).unwrap();

    let results = client.send_all(&[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Err(WebhookFault::Connection)]);
}

#[tokio::test]
async fn webhook_gives_a_tls_fault_for_an_unknown_ca() {
    let server = TlsServer::start(always(204));

    let results = send(&server, Some(pki::CA_B), AMPLE, &[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Err(WebhookFault::Tls)]);
    assert!(server.seen().is_empty());
}

#[tokio::test]
async fn webhook_without_a_ca_file_trusts_the_webpki_roots_and_not_the_test_ca() {
    let server = TlsServer::start(always(204));

    let results = send(&server, None, AMPLE, &[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Err(WebhookFault::Tls)]);
    assert!(server.seen().is_empty());
}

#[tokio::test]
async fn webhook_gives_a_timeout_fault_when_the_host_stays_silent() {
    let server = TlsServer::start(silent(Duration::from_secs(3)));
    let timeout = Duration::from_millis(300);
    let started = std::time::Instant::now();

    let results = send(&server, Some(pki::CA_A), timeout, &[AlertCondition::Panics]).await;

    assert_eq!(results, vec![Err(WebhookFault::Timeout)]);
    assert!(started.elapsed() >= timeout);
    assert!(started.elapsed() < Duration::from_secs(2));
}

#[tokio::test]
async fn webhook_ten_slow_calls_end_within_about_one_timeout_and_run_at_the_same_time() {
    let server = TlsServer::start(silent(Duration::from_secs(30)));
    let timeout = Duration::from_secs(3);
    let started = std::time::Instant::now();

    let results = send(&server, Some(pki::CA_A), timeout, &AlertCondition::ALL).await;

    assert_eq!(results, vec![Err(WebhookFault::Timeout); 10]);
    assert!(started.elapsed() >= timeout);
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "ten calls in sequence would take 10 s"
    );
    assert_eq!(server.peak(), 10, "all ten requests were open together");
}

#[tokio::test]
async fn webhook_keeps_the_answers_that_arrive_before_the_timeout() {
    let script: Script = std::sync::Arc::new(|seen| {
        if seen.body.contains("panics") {
            Reply::Silence(Duration::from_secs(5))
        } else {
            Reply::Status(204)
        }
    });
    let server = TlsServer::start(script);
    let conditions = [AlertCondition::Panics, AlertCondition::LowDisk];

    let results = send(
        &server,
        Some(pki::CA_A),
        Duration::from_secs(1),
        &conditions,
    )
    .await;

    assert_eq!(results, vec![Err(WebhookFault::Timeout), Ok(())]);
}

#[tokio::test]
async fn webhook_fault_text_and_client_debug_hold_no_part_of_the_url() {
    let silent_server = TlsServer::start(silent(Duration::from_secs(3)));
    let faulty = TlsServer::start(always(500));
    let timeout = Duration::from_millis(300);
    let mut texts = Vec::new();
    for (server, ca) in [
        (&silent_server, Some(pki::CA_A)),
        (&faulty, Some(pki::CA_A)),
        (&faulty, Some(pki::CA_B)),
    ] {
        let client = client_for(server, ca, timeout);
        texts.push(format!("{client:?}"));
        for result in client.send_all(&[AlertCondition::Panics]).await {
            let fault = result.unwrap_err();
            texts.push(format!("{fault} | {fault:?}"));
        }
    }

    assert_eq!(texts.len(), 6);
    for text in &texts {
        assert_holds_no_url_part(text, silent_server.address());
        assert_holds_no_url_part(text, faulty.address());
    }
}

#[test]
fn webhook_url_holder_debug_hides_the_url() {
    let text = format!("https://127.0.0.1:9{SECRET_PATH}?{SECRET_QUERY}");
    let url = parse_webhook_text(&text).unwrap().unwrap();

    let shown = format!("{url:?}");

    for part in ["TOKEN-9f3c1a7e5b", "QUERY-4d2b8c", "hooks", "127.0.0.1"] {
        assert!(!shown.contains(part), "`{shown}` holds `{part}`");
    }
}

/// The error text of a refused CA file in the two forms that a start can print.
fn ca_refusal(path: &Path) -> String {
    let url = parse_webhook_text("https://127.0.0.1:9/x")
        .unwrap()
        .unwrap();
    let error = WebhookClient::new(&url, Some(path), AMPLE).unwrap_err();
    format!("{error} | {error:#} | {error:?}")
}

#[test]
fn webhook_refuses_a_ca_file_without_a_certificate_and_names_the_key_only() {
    let sandbox = Sandbox::new();
    let cases = [
        ("empty-ca-file.pem", ""),
        ("text-ca-file.pem", "no certificate here\n"),
        ("key-ca-file.pem", pki::LEAF_KEY),
        (
            "bad-ca-file.pem",
            "-----BEGIN CERTIFICATE-----\nAAAA\n-----END CERTIFICATE-----\n",
        ),
    ];
    for (name, text) in cases {
        let path = sandbox.write(name, text);

        let shown = ca_refusal(&path);

        assert!(shown.contains(KEY_WEBHOOK_CA_FILE), "{name}: {shown}");
        assert!(
            !shown.contains(name),
            "{name}: the file name leaked: {shown}"
        );
        assert!(
            !shown.contains(&*path.to_string_lossy()),
            "{name}: the path leaked: {shown}"
        );
        assert!(!shown.contains("recipe-server-webhook"), "{name}: {shown}");
    }
}

#[test]
fn webhook_refuses_a_missing_ca_file_and_names_the_key_only() {
    let sandbox = Sandbox::new();
    let path = sandbox.0.join("absent-ca-file.pem");

    let shown = ca_refusal(&path);

    assert!(shown.contains(KEY_WEBHOOK_CA_FILE), "{shown}");
    assert!(!shown.contains("absent-ca-file"), "{shown}");
    assert!(!shown.contains(&*path.to_string_lossy()), "{shown}");
}

#[test]
fn webhook_accepts_a_ca_file_with_a_certificate_among_other_sections() {
    let sandbox = Sandbox::new();
    let text = format!("a note\n{}{}", pki::LEAF_KEY, pki::CA_A);
    let path = sandbox.write("mixed-ca-file.pem", &text);
    let url = parse_webhook_text("https://127.0.0.1:9/x")
        .unwrap()
        .unwrap();

    assert!(WebhookClient::new(&url, Some(&path), AMPLE).is_ok());
}

#[tokio::test]
async fn webhook_no_webhook_adapter_answers_ok_for_every_condition() {
    let results = NoWebhook.send_all(&AlertCondition::ALL).await;

    assert_eq!(results, vec![Ok(()); 10]);
    assert!(NoWebhook.send_all(&[]).await.is_empty());
}

fn after(delay: Duration, result: Result<(), WebhookFault>) -> Call {
    Box::pin(async move {
        tokio::time::sleep(delay).await;
        result
    })
}

#[tokio::test(start_paused = true)]
async fn run_bounded_runs_all_calls_at_the_same_time() {
    let calls: Vec<Call> = (0..10)
        .map(|_| after(Duration::from_secs(4), Ok(())))
        .collect();
    let started = Instant::now();

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Ok(()); 10]);
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "calls ran in sequence"
    );
}

#[tokio::test(start_paused = true)]
async fn run_bounded_ends_all_open_calls_at_one_timeout() {
    let calls: Vec<Call> = (0..10)
        .map(|_| after(Duration::from_secs(60), Ok(())))
        .collect();
    let started = Instant::now();

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Err(WebhookFault::Timeout); 10]);
    assert!(started.elapsed() >= Duration::from_secs(10));
    assert!(started.elapsed() < Duration::from_secs(11));
}

#[tokio::test(start_paused = true)]
async fn run_bounded_keeps_finished_answers_and_marks_the_rest_as_timeout() {
    let calls: Vec<Call> = vec![
        after(Duration::from_secs(1), Ok(())),
        after(Duration::from_secs(60), Ok(())),
        after(Duration::ZERO, Err(WebhookFault::Status { code: 500 })),
    ];

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(
        results,
        vec![
            Ok(()),
            Err(WebhookFault::Timeout),
            Err(WebhookFault::Status { code: 500 })
        ]
    );
}

#[tokio::test(start_paused = true)]
async fn run_bounded_answers_in_the_order_of_the_argument() {
    let calls: Vec<Call> = vec![
        after(
            Duration::from_secs(3),
            Err(WebhookFault::Status { code: 301 }),
        ),
        after(
            Duration::from_secs(1),
            Err(WebhookFault::Status { code: 302 }),
        ),
        after(
            Duration::from_secs(2),
            Err(WebhookFault::Status { code: 303 }),
        ),
    ];

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    let codes: Vec<_> = results
        .into_iter()
        .map(|result| result.unwrap_err())
        .collect();
    assert_eq!(
        codes,
        vec![
            WebhookFault::Status { code: 301 },
            WebhookFault::Status { code: 302 },
            WebhookFault::Status { code: 303 }
        ]
    );
}

fn broken_step() -> Result<(), WebhookFault> {
    Err(WebhookFault::Timeout)
}

#[tokio::test(start_paused = true)]
async fn run_bounded_turns_a_panic_into_the_fault_of_that_entry_only() {
    let panicking: Call = Box::pin(async {
        // The unwrap panics, as a bug in a call would.
        broken_step().unwrap();
        Ok(())
    });
    let calls: Vec<Call> = vec![
        after(Duration::from_secs(1), Ok(())),
        panicking,
        after(Duration::from_secs(2), Ok(())),
    ];

    let results = run_bounded(calls, Duration::from_secs(10)).await;

    assert_eq!(results, vec![Ok(()), Err(WebhookFault::Connection), Ok(())]);
}

#[tokio::test(start_paused = true)]
async fn run_bounded_gives_an_empty_list_for_no_call() {
    let results = run_bounded(Vec::<Call>::new(), Duration::from_secs(10)).await;

    assert!(results.is_empty());
}
