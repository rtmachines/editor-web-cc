//! The one place that turns a use case error into an `anyhow` error with its error code.
//!
//! The helper solves one problem. The `ERROR` line of a failed subcommand must hold the error
//! code and must keep the chain of causes (`reliability.md` section 1.1).

use recipe_app::error::AppError;

/// Turn an error into an `anyhow` error with its error code as the outer context.
///
/// The error stays the source of the result. The plain text of the result is the code. The
/// alternate text `{:#}` is the code, the message, and every cause. A secret never enters the
/// chain, because no error variant carries one.
pub fn coded(error: impl Into<AppError>) -> anyhow::Error {
    let error = error.into();
    let code = error.code();
    anyhow::Error::new(error).context(code)
}

#[cfg(test)]
mod tests;
