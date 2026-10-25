//! The check of the Compose override files (design D9, finding VX10-3).
//!
//! The reader of `lines.rs` cannot merge a file into `compose.yaml`. So the rule is stricter than a
//! merge: an override may hold only the keys that D9 names. Any other key is a controlled setting
//! (limits, ports, timeouts, log options, healthcheck values, networks, secrets) and gives a fault.

use crate::fault::Report;
use crate::lines::{code_lines, indent_of, without_comment};

const SERVICES_KEY: &str = "services";
const SERVICE_INDENT: usize = 2;
const KEY_INDENT: usize = 4;

/// What one override file may hold under `services:`.
pub struct Allowed {
    /// The services that the file may name, or `None` for every service.
    pub services: Option<&'static [&'static str]>,
    /// The keys that a service of the file may hold.
    pub keys: &'static [&'static str],
}

/// `compose.dev.yaml` changes the port, the volumes, and the secrets of `proxy`.
pub const DEV: Allowed = Allowed {
    services: Some(&["proxy"]),
    keys: &["ports", "volumes", "secrets"],
};

/// `compose.build.yaml` adds `build:` to the services of an image of the project.
pub const BUILD: Allowed = Allowed {
    services: None,
    keys: &["build"],
};

fn key_of(text: &str) -> &str {
    text.split(':').next().unwrap_or(text).trim()
}

fn is_top_level_allowed(key: &str) -> bool {
    key == "name" || key == SERVICES_KEY || key.starts_with("x-")
}

/// Check the lines below `services:`, one line at a time.
fn check_service_line(
    report: &mut Report,
    path: &str,
    allowed: &Allowed,
    service: &mut String,
    line: &str,
) {
    let level = indent_of(line);
    let text = line.trim();
    if level == SERVICE_INDENT {
        if !text.ends_with(':') {
            report.fail(
                path,
                text,
                "a service block that ends in a colon",
                "unreadable",
            );
        }
        *service = key_of(text).to_owned();
        let known = allowed
            .services
            .is_none_or(|names| names.contains(&service.as_str()));
        if !known {
            report.fail(path, service.as_str(), "no override of this service", text);
        }
    } else if level == KEY_INDENT && !text.starts_with("- ") {
        let key = key_of(text);
        if !allowed.keys.contains(&key) {
            let expected = format!("only {} in an override", allowed.keys.join(", "));
            report.fail(path, &format!("{service}.{key}"), expected, text);
        }
    } else if level % 2 == 1 {
        report.fail(
            path,
            text,
            "an indent of two spaces for each level",
            "unreadable",
        );
    }
}

/// Report each key of an override file that the rule does not allow.
pub fn check_override(path: &str, text: &str, allowed: &Allowed) -> Report {
    let mut report = Report::default();
    let mut top = String::new();
    let mut service = String::new();
    for raw in code_lines(text) {
        let line = without_comment(raw).trim_end();
        if indent_of(line) == 0 {
            top = key_of(line).to_owned();
            if !is_top_level_allowed(&top) {
                report.fail(path, &top, "no top-level section in an override", line);
            }
            let rest = line.split_once(':').map_or("", |(_, rest)| rest.trim());
            if top == SERVICES_KEY && !rest.is_empty() {
                report.fail(path, &top, "a block of services", "unreadable");
            }
        } else if top == SERVICES_KEY {
            check_service_line(&mut report, path, allowed, &mut service, line);
        }
    }
    report
}
