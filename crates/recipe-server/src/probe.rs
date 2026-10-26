//! The health probe of `recipe-server` (ADR-0059).
//!
//! The subcommand `healthcheck` and the watchdog thread (`reliability.md` section 2.4 rule 5)
//! share this probe. Docker runs the subcommand in an image without a shell, and the watchdog
//! runs outside the Tokio runtime. The probe therefore uses the standard library only. It needs
//! no runtime, no secret, and no database.
//!
//! The probe sends one `GET` request with `Connection: close` to `127.0.0.1`. It reads the status
//! line and nothing more. The status line `HTTP/1.1 200` is the only success.
//!
//! The whole probe stays inside one budget (`watchdog.probe_timeout`). The connect, the write,
//! and each read get only the part of the budget that is left. A server that sends one byte at a
//! time therefore cannot keep the probe open past the budget.

use std::env::VarError;
use std::fmt;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::time::{Duration, Instant};

use anyhow::{Result, anyhow, bail};

use crate::port::parse_port;

/// The budget of one probe (`watchdog.probe_timeout`, `reliability.md` section 7.11).
///
/// The value is above `health.db_check_timeout`, so a probe of `/readyz` waits for the
/// readiness check (ADR-0059 (e)).
pub const PROBE_BUDGET: Duration = Duration::from_secs(3);

pub use crate::port::DEFAULT_INTERNAL_PORT;

/// The environment variable that holds the port of the internal listener (ADR-0059 (c)).
const PORT_VARIABLE: &str = "RECIPE_HTTP__INTERNAL_PORT";

/// The most bytes that the probe reads. A status line must end inside this limit.
const MAX_STATUS_LINE: usize = 64;

/// The start of the only status line that counts as an answer of the server.
const STATUS_PREFIX: &[u8] = b"HTTP/1.1 ";

/// The number of digits of a status code.
const CODE_LENGTH: usize = 3;

/// The status code of success.
const STATUS_OK: u16 = 200;

/// The path of a probe, checked once so that it is safe inside a request line.
///
/// The type solves one problem: the path comes from the command line and goes into the text of
/// a request, so a space or a line break in it would change the request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbePath(String);

impl ProbePath {
    /// Check the text and keep it as a path.
    ///
    /// # Errors
    ///
    /// Returns an error when the text does not start with `/` or holds a byte that is not a
    /// visible ASCII character, or holds `#`.
    pub fn parse(text: &str) -> Result<Self> {
        let is_safe = text.starts_with('/')
            && text
                .bytes()
                .all(|byte| byte.is_ascii_graphic() && byte != b'#');
        if !is_safe {
            bail!("a path starts with / and holds visible ASCII characters and no #");
        }
        Ok(Self(text.to_owned()))
    }

    /// The path of the liveness endpoint, which the watchdog probes.
    #[must_use]
    pub fn liveness() -> Self {
        Self("/healthz".to_owned())
    }

    /// The path of the readiness endpoint, which the Docker healthcheck probes.
    #[must_use]
    pub fn readiness() -> Self {
        Self("/readyz".to_owned())
    }

    /// The path as text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// The reason why a probe failed.
///
/// The type has no `code()` and does not implement `std::error::Error`. It is a reason type like
/// `WebhookFault` (ADR-0025). The text of a fault names the cause and never holds text of the
/// server. `run_healthcheck` wraps the fault with `anyhow!`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeFault {
    /// The connection could not open. The kind comes from the operating system.
    Connect(ErrorKind),
    /// The request could not be sent.
    Write(ErrorKind),
    /// The answer could not be read.
    Read(ErrorKind),
    /// The budget ended before the status line arrived.
    Timeout,
    /// The server closed the connection before the status line ended.
    Closed,
    /// The status line did not end inside the read limit.
    LineTooLong,
    /// The first line is not a status line of HTTP/1.1.
    NotHttp,
    /// The server answered with a status other than 200.
    Status(u16),
}

impl fmt::Display for ProbeFault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(kind) => write!(formatter, "cannot connect to the listener: {kind}"),
            Self::Write(kind) => write!(formatter, "cannot send the request: {kind}"),
            Self::Read(kind) => write!(formatter, "cannot read the answer: {kind}"),
            Self::Timeout => formatter.write_str("the budget of the probe ended before the answer"),
            Self::Closed => {
                formatter.write_str("the server closed the connection before the status line")
            }
            Self::LineTooLong => write!(
                formatter,
                "the status line is longer than {MAX_STATUS_LINE} bytes"
            ),
            Self::NotHttp => formatter.write_str("the answer is not an HTTP/1.1 status line"),
            Self::Status(status) => {
                write!(formatter, "the server answered with the status {status}")
            }
        }
    }
}

/// The part of one budget that is left.
struct Deadline {
    started: Instant,
    budget: Duration,
}

impl Deadline {
    fn start(budget: Duration) -> Self {
        Self {
            started: Instant::now(),
            budget,
        }
    }

    /// The time that is left. A spent budget gives `Timeout`, because the standard library refuses
    /// a timeout of zero.
    fn remaining(&self) -> Result<Duration, ProbeFault> {
        self.budget
            .checked_sub(self.started.elapsed())
            .filter(|left| !left.is_zero())
            .ok_or(ProbeFault::Timeout)
    }
}

fn is_timeout(kind: ErrorKind) -> bool {
    matches!(kind, ErrorKind::TimedOut | ErrorKind::WouldBlock)
}

fn connect_fault(error: &io::Error) -> ProbeFault {
    if is_timeout(error.kind()) {
        ProbeFault::Timeout
    } else {
        ProbeFault::Connect(error.kind())
    }
}

