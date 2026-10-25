//! Helpers that the tests of the in-memory adapters of the jobs share.

use std::net::{IpAddr, Ipv4Addr};

use recipe_domain::audit::{LoginSubject, RequestId, SecurityEvent, UserAgent};
use recipe_domain::identity::{CsrfToken, SessionId, SessionToken, UserId};
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::memory::{MemoryClock, MemoryStore};
use crate::ports::NewSession;

/// The time of the clock at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 1_000_000;

/// The idle timeout of a seeded session: 30 minutes.
pub(super) const IDLE_SECONDS: u64 = 30 * 60;

pub(super) fn base() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap()
}

/// The instant `seconds` after the start of the test. A negative value is before it.
pub(super) fn at(seconds: i64) -> Timestamp {
    Timestamp::from(base().checked_add(TimeDuration::seconds(seconds)).unwrap())
}

/// An empty store whose clock starts at the start of the test.
pub(super) fn store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(base()))
}

/// Move the clock of `store` to `seconds` after the start of the test.
pub(super) fn set_time(store: &MemoryStore, seconds: i64) {
    store.clock().set(at(seconds));
}

pub(super) fn user_id(n: u128) -> UserId {
    UserId::from(Uuid::from_u128(n))
}

pub(super) fn request_id(n: u128) -> RequestId {
    RequestId::from(Uuid::from_u128(n))
}

pub(super) fn ip() -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1))
}

/// A session that expires `IDLE_SECONDS` after the time of the clock at the seed.
pub(super) fn new_session(n: u8) -> NewSession {
    NewSession {
        id: SessionId::of(&SessionToken::from_bytes([n; 16])),
        user_id: user_id(1),
        epoch: 0,
        csrf_token: CsrfToken::from_bytes([n; 32]),
        idle_timeout: std::time::Duration::from_secs(IDLE_SECONDS),
    }
}

pub(super) fn login_failed(n: u128) -> SecurityEvent {
    SecurityEvent::login_failed(
        LoginSubject::User(user_id(n)),
        request_id(n),
        ip(),
        Some(UserAgent::from("test")),
    )
}

pub(super) fn csrf_rejected(n: u128) -> SecurityEvent {
    SecurityEvent::csrf_rejected(request_id(n), ip(), None)
}

pub(super) fn forbidden(n: u128, operation: &str) -> SecurityEvent {
    SecurityEvent::forbidden(user_id(n), operation, request_id(n), Some(ip()), None)
}
