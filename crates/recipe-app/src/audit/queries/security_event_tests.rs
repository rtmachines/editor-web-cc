//! Tests of `ListSecurityEvents` (spec "Security event list query", ADR-0051 (h)).
//!
//! | id | time (UTC)       | kind           | subject                          | client      |
//! |----|------------------|----------------|----------------------------------|-------------|
//! | 1  | 2026-10-16 10:00 | `login_failed` | `ghost`, HMAC of the current key | 192.0.2.1   |
//! | 2  | 2026-10-16 10:05 | `login_failed` | `ghost`, HMAC of the current key | 192.0.2.1   |
//! | 3  | 2026-10-16 10:10 | `login_failed` | `ghost`, HMAC of the older key   | 192.0.2.1   |
//! | 4  | 2026-10-16 10:15 | `forbidden`    | user `mhoffman`                  | 192.0.2.2   |
//! | 5  | 2026-10-17 00:00 | `csrf_rejected`| none                             | 192.0.2.3   |

use std::net::{IpAddr, Ipv4Addr};
use std::num::NonZeroU64;

use recipe_domain::audit::{LoginHmac, LoginSubject, RequestId, SecurityEvent, SecurityEventKind};
use recipe_domain::identity::{KeyRing, Login, PasswordHash, Role, TypedLogin};
use recipe_domain::timestamp::Timestamp;
use uuid::Uuid;

use super::test_world::october;
use super::{ListSecurityEvents, SecurityEventListInput};
use crate::blocking::{AppSettings, PageSizes};
use crate::context::AppContext;
use crate::identity::IdentityContext;
use crate::identity::fixtures::IdentityFixture;
use crate::memory::fixtures::user;
use crate::ports::{
    CursorPage, NewUser, SecurityEventId, SecurityEventRow, SecurityEventSink as _, TimeRange,
};

/// The current key of the login key ring of these tests.
const CURRENT_KEY: &str = "0202020202020202020202020202020202020202020202020202020202020202";
/// The older key of the login key ring of these tests.
const OLDER_KEY: &str = "0303030303030303030303030303030303030303030303030303030303030303";
/// The number of the user `mhoffman`.
const MHOFFMAN: u128 = 4;

/// Return the address `192.0.2.<last>`.
fn address(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 0, 2, last))
}

/// Return the typed login `text`.
fn typed(text: &str) -> TypedLogin {
    TypedLogin::parse(text).unwrap()
}

/// Return the request id `n`.
fn request(n: u128) -> RequestId {
    RequestId::from(Uuid::from_u128(n))
}

/// Return a `login_failed` event of the unknown login `text` with the HMAC under `key`.
fn failed_for(n: u128, key: &str, text: &str) -> SecurityEvent {
    let hmac = LoginHmac::compute(&KeyRing::parse(key).unwrap(), &typed(text));
    SecurityEvent::login_failed(LoginSubject::Unknown(hmac), request(n), address(1), None)
}

/// Return a `login_failed` event of the unknown login `ghost` with the HMAC under `key`.
fn ghost_failed(n: u128, key: &str) -> SecurityEvent {
    failed_for(n, key, "ghost")
}

/// Return the five events of the module table with their times.
fn events() -> [(Timestamp, SecurityEvent); 5] {
    let forbidden = SecurityEvent::forbidden(
        user(MHOFFMAN),
        "CreateMachine",
        request(4),
        Some(address(2)),
        None,
    );
    [
        (october(16, 10, 0), ghost_failed(1, CURRENT_KEY)),
        (october(16, 10, 5), ghost_failed(2, CURRENT_KEY)),
        (october(16, 10, 10), ghost_failed(3, OLDER_KEY)),
        (october(16, 10, 15), forbidden),
        (
            october(17, 0, 0),
            SecurityEvent::csrf_rejected(request(5), address(3), None),
        ),
    ]
}

/// Return the fixture with `mhoffman` and the five events of the module table.
async fn world() -> IdentityFixture {
    let fixture = IdentityFixture::new();
    fixture.store.seed_user(&NewUser {
        id: user(MHOFFMAN),
        login: Login::parse("mhoffman").unwrap(),
        password_hash: PasswordHash::parse("$argon2id$fake$secret").unwrap(),
        role: Role::Operator,
        must_change_password: false,
    });
    for (instant, event) in events() {
        fixture.store.clock().set(instant);
        fixture.store.sink().record(event).await.unwrap();
    }
    fixture
}

/// Return the context of `fixture` with the ring of the current key and the older key.
fn context(fixture: &IdentityFixture, pages: u32) -> IdentityContext {
    let settings = AppSettings {
        pages: PageSizes {
            security_events: pages,
            ..PageSizes::default()
        },
        ..AppSettings::default()
    };
    let app = AppContext {
        settings,
        ..fixture.context.app.clone()
    };
    IdentityContext {
        app,
        login_keys: KeyRing::parse(&format!("{CURRENT_KEY}\n{OLDER_KEY}")).unwrap(),
        ..fixture.context.clone()
    }
}

/// Run `ListSecurityEvents` as the Administrator with `input` and pages of `pages` rows.
async fn list_with(
    fixture: &IdentityFixture,
    pages: u32,
    input: SecurityEventListInput,
) -> CursorPage<SecurityEventRow> {
    let (auth, meta) = fixture.actor(Role::Administrator).await;
    ListSecurityEvents::new(context(fixture, pages))
        .execute(&auth, &meta, input)
        .await
        .unwrap()
}

