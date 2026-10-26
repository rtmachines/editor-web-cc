//! The routing of the three cut counts of the stop (ADR-0088 (e), design D4).
//!
//! The test keeps open work in each of the three places: three public connections, two internal
//! connections, and one job run. None of them ends. The stop must cut every one and put each count
//! into its own field of the report. The three counts differ, so a swap of two routes fails the
//! test. The times are short and real, because the loops need real sockets.

use std::io::Write;
use std::net::{Ipv4Addr, SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use axum::routing::get;
use recipe_app::error::AppError;
use recipe_app::jobs::RunKind;
use recipe_domain::audit::RequestId;
use tokio::net::TcpListener;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::tests::IdleWatchdog;
use super::{Opened, Resources};
use crate::listener::{AcceptLoop, ListenerSettings};
use crate::schedule::Schedule;
use crate::serve::host::ListenerKind;
use crate::serve::plan::ShutdownSettings;
use crate::serve::server::ListenerHandle;
use crate::serve::test_support::{LogCapture, START_TEXT, TokioClock};
use crate::supervisor::{JobName, JobRun, Supervisor, SupervisorSettings};

/// The longest wait of the test for the open work.
const ENTER_LIMIT: Duration = Duration::from_secs(10);
/// The header read timeout of the loops. It is longer than every drain of the test.
const HEADER_TIMEOUT: Duration = Duration::from_secs(30);
/// The pause after an accept fault. The test makes none.
const ACCEPT_PAUSE: Duration = Duration::from_millis(10);
/// The restart delay of the supervisor. The job of the test never panics.
const RESTART_DELAY: Duration = Duration::from_secs(10);
/// The run interval of the job. The first run starts at once.
const JOB_INTERVAL: Duration = Duration::from_secs(60);

/// The open work of the test, by place. The three counts differ on purpose.
const PUBLIC_CONNECTIONS: usize = 3;
const INTERNAL_CONNECTIONS: usize = 2;
const JOB_RUNS: usize = 1;

/// A job run that reports its start and never ends.
struct HangingJob {
    entered: Arc<Semaphore>,
}

impl JobRun for HangingJob {
    async fn run(&self, _request_id: RequestId, _kind: RunKind) -> Result<(), AppError> {
        self.entered.add_permits(1);
        std::future::pending().await
    }
}

/// A router whose handler reports its start and never answers.
fn hanging_router(entered: &Arc<Semaphore>) -> Router {
    let entered = Arc::clone(entered);
    Router::new().route(
        "/hang",
        get(move || {
            let entered = Arc::clone(&entered);
            async move {
                entered.add_permits(1);
                std::future::pending::<&'static str>().await
            }
        }),
    )
}

/// Open an accept loop on a free port and wrap its task in a handle.
async fn open_listener(
    kind: ListenerKind,
    entered: &Arc<Semaphore>,
    drain_deadline: Duration,
) -> (ListenerHandle, SocketAddr) {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).await.unwrap();
    let address = listener.local_addr().unwrap();
    let stop = CancellationToken::new();
    let settings = ListenerSettings {
        header_read_timeout: HEADER_TIMEOUT,
        drain_deadline,
        accept_error_pause: ACCEPT_PAUSE,
    };
    let accept_loop = AcceptLoop::new(hanging_router(entered), settings, stop.clone());
    let task = tokio::spawn(accept_loop.run(listener));
    (ListenerHandle::new(kind, stop, task), address)
}

/// Send a request that no handler answers. The returned stream keeps the connection open.
fn send_hanging_request(address: SocketAddr) -> TcpStream {
    let mut stream = TcpStream::connect(address).unwrap();
    write!(stream, "GET /hang HTTP/1.1\r\nHost: localhost\r\n\r\n").unwrap();
    stream
}

/// Wait until the handlers and the job run have reported `count` starts.
fn wait_for_starts(runtime: &Runtime, entered: &Semaphore, count: usize) {
    let count = u32::try_from(count).unwrap();
    runtime.block_on(async {
        let permits = tokio::time::timeout(ENTER_LIMIT, entered.acquire_many(count))
            .await
            .expect("the open work did not start")
            .unwrap();
        permits.forget();
    });
}

/// The times of the stop in this test. They are short and real.
fn short_shutdown() -> ShutdownSettings {
    ShutdownSettings {
        drain: Duration::from_millis(300),
        pool_close: Duration::from_millis(100),
        internal_drain: Duration::ZERO,
        runtime_stop: Duration::from_millis(100),
    }
}

/// Open both listeners and the supervisor with its hanging job. Return the two addresses.
async fn open_resources(
    entered: &Arc<Semaphore>,
    shutdown: ShutdownSettings,
) -> (Resources, [SocketAddr; 2]) {
    let (public, public_address) =
        open_listener(ListenerKind::Public, entered, shutdown.drain).await;
    let (internal, internal_address) =
        open_listener(ListenerKind::Internal, entered, shutdown.internal_drain).await;
    let settings = SupervisorSettings {
        restart_delay: RESTART_DELAY,
    };
    let mut supervisor = Supervisor::new(
        settings,
        TokioClock::at(START_TEXT),
        CancellationToken::new(),
    );
    let job = HangingJob {
        entered: Arc::clone(entered),
    };
    let schedule = Schedule::now_then_every(JOB_INTERVAL).unwrap();
    supervisor.spawn(JobName::Alerts, schedule, job);
    let resources = Resources {
        supervisor: Some(supervisor),
        internal: Some(internal),
        public: Some(public),
        ..Resources::default()
    };
    (resources, [public_address, internal_address])
}

#[test]
fn each_count_of_the_cut_goes_into_its_own_field_and_the_cut_line_shows_them() {
    let logs = LogCapture::start();
    let runtime = Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let shutdown = short_shutdown();
    let entered = Arc::new(Semaphore::new(0));
    let (resources, [public_address, internal_address]) =
        runtime.block_on(open_resources(&entered, shutdown));
    let mut clients: Vec<TcpStream> = (0..PUBLIC_CONNECTIONS)
        .map(|_| send_hanging_request(public_address))
        .collect();
    clients.extend((0..INTERNAL_CONNECTIONS).map(|_| send_hanging_request(internal_address)));
    let starts = PUBLIC_CONNECTIONS + INTERNAL_CONNECTIONS + JOB_RUNS;
    wait_for_starts(&runtime, &entered, starts);

    let opened = Opened {
        watchdog: Box::new(IdleWatchdog),
        runtime,
        resources,
        shutdown,
    };
    let report = opened.close();

    assert_eq!(report.cut_public, PUBLIC_CONNECTIONS, "{report:?}");
    assert_eq!(report.cut_internal, INTERNAL_CONNECTIONS, "{report:?}");
    assert_eq!(report.cut_jobs, JOB_RUNS, "{report:?}");
    drop(clients);
    let _start_lines = logs.lines();
    report.log();
    let warnings = logs.at_level("WARN");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    for text in ["\"cut_public\":3", "\"cut_internal\":2", "\"cut_jobs\":1"] {
        assert!(warnings[0].contains(text), "{text} in {warnings:?}");
    }
}
