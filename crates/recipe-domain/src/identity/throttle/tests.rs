use std::time::Duration;

use proptest::prelude::*;
use time::OffsetDateTime;

use super::*;
use crate::error_codes::ErrorCode;
use crate::timestamp::Timestamp;

const RELIABILITY: &str = include_str!("../../../../../docs/architecture/reliability.md");

fn at(seconds: i64) -> Timestamp {
    Timestamp::from(OffsetDateTime::from_unix_timestamp(seconds).unwrap())
}

fn at_millis(millis: i64) -> Timestamp {
    let nanos = i128::from(millis).checked_mul(1_000_000).unwrap();
    Timestamp::from(OffsetDateTime::from_unix_timestamp_nanos(nanos).unwrap())
}

fn count(pair: u32, last: Option<Timestamp>, now: Timestamp) -> AttemptCount {
    AttemptCount {
        pair_failures: pair,
        last_pair_failure: last,
        address_failures: pair,
        oldest_address_failure: last,
        now,
    }
}

fn decide(count: &AttemptCount) -> ThrottleDecision {
    LoginThrottle::default().decide(count)
}

fn row(name: &str) -> &'static str {
    let prefix = format!("| `{name}` |");
    RELIABILITY
        .lines()
        .find(|line| line.starts_with(&prefix))
        .unwrap()
}

#[test]
fn typed_login_folds_ascii_letters() {
    assert_eq!(
        TypedLogin::parse("Admin.User").unwrap().as_str(),
        "admin.user"
    );
}

#[test]
fn typed_login_keeps_text_outside_the_login_grammar() {
    assert_eq!(TypedLogin::parse("A b!").unwrap().as_str(), "a b!");
    assert_eq!(TypedLogin::parse("a").unwrap().as_str(), "a");
}

#[test]
fn typed_login_counts_characters_not_bytes() {
    assert!(TypedLogin::parse(&"ł".repeat(64)).is_ok());
    assert_eq!(
        TypedLogin::parse(&"ł".repeat(65)),
        Err(TypedLoginReason::Length)
    );
}

#[test]
fn typed_login_of_64_characters_passes_and_65_fails() {
    assert!(TypedLogin::parse(&"a".repeat(64)).is_ok());
    assert_eq!(
        TypedLogin::parse(&"a".repeat(65)),
        Err(TypedLoginReason::Length)
    );
}

#[test]
fn empty_typed_login_fails() {
    assert_eq!(TypedLogin::parse(""), Err(TypedLoginReason::Length));
}

#[test]
fn typed_login_with_a_control_character_fails() {
    for text in ["\u{0}", "ad\tmin", "\u{7f}", "\u{85}", "admin\u{0}"] {
        assert_eq!(
            TypedLogin::parse(text),
            Err(TypedLoginReason::Control),
            "{text:?}"
        );
    }
}

#[test]
fn control_reason_display_names_the_rule() {
    assert_eq!(
        TypedLoginReason::Control.to_string(),
        "a typed login holds no control character"
    );
}

#[test]
fn typed_login_debug_hides_the_text() {
    let typed = TypedLogin::parse("jkowalski").unwrap();
    let shown = format!("{typed:?}");
    assert!(!shown.contains("jkowalski"));
    assert!(shown.contains("[redacted]"));
}

#[test]
fn typed_login_reason_display_names_the_limits() {
    assert_eq!(
        TypedLoginReason::Length.to_string(),
        "a typed login is 1 to 64 characters"
    );
}

#[test]
fn default_policy_equals_the_rows_of_section_7_11() {
    let policy = ThrottlePolicy::default();
    let free = policy.free_failures();
    let window_minutes = policy.window().as_secs().checked_div(60).unwrap();
    assert!(row("login.delay_after_failures").contains(&format!("{free} failures per")));
    assert!(row("login.delay_after_failures").contains(&format!("{window_minutes} min")));
    assert!(row("login.delay_formula").contains(&format!("`2^(n-{free})` s")));
    assert!(
        row("login.delay_formula").contains(&format!("cap {} s", policy.delay_cap().as_secs()))
    );
    assert!(row("login.ip_budget").contains(&format!(
        "{} failures per client_ip",
        policy.address_budget()
    )));
    assert!(row("login.ip_budget").contains(&format!("{window_minutes} min")));
}

#[test]
fn delay_is_zero_below_the_free_failures() {
    let policy = ThrottlePolicy::default();
    for failures in 0..3 {
        assert_eq!(policy.delay(failures), Duration::ZERO);
    }
}

