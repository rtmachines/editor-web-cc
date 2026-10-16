//! Tests of the messages of the recipe errors in `pl` and `en` (task 7.3, design D13).
//!
//! Each line goes through `leaf_lines` and `line_view`, the path of every error list. The English
//! text of an error without a word of the UI equals its `Display`, which is the registry text.

use std::collections::BTreeSet;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Login;
use recipe_domain::plant::{
    LeafPath, MemberName, PlcPlatform, Primitive, S7Address, S7Width, TypeName,
};
use recipe_domain::recipe::{
    Budget, Expected, LangCode, ParameterKey, ParameterTag, RebindTargetReason, RecipeError,
    RecipeErrors, ReferenceNumber, SnapshotError, SnapshotItem, VersionStatus,
};
use uuid::Uuid;

use super::lists::{leaf_lines, line_view};
use crate::i18n::{Bundles, Lang};

/// An error, and its English text when that text is not the `Display` of the error.
type Case = (RecipeError, Option<&'static str>);

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-100").unwrap()
}

fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

fn leaf(text: &str) -> LeafPath {
    LeafPath::parse(text).unwrap()
}

fn address(text: &str) -> S7Address {
    S7Address::parse(text).unwrap()
}

/// The errors of a recipe.
fn recipe_errors() -> Vec<Case> {
    vec![
        (
            RecipeError::DuplicateReference {
                reference: reference(),
            },
            None,
        ),
        (
            RecipeError::RecipeDeleted {
                reference: reference(),
            },
            None,
        ),
        (
            RecipeError::RecipeHasApprovedVersion {
                reference: reference(),
            },
            None,
        ),
        (
            RecipeError::RecipeHasRetiredVersion {
                reference: reference(),
            },
            None,
        ),
        (
            RecipeError::RecipeNotDeleted {
                reference: reference(),
            },
            None,
        ),
    ]
}

/// The errors of the state of a version.
fn version_errors() -> Vec<Case> {
    let number = 3;
    vec![
        (
            RecipeError::VersionNotDraft {
                reference: reference(),
                number,
            },
            None,
        ),
        (
            RecipeError::VersionNotApproved {
                reference: reference(),
                number,
            },
            None,
        ),
        (
            RecipeError::DraftAlreadyExists {
                reference: reference(),
                number,
            },
            None,
        ),
        (
            RecipeError::DraftHasNoParameters {
                reference: reference(),
                number,
            },
            None,
        ),
    ]
}

/// The errors with a word of the UI, the approver, and the language code.
fn state_errors() -> Vec<Case> {
    let number = 3;
    vec![
        (
            RecipeError::VersionNotEditable {
                reference: reference(),
                number,
                status: VersionStatus::Approved,
            },
            Some("Version 3 of recipe R-100 is Approved. Create a new draft to make changes."),
        ),
        (
            RecipeError::ApproverEditedDraft {
                reference: reference(),
                number,
                login: Login::parse("anowak").unwrap(),
            },
            None,
        ),
        (
            RecipeError::SizeBudgetExceeded {
                budget: Budget::Leaves,
                limit: 262_144,
                actual: 262_145,
            },
            Some("The version exceeds the limit of 262144 scalar leaves. Current count: 262145."),
        ),
        (
            RecipeError::InvalidLangCode {
                lang: "x_1".to_owned(),
            },
            None,
        ),
    ]
}

/// The errors of a key or a language that the draft does not hold.
fn key_errors() -> Vec<Case> {
    let number = 3;
    vec![
        (
            RecipeError::TranslationNotFound {
                reference: reference(),
                number,
                tag: tag("SPEED"),
                lang: LangCode::parse("de-AT").unwrap(),
            },
            None,
        ),
        (
            RecipeError::ParameterNotFound {
                reference: reference(),
                number,
                key: ParameterKey::from(Uuid::nil()),
            },
            None,
        ),
    ]
}

/// The errors of a value and a limit of one parameter.
fn value_errors() -> Vec<Case> {
    let (speed, at) = (tag("SPEED_TABLE"), leaf("[3]"));
    vec![
        (
            RecipeError::ValueOutOfLimit {
                tag: speed.clone(),
                leaf: at.clone(),
                value: "3500".to_owned(),
                min: "0".to_owned(),
                max: "3000".to_owned(),
            },
            None,
        ),
        (
            RecipeError::ValueShapeMismatch {
                tag: tag("LABEL"),
                leaf: leaf("[2]"),
                expected: Expected::Char { max: 0xFF },
            },
            None,
        ),
        (
            RecipeError::MinGreaterThanMax {
                tag: speed,
                leaf: at,
                min: "10".to_owned(),
                max: "5".to_owned(),
            },
            None,
        ),
    ]
}

