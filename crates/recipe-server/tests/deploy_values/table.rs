//! A reader for the tables of `docs/architecture/reliability.md`.
//!
//! A row starts with a backticked key cell. One row can hold several keys (`a` / `b`) and
//! several values (`1 s / 2 s`). The reader gives each key its own part of the value cell.

use std::collections::BTreeMap;

use crate::canon::Canon;
use crate::units::{kind_of, read_typed};

/// The heading of the table of settings, `reliability.md` section 7.11.
pub const HEADING_SETTINGS: &str = "### 7.11 Settings";
/// The heading of the table of PostgreSQL settings, `reliability.md` section 7.6.
pub const HEADING_POSTGRES: &str = "### 7.6 PostgreSQL settings for data safety";

/// The rows of one table, keyed by the text inside the backticks.
pub struct Table {
    rows: BTreeMap<String, String>,
}

fn keys_of(cell: &str) -> Vec<&str> {
    cell.split('`').skip(1).step_by(2).collect()
}

fn values_of(cell: &str, key_count: usize) -> Vec<&str> {
    if key_count == 1 {
        return vec![cell];
    }
    cell.split(" / ").map(str::trim).take(key_count).collect()
}

impl Table {
    /// Read the rows between the heading and the next heading.
    /// A key that appears twice stops the test run and names the key.
    pub fn parse(markdown: &str, heading: &str) -> Table {
        let mut rows = BTreeMap::new();
        let mut in_section = false;
        for line in markdown.lines() {
            if line.starts_with('#') {
                in_section = line.trim() == heading;
                continue;
            }
            if !in_section || !line.starts_with("| `") {
                continue;
            }
            let inner = line.trim().trim_start_matches('|').trim_end_matches('|');
            let cells: Vec<&str> = inner.split(" | ").map(str::trim).collect();
            let (Some(key_cell), Some(value_cell)) = (cells.first(), cells.get(1)) else {
                continue;
            };
            let keys = keys_of(key_cell);
            let values = values_of(value_cell, keys.len());
            if values.len() < keys.len() {
                continue;
            }
            for (key, value) in keys.into_iter().zip(values) {
                let earlier = rows.insert(key.to_owned(), value.to_owned());
                assert!(
                    earlier.is_none(),
                    "duplicate key `{key}` in the table {heading}: the document must give each key one row"
                );
            }
        }
        Table { rows }
    }

    /// The number of keys.
    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Every key of the table.
    pub fn keys(&self) -> impl Iterator<Item = &String> {
        self.rows.keys()
    }

    /// The value text of a key, with backticks.
    pub fn raw(&self, key: &str) -> Option<&str> {
        self.rows.get(key).map(String::as_str)
    }

    /// The text between the first pair of backticks of the value, for example `read committed`.
    pub fn literal(&self, key: &str) -> Option<String> {
        let raw = self.raw(key)?;
        raw.split('`').nth(1).map(str::to_owned)
    }

    /// The leading value of a key in its canonical form, read by the type of the key.
    /// A word that the type does not know gives `None`. `check_units` reports that word.
    pub fn canon(&self, key: &str) -> Option<Canon> {
        read_typed(key, kind_of(key), self.raw(key)?).ok()
    }

    /// The leading value of a time key in milliseconds.
    pub fn millis(&self, key: &str) -> Option<u128> {
        match self.canon(key)? {
            Canon::Millis(value) => Some(value),
            _ => None,
        }
    }

    /// The text to show in a fault: the start of the value cell.
    pub fn shown(&self, key: &str) -> String {
        let raw = self.raw(key).unwrap_or("no row");
        let short: String = raw.chars().take(40).collect();
        format!("{short} (7.11 `{key}`)")
    }

    /// The expected value of a key for `Report::compare`.
    pub fn expected(&self, key: &str) -> Option<(Canon, String)> {
        Some((self.canon(key)?, self.shown(key)))
    }
}

fn table_text(rows: &[&str]) -> String {
    format!(
        "### 7.11 Settings\n\n| Key | Value | Used by |\n|---|---|---|\n{}\n",
        rows.join("\n")
    )
}

#[test]
#[should_panic(expected = "duplicate key `http.request_timeout`")]
fn table_duplicate_key_in_two_rows_fails_and_names_the_key() {
    let markdown = table_text(&[
        "| `http.request_timeout` | 30 s | 7.3 |",
        "| `shutdown.drain` | 30 s | 7.3 |",
        "| `http.request_timeout` | 25 s | 7.3 |",
    ]);
    Table::parse(&markdown, HEADING_SETTINGS);
}

#[test]
#[should_panic(expected = "duplicate key `shutdown.drain`")]
fn table_duplicate_key_inside_one_row_fails_and_names_the_key() {
    let markdown = table_text(&["| `shutdown.drain` / `shutdown.drain` | 30 s / 40 s | 7.3 |"]);
    Table::parse(&markdown, HEADING_SETTINGS);
}

#[test]
fn table_distinct_keys_read_without_a_fault() {
    let markdown = table_text(&[
        "| `shutdown.drain` / `app.stop_grace_period` | 30 s / 40 s | 7.3 |",
        "| `http.request_timeout` | 30 s | 7.3 |",
    ]);
    let table = Table::parse(&markdown, HEADING_SETTINGS);
    assert_eq!(table.len(), 3);
}
