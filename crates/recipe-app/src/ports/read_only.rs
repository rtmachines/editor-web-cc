//! Port of the read-only flag (`reliability.md` section 7.8, Assumption A24).

/// Switch of the read-only mode.
///
/// The port solves one problem: the job `disk_check` in `recipe-app` must set the flag that the
/// gate of `recipe-web` reads on every request. `recipe-app` must not depend on `recipe-web`.
/// `recipe-web::ReadOnlyMode` implements the port, and a clone of the flag shares its value.
pub trait ReadOnlySwitch: Send + Sync {
    /// Turn the read-only mode on or off.
    fn set(&self, on: bool);

    /// Return true when the read-only mode is on.
    fn is_on(&self) -> bool;
}
