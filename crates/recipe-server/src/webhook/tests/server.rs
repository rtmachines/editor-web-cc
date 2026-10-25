//! A local HTTPS server for the webhook tests (design D14 row T27).
//!
//! The server uses `rustls` on standard threads, so the tests need no extra dependency. It counts
//! the TCP connections, records each request, and answers by a script.

use std::io::{ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ServerConfig, ServerConnection, StreamOwned};

use super::pki;

/// The longest wait of the server for one read. A stuck test cannot hang the process.
const IO_WAIT: Duration = Duration::from_secs(5);

/// The pause between two polls of the non-blocking `accept`.
const ACCEPT_POLL: Duration = Duration::from_millis(5);

/// The pause between two checks of the stop flag while the server stays silent.
const SILENCE_POLL: Duration = Duration::from_millis(10);

/// One request that the server received.
#[derive(Clone, Debug)]
pub(super) struct Seen {
    pub(super) method: String,
    pub(super) target: String,
    pub(super) content_type: Option<String>,
    pub(super) body: String,
}

/// What the server does with a request.
pub(super) enum Reply {
    /// Answer with the status and an empty body.
    Status(u16),
    /// Answer with `301` and the location.
    Redirect(String),
    /// Keep the connection open and answer nothing for the time.
    Silence(Duration),
}

/// The script of a server: one reply for each request.
pub(super) type Script = Arc<dyn Fn(&Seen) -> Reply + Send + Sync>;

/// A script that answers every request with one status.
pub(super) fn always(status: u16) -> Script {
    Arc::new(move |_seen| Reply::Status(status))
}

/// A script that stays silent for the time on every request.
pub(super) fn silent(time: Duration) -> Script {
    Arc::new(move |_seen| Reply::Silence(time))
}

#[derive(Default)]
struct Counters {
    accepted: AtomicUsize,
    active: AtomicUsize,
    peak: AtomicUsize,
}

/// A server on `127.0.0.1` with a random port. The drop stops it.
pub(super) struct TlsServer {
    address: SocketAddr,
    counters: Arc<Counters>,
    requests: Receiver<Seen>,
    stop: Arc<AtomicBool>,
    accept_thread: Option<JoinHandle<()>>,
}

impl TlsServer {
    /// Start a server with the leaf certificate for `127.0.0.1` (authority A).
    pub(super) fn start(script: Script) -> Self {
        Self::start_with(pki::LEAF_CERTIFICATE, pki::LEAF_KEY, script)
    }

    /// Start a server with the given certificate and key.
    pub(super) fn start_with(certificate: &str, key: &str, script: Script) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let counters = Arc::new(Counters::default());
        let stop = Arc::new(AtomicBool::new(false));
        let (sender, requests) = channel();
        let shared = Shared {
            config: server_config(certificate, key),
            script,
            counters: Arc::clone(&counters),
            sender,
            stop: Arc::clone(&stop),
        };
        let accept_thread = std::thread::spawn(move || accept_loop(&listener, &shared));
        Self {
            address,
            counters,
            requests,
            stop,
            accept_thread: Some(accept_thread),
        }
    }

    /// The address of the server.
    pub(super) const fn address(&self) -> SocketAddr {
        self.address
    }

    /// The URL of a path on this server, with the scheme `https`.
    pub(super) fn url(&self, path_and_query: &str) -> String {
        format!("https://{}{path_and_query}", self.address)
    }

    /// The number of TCP connections that the server accepted.
    pub(super) fn accepted(&self) -> usize {
        self.counters.accepted.load(Ordering::SeqCst)
    }

    /// The most requests that were open at the same time.
    pub(super) fn peak(&self) -> usize {
        self.counters.peak.load(Ordering::SeqCst)
    }

    /// The requests that arrived since the last call.
    pub(super) fn seen(&self) -> Vec<Seen> {
        self.requests.try_iter().collect()
    }
}

impl Drop for TlsServer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.accept_thread.take() {
            let _ = thread.join();
        }
    }
}

/// What every connection thread needs.
struct Shared {
    config: Arc<ServerConfig>,
    script: Script,
    counters: Arc<Counters>,
    sender: Sender<Seen>,
    stop: Arc<AtomicBool>,
}

fn server_config(certificate: &str, key: &str) -> Arc<ServerConfig> {
    let chain = vec![CertificateDer::from_pem_slice(certificate.as_bytes()).unwrap()];
    let key = PrivateKeyDer::from_pem_slice(key.as_bytes()).unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(chain, key)
        .unwrap();
    Arc::new(config)
}

