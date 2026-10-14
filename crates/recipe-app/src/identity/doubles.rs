//! Test doubles of [`PasswordHasher`] (design D5).
//!
//! The doubles make the tests of the Argon2 gate deterministic, because no real hash takes time.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use parking_lot::{Condvar, Mutex};
use recipe_domain::identity::{NewPassword, Password, PasswordHash};

use super::{HashFailure, PasswordHasher};

/// A valid PHC string that the hash of [`BlockingHasher`] returns.
const FIXED_HASH: &str =
    "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0$ZG91YmxlZG91YmxlZG91YmxlZG91Ymxl";
/// The longest real time that `wait_started` waits.
const START_WAIT_LIMIT: Duration = Duration::from_secs(10);

/// A hasher whose calls wait until a test opens it.
///
/// The type solves one problem: a flood test must keep several checks running at one time
/// without a slow hash. It records how many calls run now, the most that ran at one time, and
/// how many calls started. A call returns `true` from `verify` when it gets a stored hash.
#[derive(Debug, Default)]
pub struct BlockingHasher {
    open: Mutex<bool>,
    opened: Condvar,
    running: AtomicUsize,
    max_running: AtomicUsize,
    started: AtomicUsize,
}

impl BlockingHasher {
    /// Return a closed hasher. Its calls wait until [`BlockingHasher::release`].
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the hasher. The waiting calls and every later call end at once.
    pub fn release(&self) {
        *self.open.lock() = true;
        self.opened.notify_all();
    }

    /// Return the number of calls that run now.
    #[must_use]
    pub fn running(&self) -> usize {
        self.running.load(Ordering::SeqCst)
    }

    /// Return the most calls that ran at one time.
    #[must_use]
    pub fn max_running(&self) -> usize {
        self.max_running.load(Ordering::SeqCst)
    }

    /// Return the number of calls that started.
    #[must_use]
    pub fn started(&self) -> usize {
        self.started.load(Ordering::SeqCst)
    }

    /// Wait until `count` calls have started. Return `false` after ten real seconds.
    pub async fn wait_started(&self, count: usize) -> bool {
        let deadline = Instant::now().checked_add(START_WAIT_LIMIT);
        while self.started() < count {
            if deadline.is_none_or(|limit| Instant::now() > limit) {
                return false;
            }
            tokio::task::yield_now().await;
            std::thread::yield_now();
        }
        true
    }

    /// Record the start of a call, wait for the release, and record the end.
    fn pass(&self) {
        self.started.fetch_add(1, Ordering::SeqCst);
        let now = self
            .running
            .fetch_add(1, Ordering::SeqCst)
            .saturating_add(1);
        self.max_running.fetch_max(now, Ordering::SeqCst);
        let mut open = self.open.lock();
        while !*open {
            self.opened.wait(&mut open);
        }
        drop(open);
        self.running.fetch_sub(1, Ordering::SeqCst);
    }
}

impl PasswordHasher for BlockingHasher {
    fn hash(&self, _password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        self.pass();
        PasswordHash::parse(FIXED_HASH).map_err(|_malformed| HashFailure::Compute)
    }

    fn verify(&self, _password: &Password, hash: Option<&PasswordHash>) -> bool {
        self.pass();
        hash.is_some()
    }
}

/// The payload of the panic of [`PanickingHasher`].
#[cfg(test)]
#[derive(Clone, Copy, Debug)]
struct HasherPanic;

/// A hasher whose every call panics.
///
/// The type solves one problem: a test must prove that a panic in the blocking task gives
/// `E-WEB-011` and returns the permit. The panic unwinds without a message on stderr.
/// The type exists in test builds only, because the feature `memory` allows no panic path.
#[cfg(test)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PanickingHasher;

#[cfg(test)]
impl PasswordHasher for PanickingHasher {
    fn hash(&self, _password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        std::panic::resume_unwind(Box::new(HasherPanic))
    }

    fn verify(&self, _password: &Password, _hash: Option<&PasswordHash>) -> bool {
        std::panic::resume_unwind(Box::new(HasherPanic))
    }
}
