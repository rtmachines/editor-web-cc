use super::*;

#[test]
fn plc_password_of_one_and_one_hundred_twenty_eight_characters_passes() {
    let longest = "a".repeat(128);
    assert_eq!(
        PlcPassword::parse("a").as_ref().map(PlcPassword::expose),
        Ok("a")
    );
    assert_eq!(
        PlcPassword::parse(&longest)
            .as_ref()
            .map(PlcPassword::expose),
        Ok(longest.as_str())
    );
}

#[test]
fn plc_password_counts_characters_not_bytes() {
    let password = "\u{105}".repeat(128);
    assert_eq!(
        PlcPassword::parse(&password)
            .as_ref()
            .map(PlcPassword::expose),
        Ok(password.as_str())
    );
}

#[test]
fn empty_plc_password_fails() {
    assert_eq!(
        PlcPassword::parse("").err(),
        Some(ConnectionTextReason::Empty)
    );
}

#[test]
fn plc_password_of_one_hundred_twenty_nine_characters_fails() {
    assert_eq!(
        PlcPassword::parse(&"a".repeat(129)).err(),
        Some(ConnectionTextReason::TooLong { max: 128 })
    );
}

#[test]
fn plc_password_permits_control_characters() {
    for text in ["a\0b", "a\tb", "a\nb", "a\u{7f}b"] {
        assert_eq!(
            PlcPassword::parse(text).as_ref().map(PlcPassword::expose),
            Ok(text),
            "{text:?}"
        );
    }
}

#[test]
fn plc_password_debug_is_redacted() {
    let password = PlcPassword::parse("s3cret-text").expect("a valid password must parse");
    let shown = format!("{password:?}");
    assert!(shown.contains("<redacted>"), "{shown}");
    assert!(!shown.contains("s3cret-text"), "{shown}");
    assert_eq!(shown, "PlcPassword(<redacted>)");
}

#[test]
fn plc_password_debug_alternate_is_redacted() {
    let password = PlcPassword::parse("s3cret-text").expect("a valid password must parse");
    let shown = format!("{password:#?}");
    assert!(!shown.contains("s3cret-text"), "{shown}");
}
