//! The adapter of the port `CredentialCipher` (ADR-0154 (g), design D5).
//!
//! The adapter seals the PLC password with AES-256-GCM. It holds the key of the Docker secret
//! `plc_credential_key`. The sealed form is the version byte `0x01`, the nonce of 12 bytes, and
//! the ciphertext with the tag of 16 bytes. The associated data is the 16 bytes of the machine
//! id, so a sealed secret does not open on another machine. The module writes no log line. No
//! `Debug` text, fault, or message of this module holds the key, a password, or a sealed byte.

use std::fmt;

use aes_gcm::aead::{Aead, Payload};
use aes_gcm::{Aes256Gcm, Key, KeyInit, Nonce};
use recipe_app::ports::{CipherFault, CredentialCipher, SealedSecret};
use recipe_domain::plant::{MachineId, PlcPassword};
use zeroize::Zeroizing;

/// The length of the key in bytes.
pub const KEY_LENGTH: usize = 32;
/// The length of the nonce in bytes.
const NONCE_LENGTH: usize = 12;
/// The length of the authentication tag in bytes.
const TAG_LENGTH: usize = 16;
/// The version byte of the sealed form.
const VERSION: u8 = 1;

/// The 32 bytes of the key of the cipher.
///
/// The type solves one problem: the key must not leak through a log line, a `Debug` text, or a
/// copy that stays in memory. The buffer clears itself on drop. The type has no `Display` and no
/// `Clone`, and its `Debug` hides the bytes.
pub struct CipherKey(Zeroizing<[u8; KEY_LENGTH]>);

impl CipherKey {
    /// Wrap the 32 bytes of a key.
    ///
    /// The buffer moves into the key, so no copy of the bytes stays unzeroized on the stack.
    #[must_use]
    pub fn from_bytes(bytes: Zeroizing<[u8; KEY_LENGTH]>) -> Self {
        Self(bytes)
    }
}

impl fmt::Debug for CipherKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CipherKey(<redacted>)")
    }
}

/// Adapter of [`CredentialCipher`] with AES-256-GCM.
///
/// The type solves one problem: `recipe-app` must seal and open the PLC password and must not
/// hold the key (design D5). The wrapped cipher clears its key schedule on drop. Each `seal` uses
/// a new random nonce from the operating system.
pub struct AesGcmCipher {
    /// The AES-256-GCM state. It holds the key schedule.
    cipher: Aes256Gcm,
}

impl AesGcmCipher {
    /// Build the adapter from the key.
    #[must_use]
    pub fn new(key: &CipherKey) -> Self {
        Self {
            cipher: Aes256Gcm::new(<&Key<Aes256Gcm>>::from(&*key.0)),
        }
    }
}

impl fmt::Debug for AesGcmCipher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AesGcmCipher(<redacted>)")
    }
}

impl CredentialCipher for AesGcmCipher {
    fn seal(
        &self,
        machine: MachineId,
        password: &PlcPassword,
    ) -> Result<SealedSecret, CipherFault> {
        let mut nonce_bytes = [0_u8; NONCE_LENGTH];
        getrandom::fill(&mut nonce_bytes).map_err(|_failed| CipherFault::Random)?;
        let nonce = Nonce::<aes_gcm::aead::consts::U12>::from(nonce_bytes);
        let payload = Payload {
            msg: password.expose().as_bytes(),
            aad: machine.as_uuid().as_bytes(),
        };
        let encrypted = self
            .cipher
            .encrypt(&nonce, payload)
            .map_err(|_failed| CipherFault::Cipher)?;
        let sealed = [
            [VERSION].as_slice(),
            nonce_bytes.as_slice(),
            encrypted.as_slice(),
        ]
        .concat();
        Ok(SealedSecret::from_bytes(sealed))
    }

    fn open(&self, machine: MachineId, sealed: &SealedSecret) -> Result<PlcPassword, CipherFault> {
        let (version, rest) = sealed.as_bytes().split_first().ok_or(CipherFault::Format)?;
        if *version != VERSION {
            return Err(CipherFault::Format);
        }
        let (nonce_bytes, encrypted) = rest
            .split_at_checked(NONCE_LENGTH)
            .ok_or(CipherFault::Format)?;
        if encrypted.len() < TAG_LENGTH {
            return Err(CipherFault::Format);
        }
        let nonce = Nonce::<aes_gcm::aead::consts::U12>::try_from(nonce_bytes)
            .map_err(|_failed| CipherFault::Format)?;
        let payload = Payload {
            msg: encrypted,
            aad: machine.as_uuid().as_bytes(),
        };
        let plain = Zeroizing::new(
            self.cipher
                .decrypt(&nonce, payload)
                .map_err(|_failed| CipherFault::Authentication)?,
        );
        let text = std::str::from_utf8(&plain).map_err(|_failed| CipherFault::Format)?;
        PlcPassword::parse(text).map_err(|_failed| CipherFault::Format)
    }
}

#[cfg(test)]
mod tests;
