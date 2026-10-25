//! The accept loop of both listeners (ADR-0058).
//!
//! `axum::serve` sets no header read timeout and starts each connection as a detached task. This
//! loop replaces it for the public listener and for the internal listener. Both use the same
//! loop and the same header read timeout, so a neighbor container cannot stall the internal port
//! without a bound (ADR-0058 (c)). Each listener has its own drain deadline (ADR-0087 (c)).
//!
//! The loop owns every connection task in one `JoinSet`. It collects a finished task with
//! `join_next` in its `select!`, so the memory of a 24/7 process stays bounded (ADR-0058 (e)). A
//! panic of a connection task ends only that connection. The join of the `JoinSet` catches the
//! panic, which makes the `JoinSet` one of the places of `reliability.md` section 2.4 rule 6
//! (ADR-0058 (f)).
//!
//! After the drain deadline, the loop aborts the open connection tasks with `abort_all`. It does
//! not wait for them, because a task that does not yield would block the stop without a bound.
//! The pool close of the caller waits for the connections of the aborted tasks (ADR-0087 (d)).
//!
//! The loop puts `ConnectInfo<SocketAddr>` into each request, because the layer `client_address`
//! of `recipe-web` gives `E-WEB-010` without it. The loop reads neither the configuration nor the
//! environment. The caller passes the settings.

use std::future::Future;
use std::io::{self, ErrorKind};
use std::net::SocketAddr;
#[cfg(test)]
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use axum::Router;
use axum::extract::ConnectInfo;
use hyper::Request;
use hyper::body::Incoming;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::{TokioIo, TokioTimer};
use hyper_util::server::graceful::GracefulShutdown;
use tokio::net::{TcpListener, TcpStream};
use tokio::task::{JoinError, JoinSet};
use tokio_util::sync::CancellationToken;
use tower::Service;

/// The settings of one accept loop.
///
/// The caller fills every field from `reliability.md` section 7.11. The type has no default,
/// because a default in code must equal that table and the caller owns the configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ListenerSettings {
    /// The time that a client has to send a complete request header (`http.body_read_timeout`).
    /// An idle keep-alive connection closes after the same time (ADR-0058 (k)).
    pub header_read_timeout: Duration,
    /// The time that open connections have to finish after the stop (`shutdown.drain`).
    pub drain_deadline: Duration,
    /// The pause after an accept fault that is not a fault of one connection.
    pub accept_error_pause: Duration,
}

/// The result of the drain after a stop.
///
/// The type solves one problem: the caller writes the `WARN` line of the stop (design D4), so it
/// needs the count of the cut connections and not a log line of the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DrainOutcome {
    cut: usize,
}

impl DrainOutcome {
    /// The number of connections that the loop aborted at the end of the drain.
    #[must_use]
    pub const fn cut(self) -> usize {
        self.cut
    }

    /// Whether every connection finished before the deadline.
    #[must_use]
    pub const fn is_complete(self) -> bool {
        self.cut == 0
    }
}

/// The number of connection tasks that the loop holds.
///
/// A task leaves the count when the loop collects it. The count therefore shows whether the loop
/// leaks finished tasks. A clone reads the same count. Only test builds have this counter, so
/// the production build holds no interior mutability here (`reliability.md` section 8).
#[cfg(test)]
#[derive(Debug, Clone, Default)]
struct OpenConnections(Arc<AtomicUsize>);

#[cfg(test)]
impl OpenConnections {
    /// The number of connection tasks at the last change of the loop.
    fn count(&self) -> usize {
        self.0.load(Ordering::Relaxed)
    }

    fn record(&self, count: usize) {
        self.0.store(count, Ordering::Relaxed);
    }
}

/// The source of the connections of one loop.
///
/// The trait solves one problem: a test must inject an accept fault, and the operating system does
/// not give a portable way to make a real `accept` fail.
trait Acceptor: Send + 'static {
    fn accept(&self) -> impl Future<Output = io::Result<(TcpStream, SocketAddr)>> + Send;
}

impl Acceptor for TcpListener {
    async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        TcpListener::accept(self).await
    }
}

/// An accept error that comes from one connection and not from the listener.
fn is_connection_fault(kind: ErrorKind) -> bool {
    matches!(
        kind,
        ErrorKind::ConnectionAborted | ErrorKind::ConnectionReset | ErrorKind::ConnectionRefused
    )
}

/// The accept loop of one listener.
#[derive(Debug)]
pub struct AcceptLoop {
    router: Router,
    settings: ListenerSettings,
    stop: CancellationToken,
    #[cfg(test)]
    open: OpenConnections,
}

impl AcceptLoop {
    /// Prepare a loop that serves `router`. The loop stops accepting when `stop` is cancelled.
    #[must_use]
    pub fn new(router: Router, settings: ListenerSettings, stop: CancellationToken) -> Self {
        Self {
            router,
            settings,
            stop,
            #[cfg(test)]
            open: OpenConnections::default(),
        }
    }

