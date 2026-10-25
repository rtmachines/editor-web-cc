//! Proof that the list reader hides no `cap_add` item and no `ports` item of a service.
//!
//! The reader gave an empty list for an indentless block sequence and for a key from a merge.
//! Each case changes one copy of `compose.yaml` and asserts a fault that names the service and the key.

use crate::inputs::{Mutation, assert_mutations_fail};

const BACKUP_ANCHOR: &str = "    init: true";
const DB_ANCHOR: &str = "    user: \"999:999\"";

#[test]
fn an_indentless_block_sequence_is_a_fault() {
    let cases: [Mutation; 2] = [
        (
            "compose.yaml",
            BACKUP_ANCHOR,
            "    cap_add:\n    - SYS_ADMIN\n    init: true",
            "backup.cap_add",
        ),
        (
            "compose.yaml",
            DB_ANCHOR,
            "    ports:\n    - \"5432:5432\"\n    user: \"999:999\"",
            "db.ports",
        ),
    ];
    assert_mutations_fail(&cases);
}

#[test]
fn a_key_without_a_readable_item_is_a_fault() {
    let cases: [Mutation; 3] = [
        (
            "compose.yaml",
            BACKUP_ANCHOR,
            "    cap_add:\n    [SYS_ADMIN]\n    init: true",
            "backup.cap_add",
        ),
        (
            "compose.yaml",
            BACKUP_ANCHOR,
            "    cap_add:\n    init: true",
            "backup.cap_add",
        ),
        (
            "compose.yaml",
            DB_ANCHOR,
            "    ports:\n    user: \"999:999\"",
            "db.ports",
        ),
    ];
    assert_mutations_fail(&cases);
}

#[test]
fn an_extra_block_item_of_cap_add_or_ports_is_a_fault() {
    let cases: [Mutation; 2] = [
        (
            "compose.yaml",
            DB_ANCHOR,
            "    cap_add:\n      - SYS_ADMIN\n    user: \"999:999\"",
            "db.cap_add",
        ),
        (
            "compose.yaml",
            DB_ANCHOR,
            "    ports:\n      - \"5432:5432\"\n    user: \"999:999\"",
            "db.ports",
        ),
    ];
    assert_mutations_fail(&cases);
}

#[test]
fn a_merge_key_inside_a_service_is_a_fault() {
    let cases: [Mutation; 2] = [
        (
            "compose.yaml",
            BACKUP_ANCHOR,
            "    <<: *extra\n    init: true",
            "backup.<<",
        ),
        (
            "compose.yaml",
            DB_ANCHOR,
            "    <<: *extra\n    user: \"999:999\"",
            "db.<<",
        ),
    ];
    assert_mutations_fail(&cases);
}
