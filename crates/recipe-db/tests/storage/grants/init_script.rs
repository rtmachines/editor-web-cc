//! Script tests of `deploy/db/initdb/01-roles.sh` (ADR-0031, design D17).
//!
//! Each test runs the script inside the harness container with `docker exec`. The script reads
//! its secrets from `SECRET_DIR` and checks every input before it starts `psql`. `PGHOST` points
//! to a directory without a server, so a script that skips a check ends with another exit code.

use std::process::Command;

use crate::support::server;

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
    for (index, subnet) in ["", "172.28.0.0", "172.28.0.0/33", "999.1.1.1/16", "::1/128"]
        .iter()
        .enumerate()
    {
        let run = run_script(&format!("subnet-{index}"), NO_BAD_SECRET, "", subnet);

        assert_stopped(&run);
    }
}
