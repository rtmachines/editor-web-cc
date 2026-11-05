use super::*;
use uuid::Uuid;

#[test]
fn user_id_from_uuid_keeps_the_uuid() {
    let raw = Uuid::from_u128(201);
    assert_eq!(UserId::from(raw).as_uuid(), &raw);
}

#[test]
fn user_id_display_is_the_hyphenated_uuid() {
    let raw = Uuid::from_u128(202);
    assert_eq!(UserId::from(raw).to_string(), raw.to_string());
}
