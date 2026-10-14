//! Identity and session use cases and their helpers (design D2).

mod activate_user;
mod admin_change;
mod authenticate;
mod bootstrap;
mod change_own_password;
mod context;
mod create_user;
mod deactivate_user;
#[cfg(any(test, feature = "memory"))]
mod doubles;
#[cfg(test)]
pub(crate) mod fixtures;
mod gate;
mod hasher;
mod issue;
mod login;
mod logout;
mod no_session;
mod one_time;
mod queries;
mod reset_password;
mod retry;
mod set_user_role;
mod settings;
mod throttle;
mod unlock_login;

pub use activate_user::ActivateUser;
pub use authenticate::{Authenticated, authenticate};
pub use bootstrap::{BootstrapOutcome, bootstrap_administrator};
pub use change_own_password::{ChangeOwnPassword, ChangeOwnPasswordInput};
pub use context::IdentityContext;
pub use create_user::{CreateUser, CreateUserInput};
pub use deactivate_user::DeactivateUser;
#[cfg(any(test, feature = "memory"))]
pub use doubles::BlockingHasher;
#[cfg(test)]
pub use doubles::PanickingHasher;
pub use gate::{HashGate, HashPermit};
pub use hasher::{Argon2Hasher, Argon2Params, HashFailure, PasswordHasher};
pub use issue::{IssuedSession, NONCE_BYTES, new_request_id, random_nonce};
pub use login::{Login, LoginInput, LoginOutcome};
pub use logout::Logout;
pub use one_time::IssuedPassword;
pub use queries::{
    GetLoginForm, GetOwnPasswordForm, GetOwnProfile, GetUser, GetUserForm, ListUsers,
    ListUsersInput, LoginForm, OwnPasswordForm, OwnProfile, UserDetail, UserForm,
};
pub use reset_password::ResetPassword;
pub use retry::serializable_once_more;
pub use set_user_role::{SetUserRole, SetUserRoleInput};
pub use settings::IdentitySettings;
pub use unlock_login::UnlockLogin;
