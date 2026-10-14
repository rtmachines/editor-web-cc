use super::*;
use crate::test_support::state;

fn assert_shareable<T: Clone + Send + Sync + 'static>() {}

#[test]
fn state_is_cheap_to_clone_and_shareable_between_threads() {
    assert_shareable::<WebState>();
}

#[test]
fn read_only_mode_starts_off() {
    assert!(!ReadOnlyMode::default().is_on());
    assert!(!state().read_only().is_on());
}

#[test]
fn read_only_mode_clones_share_one_flag() {
    let first = ReadOnlyMode::default();
    let second = first.clone();
    first.set(true);
    assert!(second.is_on());
    second.set(false);
    assert!(!first.is_on());
}

#[test]
fn state_clones_share_the_read_only_flag() {
    let first = state();
    let second = first.clone();
    first.read_only().set(true);
    assert!(second.read_only().is_on());
}

#[tokio::test]
async fn state_security_sink_is_the_sink_of_the_app_context() {
    let fixture = crate::test_support::Fixture::new();
    let event = recipe_domain::audit::SecurityEvent::csrf_rejected(
        recipe_domain::audit::RequestId::from(uuid::Uuid::nil()),
        std::net::IpAddr::from([192, 0, 2, 1]),
        None,
    );
    fixture.state.security().record(event).await.unwrap();
    assert_eq!(fixture.sink.events().len(), 1);
}

#[test]
fn state_holds_the_default_settings() {
    assert_eq!(
        state().settings().body_limit_form,
        WebSettings::default().body_limit_form
    );
}
