use super::*;

#[test]
fn text_check_passes_one_and_max_characters() {
    assert_eq!(check_text("a", 3), Ok(()));
    assert_eq!(check_text("abc", 3), Ok(()));
}

#[test]
fn text_check_counts_characters_not_bytes() {
    assert_eq!(check_text("\u{105}\u{107}\u{119}", 3), Ok(()));
}

#[test]
fn text_check_refuses_an_empty_text() {
    assert_eq!(check_text("", 3), Err(ConnectionTextReason::Empty));
}

#[test]
fn text_check_refuses_more_than_max_characters() {
    assert_eq!(
        check_text("abcd", 3),
        Err(ConnectionTextReason::TooLong { max: 3 })
    );
}

#[test]
fn text_check_refuses_every_kind_of_control_character() {
    for text in ["a\0b", "a\tb", "a\nb", "a\rb", "a\u{7f}b", "a\u{85}b"] {
        assert_eq!(
            check_text(text, 10),
            Err(ConnectionTextReason::ControlCharacter),
            "{text:?}"
        );
    }
}

#[test]
fn text_check_reports_length_before_control_character() {
    assert_eq!(
        check_text("a\0bcd", 3),
        Err(ConnectionTextReason::TooLong { max: 3 })
    );
}
