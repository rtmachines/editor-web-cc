use super::*;

const STRING: StringKind = StringKind::String;
const WSTRING: StringKind = StringKind::Wstring;

fn ok(kind: StringKind, platform: PlcPlatform, length: u32) -> bool {
    StringLength::parse(kind, platform, length).is_ok()
}

#[test]
fn string_on_codesys_accepts_1_and_255_and_rejects_256() {
    assert!(ok(STRING, PlcPlatform::CodesysIec, 1));
    assert!(ok(STRING, PlcPlatform::CodesysIec, 255));
    assert_eq!(
        StringLength::parse(STRING, PlcPlatform::CodesysIec, 256),
        Err(StringLengthReason::OutOfRange)
    );
}

#[test]
fn string_on_twincat3_accepts_255_and_rejects_256() {
    assert!(ok(STRING, PlcPlatform::TwinCat3, 255));
    assert_eq!(
        StringLength::parse(STRING, PlcPlatform::TwinCat3, 256),
        Err(StringLengthReason::OutOfRange)
    );
}

#[test]
fn string_on_s7_platforms_accepts_254_and_rejects_255() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        assert!(ok(STRING, platform, 1), "{platform:?}");
        assert!(ok(STRING, platform, 254), "{platform:?}");
        assert_eq!(
            StringLength::parse(STRING, platform, 255),
            Err(StringLengthReason::OutOfRange),
            "{platform:?}"
        );
    }
}

#[test]
fn zero_length_is_out_of_range_on_every_platform() {
    for platform in PlcPlatform::ALL {
        assert_eq!(
            StringLength::parse(STRING, platform, 0),
            Err(StringLengthReason::OutOfRange),
            "{platform:?}"
        );
    }
}

#[test]
fn length_beyond_u16_is_out_of_range() {
    assert_eq!(
        StringLength::parse(STRING, PlcPlatform::CodesysIec, 65_536),
        Err(StringLengthReason::OutOfRange)
    );
}

#[test]
fn wstring_on_codesys_and_twincat_accepts_255_and_rejects_256() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        assert!(ok(WSTRING, platform, 255), "{platform:?}");
        assert_eq!(
            StringLength::parse(WSTRING, platform, 256),
            Err(StringLengthReason::OutOfRange),
            "{platform:?}"
        );
    }
}

#[test]
fn wstring_on_s7_1200_and_1500_accepts_16382_and_rejects_16383() {
    for platform in [PlcPlatform::S7_1200, PlcPlatform::S7_1500] {
        assert!(ok(WSTRING, platform, 16_382), "{platform:?}");
        assert_eq!(
            StringLength::parse(WSTRING, platform, 16_383),
            Err(StringLengthReason::OutOfRange),
            "{platform:?}"
        );
    }
}

#[test]
fn wstring_on_s7_300_is_not_on_platform() {
    for length in [1, 254, 16_382] {
        assert_eq!(
            StringLength::parse(WSTRING, PlcPlatform::S7_300, length),
            Err(StringLengthReason::NotOnPlatform),
            "{length}"
        );
    }
}

#[test]
fn default_length_matches_the_seed_rows_for_all_ten_combinations() {
    let expected = [
        (STRING, PlcPlatform::S7_300, Some(254)),
        (STRING, PlcPlatform::S7_1200, Some(254)),
        (STRING, PlcPlatform::S7_1500, Some(254)),
        (STRING, PlcPlatform::CodesysIec, Some(80)),
        (STRING, PlcPlatform::TwinCat3, Some(80)),
        (WSTRING, PlcPlatform::S7_300, None),
        (WSTRING, PlcPlatform::S7_1200, Some(254)),
        (WSTRING, PlcPlatform::S7_1500, Some(254)),
        (WSTRING, PlcPlatform::CodesysIec, Some(80)),
        (WSTRING, PlcPlatform::TwinCat3, Some(80)),
    ];
    for (kind, platform, default) in expected {
        assert_eq!(
            StringLength::default_length(kind, platform),
            default,
            "{kind:?} on {platform:?}"
        );
    }
}

#[test]
fn default_length_is_valid_on_its_platform() {
    for (kind, platform) in [
        (STRING, PlcPlatform::S7_1200),
        (WSTRING, PlcPlatform::TwinCat3),
    ] {
        let default = StringLength::default_length(kind, platform);
        let length = default.and_then(|n| StringLength::parse(kind, platform, u32::from(n)).ok());
        assert!(length.is_some(), "{kind:?} on {platform:?}");
    }
}

#[test]
fn size_of_string_20_is_22_bytes() {
    let length = StringLength::parse(STRING, PlcPlatform::S7_1500, 20);
    assert_eq!(length.map(|l| l.size_bytes(STRING)), Ok(Some(22)));
}

#[test]
fn size_of_wstring_80_is_164_bytes() {
    let length = StringLength::parse(WSTRING, PlcPlatform::S7_1500, 80);
    assert_eq!(length.map(|l| l.size_bytes(WSTRING)), Ok(Some(164)));
}

#[test]
fn size_of_the_largest_lengths_fits_u32() {
    let string = StringLength::parse(STRING, PlcPlatform::S7_1500, 254);
    let wstring = StringLength::parse(WSTRING, PlcPlatform::S7_1500, 16_382);
    assert_eq!(string.map(|l| l.size_bytes(STRING)), Ok(Some(256)));
    assert_eq!(wstring.map(|l| l.size_bytes(WSTRING)), Ok(Some(32_768)));
}

#[test]
fn value_and_display_write_the_number() {
    let length = StringLength::parse(STRING, PlcPlatform::CodesysIec, 254);
    assert_eq!(length.map(StringLength::value), Ok(254));
    assert_eq!(length.map(|l| l.to_string()), Ok("254".to_owned()));
}
