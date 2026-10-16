use super::*;
use crate::plant::{LeafPath, PlantError, UnitPath};
use crate::recipe::Expected;

fn lang(text: &str) -> RecipeError {
    RecipeError::InvalidLangCode {
        lang: text.to_owned(),
    }
}

#[test]
fn duplicate_reference_has_its_code_registry_row_and_message() {
    let error = RecipeError::DuplicateReference {
        reference: ReferenceNumber::parse("R-1").unwrap(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_001);
    assert_eq!(error.code().as_str(), "E-RCP-001");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_001));
    assert_eq!(error.to_string(), "Reference number R-1 is already in use.");
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("R-1").unwrap()
}

#[test]
fn recipe_deleted_has_its_code_registry_row_and_message() {
    let error = RecipeError::RecipeDeleted {
        reference: reference(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_012);
    assert_eq!(error.code().as_str(), "E-RCP-012");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_012));
    assert_eq!(
        error.to_string(),
        "Recipe R-1 is deleted. An administrator can restore it."
    );
}

#[test]
fn approver_edited_draft_has_its_code_registry_row_and_message() {
    let error = RecipeError::ApproverEditedDraft {
        reference: reference(),
        number: 4,
        login: crate::identity::Login::parse("anna").unwrap(),
    };
    assert_eq!(error.code(), ErrorCode::RCP_015);
    assert_eq!(error.code().as_str(), "E-RCP-015");
    assert!(crate::error_codes::REGISTRY.contains(&ErrorCode::RCP_015));
    assert_eq!(
        error.to_string(),
        "User anna created or edited draft 4 of recipe R-1. A different user must approve it."
    );
}

#[test]
fn content_variants_have_their_code_registry_row_and_message() {
    let tag = || ParameterTag::parse("SPEED").unwrap();
    let leaf = || LeafPath::parse(".zones[2]").unwrap();
    let cases = [
        (
            RecipeError::ValueOutOfLimit {
                tag: tag(),
                leaf: leaf(),
                value: "150".to_owned(),
                min: "0".to_owned(),
                max: "100".to_owned(),
            },
            ErrorCode::RCP_004,
            "Value 150 of SPEED.zones[2] is outside the process limit 0 to 100.",
        ),
        (
            RecipeError::ValueShapeMismatch {
                tag: tag(),
                leaf: leaf(),
                expected: Expected::Bool,
            },
            ErrorCode::RCP_005,
            "Value of SPEED.zones[2] does not match the type. Expected true or false.",
        ),
        (
            RecipeError::LimitOutsideTypeRange {
                tag: tag(),
                leaf: leaf(),
            },
            ErrorCode::RCP_006,
            "Process limit of SPEED.zones[2] is outside the range of the data type.",
        ),
        (
            RecipeError::LeafPathNotFound {
                tag: tag(),
                leaf: leaf(),
            },
            ErrorCode::RCP_017,
            "Path .zones[2] does not exist in the type of SPEED.",
        ),
        (
            RecipeError::MinGreaterThanMax {
                tag: tag(),
                leaf: leaf(),
                min: "10".to_owned(),
                max: "5".to_owned(),
            },
            ErrorCode::RCP_018,
            "The minimum 10 of SPEED.zones[2] is greater than the maximum 5.",
        ),
        (
            RecipeError::LimitNotApplicable {
                tag: tag(),
                leaf: leaf(),
            },
            ErrorCode::RCP_019,
            "SPEED.zones[2] does not accept this limit. A BOOL, CHAR, or WCHAR leaf has no limit. A string leaf has only a maximum length.",
        ),
    ];
    for (error, code, message) in cases {
        assert_eq!(error.code(), code);
        assert!(crate::error_codes::REGISTRY.contains(&code));
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn from_vec_of_nothing_gives_none() {
    assert!(RecipeErrors::from_vec(Vec::new()).is_none());
}

#[test]
fn list_of_one_error_has_length_one_and_its_code() {
    let errors = RecipeErrors::from(lang("xx-"));
    assert_eq!(errors.len(), 1);
    assert_eq!(errors.code(), ErrorCode::RCP_027);
}

#[test]
fn push_keeps_insertion_order_and_the_first_code() {
    let mut errors = RecipeErrors::from(lang("first-"));
    errors.push(RecipeError::InvalidLangCode {
        lang: "second-".to_owned(),
    });
    assert_eq!(errors.len(), 2);
    let texts: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert_eq!(
        texts,
        vec![lang("first-").to_string(), lang("second-").to_string(),]
    );
    assert_eq!(errors.code(), ErrorCode::RCP_027);
    assert_eq!(errors.first().to_string(), lang("first-").to_string());
}

#[test]
fn into_vec_gives_all_entries_in_order() {
    let errors = RecipeErrors::from_vec(vec![lang("a-"), lang("b-")]).unwrap();
    let entries = errors.into_vec();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].to_string(), lang("a-").to_string());
}

#[test]
fn display_writes_one_line_for_each_entry_without_trailing_newline() {
    let mut errors = RecipeErrors::from(lang("first-"));
    errors.push(lang("second-"));
    let expected = format!("{}\n{}", lang("first-"), lang("second-"));
    assert_eq!(errors.to_string(), expected);
}

#[test]
fn recipe_error_converts_into_draft_error_with_its_code() {
    let error = DraftError::from(lang("xx-"));
    assert_eq!(error.code(), ErrorCode::RCP_027);
}

#[test]
fn plant_error_converts_into_draft_error_with_its_code() {
    let path = UnitPath::parse("U1.9").unwrap();
    let error = DraftError::from(PlantError::UnitNotFound { path });
    assert_eq!(error.code(), ErrorCode::PLT_020);
}

#[test]
fn draft_error_of_a_list_has_the_code_of_its_first_entry() {
    let mut errors = RecipeErrors::from(lang("first-"));
    errors.push(lang("second-"));
    let error = DraftError::from(errors);
    assert_eq!(error.code(), ErrorCode::RCP_027);
}
