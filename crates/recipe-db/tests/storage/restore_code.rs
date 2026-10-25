//! Tests of `deploy/pgtools/restore-code.sql` and `deploy/pgtools/restore-code.expected`
//! (ADR-0067 (i), (j), ADR-0076 (a), (b), and ADR-0078).
//!
//! The query builds one line for each function, `CHECK` constraint, generated column, trigger, and
//! event trigger. The file holds the lines of the release, as `tests/grants.expected` does for the
//! grants. The format tests need no database. The other tests run the query on the harness database
//! after `01-roles.sh` and every migration.

use std::collections::BTreeSet;
use std::process::Command;

use sqlx::{PgConnection, Row as _};

use crate::support::{Role, TestDatabase, server};

const QUERY_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/pgtools/restore-code.sql"
);
const EXPECTED_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/pgtools/restore-code.expected"
);
/// The kinds of a line, in byte order.
const KINDS: [&str; 5] = ["CHECK", "EVENT TRIGGER", "FUNCTION", "GENERATED", "TRIGGER"];
/// The kinds whose entries `pg_restore -l` names with the same words (ADR-0076 (b)).
const LISTED_KINDS: [&str; 3] = ["EVENT TRIGGER", "FUNCTION", "TRIGGER"];
const FIELD_COUNT: usize = 4;
const SHA256_HEX_LENGTH: usize = 64;
const DUMP_AND_LIST: &str = "pg_dump --format=custom --file=/tmp/restore-code.dump \"$1\" \
     && pg_restore --list /tmp/restore-code.dump; status=$?; rm -f /tmp/restore-code.dump; exit $status";

/// One parsed line of the file or of the query.
struct Line<'a> {
    kind: &'a str,
    name: &'a str,
}

fn read(path: &str) -> String {
    std::fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))
        .unwrap()
}

/// The non-empty lines of `restore-code.expected`, in file order.
fn expected_lines() -> Vec<String> {
    read(EXPECTED_PATH)
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Split a line into its four fields and check them. The error names the first fault.
fn parse_line(line: &str) -> Result<Line<'_>, String> {
    let fields: Vec<&str> = line.split('|').collect();
    if fields.len() != FIELD_COUNT {
        return Err(format!("{} fields instead of {FIELD_COUNT}", fields.len()));
    }
    let (kind, name, owner, hash) = (fields[0], fields[1], fields[2], fields[3]);
    if !KINDS.contains(&kind) {
        return Err(format!("unknown kind {kind}"));
    }
    if name.is_empty() || owner.is_empty() {
        return Err("empty name or owner".to_owned());
    }
    let is_hash = hash.len() == SHA256_HEX_LENGTH
        && hash
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte));
    if !is_hash {
        return Err(format!("{hash} is no lower-case SHA-256"));
    }
    Ok(Line { kind, name })
}

/// Run the query on one connection and return its lines.
async fn query_lines(conn: &mut PgConnection) -> Vec<String> {
    let query = read(QUERY_PATH);
    let rows = sqlx::raw_sql(&query).fetch_all(&mut *conn).await.unwrap();
    rows.iter()
        .map(|row| row.try_get::<String, _>(0).unwrap())
        .collect()
}

/// The lines that only one side holds. The text lists every line of both lists.
fn difference(expected: &[String], actual: &[String]) -> (Vec<String>, Vec<String>) {
    let want: BTreeSet<&String> = expected.iter().collect();
    let have: BTreeSet<&String> = actual.iter().collect();
    let missing = want.difference(&have).map(|line| (*line).clone()).collect();
    let extra = have.difference(&want).map(|line| (*line).clone()).collect();
    (missing, extra)
}

#[test]
fn every_line_has_four_fields_a_known_kind_and_a_lower_case_sha256() {
    let lines = expected_lines();

    assert!(!lines.is_empty(), "restore-code.expected has no line");
    for line in &lines {
        let parsed = parse_line(line);
        assert!(parsed.is_ok(), "bad line {line}: {:?}", parsed.err());
    }
}

