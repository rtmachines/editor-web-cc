use super::*;

#[test]
fn the_four_mode_names_pass() {
    assert_eq!(CommMode::parse("Ads"), Ok(CommMode::Ads));
    assert_eq!(CommMode::parse("OpcUa"), Ok(CommMode::OpcUa));
    assert_eq!(CommMode::parse("CodesysV3"), Ok(CommMode::CodesysV3));
    assert_eq!(CommMode::parse("ModbusTcp"), Ok(CommMode::ModbusTcp));
}

#[test]
fn an_unknown_mode_name_fails() {
    assert_eq!(CommMode::parse("Profinet"), Err(ModeReason::Unknown));
}

#[test]
fn a_mode_name_is_case_sensitive() {
    for text in ["ads", "ADS", "opcua", "OPCUA", "codesysv3", "modbustcp"] {
        assert_eq!(CommMode::parse(text), Err(ModeReason::Unknown), "{text:?}");
    }
}

#[test]
fn a_mode_name_with_extra_characters_fails() {
    for text in ["", " Ads", "Ads ", "Ads\n", "Opc Ua", "Ads\0"] {
        assert_eq!(CommMode::parse(text), Err(ModeReason::Unknown), "{text:?}");
    }
}

#[test]
fn as_str_returns_the_name_that_parse_accepts() {
    for mode in [
        CommMode::Ads,
        CommMode::OpcUa,
        CommMode::CodesysV3,
        CommMode::ModbusTcp,
    ] {
        assert_eq!(CommMode::parse(mode.as_str()), Ok(mode));
    }
}

#[test]
fn as_str_returns_the_exact_names() {
    assert_eq!(CommMode::Ads.as_str(), "Ads");
    assert_eq!(CommMode::OpcUa.as_str(), "OpcUa");
    assert_eq!(CommMode::CodesysV3.as_str(), "CodesysV3");
    assert_eq!(CommMode::ModbusTcp.as_str(), "ModbusTcp");
}

#[test]
fn the_mode_displays_as_its_name_and_the_reason_has_a_message() {
    assert_eq!(CommMode::OpcUa.to_string(), "OpcUa");
    assert!(!ModeReason::Unknown.to_string().is_empty());
}
