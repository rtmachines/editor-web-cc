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

/// Return the language tags of the row `i18n.translation_langs`: the codes in backticks before
/// the first full stop, separated by commas.
fn tags(key: &str) -> Vec<String> {
    let value = cell(key);
    let list = value.split(". ").next().unwrap_or_default();
    list.split(',')
        .map(|tag| {
            tag.trim()
                .trim_matches('`')
                .trim_end_matches('.')
                .to_owned()
        })
        .collect()
}

#[test]
fn default_translation_langs_equal_the_table() {
    let settings = WebSettings::default();
    let listed: Vec<String> = settings
        .translation_langs
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(listed, tags("i18n.translation_langs"));
    assert_eq!(listed, ["pl"]);
}

#[test]
fn default_array_chunk_equals_the_table() {
    let value = cell("form.array_chunk");
    let number: usize = value.split_whitespace().next().unwrap().parse().unwrap();
    assert_eq!(WebSettings::default().form_array_chunk, number);
    assert_eq!(number, 100);
}

#[test]
fn default_rebind_leaves_equals_the_table() {
    // The table writes "16 384", so the digit groups before the first word form the number.
    let value = cell("form.rebind_leaves");
    let digits: String = value
        .split_whitespace()
        .take_while(|word| word.chars().all(|c| c.is_ascii_digit()))
        .collect();
    let number: usize = digits.parse().unwrap();
    assert_eq!(WebSettings::default().form_rebind_leaves, number);
    assert_eq!(number, 16_384);
}

#[test]
fn translation_langs_hold_no_en_and_no_duplicate() {
    let settings = WebSettings::default();
    let listed: Vec<&str> = settings
        .translation_langs
        .iter()
        .map(recipe_domain::recipe::LangCode::as_str)
        .collect();
    assert!(!listed.contains(&"en"));
    let mut sorted = listed.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), listed.len());
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
    assert_eq!(settings.request_timeout, duration("http.request_timeout"));
    assert_eq!(
        settings.body_read_timeout,
        duration("http.body_read_timeout")
    );
    assert_eq!(
        settings.lang_cookie_max_age,
        duration("i18n.lang_cookie_max_age")
    );
}

#[test]
fn default_trusted_proxies_is_the_one_address_of_the_proxy() {
    let settings = WebSettings::default();
    let proxy: IpAddr = "172.28.0.2".parse().unwrap();
    let neighbor: IpAddr = "172.28.0.3".parse().unwrap();
    let outside: IpAddr = "203.0.113.9".parse().unwrap();
    assert!(settings.trusted_proxies.contains(proxy));
    assert!(!settings.trusted_proxies.contains(neighbor));
    assert!(!settings.trusted_proxies.contains(outside));
}

#[test]
fn debug_output_hides_the_fields() {
    let shown = format!("{:?}", WebSettings::default());
    assert_eq!(shown, "WebSettings { .. }");
}
