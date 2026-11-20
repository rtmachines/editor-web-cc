use super::*;
use crate::error_codes::ErrorCode;

fn machine_code() -> MachineCode {
    MachineCode::parse("M7").unwrap()
}

#[test]
fn adr_0202_string_encoding_parses_two_values() {
    assert_eq!(StringEncoding::parse("Latin1"), Ok(StringEncoding::Latin1));
    assert_eq!(StringEncoding::parse("Utf8"), Ok(StringEncoding::Utf8));
    assert_eq!(StringEncoding::Latin1.as_str(), "Latin1");
    assert_eq!(StringEncoding::Utf8.as_str(), "Utf8");
    assert_eq!(StringEncoding::Utf8.to_string(), "Utf8");
}

#[test]
fn adr_0202_string_encoding_rejects_any_other_text() {
    for text in [
        "", "latin1", "LATIN1", "utf8", "UTF-8", "Utf-8", " Utf8", "Utf8 ", "Ascii",
    ] {
        assert_eq!(
            StringEncoding::parse(text),
            Err(StringEncodingReason::Unknown),
            "{text:?}"
        );
    }
}

#[test]
fn adr_0202_utf8_fails_on_s7() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let error =
            check_string_encoding(machine_code(), platform, StringEncoding::Utf8).unwrap_err();
        assert!(
            matches!(
                &error,
                PlantError::StringEncodingNotOnPlatform {
                    code,
                    encoding: StringEncoding::Utf8,
                    platform: reported,
                } if *code == machine_code() && *reported == platform
            ),
            "{error:?}"
        );
        assert_eq!(error.code(), ErrorCode::PLT_053);
    }
}

#[test]
fn adr_0202_latin1_passes_on_every_platform() {
    for platform in PlcPlatform::ALL {
        assert!(
            check_string_encoding(machine_code(), platform, StringEncoding::Latin1).is_ok(),
            "{platform}"
        );
    }
}

#[test]
fn adr_0202_utf8_passes_on_iec_platforms() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        assert!(
            check_string_encoding(machine_code(), platform, StringEncoding::Utf8).is_ok(),
            "{platform}"
        );
    }
}
