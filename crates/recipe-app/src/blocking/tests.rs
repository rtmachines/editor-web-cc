use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use recipe_domain::error_codes::ErrorCode;

use super::{
    AppSettings, Blocking, DiffGate, DiffPermit, DiffSettings, ExportGate, ExportPermit,
    ExportSettings, PageSizes,
};
use crate::error::{AppError, TaskStage, TimeoutStage};

const SHORT: Duration = Duration::from_millis(20);
const LONG: Duration = Duration::from_millis(400);
/// Bound of a wait for an export permit. A task that ends frees the permit at once, so the bound
/// only turns a leaked permit into a failure. It is virtual time in a test with a paused clock.
const PERMIT_WAIT_BOUND: Duration = Duration::from_secs(10);

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

/// Return a gate of the exports with one permit and its permit.
async fn one_export_permit() -> (ExportGate, ExportPermit) {
    let gate = ExportGate::new(1);
    let permit = gate.acquire().await.unwrap();
    (gate, permit)
}

#[tokio::test]
async fn export_rebuild_passes_the_result_of_the_closure_and_frees_the_permit() {
    let (gate, permit) = one_export_permit().await;
    let value = Blocking::default()
        .rebuild_export(permit, || 20_u8.checked_add(22))
        .await
        .unwrap();
    assert_eq!(value, Some(42));
    assert_eq!(gate.available(), 1);
}

#[tokio::test]
async fn a_panicking_export_rebuild_gives_task_failed_with_the_export_stage() {
    let (gate, permit) = one_export_permit().await;
    let error = Blocking::default()
        .rebuild_export(permit, || std::hint::black_box(Option::<u8>::None).unwrap())
        .await
        .unwrap_err();
    assert_eq!(gate.available(), 1);
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
fn page_sizes_follow_section_7_11() {
    let expected = PageSizes {
        machines: 100,
        type_usage: 100,
        recipes: 100,
        versions: 100,
        parameters: 100,
        audit: 100,
        history: 100,
        security_events: 100,
        diff: 50,
    };

    assert_eq!(PageSizes::default(), expected);
    assert_eq!(AppSettings::default().pages, expected);
}

#[test]
fn diff_settings_follow_section_7_11() {
    let expected = DiffSettings {
        leaf_rows: 1_000,
        permits: 2,
    };

    assert_eq!(DiffSettings::default(), expected);
    assert_eq!(AppSettings::default().diff, expected);
}

#[test]
fn export_settings_follow_section_7_11() {
    let expected = ExportSettings { permits: 2 };

    assert_eq!(ExportSettings::default(), expected);
    assert_eq!(AppSettings::default().export, expected);
}

#[tokio::test(start_paused = true)]
async fn a_second_export_permit_waits_until_the_first_returns() {
    let (gate, held) = one_export_permit().await;
    let mut second = std::pin::pin!(gate.acquire());

    let waited = tokio::time::timeout(Duration::from_secs(60), &mut second).await;
    assert!(waited.is_err());
    drop(held);
    let permit = second.await.unwrap();

    assert_eq!(gate.available(), 0);
    drop(permit);
    assert_eq!(gate.available(), 1);
}

/// The limit of `timeout.export` elapses while the rebuild runs. The paused clock moves only
/// through `advance`, which the test calls after the closure signals its start.
#[tokio::test(start_paused = true)]
async fn an_export_rebuild_beyond_the_limit_keeps_its_permit_until_the_task_ends() {
    let blocking = Blocking::new(Duration::from_secs(10), SHORT);
    let (gate, permit) = one_export_permit().await;
    let (release, released) = std::sync::mpsc::channel::<()>();
    let (started_sender, started) = tokio::sync::oneshot::channel::<()>();
    let work = move || {
        started_sender.send(()).is_ok() && released.recv_timeout(Duration::from_secs(5)).is_ok()
    };
    let export = blocking.export(blocking.rebuild_export(permit, work));
    let pass_the_limit = async {
        started.await.unwrap();
        tokio::time::advance(SHORT.checked_add(Duration::from_millis(1)).unwrap()).await;
    };

    let (result, ()) = tokio::join!(export, pass_the_limit);
    let error = result.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Export
        }
    ));
    assert_eq!(gate.available(), 0);
    release.send(()).unwrap();
    let returned = tokio::time::timeout(PERMIT_WAIT_BOUND, gate.acquire())
        .await
        .expect("the end of the task did not free the permit");
    drop(returned.unwrap());
    assert_eq!(gate.available(), 1);
}

