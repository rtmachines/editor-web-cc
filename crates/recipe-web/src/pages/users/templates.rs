//! The templates of the user administration.

use askama::Template;
use recipe_app::identity::UserDetail;
use recipe_app::ports::UserSummary;
use recipe_domain::identity::Role;
use recipe_domain::timestamp::Timestamp;

use crate::i18n::Lang;
use crate::pages::texts::Texts;

/// Write an instant as `yyyy-mm-dd hh:mm:ss` with its offset.
fn instant(stamp: &Timestamp) -> String {
    let moment = stamp.as_offset_date_time();
    let time = moment.time();
    format!(
        "{} {:02}:{:02}:{:02} {}",
        moment.date(),
        time.hour(),
        time.minute(),
        time.second(),
        moment.offset()
    )
}

/// One option of a role list.
pub(super) struct RoleOption {
    value: &'static str,
    label: String,
    selected: bool,
}

/// Build the options for all roles. `selected` marks the role that the form shows first.
pub(super) fn role_options(
    t: Texts<'_>,
    roles: &[Role],
    selected: Option<Role>,
) -> Vec<RoleOption> {
    roles
        .iter()
        .map(|role| RoleOption {
            value: role.as_sql(),
            label: t.role(*role),
            selected: selected == Some(*role),
        })
        .collect()
}

/// One row of the user list.
pub(super) struct UserRow {
    login: String,
    role: String,
    status: String,
    must_change: bool,
}

impl UserRow {
    fn of(t: Texts<'_>, user: &UserSummary) -> Self {
        Self {
            login: user.login.to_string(),
            role: t.role(user.role),
            status: status_label(t, user.is_active),
            must_change: user.must_change_password,
        }
    }
}

fn status_label(t: Texts<'_>, is_active: bool) -> String {
    t.get(if is_active {
        "user-status-active"
    } else {
        "user-status-inactive"
    })
}

/// The page `users.html`.
#[derive(Template)]
#[template(path = "users.html")]
pub(crate) struct UsersPage<'a> {
    lang: Lang,
    title: String,
    csrf_token: &'a str,
    t: Texts<'a>,
    q: &'a str,
    rows: Vec<UserRow>,
    prev_url: Option<String>,
    next_url: Option<String>,
}

impl<'a> UsersPage<'a> {
    /// Build the page from one page of users and the links to the pages around it.
    pub(super) fn new(
        t: Texts<'a>,
        lang: Lang,
        csrf_token: &'a str,
        q: &'a str,
        users: &[UserSummary],
        links: (Option<String>, Option<String>),
    ) -> Self {
        Self {
            lang,
            title: t.get("users-title"),
            csrf_token,
            t,
            q,
            rows: users.iter().map(|user| UserRow::of(t, user)).collect(),
            prev_url: links.0,
            next_url: links.1,
        }
    }
}

/// The page `user-new.html`.
#[derive(Template)]
#[template(path = "user-new.html")]
pub(crate) struct NewUserPage<'a> {
    lang: Lang,
    title: String,
    csrf_token: &'a str,
    t: Texts<'a>,
    roles: Vec<RoleOption>,
}

impl<'a> NewUserPage<'a> {
    /// Build the page from the roles that the form offers.
    pub(super) fn new(t: Texts<'a>, lang: Lang, csrf_token: &'a str, roles: &[Role]) -> Self {
        Self {
            lang,
            title: t.get("user-new-title"),
            csrf_token,
            t,
            roles: role_options(t, roles, Some(Role::Operator)),
        }
    }
}

/// The page `user.html`.
#[derive(Template)]
#[template(path = "user.html")]
pub(crate) struct UserPage<'a> {
    lang: Lang,
    title: String,
    csrf_token: &'a str,
    t: Texts<'a>,
    login: String,
    role: String,
    status: String,
    must_change: bool,
    created: String,
    deactivated: Option<String>,
    failures: u32,
    last_failure: Option<String>,
    roles: Vec<RoleOption>,
}

impl<'a> UserPage<'a> {
    /// Build the page from the user and the throttle state of the login.
    pub(super) fn new(t: Texts<'a>, lang: Lang, csrf_token: &'a str, detail: &UserDetail) -> Self {
        let user = &detail.user;
        Self {
            lang,
            title: t.get("user-title"),
            csrf_token,
            t,
            login: user.login.to_string(),
            role: t.role(user.role),
            status: status_label(t, user.is_active),
            must_change: user.must_change_password,
            created: instant(&user.created_at),
            deactivated: user.deactivated_at.as_ref().map(instant),
            failures: detail.throttle.failures,
            last_failure: detail.throttle.last_failure.as_ref().map(instant),
            roles: role_options(t, &Role::ALL, Some(user.role)),
        }
    }
}

/// The result of a command: one sentence and, for a new password, the password.
///
/// The type solves one problem: the one-time password must appear in the answer of its own
/// command and nowhere else. It is a fragment for HTMX and a page for the other requests.
#[derive(Template)]
#[template(path = "user-result.html")]
pub(crate) struct ResultFragment<'a> {
    t: Texts<'a>,
    message: String,
    password: Option<String>,
    login: String,
}

impl<'a> ResultFragment<'a> {
    /// Build the fragment. `password` is the one-time password of the command, or `None`.
    pub(super) const fn new(
        t: Texts<'a>,
        message: String,
        password: Option<String>,
        login: String,
    ) -> Self {
        Self {
            t,
            message,
            password,
            login,
        }
    }
}

/// The page `user-result-page.html`: the result of a command without HTMX.
#[derive(Template)]
#[template(path = "user-result-page.html")]
pub(crate) struct ResultPage<'a> {
    lang: Lang,
    title: String,
    csrf_token: &'a str,
    t: Texts<'a>,
    message: String,
    password: Option<String>,
    login: String,
}

impl<'a> ResultPage<'a> {
    /// Build the page. `title` is the heading of the page.
    pub(super) fn new(
        t: Texts<'a>,
        lang: Lang,
        csrf_token: &'a str,
        title: String,
        fragment: ResultFragment<'a>,
    ) -> Self {
        Self {
            lang,
            title,
            csrf_token,
            t,
            message: fragment.message,
            password: fragment.password,
            login: fragment.login,
        }
    }
}
