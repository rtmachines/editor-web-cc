//! Process limits of a parameter (`domain-model.md` section 2.2, design D9).
//!
//! The module holds the rows of a limit request and the list check. The list check runs before
//! each row is resolved against a type tree. The module also holds the limit that a tree accepts.

mod fault;
mod list;
mod parse;
mod process;
mod render;

pub use fault::LimitFault;
pub use list::{LimitInput, LimitList, LimitListReason};
pub use process::{ProcessLimit, StoredLimit};
pub(in crate::recipe) use render::bound_text;
