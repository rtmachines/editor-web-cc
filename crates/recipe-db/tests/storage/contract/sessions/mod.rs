//! Contract cases of `SessionRepository` and `SessionStore` (task 5.5 of `identity-and-sessions`).
//!
//! Each case runs on `MemoryStore` and on `Database` with the same assertions. The cases that
//! need PostgreSQL carry the prefix `postgres_only`. A time rule uses a zero timeout, because a
//! later transaction always has a later `now()` than the insert.

mod postgres;

use std::time::Duration;

use recipe_app::error::StorageError;
use recipe_app::ports::{NewSession, SessionLimits};
use recipe_domain::identity::{CsrfToken, Role, SessionId, SessionToken, UserId};

use crate::contract::harness::{Harness, begin, commit, new_user, seed_user};
use crate::contract::seed::machine_id;

/// The limits of a normal request: idle 30 minutes, absolute 12 hours.
const LIMITS: SessionLimits = SessionLimits {
    idle_timeout: Duration::from_secs(30 * 60),
    absolute_timeout: Duration::from_secs(12 * 3600),
};

/// The idle timeout that the insert of a session uses.
const INSERT_IDLE: Duration = Duration::from_secs(10 * 60);

/// A session value with a token of equal bytes. `byte` makes the token unique.
fn new_session(user: UserId, epoch: i64, byte: u8, idle: Duration) -> (SessionToken, NewSession) {
    let token = SessionToken::from_bytes([byte; 16]);
    let session = NewSession {
        id: SessionId::of(&token),
        user_id: user,
        epoch,
        csrf_token: CsrfToken::from_bytes([byte.wrapping_add(1); 32]),
        idle_timeout: idle,
    };
    (token, session)
}

/// Insert a session in its own committed transaction.
async fn store(h: &dyn Harness, session: &NewSession) {
    let mut tx = begin(h).await;
    tx.sessions().insert(session).await.unwrap();
    commit(h, tx, machine_id(1)).await;
}

/// Seed a user and one session with epoch 0.
async fn user_with_session(h: &dyn Harness, login: &str, byte: u8) -> (UserId, SessionId) {
    let user = new_user(login, Role::ProcessEngineer);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let (_, session) = new_session(user.id, 0, byte, INSERT_IDLE);
    store(h, &session).await;
    (user.id, session.id)
}

async fn authenticate(h: &dyn Harness, id: &SessionId) -> bool {
    h.session_store()
        .authenticate(id, &LIMITS)
        .await
        .unwrap()
        .is_some()
}

async fn a_valid_session_gives_the_session_and_the_user_columns(h: &dyn Harness) {
    let mut user = new_user("akowal", Role::ProcessEngineer);
    user.must_change_password = true;
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let (_, session) = new_session(user.id, 0, 7, INSERT_IDLE);
    store(h, &session).await;

    let row = h
        .session_store()
        .authenticate(&session.id, &LIMITS)
        .await
        .unwrap()
        .expect("the session is valid");

    assert_eq!(row.id, session.id);
    assert_eq!(row.user_id, user.id);
    assert_eq!(row.login, user.login);
    assert_eq!(row.role, Role::ProcessEngineer);
    assert!(row.must_change_password);
    assert_eq!(row.epoch, 0);
    assert_eq!(row.csrf_token.to_hex(), session.csrf_token.to_hex());
    assert!(row.last_seen_at >= row.created_at);
}

async fn a_valid_call_moves_expires_at_by_the_idle_timeout(h: &dyn Harness) {
    let (_, id) = user_with_session(h, "akowal", 7).await;

    let row = h
        .session_store()
        .authenticate(&id, &LIMITS)
        .await
        .unwrap()
        .expect("the session is valid");

    let idle = time::Duration::try_from(LIMITS.idle_timeout).unwrap();
    let first_deadline = row
        .created_at
        .as_offset_date_time()
        .checked_add(time::Duration::try_from(INSERT_IDLE).unwrap())
        .unwrap();
    assert_eq!(
        row.last_seen_at.as_offset_date_time().checked_add(idle),
        Some(*row.expires_at.as_offset_date_time())
    );
    assert!(*row.expires_at.as_offset_date_time() > first_deadline);
}

async fn an_idle_session_gives_none(h: &dyn Harness) {
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    let (_, session) = new_session(user.id, 0, 7, Duration::ZERO);
    store(h, &session).await;

    assert!(!authenticate(h, &session.id).await);
}

async fn a_session_older_than_the_absolute_timeout_gives_none(h: &dyn Harness) {
    let (_, id) = user_with_session(h, "akowal", 7).await;
    let limits = SessionLimits {
        idle_timeout: LIMITS.idle_timeout,
        absolute_timeout: Duration::ZERO,
    };

    let row = h.session_store().authenticate(&id, &limits).await.unwrap();

    assert!(row.is_none());
    assert!(authenticate(h, &id).await);
}

async fn the_session_of_an_inactive_user_gives_none(h: &dyn Harness) {
    let user = new_user("akowal", Role::Operator);
    seed_user(h.unit_of_work().as_ref(), &user).await;
    // The deactivation raises the epoch to 1, so the session below matches the epoch.
    let (_, session) = new_session(user.id, 1, 7, INSERT_IDLE);
    let mut tx = begin(h).await;
    tx.users().deactivate(user.id).await.unwrap();
    tx.sessions().insert(&session).await.unwrap();
    commit(h, tx, machine_id(1)).await;

    assert!(!authenticate(h, &session.id).await);
}

