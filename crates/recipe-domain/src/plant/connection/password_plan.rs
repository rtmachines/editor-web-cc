//! What a connection command does with the stored password (ADR-0154, design decision D3).

use super::PlcPassword;

/// Tells whether the stored record of a machine holds a password.
///
/// The state comes from a read without a lock. The `row_version` compare of the command
/// proves that the state did not change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasswordState {
    /// The record holds no password, or no record exists.
    Absent,
    /// The record holds a password.
    Stored,
}

/// The action of a connection command on the stored password.
///
/// The type has no `PartialEq`, because a password has no constant-time compare.
#[derive(Debug)]
pub enum PasswordPlan {
    /// Keep the stored ciphertext.
    Keep,
    /// Seal this password and store it.
    Replace(PlcPassword),
    /// Delete the stored ciphertext.
    Clear,
}
