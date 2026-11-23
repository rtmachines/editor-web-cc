//! Heap peak measurement for the tests of the grid answers (design D6, ADR-0223).
//!
//! The crate forbids `unsafe`, so the allocator comes from the dev-dependency `dhat`.
//! Without a running profiler, `dhat::Alloc` forwards each call to the system allocator.
//! The allocator and the profiler exist only with the flag `--cfg heap_measure`.
//! The normal test build keeps the system allocator, so the normal tests run at full speed.
//! A measurement test has `#[ignore]` and runs alone, in its own target directory:
//! `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web
//! --all-features --locked -- --ignored --test-threads=1`.
//! Without the flag, `peak_in` fails at once, because its result would be wrong.

use std::future::Future;

use axum::Router;
use axum::body::Body;
use axum::http::{HeaderMap, Request, StatusCode};
use http_body_util::BodyExt as _;
use sha2::{Digest, Sha256};
use tower::ServiceExt as _;

#[cfg(heap_measure)]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

/// Tell whether the build has the flag `--cfg heap_measure`.
///
/// A plain function, because `assert!(cfg!(..))` is a constant assertion that clippy denies.
fn has_heap_measure_flag() -> bool {
    cfg!(heap_measure)
}

/// Start a profiler that writes no file and keeps short backtraces.
fn new_profiler() -> dhat::Profiler {
    dhat::Profiler::builder()
        .testing()
        .trim_backtraces(Some(4))
        .build()
}

/// Run `work` under a new profiler and return the peak heap above the heap at its start.
///
/// Solves the problem that a process-wide peak includes the heap of earlier tests.
/// The profiler counts only blocks that it sees allocated, so the result excludes the older heap.
/// The testing mode writes no `dhat-heap.json` file. Only one profiler can run at a time.
///
/// # Panics
///
/// Panics without the flag `--cfg heap_measure`, because the profiler sees no allocation then.
pub(crate) fn peak_in<T>(work: impl FnOnce() -> T) -> (T, usize) {
    assert!(
        has_heap_measure_flag(),
        "peak_in needs the flag: build with RUSTFLAGS=\"--cfg heap_measure\" (design D6)"
    );
    let profiler = new_profiler();
    let output = work();
    let peak = dhat::HeapStats::get().max_bytes;
    drop(profiler);
    (output, peak)
}

/// The result of [`peak_of`]: the answer as a length and a hash, and the heap peak.
#[derive(Debug)]
pub(crate) struct Measured {
    pub(crate) status: StatusCode,
    /// The headers of the answer, for example `HX-Trigger` or `Location`.
    pub(crate) headers: HeaderMap,
    /// The length of the answer body in bytes.
    pub(crate) len: usize,
    /// The SHA-256 of the answer body in lower-case hex, with the request id replaced.
    pub(crate) sha256: String,
    /// The peak heap in bytes above the heap at the start of the interval.
    pub(crate) peak: usize,
}

/// Send one request through `router` and measure its peak heap.
///
/// Solves the problem that a test needs the peak of a whole request, with the request body in
/// it, but must not hold a second copy of a large answer. `build` runs inside the interval, so
/// it allocates the request body there. The answer body goes frame by frame into [`BodyDigest`].
/// The interval ends after the drop of the answer body.
///
/// # Panics
///
/// Panics without the flag `--cfg heap_measure`, because the profiler sees no allocation then.
pub(crate) async fn peak_of(router: &Router, build: impl FnOnce() -> Request<Body>) -> Measured {
    assert!(
        has_heap_measure_flag(),
        "peak_of needs the flag: build with RUSTFLAGS=\"--cfg heap_measure\" (design D6)"
    );
    let (measured, peak) = peak_during(async {
        let response = router.clone().oneshot(build()).await.unwrap();
        let (parts, mut body) = response.into_parts();
        let request_id = parts
            .headers
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .unwrap_or_default();
        let mut digest = BodyDigest::new(request_id);
        while let Some(frame) = body.frame().await {
            if let Ok(data) = frame.unwrap().into_data() {
                digest.push(&data);
            }
        }
        drop(body);
        let (len, sha256) = digest.finish();
        (parts.status, parts.headers, len, sha256)
    })
    .await;
    let (status, headers, len, sha256) = measured;
    Measured {
        status,
        headers,
        len,
        sha256,
        peak,
    }
}

