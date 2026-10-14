//! Login throttle of the Identity and Access context (design D6, `domain-model.md` 2.3).
//!
//! The throttle is a pure decision on counts that the store read in one transaction.
//! It uses the database time `now` of that transaction, so no host clock mixes in.

use std::fmt;
use std::time::Duration;

use crate::timestamp::Timestamp;

/// Longest number of characters of a typed login (`reliability.md` section 4.3).
const TYPED_LOGIN_MAX_CHARS: usize = 64;

/// Failures of one pair (login, address) that pass without a delay (`login.delay_after_failures`).
const FREE_FAILURES: u32 = 3;

/// Longest delay after a failure (`login.delay_formula`).
const DELAY_CAP_SECONDS: u64 = 60;

/// Length of the throttle window (`login.delay_after_failures`).
const WINDOW_SECONDS: u64 = 15 * 60;

/// Failures of one address in the window before it is blocked (`login.ip_budget`).
const ADDRESS_BUDGET: u32 = 100;

/// Nanoseconds in one second.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// The login text of a login form field, folded to ASCII lower case.
///
/// The text has 1 to 64 characters and may break the grammar of `Login`.
/// A typed login outside that grammar counts as an unknown login (design D6).
/// The text is user input, so `Debug` redacts it.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct TypedLogin(String);

/// Reason why a text is not a typed login.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TypedLoginReason {
    /// The text is empty or has more than 64 characters.
    Length,
    /// The text holds a control character, such as `U+0000`. PostgreSQL rejects `U+0000` in text.
    Control,
}

impl fmt::Display for TypedLoginReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Length => f.write_str("a typed login is 1 to 64 characters"),
            Self::Control => f.write_str("a typed login holds no control character"),
        }
    }
}

impl TypedLogin {
    /// Parse the text of a login form field. ASCII letters fold to lower case.
    ///
    /// # Errors
    ///
    /// Returns [`TypedLoginReason::Length`] when the text has no character or more than 64.
    /// Returns [`TypedLoginReason::Control`] when a character is a control character.
    pub fn parse(text: &str) -> Result<Self, TypedLoginReason> {
        let length = text.chars().count();
        if length == 0 || length > TYPED_LOGIN_MAX_CHARS {
            return Err(TypedLoginReason::Length);
        }
        if text.chars().any(char::is_control) {
            return Err(TypedLoginReason::Control);
        }
        Ok(Self(text.to_ascii_lowercase()))
    }

    /// Return the folded text of the typed login.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for TypedLogin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TypedLogin([redacted])")
    }
}

/// The values of `reliability.md` section 7.11 that the login throttle uses.
///
/// One struct gives the decision and the store one source for these numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ThrottlePolicy {
    /// Failures of a pair that pass without a delay.
    pub free_failures: u32,
    /// Longest delay after a failure.
    pub delay_cap: Duration,
    /// Length of the window that the counts cover.
    pub window: Duration,
    /// Failures of one address in the window that block the address.
    pub address_budget: u32,
}

impl Default for ThrottlePolicy {
    fn default() -> Self {
        Self {
            free_failures: FREE_FAILURES,
            delay_cap: Duration::from_secs(DELAY_CAP_SECONDS),
            window: Duration::from_secs(WINDOW_SECONDS),
            address_budget: ADDRESS_BUDGET,
        }
    }
}

impl ThrottlePolicy {
    /// Return the delay after `failures` failures of a pair: `2^(n-3)` seconds up to the cap.
    ///
    /// The result is zero below the free failures. The calculation cannot overflow.
    #[must_use]
    pub fn delay(&self, failures: u32) -> Duration {
        let Some(exponent) = failures.checked_sub(self.free_failures) else {
            return Duration::ZERO;
        };
        let seconds = 2u64.checked_pow(exponent).unwrap_or(u64::MAX);
        Duration::from_secs(seconds).min(self.delay_cap)
    }

