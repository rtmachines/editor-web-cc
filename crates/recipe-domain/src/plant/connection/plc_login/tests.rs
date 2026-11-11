use super::*;

#[test]
fn plc_login_of_one_and_sixty_four_characters_passes() {
    let longest = "a".repeat(64);
    assert_eq!(PlcLogin::parse("a").as_ref().map(PlcLogin::as_str), Ok("a"));
    assert_eq!(
        PlcLogin::parse(&longest).as_ref().map(PlcLogin::as_str),
        Ok(longest.as_str())
    );
}

#[test]
fn plc_login_counts_characters_not_bytes() {
    let login = "\u{105}".repeat(64);
    assert_eq!(
        PlcLogin::parse(&login).as_ref().map(PlcLogin::as_str),
        Ok(login.as_str())
    );
}

#[test]
fn empty_plc_login_fails() {
    assert_eq!(PlcLogin::parse(""), Err(ConnectionTextReason::Empty));
}

#[test]
fn plc_login_of_sixty_five_characters_fails() {
    assert_eq!(
        PlcLogin::parse(&"a".repeat(65)),
        Err(ConnectionTextReason::TooLong { max: 64 })
    );
}

#[test]
fn plc_login_with_a_control_character_fails() {
    for text in ["a\0b", "a\tb", "a\nb", "a\u{7f}b"] {
        assert_eq!(
            PlcLogin::parse(text),
            Err(ConnectionTextReason::ControlCharacter),
            "{text:?}"
        );
    }
}

#[test]
fn plc_login_displays_as_its_text() {
    let login = PlcLogin::parse("adapter").expect("a valid login must parse");
    assert_eq!(login.to_string(), "adapter");
}
