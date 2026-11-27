use std::net::IpAddr;
use std::time::Duration;

use recipe_domain::plant::{IecLiteral, IecLiteralReason, PlcPlatform, Primitive, TimeType};

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
fn limit_text_max_equals_the_table() {
    let value = cell("form.limit_text_max");
    let mut words = value.split_whitespace();
    let number: usize = words.next().unwrap().parse().unwrap();
    assert_eq!(
        words.next().map(|word| word.trim_end_matches('.')),
        Some("bytes")
    );
    assert_eq!(LIMIT_TEXT_MAX, number);
    assert_eq!(number, 256);
}

/// Digits of one part of a duration literal, as the grammar allows (`digits := [0-9]{1,19}`).
const DURATION_PART_DIGITS: usize = 19;

/// A duration type: its long prefix, whether it takes a sign, and its units, largest first.
struct DurationShape {
    primitive: Primitive,
    prefix: &'static str,
    signed: bool,
    units: &'static [&'static str],
}

const DURATIONS: [DurationShape; 3] = [
    DurationShape {
        primitive: Primitive::Time,
        prefix: "TIME",
        signed: true,
        units: &["d", "h", "m", "s", "ms"],
    },
    DurationShape {
        primitive: Primitive::Ltime,
        prefix: "LTIME",
        signed: true,
        units: &["d", "h", "m", "s", "ms", "us", "ns"],
    },
    DurationShape {
        primitive: Primitive::S5Time,
        prefix: "S5T",
        signed: false,
        units: &["h", "m", "s", "ms"],
    },
];

/// An instant type: its long prefix, its fields, and its longest fraction (`instant.rs:114-120`).
struct InstantShape {
    primitive: Primitive,
    prefix: &'static str,
    has_date: bool,
    has_clock: bool,
    fraction: usize,
}

const INSTANTS: [InstantShape; 7] = [
    InstantShape {
        primitive: Primitive::Date,
        prefix: "DATE",
        has_date: true,
        has_clock: false,
        fraction: 0,
    },
    InstantShape {
        primitive: Primitive::Ldate,
        prefix: "LDATE",
        has_date: true,
        has_clock: false,
        fraction: 0,
    },
    InstantShape {
        primitive: Primitive::TimeOfDay,
        prefix: "TIME_OF_DAY",
        has_date: false,
        has_clock: true,
        fraction: 3,
    },
    InstantShape {
        primitive: Primitive::Ltod,
        prefix: "LTIME_OF_DAY",
        has_date: false,
        has_clock: true,
        fraction: 9,
    },
    InstantShape {
        primitive: Primitive::DateAndTime,
        prefix: "DATE_AND_TIME",
        has_date: true,
        has_clock: true,
        fraction: 0,
    },
    InstantShape {
        primitive: Primitive::Ldt,
        prefix: "LDATE_AND_TIME",
        has_date: true,
        has_clock: true,
        fraction: 9,
    },
    InstantShape {
        primitive: Primitive::Dtl,
        prefix: "DTL",
        has_date: true,
        has_clock: true,
        fraction: 9,
    },
];

/// Build the longest duration literal of a type. The first part has `first_digits` zeros and each
/// other part has the 19 zeros of the grammar.
fn duration_literal(shape: &DurationShape, first_digits: usize) -> String {
    let parts: String = shape
        .units
        .iter()
        .enumerate()
        .map(|(position, unit)| {
            let digits = if position == 0 {
                first_digits
            } else {
                DURATION_PART_DIGITS
            };
            format!("{}{unit}", "0".repeat(digits))
        })
        .collect();
    let sign = if shape.signed { "-" } else { "" };
    format!("{}#{sign}{parts}", shape.prefix)
}

