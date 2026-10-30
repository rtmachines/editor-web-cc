//! Tests of `deploy/pgtools/restore-roles.sql` and `deploy/pgtools/restore-roles.expected`
//! (ADR-0103 (m), design D5 of the change `restore-volume-switch`).
//!
//! The query prints one line for each client rule, database access item, role, membership, role
//! setting, and grant of a cluster. The file holds the lines of the release, with the placeholder
//! `@COMPOSE_SUBNET@` in the client rules. The format tests read the file only. The database test
//! runs the query on the harness cluster after `01-roles.sh` and every migration.

use std::collections::BTreeSet;
use std::process::Command;

use sqlx::Row as _;

use crate::support::{Role, TestDatabase, server};

const QUERY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/pgtools/restore-roles.sql"
);
const EXPECTED_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/pgtools/restore-roles.expected"
);
const GRANTS: &str = include_str!("../grants.expected");
const PLACEHOLDER: &str = "@COMPOSE_SUBNET@";
/// The subnet of the harness (`support/container.rs`). `01-roles.sh` writes it into the file.
const HARNESS_SUBNET: &str = "172.28.0.0/16";
/// Two subnets with a prefix length other than 16, for the padding of `01-roles.sh`.
const OTHER_SUBNETS: [&str; 2] = ["10.0.0.0/8", "192.168.100.0/24"];
/// The shared file of the client rules in the harness container (ADR-0103 (l)).
const CLUSTER_ACCESS: &str = "/etc/recipe-db/cluster-access.sh";
/// The two lines of `support/02-test-hosts.sh` in the form of the query. The harness connects
/// from the Docker host, so it needs them. No production cluster has them.
const HARNESS_HBA_LINES: [&str; 2] = [
    "HBA|host all all all scram-sha-256",
    "HBA|host replication recipe_replicator all scram-sha-256",
];
/// The kinds in the order of the file. The order is the byte order of the kind names.
const KINDS: [&str; 6] = ["ACL", "GRANT", "HBA", "MEMBER", "ROLE", "SETTING"];
/// The number of fields of a line, with the kind as the first field.
const FIELD_COUNTS: [(&str, usize); 6] = [
    ("ACL", 6),
    ("GRANT", 3),
    ("HBA", 2),
    ("MEMBER", 7),
    ("ROLE", 11),
    ("SETTING", 4),
];
/// The index of the field `rolconnlimit` in a `ROLE` line. It is a number and not an OID.
const CONNLIMIT_FIELD: usize = 9;
const SCRAM_MARKER: &str = "SCRAM-SHA-256$";
/// Lines that must exist, to show that the file is not a sample.
const WITNESS_PREFIXES: [&str; 6] = [
    "ACL|recipe|",
    "GRANT|",
    "HBA|local all postgres peer",
    "MEMBER|audit_owner|recipe_migrator|",
    "ROLE|recipe_app|",
    "SETTING|",
];

fn read(path: &str) -> String {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))
        .unwrap()
}

/// The non-empty lines of `restore-roles.expected`, in file order.
fn expected_lines() -> Vec<String> {
    read(EXPECTED_PATH)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn kind_of(line: &str) -> &str {
    line.split('|').next().unwrap_or_default()
}

/// Check that a line holds no password hash and no OID. The error names the first fault.
fn check_line_content(line: &str) -> Result<(), String> {
    if line.contains(SCRAM_MARKER) {
        return Err("a password hash".to_owned());
    }
    let fields: Vec<&str> = line.split('|').collect();
    for (index, field) in fields.iter().enumerate() {
        let is_connlimit = fields.first() == Some(&"ROLE") && index == CONNLIMIT_FIELD;
        let is_number = !field.is_empty() && field.bytes().all(|byte| byte.is_ascii_digit());
        if is_number && !is_connlimit {
            return Err(format!("the number {field}, maybe an OID"));
        }
    }
    Ok(())
}

/// Replace the placeholder of the file with a subnet, as `restore-volume` does.
fn with_subnet(lines: &[String], subnet: &str) -> Vec<String> {
    lines
        .iter()
        .map(|line| line.replace(PLACEHOLDER, subnet))
        .collect()
}

/// The rules of a `pg_hba.conf` text as the query prints them: no comment, no blank line, and the
/// fields of a rule joined by one space.
fn normalized_rules(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|line| line.split_whitespace().collect::<Vec<_>>().join(" "))
        .collect()
}