/// Await `work` under a new profiler and return the peak heap above the heap at its start.
async fn peak_during<T>(work: impl Future<Output = T>) -> (T, usize) {
    let profiler = new_profiler();
    let output = work.await;
    let peak = dhat::HeapStats::get().max_bytes;
    drop(profiler);
    (output, peak)
}

/// The header that carries the request id (`pipeline.rs`).
const REQUEST_ID_HEADER: &str = "x-request-id";

/// A SHA-256 and a length of an answer body, built from its frames.
///
/// Solves the problem that each answer holds a new request id, so a pinned hash must not depend
/// on it. The digest replaces each copy of the id with `0` bytes of the same length. A copy can
/// cross the border of two frames. The `carry` holds the last bytes that are not hashed yet, at
/// most one byte less than the id. The digest never copies a frame, apart from a join of two
/// short pieces.
pub(crate) struct BodyDigest {
    hasher: Sha256,
    id: Vec<u8>,
    fixed: Vec<u8>,
    carry: Vec<u8>,
    len: usize,
}

impl BodyDigest {
    /// Start a digest. An empty `request_id` means that the digest replaces nothing.
    pub(crate) fn new(request_id: &str) -> Self {
        Self {
            hasher: Sha256::new(),
            id: request_id.as_bytes().to_vec(),
            fixed: vec![b'0'; request_id.len()],
            carry: Vec::new(),
            len: 0,
        }
    }

    /// The number of bytes that wait for the next frame.
    fn carry_len(&self) -> usize {
        self.carry.len()
    }

    /// Add one frame.
    pub(crate) fn push(&mut self, frame: &[u8]) {
        self.len = self.len.saturating_add(frame.len());
        if self.id.is_empty() {
            self.hasher.update(frame);
            return;
        }
        let border = self.id.len().saturating_sub(1);
        if frame.len() < border {
            let mut joined = std::mem::take(&mut self.carry);
            joined.extend_from_slice(frame);
            self.scan(&joined);
            return;
        }
        let rest = self.join_border(frame, border);
        self.scan(rest);
    }

    /// Hash the carry, replace an id that crosses the border, and return the frame after it.
    ///
    /// The carry is shorter than the id and `frame` holds `border` bytes or more. So the first
    /// `border` bytes of the frame decide each copy that starts in the carry.
    fn join_border<'a>(&mut self, frame: &'a [u8], border: usize) -> &'a [u8] {
        let mut window = self.carry.clone();
        window.extend_from_slice(&frame[..border]);
        let start = (0..self.carry.len()).find(|&at| window[at..].starts_with(&self.id));
        let rest = match start {
            Some(at) => {
                self.hasher.update(&self.carry[..at]);
                self.hasher.update(&self.fixed);
                &frame[at
                    .saturating_add(self.id.len())
                    .saturating_sub(self.carry.len())..]
            }
            None => {
                self.hasher.update(&self.carry);
                frame
            }
        };
        self.carry.clear();
        rest
    }

    /// Hash `data`, replace each id in it, and keep the last `id.len() - 1` bytes as carry.
    fn scan(&mut self, mut data: &[u8]) {
        while let Some(at) = find(data, &self.id) {
            self.hasher.update(&data[..at]);
            self.hasher.update(&self.fixed);
            data = &data[at.saturating_add(self.id.len())..];
        }
        let (done, kept) =
            data.split_at(data.len().saturating_sub(self.id.len().saturating_sub(1)));
        self.hasher.update(done);
        self.carry.clear();
        self.carry.extend_from_slice(kept);
    }

    /// End the digest and return the length and the hash in lower-case hex.
    pub(crate) fn finish(mut self) -> (usize, String) {
        self.hasher.update(&self.carry);
        let hash = self.hasher.finalize();
        (
            self.len,
            hash.iter().map(|byte| format!("{byte:02x}")).collect(),
        )
    }
}

/// Return the first position of `needle` in `haystack`.
fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    let (first, rest) = needle.split_first()?;
    let mut from = 0;
    while let Some(offset) = haystack.get(from..)?.iter().position(|byte| byte == first) {
        let at = from.saturating_add(offset);
        if haystack.get(at.saturating_add(1)..)?.starts_with(rest) {
            return Some(at);
        }
        from = at.saturating_add(1);
    }
    None
}

