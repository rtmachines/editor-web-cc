//! Caddyfile checks: the keepalive, HSTS, and upstream port lines of both Caddyfiles.

use crate::inputs::{Inputs, Mutation, assert_mutations_fail, check_all};

const CADDYFILE: [Mutation; 10] = [
    // The header name and the directive must be whole words.
    (
        "Caddyfile",
        "?Strict-Transport-Security",
        "?X-Strict-Transport-Security",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "includeSubDomains",
        "xincludeSubDomains",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "keepalive 9s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "keepalive 11s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "keepalive 5s",
        "tcp_keepalive 5s",
        "proxy.upstream_keepalive",
    ),
    (
        "Caddyfile",
        "max-age=31536000",
        "max-age=100",
        "http.hsts_max_age",
    ),
    (
        "Caddyfile",
        "includeSubDomains",
        "preload",
        "Strict-Transport-Security",
    ),
    (
        "Caddyfile",
        "reverse_proxy app:8080",
        "reverse_proxy app:8081",
        "http.public_port",
    ),
    (
        "Caddyfile.dev",
        "keepalive 5s",
        "keepalive 9s",
        "Caddyfile.dev",
    ),
    (
        "Caddyfile.dev",
        "reverse_proxy app:8080",
        "reverse_proxy app:8081",
        "Caddyfile.dev",
    ),
];

#[test]
fn a_changed_keepalive_hsts_or_upstream_port_line_is_a_fault() {
    assert_mutations_fail(&CADDYFILE);
}

#[test]
fn a_keepalive_that_equals_the_table_but_reaches_the_body_read_time_is_a_fault() {
    // Rule: proxy.upstream_keepalive is below http.body_read_timeout (reliability.md 7.1, ADR-0065 (i)).
    let table_row = "| `proxy.upstream_keepalive` | 5 s.";
    let base = Inputs::load();
    let changed = base
        .with_replaced(
            "reliability.md",
            table_row,
            "| `proxy.upstream_keepalive` | 10 s.",
        )
        .with_replaced("Caddyfile", "keepalive 5s", "keepalive 10s");
    let report = check_all(&changed);
    assert!(
        report.has_fault_for("proxy.upstream_keepalive"),
        "{}",
        report.message()
    );
    assert!(
        report.message().contains("http.body_read_timeout"),
        "{}",
        report.message()
    );
}
