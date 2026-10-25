//! The settings of the web layer (design D20, `reliability.md` section 7.11).

use std::fmt;
use std::time::Duration;

use recipe_domain::recipe::LangCode;

mod cidr;

pub use cidr::{Cidr, CidrList, CidrReason};

/// Default of `http.trusted_proxies`: the one address of `proxy` in the Compose network (ADR-0065 (j)).
const DEFAULT_TRUSTED_PROXY: &str = "172.28.0.2/32";
/// Default of `http.request_timeout` in seconds.
const DEFAULT_REQUEST_TIMEOUT_SECONDS: u64 = 30;
/// Default of `http.body_read_timeout` in seconds.
const DEFAULT_BODY_READ_TIMEOUT_SECONDS: u64 = 10;
/// Default of `http.body_limit.form`: 256 KiB.
const DEFAULT_BODY_LIMIT_FORM: usize = 256 * 1024;
/// Default of `http.body_limit.value`: 4 MiB (ADR-0039).
const DEFAULT_BODY_LIMIT_VALUE: usize = 4 * 1024 * 1024;
/// Default of `http.body_limit.translation`: 64 KiB.
const DEFAULT_BODY_LIMIT_TRANSLATION: usize = 64 * 1024;
/// Default of `i18n.translation_langs`: the one language `pl` (ADR-0043).
const DEFAULT_TRANSLATION_LANG: &str = "pl";
/// Default of `form.array_chunk`: 100 elements per group of the leaf grid (ADR-0039 (l)).
const DEFAULT_ARRAY_CHUNK: usize = 100;
/// Default of `form.rebind_leaves`: 16 384 new member leaves in the rebind forms of one page (ADR-0044).
const DEFAULT_REBIND_LEAVES: usize = 16_384;
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
    /// The time that one request may take from its start to its answer (`http.request_timeout`).
    pub request_timeout: Duration,
    /// The time that the client may take to send the whole request body (`http.body_read_timeout`).
    pub body_read_timeout: Duration,
    /// The `Max-Age` of the cookie `recipe_lang` (`i18n.lang_cookie_max_age`).
    pub lang_cookie_max_age: Duration,
    /// The languages of the translation rows (`i18n.translation_langs`).
    pub translation_langs: Vec<LangCode>,
    /// The number of elements in one group of an array in the leaf grid (`form.array_chunk`).
    pub form_array_chunk: usize,
    /// The bound on the new member leaves of all rebind forms of one version page (`form.rebind_leaves`).
    pub form_rebind_leaves: usize,
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
            request_timeout: Duration::from_secs(DEFAULT_REQUEST_TIMEOUT_SECONDS),
            body_read_timeout: Duration::from_secs(DEFAULT_BODY_READ_TIMEOUT_SECONDS),
            lang_cookie_max_age: Duration::from_secs(DEFAULT_LANG_COOKIE_MAX_AGE_SECONDS),
            translation_langs: LangCode::parse(DEFAULT_TRANSLATION_LANG)
                .map(|lang| vec![lang])
                .unwrap_or_default(),
            form_array_chunk: DEFAULT_ARRAY_CHUNK,
            form_rebind_leaves: DEFAULT_REBIND_LEAVES,
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
