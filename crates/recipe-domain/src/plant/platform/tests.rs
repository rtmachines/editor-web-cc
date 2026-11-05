use super::*;

#[test]
fn codesys_iec_round_trips_through_parse_and_display() {
    assert_round_trip(PlcPlatform::CodesysIec, "CodesysIec");
}

#[test]
fn twincat3_round_trips_through_parse_and_display() {
    assert_round_trip(PlcPlatform::TwinCat3, "TwinCat3");
}

#[test]
fn s7_1200_round_trips_through_parse_and_display() {
    assert_round_trip(PlcPlatform::S7_1200, "S7_1200");
}

#[test]
fn s7_1500_round_trips_through_parse_and_display() {
    assert_round_trip(PlcPlatform::S7_1500, "S7_1500");
}

#[test]
fn s7_300_round_trips_through_parse_and_display() {
    assert_round_trip(PlcPlatform::S7_300, "S7_300");
}

#[test]
fn hyphenated_s7_name_is_rejected() {
    assert_eq!(
        PlcPlatform::parse("S7-1500"),
        Err(PlcPlatformReason::Unknown)
    );
}

#[test]
fn lower_case_name_is_rejected() {
    assert_eq!(
        PlcPlatform::parse("twincat3"),
        Err(PlcPlatformReason::Unknown)
    );
}

#[test]
fn empty_text_is_rejected() {
    assert_eq!(PlcPlatform::parse(""), Err(PlcPlatformReason::Unknown));
}

#[test]
fn is_s7_is_true_for_exactly_the_three_siemens_platforms() {
    // A new variant fails this count until a case for it exists below.
    assert_eq!(PlcPlatform::ALL.len(), 5);
    assert!(!PlcPlatform::CodesysIec.is_s7());
    assert!(!PlcPlatform::TwinCat3.is_s7());
    assert!(PlcPlatform::S7_300.is_s7());
    assert!(PlcPlatform::S7_1200.is_s7());
    assert!(PlcPlatform::S7_1500.is_s7());
}

fn assert_round_trip(platform: PlcPlatform, text: &str) {
    assert_eq!(PlcPlatform::parse(text), Ok(platform));
    assert_eq!(platform.to_string(), text);
    assert_eq!(platform.as_str(), text);
}
