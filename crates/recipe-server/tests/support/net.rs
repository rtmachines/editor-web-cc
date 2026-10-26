//! Free ports, polling, and a small HTTP client for the tests that start `serve`.
//!
//! The tests have no HTTP client crate. The answers of the health routes are short, so one
//! request on a plain socket is enough.

use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::time::{Duration, Instant};

/// The time that one connect, one write, or one read may take.
const IO_LIMIT: Duration = Duration::from_secs(2);
/// The pause between two polls.
const POLL_PAUSE: Duration = Duration::from_millis(100);

/// An answer of the server: the status code and the body.
pub struct Answer {
    pub status: u16,
    pub body: String,
}

/// Two listeners that the test keeps open, for the keys `http.public_port` and
/// `http.internal_port`.
///
/// While a `HeldPorts` lives, no other process or thread can bind its two ports. A child that
/// tries to bind them fails with an address-in-use error. The ports stay taken until the drop.
pub struct HeldPorts {
    public: TcpListener,
    internal: TcpListener,
}

impl HeldPorts {
    /// The port of the first listener.
    pub fn public_port(&self) -> u16 {
        self.public.local_addr().unwrap().port()
    }

    /// The port of the second listener.
    pub fn internal_port(&self) -> u16 {
        self.internal.local_addr().unwrap().port()
    }
}

/// Bind two listeners on port 0 on all addresses, as `serve` does, and keep them open.
///
/// The function holds both at the same time, so the two ports differ.
pub fn hold_ports() -> HeldPorts {
    HeldPorts {
        public: TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap(),
        internal: TcpListener::bind((Ipv4Addr::UNSPECIFIED, 0)).unwrap(),
    }
}

/// Find two different ports that nothing holds just now, for the keys `http.public_port` and
/// `http.internal_port`.
///
/// The function takes the ports of `hold_ports` and frees the listeners again.
///
/// The caller must use the ports at once. Another process may take a freed port at any time.
/// A test that needs a port that stays closed must use `hold_ports` and not this function.
pub fn free_ports() -> (u16, u16) {
    let held = hold_ports();
    (held.public_port(), held.internal_port())
}

/// Call `probe` until it returns a value, at most for `limit`. Return `None` after the limit.
pub fn poll_until<T>(limit: Duration, mut probe: impl FnMut() -> Option<T>) -> Option<T> {
    let start = Instant::now();
    loop {
        if let Some(value) = probe() {
            return Some(value);
        }
        if start.elapsed() >= limit {
            return None;
        }
        std::thread::sleep(POLL_PAUSE);
    }
}

/// Return true when a TCP connection to `port` on the loopback address succeeds.
pub fn port_is_open(port: u16) -> bool {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    TcpStream::connect_timeout(&address, IO_LIMIT).is_ok()
}

/// Send `GET path` to `port` on the loopback address and read the whole answer.
///
/// Returns `None` when the connection, the write, or the read fails or times out.
pub fn http_get(port: u16, path: &str) -> Option<Answer> {
    let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
    let mut stream = TcpStream::connect_timeout(&address, IO_LIMIT).ok()?;
    stream.set_read_timeout(Some(IO_LIMIT)).ok()?;
    stream.set_write_timeout(Some(IO_LIMIT)).ok()?;
    let request = format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
    stream.write_all(request.as_bytes()).ok()?;
    let mut text = String::new();
    stream.read_to_string(&mut text).ok()?;
    let (head, body) = text.split_once("\r\n\r\n")?;
    let status = head
        .lines()
        .next()?
        .split_whitespace()
        .nth(1)?
        .parse()
        .ok()?;
    Some(Answer {
        status,
        body: body.to_owned(),
    })
}
