//! The settings of the identity use cases (design D20, `reliability.md` section 7.11).

use std::time::Duration;

use recipe_domain::identity::ThrottlePolicy;

use super::Argon2Params;
use crate::ports::SessionLimits;

/// Default of `argon2.permits` (`reliability.md` section 7.11).
const DEFAULT_ARGON2_PERMITS: usize = 4;
/// Default of `argon2.acquire_timeout` in seconds.
const DEFAULT_ACQUIRE_TIMEOUT_SECONDS: u64 = 2;
/// Default of `session.idle_timeout` in seconds: 30 minutes.
const DEFAULT_IDLE_TIMEOUT_SECONDS: u64 = 30 * 60;
/// Default of `session.absolute_timeout` in seconds: 12 hours.
const DEFAULT_ABSOLUTE_TIMEOUT_SECONDS: u64 = 12 * 60 * 60;
/// Default of `page.users`.
const DEFAULT_PAGE_USERS: u32 = 100;
/// Default of `audit.unlock_rows`.
const DEFAULT_UNLOCK_ROWS: u32 = 100;

/// The numeric settings that the identity use cases read.
///
/// The type solves one problem: use cases must not read a configuration file. `recipe-server`
/// builds this value and passes it down. `Default` equals the table of `reliability.md`
/// section 7.11, and a test compares both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IdentitySettings {
    /// The cost parameters of Argon2id.
    pub argon2: Argon2Params,
    /// The number of hashes that may run at one time (`argon2.permits`).
    pub argon2_permits: usize,
    /// The longest wait for a permit (`argon2.acquire_timeout`).
    pub argon2_acquire_timeout: Duration,
    /// The login throttle.
    pub throttle: ThrottlePolicy,
    /// The idle time after which a session ends (`session.idle_timeout`).
    pub session_idle_timeout: Duration,
    /// The age after which a session ends (`session.absolute_timeout`).
    pub session_absolute_timeout: Duration,
    /// The rows of one page of `ListUsers` (`page.users`).
    pub page_users: u32,
    /// The deleted attempts in the image of `LoginUnlocked` (`audit.unlock_rows`).
    pub unlock_rows: u32,
}

impl IdentitySettings {
    /// Return the two session timeouts as the limits of `SessionStore::authenticate`.
    #[must_use]
    pub const fn session_limits(&self) -> SessionLimits {
        SessionLimits {
            idle_timeout: self.session_idle_timeout,
            absolute_timeout: self.session_absolute_timeout,
        }
    }
}

impl Default for IdentitySettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            argon2: Argon2Params::default(),
            argon2_permits: DEFAULT_ARGON2_PERMITS,
            argon2_acquire_timeout: Duration::from_secs(DEFAULT_ACQUIRE_TIMEOUT_SECONDS),
            throttle: ThrottlePolicy::default(),
            session_idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECONDS),
            session_absolute_timeout: Duration::from_secs(DEFAULT_ABSOLUTE_TIMEOUT_SECONDS),
            page_users: DEFAULT_PAGE_USERS,
            unlock_rows: DEFAULT_UNLOCK_ROWS,
        }
    }
}