#[test]
fn delay_doubles_from_one_second_to_the_cap() {
    let policy = ThrottlePolicy::default();
    let seconds: Vec<u64> = (3..=10).map(|n| policy.delay(n).as_secs()).collect();
    assert_eq!(seconds, [1, 2, 4, 8, 16, 32, 60, 60]);
}

#[test]
fn delay_stays_at_the_cap_for_a_huge_count() {
    let policy = ThrottlePolicy::default();
    assert_eq!(policy.delay(u32::MAX), Duration::from_secs(60));
}

#[test]
fn budget_is_reached_at_the_budget_and_not_below() {
    let policy = ThrottlePolicy::default();
    assert!(!policy.budget_reached(99));
    assert!(policy.budget_reached(100));
    assert!(policy.budget_reached(101));
}

#[test]
fn four_failures_with_the_last_one_second_ago_wait_the_delay_of_the_fifth_failure() {
    let decision = decide(&count(4, Some(at(1_000)), at(1_001)));
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 4 });
}

#[test]
fn a_pair_at_the_cap_waits_the_cap() {
    let decision = decide(&count(9, Some(at(1_000)), at(1_000)));
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 60 });
}

#[test]
fn a_pair_with_the_largest_count_waits_the_cap() {
    let mut attempts = count(u32::MAX, Some(at(1_000)), at(1_000));
    attempts.address_failures = 0;
    let decision = decide(&attempts);
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 60 });
}

#[test]
fn two_failures_are_allowed() {
    assert_eq!(
        decide(&count(2, Some(at(1_000)), at(1_000))),
        ThrottleDecision::Allowed
    );
}

#[test]
fn no_failure_is_allowed() {
    assert_eq!(
        decide(&count(0, None, at(1_000))),
        ThrottleDecision::Allowed
    );
}

#[test]
fn failures_without_a_last_time_are_allowed() {
    assert_eq!(
        decide(&count(5, None, at(1_000))),
        ThrottleDecision::Allowed
    );
}

#[test]
fn three_failures_just_now_wait_the_delay_of_the_fourth_failure() {
    assert_eq!(
        decide(&count(3, Some(at(1_000)), at(1_000))),
        ThrottleDecision::Throttled { retry_after: 2 }
    );
}

#[test]
fn the_attempt_passes_when_the_delay_has_ended() {
    assert_eq!(
        decide(&count(4, Some(at(1_000)), at(1_002))),
        ThrottleDecision::Allowed
    );
}

#[test]
fn a_part_of_a_second_of_the_budget_window_rounds_up() {
    let mut attempts = count(0, None, at_millis(10_300_500));
    attempts.address_failures = 100;
    attempts.oldest_address_failure = Some(at_millis(10_000_000));
    let decision = decide(&attempts);
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 600 });
}

#[test]
fn a_negative_elapsed_time_counts_as_zero() {
    let decision = decide(&count(5, Some(at(2_000)), at(1_990)));
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 8 });
}

#[test]
fn the_cap_limits_the_wait_after_many_failures() {
    let decision = decide(&count(40, Some(at(1_000)), at(1_010)));
    assert_eq!(decision, ThrottleDecision::Throttled { retry_after: 60 });
}

#[test]
fn a_reached_budget_waits_for_the_rest_of_the_window() {
    let mut attempts = count(0, None, at(10_300));
    attempts.address_failures = 100;
    attempts.oldest_address_failure = Some(at(10_000));
    assert_eq!(
        decide(&attempts),
        ThrottleDecision::Throttled { retry_after: 600 }
    );
}

#[test]
fn the_budget_wins_over_the_pair_delay() {
    let mut attempts = count(4, Some(at(10_299)), at(10_300));
    attempts.address_failures = 100;
    attempts.oldest_address_failure = Some(at(10_000));
    assert_eq!(
        decide(&attempts),
        ThrottleDecision::Throttled { retry_after: 600 }
    );
}

#[test]
fn a_budget_below_the_limit_does_not_throttle() {
    let mut attempts = count(0, None, at(10_300));
    attempts.address_failures = 99;
    attempts.oldest_address_failure = Some(at(10_000));
    assert_eq!(decide(&attempts), ThrottleDecision::Allowed);
}

#[test]
fn an_oldest_failure_at_the_window_end_gives_one_second() {
    let mut attempts = count(0, None, at(10_900));
    attempts.address_failures = 100;
    attempts.oldest_address_failure = Some(at(10_000));
    assert_eq!(
        decide(&attempts),
        ThrottleDecision::Throttled { retry_after: 1 }
    );
}

