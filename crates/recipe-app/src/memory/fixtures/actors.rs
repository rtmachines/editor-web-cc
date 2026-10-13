//! The fixed users and the request data of the tests.

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{RequestId, SessionRef, UserAgent};
use recipe_domain::identity::{Login, Role};

use super::{user, uuid};
use crate::auth::{CurrentUser, RequestMeta};

/// Build a login from a text that the test knows is valid.
fn login_of(text: &str) -> Login {
    Login::parse(text).unwrap()
}

/// Return the fixed user of `role`: `admin`, `jkowalski`, or `mhoffman`.
///
/// `anowak` has the role `ProcessEngineer` too. Use [`second_engineer`] for that user.
pub(crate) fn actor(role: Role) -> CurrentUser {
    match role {
        Role::Administrator => CurrentUser::new(user(1), login_of("admin"), role),
        Role::ProcessEngineer => CurrentUser::new(user(2), login_of("jkowalski"), role),
        Role::Operator => CurrentUser::new(user(4), login_of("mhoffman"), role),
    }
}

/// Return the second process engineer `anowak`, for separation-of-duties tests.
pub(crate) fn second_engineer() -> CurrentUser {
    CurrentUser::new(user(3), login_of("anowak"), Role::ProcessEngineer)
}

/// Return the request data of a request from `192.0.2.10` with a session.
pub(crate) fn meta() -> RequestMeta {
    RequestMeta::new(
        RequestId::from(uuid(0x0123)),
        Some(IpAddr::V4(Ipv4Addr::new(192, 0, 2, 10))),
        Some(UserAgent::from("recipe-app-test")),
        Some(SessionRef::parse("0123456789abcdef").unwrap()),
    )
}
