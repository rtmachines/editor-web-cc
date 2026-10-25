//! The subcommand `migrate` (`reliability.md` section 5).

use std::path::Path;

use anyhow::{Context, Result};
use recipe_db::settings::MigrateSettings;

use crate::coded::coded;
use crate::config;

/// Apply the migrations of `recipe-db` as `recipe_migrator`.
///
/// The function reads the settings and the secret file before it starts the runtime and opens a
/// connection.
///
/// # Errors
///
/// Returns the error of the configuration, of the runtime, or of the migration.
pub fn run(file: Option<&Path>) -> Result<()> {
    let settings = config::load(file)?;
    let target = config::migrator_connection(&settings)?;
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("cannot start the async runtime")?;
    runtime
        .block_on(recipe_db::migrate(&target, &MigrateSettings::default()))
        .map_err(coded)
        .context("the migration failed")?;
    tracing::info!("the migrations are applied");
    Ok(())
}