    /// Tell whether `address_failures` use up the budget of one address.
    ///
    /// The store and the decision share this rule. The store inserts no attempt when it is true.
    #[must_use]
    pub const fn budget_reached(&self, address_failures: u32) -> bool {
        address_failures >= self.address_budget
    }
}

/// The counts that the first transaction of `Login` reads in one scan.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttemptCount {
    /// Failures of the pair (login, address) in the window.
    pub pair_failures: u32,
    /// Time of the last failure of the pair.
    pub last_pair_failure: Option<Timestamp>,
    /// Failures of the address in the window, over every login.
    pub address_failures: u32,
    /// Time of the oldest failure of the address in the window.
    pub oldest_address_failure: Option<Timestamp>,
    /// The database time of the transaction.
    pub now: Timestamp,
}

/// The result of the login throttle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThrottleDecision {
    /// The attempt may go on.
    Allowed,
    /// The attempt is inside a delay or the address budget is used up.
    Throttled {
        /// Whole seconds until the attempt can pass. At least 1.
        ///
        /// For a pair this is the whole `delay(n + 1)`, because the throttled attempt counts as a
        /// failure. For the budget of an address it is the rest of the window.
        retry_after: u64,
    },
}

/// The decision of the login throttle (`domain-model.md` 2.3).
///
/// It solves one problem: the delay of a pair and the budget of an address must use one rule.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LoginThrottle {
    policy: ThrottlePolicy,
}

impl LoginThrottle {
    /// Build a throttle with the given values.
    #[must_use]
    pub const fn new(policy: ThrottlePolicy) -> Self {
        Self { policy }
    }

    /// Return the values of the throttle.
    #[must_use]
    pub const fn policy(&self) -> &ThrottlePolicy {
        &self.policy
    }

    /// Decide whether an attempt may go on.
    ///
    /// A used budget gives the rest of the window after the oldest address failure.
    /// T1 inserts no row at the budget, so that rest is exact.
    /// Else a pair inside its delay gives the whole `delay(n + 1)`.
    /// T1 inserts the throttled attempt as failure `n + 1` at the time of T1, so the new delay starts now.
    /// A negative elapsed time counts as 0.
    #[must_use]
    pub fn decide(&self, count: &AttemptCount) -> ThrottleDecision {
        if self.policy.budget_reached(count.address_failures) {
            let elapsed = count
                .oldest_address_failure
                .map_or(0, |oldest| elapsed_nanos(oldest, count.now));
            return throttled(self.policy.window, elapsed);
        }
        let delay = self.policy.delay(count.pair_failures);
        let Some(last) = count.last_pair_failure else {
            return ThrottleDecision::Allowed;
        };
        let elapsed = elapsed_nanos(last, count.now);
        if elapsed >= delay.as_nanos() {
            return ThrottleDecision::Allowed;
        }
        let next = count
            .pair_failures
            .checked_add(1)
            .map_or(self.policy.delay_cap, |failures| {
                self.policy.delay(failures)
            });
        throttled(next, 0)
    }
}

/// Return the nanoseconds from `earlier` to `later`. A negative value gives 0.
fn elapsed_nanos(earlier: Timestamp, later: Timestamp) -> u128 {
    let later_nanos = later.as_offset_date_time().unix_timestamp_nanos();
    let earlier_nanos = earlier.as_offset_date_time().unix_timestamp_nanos();
    later_nanos
        .checked_sub(earlier_nanos)
        .and_then(|difference| u128::try_from(difference).ok())
        .unwrap_or(0)
}

/// Build the decision for the rest of `total` after `elapsed` nanoseconds.
///
/// The rest rounds up to whole seconds and is at least 1.
fn throttled(total: Duration, elapsed: u128) -> ThrottleDecision {
    let rest = total.as_nanos().saturating_sub(elapsed);
    let seconds = rest.div_ceil(NANOS_PER_SECOND).max(1);
    ThrottleDecision::Throttled {
        retry_after: u64::try_from(seconds).unwrap_or(u64::MAX),
    }
}

#[cfg(test)]
mod tests;
