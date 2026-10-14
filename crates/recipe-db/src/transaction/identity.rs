//! The identity sub-ports on the transaction (`identity-and-sessions`, tasks 5.4 to 5.6).
//!
//! `PgTransaction` implements `UserRepository` in `users`, `SessionRepository` in `sessions`,
//! and `AttemptRepository` in `attempts`.

mod attempts;
mod sessions;
mod users;
