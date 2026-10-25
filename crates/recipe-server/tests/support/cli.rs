//! Runs of the built binary `recipe-server` that end by themselves, and the reading of their logs.

use std::path::PathBuf;
use std::process::{Command, Output};

/// Run `recipe-server migrate --config <path>` and return its output.
pub async fn run_migrate(config: PathBuf) -> Output {
    run_migrate_with_env(config, Vec::new()).await
}

/// Run the same command with extra environment values for the child process only.
pub async fn run_migrate_with_env(config: PathBuf, envs: Vec<(&'static str, String)>) -> Output {
    tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_recipe-server"))
            .arg("migrate")
            .arg("--config")
            .arg(config)
            .env_remove("RUST_LOG")
            .envs(envs)
            .output()
            .unwrap()
    })
    .await
    .unwrap()
}

/// The standard output and the standard error of a run as one text.
pub fn text_of(output: &Output) -> String {
    format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// The number of log lines on the standard output with the level `level`, for example `ERROR`.
pub fn lines_with(output: &Output, level: &str) -> usize {
    let marker = format!("\"level\":\"{level}\"");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains(&marker))
        .count()
}
