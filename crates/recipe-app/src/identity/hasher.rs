//! The password hasher port and its Argon2id adapter (design D5, `reliability.md` section 7.11).

use std::fmt;

use argon2::password_hash::{PasswordHasher as _, PasswordVerifier as _};
use argon2::{Algorithm, Argon2, Params, Version};
use recipe_domain::identity::{NewPassword, Password, PasswordHash};

use crate::error::{AppError, TaskStage};

/// Default of `argon2.m_cost` in KiB: 19 MiB (`reliability.md` section 7.11).
const DEFAULT_M_COST_KIB: u32 = 19 * 1024;
/// Default of `argon2.t_cost` (`reliability.md` section 7.11).
const DEFAULT_T_COST: u32 = 2;
/// Default of `argon2.p_cost` (`reliability.md` section 7.11).
const DEFAULT_P_COST: u32 = 1;
/// Length of a salt in bytes. This is the length that RFC 9106 recommends.
const SALT_BYTES: usize = 16;
/// Length of the random password that the dummy hash covers.
const DUMMY_PASSWORD_BYTES: usize = 16;
/// Text that `Debug` prints in place of a secret.
const REDACTED: &str = "[redacted]";

/// Why a hash did not complete.
///
/// The type is the reason of a failed hash. It is not an error type and has no `code()`.
/// [`AppError`] carries it as `E-WEB-011` (design D5).
///
/// It solves one problem: the hasher reports a failure without the password and without the hash.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HashFailure {
    /// Argon2 refuses the cost parameters.
    Parameters,
    /// The operating system gave no random bytes for the salt.
    Random,
    /// Argon2 could not compute the hash.
    Compute,
}

impl fmt::Display for HashFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parameters => f.write_str("The Argon2 parameters are not valid."),
            Self::Random => f.write_str("The system gave no random bytes."),
            Self::Compute => f.write_str("Argon2 could not compute the hash."),
        }
    }
}

impl From<HashFailure> for AppError {
    fn from(_failure: HashFailure) -> Self {
        Self::TaskFailed {
            stage: TaskStage::PasswordHash,
        }
    }
}

/// The cost parameters of Argon2id (`argon2.m_cost`, `argon2.t_cost`, `argon2.p_cost`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Argon2Params {
    /// Memory in KiB.
    pub m_cost_kib: u32,
    /// Number of passes.
    pub t_cost: u32,
    /// Degree of parallelism.
    pub p_cost: u32,
}

impl Default for Argon2Params {
    /// Return the values of `reliability.md` section 7.11: 19 MiB, 2 passes, 1 lane.
    fn default() -> Self {
        Self {
            m_cost_kib: DEFAULT_M_COST_KIB,
            t_cost: DEFAULT_T_COST,
            p_cost: DEFAULT_P_COST,
        }
    }
}

/// Port that hashes and checks passwords.
///
/// The port solves one problem: a use case must hash a password without a dependency on Argon2,
/// so a test can replace the slow hash. The methods are synchronous and take seconds of CPU
/// time. Call them only under a `HashPermit` and inside `spawn_blocking`.
pub trait PasswordHasher: Send + Sync {
    /// Hash a new password into a PHC string.
    ///
    /// # Errors
    ///
    /// Returns [`HashFailure`] when the hash does not complete.
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure>;

    /// Check a typed password against a stored hash.
    ///
    /// With `None`, the check runs against a dummy hash and returns `false`. A caller passes
    /// `None` for an unknown or inactive user, so every check costs the same time.
    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool;
}

/// The Argon2id adapter of [`PasswordHasher`].
///
/// The type solves one problem: a check of an unknown user must cost as much as a check of a
/// known user. The constructor hashes a random password once. That dummy hash has the same
/// parameters as a real hash.
pub struct Argon2Hasher {
    argon2: Argon2<'static>,
    params: Argon2Params,
    dummy: PasswordHash,
}

impl Argon2Hasher {
    /// Build a hasher for Argon2id, version `0x13`, with the given cost parameters.
    ///
    /// The constructor computes the dummy hash, so it takes as long as one hash.
    ///
    /// # Errors
    ///
    /// Returns [`HashFailure::Parameters`] when Argon2 refuses the parameters.
    /// Returns [`HashFailure::Random`] or [`HashFailure::Compute`] when the dummy hash fails.
    pub fn new(params: Argon2Params) -> Result<Self, HashFailure> {
        let cost = Params::new(params.m_cost_kib, params.t_cost, params.p_cost, None)
            .map_err(|_refused| HashFailure::Parameters)?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, cost);
        let mut random = [0_u8; DUMMY_PASSWORD_BYTES];
        getrandom::fill(&mut random).map_err(|_failed| HashFailure::Random)?;
        let text: String = random.iter().map(|byte| format!("{byte:02x}")).collect();
        let dummy = hash_text(&argon2, &text)?;
        Ok(Self {
            argon2,
            params,
            dummy,
        })
    }

    /// Return the dummy hash that a check without a stored hash uses.
    fn dummy_hash(&self) -> &PasswordHash {
        &self.dummy
    }
}

/// Hash `text` with a fresh salt and return the PHC string.
fn hash_text(argon2: &Argon2<'_>, text: &str) -> Result<PasswordHash, HashFailure> {
    let mut salt = [0_u8; SALT_BYTES];
    getrandom::fill(&mut salt).map_err(|_failed| HashFailure::Random)?;
    let phc = argon2
        .hash_password_with_salt(text.as_bytes(), &salt)
        .map_err(|_failed| HashFailure::Compute)?;
    PasswordHash::parse(&phc.to_string()).map_err(|_malformed| HashFailure::Compute)
}

impl PasswordHasher for Argon2Hasher {
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure> {
        hash_text(&self.argon2, password.as_str())
    }

    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool {
        let Some(stored) = hash else {
            // The result is discarded. The check only spends the time of a real check.
            let _spent = self.check(password, self.dummy_hash());
            return false;
        };
        self.check(password, stored)
    }
}

impl Argon2Hasher {
    /// Check `password` against a PHC string. An unreadable string gives `false`.
    fn check(&self, password: &Password, stored: &PasswordHash) -> bool {
        let Ok(parsed) = argon2::PasswordHash::new(stored.as_str()) else {
            return false;
        };
        self.argon2
            .verify_password(password.as_str().as_bytes(), &parsed)
            .is_ok()
    }
}

impl fmt::Debug for Argon2Hasher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Argon2Hasher")
            .field("params", &self.params)
            .field("dummy", &REDACTED)
            .finish()
    }
}

#[cfg(test)]
mod tests;
