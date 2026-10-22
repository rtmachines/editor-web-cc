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

#[tokio::test]
async fn security_returns_the_sink_that_the_context_holds() {
    let fixture = Fixture::new();
    let context = fixture.context();
    let event = recipe_domain::audit::SecurityEvent::csrf_rejected(
        meta().request_id(),
        std::net::IpAddr::from([192, 0, 2, 1]),
        None,
    );
    context.security().record(event).await.unwrap();
    assert_eq!(fixture.sink.events().len(), 1);
}

#[tokio::test]
async fn every_clone_of_a_context_shares_the_diff_gate_of_the_settings() {
    let fixture = Fixture::new();
    let settings = crate::blocking::AppSettings {
        diff: crate::blocking::DiffSettings {
            permits: 3,
            ..crate::blocking::DiffSettings::default()
        },
        ..crate::blocking::AppSettings::default()
    };
    let context = fixture.context_with(settings);
    let copy = context.clone();
    let deadline = context.settings.blocking.diff_deadline().unwrap();

    let permit = context.diff_gate.acquire(deadline).await.unwrap();

    assert_eq!(copy.diff_gate.available(), 2);
    drop(permit);
    assert_eq!(copy.diff_gate.available(), 3);
}

#[test]
fn the_fixture_context_uses_the_store_the_sink_and_the_clock_of_the_fixture() {
    let fixture = Fixture::new();
    let context = fixture.context();
    assert_eq!(context.settings, crate::blocking::AppSettings::default());
    assert_eq!(context.clock.now(), fixture.clock.now());
}
