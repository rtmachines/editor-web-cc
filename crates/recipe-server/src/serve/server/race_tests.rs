//! The race between the stop signal and the end of an accept loop (ADR-0125).

use std::future::pending;
use std::time::Duration;

use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;

use super::{ListenerHandle, StopCause, race_stop};
use crate::listener::DrainOutcome;
use crate::serve::host::{ListenerKind, Signal};

/// The time limit of one race. A race that waits for a signal that never fires fails at it.
const RACE_LIMIT: Duration = Duration::from_secs(5);

/// A handle whose task never ends.
fn running(kind: ListenerKind) -> ListenerHandle {
    let task: JoinHandle<DrainOutcome> = tokio::spawn(pending());
    ListenerHandle::new(kind, CancellationToken::new(), task)
}

/// A handle whose task panics at once.
fn panicking(kind: ListenerKind) -> ListenerHandle {
    let task = tokio::spawn(async { std::hint::black_box(None::<DrainOutcome>).unwrap() });
    ListenerHandle::new(kind, CancellationToken::new(), task)
}

/// A signal that never fires.
fn silent() -> Signal {
    Box::pin(pending())
}

/// A signal that is ready at once.
fn ready() -> Signal {
    Box::pin(async {})
}

#[tokio::test]
async fn a_panicked_internal_loop_ends_the_race_without_a_signal() {
    let mut public = running(ListenerKind::Public);
    let mut internal = panicking(ListenerKind::Internal);
    let cause = tokio::time::timeout(RACE_LIMIT, race_stop(silent(), &mut public, &mut internal))
        .await
        .expect("the race ignored the end of the internal loop");
    assert_eq!(
        cause,
        StopCause::LoopEnded {
            kind: ListenerKind::Internal,
            panicked: true
        }
    );
}

/// The number of races of the test of the signal order. Without `biased`, `select!` starts at a
/// random branch, so one race finds a missing `biased` in about half of the runs. 32 races miss
/// it with a probability of 2 to the power of -32.
const ORDER_RACES: usize = 32;

/// Wait until the task of a handle has ended, so the next poll of the handle is ready at once.
async fn wait_until_finished(handle: &ListenerHandle) {
    tokio::time::timeout(RACE_LIMIT, async {
        while !handle.task.is_finished() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the task of the loop did not end");
}

#[tokio::test]
async fn a_ready_signal_wins_against_a_finished_loop() {
    for _ in 0..ORDER_RACES {
        let mut public = running(ListenerKind::Public);
        let mut internal = panicking(ListenerKind::Internal);
        wait_until_finished(&internal).await;
        let cause =
            tokio::time::timeout(RACE_LIMIT, race_stop(ready(), &mut public, &mut internal))
                .await
                .expect("the race did not end");
        assert_eq!(cause, StopCause::Signal);
    }
}

#[tokio::test]
async fn the_kept_join_result_is_read_once_and_the_task_is_not_awaited_again() {
    let mut public = running(ListenerKind::Public);
    let mut internal = panicking(ListenerKind::Internal);
    let _cause = tokio::time::timeout(RACE_LIMIT, race_stop(silent(), &mut public, &mut internal))
        .await
        .expect("the race ignored the end of the internal loop");
    let error = tokio::time::timeout(RACE_LIMIT, internal.join())
        .await
        .expect("the join awaited the finished task again")
        .expect_err("the kept result is the panic");
    assert!(error.is_panic());
}
