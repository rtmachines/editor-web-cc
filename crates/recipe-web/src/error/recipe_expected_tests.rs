//! Tests of the phrases of the expected forms in the messages of the recipe errors (task 7.3).
//!
//! The tests use the helpers of `recipe_tests`.

use recipe_domain::plant::{MemberName, Primitive};
use recipe_domain::recipe::{Expected, RecipeError};

use super::recipe_tests::{leaf, message, tag};
use crate::i18n::{Bundles, Lang};

/// An expected form with its English and its Polish phrase.
type Phrase = (Expected, &'static str, &'static str);

/// Every expected form, each with its English and Polish phrase.
fn expected_cases() -> Vec<Phrase> {
    let mut cases = shape_phrases();
    cases.extend(typed_phrases());
    cases.extend(text_phrases());
    cases
}

/// The expected forms of a shape, a string, and a character.
fn shape_phrases() -> Vec<Phrase> {
    let members = vec![
        MemberName::parse("min").unwrap(),
        MemberName::parse("max").unwrap(),
    ];
    vec![
        (Expected::Json, "valid JSON text", "poprawny tekst JSON"),
        (
            Expected::Depth { levels: 32 },
            "a value with at most 32 levels",
            "warto\u{15b}\u{107} o najwy\u{17c}ej 32 poziomach",
        ),
        (
            Expected::Object { members },
            "an object with the members min, max",
            "obiekt ze sk\u{142}adnikami min, max",
        ),
        (Expected::Array, "an array", "tablica"),
        (
            Expected::ArrayCount { count: 8 },
            "an array of 8 elements",
            "tablica z 8 elementami",
        ),
        (
            Expected::Index,
            "an index inside the INTEGER range",
            "indeks w zakresie INTEGER",
        ),
        (
            Expected::Scalar,
            "a scalar value",
            "warto\u{15b}\u{107} skalarna",
        ),
        (
            Expected::Wstring { length: 10 },
            "a string of at most 10 UTF-16 units without U+0000",
            "tekst o najwy\u{17c}ej 10 jednostkach UTF-16 bez znaku U+0000",
        ),
        (
            Expected::String { length: 20 },
            "a string of at most 20 characters from Latin-1 without U+0000",
            "tekst o najwy\u{17c}ej 20 znakach z Latin-1 bez znaku U+0000",
        ),
        (
            Expected::Char { max: 0xFFFF },
            "one character from U+0001 to U+FFFF",
            "jeden znak od U+0001 do U+FFFF",
        ),
        (
            Expected::Char { max: 0xD7FF },
            "one character from U+0001 to U+D7FF",
            "jeden znak od U+0001 do U+D7FF",
        ),
        (
            Expected::WstringS7 {
                length: 10,
                max: 0xCFFF,
            },
            "a string of at most 10 characters from U+0001 to U+CFFF",
            "tekst o najwy\u{17c}ej 10 znakach od U+0001 do U+CFFF",
        ),
    ]
}

/// The expected forms with a type name.
fn typed_phrases() -> Vec<Phrase> {
    let int = Primitive::Int;
    vec![
        (
            Expected::Literal {
                type_name: Primitive::Time,
            },
            "an IEC literal of type TIME",
            "litera\u{142} IEC typu TIME",
        ),
        (
            Expected::Count {
                type_name: Primitive::Time,
            },
            "a count of type TIME",
            "liczba jednostek typu TIME",
        ),
        (
            Expected::Integer { type_name: int },
            "an integer of type INT",
            "liczba ca\u{142}kowita typu INT",
        ),
        (
            Expected::NumberRange {
                type_name: int,
                low: "-32768".parse().unwrap(),
                high: "32767".parse().unwrap(),
            },
            "a number of type INT from -32768 to 32767",
            "liczba typu INT od -32768 do 32767",
        ),
        (
            Expected::NumberDigits {
                type_name: Primitive::Real,
                digits: 7,
            },
            "a number of type REAL with at most 7 significant digits",
            "liczba typu REAL z najwy\u{17c}ej 7 cyframi znacz\u{105}cymi",
        ),
    ]
}

/// The expected forms of a parameter value and of the leaf texts.
fn text_phrases() -> Vec<Phrase> {
    vec![
        (
            Expected::ParameterType,
            "a value of the type of the parameter",
            "warto\u{15b}\u{107} typu parametru",
        ),
        (
            Expected::NewValue,
            "a new value for the new type",
            "nowa warto\u{15b}\u{107} dla nowego typu",
        ),
        (Expected::Bool, "TRUE or FALSE", "TRUE albo FALSE"),
        (
            Expected::Decimal,
            "a decimal number with a dot, for example 1.5",
            "liczba dziesi\u{119}tna z kropk\u{105}, na przyk\u{142}ad 1.5",
        ),
        (
            Expected::Leaf,
            "a value for this field",
            "warto\u{15b}\u{107} tego pola",
        ),
        (
            Expected::KnownLeaf,
            "a path of the type of the parameter",
            "\u{15b}cie\u{17c}ka z typu parametru",
        ),
    ]
}

#[test]
fn recipe_each_expected_form_gives_its_phrase_in_both_languages() {
    let bundles = Bundles::load().unwrap();
    for (expected, en, pl) in expected_cases() {
        let fault = || RecipeError::ValueShapeMismatch {
            tag: tag("T"),
            leaf: leaf(""),
            expected: expected.clone(),
        };
        assert_eq!(
            message(&bundles, fault(), Lang::En),
            format!("Value of T does not match the type. Expected {en}."),
        );
        assert_eq!(
            message(&bundles, fault(), Lang::Pl),
            format!("Warto\u{15b}\u{107} parametru T nie pasuje do typu. Oczekiwano: {pl}."),
        );
    }
}

#[test]
fn recipe_the_expected_forms_cover_each_variant() {
    assert_eq!(expected_cases().len(), 23);
}

/// The message of `E-RCP-005` for `expected` in `lang`.
fn shape_message(expected: Expected, lang: Lang) -> String {
    let bundles = Bundles::load().unwrap();
    let error = RecipeError::ValueShapeMismatch {
        tag: tag("NOTE"),
        leaf: leaf(""),
        expected,
    };
    message(&bundles, error, lang)
}

#[test]
fn adr_0180_the_english_message_of_a_refused_wstring_names_the_length_and_the_range() {
    let text = shape_message(
        Expected::WstringS7 {
            length: 10,
            max: 0xCFFF,
        },
        Lang::En,
    );
    assert!(text.contains("10"), "{text}");
    assert!(text.contains("U+0001"), "{text}");
    assert!(text.contains("U+CFFF"), "{text}");
}

#[test]
fn adr_0180_the_polish_message_of_a_refused_wchar_names_u_d7ff() {
    let text = shape_message(Expected::Char { max: 0xD7FF }, Lang::Pl);
    assert!(text.contains("U+D7FF"), "{text}");
    assert!(text.contains("Oczekiwano"), "{text}");
}

#[test]
fn adr_0180_the_polish_message_of_a_refused_wstring_names_the_length_and_the_range() {
    let text = shape_message(
        Expected::WstringS7 {
            length: 10,
            max: 0xCFFF,
        },
        Lang::Pl,
    );
    assert!(text.contains("10"), "{text}");
    assert!(text.contains("U+0001"), "{text}");
    assert!(text.contains("U+CFFF"), "{text}");
}
