use super::*;
use proptest::prelude::*;

#[test]
fn smallest_code_passes() {
    assert_eq!(MachineCode::parse("M1").map(MachineCode::number), Ok(1));
}

#[test]
fn largest_code_passes() {
    assert_eq!(
        MachineCode::parse("M9999999").map(MachineCode::number),
        Ok(9_999_999)
    );
}

#[test]
fn zero_fails() {
    assert_eq!(MachineCode::parse("M0"), Err(MachineCodeReason::Format));
}

#[test]
fn leading_zero_fails() {
    assert_eq!(MachineCode::parse("M01"), Err(MachineCodeReason::Format));
}

#[test]
fn eight_digits_fail() {
    assert_eq!(
        MachineCode::parse("M10000000"),
        Err(MachineCodeReason::Format)
    );
}

#[test]
fn lower_case_prefix_fails() {
    assert_eq!(MachineCode::parse("m1"), Err(MachineCodeReason::Format));
}

#[test]
fn prefix_without_digits_fails() {
    assert_eq!(MachineCode::parse("M"), Err(MachineCodeReason::Format));
}

#[test]
fn letter_inside_digits_fails() {
    assert_eq!(MachineCode::parse("M12a"), Err(MachineCodeReason::Format));
}

#[test]
fn leading_space_fails() {
    assert_eq!(MachineCode::parse(" M1"), Err(MachineCodeReason::Format));
}

#[test]
fn plus_sign_fails() {
    assert_eq!(MachineCode::parse("M+1"), Err(MachineCodeReason::Format));
}

#[test]
fn arabic_digit_fails() {
    assert_eq!(
        MachineCode::parse("M\u{0661}"),
        Err(MachineCodeReason::Format)
    );
}

proptest! {
    #[test]
    fn parse_and_display_round_trip(n in 1u32..=9_999_999) {
        let text = format!("M{n}");
        let code = MachineCode::parse(&text).map_err(|e| TestCaseError::fail(e.to_string()))?;
        prop_assert_eq!(code.to_string(), text);
        prop_assert_eq!(code.number(), n);
    }
}
