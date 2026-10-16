//! The words of a grid: the hints, the type of a leaf, and the text of a time bound (design D8).

use fluent::FluentArgs;
use recipe_domain::plant::{IecLiteral, LeafClass, PlcPlatform, Primitive, TimeType};

use crate::pages::Texts;

/// The hint of one class of leaves: its fixed `id`, its key, and its argument.
pub(super) struct HintKey {
    /// The `id` of the hint element. It comes from the class, never from user data.
    pub(super) id: String,
    /// The key of the sentence in the bundles.
    pub(super) key: &'static str,
    /// The name and the text of the argument of the sentence, if it has one.
    pub(super) arg: Option<(&'static str, String)>,
}

/// Return the hint of a leaf, or `None` for a `BOOL` or a `CHAR` leaf (design D8).
///
/// A time hint shows an example with the prefix of its type, so each prefix has its own hint. A
/// string hint names the length, so each length has its own hint.
pub(super) fn hint_key(primitive: Primitive, string_length: Option<u16>) -> Option<HintKey> {
    let plain = |id: &str, key| HintKey {
        id: id.to_owned(),
        key,
        arg: None,
    };
    match primitive.leaf_class() {
        LeafClass::Numeric => Some(match primitive {
            Primitive::Real | Primitive::Lreal => plain("grid-hint-number", "grid-hint-number"),
            _integer => plain("grid-hint-integer", "grid-hint-integer"),
        }),
        LeafClass::String => {
            let length = string_length.unwrap_or_default();
            Some(HintKey {
                id: format!("grid-hint-string-{length}"),
                key: "grid-hint-string",
                arg: Some(("length", length.to_string())),
            })
        }
        LeafClass::Duration | LeafClass::Instant => time_hint(primitive),
        LeafClass::Bool | LeafClass::Char => None,
    }
}

/// Return the hint of a time leaf with an example literal of its type.
fn time_hint(primitive: Primitive) -> Option<HintKey> {
    let (key, example) = match primitive {
        Primitive::Time => ("grid-hint-duration", "T#1m30s"),
        Primitive::Ltime => ("grid-hint-duration", "LT#1m30s"),
        Primitive::S5Time => ("grid-hint-duration", "S5T#1s"),
        Primitive::Date => ("grid-hint-date", "D#2026-10-12"),
        Primitive::Ldate => ("grid-hint-date", "LD#2026-10-12"),
        Primitive::TimeOfDay => ("grid-hint-time-of-day", "TOD#12:30:00"),
        Primitive::Ltod => ("grid-hint-time-of-day", "LTOD#12:30:00"),
        Primitive::DateAndTime => ("grid-hint-date-time", "DT#2026-10-12-12:30:00"),
        Primitive::Ldt => ("grid-hint-date-time", "LDT#2026-10-12-12:30:00"),
        Primitive::Dtl => ("grid-hint-date-time", "DTL#2026-10-12-12:30:00"),
        _other => return None,
    };
    let (prefix, _rest) = example.split_once('#')?;
    Some(HintKey {
        id: format!("{key}-{}", prefix.to_ascii_lowercase()),
        key,
        arg: Some(("example", example.to_owned())),
    })
}

/// Write the type of a leaf, such as `INT` or `STRING(20)`.
pub(super) fn type_text(primitive: Primitive, string_length: Option<u16>) -> String {
    match string_length {
        Some(length) => format!("{}({length})", primitive.name()),
        None => primitive.name().to_owned(),
    }
}

/// Write a stored bound for a page (`domain-model.md` section 2.1).
///
/// A time bound is a count of the base unit. It shows as its canonical IEC literal. Any other
/// bound keeps its canonical decimal text.
pub(super) fn bound_text(primitive: Primitive, platform: PlcPlatform, bound: String) -> String {
    let literal = TimeType::of(primitive, platform).and_then(|time_type| {
        let count: i128 = bound.parse().ok()?;
        IecLiteral::try_from((time_type, count)).ok()
    });
    literal.map_or(bound, |literal| literal.to_string())
}

/// Return the sentence `key` with its arguments in the language of `t`.
pub(super) fn message(t: Texts<'_>, key: &str, args: &[(&'static str, &str)]) -> String {
    let mut values = FluentArgs::new();
    for (name, value) in args {
        values.set(*name, (*value).to_owned());
    }
    t.with(key, &values)
}
