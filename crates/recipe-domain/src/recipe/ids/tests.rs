use super::*;
use uuid::Uuid;

#[test]
fn recipe_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(101);
    assert_eq!(RecipeId::from(raw).as_uuid(), &raw);
}

#[test]
fn recipe_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(102);
    assert_eq!(RecipeId::from(raw).to_string(), raw.to_string());
}

#[test]
fn recipe_version_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(103);
    assert_eq!(RecipeVersionId::from(raw).as_uuid(), &raw);
}

#[test]
fn recipe_version_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(104);
    assert_eq!(RecipeVersionId::from(raw).to_string(), raw.to_string());
}

#[test]
fn parameter_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(105);
    assert_eq!(ParameterId::from(raw).as_uuid(), &raw);
}

#[test]
fn parameter_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(106);
    assert_eq!(ParameterId::from(raw).to_string(), raw.to_string());
}

#[test]
fn parameter_key_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(107);
    assert_eq!(ParameterKey::from(raw).as_uuid(), &raw);
}

#[test]
fn parameter_key_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(108);
    assert_eq!(ParameterKey::from(raw).to_string(), raw.to_string());
}

#[test]
fn parameter_key_parse_of_a_uuid_text_keeps_the_uuid() {
    let raw = Uuid::from_u128(109);
    let parsed = ParameterKey::parse(&raw.to_string());
    assert_eq!(parsed.map(|key| *key.as_uuid()), Ok(raw));
}

#[test]
fn parameter_key_parse_of_not_a_uuid_gives_format() {
    assert_eq!(
        ParameterKey::parse("not-a-uuid"),
        Err(ParameterKeyReason::Format)
    );
}

// domain-model.md section 2.2: a parameter key is a UUID, so the text names the UUID.
#[test]
fn parameter_key_format_text_names_a_uuid_as_a_lower_case_fragment() {
    assert!(ParameterKey::parse(&Uuid::from_u128(110).to_string()).is_ok());
    assert_eq!(ParameterKey::parse(""), Err(ParameterKeyReason::Format));
    let text = ParameterKeyReason::Format.to_string();
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
    assert!(text.contains("UUID"), "{text}");
}