/// Build the longest instant literal of a type with a fraction of `fraction` digits. Each field of
/// an instant has a fixed width, so the date and the clock have no padding.
fn instant_literal(shape: &InstantShape, fraction: usize) -> String {
    let fields: Vec<&str> = [
        (shape.has_date, "2000-01-01"),
        (shape.has_clock, "12:34:56"),
    ]
    .into_iter()
    .filter_map(|(is_present, field)| is_present.then_some(field))
    .collect();
    let digits: String = "123456789".chars().cycle().take(fraction).collect();
    let dot = if fraction == 0 { "" } else { "." };
    format!("{}#{}{dot}{digits}", shape.prefix, fields.join("-"))
}

/// Return the type of `primitive` on `platform`, or `None` when the platform has no such type.
fn time_type_on(primitive: Primitive, platform: PlcPlatform) -> Option<TimeType> {
    TimeType::of(primitive, platform).filter(|ty| ty.range().is_some())
}

/// Parse the longest duration literal of each shape on each platform. Return the literal with its
/// primitive and platform. A literal with one more digit must fail with the grammar reason.
fn longest_duration_literals() -> Vec<(Primitive, PlcPlatform, String)> {
    let mut literals = Vec::new();
    for shape in &DURATIONS {
        let longest = duration_literal(shape, DURATION_PART_DIGITS);
        let too_long = duration_literal(shape, DURATION_PART_DIGITS + 1);
        for platform in PlcPlatform::ALL {
            let Some(ty) = time_type_on(shape.primitive, platform) else {
                continue;
            };
            let parsed = IecLiteral::parse(ty, &longest);
            assert!(parsed.is_ok(), "{platform:?} {longest}: {parsed:?}");
            assert_eq!(
                IecLiteral::parse(ty, &too_long),
                Err(IecLiteralReason::Grammar),
                "{platform:?} {too_long}"
            );
            literals.push((shape.primitive, platform, longest.clone()));
        }
    }
    literals
}

/// Parse the longest instant literal of each shape on each platform. Return the literal with its
/// primitive and platform. A literal with one more fraction digit must fail.
fn longest_instant_literals() -> Vec<(Primitive, PlcPlatform, String)> {
    let mut literals = Vec::new();
    for shape in &INSTANTS {
        let longest = instant_literal(shape, shape.fraction);
        let too_long = instant_literal(shape, shape.fraction.saturating_add(1));
        for platform in PlcPlatform::ALL {
            let Some(ty) = time_type_on(shape.primitive, platform) else {
                continue;
            };
            let parsed = IecLiteral::parse(ty, &longest);
            assert!(parsed.is_ok(), "{platform:?} {longest}: {parsed:?}");
            let refused = IecLiteral::parse(ty, &too_long);
            assert!(refused.is_err(), "{platform:?} {too_long}: {refused:?}");
            literals.push((shape.primitive, platform, longest.clone()));
        }
    }
    literals
}

#[test]
fn limit_text_max_follows_the_longest_time_literal() {
    let mut literals = longest_duration_literals();
    literals.extend(longest_instant_literals());

    let platforms_of_ltime: Vec<PlcPlatform> = literals
        .iter()
        .filter(|(primitive, _platform, _text)| *primitive == Primitive::Ltime)
        .map(|(_primitive, platform, _text)| *platform)
        .collect();
    assert_eq!(
        platforms_of_ltime,
        [
            PlcPlatform::CodesysIec,
            PlcPlatform::TwinCat3,
            PlcPlatform::S7_1500
        ]
    );
    for (primitive, platform, text) in &literals {
        assert!(
            text.len() <= LIMIT_TEXT_MAX,
            "{primitive:?} {platform:?} {text}"
        );
    }
    let (primitive, _platform, longest) = literals
        .iter()
        .max_by_key(|(_primitive, _platform, text)| text.len())
        .unwrap();
    assert_eq!(*primitive, Primitive::Ltime);
    assert_eq!(longest.len(), 150, "{longest}");
    assert_eq!(longest.len().next_power_of_two(), LIMIT_TEXT_MAX);
}

#[test]
fn default_value_body_permits_equals_the_table() {
    let value = cell("http.value_body_permits");
    let number: usize = value.split_whitespace().next().unwrap().parse().unwrap();
    assert_eq!(WebSettings::default().value_body_permits, number);
    assert_eq!(number, 4);
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
