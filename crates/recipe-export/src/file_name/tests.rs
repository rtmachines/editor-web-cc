use recipe_domain::export::ExportStatus;
use recipe_domain::recipe::ReferenceNumber;

use super::file_name;

fn reference(text: &str) -> ReferenceNumber {
    ReferenceNumber::parse(text).unwrap()
}

#[test]
fn approved_version_has_no_prefix() {
    let name = file_name(&reference("R-100"), 3, ExportStatus::Approved);

    assert_eq!(name, "recipe_R-100_v3.json");
}

#[test]
fn retired_version_has_the_prefix_retired() {
    let name = file_name(&reference("R-100"), 2, ExportStatus::Retired);

    assert_eq!(name, "RETIRED_recipe_R-100_v2.json");
}

#[test]
fn reference_with_underscore_and_digits_is_kept_as_is() {
    let name = file_name(&reference("a_1-b"), 12, ExportStatus::Approved);

    assert_eq!(name, "recipe_A_1-B_v12.json");
}

#[test]
fn largest_version_number_is_written_in_full() {
    let name = file_name(&reference("R1"), u32::MAX, ExportStatus::Retired);

    assert_eq!(name, "RETIRED_recipe_R1_v4294967295.json");
}

#[test]
fn name_of_the_longest_reference_has_only_header_safe_characters() {
    let text = format!("A{}", "_-9Z".repeat(12));
    let name = file_name(&reference(&text), 1, ExportStatus::Retired);

    assert!(
        name.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.')
    );
}
