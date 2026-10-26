//! Runs of the built binary `recipe-server` that end by themselves, and the reading of their logs.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// The time that a run of `migrate` may take before the helper kills the child.
const CHILD_LIMIT: Duration = Duration::from_secs(60);
/// The number of bytes that the helper keeps of each pipe. It reads and drops the rest.
const OUTPUT_LIMIT: usize = 1024 * 1024;
/// The pause between two checks of a child that may have exited.
const EXIT_POLL_PAUSE: Duration = Duration::from_millis(50);

/// Run `recipe-server migrate --config <path>` and return its output.
pub async fn run_migrate(config: PathBuf) -> Output {
    run_migrate_with_env(config, Vec::new()).await
}

/// Run the same command with extra environment values for the child process only.
pub async fn run_migrate_with_env(config: PathBuf, envs: Vec<(&'static str, String)>) -> Output {
    tokio::task::spawn_blocking(move || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
        command
            .arg("migrate")
            .arg("--config")
            .arg(config)
            .env_remove("RUST_LOG")
            .envs(envs);
        run_bounded(command, CHILD_LIMIT, OUTPUT_LIMIT)
    })
    .await
    .unwrap()
}

/// Run `command` and return its output. The run has two bounds.
///
/// The helper keeps at most `max_bytes` of each pipe and drops the rest. When the child does
/// not exit within `limit`, the helper kills it and fails the test with the captured output.
/// The child must not start a process that keeps its pipes open after the kill.
pub fn run_bounded(mut command: Command, limit: Duration, max_bytes: usize) -> Output {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let stdout_reader = std::thread::spawn(move || read_capped(stdout, max_bytes));
    let stderr_reader = std::thread::spawn(move || read_capped(stderr, max_bytes));

    let start = Instant::now();
    let mut status = child.try_wait().unwrap();
    while status.is_none() && start.elapsed() < limit {
        std::thread::sleep(EXIT_POLL_PAUSE);
        status = child.try_wait().unwrap();
    }
    let timed_out = status.is_none();
    if timed_out {
        // The child may have exited since the last check. Then `kill` fails, and that is fine.
        let _ = child.kill();
        // After a kill the operating system ends the child, so this wait returns at once.
        status = child.wait().ok();
    }
    // The readers end when the pipes close, that is when the child has exited.
    let stdout = stdout_reader.join().unwrap();
    let stderr = stderr_reader.join().unwrap();
    assert!(
        !timed_out,
        "the child did not exit within {limit:?}:
{}
{}",
        String::from_utf8_lossy(&stdout),
        String::from_utf8_lossy(&stderr)
    );
    Output {
        status: status.unwrap(),
        stdout,
        stderr,
    }
}

/// Read `reader` to its end. Keep the first `max_bytes` bytes and drop the rest.
///
/// The function reads the rest too, so a child never blocks on a full pipe.
pub fn read_capped(reader: impl Read, max_bytes: usize) -> Vec<u8> {
    let mut reader = reader;
    let mut kept = Vec::new();
    let cap = u64::try_from(max_bytes).unwrap_or(u64::MAX);
    // A read fault ends the capture. The child status and the kept bytes still tell the story.
    let _ = (&mut reader).take(cap).read_to_end(&mut kept);
    let _ = std::io::copy(&mut reader, &mut std::io::sink());
    kept
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
