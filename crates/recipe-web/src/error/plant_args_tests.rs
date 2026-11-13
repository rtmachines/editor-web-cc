//! Tests of the arguments of the plant error messages (task 13.5, design D20 U1).
//!
//! Each test builds the answer of an HTMX request and reads the sentence in the fragment. An
//! argument that is missing makes `fluent` fail, and the answer then shows the English `Display`
//! text. So the Polish sentence proves that each placeable got a value.

use axum::body::to_bytes;
use recipe_app::error::AppError;
use recipe_domain::audit::RequestId;
use recipe_domain::plant::{
    CommMode, DerivedTypeReason, MachineCode, MemberName, PlantError, PlcAddress, PlcPlatform,
    TypeLimit, TypeName, UnitPath,
};
use recipe_domain::recipe::ReferenceNumber;
use uuid::Uuid;

use super::answer::{ErrorContext, answer};
use super::plant::plant_line_args;
use super::{LineArg, WebError};
use crate::i18n::{Bundles, Lang};
use crate::test_support::state;

/// The largest body that a test reads.
const BODY_LIMIT: usize = 1024 * 1024;

/// One error, the English sentence, and the Polish sentence that the answer shows.
type Sample = (PlantError, &'static str, &'static str);

fn code() -> MachineCode {
    MachineCode::parse("M7").unwrap()
}

fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("r-100").unwrap()
}

fn zone() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

fn speed() -> MemberName {
    MemberName::parse("speed").unwrap()
}

