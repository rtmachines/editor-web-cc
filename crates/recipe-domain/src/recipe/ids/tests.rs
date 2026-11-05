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
