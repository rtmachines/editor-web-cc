use super::{new_request_id, random_nonce, random_uuid};

#[test]
fn random_uuid_is_a_version_4_uuid_with_the_rfc_variant() {
    let id = random_uuid().unwrap();
    assert_eq!(id.get_version_num(), 4);
    assert_eq!(id.get_variant(), uuid::Variant::RFC4122);
}

#[test]
fn two_random_uuids_differ() {
    assert_ne!(random_uuid().unwrap(), random_uuid().unwrap());
}

#[test]
fn request_id_is_a_version_4_uuid() {
    let id = new_request_id().unwrap();
    assert_eq!(id.as_uuid().get_version_num(), 4);
}

#[test]
fn two_request_ids_differ() {
    assert_ne!(new_request_id().unwrap(), new_request_id().unwrap());
}

#[test]
fn nonce_has_128_bits() {
    let nonce = random_nonce().unwrap();
    assert_eq!(nonce.len(), 16);
}

#[test]
fn two_nonces_differ() {
    assert_ne!(random_nonce().unwrap(), random_nonce().unwrap());
}

#[test]
fn nonce_is_not_all_zero() {
    assert_ne!(random_nonce().unwrap(), [0_u8; 16]);
}
