//! The text form of a client address for the `inet` columns (ADR-0035 item p).

use std::net::IpAddr;

/// Return the canonical text of a client address, ready for the cast `::inet`.
///
/// The function solves one problem: `::ffff:10.0.0.5` and `10.0.0.5` are one client. PostgreSQL
/// keeps them as two different `inet` values, so the throttle count would split. Every adapter
/// that binds a client address calls this function.
pub(crate) fn canonical_text(address: IpAddr) -> String {
    address.to_canonical().to_string()
}

#[cfg(test)]
mod tests;