#[test]
fn the_line_check_rejects_each_fault_of_the_format() {
    let hash = "0".repeat(SHA256_HEX_LENGTH);
    let upper = "A".repeat(SHA256_HEX_LENGTH);
    let cases = [
        format!("FUNCTION|public f()|owner|{}", &hash[1..]),
        format!("FUNCTION|public f()|owner|{upper}"),
        format!("ROUTINE|public f()|owner|{hash}"),
        format!("FUNCTION|public f()|{hash}"),
        format!("FUNCTION|public f()|owner|extra|{hash}"),
        format!("FUNCTION||owner|{hash}"),
        format!("FUNCTION|public f()||{hash}"),
    ];

    for case in &cases {
        assert!(parse_line(case).is_err(), "the check accepts {case}");
    }
    let text = format!("EVENT TRIGGER|- guard|postgres|{hash}");
    let good = parse_line(&text).unwrap();
    assert_eq!((good.kind, good.name), ("EVENT TRIGGER", "- guard"));
}

#[test]
fn lines_are_sorted_in_byte_order_without_duplicates() {
    let lines = expected_lines();

    let unordered = lines.windows(2).find(|pair| pair[0] >= pair[1]);

    assert!(
        unordered.is_none(),
        "not sorted or duplicated: {unordered:?}"
    );
}

#[test]
fn the_file_holds_a_line_of_every_kind() {
    let lines = expected_lines();

    for kind in KINDS {
        let prefix = format!("{kind}|");
        assert!(
            lines.iter().any(|line| line.starts_with(&prefix)),
            "restore-code.expected has no line of the kind {kind}"
        );
    }
}

#[tokio::test]
async fn the_query_gives_exactly_the_lines_of_the_file() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;
    let expected = expected_lines();

    let actual = query_lines(&mut conn).await;

    let (missing, extra) = difference(&expected, &actual);
    assert!(
        missing.is_empty() && extra.is_empty(),
        "restore-code.expected differs from the database.\nmissing:\n{}\nextra:\n{}",
        missing.join("\n"),
        extra.join("\n")
    );
    assert_eq!(actual, expected, "the order of the query is the file order");
}

#[tokio::test]
async fn a_changed_function_body_changes_exactly_one_line() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;
    let before = query_lines(&mut conn).await;

    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION public.set_updated_at() RETURNS trigger \
         LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$ \
         BEGIN NEW.updated_at := clock_timestamp(); RETURN NEW; END; $$",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    let after = query_lines(&mut conn).await;

    let (missing, extra) = difference(&before, &after);
    let prefix = "FUNCTION|public set_updated_at()|";
    assert_eq!(
        (missing.len(), extra.len()),
        (1, 1),
        "{missing:?} {extra:?}"
    );
    assert!(missing[0].starts_with(prefix), "{missing:?}");
    assert!(extra[0].starts_with(prefix), "{extra:?}");
}

#[tokio::test]
async fn an_added_trigger_gives_one_extra_line() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;
    let before = query_lines(&mut conn).await;

    sqlx::raw_sql(
        "CREATE TRIGGER planted BEFORE UPDATE ON public.machine \
         FOR EACH ROW EXECUTE FUNCTION public.set_updated_at()",
    )
    .execute(&mut conn)
    .await
    .unwrap();
    let after = query_lines(&mut conn).await;

    let (missing, extra) = difference(&before, &after);
    assert!(missing.is_empty(), "{missing:?}");
    assert_eq!(extra.len(), 1, "{extra:?}");
    assert!(
        extra[0].starts_with("TRIGGER|public machine planted|"),
        "{extra:?}"
    );
}

/// `pg_get_triggerdef` does not show the enabled state. The line of a trigger holds `tgenabled`
/// in its definition, so a disabled trigger changes its line and no other (ADR-0078 (a)).
#[tokio::test]
async fn a_disabled_trigger_changes_exactly_the_line_of_that_trigger() {
    let database = TestDatabase::create().await;
    let mut conn = database.connect(Role::Superuser).await;
    let before = query_lines(&mut conn).await;

    sqlx::raw_sql("ALTER TABLE public.machine DISABLE TRIGGER machine_set_updated_at")
        .execute(&mut conn)
        .await
        .unwrap();
    let after = query_lines(&mut conn).await;

    let (missing, extra) = difference(&before, &after);
    let prefix = "TRIGGER|public machine machine_set_updated_at|";
    assert_eq!(
        (missing.len(), extra.len()),
        (1, 1),
        "{missing:?} {extra:?}"
    );
    assert!(missing[0].starts_with(prefix), "{missing:?}");
    assert!(extra[0].starts_with(prefix), "{extra:?}");
}

