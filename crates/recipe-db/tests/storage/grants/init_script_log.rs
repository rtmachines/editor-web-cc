//! The server log of the init script holds no password (ADR-0123, design D1 items 3 to 6).
//!
//! A fresh container logs every statement with its duration. `01-roles.sh` must turn that log
//! off for its own session before the first statement with a password.

use crate::support::{Role, init_log};

/// The two forms in which a `CREATE ROLE` or `ALTER ROLE` statement writes a password literal.
const PASSWORD_LITERALS: [&str; 2] = ["PASSWORD '", "PASSWORD E'"];

/// The statement of `01-roles.sh` that runs in the second session of the init script. That
/// session does not turn the duration log off, so the log of the init server holds its duration.
const DURATION_PROBE_STATEMENT: &str = "CREATE SCHEMA audit_guard";

/// Whether the log of the init server holds the duration line of the probe statement. Another
/// line with `duration:` does not count, because the final server also logs durations.
fn init_server_logs_durations(init_server_log: &str) -> bool {
    init_server_log
        .lines()
        .any(|line| line.contains("duration:") && line.contains(DURATION_PROBE_STATEMENT))
}

/// Group the log into entries. An entry is a line that starts at column 0 and its continuation
/// lines, which start with a tab or a space. PostgreSQL indents the lines of a long statement.
fn entries(log: &str) -> Vec<String> {
    let mut entries: Vec<String> = Vec::new();
    for line in log.lines() {
        let continues = line.starts_with(['\t', ' ']);
        match entries.last_mut() {
            Some(last) if continues => {
                last.push('\n');
                last.push_str(line);
            }
            _ => entries.push(line.to_owned()),
        }
    }
    entries
}

/// The names of the roles whose statement holds a password literal. The value never leaves
/// this function.
fn roles_with_password_literal(log: &str) -> Vec<&'static str> {
    let entries = entries(log);
    let mut names: Vec<&'static str> = Role::WITH_SECRET_FILE
        .iter()
        .map(|role| role.name())
        .filter(|name| {
            entries.iter().any(|entry| {
                entry.contains(name) && PASSWORD_LITERALS.iter().any(|token| entry.contains(token))
            })
        })
        .collect();
    names.sort_unstable();
    names
}

/// The names of the roles whose test password appears in the log.
fn roles_with_test_password(log: &str) -> Vec<&'static str> {
    Role::WITH_SECRET_FILE
        .iter()
        .filter(|role| log.contains(role.password()))
        .map(|role| role.name())
        .collect()
}

#[tokio::test]
async fn the_init_script_log_holds_no_password_of_a_role() {
    let log = init_log::read_init_log()
        .await
        .expect("the container starts and its log is readable");

    assert_eq!(
        log.roles,
        init_log::expected_roles(),
        "the init created the five roles"
    );
    assert!(
        init_server_logs_durations(&log.init_text),
        "the server option is live, so the log of the init server holds the duration of          `{DURATION_PROBE_STATEMENT}`"
    );
    assert!(
        !PASSWORD_LITERALS
            .iter()
            .any(|token| log.text.contains(token)),
        "the log holds a password literal in the statement of the roles {:?}",
        roles_with_password_literal(&log.text)
    );
    assert!(
        roles_with_test_password(&log.text).is_empty(),
        "the log holds the test password of the roles {:?}",
        roles_with_test_password(&log.text)
    );
}

#[cfg(test)]
mod probe_tests {
    use super::*;

    const PROBE_LINE: &str = "LOG:  duration: 0.412 ms  statement: CREATE SCHEMA audit_guard;";

    #[test]
    fn the_probe_line_of_the_init_server_proves_the_option_is_live() {
        let log = format!(
            "LOG:  starting
{PROBE_LINE}
LOG:  ready"
        );

        assert!(init_server_logs_durations(&log));
    }

    #[test]
    fn a_duration_line_of_another_statement_does_not_prove_the_option_is_live() {
        let log = "LOG:  duration: 0.1 ms  statement: SELECT 1;
LOG:  ready";

        assert!(!init_server_logs_durations(log));
    }

    #[test]
    fn a_log_of_the_init_server_without_durations_is_not_live() {
        let log = format!(
            "LOG:  statement: CREATE SCHEMA audit_guard;
{}",
            "LOG:  ready"
        );

        assert!(!init_server_logs_durations(&log));
    }

    #[test]
    fn removing_the_duration_lines_turns_the_check_red() {
        let log = format!(
            "LOG:  starting
{PROBE_LINE}
LOG:  ready"
        );
        let without_durations: String = log
            .lines()
            .filter(|line| !line.contains("duration:"))
            .collect::<Vec<_>>()
            .join(
                "
",
            );

        assert!(init_server_logs_durations(&log));
        assert!(!init_server_logs_durations(&without_durations));
    }
}