    /// A handle that reads the number of connection tasks of this loop.
    #[cfg(test)]
    fn open_connections(&self) -> OpenConnections {
        self.open.clone()
    }

    /// Serve the listener until the stop. Then close the listener and drain the connections.
    ///
    /// The drain waits at most `ListenerSettings::drain_deadline`. The loop aborts the
    /// connections that stay open and returns their count. It does not wait for an aborted task.
    /// The pool close of the caller waits for the database connections of these tasks.
    pub async fn run(self, listener: TcpListener) -> DrainOutcome {
        self.run_with(listener).await
    }

    async fn run_with<A: Acceptor>(self, acceptor: A) -> DrainOutcome {
        let mut connections = Connections::new(&self);
        loop {
            tokio::select! {
                biased;
                () = self.stop.cancelled() => break,
                Some(joined) = connections.tasks.join_next() => connections.collect(joined),
                accepted = acceptor.accept() => {
                    if self.take(accepted, &mut connections).await {
                        break;
                    }
                }
            }
        }
        drop(acceptor);
        connections.drain(self.settings.drain_deadline).await
    }

    /// Serve an accepted connection or handle the accept error.
    ///
    /// Return `true` when the stop came during the pause after an error.
    async fn take(
        &self,
        accepted: io::Result<(TcpStream, SocketAddr)>,
        connections: &mut Connections,
    ) -> bool {
        let error = match accepted {
            Ok((stream, peer)) => {
                connections.spawn(stream, peer);
                return false;
            }
            Err(error) => error,
        };
        if is_connection_fault(error.kind()) {
            tracing::debug!(kind = ?error.kind(), "a connection failed before accept");
            return false;
        }
        tracing::warn!(kind = ?error.kind(), "accept failed, the loop pauses and goes on");
        tokio::select! {
            () = self.stop.cancelled() => true,
            () = tokio::time::sleep(self.settings.accept_error_pause) => false,
        }
    }
}

/// The connection tasks of one loop and what they need.
struct Connections {
    tasks: JoinSet<()>,
    graceful: GracefulShutdown,
    router: Router,
    header_read_timeout: Duration,
    #[cfg(test)]
    open: OpenConnections,
}

impl Connections {
    fn new(accept_loop: &AcceptLoop) -> Self {
        Self {
            tasks: JoinSet::new(),
            graceful: GracefulShutdown::new(),
            router: accept_loop.router.clone(),
            header_read_timeout: accept_loop.settings.header_read_timeout,
            #[cfg(test)]
            open: accept_loop.open.clone(),
        }
    }

    /// Serve one connection in its own task. The task is watched for the graceful stop.
    fn spawn(&mut self, stream: TcpStream, peer: SocketAddr) {
        let router = self.router.clone();
        // `Router` is always ready, so the call needs no `poll_ready` (axum docs do the same).
        let service = service_fn(move |mut request: Request<Incoming>| {
            request.extensions_mut().insert(ConnectInfo(peer));
            let mut router = router.clone();
            async move { router.call(request).await }
        });
        let connection = http1::Builder::new()
            .timer(TokioTimer::new())
            .header_read_timeout(self.header_read_timeout)
            .serve_connection(TokioIo::new(stream), service);
        let watched = self.graceful.watch(connection);
        self.tasks.spawn(async move {
            if let Err(error) = watched.await {
                tracing::debug!(%error, "a connection ended with an error");
            }
        });
        #[cfg(test)]
        self.open.record(self.tasks.len());
    }

    /// Take one finished task out of the set. Log a panic once and go on.
    fn collect(&self, joined: Result<(), JoinError>) {
        if let Err(error) = joined
            && error.is_panic()
        {
            tracing::error!("a connection task panicked, the loop goes on");
        }
        #[cfg(test)]
        self.open.record(self.tasks.len());
    }

    /// Wait for the open connections up to the deadline, then abort the rest.
    ///
    /// One deadline covers the graceful stop and the collection of the tasks. A task that blocks
    /// after its connection ended therefore cannot exceed the deadline. The abort waits for no
    /// task, because a task that does not yield would keep the wait open without a bound. The
    /// pool close of the caller waits for the connections of the aborted tasks (ADR-0087 (d)).
    async fn drain(mut self, deadline: Duration) -> DrainOutcome {
        let graceful = std::mem::take(&mut self.graceful);
        let finished = tokio::time::timeout(deadline, async {
            graceful.shutdown().await;
            while let Some(joined) = self.tasks.join_next().await {
                self.collect(joined);
            }
        })
        .await
        .is_ok();
        if finished {
            return DrainOutcome { cut: 0 };
        }
        while let Some(joined) = self.tasks.try_join_next() {
            self.collect(joined);
        }
        let cut = self.tasks.len();
        self.tasks.abort_all();
        #[cfg(test)]
        self.open.record(0);
        DrainOutcome { cut }
    }
}

#[cfg(test)]
mod tests;
