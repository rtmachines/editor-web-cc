//! The canonical form of a setting value, so that `30 s`, `30s`, and `0.5 min` compare as equal.
//!
//! A time becomes milliseconds. A size becomes bytes with the binary factor of Docker and PostgreSQL
//! (`m`, `MB`, and `MiB` are 1 048 576 bytes). A bare number keeps three decimals.

/// What a value means after the unit is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Canon {
    Millis(u128),
    Bytes(u128),
    Percent(u128),
    Number(u128),
    Text(String),
}

/// The meaning of a number that has no unit in the file under test.
#[derive(Debug, Clone, Copy)]
pub enum Bare {
    Plain,
    Millis,
    Seconds,
    Days,
    Percent,
}

const MILLI: u128 = 1000;
const MS_PER_DAY: u128 = 86_400_000;

fn millis_per_unit(unit: &str) -> Option<u128> {
    match unit {
        "ms" => Some(1),
        "s" => Some(MILLI),
        "min" => Some(60_000),
        "h" => Some(3_600_000),
        "day" | "days" => Some(MS_PER_DAY),
        _ => None,
    }
}

fn bytes_per_unit(unit: &str) -> Option<u128> {
    match unit {
        "kB" | "KiB" | "k" => Some(1024),
        "MB" | "MiB" | "m" => Some(1_048_576),
        "GB" | "GiB" | "g" => Some(1_073_741_824),
        _ => None,
    }
}

/// A unit that may stand alone as the second word of a value. The letters `k`, `m`, and `g` do not.
fn is_word_unit(unit: &str) -> bool {
    unit == "%"
        || (millis_per_unit(unit).is_some())
        || (bytes_per_unit(unit).is_some() && unit.len() > 1)
}

/// Parse a decimal text such as `0.5` or `9000` into thousandths.
pub fn parse_milli(text: &str) -> Option<u128> {
    let (whole_text, fraction_text) = text.split_once('.').unwrap_or((text, ""));
    if whole_text.is_empty() || fraction_text.len() > 3 || text.ends_with('.') {
        return None;
    }
    let whole: u128 = whole_text.parse().ok()?;
    let mut fraction: u128 = if fraction_text.is_empty() {
        0
    } else {
        fraction_text.parse().ok()?
    };
    for _ in fraction_text.len()..3 {
        fraction = fraction.checked_mul(10)?;
    }
    whole.checked_mul(MILLI)?.checked_add(fraction)
}

pub fn apply_unit(milli: u128, unit: &str) -> Option<Canon> {
    if unit == "%" {
        return Some(Canon::Percent(milli));
    }
    if let Some(ms) = millis_per_unit(unit) {
        return Some(Canon::Millis(milli.checked_mul(ms)?.checked_div(MILLI)?));
    }
    let factor = bytes_per_unit(unit)?;
    Some(Canon::Bytes(milli.checked_mul(factor)?.checked_div(MILLI)?))
}

fn apply_bare(milli: u128, bare: Bare) -> Option<Canon> {
    match bare {
        Bare::Plain => Some(Canon::Number(milli)),
        Bare::Millis => Some(Canon::Millis(milli.checked_div(MILLI)?)),
        Bare::Seconds => Some(Canon::Millis(milli)),
        Bare::Percent => Some(Canon::Percent(milli)),
        Bare::Days => Some(Canon::Millis(
            milli.checked_mul(MS_PER_DAY)?.checked_div(MILLI)?,
        )),
    }
}

pub fn trim_mark(word: &str) -> &str {
    word.trim_end_matches(['.', ','])
}

pub fn strip_marks(text: &str) -> String {
    text.chars()
        .filter(|c| !matches!(c, '`' | '"' | '\''))
        .collect()
}

/// The value of one word, with an optional unit word after it.
fn read(first: &str, unit_word: Option<&str>, bare: Bare) -> Option<Canon> {
    let split = first
        .find(|c: char| !(c.is_ascii_digit() || c == '.'))
        .unwrap_or(first.len());
    let (digits, suffix) = first.split_at(split);
    let Some(milli) = parse_milli(digits) else {
        return Some(Canon::Text(first.to_owned()));
    };
    if suffix.is_empty() {
        return match unit_word {
            Some(word) => apply_unit(milli, word),
            None => apply_bare(milli, bare),
        };
    }
    match apply_unit(milli, suffix) {
        Some(value) => Some(value),
        None => Some(Canon::Text(first.to_owned())),
    }
}

/// Read the leading value of a text of a file under test. Quotes and backticks do not count.
/// Words after the value do not count. The table of the document does not use this reader,
/// because it drops an unknown unit word. The table uses `units::read_typed`.
///
/// Returns `None` only when a number is too large or the text is empty.
pub fn canon(text: &str, bare: Bare) -> Option<Canon> {
    let cleaned = strip_marks(text);
    let mut words = cleaned.split_whitespace();
    let first = trim_mark(words.next()?);
    let unit = words
        .next()
        .map(trim_mark)
        .filter(|word| is_word_unit(word));
    read(first, unit, bare)
}

/// Read the whole text of a value in a file under test. The caller removes the quotes and the
/// comment with `unquote`. Any word or mark after the value makes the result a `Canon::Text` of
/// the whole text, so a changed value never equals the value of the document.
///
/// Returns `None` only when a number is too large or the text is empty.
pub fn canon_whole(text: &str, bare: Bare) -> Option<Canon> {
    let value = text.trim();
    let words: Vec<&str> = value.split_whitespace().collect();
    match words.as_slice() {
        [] => None,
        [one] => read(one, None, bare),
        [one, unit]
            if is_word_unit(unit) && one.chars().all(|c| c.is_ascii_digit() || c == '.') =>
        {
            read(one, Some(unit), bare)
        }
        _ => Some(Canon::Text(value.to_owned())),
    }
}