/// The errors of one leaf of a parameter.
fn leaf_errors() -> Vec<Case> {
    let (speed, at) = (tag("SPEED_TABLE"), leaf("[3]"));
    vec![
        (
            RecipeError::LimitOutsideTypeRange {
                tag: speed.clone(),
                leaf: at.clone(),
            },
            None,
        ),
        (
            RecipeError::LeafPathNotFound {
                tag: speed.clone(),
                leaf: at.clone(),
            },
            None,
        ),
        (
            RecipeError::LimitNotApplicable {
                tag: speed.clone(),
                leaf: at.clone(),
            },
            None,
        ),
        (
            RecipeError::LimitMissing {
                tag: speed.clone(),
                leaf: at.clone(),
            },
            None,
        ),
        (
            RecipeError::RebindNeedsValue {
                tag: speed.clone(),
                leaf: at.clone(),
            },
            None,
        ),
        (
            RecipeError::RebindDropsLimit {
                tag: speed,
                leaf: at,
            },
            None,
        ),
    ]
}

/// The errors of a rebind and of a type.
fn type_errors() -> Vec<Case> {
    vec![
        (
            RecipeError::RebindTargetInvalid {
                type_name: "ST_Zone@2".to_owned(),
                reason: RebindTargetReason::NotNewer,
            },
            Some(
                "The rebind of ST_Zone@2 is not possible: the target revision is not higher than the source revision.",
            ),
        ),
        (
            RecipeError::TypeRevisionOutdated {
                tag: tag("ZONES"),
                type_name: TypeName::parse("ST_Zone").unwrap(),
                current: 2,
                newest: 3,
            },
            None,
        ),
    ]
}

/// The errors of the tag, the type, the address, and the snapshot of a parameter.
fn address_errors() -> Vec<Case> {
    let speed = tag("SPEED");
    vec![
        (
            RecipeError::DuplicateTag {
                tag: speed.clone(),
                number: 3,
            },
            None,
        ),
        (RecipeError::S7AddressRequired { tag: speed.clone() }, None),
        (
            RecipeError::S7AddressOverlap {
                tag: speed.clone(),
                other_tag: tag("HOLD"),
                address: address("DB1.DBW0"),
            },
            None,
        ),
        (
            RecipeError::S7AddressMisaligned {
                tag: speed.clone(),
                address: address("DB1.DBW1"),
            },
            None,
        ),
    ]
}

/// The errors with a platform, and the snapshot check.
fn platform_errors() -> Vec<Case> {
    let speed = tag("SPEED");
    vec![
        (
            RecipeError::S7AddressNotAllowed {
                tag: speed.clone(),
                platform: PlcPlatform::TwinCat3,
            },
            Some(
                "Parameter SPEED cannot have a PLC address on platform TwinCAT 3. Delete the address.",
            ),
        ),
        (
            RecipeError::S7AddressWidthMismatch {
                tag: speed.clone(),
                address: address("DB1.DBB0"),
                width: S7Width::W,
            },
            None,
        ),
        (
            RecipeError::TypeNotOnPlatform {
                tag: speed,
                type_name: "WSTRING(10)".to_owned(),
                platform: PlcPlatform::S7_300,
            },
            Some(
                "Parameter SPEED uses type WSTRING(10). This type does not exist on Siemens S7-300.",
            ),
        ),
        (RecipeError::ValidationFailed { items: items() }, None),
    ]
}

/// Three items of two parameters and one item of the version.
fn items() -> Vec<SnapshotItem> {
    let missing = |name: &str| SnapshotItem::Parameter {
        tag: tag(name),
        error: SnapshotError::Recipe(RecipeError::LimitMissing {
            tag: tag(name),
            leaf: leaf(""),
        }),
    };
    vec![
        missing("SPEED"),
        missing("SPEED"),
        missing("HOLD"),
        SnapshotItem::Version {
            error: SnapshotError::Recipe(RecipeError::RecipeDeleted {
                reference: reference(),
            }),
        },
    ]
}

fn every_case() -> Vec<Case> {
    let mut cases = recipe_errors();
    cases.extend(version_errors());
    cases.extend(state_errors());
    cases.extend(key_errors());
    cases.extend(value_errors());
    cases.extend(leaf_errors());
    cases.extend(type_errors());
    cases.extend(address_errors());
    cases.extend(platform_errors());
    cases
}

/// Return the message of the one line of `error` in `lang`.
fn message(bundles: &Bundles, error: RecipeError, lang: Lang) -> String {
    let lines = leaf_lines(&RecipeErrors::from(error), &|_path: &LeafPath| None);
    line_view(lines.first().unwrap(), bundles, lang).message
}