fn accept_loop(listener: &TcpListener, shared: &Shared) {
    while !shared.stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _peer)) => {
                shared.counters.accepted.fetch_add(1, Ordering::SeqCst);
                let connection = Connection {
                    config: Arc::clone(&shared.config),
                    script: Arc::clone(&shared.script),
                    counters: Arc::clone(&shared.counters),
                    sender: shared.sender.clone(),
                    stop: Arc::clone(&shared.stop),
                };
                std::thread::spawn(move || connection.run(stream));
            }
            Err(error) if error.kind() == ErrorKind::WouldBlock => {
                std::thread::sleep(ACCEPT_POLL);
            }
            Err(_other) => break,
        }
    }
}

type TlsStream = StreamOwned<ServerConnection, TcpStream>;

struct Connection {
    config: Arc<ServerConfig>,
    script: Script,
    counters: Arc<Counters>,
    sender: Sender<Seen>,
    stop: Arc<AtomicBool>,
}

impl Connection {
    /// Serve one request. A failed handshake or a broken request ends the thread without a reply.
    fn run(self, stream: TcpStream) {
        // A socket from a non-blocking listener can be non-blocking on Windows.
        if stream.set_nonblocking(false).is_err() || stream.set_read_timeout(Some(IO_WAIT)).is_err()
        {
            return;
        }
        let Ok(connection) = ServerConnection::new(Arc::clone(&self.config)) else {
            return;
        };
        let mut tls = StreamOwned::new(connection, stream);
        let Some(seen) = read_request(&mut tls) else {
            return;
        };
        let active = self
            .counters
            .active
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        self.counters.peak.fetch_max(active, Ordering::SeqCst);
        let reply = (self.script)(&seen);
        let _ = self.sender.send(seen);
        self.respond(&mut tls, &reply);
        self.counters.active.fetch_sub(1, Ordering::SeqCst);
    }

    fn respond(&self, tls: &mut TlsStream, reply: &Reply) {
        let text = match reply {
            Reply::Status(code) => {
                format!("HTTP/1.1 {code} Test\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
            }
            Reply::Redirect(location) => format!(
                "HTTP/1.1 301 Moved\r\nLocation: {location}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
            ),
            Reply::Silence(time) => {
                self.stay_silent(*time);
                return;
            }
        };
        let _ = tls.write_all(text.as_bytes());
        tls.conn.send_close_notify();
        let _ = tls.flush();
    }

    fn stay_silent(&self, time: Duration) {
        let end = Instant::now().checked_add(time).unwrap();
        while Instant::now() < end && !self.stop.load(Ordering::SeqCst) {
            std::thread::sleep(SILENCE_POLL);
        }
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Read the head and the body of one request through the TLS stream.
fn read_request(tls: &mut TlsStream) -> Option<Seen> {
    let mut raw = Vec::new();
    let mut chunk = [0_u8; 1024];
    let head_end = loop {
        if let Some(position) = find(&raw, b"\r\n\r\n") {
            break position;
        }
        let count = tls.read(&mut chunk).ok().filter(|count| *count > 0)?;
        raw.extend_from_slice(&chunk[..count]);
    };
    let head = String::from_utf8_lossy(&raw[..head_end]).into_owned();
    let (method, target, content_type, length) = parse_head(&head)?;
    let body_start = head_end.checked_add(4).unwrap();
    let body_end = body_start.checked_add(length).unwrap();
    while raw.len() < body_end {
        let count = tls.read(&mut chunk).ok().filter(|count| *count > 0)?;
        raw.extend_from_slice(&chunk[..count]);
    }
    let body = String::from_utf8_lossy(&raw[body_start..body_end]).into_owned();
    Some(Seen {
        method,
        target,
        content_type,
        body,
    })
}

/// Split the head into the method, the target, the content type, and the body length.
fn parse_head(head: &str) -> Option<(String, String, Option<String>, usize)> {
    let mut lines = head.split("\r\n");
    let mut request_line = lines.next()?.split(' ');
    let method = request_line.next()?.to_owned();
    let target = request_line.next()?.to_owned();
    let mut content_type = None;
    let mut length = 0_usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name.to_ascii_lowercase().as_str() {
            "content-type" => content_type = Some(value.trim().to_owned()),
            "content-length" => length = value.trim().parse().ok()?,
            _other => {}
        }
    }
    Some((method, target, content_type, length))
}