/// The pool has one thread, which a held task occupies, so the rebuild waits in the queue
/// until the limit elapses. The drop of the export future must abort the queued task.
#[test]
fn an_export_rebuild_queued_beyond_the_limit_never_runs_and_frees_the_permit() {
    one_blocking_thread().block_on(async {
        let blocking = Blocking::new(Duration::from_secs(10), SHORT);
        let (gate, permit) = one_export_permit().await;
        let (release, released) = std::sync::mpsc::channel::<()>();
        let busy = tokio::task::spawn_blocking(move || {
            released.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);

        let error = blocking
            .export(blocking.rebuild_export(permit, move || flag.store(true, Ordering::SeqCst)))
            .await
            .unwrap_err();
        release.send(()).unwrap();
        assert!(busy.await.unwrap());
        let next = tokio::time::timeout(PERMIT_WAIT_BOUND, gate.acquire())
            .await
            .expect("the dropped export future did not free the permit")
            .unwrap();

        assert_eq!(error.code(), ErrorCode::WEB_009);
        assert!(!ran.load(Ordering::SeqCst));
        assert_eq!(gate.available(), 0);
        drop(next);
        assert_eq!(gate.available(), 1);
    });
}

/// Return the deadline of a diff of `blocking`, which starts now.
fn deadline_of(blocking: &Blocking) -> tokio::time::Instant {
    blocking.diff_deadline().unwrap()
}

/// Return a gate of one permit and its permit.
async fn one_permit() -> (DiffGate, DiffPermit) {
    let gate = DiffGate::new(1);
    let permit = gate
        .acquire(deadline_of(&Blocking::default()))
        .await
        .unwrap();
    (gate, permit)
}

/// Wait until `gate` has `free` free permits, at most two seconds.
pub(crate) async fn wait_for_free(gate: &DiffGate, free: usize) {
    for _ in 0..400 {
        if gate.available() == free {
            return;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

#[tokio::test]
async fn diff_passes_the_result_and_the_permit_through() {
    let blocking = Blocking::default();
    let (gate, permit) = one_permit().await;

    let (value, permit) = blocking
        .diff(deadline_of(&blocking), permit, || 41_u8.checked_add(1))
        .await
        .unwrap();

    assert_eq!(value, Some(42));
    assert_eq!(gate.available(), 0);
    drop(permit);
    assert_eq!(gate.available(), 1);
}

/// `start_paused` pauses the clock. A current-thread runtime does not auto-advance a paused clock
/// while a blocking task lives. So the deadline passes only when the test calls `advance`, and
/// the test calls it after the closure signals its start. The semaphore wakes the wait for the
/// permit. That wait has a deadline on the paused clock. The clock cannot auto-advance while the
/// task lives, and the task drops the permit before it ends.
#[tokio::test(start_paused = true)]
async fn a_diff_beyond_the_deadline_holds_its_permit_until_the_task_ends() {
    let blocking = Blocking::new(SHORT, Duration::from_secs(10));
    let (gate, permit) = one_permit().await;
    let (release, released) = std::sync::mpsc::channel::<()>();
    let (started_sender, started) = tokio::sync::oneshot::channel::<()>();
    let work = move || {
        started_sender.send(()).is_ok() && released.recv_timeout(Duration::from_secs(5)).is_ok()
    };
    let diff = blocking.diff(deadline_of(&blocking), permit, work);
    let pass_the_deadline = async {
        started.await.unwrap();
        tokio::time::advance(SHORT.checked_add(Duration::from_millis(1)).unwrap()).await;
    };

    let (result, ()) = tokio::join!(diff, pass_the_deadline);
    let error = result.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    assert_eq!(gate.available(), 0);
    release.send(()).unwrap();
    let returned = gate
        .acquire(deadline_of(&Blocking::default()))
        .await
        .unwrap();
    drop(returned);
    assert_eq!(gate.available(), 1);
}

/// Build a runtime whose blocking pool has one thread, so a second blocking task waits.
pub(crate) fn one_blocking_thread() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .max_blocking_threads(1)
        .build()
        .unwrap()
}

#[test]
fn a_diff_that_waits_for_the_pool_beyond_its_deadline_never_runs_and_frees_the_permit() {
    one_blocking_thread().block_on(async {
        let blocking = Blocking::new(SHORT, Duration::from_secs(10));
        let (gate, permit) = one_permit().await;
        let (release, released) = std::sync::mpsc::channel::<()>();
        let busy = tokio::task::spawn_blocking(move || {
            released.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);

        let error = blocking
            .diff(deadline_of(&blocking), permit, move || {
                flag.store(true, Ordering::SeqCst)
            })
            .await
            .unwrap_err();
        release.send(()).unwrap();
        assert!(busy.await.unwrap());
        wait_for_free(&gate, 1).await;

        assert_eq!(error.code(), ErrorCode::WEB_009);
        assert!(!ran.load(Ordering::SeqCst));
        assert_eq!(gate.available(), 1);
    });
}

/// The caller drops the future of `Blocking::diff` while its task waits in the queue of the
/// blocking pool. The runtime has one blocking thread, which a held task occupies. The test
/// frees that thread and then waits for the permit. A task that runs gives its permit back only
/// after its closure ran. An aborted task gives it back without a run. So the wait needs no
/// assumption about the order of the queue.
#[test]
fn a_diff_whose_caller_drops_the_future_while_queued_never_runs_and_frees_the_permit() {
    one_blocking_thread().block_on(async {
        let blocking = Blocking::default();
        let (gate, permit) = one_permit().await;
        let (release, released) = std::sync::mpsc::channel::<()>();
        let busy = tokio::task::spawn_blocking(move || {
            released.recv_timeout(Duration::from_secs(5)).is_ok()
        });
        let ran = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&ran);
        let diff = blocking.diff(deadline_of(&blocking), permit, move || {
            flag.store(true, Ordering::SeqCst)
        });

        let finished = tokio::select! {
            biased;
            _ = diff => true,
            () = std::future::ready(()) => false,
        };
        release.send(()).unwrap();
        assert!(busy.await.unwrap());
        let next = gate.acquire(deadline_of(&blocking)).await.unwrap();

        assert!(!finished);
        assert!(!ran.load(Ordering::SeqCst));
        assert_eq!(gate.available(), 0);
        drop(next);
        assert_eq!(gate.available(), 1);
    });
}

#[tokio::test]
async fn a_diff_after_its_deadline_does_not_start_the_work() {
    let blocking = Blocking::default();
    let (gate, permit) = one_permit().await;
    let ran = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&ran);
    let passed = tokio::time::Instant::now();

    let error = blocking
        .diff(passed, permit, move || flag.store(true, Ordering::SeqCst))
        .await
        .unwrap_err();
    tokio::time::sleep(Duration::from_millis(50)).await;

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(!ran.load(Ordering::SeqCst));
    assert_eq!(gate.available(), 1);
}

#[tokio::test]
async fn a_slow_diff_gives_the_diff_timeout() {
    let blocking = Blocking::new(SHORT, Duration::from_secs(10));
    let (_gate, permit) = one_permit().await;
    let error = blocking
        .diff(deadline_of(&blocking), permit, || std::thread::sleep(LONG))
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
}

#[tokio::test]
async fn a_panicking_diff_gives_task_failed_with_the_diff_stage() {
    let blocking = Blocking::default();
    let (gate, permit) = one_permit().await;
    let error = blocking
        .diff(deadline_of(&blocking), permit, || {
            std::hint::black_box(Option::<u8>::None).unwrap()
        })
        .await
        .unwrap_err();
    assert_eq!(error.code(), ErrorCode::WEB_011);
    assert!(matches!(
        error,
        AppError::TaskFailed {
            stage: TaskStage::Diff
        }
    ));
    assert_eq!(gate.available(), 1);
}

#[tokio::test]
async fn the_doubles_give_the_diff_stage_without_running_the_work() {
    let ran = Arc::new(AtomicBool::new(false));
    let elapsed = Blocking::elapsed();
    let flag = Arc::clone(&ran);
    let (_gate, permit) = one_permit().await;
    let timeout = elapsed
        .diff(deadline_of(&elapsed), permit, move || {
            flag.store(true, Ordering::SeqCst)
        })
        .await
        .unwrap_err();
    let panicked = Blocking::panicked();
    let flag = Arc::clone(&ran);
    let (_gate, permit) = one_permit().await;
    let failed = panicked
        .diff(deadline_of(&panicked), permit, move || {
            flag.store(true, Ordering::SeqCst)
        })
        .await
        .unwrap_err();

    assert!(matches!(
        timeout,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    assert!(matches!(
        failed,
        AppError::TaskFailed {
            stage: TaskStage::Diff
        }
    ));
    assert!(!ran.load(Ordering::SeqCst));
}

#[tokio::test]
async fn a_busy_gate_gives_the_diff_timeout_after_the_deadline() {
    let blocking = Blocking::new(SHORT, Duration::from_secs(10));
    let gate = DiffGate::new(1);
    let held = gate
        .acquire(deadline_of(&Blocking::default()))
        .await
        .unwrap();

    let error = gate.acquire(deadline_of(&blocking)).await.unwrap_err();

    assert_eq!(error.code(), ErrorCode::WEB_009);
    assert!(matches!(
        error,
        AppError::Timeout {
            stage: TimeoutStage::Diff
        }
    ));
    drop(held);
}

#[tokio::test]
async fn a_permit_returns_to_the_gate_when_its_holder_drops_it() {
    let blocking = Blocking::default();
    let gate = DiffGate::new(2);

    let permit = gate.acquire(deadline_of(&blocking)).await.unwrap();
    assert_eq!(gate.available(), 1);
    drop(permit);

    assert_eq!(gate.available(), 2);
}

#[test]
fn a_gate_above_the_largest_count_keeps_the_largest_count() {
    let gate = DiffGate::new(usize::MAX);
    assert!(gate.available() > 0);
}
