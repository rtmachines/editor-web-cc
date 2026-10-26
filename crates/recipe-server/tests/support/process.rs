//! The built binary `recipe-server serve` as a child process that the test controls.
//!
//! `serve` runs until a signal stops it. The helper therefore reads the log on two threads and
//! bounds every wait. A hung child gets killed, so a regression fails the test and does not hang
//! it. The drop kills a child that still runs.
//!
//! A reader thread sends each line through a channel. The helper owns the receivers and moves the
//! received lines into its own buffers, so no lock is shared with a thread.

use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::environment::remove_foreign_variables;
use super::net::{free_ports, http_get, poll_until};
use super::support::cli::level_marker;

/// The pause between two checks of a child that may have exited.
const EXIT_POLL_PAUSE: Duration = Duration::from_millis(50);
/// The text that the `ERROR` line holds when `serve` cannot bind one of its two ports.
const BIND_FAULT: &str = "listener cannot bind its port";

/// The lines of one pipe. The reader thread sends them, and the owner keeps what it received.
struct Pipe {
    receiver: Receiver<String>,
    lines: Vec<String>,
}

impl Pipe {
    /// Move the lines that the reader sent since the last call into the buffer.
    fn drain(&mut self) -> &[String] {
        self.lines.extend(self.receiver.try_iter());
        &self.lines
    }

    /// The lines as bytes, one line end after each line.
    fn drain_bytes(&mut self) -> Vec<u8> {
        let mut joined = self.drain().join("\n");
        joined.push('\n');
        joined.into_bytes()
    }
}

/// A running `recipe-server serve`.
///
/// The `Drop` impl kills the child. When the test process itself gets `SIGKILL`, `Drop` does not
/// run, and the `serve` child keeps running. The project accepts this for test code.
pub struct ServerProcess {
    child: Child,
    stdout: Pipe,
    stderr: Pipe,
    readers: Vec<JoinHandle<()>>,
    public_port: u16,
    internal_port: u16,
}

impl ServerProcess {
    /// Start `recipe-server serve` with `environment` and two ports that are free just now.
    ///
    /// The ports are the keys `http.public_port` and `http.internal_port`. A port may be taken
    /// between the choice and the bind. Then the child exits with an `ERROR` line about the bind.
    /// The function waits at most `limit` for the start or the exit and starts the child once
    /// more with two new ports.
    pub fn spawn_on_free_ports(environment: &[(&str, String)], limit: Duration) -> Self {
        let (public_port, internal_port) = free_ports();
        let mut server = Self::spawn_on_ports(environment, public_port, internal_port);
        if server.failed_to_bind(limit) {
            let (public_port, internal_port) = free_ports();
            server = Self::spawn_on_ports(environment, public_port, internal_port);
        }
        server
    }

    /// Start `recipe-server serve` with `environment` and the two ports `public_port` and
    /// `internal_port`. The function does not retry.
    ///
    /// A restart that must record no setting change needs the same ports as the start before.
    pub fn spawn_on_ports(
        environment: &[(&str, String)],
        public_port: u16,
        internal_port: u16,
    ) -> Self {
        let mut with_ports = environment.to_vec();
        with_ports.push(("RECIPE_HTTP__PUBLIC_PORT", public_port.to_string()));
        with_ports.push(("RECIPE_HTTP__INTERNAL_PORT", internal_port.to_string()));
        Self::spawn(&with_ports, public_port, internal_port)
    }

    /// The port of the public listener.
    pub fn public_port(&self) -> u16 {
        self.public_port
    }

    /// The port of the internal listener.
    pub fn internal_port(&self) -> u16 {
        self.internal_port
    }