#[test]
fn a_reached_budget_without_an_oldest_time_waits_for_a_whole_window() {
    let mut attempts = count(0, None, at(10_000));
    attempts.address_failures = 100;
    attempts.oldest_address_failure = None;
    assert_eq!(
        decide(&attempts),
        ThrottleDecision::Throttled { retry_after: 900 }
    );
}

proptest! {
    #[test]
    fn a_delay_never_passes_sixty_seconds(failures in any::<u32>()) {
        let delay = ThrottlePolicy::default().delay(failures);
        prop_assert!(delay <= Duration::from_secs(60));
    }

    #[test]
    fn a_throttled_attempt_waits_between_one_and_the_window(
        pair in 0u32..200,
        address in 0u32..200,
        last in 0i64..100_000,
        oldest in 0i64..100_000,
        now in 0i64..100_000,
        has_last in any::<bool>(),
        has_oldest in any::<bool>(),
    ) {
        let attempts = AttemptCount {
            pair_failures: pair,
            last_pair_failure: has_last.then(|| at(last)),
            address_failures: address,
            oldest_address_failure: has_oldest.then(|| at(oldest)),
            now: at(now),
        };
        if let ThrottleDecision::Throttled { retry_after } = decide(&attempts) {
            prop_assert!(retry_after >= 1);
            prop_assert!(retry_after <= 900);
        }
    }
}

#[test]
fn reliability_7_11_default_throttle_holds_the_login_rows() {
    // `login.delay_after_failures` 3 in 15 min, `login.delay_formula` cap 60 s,
    // `login.ip_budget` 100 in 15 min. The values are typed from the document.
    let throttle = LoginThrottle::default();
    let expected = ThrottlePolicy::parse(
        3,
        Duration::from_secs(60),
        Duration::from_secs(15 * 60),
        100,
    )
    .unwrap();
    assert_eq!(throttle.policy(), &expected);
}

#[test]
fn a_throttle_built_from_a_policy_returns_that_policy() {
    let custom = ThrottlePolicy::parse(5, secs(10), secs(60), 7).unwrap();
    assert_ne!(custom, ThrottlePolicy::default());
    assert_eq!(LoginThrottle::from(custom).policy(), &custom);
}

fn secs(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

/// Assert that `parse` refuses the four values and names `field`, with the code `E-IDN-012`.
fn assert_refused(
    free_failures: u32,
    delay_cap: Duration,
    window: Duration,
    address_budget: u32,
    field: &'static str,
) {
    let error =
        ThrottlePolicy::parse(free_failures, delay_cap, window, address_budget).unwrap_err();
    assert_eq!(error, IdentityError::ThrottlePolicyInvalid { field });
    assert_eq!(error.code(), ErrorCode::IDN_012);
}

#[test]
fn throttle_policy_parse_refuses_a_bad_field() {
    assert_refused(0, secs(60), secs(900), 100, "free_failures");
    assert_refused(100, secs(60), secs(900), 100, "free_failures");
    assert_refused(1, secs(60), secs(900), 1, "free_failures");
    assert_refused(3, secs(901), secs(900), 100, "delay_cap");
    assert_refused(3, Duration::ZERO, secs(900), 100, "delay_cap");
    assert_refused(3, secs(1), Duration::ZERO, 100, "window");
    assert_refused(3, secs(1), secs(86_401), 100, "window");
}

#[test]
fn throttle_policy_parse_takes_the_defaults_of_section_7_11() {
    let parsed = ThrottlePolicy::parse(3, secs(60), secs(900), 100).unwrap();
    assert_eq!(parsed, ThrottlePolicy::default());
}

#[test]
fn throttle_policy_parse_includes_both_bounds_of_each_rule() {
    // `free_failures` from 1 to `address_budget` - 1.
    assert!(ThrottlePolicy::parse(1, secs(1), secs(1), 2).is_ok());
    assert!(ThrottlePolicy::parse(99, secs(60), secs(900), 100).is_ok());
    // `delay_cap` from 1 s to `window`.
    assert!(ThrottlePolicy::parse(3, secs(900), secs(900), 100).is_ok());
    // `window` from 1 s to 24 h.
    assert!(ThrottlePolicy::parse(3, secs(86_400), secs(86_400), 100).is_ok());
}

#[test]
fn throttle_policy_accessors_return_the_parsed_values() {
    let policy = ThrottlePolicy::parse(2, secs(5), secs(30), 9).unwrap();
    assert_eq!(policy.free_failures(), 2);
    assert_eq!(policy.delay_cap(), secs(5));
    assert_eq!(policy.window(), secs(30));
    assert_eq!(policy.address_budget(), 9);
}