/// Run `ListSecurityEvents` as the Administrator with `input` and return the ids.
async fn list(fixture: &IdentityFixture, input: SecurityEventListInput) -> Vec<u64> {
    let page = list_with(fixture, 100, input).await;
    page.rows.iter().map(|row| row.id.get()).collect()
}

/// Return the input without a filter and without a cursor.
fn no_filter() -> SecurityEventListInput {
    SecurityEventListInput {
        kind: None,
        login: None,
        client_ip: None,
        range: TimeRange::default(),
        before: None,
    }
}

#[tokio::test]
async fn the_list_holds_the_newest_event_first() {
    let fixture = world().await;
    assert_eq!(list(&fixture, no_filter()).await, [5, 4, 3, 2, 1]);
}

#[tokio::test]
async fn login_matches_the_events_of_an_unknown_login_by_the_hmac_of_the_current_key() {
    let fixture = world().await;
    let input = SecurityEventListInput {
        login: Some(typed("ghost")),
        ..no_filter()
    };
    assert_eq!(list(&fixture, input).await, [2, 1]);
}

#[tokio::test]
async fn an_event_under_an_older_key_does_not_match_the_login() {
    let fixture = world().await;
    let input = SecurityEventListInput {
        login: Some(typed("ghost")),
        ..no_filter()
    };
    assert!(!list(&fixture, input).await.contains(&3));
    let all = list(&fixture, no_filter()).await;
    assert!(all.contains(&3), "the event exists");
}

#[tokio::test]
async fn a_typed_login_outside_the_login_grammar_matches_only_by_its_hmac() {
    let fixture = world().await;
    let text = "jan@firma.pl";
    assert!(
        Login::parse(text).is_err(),
        "the typed login breaks the grammar"
    );
    for (n, instant) in [(6, october(16, 11, 0)), (7, october(16, 11, 5))] {
        fixture.store.clock().set(instant);
        let event = failed_for(n, CURRENT_KEY, text);
        fixture.store.sink().record(event).await.unwrap();
    }
    let input = SecurityEventListInput {
        login: Some(typed(text)),
        ..no_filter()
    };
    assert_eq!(list(&fixture, input).await, [7, 6]);
    let all = list(&fixture, no_filter()).await;
    assert_eq!(all.len(), 7, "the other events exist");
}

#[tokio::test]
async fn login_matches_the_events_of_a_known_user() {
    let fixture = world().await;
    let input = SecurityEventListInput {
        login: Some(typed("mhoffman")),
        ..no_filter()
    };
    assert_eq!(list(&fixture, input).await, [4]);
}

#[tokio::test]
async fn kind_client_ip_and_range_keep_their_events() {
    let fixture = world().await;
    let cases = [
        (
            "kind",
            SecurityEventListInput {
                kind: Some(SecurityEventKind::LoginFailed),
                ..no_filter()
            },
            vec![3, 2, 1],
        ),
        (
            "client_ip",
            SecurityEventListInput {
                client_ip: Some(address(2)),
                ..no_filter()
            },
            vec![4],
        ),
        (
            "range",
            SecurityEventListInput {
                range: TimeRange {
                    from: Some(october(16, 10, 5)),
                    until: Some(october(17, 0, 0)),
                },
                ..no_filter()
            },
            vec![4, 3, 2],
        ),
    ];
    for (name, input, expected) in cases {
        assert_eq!(list(&fixture, input).await, expected, "{name}");
    }
}

#[tokio::test]
async fn an_ipv4_mapped_client_ip_matches_the_events_of_its_ipv4_address() {
    let fixture = world().await;
    let mapped = Ipv4Addr::new(192, 0, 2, 2).to_ipv6_mapped();
    let input = SecurityEventListInput {
        client_ip: Some(IpAddr::V6(mapped)),
        ..no_filter()
    };
    assert_eq!(list(&fixture, input).await, [4]);
}

#[tokio::test]
async fn the_cursor_gives_the_older_events_and_tells_whether_more_exist() {
    let fixture = world().await;
    let mut seen = Vec::new();
    for before in [None, Some(4), Some(2)] {
        let input = SecurityEventListInput {
            before: before.and_then(NonZeroU64::new).map(SecurityEventId::from),
            ..no_filter()
        };
        let page = list_with(&fixture, 2, input).await;
        let ids: Vec<u64> = page.rows.iter().map(|row| row.id.get()).collect();
        seen.push((ids, page.has_older));
    }
    assert_eq!(
        seen,
        [(vec![5, 4], true), (vec![3, 2], true), (vec![1], false)]
    );
}

#[tokio::test]
async fn a_last_page_that_fills_the_page_size_tells_that_no_older_event_exists() {
    let fixture = world().await;
    let input = SecurityEventListInput {
        before: NonZeroU64::new(4).map(SecurityEventId::from),
        ..no_filter()
    };
    let page = list_with(&fixture, 3, input).await;
    let ids: Vec<u64> = page.rows.iter().map(|row| row.id.get()).collect();
    assert_eq!(ids, [3, 2, 1]);
    assert!(!page.has_older);
}

#[tokio::test]
async fn a_row_names_the_login_of_its_user() {
    let fixture = world().await;
    let page = list_with(&fixture, 100, no_filter()).await;
    let logins: Vec<Option<String>> = page
        .rows
        .iter()
        .map(|row| row.login.as_ref().map(ToString::to_string))
        .collect();
    assert_eq!(
        logins,
        [None, Some("mhoffman".to_owned()), None, None, None]
    );
}