/// Return the body of the HTMX answer to `error` in `lang`.
async fn body_of(error: PlantError, lang: Lang) -> String {
    let state = state();
    let ctx = ErrorContext {
        request_id: RequestId::from(Uuid::nil()),
        lang,
        htmx: true,
        retry: None,
        state: &state,
    };
    let response = answer(&WebError::App(AppError::Plant(error)), &ctx);
    let bytes = to_bytes(response.into_body(), BODY_LIMIT).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

/// The 4 samples of the errors about a machine.
fn machine_samples() -> Vec<Sample> {
    vec![
        (
            PlantError::DuplicateMachineCode { code: code() },
            "Machine code M7 is already in use. Use a different code.",
            "Kod maszyny M7 jest już używany. Użyj innego kodu.",
        ),
        (
            PlantError::LastMainUnit {
                code: code(),
                path: path("U1"),
            },
            "Unit U1 is the last main unit of machine M7. A machine needs one main unit.",
            "Jednostka U1 jest ostatnią jednostką główną maszyny M7. Maszyna potrzebuje jednej jednostki głównej.",
        ),
        (
            PlantError::MachineArchived { code: code() },
            "Machine M7 is archived.",
            "Maszyna M7 jest zarchiwizowana.",
        ),
        (
            PlantError::MachineHasDraft {
                code: code(),
                reference: reference(),
            },
            "Machine M7 has a draft in recipe R-100. Approve or discard the draft first.",
            "Maszyna M7 ma wersję roboczą w recepturze R-100. Najpierw zatwierdź albo odrzuć wersję roboczą.",
        ),
    ]
}

/// The 7 samples of the errors about a unit.
fn unit_samples() -> Vec<Sample> {
    vec![
        (
            PlantError::UnitDepthExceeded {
                path: "U1.2.3.4.5.6".to_owned(),
            },
            "Unit path U1.2.3.4.5.6 has more than 5 levels. Use at most 5 levels.",
            "Ścieżka jednostki U1.2.3.4.5.6 ma więcej niż 5 poziomów. Użyj najwyżej 5 poziomów.",
        ),
        (
            PlantError::ParentNotFound { path: path("U1.2") },
            "The parent unit of U1.2 does not exist. Create the parent unit first.",
            "Jednostka nadrzędna dla U1.2 nie istnieje. Najpierw utwórz jednostkę nadrzędną.",
        ),
        (
            PlantError::DuplicateUnitPath { path: path("U1.2") },
            "Unit U1.2 already exists on this machine.",
            "Jednostka U1.2 już istnieje na tej maszynie.",
        ),
        (
            PlantError::UnitArchived { path: path("U1.2") },
            "Unit U1.2 is archived. Select a live unit.",
            "Jednostka U1.2 jest zarchiwizowana. Wybierz aktywną jednostkę.",
        ),
        (
            PlantError::UnitReferencedByDraft {
                path: path("U1.2"),
                reference: reference(),
            },
            "Unit U1.2 is used by the draft of recipe R-100. Move or delete those parameters first.",
            "Jednostka U1.2 jest używana w wersji roboczej receptury R-100. Najpierw przenieś albo usuń te parametry.",
        ),
        (
            PlantError::UnitReferencedByApproved {
                path: path("U1.2"),
                reference: reference(),
            },
            "Unit U1.2 is used by the approved version of recipe R-100. An Administrator can retire the version.",
            "Jednostka U1.2 jest używana w zatwierdzonej wersji receptury R-100. Administrator może wycofać tę wersję.",
        ),
        (
            PlantError::InvalidUnitPath {
                text: "U65".to_owned(),
            },
            "U65 is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2.",
            "U65 nie jest ścieżką jednostki. Ścieżka jednostki to U i od 1 do 5 liczb od 1 do 64, z kropką między dwiema liczbami. Przykład: U1.2.",
        ),
    ]
}

/// The 8 samples of the errors about a type.
fn type_samples() -> Vec<Sample> {
    vec![
        (
            PlantError::TypeCycle { type_name: zone() },
            "Type ST_Zone contains itself. Delete the circular member.",
            "Typ ST_Zone zawiera sam siebie. Usuń składnik, który tworzy cykl.",
        ),
        (
            PlantError::MemberTypeNotOnPlatform {
                type_name: zone(),
                member: speed(),
                platform: PlcPlatform::S7_300,
            },
            "Member speed of ST_Zone uses a type that does not exist on Siemens S7-300.",
            "Składnik speed typu ST_Zone używa typu, który nie istnieje na platformie Siemens S7-300.",
        ),
        (
            PlantError::DerivedTypeInvalid {
                type_expr: "STRING(255)".to_owned(),
                reason: DerivedTypeReason::OutOfRange,
            },
            "The type STRING(255) is not valid: a bound, a dimension, or a length is outside its range. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL.",
            "Typ STRING(255) jest niepoprawny: granica, wymiar albo długość jest poza zakresem. Użyj typu takiego jak INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL.",
        ),
        (
            PlantError::DuplicateTypeName { type_name: zone() },
            "Type ST_Zone already exists on this machine. Create a new revision instead.",
            "Typ ST_Zone już istnieje na tej maszynie. Zamiast tego utwórz nową rewizję.",
        ),
        (
            PlantError::StructHasNoMembers { type_name: zone() },
            "Type ST_Zone has no members. Add at least one member.",
            "Typ ST_Zone nie ma składników. Dodaj co najmniej jeden składnik.",
        ),
        (
            PlantError::TypeTooLarge {
                type_name: "ST_Zone".to_owned(),
                limit: TypeLimit::Members,
            },
            "Type ST_Zone is larger than the limit 1024 members. Use fewer elements, fewer members, or fewer levels.",
            "Typ ST_Zone przekracza limit 1024 składników. Użyj mniej elementów, mniej składników lub mniej poziomów.",
        ),
        (
            PlantError::DuplicateMemberName {
                type_name: zone(),
                member: speed(),
            },
            "Type ST_Zone has two members with the name speed. Give each member a different name.",
            "Typ ST_Zone ma dwa składniki o nazwie speed. Nadaj każdemu składnikowi inną nazwę.",
        ),
        (
            PlantError::RevisionUnchanged {
                type_name: zone(),
                revision: 3,
            },
            "Revision 3 of type ST_Zone has the same members. Change a member before you create the revision.",
            "Rewizja 3 typu ST_Zone ma te same składniki. Zmień składnik, zanim utworzysz rewizję.",
        ),
    ]
}

/// One sample of each of the 19 variants with a message, the English and the Polish sentence.
fn samples() -> Vec<Sample> {
    [machine_samples(), unit_samples(), type_samples()].concat()
}

/// Assert that the fragment shows `sentence` and no hole of a placeable.
fn assert_sentence(body: &str, sentence: &str, what: &str) {
    assert!(body.contains(sentence), "{what}: {body}");
    assert!(
        !body.contains("{ $") && !body.contains("{$"),
        "{what}: {body}"
    );
}

#[tokio::test]
async fn plant_error_args_give_every_placeable_a_value_in_both_languages() {
    let samples = samples();
    assert_eq!(samples.len(), 19);
    for (error, english, polish) in samples {
        let what = error.code().as_str();
        assert_sentence(&body_of(error.clone(), Lang::En).await, english, what);
        assert_sentence(&body_of(error, Lang::Pl).await, polish, what);
    }
}

#[tokio::test]
async fn plant_error_args_write_each_reason_of_a_type_in_the_ui_language() {
    let cases = [
        (
            DerivedTypeReason::Grammar,
            "the text does not match the type grammar",
            "tekst nie pasuje do gramatyki typu",
        ),
        (
            DerivedTypeReason::UnknownStruct,
            "the struct or the revision does not exist on the machine",
            "struktura albo rewizja nie istnieje na tej maszynie",
        ),
        (
            DerivedTypeReason::NotOnPlatform,
            "the type does not exist on the platform of the machine",
            "typ nie istnieje na platformie maszyny",
        ),
        (
            DerivedTypeReason::OutOfRange,
            "a bound, a dimension, or a length is outside its range",
            "granica, wymiar albo długość jest poza zakresem",
        ),
        (
            DerivedTypeReason::RowShape,
            "the stored type row does not match its kind",
            "zapisany wiersz typu nie pasuje do swojego rodzaju",
        ),
        (
            DerivedTypeReason::ReservedName,
            "the struct name is a reserved IEC 61131-3 word or a primitive name",
            "nazwa struktury jest zastrzeżonym słowem IEC 61131-3 albo nazwą typu prostego",
        ),
    ];
    for (reason, english, polish) in cases {
        let error = PlantError::DerivedTypeInvalid {
            type_expr: "FOO".to_owned(),
            reason,
        };
        let en = body_of(error.clone(), Lang::En).await;
        let pl = body_of(error, Lang::Pl).await;
        assert!(
            en.contains(&format!("The type FOO is not valid: {english}.")),
            "{en}"
        );
        assert!(
            pl.contains(&format!("Typ FOO jest niepoprawny: {polish}.")),
            "{pl}"
        );
    }
}

#[test]
fn plant_error_args_name_the_reserved_name_reason_by_its_key() {
    let reason = DerivedTypeReason::ReservedName;
    let error = PlantError::DerivedTypeInvalid {
        type_expr: "Int@1".to_owned(),
        reason,
    };
    let word = plant_line_args(&error)
        .into_iter()
        .find_map(|(name, arg)| (name == "reason").then_some(arg));
    assert_eq!(
        word,
        Some(LineArg::Word {
            key: "type-reason-reserved-name".to_owned(),
            args: Vec::new(),
            fallback: reason.to_string(),
        })
    );
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        bundles.message(Lang::En, "type-reason-reserved-name", None),
        Some(reason.to_string())
    );
}

