//! Binary of the recipe editor.
//!
//! This crate loads the configuration and connects the adapters to the use cases.
//! It runs the subcommands `serve`, `migrate`, and `create-admin`.
//! It is the only crate that uses `anyhow` (Assumption A37).
//!
//! See `docs/architecture/overview.md` section 5 and `docs/architecture/reliability.md` section 2.

#![forbid(unsafe_code)]

/// Starts the recipe editor.
///
/// The skeleton does no work and returns `Ok`.
/// Later steps add the configuration, the wiring, and the subcommands.
///
/// # Errors
///
/// The skeleton returns no error.
fn main() -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn skeleton_main_returns_ok() {
        assert!(super::main().is_ok());
    }
}
