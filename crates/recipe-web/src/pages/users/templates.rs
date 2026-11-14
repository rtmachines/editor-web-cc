//! The templates of the user administration.

use askama::Template;
use recipe_app::auth::Operation;
use recipe_app::identity::UserDetail;
use recipe_app::ports::UserSummary;
use recipe_domain::identity::{Login, Role};

use crate::error::{FormErrors, WebError};
use crate::layout::{Layout, NavTarget};
use crate::pages::render::instant;
use crate::pages::request::PageRequest;
use crate::pages::texts::Texts;

/// One option of a role list: the value that the form sends and the text that the user reads.
///
/// The field names `value` and `text` are the ones that the macro `fields::select` reads.
pub(super) struct RoleOption {
    value: &'static str,
    text: String,
}

/// Build the options for the roles. The form passes its selected value to `fields::select`.
pub(super) fn role_options(t: Texts<'_>, roles: &[Role]) -> Vec<RoleOption> {
    roles
        .iter()
        .map(|role| RoleOption {
            value: role.as_sql(),
            text: t.role(*role),
        })
        .collect()
}

/// Return the roles of the user form: the roles of the use case, then `ExportClient`.
///
/// The set-role form of a user never offers `ExportClient`. Only the form of a new user does
/// (`web-ui` spec, ADR-0109 (a)).
fn offered_on_create(roles: &[Role]) -> Vec<Role> {
    let mut offered = roles.to_vec();
    if !offered.contains(&Role::ExportClient) {
        offered.push(Role::ExportClient);
    }
    offered
}

/// One row of the user list.
pub(super) struct UserRow {
    login: String,
    role: String,
    active: bool,
    status: String,
    must_change: bool,
}

impl UserRow {
    fn of(t: Texts<'_>, user: &UserSummary) -> Self {
        Self {
            login: user.login.to_string(),
            role: t.role(user.role),
            active: user.is_active,
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
    layout: Layout<'a>,
    t: Texts<'a>,
    q: &'a str,
    rows: Vec<UserRow>,
    /// The address of the previous page. Empty when the page has no previous link.
    previous_url: String,
    /// The address of the next page. Empty when the page has no next link.
    next_url: String,
    /// The number of the current page, counted from 1.
    current: u32,
    /// True for a page after the last page: no row, and the page number is above 1.
    after_end: bool,
    /// The address of the first page. The page `after_end` links to it.
    first_url: String,
    /// True when the user may create a user in the current mode (ADR-0113 (b)).
    can_create: bool,
}

/// The links and the position of one page of the user list.
///
/// The type solves one problem: the handler computes the pager once, and the page only shows it.
pub(super) struct UsersPager {
    /// The address of the previous page, or empty for the first page and for a page after the end.
    pub(super) previous: String,
    /// The address of the next page, or empty when the page is not full.
    pub(super) next: String,
    /// The number of the current page, counted from 1.
    pub(super) current: u32,
    /// True for a page after the last page.
    pub(super) after_end: bool,
    /// The address of the first page.
    pub(super) first: String,
}

impl<'a> UsersPage<'a> {
    /// Build the page from one page of users and the pager around it.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(super) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        q: &'a str,
        users: &[UserSummary],
        pager: UsersPager,
    ) -> Result<Self, WebError> {
        Ok(Self {
            layout: Layout::for_session(t, request, t.get("users-title"), Some(NavTarget::Users))?,
            t,
            q,
            rows: users.iter().map(|user| UserRow::of(t, user)).collect(),
            previous_url: pager.previous,
            next_url: pager.next,
            current: pager.current,
            after_end: pager.after_end,
            first_url: pager.first,
            can_create: request.may_run(Operation::CreateUser)?,
        })
    }
}

/// The data of the form of a new user: the roles, the sent texts, and the error lines.
pub(super) struct NewUserForm {
    csrf_token: String,
    roles: Vec<RoleOption>,
    /// The value of the role that the form shows first.
    selected_role: &'static str,
    login: String,
    /// True when the answer swaps the form out of band (ADR-0041 (e)).
    oob: bool,
    errors: FormErrors,
    /// True when the mode blocks the command. The page then shows a notice (ADR-0113 (d)).
    blocked: bool,
}

impl NewUserForm {
    /// Build an empty form from the roles that the use case offers.
    pub(super) fn new(t: Texts<'_>, roles: &[Role], csrf_token: String) -> Self {
        Self {
            csrf_token,
            roles: role_options(t, &offered_on_create(roles)),
            selected_role: Role::Operator.as_sql(),
            login: String::new(),
            oob: false,
            errors: FormErrors::default(),
            blocked: false,
        }
    }