    /// Wait until the child is ready or exited. Tell whether it exited for a bind fault.
    fn failed_to_bind(&mut self, limit: Duration) -> bool {
        let internal_port = self.internal_port;
        let _ = poll_until(limit, || {
            let is_ready =
                http_get(internal_port, "/readyz").is_some_and(|answer| answer.status == 200);
            (is_ready || self.exited()).then_some(())
        });
        if !self.exited() {
            return false;
        }
        self.join_readers();
        has_line(self.stdout.drain(), "ERROR", BIND_FAULT)
            || has_line(self.stderr.drain(), "ERROR", BIND_FAULT)
    }

    /// Start `recipe-server serve` with `environment` and nothing else of the variables
    /// `PG*`, `RECIPE_*`, and `RUST_LOG`.
    fn spawn(environment: &[(&str, String)], public_port: u16, internal_port: u16) -> Self {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
        command.arg("serve").env_remove("RUST_LOG");
        remove_foreign_variables(&mut command);
        for (name, value) in environment {
            command.env(name, value);
        }
        let mut child = command
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let (stdout, stdout_reader) = collect(child.stdout.take().unwrap());
        let (stderr, stderr_reader) = collect(child.stderr.take().unwrap());
        Self {
            child,
            stdout,
            stderr,
            readers: vec![stdout_reader, stderr_reader],
            public_port,
            internal_port,
        }
    }

    /// The process identifier, for the signal.
    #[cfg(unix)]
    pub fn id(&self) -> u32 {
        self.child.id()
    }

    /// The lines that the child wrote to its standard output so far.
    pub fn stdout_lines(&mut self) -> &[String] {
        self.stdout.drain()
    }

    /// Both logs as one text, for the message of a failed assertion.
    pub fn log(&mut self) -> String {
        let stdout = self.stdout.drain().join("\n");
        let stderr = self.stderr.drain().join("\n");
        format!("{stdout}\n{stderr}")
    }

    /// Tell whether the child has exited.
    pub fn exited(&mut self) -> bool {
        self.child.try_wait().unwrap().is_some()
    }

    /// Wait for the exit of the child, at most for `limit`, and return its output.
    ///
    /// When the limit ends, the function kills the child and fails the test.
    pub fn wait_exit(&mut self, limit: Duration) -> Output {
        let start = Instant::now();
        let mut status: Option<ExitStatus> = None;
        while status.is_none() && start.elapsed() < limit {
            status = self.child.try_wait().unwrap();
            if status.is_none() {
                std::thread::sleep(EXIT_POLL_PAUSE);
            }
        }
        if status.is_none() {
            self.kill();
        }
        assert!(
            status.is_some(),
            "the server did not exit within {limit:?}:\n{}",
            self.log()
        );
        self.join_readers();
        Output {
            status: status.unwrap(),
            stdout: self.stdout.drain_bytes(),
            stderr: self.stderr.drain_bytes(),
        }
    }

    /// Wait for the reader threads. They end when the child has exited and its pipes are closed.
    fn join_readers(&mut self) {
        for reader in self.readers.drain(..) {
            reader.join().unwrap();
        }
    }

    /// Kill the child and wait for it. The call does nothing for a child that already exited.
    pub fn kill(&mut self) {
        // The child may have exited already. Then `kill` returns `Ok(())` and `wait` returns the
        // saved exit status. Both results are ignored here.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Drop for ServerProcess {
    fn drop(&mut self) {
        self.kill();
    }
}

/// Tell whether one line has the level `level` and holds `message`.
pub fn has_line(lines: &[String], level: &str, message: &str) -> bool {
    let marker = level_marker(level);
    lines
        .iter()
        .any(|line| line.contains(&marker) && line.contains(message))
}

/// Read the lines of `pipe` on a new thread and send them. The thread ends when the pipe closes.
fn collect(pipe: impl Read + Send + 'static) -> (Pipe, JoinHandle<()>) {
    let (sender, receiver): (Sender<String>, Receiver<String>) = channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(pipe).lines().map_while(Result::ok) {
            if sender.send(line).is_err() {
                break;
            }
        }
    });
    let lines = Vec::new();
    (Pipe { receiver, lines }, reader)
}
