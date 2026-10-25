#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Checks that the environment of a process reaches `serve::load` (ADR-0062, design D7).
//!
//! The subcommand `serve` does not start in this build, and `std::env::set_var` is unsafe
//! (edition 2024). So the test runs its own test binary as a child process. The parent delivers
//! the variables with `Command::env`. The child calls the real `serve::load()` and writes the
//! canonical pairs to a file. The parent compares that file with the expected texts.

use std::hash::{BuildHasher, Hasher, RandomState};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

mod bounded_run;
use bounded_run::run_bounded;

/// The variable that tells the child where to write. Without it, the child test does nothing.
const OUTPUT_VARIABLE: &str = "SERVE_ENV_TEST_OUTPUT";
const CHILD_TEST: &str = "child_loads_the_environment";
const CHILD_LIMIT: Duration = Duration::from_secs(60);
const HEX_WORDS: usize = 2;
const KEY_A: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const KEY_B: &str = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";
const WEBHOOK: &str = "https://hooks.plant.example:8443/alert/tokenvalue0123456789";

/// A temporary directory that the drop removes.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        let random: String = (0..HEX_WORDS)
            .map(|_| format!("{:016x}", RandomState::new().build_hasher().finish()))
            .collect();
        let path = std::env::temp_dir().join(format!(
            "recipe-server-serve-env-{}-{random}",
            std::process::id()
        ));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }

    fn write(&self, name: &str, text: &str) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        // A leftover directory in the temporary folder does no harm.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The child half. It runs the real loader on the environment that the parent delivered.
#[test]
fn child_loads_the_environment() {
    let Some(output) = std::env::var_os(OUTPUT_VARIABLE) else {
        return;
    };
    let config = recipe_server::config::serve::load().unwrap();
    let pairs = config.configured().canonical().unwrap();
    let text: String = pairs
        .iter()
        .map(|(key, value)| format!("{}={}\n", key.as_str(), value.as_str()))
        .collect();
    std::fs::write(output, text).unwrap();
}

const PORT_CHILD_TEST: &str = "child_loads_the_ports";
const PUBLIC_PORT_VARIABLE: &str = "RECIPE_HTTP__PUBLIC_PORT";
const INTERNAL_PORT_VARIABLE: &str = "RECIPE_HTTP__INTERNAL_PORT";
const ACCEPTED_PORT_TEXTS: [&str; 3] = ["1", "9000", "65535"];
const REFUSED_PORT_TEXTS: [&str; 7] = ["0", " 9000", "9000 ", "+9000", "", "70000", "abc"];

/// The child half of the port tests. It writes `ok <public> <internal>` or `err <refusal>`.
#[test]
fn child_loads_the_ports() {
    let Some(output) = std::env::var_os(OUTPUT_VARIABLE) else {
        return;
    };
    let line = match recipe_server::config::serve::load() {
        Ok(config) => {
            let pairs = config.configured().canonical().unwrap();
            let port = |key: &str| {
                let (_key, value) = pairs.iter().find(|(k, _)| k.as_str() == key).unwrap();
                value.as_str().to_owned()
            };
            format!(
                "ok {} {}",
                port("http.public_port"),
                port("http.internal_port")
            )
        }
        Err(fault) => format!("err {fault:#}"),
    };
    std::fs::write(output, line).unwrap();
}

/// A scratch directory with the two key files that `serve::load` reads.
struct PortRun {
    dir: TempDir,
    login: PathBuf,
    csrf: PathBuf,
}

impl PortRun {
    fn new() -> Self {
        let dir = TempDir::new();
        let login = dir.write("login_key", &format!("{KEY_A}\n"));
        let csrf = dir.write("csrf_key", &format!("{KEY_B}\n"));
        Self { dir, login, csrf }
    }

    /// Run the child with the one variable `variable` set to `value`. Return its line.
    fn outcome(&self, variable: &str, value: &str) -> String {
        let output = self.dir.0.join("ports.txt");
        if output.exists() {
            std::fs::remove_file(&output).unwrap();
        }
        let variables = [
            (
                "RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE",
                self.login.to_str().unwrap().to_owned(),
            ),
            (
                "RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE",
                self.csrf.to_str().unwrap().to_owned(),
            ),
            (variable, value.to_owned()),
        ];
        let result = run_bounded(
            child_command(PORT_CHILD_TEST, &output, &variables),
            CHILD_LIMIT,
        );
        let log = String::from_utf8_lossy(&result.stderr).into_owned();
        assert!(result.status.success(), "{variable}={value:?}: {log}");
        let line = std::fs::read_to_string(&output).unwrap_or_default();
        assert!(!line.is_empty(), "{variable}={value:?}: no line: {log}");
        line
    }
}

