use super::*;
use crate::plant::UnitPath;

fn missing(text: &str) -> PlantError {
    PlantError::UnitNotFound {
        path: UnitPath::parse(text).unwrap(),
    }
}

fn texts(errors: &PlantErrors) -> Vec<String> {
    errors.iter().map(ToString::to_string).collect()
}

#[test]
fn from_vec_of_nothing_gives_none() {
    assert!(PlantErrors::from_vec(Vec::new()).is_none());
}

#[test]
fn list_of_one_error_has_length_one_and_is_never_empty() {
    let errors = PlantErrors::from(missing("U1"));
    assert_eq!(errors.len(), 1);
    assert!(!errors.is_empty());
    assert_eq!(errors.first().to_string(), missing("U1").to_string());
}

#[test]
fn push_keeps_insertion_order() {
    let mut errors = PlantErrors::from(missing("U1"));
    errors.push(missing("U2"));
    errors.push(missing("U3"));
    assert_eq!(errors.len(), 3);
    let expected: Vec<String> = ["U1", "U2", "U3"]
        .iter()
        .map(|text| missing(text).to_string())
        .collect();
    assert_eq!(texts(&errors), expected);
}

#[test]
fn from_vec_and_into_vec_keep_order() {
    let errors = PlantErrors::from_vec(vec![missing("U2"), missing("U1")]).unwrap();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors.first().to_string(), missing("U2").to_string());
    let entries = errors.into_vec();
    let order: Vec<String> = entries.iter().map(ToString::to_string).collect();
    assert_eq!(
        order,
        vec![missing("U2").to_string(), missing("U1").to_string()]
    );
}

#[test]
fn extend_appends_after_the_existing_entries() {
    let mut errors = PlantErrors::from(missing("U1"));
    errors.extend([missing("U2"), missing("U3")]);
    let expected: Vec<String> = ["U1", "U2", "U3"]
        .iter()
        .map(|text| missing(text).to_string())
        .collect();
    assert_eq!(texts(&errors), expected);
}

#[test]
fn display_writes_one_line_for_each_entry_without_trailing_newline() {
    let mut errors = PlantErrors::from(missing("U1"));
    errors.push(missing("U2"));
    let expected = format!("{}\n{}", missing("U1"), missing("U2"));
    assert_eq!(errors.to_string(), expected);
}

#[test]
fn list_is_a_std_error() {
    fn takes_error(_: &dyn std::error::Error) {}
    takes_error(&PlantErrors::from(missing("U1")));
}
