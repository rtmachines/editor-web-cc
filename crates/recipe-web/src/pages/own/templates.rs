//! The templates of the own-account pages.

use askama::Template;
use recipe_app::identity::{OwnPasswordForm, OwnProfile};

use crate::error::{ErrorView, WebError};
use crate::i18n::Lang;
use crate::layout::{Layout, NavTarget};
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;

/// The page `login.html`.
///
/// The page solves one problem: the same form serves `GET /login` and every failed `POST /login`.
/// A failure shows the error on top. The nonce is the `csrf_pre` of the cookie that the same
/// answer sets. The page has no session, so it has no CSRF token for `hx-headers`.
#[derive(Template)]
#[template(path = "login.html")]
pub(crate) struct LoginPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    nonce: &'a str,
    view: Option<&'a ErrorView>,
}

impl<'a> LoginPage<'a> {
    /// Build the page. `view` is the error to show on top, or `None`.
    pub(crate) fn new(
        t: Texts<'a>,
        lang: Lang,
        nonce: &'a str,
        view: Option<&'a ErrorView>,
    ) -> Self {
        Self {
            layout: Layout::for_login(t, lang, t.get("login-title"), nonce),
            t,
            nonce,
            view,
        }
    }
}

/// Build the login page with the error of a failed login on top.
pub(crate) fn login_page_with_error<'a>(
    t: Texts<'a>,
    lang: Lang,
    nonce: &'a str,
    view: &'a ErrorView,
) -> LoginPage<'a> {
    LoginPage::new(t, lang, nonce, Some(view))
}

/// The page `me.html`.
#[derive(Template)]
#[template(path = "me.html")]
pub(crate) struct ProfilePage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    login: &'a str,
    role: String,
}

impl<'a> ProfilePage<'a> {
    /// Build the page from the profile of the user.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(crate) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        profile: &'a OwnProfile,
    ) -> Result<Self, WebError> {
        Ok(Self {
            layout: Layout::for_session(t, request, t.get("me-title"), Some(NavTarget::Me))?,
            t,
            login: profile.login.as_str(),
            role: t.role(profile.role),
        })
    }
}

/// The page `password.html`.
#[derive(Template)]
#[template(path = "password.html")]
pub(crate) struct PasswordPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    must_change: bool,
}

impl<'a> PasswordPage<'a> {
    /// Build the page from the form data.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(crate) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        form: OwnPasswordForm,
    ) -> Result<Self, WebError> {
        Ok(Self {
            layout: Layout::for_session(t, request, t.get("password-title"), Some(NavTarget::Me))?,
            t,
            must_change: form.must_change_password,
        })
    }
}

/// The fragment `password-changed.html`. HTMX reloads the page right after it.
#[derive(Template)]
#[template(path = "password-changed.html")]
pub(crate) struct PasswordChanged<'a> {
    t: Texts<'a>,
}

impl<'a> PasswordChanged<'a> {
    /// Build the fragment.
    pub(crate) const fn new(t: Texts<'a>) -> Self {
        Self { t }
    }
}