#[test]
fn serve_load_refuses_a_bad_port_text_of_the_real_environment_and_names_the_key_only() {
    let run = PortRun::new();
    for variable in [PUBLIC_PORT_VARIABLE, INTERNAL_PORT_VARIABLE] {
        let key = variable
            .trim_start_matches("RECIPE_HTTP__")
            .to_ascii_lowercase();
        for text in REFUSED_PORT_TEXTS {
            let line = run.outcome(variable, text);
            let case = format!("{variable}={text:?}: {line}");
            assert!(line.starts_with("err "), "{case}");
            assert!(line.to_ascii_lowercase().contains(&key), "{case}");
            let value = text.trim();
            assert!(value.len() < 3 || !line.contains(value), "{case}");
        }
    }
}

#[test]
fn serve_load_accepts_the_port_bounds_of_the_real_environment() {
    let run = PortRun::new();
    for text in ACCEPTED_PORT_TEXTS {
        let line = run.outcome(PUBLIC_PORT_VARIABLE, text);
        assert_eq!(line, format!("ok {text} 9000"), "public {text:?}");
        let line = run.outcome(INTERNAL_PORT_VARIABLE, text);
        assert_eq!(line, format!("ok 8080 {text}"), "internal {text:?}");
    }
}

#[test]
fn serve_load_and_healthcheck_agree_on_every_internal_port_text() {
    let run = PortRun::new();
    for text in ACCEPTED_PORT_TEXTS.into_iter().chain(REFUSED_PORT_TEXTS) {
        let loader_accepts = run.outcome(INTERNAL_PORT_VARIABLE, text).starts_with("ok ");
        let probe_accepts = recipe_server::probe::internal_port_from(Some(text)).is_ok();
        assert_eq!(loader_accepts, probe_accepts, "internal port text {text:?}");
    }
}

/// The build decides: only the feature `dev` accepts `session.secure=false` (ADR-0062 (b)).
///
/// The test runs the real `load()` in the child. A `load()` that passes a constant flag to
/// `load_with` breaks one of the two builds: `--all-features` or `--no-default-features`.
#[test]
fn serve_load_accepts_an_insecure_session_only_in_a_build_with_the_feature_dev() {
    let line = PortRun::new().outcome("RECIPE_SESSION__SECURE", "false");
    let accepted = line.starts_with("ok ");
    assert_eq!(accepted, cfg!(feature = "dev"), "{line}");
    if !accepted {
        assert!(line.starts_with("err "), "{line}");
        assert!(line.contains("session.secure"), "{line}");
    }
}

/// Return the text of the path that `files` names `name`.
fn path_of(files: &[(&str, &Path)], name: &str) -> String {
    let (_name, path) = files.iter().find(|(file, _)| *file == name).unwrap();
    path.to_str().unwrap().to_owned()
}

/// The 17 environment variables, each with a value that differs from its default.
fn environment(files: &[(&str, &Path)]) -> Vec<(&'static str, String)> {
    let path = |name: &str| path_of(files, name);
    vec![
        ("RECIPE_DB__HOST", "database".to_owned()),
        ("RECIPE_DB__PORT", "6543".to_owned()),
        ("RECIPE_DB__NAME", "plant".to_owned()),
        ("RECIPE_DB__APP_USER", "app_x".to_owned()),
        ("RECIPE_SECRETS__APP_PASSWORD_FILE", "/s/app".to_owned()),
        ("RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE", path("login")),
        ("RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE", path("csrf")),
        (
            "RECIPE_HTTP__TRUSTED_PROXIES",
            "10.0.0.0/8,172.28.0.2/32".to_owned(),
        ),
        ("RECIPE_HTTP__PUBLIC_PORT", "8081".to_owned()),
        ("RECIPE_HTTP__INTERNAL_PORT", "9001".to_owned()),
        ("RECIPE_APPROVAL__SEPARATION_OF_DUTIES", "false".to_owned()),
        ("RECIPE_ALERTS__WEBHOOK_URL_FILE", path("hook")),
        ("RECIPE_ALERTS__WEBHOOK_CA_FILE", path("ca")),
        ("RECIPE_SESSION__SECURE", "true".to_owned()),
        ("RECIPE_SESSION__IDLE_TIMEOUT", "600".to_owned()),
        ("RECIPE_SESSION__ABSOLUTE_TIMEOUT", "7200".to_owned()),
        ("RECIPE_I18N__TRANSLATION_LANGS", "pl,de".to_owned()),
    ]
}

