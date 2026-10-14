//! Tests of the in-memory identity ports (task 3.4).

mod attempts;
mod begin_attempt;
mod session_store;
mod sessions;
mod users;

use std::net::{IpAddr, Ipv4Addr};
use std::time::Duration;

use recipe_domain::identity::{
    CsrfToken, Login, PasswordHash, Role, SessionId, SessionToken, TypedLogin, UserId,
};
use recipe_domain::timestamp::Timestamp;
use time::{Duration as TimeDuration, OffsetDateTime};
use uuid::Uuid;

use crate::memory::{JournalEntry, Lock, MemoryClock, MemoryStore, MemoryTransaction};
use crate::ports::{Isolation, NewSession, NewUser};

/// The time of the clock at the start of a test, in Unix seconds.
const BASE_SECONDS: i64 = 1_000_000;
/// A valid PHC string for a seeded user.
const HASH_A: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";
/// A second valid PHC string, so a test sees a changed hash.
const HASH_B: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$b3RoZXJzYWx0c2FsdA$b3RoZXJoYXNob3RoZXJoYXNob3RoZXJoYXNo";
/// The idle timeout of a seeded session: 30 minutes.
pub(super) const IDLE: Duration = Duration::from_secs(30 * 60);

pub(super) fn base() -> OffsetDateTime {
    OffsetDateTime::from_unix_timestamp(BASE_SECONDS).unwrap()
}

/// The instant `seconds` after the start of the test. A negative value is before it.
pub(super) fn at(seconds: i64) -> Timestamp {
    Timestamp::from(base().checked_add(TimeDuration::seconds(seconds)).unwrap())
}

pub(super) fn store() -> MemoryStore {
    MemoryStore::with_clock(MemoryClock::new(base()))
}

pub(super) fn login(text: &str) -> Login {
    Login::parse(text).unwrap()
}

pub(super) fn typed(text: &str) -> TypedLogin {
    TypedLogin::parse(text).unwrap()
}

pub(super) fn user_id(n: u128) -> UserId {
    UserId::from(Uuid::from_u128(n))
}

pub(super) fn hash_a() -> PasswordHash {
    PasswordHash::parse(HASH_A).unwrap()
}

pub(super) fn hash_b() -> PasswordHash {
    PasswordHash::parse(HASH_B).unwrap()
}

pub(super) fn new_user(n: u128, text: &str, role: Role) -> NewUser {
    NewUser {
        id: user_id(n),
        login: login(text),
        password_hash: hash_a(),
        role,
        must_change_password: false,
    }
}

pub(super) fn session_id(n: u8) -> SessionId {
    SessionId::of(&SessionToken::from_bytes([n; 16]))
}

pub(super) fn csrf(n: u8) -> CsrfToken {
    CsrfToken::from_bytes([n; 32])
}

pub(super) fn new_session(n: u8, user: UserId, epoch: i64) -> NewSession {
    NewSession {
        id: session_id(n),
        user_id: user,
        epoch,
        csrf_token: csrf(n),
        idle_timeout: IDLE,
    }
}

pub(super) fn ip(last: u8) -> IpAddr {
    IpAddr::V4(Ipv4Addr::new(192, 0, 2, last))
}

pub(super) fn begin(store: &MemoryStore) -> MemoryTransaction {
    store.begin_memory(Isolation::ReadCommitted).unwrap()
}

/// The lock calls of the journal, without the `Begin` lines.
pub(super) fn locks(store: &MemoryStore) -> Vec<Lock> {
    store
        .journal()
        .into_iter()
        .filter_map(|line| match line {
            JournalEntry::Lock(lock) => Some(lock),
            JournalEntry::Begin(_) => None,
        })
        .collect()
}

/// The text of an instant in a row image.
pub(super) fn text(instant: Timestamp) -> String {
    instant.as_offset_date_time().to_string()
}
