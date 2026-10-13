//! Calendar arithmetic of the IEC literals (`domain-model.md` section 2.1, ADR-0026).
//!
//! The day count uses the days-from-civil algorithm of Howard Hinnant.
//! Every step uses checked integer arithmetic.

/// Return the days from 1970-01-01 to the given date.
///
/// Returns `None` when the date does not exist in the Gregorian calendar.
#[must_use]
pub(crate) fn days_from_civil(year: i64, month: u8, day: u8) -> Option<i64> {
    if !is_valid_date(year, month, day) {
        return None;
    }
    let shifted_year = if month <= 2 {
        year.checked_sub(1)?
    } else {
        year
    };
    let era = shifted_year.checked_div_euclid(400)?;
    let year_of_era = shifted_year.checked_sub(era.checked_mul(400)?)?;
    let shifted_month = if month > 2 {
        i64::from(month).checked_sub(3)?
    } else {
        i64::from(month).checked_add(9)?
    };
    let day_of_year = (shifted_month.checked_mul(153)?.checked_add(2)?)
        .checked_div(5)?
        .checked_add(i64::from(day))?
        .checked_sub(1)?;
    let day_of_era = year_of_era
        .checked_mul(365)?
        .checked_add(year_of_era.checked_div(4)?)?
        .checked_sub(year_of_era.checked_div(100)?)?
        .checked_add(day_of_year)?;
    era.checked_mul(146_097)?
        .checked_add(day_of_era)?
        .checked_sub(719_468)
}

/// Return the date of the given day count since 1970-01-01.
///
/// Returns `None` when the result does not fit the calendar fields.
#[must_use]
pub(crate) fn civil_from_days(days: i64) -> Option<(i64, u8, u8)> {
    let shifted = days.checked_add(719_468)?;
    let era = shifted.checked_div_euclid(146_097)?;
    let day_of_era = shifted.checked_sub(era.checked_mul(146_097)?)?;
    let year_of_era = day_of_era
        .checked_sub(day_of_era.checked_div(1460)?)?
        .checked_add(day_of_era.checked_div(36_524)?)?
        .checked_sub(day_of_era.checked_div(146_096)?)?
        .checked_div(365)?;
    let year = year_of_era.checked_add(era.checked_mul(400)?)?;
    let day_of_year = day_of_era.checked_sub(
        year_of_era
            .checked_mul(365)?
            .checked_add(year_of_era.checked_div(4)?)?
            .checked_sub(year_of_era.checked_div(100)?)?,
    )?;
    let month_index = (day_of_year.checked_mul(5)?.checked_add(2)?).checked_div(153)?;
    let day = day_of_year
        .checked_sub((month_index.checked_mul(153)?.checked_add(2)?).checked_div(5)?)?
        .checked_add(1)?;
    let month = if month_index < 10 {
        month_index.checked_add(3)?
    } else {
        month_index.checked_sub(9)?
    };
    let year = if month <= 2 {
        year.checked_add(1)?
    } else {
        year
    };
    Some((year, u8::try_from(month).ok()?, u8::try_from(day).ok()?))
}

/// Return `true` when the date exists in the Gregorian calendar.
fn is_valid_date(year: i64, month: u8, day: u8) -> bool {
    if !(1..=12).contains(&month) {
        return false;
    }
    let days_in_month = match month {
        2 if is_leap_year(year) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    (1..=days_in_month).contains(&day)
}

/// Return `true` for a leap year of the Gregorian calendar.
fn is_leap_year(year: i64) -> bool {
    let by_four = year.checked_rem_euclid(4) == Some(0);
    let by_hundred = year.checked_rem_euclid(100) == Some(0);
    let by_four_hundred = year.checked_rem_euclid(400) == Some(0);
    by_four && (!by_hundred || by_four_hundred)
}

#[cfg(test)]
mod tests {
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
}
