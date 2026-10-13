//! Identity value types of the domain (`reliability.md` section 1.3, design D2).

mod login;
mod user_id;

pub use login::{Login, LoginReason};
pub use user_id::UserId;
