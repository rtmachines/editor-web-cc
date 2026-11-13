use crate::canon::{Bare, Canon, canon};
use crate::fault::Report;
use crate::lines::{code_lines, unquote};
use crate::table::Table;

#[test]
fn caddy_http_export_paths_return_403_without_a_redirect() {
    let inputs = crate::inputs::Inputs::load();
    for file in ["deploy/Caddyfile", "deploy/Caddyfile.dev"] {
        let lines = code_lines(inputs.text(file));
        let site: Vec<_> = lines
            .iter()
            .skip_while(|line| line.trim() != "http://{$SITE_ADDRESS} {")
            .skip(1)
            .take_while(|line| **line != "}")
            .map(|line| line.trim())
            .collect();
        // Exclusive handles prevent Caddy from sorting the redirect before the refusal.
        assert_eq!(
            site,
            [
                "import default_headers",
                "@export path /export/*",
                "handle @export {",
                "respond 403",
                "}",
                "handle {",
                "redir https://{host}{uri} 308",
                "}",
            ],
            "{file}: HTTP must refuse every /export/ path before the fallback redirect"
        );
    }
}

#[test]
fn caddyfiles_never_enable_credential_logging() {
    let inputs = crate::inputs::Inputs::load();
    for file in ["deploy/Caddyfile", "deploy/Caddyfile.dev"] {
        assert!(
            !inputs.text(file).contains("log_credentials"),
            "{file}: credential logging must stay disabled"
        );
    }
}

/// The upstream keep-alive, the HSTS age, and the port of `reverse_proxy` of a Caddyfile
/// (`reliability.md` 7.11 rows `proxy.upstream_keepalive`, `http.hsts_max_age`, `http.public_port`).
pub fn check_caddyfile(file: &str, text: &str, table: &Table) -> Report {
    let mut report = Report::default();
    let lines = code_lines(text);
    let keepalive = lines
        .iter()
        .find_map(|line| line.trim().strip_prefix("keepalive "));
    report.cover("proxy.upstream_keepalive");
    report.compare(
        file,
        "keepalive (proxy.upstream_keepalive)",
        table.expected("proxy.upstream_keepalive"),
        keepalive,
        Bare::Plain,
    );
    if let Some(fault) = keepalive_below_body_read(keepalive, table) {
        report.fail(
            file,
            "keepalive (proxy.upstream_keepalive)",
            fault,
            keepalive.unwrap_or("missing"),
        );
    }
    check_hsts(&mut report, file, &lines, table);
    let target = lines
        .iter()
        .find_map(|line| line.trim().strip_prefix("reverse_proxy app:"));
    let port = target.and_then(|rest| rest.split_whitespace().next());
    report.cover("http.public_port");
    report.compare(
        file,
        "reverse_proxy port (http.public_port)",
        table.expected("http.public_port"),
        port,
        Bare::Plain,
    );
    report
}

/// The directives of the line `header ?Strict-Transport-Security "<directives>"`. The header name
/// must be the whole name, with or without the mark `?` of Caddy.
fn hsts_directives(lines: &[&str]) -> Vec<String> {
    let value = lines.iter().find_map(|line| {
        let rest = line.trim().strip_prefix("header ")?;
        let (name, value) = rest.split_once(' ')?;
        matches!(
            name,
            "?Strict-Transport-Security" | "Strict-Transport-Security"
        )
        .then_some(value)
    });
    value
        .map(unquote)
        .unwrap_or_default()
        .split(';')
        .map(|directive| directive.trim().to_owned())
        .collect()
}

/// The `max-age` of the HSTS header equals `http.hsts_max_age`, and the header has `includeSubDomains`.
fn check_hsts(report: &mut Report, file: &str, lines: &[&str], table: &Table) {
    let directives = hsts_directives(lines);
    let age = directives
        .iter()
        .find_map(|directive| directive.strip_prefix("max-age="));
    report.cover("http.hsts_max_age");
    report.compare(
        file,
        "Strict-Transport-Security max-age (http.hsts_max_age)",
        table.expected("http.hsts_max_age"),
        age,
        Bare::Seconds,
    );
    if !directives
        .iter()
        .any(|directive| directive == "includeSubDomains")
    {
        report.fail(
            file,
            "Strict-Transport-Security",
            "includeSubDomains",
            "missing",
        );
    }
}

/// The rule `proxy.upstream_keepalive` is below `http.body_read_timeout` (`reliability.md` 7.1 and 7.11).
/// Returns the text of the fault, or `None` when the rule holds or the keep-alive is missing.
fn keepalive_below_body_read(keepalive: Option<&str>, table: &Table) -> Option<String> {
    let Some(Canon::Millis(keep)) = keepalive.and_then(|text| canon(text, Bare::Plain)) else {
        return None;
    };
    let body = table.millis("http.body_read_timeout")?;
    (keep >= body).then(|| format!("below {body} ms (http.body_read_timeout)"))
}
