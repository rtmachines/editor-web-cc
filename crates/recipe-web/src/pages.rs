//! The pages and handlers of `api.md` section 2 (design D13).
//!
//! `own` holds the login, logout, profile, and password pages. Each handler parses the request
//! into value objects, calls one use case, and renders one template.

mod approval;
#[cfg(test)]
mod approval_security_tests;
#[cfg(test)]
mod approval_sweep_tests;
#[cfg(test)]
mod approval_text_xss_tests;
#[cfg(test)]
mod approval_xss_tests;
pub(crate) mod audit;
mod diff;
#[cfg(test)]
mod diff_lang_tests;
mod diff_view;
mod draft_reply;
pub(crate) mod history;
#[cfg(test)]
mod lifecycle_body_tests;
pub(crate) mod machines;
mod own;
#[cfg(test)]
mod page_tests;
pub(crate) mod parameters;
mod path_values;
#[cfg(test)]
mod plant_security_tests;
#[cfg(test)]
mod plant_sweep_tests;
#[cfg(test)]
mod plant_xss_tests;
#[cfg(test)]
mod recipe_security_tests;
#[cfg(test)]
mod recipe_sweep_tests;
#[cfg(test)]
mod recipe_xss_tests;
pub(crate) mod recipes;
#[cfg(test)]
mod redaction_tests;
mod render;
mod request;
#[cfg(test)]
mod review_fix_tests;
pub(crate) mod security_events;
#[cfg(test)]
mod session_end_tests;
#[cfg(test)]
mod sweep_tests;
#[cfg(test)]
pub(crate) mod test_helpers;
mod texts;
pub(crate) mod types;
pub(crate) mod units;
mod users;
pub(crate) mod versions;
#[cfg(test)]
mod xss_kit;
#[cfg(test)]
mod xss_tests;

pub(crate) use own::{
    change_password, login_form, login_page_with_error, login_submit, logout, password_form,
    profile,
};
pub(crate) use request::PageRequest;
pub(crate) use texts::Texts;
pub(crate) use users::{
    activate, create, deactivate, detail, list, new_form, reset_password, set_role, unlock,
};
