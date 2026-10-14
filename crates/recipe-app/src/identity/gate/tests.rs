use std::sync::Arc;
use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::{NewPassword, Password};

use super::HashGate;
use crate::error::{AppError, TaskStage, TimeoutStage};
use crate::identity::{BlockingHasher, PanickingHasher};

const ACQUIRE_TIMEOUT: Duration = Duration::from_secs(2);
const PERMITS: usize = 4;
const CALLERS: usize = 32;

fn typed() -> Password {
    Password::parse("tulip-orbit-lantern-77").unwrap()
}

fn fresh() -> NewPassword {
    NewPassword::parse("tulip-orbit-lantern-78").unwrap()
}

fn gate_with(permits: usize, hasher: &Arc<BlockingHasher>) -> HashGate {
    HashGate::new(permits, ACQUIRE_TIMEOUT, hasher.clone())
}

/// Wait until the permits of the gate come back, or until ten real seconds pass.
async fn wait_for_permits(gate: &HashGate, expected: usize) -> bool {
    let deadline = std::time::Instant::now()
        .checked_add(Duration::from_secs(10))
        .unwrap();
    while gate.available_permits() != expected {
        if std::time::Instant::now() > deadline {
            return false;
        }
        tokio::task::yield_now().await;
        std::thread::yield_now();
    }
    true
}

#[tokio::test(start_paused = true)]
async fn the_fifth_acquire_waits_for_the_timeout_and_gives_e_web_009() {
    let hasher = Arc::new(BlockingHasher::new());
    let gate = gate_with(PERMITS, &hasher);
    let mut held = Vec::new();
    for _ in 0..PERMITS {
        held.push(gate.acquire().await.unwrap());
    }
    assert_eq!(gate.available_permits(), 0);

    let started = tokio::time::Instant::now();
    let error = gate.acquire().await.err().unwrap();

    assert!(started.elapsed() >= ACQUIRE_TIMEOUT);
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Argon2Permit
        }
    ));
    drop(held);
    assert_eq!(gate.available_permits(), PERMITS);
}

#[tokio::test(start_paused = true)]
async fn a_dropped_permit_lets_the_next_caller_in() {
    let hasher = Arc::new(BlockingHasher::new());
    let gate = gate_with(1, &hasher);
    let first = gate.acquire().await.unwrap();

    drop(first);

    assert!(gate.acquire().await.is_ok());
}

#[tokio::test]
async fn one_permit_serves_a_check_and_then_a_hash() {
    let hasher = Arc::new(BlockingHasher::new());
    hasher.release();
    let gate = gate_with(1, &hasher);
    let permit = gate.acquire().await.unwrap();

    let (matched, permit) = permit.verify(typed(), None).await.unwrap();
    assert!(!matched);
    assert_eq!(gate.available_permits(), 0, "the permit must stay taken");
    let (hash, permit) = permit.hash(fresh()).await.unwrap();

    assert!(hash.as_str().starts_with("$argon2id$"));
    assert_eq!(gate.available_permits(), 0);
    drop(permit);
    assert_eq!(gate.available_permits(), 1);
}

#[tokio::test]
async fn a_panic_in_the_check_gives_e_web_011_and_returns_the_permit() {
    let gate = HashGate::new(1, ACQUIRE_TIMEOUT, Arc::new(PanickingHasher));
    let permit = gate.acquire().await.unwrap();

    let error = permit.verify(typed(), None).await.err().unwrap();

    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::PasswordHash
        }
    ));
    assert!(wait_for_permits(&gate, 1).await);
}

#[tokio::test]
async fn a_panic_in_the_hash_gives_e_web_011_and_returns_the_permit() {
    let gate = HashGate::new(1, ACQUIRE_TIMEOUT, Arc::new(PanickingHasher));
    let permit = gate.acquire().await.unwrap();

    let error = permit.hash(fresh()).await.err().unwrap();

    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(wait_for_permits(&gate, 1).await);
}

#[tokio::test(start_paused = true)]
async fn thirty_two_callers_never_run_more_than_four_checks_at_one_time() {
    let hasher = Arc::new(BlockingHasher::new());
    let gate = gate_with(PERMITS, &hasher);
    let mut callers = Vec::new();
    for _ in 0..CALLERS {
        let gate = gate.clone();
        callers.push(tokio::spawn(async move {
            let permit = gate.acquire().await?;
            let (matched, _permit) = permit.verify(typed(), None).await?;
            Ok::<bool, AppError>(matched)
        }));
    }

    assert!(hasher.wait_started(PERMITS).await);
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert_eq!(hasher.running(), PERMITS);
    // The blocked checks hold the paused clock. An explicit advance ends the wait of the rest.
    tokio::time::advance(ACQUIRE_TIMEOUT * 2).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    hasher.release();

    let mut passed = 0_usize;
    let mut timed_out = 0_usize;
    for caller in callers {
        match caller.await.unwrap() {
            Ok(_) => passed = passed.checked_add(1).unwrap(),
            Err(error) => {
                assert_eq!(error.code(), ErrorCode::WEB_009);
                timed_out = timed_out.checked_add(1).unwrap();
            }
        }
    }
    assert_eq!(passed, PERMITS);
    assert_eq!(timed_out, CALLERS - PERMITS);
    assert_eq!(hasher.max_running(), PERMITS);
    assert_eq!(hasher.started(), PERMITS);
}

#[tokio::test(start_paused = true)]
async fn a_caller_that_drops_its_future_keeps_the_permit_until_the_hash_ends() {
    let hasher = Arc::new(BlockingHasher::new());
    let gate = gate_with(1, &hasher);
    let caller = {
        let gate = gate.clone();
        tokio::spawn(async move {
            let permit = gate.acquire().await?;
            let (matched, _permit) = permit.verify(typed(), None).await?;
            Ok::<bool, AppError>(matched)
        })
    };
    assert!(hasher.wait_started(1).await);
    assert_eq!(gate.available_permits(), 0);

    caller.abort();
    assert!(caller.await.unwrap_err().is_cancelled());

    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        gate.available_permits(),
        0,
        "the permit must stay taken while the hash runs"
    );
    hasher.release();
    assert!(wait_for_permits(&gate, 1).await);
}