/// Return the message of `error` in `lang`, from a copy that the closure builds.
fn messages(bundles: &Bundles, index: usize) -> (String, String, String, Option<&'static str>) {
    let pick = || every_case().into_iter().nth(index).unwrap();
    let (error, english) = pick();
    let display = error.to_string();
    let en = message(bundles, error, Lang::En);
    let pl = message(bundles, pick().0, Lang::Pl);
    (display, en, pl, english)
}

#[test]
fn recipe_the_cases_cover_each_code_of_recipe_error() {
    let codes: BTreeSet<ErrorCode> = every_case().iter().map(|(error, _)| error.code()).collect();
    assert_eq!(codes.len(), 34);
    assert_eq!(every_case().len(), 34);
}

#[test]
fn recipe_each_code_gives_its_english_message() {
    let bundles = Bundles::load().unwrap();
    for index in 0..every_case().len() {
        let (display, en, _pl, english) = messages(&bundles, index);
        let expected = english.map_or(display.clone(), str::to_owned);
        assert_eq!(en, expected, "{display}");
    }
}

#[test]
fn recipe_each_code_gives_its_polish_message_and_not_the_english_fallback() {
    let bundles = Bundles::load().unwrap();
    for index in 0..every_case().len() {
        let (display, en, pl, _english) = messages(&bundles, index);
        assert_ne!(pl, display, "Polish fell back to English: {display}");
        assert_ne!(pl, en, "{display}");
        assert!(!pl.contains('{') && !pl.contains('}'), "{pl}");
    }
}

#[test]
fn recipe_the_words_of_a_polish_message_come_from_the_polish_bundle() {
    let bundles = Bundles::load().unwrap();
    let polish = |index: usize| messages(&bundles, index).2;
    let all = every_case();
    let find = |code: ErrorCode| {
        all.iter()
            .position(|(error, _)| error.code() == code)
            .unwrap()
    };
    assert_eq!(
        polish(find(ErrorCode::RCP_002)),
        "Wersja 3 receptury R-100 ma stan Zatwierdzona. Utw\u{f3}rz now\u{105} wersj\u{119} robocz\u{105}, aby wprowadzi\u{107} zmiany."
    );
    assert_eq!(
        polish(find(ErrorCode::RCP_021)),
        "Wersja przekracza limit 262144 warto\u{15b}ci skalarnych. Obecna liczba: 262145."
    );
    assert_eq!(
        polish(find(ErrorCode::RCP_005)),
        "Warto\u{15b}\u{107} parametru LABEL[2] nie pasuje do typu. Oczekiwano: jeden znak od U+0000 do U+00FF."
    );
    assert_eq!(
        polish(find(ErrorCode::RCP_052)),
        "Przepi\u{119}cie ST_Zone@2 nie jest mo\u{17c}liwe: rewizja docelowa nie jest wy\u{17c}sza ni\u{17c} rewizja \u{17a}r\u{f3}d\u{142}owa."
    );
    assert_eq!(
        polish(find(ErrorCode::RCP_053)),
        "Parametr SPEED nie mo\u{17c}e mie\u{107} adresu PLC na platformie TwinCAT 3. Usu\u{144} adres."
    );
}

#[test]
fn recipe_e_rcp_020_counts_the_parameters_with_a_problem_once_each() {
    let bundles = Bundles::load().unwrap();
    let error = RecipeError::ValidationFailed { items: items() };
    assert_eq!(
        message(&bundles, error, Lang::En),
        "2 parameters are not valid. See the list."
    );
}

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
            "a string of at most 10 UTF-16 units",
            "tekst o najwy\u{17c}ej 10 jednostkach UTF-16",
        ),
        (
            Expected::String { length: 20 },
            "a string of at most 20 characters from Latin-1",
            "tekst o najwy\u{17c}ej 20 znakach z Latin-1",
        ),
        (
            Expected::Char { max: 0xFFFF },
            "one character from U+0000 to U+FFFF",
            "jeden znak od U+0000 do U+FFFF",
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
fn recipe_the_polish_message_of_e_rcp_054_names_the_type_of_the_parameter() {
    let bundles = Bundles::load().unwrap();
    let error = RecipeError::S7AddressWidthMismatch {
        tag: tag("SPEED"),
        address: address("DB1.DBB0"),
        width: S7Width::W,
    };
    let pl = message(&bundles, error, Lang::Pl);
    assert!(pl.contains("nie pasuje do typu parametru SPEED."), "{pl}");
}

#[test]
fn recipe_the_expected_forms_cover_each_variant() {
    assert_eq!(expected_cases().len(), 21);
}
