//! In-memory `CredentialCipher` (task 5.3, design D5).

use std::sync::Arc;

use parking_lot::Mutex;
use recipe_domain::plant::{MachineId, PlcPassword};

use super::call_faults::CallFaults;
use crate::ports::{CipherFault, CredentialCipher, SealedSecret};

/// The version byte of the sealed form (design D5).
const VERSION: u8 = 1;
/// The length of the machine id in the sealed form.
const MACHINE_LENGTH: usize = 16;
/// The mask that hides the password text. It is not a cipher.
const MASK: u8 = 0xA5;

/// One call that the adapter received.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CipherCall {
    /// A call of `seal` for this machine.
    Seal(MachineId),
    /// A call of `open` for this machine.
    Open(MachineId),
}

/// The methods of the port. A fault targets one method.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Method {
    /// `CredentialCipher::seal`.
    Seal,
    /// `CredentialCipher::open`.
    Open,
}

/// The values that an adapter holds.
#[derive(Debug, Default)]
struct Inner {
    /// The calls, in call order.
    calls: Vec<CipherCall>,
    /// The planned faults and the number of calls of each method.
    faults: CallFaults<Method, CipherFault>,
}

/// In-memory adapter of [`CredentialCipher`].
///
/// The type solves one problem: the tests of the use cases must see when a password is sealed
/// and must fail a `seal` or an `open`. The adapter is not a cipher. It stores the version byte,
/// the machine id, and the masked text, so the sealed form hides the text and does not open on
/// another machine. A new adapter succeeds on every call. A clone shares the record and the
/// faults.
#[derive(Clone, Debug, Default)]
pub struct MemoryCredentialCipher {
    /// The values of the adapter.
    inner: Arc<Mutex<Inner>>,
}

impl MemoryCredentialCipher {
    /// Return the calls, in call order. A failed call is part of the record.
    #[must_use]
    pub fn calls(&self) -> Vec<CipherCall> {
        self.inner.lock().calls.clone()
    }

    /// Return the number of calls of `seal`.
    #[must_use]
    pub fn seal_count(&self) -> u64 {
        self.inner.lock().faults.calls(Method::Seal)
    }

    /// Return the number of calls of `open`.
    #[must_use]
    pub fn open_count(&self) -> u64 {
        self.inner.lock().faults.calls(Method::Open)
    }

    /// Make every call of `seal` fail with `fault`.
    pub fn fail_seal(&self, fault: CipherFault) {
        self.inner
            .lock()
            .faults
            .fail_from_call(Method::Seal, 1, fault);
    }

    /// Make call number `call` of `seal` fail with `fault`. Later calls succeed.
    pub fn fail_seal_on_call(&self, call: u64, fault: CipherFault) {
        self.inner
            .lock()
            .faults
            .fail_on_call(Method::Seal, call, fault);
    }

    /// Make every call of `open` fail with `fault`.
    pub fn fail_open(&self, fault: CipherFault) {
        self.inner
            .lock()
            .faults
            .fail_from_call(Method::Open, 1, fault);
    }

    /// Stop every planned fault. The counters keep their values.
    pub fn clear_faults(&self) {
        self.inner.lock().faults.clear();
    }
}

impl CredentialCipher for MemoryCredentialCipher {
    fn seal(
        &self,
        machine: MachineId,
        password: &PlcPassword,
    ) -> Result<SealedSecret, CipherFault> {
        let mut inner = self.inner.lock();
        inner.calls.push(CipherCall::Seal(machine));
        if let Some(fault) = inner.faults.next(Method::Seal) {
            return Err(fault);
        }
        let mut bytes = vec![VERSION];
        bytes.extend_from_slice(machine.as_uuid().as_bytes());
        bytes.extend(password.expose().bytes().map(|byte| byte ^ MASK));
        Ok(SealedSecret::from_bytes(bytes))
    }

    fn open(&self, machine: MachineId, sealed: &SealedSecret) -> Result<PlcPassword, CipherFault> {
        let mut inner = self.inner.lock();
        inner.calls.push(CipherCall::Open(machine));
        if let Some(fault) = inner.faults.next(Method::Open) {
            return Err(fault);
        }
        let (version, rest) = sealed.as_bytes().split_first().ok_or(CipherFault::Format)?;
        if *version != VERSION {
            return Err(CipherFault::Format);
        }
        let (bound_machine, masked) = rest
            .split_at_checked(MACHINE_LENGTH)
            .ok_or(CipherFault::Format)?;
        if bound_machine != machine.as_uuid().as_bytes() {
            return Err(CipherFault::Authentication);
        }
        let text = String::from_utf8(masked.iter().map(|byte| byte ^ MASK).collect())
            .map_err(|_| CipherFault::Format)?;
        PlcPassword::parse(&text).map_err(|_| CipherFault::Format)
    }
}

#[cfg(test)]
mod tests;
