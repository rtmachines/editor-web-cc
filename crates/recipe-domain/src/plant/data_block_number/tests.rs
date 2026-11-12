use super::*;
use crate::error_codes::ErrorCode;

const OUT_OF_RANGE: RangeReason = RangeReason::OutOfRange { min: 1, max: 65535 };

#[test]
fn data_block_number_boundaries_pass() {
    assert_eq!(
        DataBlockNumber::parse("1").map(DataBlockNumber::value),
        Ok(1)
    );
    assert_eq!(
        DataBlockNumber::parse("65535").map(DataBlockNumber::value),
        Ok(65535)
    );
    assert_eq!(
        DataBlockNumber::parse("102").map(DataBlockNumber::value),
        Ok(102)
    );
}

#[test]
fn data_block_number_zero_fails() {
    assert_eq!(DataBlockNumber::parse("0"), Err(OUT_OF_RANGE));
}

#[test]
fn data_block_number_above_the_range_fails() {
    assert_eq!(DataBlockNumber::parse("65536"), Err(OUT_OF_RANGE));
    assert_eq!(
        DataBlockNumber::parse("99999999999999999999"),
        Err(OUT_OF_RANGE)
    );
}

#[test]
fn data_block_number_that_is_not_a_number_fails() {
    for text in ["-1", "x", "", " 1", "1 ", "+1", "1.0", "0x1", "\u{0663}"] {
        assert_eq!(
            DataBlockNumber::parse(text),
            Err(RangeReason::NotANumber),
            "{text:?}"
        );
    }
}

fn machine_code() -> MachineCode {
    MachineCode::parse("M3").unwrap()
}

fn number(text: &str) -> Option<DataBlockNumber> {
    Some(DataBlockNumber::parse(text).unwrap())
}

#[test]
fn check_data_block_without_number_on_s7_gives_missing() {
    for platform in [
        PlcPlatform::S7_300,
        PlcPlatform::S7_1200,
        PlcPlatform::S7_1500,
    ] {
        let error = check_data_block(machine_code(), platform, None).unwrap_err();
        assert!(matches!(
            &error,
            PlantError::DataBlockMissing { code, platform: reported }
                if *code == machine_code() && *reported == platform
        ));
        assert_eq!(error.code(), ErrorCode::PLT_050);
    }
}

#[test]
fn check_data_block_with_number_on_iec_gives_not_on_platform() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        let error = check_data_block(machine_code(), platform, number("10")).unwrap_err();
        assert!(matches!(
            &error,
            PlantError::DataBlockNotOnPlatform { code, platform: reported }
                if *code == machine_code() && *reported == platform
        ));
        assert_eq!(error.code(), ErrorCode::PLT_051);
    }
}

#[test]
fn check_data_block_above_limit_on_s7_300_gives_out_of_range() {
    let error = check_data_block(machine_code(), PlcPlatform::S7_300, number("16001")).unwrap_err();
    assert!(matches!(
        &error,
        PlantError::DataBlockOutOfRange { code, number: 16_001, max: 16_000 }
            if *code == machine_code()
    ));
    assert_eq!(error.code(), ErrorCode::PLT_024);
}

#[test]
fn check_data_block_above_limit_on_s7_1500_gives_out_of_range() {
    let error =
        check_data_block(machine_code(), PlcPlatform::S7_1500, number("60000")).unwrap_err();
    assert_eq!(error.code(), ErrorCode::PLT_024);
}

#[test]
fn check_data_block_at_the_limit_passes() {
    assert!(check_data_block(machine_code(), PlcPlatform::S7_300, number("16000")).is_ok());
    assert!(check_data_block(machine_code(), PlcPlatform::S7_1500, number("59999")).is_ok());
    assert!(check_data_block(machine_code(), PlcPlatform::S7_1200, number("1")).is_ok());
}

#[test]
fn check_data_block_without_number_on_iec_passes() {
    for platform in [PlcPlatform::CodesysIec, PlcPlatform::TwinCat3] {
        assert!(check_data_block(machine_code(), platform, None).is_ok());
    }
}
