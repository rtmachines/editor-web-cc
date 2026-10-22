//! A fixture with the settings of the use cases of a test: `Fixture::with_app_settings`.
//!
//! The function solves one problem: the tests of the diff pages need a small `page.diff` and the
//! failing blocking tasks of `Blocking::elapsed` and `Blocking::panicked`. `Fixture::new` always
//! takes `AppSettings::default()`.

use recipe_app::blocking::AppSettings;

use crate::WebSettings;
use crate::test_support::{CSRF_KEY_NEW, Fixture};

impl Fixture {
    /// Build a fixture with the default web settings and the settings `app` of the use cases.
    pub(crate) fn with_app_settings(app: AppSettings) -> Self {
        Self::build(WebSettings::default(), CSRF_KEY_NEW, app)
    }
}
