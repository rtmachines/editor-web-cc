use super::*;

fn parsed(text: &str) -> Result<String, AmsNetIdReason> {
    AmsNetId::parse(text).map(|id| id.as_str().to_owned())
}

#[test]
fn ams_net_id_of_six_numbers_passes() {
    for text in [
        "192.168.1.10.1.1",
        "0.0.0.0.0.0",
        "255.255.255.255.255.255",
        "5.80.201.232.1.1",
    ] {
        assert_eq!(parsed(text), Ok(text.to_owned()), "{text:?}");
    }
}

#[test]
fn ams_net_id_with_a_number_above_255_fails() {
    for text in [
        "192.168.1.10.1.256",
        "256.0.0.0.0.0",
        "1.2.3.4.5.1000",
        "1.2.3.4.5.99999999999",
    ] {
        assert_eq!(parsed(text), Err(AmsNetIdReason::Number), "{text:?}");
    }
}

#[test]
fn ams_net_id_with_a_leading_zero_fails() {
    for text in ["192.168.01.10.1.1", "00.0.0.0.0.0", "1.2.3.4.5.007"] {
        assert_eq!(parsed(text), Err(AmsNetIdReason::Number), "{text:?}");
    }
}

#[test]
fn ams_net_id_with_five_or_seven_numbers_fails() {
    for text in ["192.168.1.10.1", "192.168.1.10.1.1.1", "1", ""] {
        assert_eq!(parsed(text), Err(AmsNetIdReason::Count), "{text:?}");
    }
}

#[test]
fn ams_net_id_with_an_empty_number_fails() {
    for text in [".168.1.10.1.1", "192.168.1.10.1.", "192..1.10.1.1", "....."] {
        assert_eq!(parsed(text), Err(AmsNetIdReason::Number), "{text:?}");
    }
}

#[test]
fn ams_net_id_with_a_character_that_is_not_a_digit_fails() {
    for text in [
        "192.168.1.10.1.x",
        "192.168.1.10.1.-1",
        "192.168.1.10.1.+1",
        " 1.2.3.4.5.6",
        "1.2.3.4.5.6 ",
        "1.2.3.4.5.\u{663}",
    ] {
        assert_eq!(parsed(text), Err(AmsNetIdReason::Number), "{text:?}");
    }
}

#[test]
fn ams_net_id_prints_its_text() {
    assert_eq!(
        AmsNetId::parse("1.2.3.4.5.6").map(|id| id.to_string()),
        Ok("1.2.3.4.5.6".to_owned())
    );
}
