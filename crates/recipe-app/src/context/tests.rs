use recipe_domain::identity::Role;

use std::sync::Arc;

use super::{AppContext, audit_context};
use crate::memory::fixtures::{Fixture, actor, meta};
use crate::ports::Clock as _;

#[test]
fn audit_context_copies_the_user_and_the_request() {
    let user = actor(Role::ProcessEngineer);
    let request = meta();
    let context = audit_context(&user, &request);
    assert_eq!(context.actor, Some(user.user()));
    assert_eq!(context.request_id, request.request_id());
    assert_eq!(context.client_ip, request.client_ip());
    assert_eq!(context.session_ref.as_ref(), request.session_ref());
}

#[test]
fn new_builds_a_context_from_the_four_parts() {
    let fixture = Fixture::new();
    let settings = crate::blocking::AppSettings::default();
    let context = AppContext::new(
        Arc::new(fixture.store.clone()),
        Arc::new(fixture.sink.clone()),
        Arc::new(fixture.clock.clone()),
        settings,
    );
    assert_eq!(context.settings, settings);
    assert_eq!(context.clock.now(), fixture.clock.now());
}

#[test]
fn the_fixture_context_uses_the_store_the_sink_and_the_clock_of_the_fixture() {
    let fixture = Fixture::new();
    let context = fixture.context();
    assert_eq!(context.settings, crate::blocking::AppSettings::default());
    assert_eq!(context.clock.now(), fixture.clock.now());
}
