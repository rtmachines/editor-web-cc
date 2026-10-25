//! Proof that the `command:` list of `db` is read as a closed grammar.
//!
//! PostgreSQL accepts the long form `--name=value` and a repeated `-c` pair, and the last one wins.
//! A check that reads only the first pair of a setting would keep the checked value. Each case here
//! changes one copy of `deploy/compose.yaml` and asserts a fault that names the setting.

use crate::inputs::{Mutation, assert_mutations_fail};

const CLOSED_GRAMMAR: [Mutation; 4] = [
    // The long form after the last pair overrides the checked value.
    (
        "compose.yaml",
        "      - timezone=UTC",
        "      - timezone=UTC\n      - --max_connections=60",
        "max_connections (item outside",
    ),
    // The long form with the checked value is still an item outside the grammar.
    (
        "compose.yaml",
        "      - timezone=UTC",
        "      - timezone=UTC\n      - --max_connections=50",
        "max_connections (item outside",
    ),
    // A repeated pair with the same value is still a second pair.
    (
        "compose.yaml",
        "      - max_connections=50",
        "      - max_connections=50\n      - -c\n      - max_connections=50",
        "max_connections (second -c pair)",
    ),
    // A repeated pair with another value is a fault that names the repeat, not only the value.
    (
        "compose.yaml",
        "      - max_connections=50",
        "      - max_connections=50\n      - -c\n      - max_connections=60",
        "max_connections (second -c pair)",
    ),
];

#[test]
fn a_long_form_item_or_a_repeated_pair_in_the_db_command_is_a_fault() {
    assert_mutations_fail(&CLOSED_GRAMMAR);
}