async fn a_session_with_another_epoch_gives_none(h: &dyn Harness) {
    let (user, id) = user_with_session(h, "akowal", 7).await;
    let (_, ahead) = new_session(user, 5, 9, INSERT_IDLE);
    store(h, &ahead).await;
    assert!(authenticate(h, &id).await);

    let mut tx = begin(h).await;
    tx.users().set_role(user, Role::Operator).await.unwrap();
    commit(h, tx, machine_id(1)).await;

    assert!(!authenticate(h, &id).await);
    assert!(!authenticate(h, &ahead.id).await);
}

async fn a_deleted_and_an_unknown_session_give_none_without_an_error(h: &dyn Harness) {
    let (_, id) = user_with_session(h, "akowal", 7).await;
    let mut tx = begin(h).await;
    let existed = tx.sessions().delete(&id).await.unwrap();
    let again = tx.sessions().delete(&id).await.unwrap();
    commit(h, tx, machine_id(1)).await;
    let unknown = SessionId::of(&SessionToken::from_bytes([0xee; 16]));

    let deleted = h.session_store().authenticate(&id, &LIMITS).await;
    let missing = h.session_store().authenticate(&unknown, &LIMITS).await;

    assert!(existed);
    assert!(!again);
    assert!(matches!(deleted, Ok(None)), "{deleted:?}");
    assert!(matches!(missing, Ok(None)), "{missing:?}");
}

async fn delete_all_of_user_deletes_only_the_rows_of_that_user(h: &dyn Harness) {
    let (first, first_id) = user_with_session(h, "akowal", 1).await;
    let (_, second_id) = user_with_session(h, "jkowalski", 3).await;
    let (_, extra) = new_session(first, 0, 5, INSERT_IDLE);
    store(h, &extra).await;
    let mut tx = begin(h).await;

    let deleted = tx.sessions().delete_all_of_user(first).await.unwrap();
    let none_left = tx.sessions().delete_all_of_user(first).await.unwrap();
    commit(h, tx, machine_id(1)).await;

    assert_eq!((deleted, none_left), (2, 0));
    assert!(!authenticate(h, &first_id).await);
    assert!(!authenticate(h, &extra.id).await);
    assert!(authenticate(h, &second_id).await);
}

async fn a_known_id_and_an_unknown_user_give_the_constraint_errors(h: &dyn Harness) {
    let (user, id) = user_with_session(h, "akowal", 1).await;
    let (_, same_id) = new_session(user, 0, 1, INSERT_IDLE);
    let stranger = UserId::from(uuid::Uuid::from_u128(404));
    let (_, orphan) = new_session(stranger, 0, 2, INSERT_IDLE);
    let mut tx = begin(h).await;

    let duplicate = tx.sessions().insert(&same_id).await.unwrap_err();
    drop(tx);
    let mut tx = begin(h).await;
    let missing = tx.sessions().insert(&orphan).await.unwrap_err();

    assert_eq!(same_id.id, id);
    assert!(
        matches!(&duplicate, StorageError::Duplicate { constraint } if constraint == "session_pkey"),
        "{duplicate:?}"
    );
    assert!(
        matches!(&missing, StorageError::MissingReference { constraint } if constraint == "session_user_id_fkey"),
        "{missing:?}"
    );
}

contract_case!(
    a_valid_session_gives_the_session_and_the_user_columns,
    a_valid_session_gives_the_session_and_the_user_columns_memory,
    a_valid_session_gives_the_session_and_the_user_columns_postgres
);
contract_case!(
    a_valid_call_moves_expires_at_by_the_idle_timeout,
    a_valid_call_moves_expires_at_by_the_idle_timeout_memory,
    a_valid_call_moves_expires_at_by_the_idle_timeout_postgres
);
contract_case!(
    an_idle_session_gives_none,
    an_idle_session_gives_none_memory,
    an_idle_session_gives_none_postgres
);
contract_case!(
    a_session_older_than_the_absolute_timeout_gives_none,
    a_session_older_than_the_absolute_timeout_gives_none_memory,
    a_session_older_than_the_absolute_timeout_gives_none_postgres
);
contract_case!(
    the_session_of_an_inactive_user_gives_none,
    the_session_of_an_inactive_user_gives_none_memory,
    the_session_of_an_inactive_user_gives_none_postgres
);
contract_case!(
    a_session_with_another_epoch_gives_none,
    a_session_with_another_epoch_gives_none_memory,
    a_session_with_another_epoch_gives_none_postgres
);
contract_case!(
    a_deleted_and_an_unknown_session_give_none_without_an_error,
    a_deleted_and_an_unknown_session_give_none_without_an_error_memory,
    a_deleted_and_an_unknown_session_give_none_without_an_error_postgres
);
contract_case!(
    delete_all_of_user_deletes_only_the_rows_of_that_user,
    delete_all_of_user_deletes_only_the_rows_of_that_user_memory,
    delete_all_of_user_deletes_only_the_rows_of_that_user_postgres
);
contract_case!(
    a_known_id_and_an_unknown_user_give_the_constraint_errors,
    a_known_id_and_an_unknown_user_give_the_constraint_errors_memory,
    a_known_id_and_an_unknown_user_give_the_constraint_errors_postgres
);