    /// Return the form with the sent login and role and the error lines, swapped out of band.
    ///
    /// A sent role outside the list keeps the first choice. A sent text that is not a login is
    /// not shown again, so the answer never echoes a payload.
    pub(super) fn sent(self, login: &str, role: &str, errors: FormErrors) -> Self {
        let selected_role = Role::parse(role).map_or(self.selected_role, Role::as_sql);
        let login = Login::parse(login).map_or_else(|_reason| String::new(), |ok| ok.to_string());
        Self {
            selected_role,
            login,
            oob: true,
            errors,
            ..self
        }
    }
}

/// The page `user-new.html`.
#[derive(Template)]
#[template(path = "user-new.html")]
pub(crate) struct NewUserPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    form: NewUserForm,
}

impl<'a> NewUserPage<'a> {
    /// Build the page from the roles that the form offers.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(super) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        roles: &[Role],
    ) -> Result<Self, WebError> {
        let form = NewUserForm {
            blocked: !request.may_run(Operation::CreateUser)?,
            ..NewUserForm::new(t, roles, request.csrf_token())
        };
        Ok(Self {
            layout: Layout::for_session(t, request, t.get("user-new-title"), None)?,
            t,
            form,
        })
    }
}

/// The block `form` of `user-new.html` alone: the form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "user-new.html", block = "form")]
pub(super) struct NewUserFormBlock<'a> {
    t: Texts<'a>,
    form: NewUserForm,
}

impl<'a> NewUserFormBlock<'a> {
    /// Wrap the form with the texts of the language.
    pub(super) const fn new(t: Texts<'a>, form: NewUserForm) -> Self {
        Self { t, form }
    }
}

/// The form `user-role-form.html` alone: the set-role form that an error answer swaps out of band.
#[derive(Template)]
#[template(path = "user-role-form.html")]
pub(super) struct RoleFormBlock<'a> {
    t: Texts<'a>,
    v: UserView,
}

impl<'a> RoleFormBlock<'a> {
    /// Wrap the view with the texts of the language.
    pub(super) const fn new(t: Texts<'a>, v: UserView) -> Self {
        Self { t, v }
    }
}

/// The data of the facts and of the action forms of one user.
///
/// The type solves one problem: the page `user.html` and the out-of-band answer of a command
/// must render the same markup. Both include `user-facts.html` and `user-actions.html` with
/// this value, so the markup lives in one place (design D1).
pub(crate) struct UserView {
    login: String,
    role: String,
    active: bool,
    status: String,
    must_change: bool,
    created: String,
    deactivated: Option<String>,
    failures: u32,
    last_failure: Option<String>,
    can_set_role: bool,
    /// True when the user may activate or deactivate this user in the current mode.
    can_change_status: bool,
    /// True when the user may unlock the login in the current mode.
    can_unlock: bool,
    /// True when the user may reset the password in the current mode.
    can_reset: bool,
    roles: Vec<RoleOption>,
    /// The value of the current role of the user.
    selected_role: &'static str,
    csrf_token: String,
    /// True when the markup goes out of band into a page that already shows the user.
    oob: bool,
    /// The error lines under the inputs of the set-role form.
    role_errors: FormErrors,
    /// True when the set-role form alone goes out of band (ADR-0041 (e)).
    role_oob: bool,
}

impl UserView {
    /// Return the view with the error lines of the set-role form, which swaps out of band.
    pub(super) fn with_role_errors(self, role_errors: FormErrors) -> Self {
        Self {
            role_errors,
            role_oob: true,
            ..self
        }
    }

    /// Set the forced password change that the facts show.
    ///
    /// The method solves one problem: the reset of a password sets the flag in the same
    /// transaction, after the read that made the view. The role decides the flag.
    pub(super) fn with_must_change(self, must_change: bool) -> Self {
        Self {
            must_change,
            ..self
        }
    }

