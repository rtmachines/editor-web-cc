//! Tests of the lines of one domain error and of a `DraftError` with an input per error
//! (verifier finding M1, design D9).

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::plant::{LeafPath, PlantError, S7Address, UnitPath};
use recipe_domain::recipe::{DraftError, ParameterTag, RecipeError, RecipeErrors};

use super::lists::line_view;
use super::{DraftFault, FieldError, draft_lines, leaf_of};
use crate::i18n::{Bundles, Lang};

fn tag(text: &str) -> ParameterTag {
    ParameterTag::parse(text).unwrap()
}

fn misaligned() -> RecipeError {
    RecipeError::S7AddressMisaligned {
        tag: tag("SPEED"),
        address: S7Address::parse("DB1.DBW1").unwrap(),
    }
}

fn out_of_limit(path: &str) -> RecipeError {
    RecipeError::ValueOutOfLimit {
        tag: tag("SPEED"),
        leaf: LeafPath::parse(path).unwrap(),
        value: "3500".to_owned(),
        min: "0".to_owned(),
        max: "3000".to_owned(),
    }
}

fn unit_not_found() -> PlantError {
    PlantError::UnitNotFound {
        path: UnitPath::parse("U9").unwrap(),
    }
}

/// The input of a fault: the value input of its leaf, the address input, or the unit input.
fn input_of(fault: DraftFault<'_>) -> Option<String> {
    match fault {
        DraftFault::Recipe(error) => match leaf_of(error) {
            Some(leaf) => Some(format!(
                "value-{}",
                leaf.to_string().trim_matches(['[', ']'])
            )),
            None if error.code() == ErrorCode::RCP_023 => Some("s7_address".to_owned()),
            None => Some("tag".to_owned()),
        },
        DraftFault::Plant(_error) => Some("unit_path".to_owned()),
    }
}

fn targets(lines: &[FieldError]) -> Vec<Option<&str>> {
    lines.iter().map(FieldError::target).collect()
}

#[test]
fn draft_lines_a_single_recipe_error_gets_the_input_that_the_caller_names() {
    let line = FieldError::recipe(&misaligned(), Some("s7_address".to_owned()));
    assert_eq!(line.code(), ErrorCode::RCP_023);
    assert_eq!(line.target(), Some("s7_address"));
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        line_view(&line, &bundles, Lang::En).message,
        "Address DB1.DBW1 of SPEED must start at an even byte."
    );
}

#[test]
fn draft_lines_a_single_plant_error_gets_the_input_that_the_caller_names() {
    let line = FieldError::plant(&unit_not_found(), Some("unit_path".to_owned()));
    assert_eq!(line.code(), ErrorCode::PLT_020);
    assert_eq!(line.target(), Some("unit_path"));
    let bundles = Bundles::load().unwrap();
    assert_eq!(
        line_view(&line, &bundles, Lang::En).message,
        "Unit U9 does not exist on this machine. Select an existing unit."
    );
    assert_eq!(FieldError::plant(&unit_not_found(), None).target(), None);
}

#[test]
fn draft_lines_a_list_mixes_leaf_inputs_and_field_inputs_in_its_order() {
    let list = RecipeErrors::from_vec(vec![out_of_limit("[3]"), misaligned(), out_of_limit("[6]")])
        .unwrap();
    let lines = draft_lines(&DraftError::List(list), &input_of);
    assert_eq!(
        targets(&lines),
        [Some("value-3"), Some("s7_address"), Some("value-6")]
    );
    let codes: Vec<ErrorCode> = lines.iter().map(FieldError::code).collect();
    assert_eq!(
        codes,
        [ErrorCode::RCP_004, ErrorCode::RCP_023, ErrorCode::RCP_004]
    );
}

#[test]
fn draft_lines_a_single_draft_error_gives_one_line() {
    let duplicate = RecipeError::DuplicateTag {
        tag: tag("SPEED"),
        number: 3,
    };
    let recipe = draft_lines(&DraftError::Recipe(duplicate), &input_of);
    assert_eq!(targets(&recipe), [Some("tag")]);
    let plant = draft_lines(&DraftError::Plant(unit_not_found()), &input_of);
    assert_eq!(targets(&plant), [Some("unit_path")]);
    assert_eq!(plant.first().unwrap().code(), ErrorCode::PLT_020);
}