#[cfg(test)]
mod tests {
    use std::hint::black_box;
    use std::path::Path;

    use axum::Router;
    use axum::body::{Body, Bytes};
    use axum::http::{Method, StatusCode};
    use axum::routing::{get, post};
    use http_body_util::BodyExt as _;
    use http_body_util::channel::Channel;
    use sha2::{Digest, Sha256};
    use tower::ServiceExt as _;

    use super::{BodyDigest, peak_during, peak_in, peak_of};
    use crate::test_support::request;

    const MIB: usize = 1024 * 1024;

    fn allocate(bytes: usize) -> usize {
        let block = black_box(vec![1_u8; bytes]);
        black_box(block.len())
    }

    #[test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    fn peak_in_sees_a_block_of_eight_mebibytes() {
        let (len, peak) = peak_in(|| allocate(8 * MIB));
        assert_eq!(len, 8 * MIB);
        assert!(peak >= 8 * MIB, "peak {peak} is below 8 MiB");
    }

    #[test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    fn peak_in_runs_twice_in_one_process() {
        let held = black_box(vec![1_u8; 16 * MIB]);
        let (_, first) = peak_in(|| allocate(8 * MIB));
        let (_, second) = peak_in(|| allocate(2 * MIB));
        assert!(first >= 8 * MIB, "first peak {first}");
        assert!(second >= 2 * MIB, "second peak {second}");
        assert!(
            second < 8 * MIB,
            "second peak {second} holds the first interval or the held heap"
        );
        assert_eq!(black_box(held.len()), 16 * MIB);
    }

    #[test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    fn peak_in_writes_no_profile_file() {
        let _ = peak_in(|| allocate(MIB));
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("dhat-heap.json");
        assert!(!file.exists(), "{} exists", file.display());
    }

    #[test]
    #[cfg(not(heap_measure))]
    #[should_panic(expected = "cfg heap_measure")]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    fn peak_in_refuses_to_run_without_the_flag() {
        let _ = peak_in(|| allocate(MIB));
    }

    /// A request id as `pipeline.rs` writes it: a hyphenated UUID of 36 bytes.
    const ID: &str = "0198a5f2-7c3e-7b1d-9e44-5a2f8c6d1b30";
    const FOUR_MIB: usize = 4 * MIB;
    const FRAME: usize = 64 * 1024;
    const FRAMES: usize = FOUR_MIB / FRAME;

    fn plain_hash(body: &str, id: &str) -> String {
        let fixed = "0".repeat(id.len());
        hex(body.replace(id, &fixed).as_bytes())
    }

    fn hex(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect()
    }

    fn digest_of(frames: &[&[u8]], id: &str) -> (usize, String, usize) {
        let mut digest = BodyDigest::new(id);
        let mut widest_carry = 0;
        for frame in frames {
            digest.push(frame);
            widest_carry = widest_carry.max(digest.carry_len());
        }
        let (len, hash) = digest.finish();
        (len, hash, widest_carry)
    }

    #[test]
    fn digest_replaces_a_request_id_across_the_border_of_two_frames() {
        let body = format!("<p>{ID}</p><i>{ID}</i>");
        let cut = body.find(ID).unwrap() + 10;
        let (head, tail) = body.as_bytes().split_at(cut);
        let (len, hash, _) = digest_of(&[head, tail], ID);
        assert_eq!(len, body.len());
        assert_eq!(hash, plain_hash(&body, ID));
    }

    #[test]
    fn digest_equals_the_whole_body_hash_at_every_cut_point() {
        let body = format!("x{ID}yy{ID}{ID}0198a5f2-7c3e-7b1d-9e44-5a2f8c6d1b3z{ID}");
        let expected = plain_hash(&body, ID);
        for cut in 0..=body.len() {
            let (head, tail) = body.as_bytes().split_at(cut);
            let (len, hash, carry) = digest_of(&[head, tail], ID);
            assert_eq!(len, body.len(), "cut {cut}");
            assert_eq!(hash, expected, "cut {cut}");
            assert!(carry < ID.len(), "cut {cut}: carry {carry}");
        }
    }

    #[test]
    fn digest_equals_the_whole_body_hash_for_frames_of_one_byte_and_of_three_bytes() {
        let body = format!("a{ID}b{ID}");
        let expected = plain_hash(&body, ID);
        for size in [1, 3, 35, 36, 37] {
            let frames: Vec<&[u8]> = body.as_bytes().chunks(size).collect();
            let (len, hash, carry) = digest_of(&frames, ID);
            assert_eq!(len, body.len(), "size {size}");
            assert_eq!(hash, expected, "size {size}");
            assert!(carry <= 35, "size {size}: carry {carry}");
        }
    }

    #[test]
    fn digest_without_a_request_id_hashes_the_body_as_it_is() {
        let body = "no id here";
        let (len, hash, carry) = digest_of(&[body.as_bytes()], "");
        assert_eq!(len, body.len());
        assert_eq!(hash, hex(body.as_bytes()));
        assert_eq!(carry, 0);
    }

    fn sink_router() -> Router {
        Router::new().route("/sink", post(|| async { "ok" }))
    }

    /// Answer 4 MiB in 64 frames. A task makes the next frame when the reader took the last one.
    async fn stream_answer() -> Body {
        let (mut sender, channel) = Channel::<Bytes>::new(1);
        tokio::spawn(async move {
            for _ in 0..FRAMES {
                if sender
                    .send_data(Bytes::from(vec![b'x'; FRAME]))
                    .await
                    .is_err()
                {
                    return;
                }
            }
        });
        Body::new(channel)
    }

    #[tokio::test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    async fn peak_of_sees_a_request_body_of_four_mebibytes_built_in_the_closure() {
        let measured = peak_of(&sink_router(), || {
            request(Method::POST, "/sink")
                .body(Body::from(vec![1_u8; FOUR_MIB]))
                .unwrap()
        })
        .await;
        assert_eq!(measured.status, StatusCode::OK);
        assert_eq!(measured.len, 2);
        assert!(
            measured.peak >= FOUR_MIB,
            "peak {} is below 4 MiB",
            measured.peak
        );
        assert!(
            measured.peak < FOUR_MIB + MIB,
            "peak {} holds a copy",
            measured.peak
        );
    }

    #[tokio::test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    async fn peak_of_keeps_no_frame_of_a_stream_of_four_mebibytes() {
        let router = Router::new().route("/stream", get(stream_answer));
        let measured = peak_of(&router, || {
            request(Method::GET, "/stream").body(Body::empty()).unwrap()
        })
        .await;
        assert_eq!(measured.len, FOUR_MIB);
        let whole = "x".repeat(FOUR_MIB);
        assert_eq!(measured.sha256, plain_hash(&whole, ID));
        assert!(measured.peak < MIB, "peak {} keeps frames", measured.peak);

        let (frames, keeper_peak) = peak_during(async {
            let req = request(Method::GET, "/stream").body(Body::empty()).unwrap();
            let mut body = router.clone().oneshot(req).await.unwrap().into_body();
            let mut frames = Vec::new();
            while let Some(frame) = body.frame().await {
                frames.push(frame.unwrap().into_data().unwrap());
            }
            frames
        })
        .await;
        assert_eq!(frames.len(), FRAMES);
        assert!(
            keeper_peak >= FOUR_MIB,
            "keeper peak {keeper_peak} is below 4 MiB"
        );
    }

    #[tokio::test]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    async fn peak_of_hashes_an_answer_with_the_fixed_text_for_its_request_id() {
        let body = format!("<p>{ID}</p><i>{ID}</i>");
        let answer = body.clone();
        let router = Router::new().route(
            "/id",
            get(move || {
                let answer = answer.clone();
                async move { ([("x-request-id", ID)], answer) }
            }),
        );
        let measured = peak_of(&router, || {
            request(Method::GET, "/id").body(Body::empty()).unwrap()
        })
        .await;
        assert_eq!(measured.len, body.len());
        assert_eq!(measured.sha256, plain_hash(&body, ID));
    }

    #[tokio::test]
    #[cfg(not(heap_measure))]
    #[should_panic(expected = "cfg heap_measure")]
    #[ignore = "measurement test: run alone with --ignored --test-threads=1"]
    async fn peak_of_refuses_to_run_without_the_flag() {
        let _ = peak_of(&sink_router(), || {
            request(Method::POST, "/sink").body(Body::empty()).unwrap()
        })
        .await;
    }
}