    /// Build the view from the user and the throttle state of the login.
    ///
    /// `oob` is true for the answer of a command and false for the page of the user.
    ///
    /// Each action form needs `PageRequest::may_run` for its command, so the view shows no form
    /// in read-only mode (ADR-0113 (b)). The page and the answer of a command run the use case
    /// first, so a session exists.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(super) fn new(
        t: Texts<'_>,
        request: &PageRequest,
        detail: &UserDetail,
        oob: bool,
    ) -> Result<Self, WebError> {
        let user = &detail.user;
        let status_operation = if user.is_active {
            Operation::DeactivateUser
        } else {
            Operation::ActivateUser
        };
        let can_set_role = user.role.is_person() && request.may_run(Operation::SetUserRole)?;
        let can_change_status = request.may_run(status_operation)?;
        let can_unlock = detail.throttle.failures > 0 && request.may_run(Operation::UnlockLogin)?;
        let can_reset = request.may_run(Operation::ResetPassword)?;
        Ok(Self {
            login: user.login.to_string(),
            role: t.role(user.role),
            active: user.is_active,
            status: status_label(t, user.is_active),
            must_change: user.must_change_password,
            created: instant(&user.created_at),
            deactivated: user.deactivated_at.as_ref().map(instant),
            failures: detail.throttle.failures,
            last_failure: detail.throttle.last_failure.as_ref().map(instant),
            can_set_role,
            can_change_status,
            can_unlock,
            can_reset,
            roles: role_options(t, &Role::ALL),
            selected_role: user.role.as_sql(),
            csrf_token: request.csrf_token(),
            oob,
            role_errors: FormErrors::default(),
            role_oob: false,
        })
    }
}

/// The page `user.html`.
#[derive(Template)]
#[template(path = "user.html")]
pub(crate) struct UserPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    v: UserView,
}

impl<'a> UserPage<'a> {
    /// Build the page from the user and the throttle state of the login.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(super) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        detail: &UserDetail,
    ) -> Result<Self, WebError> {
        Ok(Self {
            layout: Layout::for_session(t, request, t.get("user-title"), None)?,
            t,
            v: UserView::new(t, request, detail, false)?,
        })
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
    technical: bool,
    login: String,
    refresh: Option<UserView>,
    facts: Option<UserView>,
}

impl<'a> ResultFragment<'a> {
    /// Build the fragment. `password` is the one-time password of the command, or `None`.
    ///
    /// `technical` is true for an `ExportClient` account. Its password is a permanent
    /// credential, so the page shows another notice (ADR-0109 (a)).
    pub(super) const fn new(
        t: Texts<'a>,
        message: String,
        password: Option<String>,
        technical: bool,
        login: String,
    ) -> Self {
        Self {
            t,
            message,
            password,
            technical,
            login,
            refresh: None,
            facts: None,
        }
    }

    /// Add the facts and the action forms of the user, sent out of band after the commit.
    pub(super) fn with_refresh(self, refresh: Option<UserView>) -> Self {
        Self { refresh, ..self }
    }

    /// Add only the facts of the user, sent out of band.
    ///
    /// The method solves one problem: the answer of `ResetPassword` must not replace the action
    /// forms, so the focus stays on the reset button (design D1, D2).
    pub(super) fn with_facts(self, facts: Option<UserView>) -> Self {
        Self { facts, ..self }
    }
}

/// The page `user-result-page.html`: the result of a command without HTMX.
#[derive(Template)]
#[template(path = "user-result-page.html")]
pub(crate) struct ResultPage<'a> {
    layout: Layout<'a>,
    t: Texts<'a>,
    password: Option<String>,
    technical: bool,
    login: String,
}

impl<'a> ResultPage<'a> {
    /// Build the page. `title` is the heading of the page.
    ///
    /// The success sentence of the fragment goes into `#toast` through the layout.
    ///
    /// # Errors
    ///
    /// Returns `E-WEB-004` when the request has no session.
    pub(super) fn new(
        t: Texts<'a>,
        request: &PageRequest,
        title: String,
        fragment: ResultFragment<'a>,
    ) -> Result<Self, WebError> {
        let layout = Layout::for_session(t, request, title, None)?.with_toast(fragment.message);
        Ok(Self {
            layout,
            t,
            password: fragment.password,
            technical: fragment.technical,
            login: fragment.login,
        })
    }
}
