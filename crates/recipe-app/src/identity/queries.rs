//! The six identity queries (design D9).
//!
//! Each query checks its requirement first. The two user queries then read in a read-only
//! transaction. The other four read only the data that their page shows.

mod forms;
mod users;

pub use forms::{
    GetLoginForm, GetOwnPasswordForm, GetOwnProfile, GetUserForm, LoginForm, OwnPasswordForm,
    OwnProfile, UserForm,
};
pub use users::{GetUser, ListUsers, ListUsersInput, UserDetail};

#[cfg(test)]
mod tests;
