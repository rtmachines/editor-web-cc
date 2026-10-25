#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Runs the built binary `recipe-server create-admin` against a PostgreSQL container (task 7.2).
//!
//! The container and the run of `migrate` come from `tests/support/`. The test applies the
//! migrations with the subcommand `migrate`, runs `create-admin` as `recipe_app`, and reads the
//! result with the `psql` of the container.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

mod support;

use support::cli::{lines_with, run_migrate, text_of};
use support::database::{
    APP_PASSWORD, MIGRATOR_PASSWORD, host_port, query, start_container, wait_until_ready,
};
use support::temp::TempDir;

const LABEL_VALUE: &str = "recipe-server-create-admin";
const ARGON2_MARKER: &str = "$argon2id$";
/// A password that meets the policy of Assumption A23.
const GOOD_PASSWORD: &str = "Zq7-plant-recipe-ok";
/// A password of 11 characters.
const SHORT_PASSWORD: &str = "Zq7-plant-o";
const SECOND_LINE: &str = "secondlinemarker";

/// The TOML file of a run. It names the migrator and the app secret files.
fn write_config(dir: &TempDir, port: u16, migrator: &Path, app: &Path) -> PathBuf {
    let text = format!(
        "[db]\nhost = '127.0.0.1'\nport = {port}\nname = 'recipe'\n\n\
         [secrets]\nmigrator_password_file = '{}'\napp_password_file = '{}'\n",
        migrator.display(),
        app.display()
    );
    dir.write("recipe.toml", &text)
}

/// Run `recipe-server create-admin` and pipe `stdin_text` into it.
async fn run_create_admin(
    config: PathBuf,
    login: &'static str,
    password_file: Option<PathBuf>,
    stdin_text: String,
) -> Output {
    tokio::task::spawn_blocking(move || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_recipe-server"));
        command
            .arg("create-admin")
            .args(["--login", login])
            .arg("--config")
            .arg(config)
            .env_remove("RUST_LOG");
        if let Some(file) = password_file {
            command.arg("--password-file").arg(file);
        }
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        // The child may exit before it reads stdin, for example with a password file.
        let _ = stdin.write_all(stdin_text.as_bytes());
        drop(stdin);
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap()
}

/// No run may print a password, a hash, or a line of a rejected file.
fn assert_no_secret(output: &Output) {
    let text = text_of(output);
    for needle in [
        GOOD_PASSWORD,
        SHORT_PASSWORD,
        APP_PASSWORD,
        MIGRATOR_PASSWORD,
        ARGON2_MARKER,
        SECOND_LINE,
    ] {
        assert!(!text.contains(needle), "output holds {needle}: {text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn create_admin_creates_one_administrator_and_refuses_bad_passwords() {
    let container = start_container(LABEL_VALUE).await;
    wait_until_ready(&container).await;
    let host_port = host_port(&container).await;
    let dir = TempDir::new();
    let migrator = dir.write("migrator_secret", &format!("{MIGRATOR_PASSWORD}\n"));
    let app = dir.write("app_secret", &format!("{APP_PASSWORD}\n"));
    let config = write_config(&dir, host_port, &migrator, &app);
    let migrated = run_migrate(config.clone()).await;
    assert_eq!(migrated.status.code(), Some(0), "{}", text_of(&migrated));

    // A password of 11 characters gives E-IDN-006, exit 1, and no user.
    let short = run_create_admin(
        config.clone(),
        "boss.one",
        None,
        format!("{SHORT_PASSWORD}\n"),
    )
    .await;
    let text = text_of(&short);
    assert_eq!(short.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&short, "ERROR"), 1, "{text}");
    assert!(text.contains("E-IDN-006"), "{text}");
    assert_no_secret(&short);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "0"
    );

    // A password file with two lines gives exit 1, and the output holds no line of the file.
    let two_lines = dir.write("two_lines", &format!("{GOOD_PASSWORD}\n{SECOND_LINE}\n"));
    let rejected =
        run_create_admin(config.clone(), "boss.one", Some(two_lines), String::new()).await;
    let text = text_of(&rejected);
    assert_eq!(rejected.status.code(), Some(1), "{text}");
    assert_eq!(lines_with(&rejected, "ERROR"), 1, "{text}");
    assert_no_secret(&rejected);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "0"
    );

    // A first run with a valid password on stdin creates the Administrator.
    let first = run_create_admin(
        config.clone(),
        "boss.one",
        None,
        format!("{GOOD_PASSWORD}\n"),
    )
    .await;
    let text = text_of(&first);
    assert_eq!(first.status.code(), Some(0), "{text}");
    assert_eq!(lines_with(&first, "INFO"), 1, "{text}");
    assert_eq!(lines_with(&first, "ERROR"), 0, "{text}");
    assert!(text.contains("boss.one"), "{text}");
    assert_no_secret(&first);
    assert_eq!(
        query(
            &container,
            "SELECT login || ',' || role || ',' || must_change_password FROM app_user"
        )
        .await,
        "boss.one,administrator,true"
    );
    assert_eq!(
        query(
            &container,
            "SELECT count(*) || ',' || count(actor_user_id) FROM audit_log WHERE action = 'UserCreated'"
        )
        .await,
        "1,0"
    );

    // A second run with another login writes one INFO line, exits 0, and creates nothing.
    let second = run_create_admin(config, "boss.two", None, format!("{GOOD_PASSWORD}\n")).await;
    let text = text_of(&second);
    assert_eq!(second.status.code(), Some(0), "{text}");
    assert_eq!(lines_with(&second, "INFO"), 1, "{text}");
    assert_eq!(lines_with(&second, "ERROR"), 0, "{text}");
    assert_no_secret(&second);
    assert_eq!(
        query(&container, "SELECT count(*) FROM app_user").await,
        "1"
    );
    assert_eq!(
        query(
            &container,
            "SELECT count(*) FROM audit_log WHERE action = 'UserCreated'"
        )
        .await,
        "1"
    );
}
