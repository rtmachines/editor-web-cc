use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::IdentityError;

use super::serializable_once_more;
use crate::error::{AppError, LockFailure, StorageError};

fn lock_failure(cause: LockFailure) -> StorageError {
    StorageError::Deadlock { cause }
}

fn serialization() -> AppError {
    AppError::Storage(lock_failure(LockFailure::Serialization))
}

fn serialization_in_audit() -> AppError {
    AppError::AuditWriteFailed {
        cause: lock_failure(LockFailure::Serialization),
    }
}

/// Run the helper with a body that fails with `failures` in order and then succeeds.
///
/// Returns the result and the number of runs.
async fn run_with(failures: Vec<AppError>) -> (Result<u32, AppError>, u32) {
    let mut queue = failures.into_iter();
    let mut runs = 0_u32;
    let result = serializable_once_more(|| {
        runs = runs.checked_add(1).unwrap();
        let next = queue.next();
        async move {
            match next {
                Some(error) => Err(error),
                None => Ok(7),
            }
        }
    })
    .await;
    (result, runs)
}

#[tokio::test]
async fn success_runs_the_body_once() {
    let (result, runs) = run_with(vec![]).await;

    assert_eq!(result.unwrap(), 7);
    assert_eq!(runs, 1);
}

#[tokio::test]
async fn one_serialization_failure_runs_the_body_once_more() {
    let (result, runs) = run_with(vec![serialization()]).await;

    assert_eq!(result.unwrap(), 7);
    assert_eq!(runs, 2);
}

#[tokio::test]
async fn one_serialization_failure_inside_audit_write_failed_runs_the_body_once_more() {
    let (result, runs) = run_with(vec![serialization_in_audit()]).await;

    assert_eq!(result.unwrap(), 7);
    assert_eq!(runs, 2);
}

#[tokio::test]
async fn two_serialization_failures_give_e_db_001() {
    let (result, runs) = run_with(vec![serialization(), serialization()]).await;

    let error = result.unwrap_err();
    assert_eq!(runs, 2);
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert!(matches!(
        error,
        AppError::Storage(StorageError::Deadlock {
            cause: LockFailure::Serialization
        })
    ));
}

#[tokio::test]
async fn two_serialization_failures_in_audit_write_failed_give_e_db_001() {
    let (result, runs) = run_with(vec![serialization_in_audit(), serialization_in_audit()]).await;

    let error = result.unwrap_err();
    assert_eq!(runs, 2);
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert!(matches!(error, AppError::Storage(_)));
}

#[tokio::test]
async fn a_mixed_pair_of_serialization_failures_gives_e_db_001() {
    let (result, runs) = run_with(vec![serialization(), serialization_in_audit()]).await;

    let error = result.unwrap_err();
    assert_eq!(runs, 2);
    assert_eq!(error.code(), ErrorCode::DB_001);
    assert!(matches!(error, AppError::Storage(_)));
}

#[tokio::test]
async fn deadlock_and_lock_timeout_never_repeat() {
    for cause in [LockFailure::Deadlock, LockFailure::LockTimeout] {
        let (result, runs) = run_with(vec![AppError::Storage(lock_failure(cause))]).await;

        assert_eq!(runs, 1, "{cause:?}");
        assert!(
            matches!(result, Err(AppError::Storage(StorageError::Deadlock { cause: got })) if got == cause),
            "{cause:?}"
        );
    }
}

#[tokio::test]
async fn a_deadlock_inside_audit_write_failed_never_repeats() {
    let error = AppError::AuditWriteFailed {
        cause: lock_failure(LockFailure::Deadlock),
    };

    let (result, runs) = run_with(vec![error]).await;

    assert_eq!(runs, 1);
    assert!(matches!(result, Err(AppError::AuditWriteFailed { .. })));
}

#[tokio::test]
async fn any_other_error_returns_at_once() {
    let (result, runs) =
        run_with(vec![AppError::Identity(IdentityError::MustChangePassword)]).await;

    assert_eq!(runs, 1);
    assert_eq!(result.unwrap_err().code(), ErrorCode::IDN_007);
}

#[tokio::test]
async fn a_different_error_on_the_second_run_is_returned_unchanged() {
    let (result, runs) = run_with(vec![
        serialization(),
        AppError::Identity(IdentityError::MustChangePassword),
    ])
    .await;

    assert_eq!(runs, 2);
    assert_eq!(result.unwrap_err().code(), ErrorCode::IDN_007);
}
