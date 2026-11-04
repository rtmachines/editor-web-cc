//! Script tests of `deploy/db/initdb/01-roles.sh` (ADR-0031, design D17).
//!
//! Runtime tests run the script inside the harness container with `docker exec`. The script reads
//! its secrets from `SECRET_DIR` and checks every input before it starts `psql`. `PGHOST` points
//! to a directory without a server, so a script that skips a check ends with another exit code.

use std::process::Command;

use crate::support::{document_sql, server};

const SCRIPT: &str = "/docker-entrypoint-initdb.d/01-roles.sh";
const VALID_SUBNET: &str = "172.28.0.0/16";
/// The name that selects no secret file, so every secret file is good.
const NO_BAD_SECRET: &str = "none";
const EXIT_CHECK_FAILED: i32 = 1;
/// The shell program of one run. It builds a secret directory and an empty `pg_hba.conf`, runs
/// the script, and prints the size of `pg_hba.conf`. The arguments are the work directory, the
/// secret that gets the test content, its content in `printf %b` form, and the subnet.
const PROGRAM: &str = r#"
work=$1; bad=$2; content=$3; subnet=$4
rm -rf "$work"; mkdir -p "$work/secrets" "$work/pgdata"
: > "$work/pgdata/pg_hba.conf"
for name in migrator app backup replicator; do
  if [ "$name" = "$bad" ]; then printf '%b' "$content" > "$work/secrets/${name}_password"
  else printf 'good-password\n' > "$work/secrets/${name}_password"
  fi
done
SECRET_DIR="$work/secrets" PGDATA="$work/pgdata" POSTGRES_USER=postgres \
  PGHOST=/nonexistent-socket-dir COMPOSE_SUBNET="$subnet" bash "$0"
code=$?
echo "HBA_BYTES=$(wc -c < "$work/pgdata/pg_hba.conf")"
exit $code
"#;

/// The result of one script run.
#[derive(Debug)]
struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Run the script in the container with one bad input.
fn run_script(case: &str, bad_secret: &str, content: &str, subnet: &str) -> Run {
    let work = format!("/tmp/init-script-{case}");
    let output = Command::new("docker")
        .args([
            "exec",
            &server().container_id,
            "bash",
            "-c",
            PROGRAM,
            SCRIPT,
        ])
        .args([work.as_str(), bad_secret, content, subnet])
        .output()
        .expect("docker exec runs");
    Run {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// Assert that the script stopped on its own check and changed nothing.
fn assert_stopped(run: &Run) {
    assert_eq!(run.code, EXIT_CHECK_FAILED, "{run:?}");
    assert!(run.stderr.contains("01-roles.sh:"), "{run:?}");
    assert!(run.stdout.contains("HBA_BYTES=0"), "{run:?}");
}

#[test]
fn the_script_stops_on_an_empty_secret_file() {
    assert_stopped(&run_script("empty", "migrator", "", VALID_SUBNET));
}

#[test]
fn the_script_stops_on_a_secret_file_with_only_a_newline() {
    assert_stopped(&run_script("newline", "app", "\\n", VALID_SUBNET));
}

#[test]
fn the_script_stops_on_a_secret_file_with_a_carriage_return() {
    assert_stopped(&run_script(
        "carriage",
        "backup",
        "password\\r\\n",
        VALID_SUBNET,
    ));
}

#[test]
fn the_script_stops_on_a_secret_file_with_a_second_line() {
    assert_stopped(&run_script(
        "second-line",
        "replicator",
        "one\\ntwo\\n",
        VALID_SUBNET,
    ));
}

#[test]
fn the_script_stops_when_the_subnet_is_not_an_ipv4_cidr() {
    for (index, subnet) in [
        "",
        "172.28.0.0",
        "172.28.0.0/33",
        "999.1.1.1/16",
        "::1/128",
        "0.0.0.0/0",
        "172.010.0.0/16",
        "172.28.0.0/016",
    ]
    .iter()
    .enumerate()
    {
        let run = run_script(&format!("subnet-{index}"), NO_BAD_SECRET, "", subnet);

        assert_stopped(&run);
    }
}

/// The text of the init script, to check what it holds and what it leaves to the shared file.
fn script_text() -> String {
    std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../deploy/db/initdb/01-roles.sh"
    ))
    .expect("01-roles.sh is readable")
}

#[test]
fn the_script_takes_the_client_rules_from_the_shared_file() {
    // ADR-0103 (l): the shared file holds the hba lines and the default database REVOKEs.
    let text = script_text();

    assert!(
        text.lines()
            .any(|line| line == "source /etc/recipe-db/cluster-access.sh"),
        "the script sources the shared file at its fixed path"
    );
    assert!(
        !text
            .lines()
            .any(|line| line.trim_start().starts_with("printf 'host")),
        "the script holds no hba line"
    );
    assert!(
        !text.lines().any(|line| line.contains("REVOKE")
            && (line.contains("postgres") || line.contains("template1"))),
        "the script holds no REVOKE on a default database"
    );
}

/// The five `SET` statements that keep a password out of the server log (ADR-0031, ADR-0123).
const LOG_GUARDS: [&str; 5] = [
    "SET log_statement = 'none';",
    "SET log_min_error_statement = 'panic';",
    "SET log_min_duration_statement = -1;",
    "SET log_transaction_sample_rate = 0;",
    "SET log_min_duration_sample = -1;",
];
/// The token that starts the password of a `CREATE ROLE` statement. The match uses case, so the
/// `\set ..._password` lines and the comments do not match.
const PASSWORD_TOKEN: &str = "PASSWORD :'";

/// The 1-based line numbers of the lines for which `matches` is true.
fn line_numbers(text: &str, matches: impl Fn(&str) -> bool) -> Vec<usize> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| matches(line))
        .map(|(index, _)| index.saturating_add(1))
        .collect()
}

#[test]
fn every_password_line_comes_after_the_five_log_guards_and_before_the_first_connect() {
    // ADR-0123 (a): a password literal reaches the log through a slow statement, unless the
    // session turns duration and sampling logs off before the first `CREATE ROLE`.
    let text = script_text();

    let guard_lines: Vec<usize> = LOG_GUARDS
        .iter()
        .map(|guard| {
            let found = line_numbers(&text, |line| line.trim() == *guard);
            assert_eq!(found.len(), 1, "01-roles.sh holds exactly one `{guard}`");
            found[0]
        })
        .collect();
    let last_guard = guard_lines.iter().copied().max().unwrap_or_default();
    let connect_lines = line_numbers(&text, |line| line.starts_with(r"\connect"));
    let first_connect = *connect_lines
        .first()
        .expect("01-roles.sh has a connect line");
    let password_lines = line_numbers(&text, |line| line.contains(PASSWORD_TOKEN));

    assert!(!password_lines.is_empty(), "01-roles.sh has password lines");
    for line in password_lines {
        assert!(
            last_guard < line && line < first_connect,
            "password line {line} must come after line {last_guard} and before line {first_connect}"
        );
    }
}

#[test]
fn the_document_block_of_the_roles_holds_the_five_log_guards() {
    let block = document_sql(&["5.1"], &[]);

    for guard in LOG_GUARDS {
        assert!(
            block.contains(guard),
            "data-model.md section 5.1 holds `{guard}`"
        );
    }
}
