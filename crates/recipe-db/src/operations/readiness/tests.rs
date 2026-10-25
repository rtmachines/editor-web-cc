use super::{AppliedRow, embedded_checksums, schema_matches};

fn row(version: i64, checksum: &[u8], success: bool) -> AppliedRow {
    (version, checksum.to_vec(), success)
}

const EMBEDDED: [(i64, &[u8]); 2] = [(1, &[1, 1]), (2, &[2, 2])];

#[test]
fn the_schema_matches_when_every_embedded_migration_is_applied() {
    let applied = [row(1, &[1, 1], true), row(2, &[2, 2], true)];

    assert!(schema_matches(EMBEDDED, &applied));
}

#[test]
fn a_newer_database_matches() {
    let applied = [
        row(1, &[1, 1], true),
        row(2, &[2, 2], true),
        row(3, &[3, 3], true),
    ];

    assert!(schema_matches(EMBEDDED, &applied));
}

#[test]
fn a_changed_checksum_does_not_match() {
    let applied = [row(1, &[1, 1], true), row(2, &[9, 9], true)];

    assert!(!schema_matches(EMBEDDED, &applied));
}

#[test]
fn a_missing_migration_does_not_match() {
    let applied = [row(1, &[1, 1], true)];

    assert!(!schema_matches(EMBEDDED, &applied));
}

#[test]
fn a_failed_row_does_not_match_even_for_a_newer_version() {
    let applied = [
        row(1, &[1, 1], true),
        row(2, &[2, 2], true),
        row(3, &[3, 3], false),
    ];

    assert!(!schema_matches(EMBEDDED, &applied));
}

#[test]
fn the_binary_embeds_forward_migrations_with_distinct_versions() {
    let versions: Vec<i64> = embedded_checksums().map(|(version, _)| version).collect();

    assert!(!versions.is_empty());
    let mut sorted = versions.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted, versions);
}
