use super::*;

#[test]
fn user_agent_of_256_characters_stays_whole() {
    let header = "a".repeat(256);
    assert_eq!(UserAgent::from(header.as_str()).as_str(), header);
}

#[test]
fn user_agent_of_257_characters_keeps_the_first_256() {
    let header = "b".repeat(257);
    let kept = UserAgent::from(header.as_str());
    assert_eq!(kept.as_str().chars().count(), 256);
    assert_eq!(kept.as_str(), &header[..256]);
}

#[test]
fn user_agent_keeps_whole_multi_byte_characters() {
    let header = "\u{fc}".repeat(300);
    let kept = UserAgent::from(header.as_str());
    assert_eq!(kept.as_str().chars().count(), 256);
    assert_eq!(kept.as_str(), "\u{fc}".repeat(256));
}

#[test]
fn request_id_displays_the_uuid() {
    let raw = Uuid::from_u128(42);
    assert_eq!(RequestId::from(raw).to_string(), raw.to_string());
    assert_eq!(*RequestId::from(raw).as_uuid(), raw);
}