#[tokio::test]
async fn plant_error_args_write_each_limit_with_its_number_and_unit() {
    let cases = [
        (
            TypeLimit::Leaves,
            "16384 leaves",
            "16384 elementów skalarnych",
        ),
        (TypeLimit::Depth, "32 levels", "32 poziomów"),
        (TypeLimit::Members, "1024 members", "1024 składników"),
    ];
    for (limit, english, polish) in cases {
        let error = PlantError::TypeTooLarge {
            type_name: "ST_Zone".to_owned(),
            limit,
        };
        let en = body_of(error.clone(), Lang::En).await;
        let pl = body_of(error, Lang::Pl).await;
        assert!(
            en.contains(&format!("larger than the limit {english}.")),
            "{en}"
        );
        assert!(pl.contains(&format!("przekracza limit {polish}.")), "{pl}");
    }
}

#[tokio::test]
async fn plant_error_args_write_each_platform_with_its_name_from_the_bundle() {
    let cases = [
        (PlcPlatform::CodesysIec, "CODESYS (IEC 61131-3)"),
        (PlcPlatform::TwinCat3, "TwinCAT 3"),
        (PlcPlatform::S7_1200, "Siemens S7-1200"),
        (PlcPlatform::S7_1500, "Siemens S7-1500"),
        (PlcPlatform::S7_300, "Siemens S7-300"),
    ];
    for (platform, text) in cases {
        let error = PlantError::MemberTypeNotOnPlatform {
            type_name: zone(),
            member: speed(),
            platform,
        };
        for lang in [Lang::En, Lang::Pl] {
            let body = body_of(error.clone(), lang).await;
            assert!(body.contains(&format!("{text}.")), "{lang}: {body}");
        }
    }
}

