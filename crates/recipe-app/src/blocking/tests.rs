use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;

use super::{AppSettings, Blocking, PageSizes};
use crate::error::{AppError, TaskStage, TimeoutStage};

const SHORT: Duration = Duration::from_millis(20);
const LONG: Duration = Duration::from_millis(400);

#[tokio::test]
async fn validate_passes_the_result_through() {
    let blocking = Blocking::default();
    let value = blocking.validate(|| 41_u8.checked_add(1)).await.unwrap();
    assert_eq!(value, Some(42));
}

#[tokio::test]
async fn a_slow_closure_gives_the_validation_timeout() {
    let blocking = Blocking::new(SHORT, Duration::from_secs(10));
    let error = blocking
        .validate(|| std::thread::sleep(LONG))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Validation
        }
    ));
}

#[tokio::test]
async fn a_panicking_closure_gives_task_failed() {
    let blocking = Blocking::default();
    let error = blocking
        .validate(|| std::hint::black_box(Option::<u8>::None).unwrap())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Validation
        }
    ));
}

#[tokio::test]
async fn the_elapsed_double_gives_the_timeout_without_running_the_work() {
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let error = Blocking::elapsed()
        .validate(move || flag.store(true, Ordering::SeqCst))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(!ran.load(Ordering::SeqCst));
}

#[tokio::test]
async fn the_panicked_double_gives_task_failed_without_running_the_work() {
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let error = Blocking::panicked()
        .validate(move || flag.store(true, Ordering::SeqCst))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(!ran.load(Ordering::SeqCst));
}

#[tokio::test(start_paused = true)]
async fn export_gives_the_export_timeout_when_the_future_is_slow() {
    let blocking = Blocking::new(Duration::from_secs(10), Duration::from_secs(10));
    let error = blocking
        .export(async {
            tokio::time::sleep(Duration::from_secs(11)).await;
            Ok::<u8, AppError>(1)
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
}

#[tokio::test(start_paused = true)]
async fn export_passes_the_result_of_a_fast_future() {
    let blocking = Blocking::default();
    let value = blocking
        .export(async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            Ok::<u8, AppError>(7)
        })
        .await
        .unwrap();
    assert_eq!(value, 7);
}

#[tokio::test(start_paused = true)]
async fn export_passes_the_error_of_the_future() {
    let error = Blocking::default()
        .export(async { Err::<u8, AppError>(AppError::Unauthenticated) })
        .await
        .unwrap_err();
    assert!(matches!(error, AppError::Unauthenticated));
}

#[tokio::test]
async fn export_rebuild_passes_the_result_of_the_closure() {
    let value = Blocking::default()
        .rebuild_export(|| 20_u8.checked_add(22))
        .await
        .unwrap();
    assert_eq!(value, Some(42));
}

#[tokio::test]
async fn a_panicking_export_rebuild_gives_task_failed_with_the_export_stage() {
    let error = Blocking::default()
        .rebuild_export(|| std::hint::black_box(Option::<u8>::None).unwrap())
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Export
        }
    ));
    assert_eq!(
        error.to_string(),
        "The operation export did not complete. Try again."
    );
}

#[test]
fn defaults_follow_section_7_11() {
    let settings = AppSettings::default();
    assert_eq!(
        settings.blocking,
        Blocking::new(Duration::from_secs(10), Duration::from_secs(10))
    );
    assert!(settings.approval_policy.separation_of_duties);
}

#[test]
fn page_sizes_default_to_100_rows_each() {
    let expected = PageSizes {
        machines: 100,
        type_usage: 100,
        recipes: 100,
        versions: 100,
        parameters: 100,
    };

    assert_eq!(PageSizes::default(), expected);
    assert_eq!(AppSettings::default().pages, expected);
}
