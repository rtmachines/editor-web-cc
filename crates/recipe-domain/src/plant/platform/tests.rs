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

// ADR-0116 (c), ADR-0140 (a): the highest data block number of each platform.
#[test]
fn max_data_block_is_16000_on_s7_300() {
    assert_eq!(PlcPlatform::S7_300.max_data_block(), Some(16_000));
}

#[test]
fn max_data_block_is_59999_on_s7_1200_and_s7_1500() {
    assert_eq!(PlcPlatform::S7_1200.max_data_block(), Some(59_999));
    assert_eq!(PlcPlatform::S7_1500.max_data_block(), Some(59_999));
}

#[test]
fn max_data_block_is_none_on_codesys_iec_and_twincat3() {
    assert_eq!(PlcPlatform::CodesysIec.max_data_block(), None);
    assert_eq!(PlcPlatform::TwinCat3.max_data_block(), None);
}

fn assert_round_trip(platform: PlcPlatform, text: &str) {
    assert_eq!(PlcPlatform::parse(text), Ok(platform));
    assert_eq!(platform.to_string(), text);
    assert_eq!(platform.as_str(), text);
}

fn assert_fragment(text: &str) {
    assert!(
        text.chars().next().is_some_and(|c| c.is_ascii_lowercase()),
        "{text}"
    );
    assert!(!text.ends_with('.'), "{text}");
}

// domain-model.md section 2.1 (PlcPlatform): the reason names the platform.
#[test]
fn unknown_reason_text_names_the_platform_as_a_lower_case_fragment() {
    let text = PlcPlatformReason::Unknown.to_string();
    assert_fragment(&text);
    assert!(text.contains("platform"), "{text}");
}
