//! Proof that a unit word of the document that the type of its key does not know is a fault.
//!
//! Each case changes a copy of `reliability.md` and feeds it to `check_all`. The files of `deploy/`
//! stay as they are, so the document and `deploy/` differ in meaning after the change.

use crate::canon::Canon;
use crate::inputs::{Inputs, RELIABILITY, assert_mutations_fail};
use crate::table::{HEADING_SETTINGS, Table};
use crate::units::check_units;

/// A table 7.11 with the given rows only.
fn table_of(rows: &[&str]) -> Table {
    let markdown = format!(
        "### 7.11 Settings\n\n| Key | Value | Used by |\n|---|---|---|\n{}\n",
        rows.join("\n")
    );
    Table::parse(&markdown, HEADING_SETTINGS)
}

fn fault_count(rows: &[&str]) -> usize {
    check_units(&table_of(rows)).faults.len()
}

#[test]
fn unit_fault_names_the_key_the_row_text_and_the_word() {
    let report = check_units(&table_of(&["| `container.app_cpus` | 1 mCPU | 7.8 |"]));
    assert_eq!(report.faults.len(), 1, "{}", report.message());
    let text = report.message();
    for part in ["container.app_cpus", "1 mCPU", "mCPU"] {
        assert!(text.contains(part), "no {part:?} in: {text}");
    }
}

#[test]
fn unit_every_row_of_the_real_table_reads_without_a_fault() {
    let inputs = Inputs::load();
    let table = Table::parse(inputs.text(RELIABILITY), HEADING_SETTINGS);
    let report = check_units(&table);
    assert!(report.faults.is_empty(), "{}", report.message());
}

#[test]
fn unit_explanation_word_counts_only_on_its_own_row() {
    assert_eq!(
        fault_count(&["| `container.small_cpus` | 0.5 each for `proxy` | 7.8 |"]),
        0
    );
    assert_eq!(fault_count(&["| `container.app_cpus` | 1 each | 7.8 |"]), 1);
}

#[test]
fn unit_groups_of_three_digits_join_into_one_number() {
    let table = table_of(&["| `budget.leaves_per_parameter` | 16 384 | 4.4 |"]);
    assert_eq!(
        table.canon("budget.leaves_per_parameter"),
        Some(Canon::Number(16_384_000))
    );
}

#[test]
fn unit_each_type_reads_its_own_words() {
    let table = table_of(&[
        "| `session.idle_timeout` | 30 min | 7.12 |",
        "| `container.oneshot_memory` | 256 MiB each for `migrate` | 7.8 |",
        "| `pg.wal_keep_size` | 1 GB | 7.6 |",
        "| `log.max_size` / `log.max_file` | 50m / 20 | 3.5 |",
        "| `pg.shared_buffers` | 25 % of the `db` container memory | 7.6 |",
        "| `container.app_cpus` | 0.5 | 7.8 |",
    ]);
    let read = |key| table.canon(key);
    assert_eq!(read("session.idle_timeout"), Some(Canon::Millis(1_800_000)));
    assert_eq!(
        read("container.oneshot_memory"),
        Some(Canon::Bytes(268_435_456))
    );
    assert_eq!(read("pg.wal_keep_size"), Some(Canon::Bytes(1_073_741_824)));
    assert_eq!(read("log.max_size"), Some(Canon::Bytes(52_428_800)));
    assert_eq!(read("log.max_file"), Some(Canon::Number(20_000)));
    assert_eq!(read("pg.shared_buffers"), Some(Canon::Percent(25_000)));
    assert_eq!(read("container.app_cpus"), Some(Canon::Number(500)));
}

#[test]
fn unit_a_unit_of_another_type_is_a_fault() {
    for row in [
        "| `session.idle_timeout` | 30 MiB | 7.12 |",
        "| `session.idle_timeout` | 30 | 7.12 |",
        "| `container.app_memory` | 512 s | 7.8 |",
        "| `container.app_memory` | 512 m | 7.8 |",
        "| `pg.wal_keep_size` | 1 GiB | 7.6 |",
        "| `log.max_size` | 50 m | 3.5 |",
        "| `container.db_cpus` | 2 s | 7.8 |",
        "| `db.pool_size` | 10 s | 7.5 |",
    ] {
        assert_eq!(fault_count(&[row]), 1, "no fault for {row}");
    }
}

#[test]
fn unit_cpu_with_a_unit_word_is_a_fault() {
    assert_mutations_fail(&[(
        "reliability.md",
        "| 512 MiB / 1 |",
        "| 512 MiB / 1 mCPU |",
        "container.app_cpus",
    )]);
}

#[test]
fn unit_duration_with_another_unit_is_a_fault() {
    assert_mutations_fail(&[(
        "reliability.md",
        "`http.request_timeout` | 30 s |",
        "`http.request_timeout` | 30 min |",
        "http.request_timeout",
    )]);
}

#[test]
fn unit_binary_size_with_a_decimal_spelling_is_a_fault() {
    assert_mutations_fail(&[(
        "reliability.md",
        "| 256 MiB each for",
        "| 256 MB each for",
        "container.oneshot_memory",
    )]);
}