/// The globals file of `backup-once` is `pg_dumpall --globals-only --no-role-passwords`. The gate of
/// `restore-test` accepts the pair `\restrict KEY` and `\unrestrict KEY` and no other backslash. This
/// test shows the layout of the real tool of the image: the pair is the first and the last line that
/// is no comment and no blank line, the keys are equal and alphanumeric, and no other line has a
/// backslash.
#[tokio::test]
async fn the_globals_file_of_pg_dumpall_has_only_the_restrict_pair_as_backslash_lines() {
    let _database = TestDatabase::create().await;
    let output = Command::new("docker")
        .args(["exec", "--user", "postgres", &server().container_id])
        .args(["pg_dumpall", "--globals-only", "--no-role-passwords"])
        .output()
        .expect("docker exec runs");
    assert!(
        output.status.success(),
        "pg_dumpall failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = String::from_utf8_lossy(&output.stdout).into_owned();

    let code: Vec<&str> = text
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with("--"))
        .collect();

    let open = code.first().unwrap().strip_prefix("\\restrict ").unwrap();
    let close = code.last().unwrap().strip_prefix("\\unrestrict ").unwrap();
    assert_eq!(open, close, "the keys differ");
    assert!(!open.is_empty() && open.bytes().all(|byte| byte.is_ascii_alphanumeric()));
    let backslash_lines = text.lines().filter(|line| line.contains('\\')).count();
    assert_eq!(backslash_lines, 2, "other lines hold a backslash:\n{text}");
}

/// The `kind|name` pairs of the entries `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER` of a
/// `pg_restore -l` listing. An entry is `id; table-oid oid KIND schema tag owner`.
fn listed_entries(listing: &str) -> BTreeSet<String> {
    let mut entries = BTreeSet::new();
    for line in listing.lines().filter(|line| !line.starts_with(';')) {
        let Some((_, rest)) = line.split_once("; ") else {
            continue;
        };
        let mut words = rest.splitn(3, ' ');
        let (_, _, tail) = (words.next(), words.next(), words.next().unwrap_or(""));
        for kind in LISTED_KINDS {
            if let Some(entry) = tail.strip_prefix(kind).and_then(|r| r.strip_prefix(' ')) {
                let (name, _owner) = entry.rsplit_once(' ').unwrap_or((entry, ""));
                entries.insert(format!("{kind}|{name}"));
            }
        }
    }
    entries
}

/// The `kind|name` pairs of the lines of the file, for the kinds that `pg_restore -l` lists.
fn expected_entries() -> BTreeSet<String> {
    expected_lines()
        .iter()
        .map(|line| parse_line(line).unwrap())
        .filter(|line| LISTED_KINDS.contains(&line.kind))
        .map(|line| format!("{}|{}", line.kind, line.name))
        .collect()
}

#[tokio::test]
async fn the_names_of_the_file_are_the_entries_of_pg_restore_list() {
    let database = TestDatabase::create().await;
    let output = Command::new("docker")
        .args(["exec", "--user", "postgres", &server().container_id])
        .args(["bash", "-c", DUMP_AND_LIST, "list", database.name()])
        .output()
        .expect("docker exec runs");
    let listing = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        output.status.success(),
        "pg_dump or pg_restore failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let listed = listed_entries(&listing);

    let expected = expected_entries();
    let missing: Vec<&String> = expected.difference(&listed).collect();
    let unknown: Vec<&String> = listed.difference(&expected).collect();
    assert!(
        missing.is_empty() && unknown.is_empty(),
        "pg_restore -l and the file disagree.\nnot listed:\n{missing:?}\nnot in the file:\n{unknown:?}"
    );
}