/// Return the arguments of `error` as the pairs of a name and a data text.
///
/// A word of the UI shows as the key of its bundle message.
fn data_args(error: &PlantError) -> Vec<(&'static str, String)> {
    plant_line_args(error)
        .into_iter()
        .map(|(name, arg)| match arg {
            LineArg::Data(text) => (name, text),
            LineArg::Word { key, .. } => (name, format!("word:{key}")),
        })
        .collect()
}

fn pairs(list: &[(&'static str, &str)]) -> Vec<(&'static str, String)> {
    list.iter()
        .map(|(name, text)| (*name, (*text).to_owned()))
        .collect()
}

type Case = (PlantError, Vec<(&'static str, String)>);

fn connection_field_and_mode_cases() -> Vec<Case> {
    vec![
        (
            PlantError::ConnectionFieldNotOnPlatform {
                code: code(),
                field: "rack",
                platform: PlcPlatform::TwinCat3,
            },
            pairs(&[
                ("code", "M7"),
                ("field", "rack"),
                ("platform", "word:platform-twincat3"),
            ]),
        ),
        (
            PlantError::ConnectionFieldMissing {
                code: code(),
                field: "host",
            },
            pairs(&[("code", "M7"), ("field", "host")]),
        ),
        (
            PlantError::DataBlockOutOfRange {
                code: code(),
                number: 6_000,
                max: 4_095,
            },
            pairs(&[("code", "M7"), ("number", "6000"), ("max", "4095")]),
        ),
        (
            PlantError::CommModeNotOnPlatform {
                code: code(),
                mode: CommMode::Ads,
                platform: PlcPlatform::S7_1500,
            },
            pairs(&[
                ("code", "M7"),
                ("mode", "Ads"),
                ("platform", "word:platform-s7-1500"),
            ]),
        ),
    ]
}

fn connection_state_and_block_cases() -> Vec<Case> {
    let address = PlcAddress::parse("plc-7").unwrap();
    vec![
        (
            PlantError::AddressNotAmsNetId {
                code: code(),
                address: address.clone(),
            },
            pairs(&[("code", "M7"), ("address", &address.to_string())]),
        ),
        (
            PlantError::ConnectionNotConfigured { code: code() },
            pairs(&[("code", "M7")]),
        ),
        (
            PlantError::ConnectionFieldNotInMode {
                code: code(),
                field: "login",
                mode: CommMode::ModbusTcp,
            },
            pairs(&[("code", "M7"), ("field", "login"), ("mode", "ModbusTcp")]),
        ),
        (
            PlantError::ConnectionRecipeMismatch {
                code: code(),
                reference: reference(),
            },
            pairs(&[("code", "M7"), ("reference", "R-100")]),
        ),
        (
            PlantError::DataBlockMissing {
                code: code(),
                platform: PlcPlatform::S7_1500,
            },
            pairs(&[("code", "M7"), ("platform", "word:platform-s7-1500")]),
        ),
        (
            PlantError::DataBlockNotOnPlatform {
                code: code(),
                platform: PlcPlatform::TwinCat3,
            },
            pairs(&[("code", "M7"), ("platform", "word:platform-twincat3")]),
        ),
    ]
}

#[test]
fn plant_error_args_give_each_connection_error_its_data() {
    let cases = connection_field_and_mode_cases()
        .into_iter()
        .chain(connection_state_and_block_cases());
    for (error, expected) in cases {
        assert_eq!(data_args(&error), expected, "{error:?}");
    }
}