fn write_fault(error: &io::Error) -> ProbeFault {
    if is_timeout(error.kind()) {
        ProbeFault::Timeout
    } else {
        ProbeFault::Write(error.kind())
    }
}

fn read_fault(error: &io::Error) -> ProbeFault {
    if is_timeout(error.kind()) {
        ProbeFault::Timeout
    } else {
        ProbeFault::Read(error.kind())
    }
}

/// Send `GET <path>` with `Connection: close` to the address.
///
/// The whole probe stays inside `budget`. The function returns `Ok` only for the status line
/// `HTTP/1.1 200`.
///
/// # Errors
///
/// Returns the `ProbeFault` that names the first step that failed.
pub fn probe(address: SocketAddr, path: &ProbePath, budget: Duration) -> Result<(), ProbeFault> {
    let deadline = Deadline::start(budget);
    let mut stream = TcpStream::connect_timeout(&address, deadline.remaining()?)
        .map_err(|error| connect_fault(&error))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n",
        path.as_str()
    );
    stream
        .set_write_timeout(Some(deadline.remaining()?))
        .map_err(|error| write_fault(&error))?;
    stream
        .write_all(request.as_bytes())
        .map_err(|error| write_fault(&error))?;
    let line = read_status_line(&mut stream, &deadline)?;
    check_status_line(&line)
}

/// Probe the internal listener on `127.0.0.1` with the budget `PROBE_BUDGET`.
///
/// # Errors
///
/// Returns the `ProbeFault` that names the first step that failed.
pub fn probe_local(port: u16, path: &ProbePath) -> Result<(), ProbeFault> {
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    probe(address, path, PROBE_BUDGET)
}

/// Read bytes until the first line feed and return the line without it.
///
/// Before each read, the function sets the read timeout to the rest of the budget.
///
/// A read that ends with a timeout does not end the probe while the budget has time left.
/// The operating system can end a read timeout a little early. The loop then reads again.
/// The next `Deadline::remaining` call gives `Timeout` when the budget is spent.
fn read_status_line(stream: &mut TcpStream, deadline: &Deadline) -> Result<Vec<u8>, ProbeFault> {
    let mut buffer = [0_u8; MAX_STATUS_LINE];
    let mut filled = 0_usize;
    loop {
        stream
            .set_read_timeout(Some(deadline.remaining()?))
            .map_err(|error| read_fault(&error))?;
        let free = buffer.get_mut(filled..).ok_or(ProbeFault::LineTooLong)?;
        let count = match stream.read(free) {
            Ok(0) => return Err(ProbeFault::Closed),
            Ok(count) => count,
            Err(error) if error.kind() == ErrorKind::Interrupted || is_timeout(error.kind()) => {
                continue;
            }
            Err(error) => return Err(ProbeFault::Read(error.kind())),
        };
        filled = filled.checked_add(count).ok_or(ProbeFault::LineTooLong)?;
        let received = buffer.get(..filled).ok_or(ProbeFault::LineTooLong)?;
        if let Some(end) = received.iter().position(|byte| *byte == b'\n') {
            return Ok(received.get(..end).unwrap_or_default().to_vec());
        }
        if filled >= MAX_STATUS_LINE {
            return Err(ProbeFault::LineTooLong);
        }
    }
}

/// Accept only `HTTP/1.1 200`, with a space and a reason phrase after the code or nothing.
fn check_status_line(line: &[u8]) -> Result<(), ProbeFault> {
    let line = line.strip_suffix(b"\r").unwrap_or(line);
    let rest = line
        .strip_prefix(STATUS_PREFIX)
        .ok_or(ProbeFault::NotHttp)?;
    let digits = rest.get(..CODE_LENGTH).ok_or(ProbeFault::NotHttp)?;
    let tail = rest.get(CODE_LENGTH..).ok_or(ProbeFault::NotHttp)?;
    if !(tail.is_empty() || tail.starts_with(b" ")) {
        return Err(ProbeFault::NotHttp);
    }
    let status = std::str::from_utf8(digits)
        .ok()
        .filter(|text| text.bytes().all(|byte| byte.is_ascii_digit()))
        .and_then(|text| text.parse::<u16>().ok())
        .ok_or(ProbeFault::NotHttp)?;
    if status == STATUS_OK {
        Ok(())
    } else {
        Err(ProbeFault::Status(status))
    }
}

/// Read the port from the text of `RECIPE_HTTP__INTERNAL_PORT`.
///
/// A missing variable gives `DEFAULT_INTERNAL_PORT`.
///
/// # Errors
///
/// Returns an error that names the variable when the text is not a port from 1 to 65535.
pub fn internal_port_from(value: Option<&str>) -> Result<u16> {
    let Some(text) = value else {
        return Ok(DEFAULT_INTERNAL_PORT);
    };
    parse_port(text).ok_or_else(|| anyhow!("{PORT_VARIABLE} is not a port number from 1 to 65535"))
}

/// Run the subcommand `healthcheck`: probe the internal listener once.
///
/// The function reads `RECIPE_HTTP__INTERNAL_PORT` and no other value (ADR-0059 (c)). It uses
/// the budget `PROBE_BUDGET`. `main` turns an error into exit code 1.
///
/// # Errors
///
/// Returns an error when the port variable is not valid or when the probe fails.
pub fn run_healthcheck(path: &ProbePath) -> Result<()> {
    let value = match std::env::var(PORT_VARIABLE) {
        Ok(text) => Some(text),
        Err(VarError::NotPresent) => None,
        Err(VarError::NotUnicode(_)) => bail!("{PORT_VARIABLE} is not valid text"),
    };
    let port = internal_port_from(value.as_deref())?;
    probe_local(port, path).map_err(|fault| anyhow!("the health probe failed: {fault}"))
}

#[cfg(test)]
mod tests;
