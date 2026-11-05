use std::net::Ipv4Addr;

use uuid::Uuid;

use super::*;

#[test]
fn current_user_returns_what_it_got() {
    let id = UserId::from(Uuid::from_u128(1));
    let login = Login::parse("anna").unwrap();
    let user = CurrentUser::new(id, login.clone(), Role::Operator);
    assert_eq!(user.user(), id);
    assert_eq!(user.login(), &login);
    assert_eq!(user.role(), Role::Operator);
}

#[test]
fn request_meta_returns_what_it_got() {
    let id = RequestId::from(Uuid::from_u128(7));
    let ip = IpAddr::V4(Ipv4Addr::LOCALHOST);
    let agent = UserAgent::from("test-agent");
    let session = SessionRef::parse("0123456789abcdef").unwrap();
    let meta = RequestMeta::new(id, Some(ip), Some(agent.clone()), Some(session.clone()));
    assert_eq!(meta.request_id(), id);
    assert_eq!(meta.client_ip(), Some(ip));
    assert_eq!(meta.user_agent(), Some(&agent));
    assert_eq!(meta.session_ref(), Some(&session));
    let bare = RequestMeta::new(id, None, None, None);
    assert_eq!(bare.session_ref(), None);
}
