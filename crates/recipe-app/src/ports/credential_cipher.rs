//! Port of the credential cipher (ADR-0154 (g), design D5).

use std::fmt;

use recipe_domain::plant::{MachineId, PlcPassword};

use crate::error::{AppError, TaskStage};

/// The sealed form of a PLC password.
///
/// The type solves one problem: the use case and the storage adapter must carry the sealed
/// password without a path to its plain text. The bytes are opaque. Only the adapter of the port
/// [`CredentialCipher`] reads their layout. The type has no `Display`, and its `Debug` hides the
/// bytes.
#[derive(Clone, PartialEq, Eq)]
pub struct SealedSecret(Vec<u8>);

impl SealedSecret {
    /// Wrap the bytes that a cipher adapter sealed or that the storage adapter read.
    ///
    /// The constructor checks no layout. `CredentialCipher::open` rejects a bad layout.
    #[must_use]
    pub const fn from_bytes(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// Return the sealed bytes, for the storage adapter and for the cipher adapter.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl fmt::Debug for SealedSecret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SealedSecret(<redacted>)")
    }
}

/// Why a call of the port `CredentialCipher` failed.
///
/// This is a reason type. It owns no error code, like `WebhookFault` (ADR-0154 (g),
/// `reliability.md` section 1.1). No variant holds a key, a password, or a sealed byte. The
/// `Display` text and the `Debug` text hold none either.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CipherFault {
    /// The system did not give random bytes for the nonce of a `seal`.
    Random,
    /// The cipher failed to seal the password.
    Cipher,
    /// The sealed secret has an unknown version or a bad layout.
    Format,
    /// The tag of the sealed secret does not match the key or the machine.
    Authentication,
}

impl fmt::Display for CipherFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Random => "The system did not give random bytes.",
            Self::Cipher => "The cipher failed to seal the password.",
            Self::Format => "The sealed secret has a bad form.",
            Self::Authentication => "The sealed secret does not match the key or the machine.",
        })
    }
}

/// Port of the cipher that seals the password of a PLC connection.
///
/// The port solves one problem: the use cases must store a password in a form that a database
/// dump does not reveal, and `recipe-app` must not hold the key. The adapter in `recipe-server`
/// holds the key (design D5). The machine id is the associated data, so a sealed secret does not
/// open on another machine. A call is short CPU work. A use case calls it before it opens a
/// transaction (`reliability.md` section 7.4).
pub trait CredentialCipher: Send + Sync {
    /// Seal `password` for `machine`. Each call uses a new random nonce.
    ///
    /// # Errors
    ///
    /// Returns [`CipherFault::Random`] or [`CipherFault::Cipher`] when the seal fails.
    fn seal(&self, machine: MachineId, password: &PlcPassword)
    -> Result<SealedSecret, CipherFault>;

    /// Open `sealed` for `machine`.
    ///
    /// # Errors
    ///
    /// Returns [`CipherFault::Format`] for a bad layout and [`CipherFault::Authentication`] when
    /// the key or the machine does not match.
    fn open(&self, machine: MachineId, sealed: &SealedSecret) -> Result<PlcPassword, CipherFault>;
}

/// Seal `password` for `machine` and map a fault to the error of the use case.
///
/// The function solves one problem: every use case that stores a password maps a failed seal in
/// the same way. A failed seal is `AppError::TaskFailed` with the stage `Token`, because the
/// failure comes from the read of random bytes or from the cipher and not from the request
/// (design D5). The error holds no fault text, so it cannot hold a secret.
///
/// # Errors
///
/// Returns `AppError::TaskFailed { stage: TaskStage::Token }` when the cipher fails.
pub fn seal_password(
    cipher: &dyn CredentialCipher,
    machine: MachineId,
    password: &PlcPassword,
) -> Result<SealedSecret, AppError> {
    cipher
        .seal(machine, password)
        .map_err(|_fault| AppError::TaskFailed {
            stage: TaskStage::Token,
        })
}