/// Compare the start of `name` with `prefix` and ignore the case of ASCII letters.
fn starts_with_ignore_case(name: &str, prefix: &str) -> bool {
    name.get(..prefix.len())
        .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
}

/// Build the child command for the child test `test`. The host environment must not change it.
fn child_command(test: &str, output: &Path, variables: &[(&str, String)]) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command.args(["--exact", test, "--test-threads=1"]);
    for (name, _value) in std::env::vars_os() {
        let name_text = name.to_string_lossy();
        if starts_with_ignore_case(&name_text, "PG")
            || starts_with_ignore_case(&name_text, "RECIPE_")
        {
            command.env_remove(&name);
        }
    }
    command.env(OUTPUT_VARIABLE, output);
    for (name, value) in variables {
        command.env(name, value);
    }
    command
}

/// The 18 pairs that the child must write, in the table order of design D7.
fn expected_lines(files: &[(&str, &Path)]) -> Vec<String> {
    let path = |name: &str| path_of(files, name);
    let fingerprint = recipe_domain::setting::SettingValue::fingerprint(WEBHOOK);
    vec![
        "db.host=database".to_owned(),
        "db.port=6543".to_owned(),
        "db.name=plant".to_owned(),
        "db.app_user=app_x".to_owned(),
        "secrets.app_password_file=/s/app".to_owned(),
        format!("security.login_hmac_key_file={}", path("login")),
        format!("security.csrf_signing_key_file={}", path("csrf")),
        "http.trusted_proxies=10.0.0.0/8,172.28.0.2/32".to_owned(),
        "http.public_port=8081".to_owned(),
        "http.internal_port=9001".to_owned(),
        "approval.separation_of_duties=false".to_owned(),
        format!("alerts.webhook_url_file={}", path("hook")),
        format!("alerts.webhook_ca_file={}", path("ca")),
        format!("alerts.webhook_url={}", fingerprint.as_str()),
        "session.secure=true".to_owned(),
        "session.idle_timeout=600s".to_owned(),
        "session.absolute_timeout=7200s".to_owned(),
        "i18n.translation_langs=pl,de".to_owned(),
    ]
}

#[test]
fn serve_load_reads_all_17_names_with_the_prefix_and_the_double_underscore() {
    let dir = TempDir::new();
    let login = dir.write("login_key", &format!("{KEY_A}\n"));
    let csrf = dir.write("csrf_key", &format!("{KEY_B}\n"));
    let hook = dir.write("hook", &format!("{WEBHOOK}\n"));
    let ca = dir.write("ca", "certificate text");
    let output = dir.0.join("loaded.txt");
    let files: [(&str, &Path); 4] = [
        ("login", &login),
        ("csrf", &csrf),
        ("hook", &hook),
        ("ca", &ca),
    ];
    let mut variables = environment(&files);
    // Names with another prefix or one underscore must not reach the configuration.
    variables.push(("RECIPE_DB_HOST", "single-underscore".to_owned()));
    variables.push(("OTHER_DB__HOST", "other-prefix".to_owned()));
    variables.push(("DB__HOST", "no-prefix".to_owned()));

    let result = run_bounded(child_command(CHILD_TEST, &output, &variables), CHILD_LIMIT);

    let log = format!(
        "{}\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(result.status.success(), "{log}");
    let loaded = std::fs::read_to_string(&output).expect("the child wrote no output");
    let lines: Vec<String> = loaded.lines().map(str::to_owned).collect();
    assert_eq!(lines, expected_lines(&files), "{log}");
}
