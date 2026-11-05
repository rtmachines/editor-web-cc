use super::*;

#[test]
fn epoch_is_day_zero() {
    assert_eq!(days_from_civil(1970, 1, 1), Some(0));
}

#[test]
fn october_twelfth_2026_is_day_20738() {
    assert_eq!(days_from_civil(2026, 10, 12), Some(20738));
}

#[test]
fn day_before_epoch_is_minus_one() {
    assert_eq!(days_from_civil(1969, 12, 31), Some(-1));
}

#[test]
fn leap_day_2000_exists() {
    assert!(days_from_civil(2000, 2, 29).is_some());
}

#[test]
fn leap_day_2024_exists() {
    assert!(days_from_civil(2024, 2, 29).is_some());
}

#[test]
fn leap_day_1900_does_not_exist() {
    assert_eq!(days_from_civil(1900, 2, 29), None);
}

#[test]
fn leap_day_2026_does_not_exist() {
    assert_eq!(days_from_civil(2026, 2, 29), None);
}

#[test]
fn month_thirteen_does_not_exist() {
    assert_eq!(days_from_civil(2026, 13, 1), None);
}

#[test]
fn month_zero_does_not_exist() {
    assert_eq!(days_from_civil(2026, 0, 1), None);
}

#[test]
fn day_thirty_two_does_not_exist() {
    assert_eq!(days_from_civil(2026, 1, 32), None);
}

#[test]
fn day_zero_does_not_exist() {
    assert_eq!(days_from_civil(2026, 1, 0), None);
}

#[test]
fn day_twenty_thousand_seven_hundred_thirty_eight_is_october_twelfth() {
    assert_eq!(civil_from_days(20738), Some((2026, 10, 12)));
}

#[test]
fn minus_one_is_the_day_before_epoch() {
    assert_eq!(civil_from_days(-1), Some((1969, 12, 31)));
}

#[test]
fn round_trip_holds_for_every_day_from_1900_to_2300() {
    let start = days_from_civil(1900, 1, 1).unwrap_or(i64::MIN);
    let end = days_from_civil(2300, 12, 31).unwrap_or(i64::MIN);
    for days in start..=end {
        let date = civil_from_days(days);
        assert!(date.is_some(), "day {days} has no date");
        if let Some((year, month, day)) = date {
            assert_eq!(
                days_from_civil(year, month, day),
                Some(days),
                "{year}-{month}-{day}"
            );
        }
    }
}
