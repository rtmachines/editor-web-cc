//! The settings of the web layer (design D20, `reliability.md` section 7.11).

use std::fmt;
use std::time::Duration;

mod cidr;

pub use cidr::{Cidr, CidrList, CidrReason};

/// Default of `http.trusted_proxies`: the subnet of the Compose network (`01-roles.sh`).
const DEFAULT_TRUSTED_PROXY: &str = "172.28.0.0/16";
/// Default of `http.body_limit.form`: 256 KiB.
const DEFAULT_BODY_LIMIT_FORM: usize = 256 * 1024;
/// Default of `http.body_limit.value`: 1 MiB.
const DEFAULT_BODY_LIMIT_VALUE: usize = 1024 * 1024;
/// Default of `http.body_limit.translation`: 64 KiB.
const DEFAULT_BODY_LIMIT_TRANSLATION: usize = 64 * 1024;
/// Seconds in one day.
const SECONDS_PER_DAY: u64 = 86_400;
/// Default of `http.hsts_max_age` in seconds: 365 days.
const DEFAULT_HSTS_MAX_AGE_SECONDS: u64 = 365 * SECONDS_PER_DAY;
/// Default of `i18n.lang_cookie_max_age` in seconds: 365 days.
const DEFAULT_LANG_COOKIE_MAX_AGE_SECONDS: u64 = 365 * SECONDS_PER_DAY;
/// Default of `http.retry_after` in seconds.
const DEFAULT_RETRY_AFTER_SECONDS: u64 = 5;

/// The numeric settings that `recipe-web` reads.
///
/// The type solves one problem: the web layer must not read a configuration file. `recipe-server`
/// builds this value and passes it down. `Default` equals the table of `reliability.md`
/// section 7.11, and a test compares both. `Debug` hides the fields, because later tasks add the
/// signing key of the pre-session CSRF cookie to this type (design D4).
#[derive(Clone, PartialEq, Eq)]
pub struct WebSettings {
    /// The networks whose `X-Forwarded-For` header counts (`http.trusted_proxies`).
    pub trusted_proxies: CidrList,
    /// The body limit of an HTML form in bytes (`http.body_limit.form`).
    pub body_limit_form: usize,
    /// The body limit of the routes that carry a parameter value (`http.body_limit.value`).
    pub body_limit_value: usize,
    /// The body limit of a translation (`http.body_limit.translation`).
    pub body_limit_translation: usize,
    /// The `max-age` of `GET /static/*` (`static.cache_max_age`).
    pub static_cache_max_age: Duration,
    /// The `max-age` of `Strict-Transport-Security` (`http.hsts_max_age`).
    pub hsts_max_age: Duration,
    /// The `Retry-After` of a 503 answer (`http.retry_after`).
    pub retry_after: Duration,
    /// The `Max-Age` of the cookie `recipe_lang` (`i18n.lang_cookie_max_age`).
    pub lang_cookie_max_age: Duration,
}

impl Default for WebSettings {
    /// Return the values of `reliability.md` section 7.11.
    fn default() -> Self {
        Self {
            trusted_proxies: CidrList::parse_all([DEFAULT_TRUSTED_PROXY]).unwrap_or_default(),
            body_limit_form: DEFAULT_BODY_LIMIT_FORM,
            body_limit_value: DEFAULT_BODY_LIMIT_VALUE,
            body_limit_translation: DEFAULT_BODY_LIMIT_TRANSLATION,
            static_cache_max_age: Duration::from_secs(SECONDS_PER_DAY),
            hsts_max_age: Duration::from_secs(DEFAULT_HSTS_MAX_AGE_SECONDS),
            retry_after: Duration::from_secs(DEFAULT_RETRY_AFTER_SECONDS),
            lang_cookie_max_age: Duration::from_secs(DEFAULT_LANG_COOKIE_MAX_AGE_SECONDS),
        }
    }
}

impl fmt::Debug for WebSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WebSettings").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests;
