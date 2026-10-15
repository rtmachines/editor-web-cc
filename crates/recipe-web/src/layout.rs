//! The layout of every full page: top bar, navigation, language switch (design D5).

use recipe_app::auth::Operation;
use recipe_domain::identity::Role;

use crate::error::WebError;
use crate::i18n::Lang;
use crate::pages::{PageRequest, Texts};

/// The page that the language switch of the login page returns to.
const LOGIN_PATH: &str = "/login";

/// The value of `HX-Target` that asks for the content answer: the `id` of `main`.
const CONTENT_TARGET: &str = "content";

/// The shape of the answer of a page route (design D6, ADR-0037 w).
///
/// The type solves one problem: a boosted link or form moves to a new page and swaps only `main`.
/// The answer must not repeat the skip link and the top bar. One value tells `base.html` which
/// shape to write, and the heading macro reads it to move the focus.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Shape {
    /// The full page: the skip link, the top bar, and `main`.
    Page,
    /// The content of `main`, the `title` element, and the field `next` of the language switch.
    Content,
}

/// A page that the navigation links to.
///
/// The type solves one problem: the navigation must show a link exactly when the use case of the
/// target page allows the role. Each target names the query of its page, so `requirement()` of
/// that query decides, and the link cannot disagree with `authorize`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NavTarget {
    /// The recipe list, `/recipes`.
    Recipes,
    /// The machine list, `/machines`. Every page below `/machines` marks this link as current.
    Machines,
    /// The user list, `/users`.
    Users,
    /// The own profile, `/me`.
    Me,
}

impl NavTarget {
    /// Every target, in the order of the top bar (ADR-0037 q).
    const ALL: [Self; 4] = [Self::Recipes, Self::Machines, Self::Users, Self::Me];

    /// Return the path of the page.
    pub(crate) const fn path(self) -> &'static str {
        match self {
            Self::Recipes => "/recipes",
            Self::Machines => "/machines",
            Self::Users => "/users",
            Self::Me => "/me",
        }
    }

    /// Return the key of the link text in the bundles.
    pub(crate) const fn text_key(self) -> &'static str {
        match self {
            Self::Recipes => "nav-recipes",
            Self::Machines => "nav-machines",
            Self::Users => "nav-users",
            Self::Me => "nav-me",
        }
    }

    /// Return the query that the page runs.
    pub(crate) const fn operation(self) -> Operation {
        match self {
            Self::Recipes => Operation::ListRecipes,
            Self::Machines => Operation::ListMachines,
            Self::Users => Operation::ListUsers,
            Self::Me => Operation::GetOwnProfile,
        }
    }
}

/// One link of the navigation.
pub(crate) struct NavLink {
    /// The path of the page.
    pub(crate) path: &'static str,
    /// The link text in the language of the page.
    pub(crate) text: String,
    /// True for the page that the user views now.
    pub(crate) current: bool,
}

/// Build the links that `role` may see.
///
/// A session with `must_change_password` gets no link, because every page except the password
/// page answers with a redirect.
fn nav_links(
    t: Texts<'_>,
    role: Role,
    must_change_password: bool,
    current: Option<NavTarget>,
) -> Vec<NavLink> {
    if must_change_password {
        return Vec::new();
    }
    NavTarget::ALL
        .into_iter()
        .filter(|target| target.operation().requirement().allows(role))
        .map(|target| NavLink {
            path: target.path(),
            text: t.get(target.text_key()),
            current: current == Some(target),
        })
        .collect()
}

/// What `base.html` shows around the content of a page.
///
/// The type solves one problem: every full page needs the same top bar, and the error layer
/// builds a page without a session. One value holds all of it, so `base.html` reads only
/// `layout` and no page struct repeats the language, the token, or the logout form.
pub(crate) struct Layout<'a> {
    /// The UI language.
    pub(crate) lang: Lang,
    /// The title of the page, without the application name.
    pub(crate) title: String,
    /// The texts of the language.
    pub(crate) t: Texts<'a>,
    /// The navigation links. Empty without a session.
    pub(crate) nav: Vec<NavLink>,
    /// The CSRF token of the session, or an empty text.
    pub(crate) csrf_token: String,
    /// The pre-session nonce of the login page, or `None`.
    pub(crate) pre_nonce: Option<String>,
    /// The login of the user, or `None` without a session.
    pub(crate) user_login: Option<String>,
    /// The target of the language switch, or `None` when the page has no switch.
    pub(crate) next: Option<String>,
    /// The success sentence for `#toast`, or `None`.
    pub(crate) toast: Option<String>,
    /// The shape of the answer: the full page or the content of `main`.
    pub(crate) shape: Shape,
}

impl<'a> Layout<'a> {
    /// Build the layout of a page with a session.
    ///
    /// `current` marks the navigation link of the page. The switch appears only when the
    /// request is a `GET`. An HTMX request with `HX-Target: content` gets the content shape.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(crate) fn for_session(
        texts: Texts<'a>,
        request: &PageRequest,
        title: String,
        current: Option<NavTarget>,
    ) -> Result<Self, WebError> {
        let auth = request.auth()?;
        let user = auth.user();
        let shape = if request.is_htmx() && request.hx_target() == Some(CONTENT_TARGET) {
            Shape::Content
        } else {
            Shape::Page
        };
        Ok(Self {
            lang: request.lang(),
            title,
            t: texts,
            nav: nav_links(texts, user.role(), auth.must_change_password(), current),
            csrf_token: request.csrf_token(),
            pre_nonce: None,
            user_login: Some(user.login().to_string()),
            next: request.path_and_query().map(str::to_owned),
            toast: None,
            shape,
        })
    }

    /// Build the layout of the login page. The switch returns to `/login`.
    pub(crate) fn for_login(texts: Texts<'a>, lang: Lang, title: String, nonce: &str) -> Self {
        Self {
            pre_nonce: Some(nonce.to_owned()),
            next: Some(LOGIN_PATH.to_owned()),
            ..Self::bare(texts, lang, title)
        }
    }

    /// Build the layout of the full error page: no navigation, no switch, and no token.
    pub(crate) const fn bare(texts: Texts<'a>, lang: Lang, title: String) -> Self {
        Self {
            lang,
            title,
            t: texts,
            nav: Vec::new(),
            csrf_token: String::new(),
            pre_nonce: None,
            user_login: None,
            next: None,
            toast: None,
            shape: Shape::Page,
        }
    }

    /// Return true when the answer is the content of `main` and not the full page.
    pub(crate) const fn is_content(&self) -> bool {
        matches!(self.shape, Shape::Content)
    }

    /// Return the layout with the success sentence for `#toast`.
    #[must_use]
    pub(crate) fn with_toast(self, text: String) -> Self {
        Self {
            toast: Some(text),
            ..self
        }
    }
}

#[cfg(test)]
mod tests;