/// Run `write_hba_conf` of the shared file in the harness container and return the file text.
fn hba_conf_written_for(subnet: &str) -> String {
    let program = format!(
        "source {CLUSTER_ACCESS} && write_hba_conf {subnet} /tmp/restore-roles-hba.conf \
         && cat /tmp/restore-roles-hba.conf"
    );
    let output = Command::new("docker")
        .args(["exec", &server().container_id, "bash", "-c", &program])
        .output()
        .expect("docker exec runs");
    assert!(
        output.status.success(),
        "write_hba_conf failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Run the query file on a copy of the harness database and return its lines.
async fn query_lines() -> Vec<String> {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;
    let query = read(QUERY_PATH);
    let rows = sqlx::raw_sql(&query).fetch_all(&mut conn).await.unwrap();
    rows.iter()
        .map(|row| row.try_get::<String, _>(0).unwrap())
        .collect()
}

#[test]
fn every_line_has_a_known_kind_and_the_columns_of_that_kind() {
    let lines = expected_lines();

    assert!(!lines.is_empty(), "restore-roles.expected has no line");
    for line in &lines {
        let kind = kind_of(line);
        let count = FIELD_COUNTS
            .iter()
            .find(|(name, _)| *name == kind)
            .map(|(_, count)| *count);
        assert!(count.is_some(), "unknown kind in {line}");
        assert_eq!(line.split('|').count(), count.unwrap(), "columns of {line}");
    }
}

#[test]
fn the_file_holds_a_line_of_every_kind() {
    let lines = expected_lines();

    for prefix in WITNESS_PREFIXES {
        assert!(
            lines.iter().any(|line| line.starts_with(prefix)),
            "restore-roles.expected has no line that starts with {prefix}"
        );
    }
    for kind in KINDS {
        let prefix = format!("{kind}|");
        assert!(lines.iter().any(|line| line.starts_with(&prefix)), "{kind}");
    }
}

#[test]
fn the_kinds_follow_the_fixed_order_and_the_lines_of_a_kind_are_sorted() {
    let lines = expected_lines();

    let kinds: Vec<&str> = lines.iter().map(|line| kind_of(line)).collect();
    assert!(
        kinds.windows(2).all(|pair| pair[0] <= pair[1]),
        "the kinds are not in byte order"
    );
    // The client rules keep the order of the file, because the first matching rule wins.
    let unordered = lines.windows(2).find(|pair| {
        let kind = kind_of(&pair[0]);
        kind == kind_of(&pair[1]) && kind != "HBA" && pair[0] >= pair[1]
    });
    assert!(
        unordered.is_none(),
        "not sorted or duplicated: {unordered:?}"
    );
}

#[test]
fn the_placeholder_stands_in_client_rules_only() {
    let lines = expected_lines();

    let with_placeholder: Vec<&String> = lines
        .iter()
        .filter(|line| line.contains(PLACEHOLDER))
        .collect();

    assert!(
        !with_placeholder.is_empty(),
        "no line holds the placeholder"
    );
    for line in &with_placeholder {
        assert_eq!(kind_of(line), "HBA", "placeholder outside HBA: {line}");
        let rule = line.strip_prefix("HBA|").unwrap();
        assert!(rule.split(' ').any(|field| field == PLACEHOLDER), "{line}");
    }
}

#[test]
fn the_file_holds_no_password_hash_and_no_oid() {
    for line in &expected_lines() {
        let verdict = check_line_content(line);
        assert!(verdict.is_ok(), "bad line {line}: {:?}", verdict.err());
    }
}

#[test]
fn the_content_check_rejects_a_hash_and_a_number() {
    let hash = "ROLE|recipe_app|SCRAM-SHA-256$4096:salt$key:key|false";
    let oid = "MEMBER|16401|recipe_app|postgres|false|true|true";
    let limit = "ROLE|recipe_app|false|true|false|false|true|false|false|-1|-";

    assert!(check_line_content(hash).is_err());
    assert!(check_line_content(oid).is_err());
    assert!(check_line_content(limit).is_ok());
}

#[test]
fn the_query_never_reads_the_password_column() {
    let query = read(QUERY_PATH);

    assert!(!query.contains("rolpassword"), "the query names the column");
    assert!(!query.contains("pg_shadow"), "the query reads pg_shadow");
}

#[test]
fn the_normalization_ignores_the_padding_of_the_subnet() {
    let padded =
        "# TYPE\nhost    recipe       recipe_app         10.0.0.0/8       scram-sha-256\n\n";
    let plain = "host recipe recipe_app 10.0.0.0/8 scram-sha-256";

    assert_eq!(normalized_rules(padded), vec![plain.to_owned()]);
}

#[test]
fn the_client_rules_equal_write_hba_conf_for_other_subnets() {
    let rules: Vec<String> = expected_lines()
        .into_iter()
        .filter_map(|line| line.strip_prefix("HBA|").map(str::to_owned))
        .collect();
    assert!(!rules.is_empty(), "the file has no client rule");

    for subnet in OTHER_SUBNETS.into_iter().chain([HARNESS_SUBNET]) {
        let written = hba_conf_written_for(subnet);

        let expected = with_subnet(&rules, subnet);

        assert_eq!(
            normalized_rules(&written),
            expected,
            "the rules differ for the subnet {subnet}"
        );
    }
}

#[tokio::test]
async fn the_query_gives_exactly_the_lines_of_the_file() {
    let actual = query_lines().await;

    for line in &actual {
        let verdict = check_line_content(line);
        assert!(verdict.is_ok(), "bad line {line}: {:?}", verdict.err());
    }
    for harness_line in HARNESS_HBA_LINES {
        assert!(
            actual.iter().any(|line| line == harness_line),
            "the harness cluster has no line {harness_line}"
        );
    }
    let production: Vec<String> = actual
        .into_iter()
        .filter(|line| !HARNESS_HBA_LINES.contains(&line.as_str()))
        .collect();
    let expected = with_subnet(&expected_lines(), HARNESS_SUBNET);
    let missing: Vec<&String> = expected
        .iter()
        .filter(|l| !production.contains(l))
        .collect();
    let extra: Vec<&String> = production
        .iter()
        .filter(|l| !expected.contains(l))
        .collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "restore-roles.expected differs from the cluster.\nmissing:\n{missing:#?}\nextra:\n{extra:#?}"
    );
    assert_eq!(
        production, expected,
        "the order of the query is the file order"
    );
}

#[tokio::test]
async fn the_grant_lines_equal_the_grant_file() {
    let lines = query_lines().await;

    let actual: BTreeSet<String> = lines
        .iter()
        .filter_map(|line| line.strip_prefix("GRANT|").map(str::to_owned))
        .collect();
    let expected: BTreeSet<String> = GRANTS
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    let missing: Vec<&String> = expected.difference(&actual).collect();
    let extra: Vec<&String> = actual.difference(&expected).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "grants.expected differs from the GRANT lines.\nmissing:\n{missing:#?}\nextra:\n{extra:#?}"
    );
}
