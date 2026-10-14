use std::net::IpAddr;
use std::time::Duration;

use super::*;

const RELIABILITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);
const SECONDS_PER_DAY: u64 = 86_400;

/// Return the value cell of the row `key` in the table of `reliability.md` section 7.11.
fn cell(key: &str) -> String {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    let prefix = format!("| `{key}` |");
    let row = document
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap();
    row.split('|').nth(2).unwrap().trim().to_owned()
}

fn bytes(key: &str) -> usize {
    const UNITS: [(&str, usize); 2] = [("KiB", 1024), ("MiB", 1024 * 1024)];
    let value = cell(key);
    let mut words = value.split_whitespace();
    let number: usize = words.next().unwrap().parse().unwrap();
    let name = words.next().unwrap();
    let (_name, factor) = UNITS.iter().find(|(unit, _factor)| *unit == name).unwrap();
    number.checked_mul(*factor).unwrap()
}

fn duration(key: &str) -> Duration {
    const UNITS: [(&str, u64); 3] = [
        ("s", 1),
        ("day", SECONDS_PER_DAY),
        ("days", SECONDS_PER_DAY),
    ];
    let value = cell(key);
    let mut words = value.split_whitespace();
    let number: u64 = words.next().unwrap().parse().unwrap();
    let name = words.next().unwrap().trim_end_matches(['.', ',']);
    let (_name, factor) = UNITS.iter().find(|(unit, _factor)| *unit == name).unwrap();
    Duration::from_secs(number.checked_mul(*factor).unwrap())
}

#[test]
fn default_body_limits_equal_the_table() {
    let settings = WebSettings::default();
    assert_eq!(settings.body_limit_form, bytes("http.body_limit.form"));
    assert_eq!(settings.body_limit_value, bytes("http.body_limit.value"));
    assert_eq!(
        settings.body_limit_translation,
        bytes("http.body_limit.translation")
    );
    assert_eq!(settings.body_limit_form, 256 * 1024);
}

#[test]
fn default_durations_equal_the_table() {
    let settings = WebSettings::default();
    assert_eq!(
        settings.static_cache_max_age,
        duration("static.cache_max_age")
    );
    assert_eq!(settings.hsts_max_age, duration("http.hsts_max_age"));
    assert_eq!(settings.retry_after, duration("http.retry_after"));
    assert_eq!(
        settings.lang_cookie_max_age,
        duration("i18n.lang_cookie_max_age")
    );
}

#[test]
fn default_trusted_proxies_is_the_compose_subnet() {
    let settings = WebSettings::default();
    let inside: IpAddr = "172.28.0.2".parse().unwrap();
    let outside: IpAddr = "203.0.113.9".parse().unwrap();
    assert!(settings.trusted_proxies.contains(inside));
    assert!(!settings.trusted_proxies.contains(outside));
}

#[test]
fn debug_output_hides_the_fields() {
    let shown = format!("{:?}", WebSettings::default());
    assert_eq!(shown, "WebSettings { .. }");
}
