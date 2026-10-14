//! Identity value types of the domain (`reliability.md` section 1.3, design D2).

mod error;
mod key_ring;
mod last_administrator;
mod login;
mod password_reason;
mod policy;
mod role;
mod secret;
mod throttle;
mod user_id;

pub use error::{CredentialFailure, IdentityError};
pub use key_ring::{KeyRing, KeyRingReason};
pub use last_administrator::{AdminChange, UserStanding, last_administrator_guard};
pub use login::{Login, LoginReason};
pub use password_reason::PasswordReason;
pub use policy::{ONE_TIME_BYTES, OneTimePassword, breached_password_count};
pub use role::{Permission, Role, RoleReason};
pub use secret::{
    CsrfToken, NewPassword, Password, PasswordHash, PasswordHashReason, SessionId, SessionToken,
    TokenReason,
};
pub use throttle::{
    AttemptCount, LoginThrottle, ThrottleDecision, ThrottlePolicy, TypedLogin, TypedLoginReason,
};
pub use user_id::UserId;
