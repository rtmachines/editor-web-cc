//! Format tests of `tests/grants.expected`. They need no database.
//!
//! A table grant is the line `table|PRIVILEGE`. A column grant is the line `table.column|PRIVILEGE`.
//! The names come from the `CREATE TABLE` blocks of `data-model.md` section 3.

use crate::support::{created_names, document_sql};

const GRANTS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/grants.expected");
const SECTIONS: [&str; 18] = [
    "3.0", "3.1", "3.2", "3.3", "3.4", "3.5", "3.6", "3.7", "3.8", "3.9", "3.10", "3.11", "3.12",
    "3.13", "3.14", "3.15", "3.16", "3.17",
];
const MIGRATION_TABLE: &str = "_sqlx_migrations";
const PRIVILEGES: [&str; 4] = ["SELECT", "INSERT", "UPDATE", "DELETE"];
/// Keywords that start a table constraint, not a column definition.
const CONSTRAINT_KEYWORDS: [&str; 6] = [
    "CONSTRAINT",
    "PRIMARY",
    "UNIQUE",
    "CHECK",
    "FOREIGN",
    "EXCLUDE",
];

/// The non-empty lines of the grant file, in file order.
fn grant_lines() -> Vec<String> {
    let text = std::fs::read_to_string(GRANTS)
        .map_err(|error| format!("cannot read grants.expected: {error}"))
        .unwrap();
    text.lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Split a grant line into its object and its privilege. `None` when the privilege is unknown.
fn split_line(line: &str) -> Option<(&str, &str)> {
    let (object, privilege) = line.split_once('|')?;
    PRIVILEGES
        .contains(&privilege)
        .then_some((object, privilege))
}

/// True when the text is a plain identifier of lower-case letters, digits, and underscores.
fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();
    let starts_well = chars
        .next()
        .is_some_and(|first| first.is_ascii_lowercase() || first == '_');
    starts_well && chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_')
}

/// True when the object of a grant line is `table` or `table.column` with identifiers only.
fn is_object(object: &str) -> bool {
    match object.split_once('.') {
        None => is_identifier(object),
        Some((table, column)) => is_identifier(table) && is_identifier(column),
    }
}

/// The top-level items of a `CREATE TABLE` body. The text starts after the opening parenthesis.
fn body_items(text: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut depth: usize = 0;
    let mut in_quote = false;
    let mut item_start = 0;
    for (index, ch) in text.char_indices() {
        if in_quote {
            in_quote = ch != '\'';
            continue;
        }
        match ch {
            '\'' => in_quote = true,
            '(' => depth = depth.saturating_add(1),
            ')' if depth == 0 => {
                items.push(&text[item_start..index]);
                break;
            }
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                items.push(&text[item_start..index]);
                item_start = index.saturating_add(1);
            }
            _ => {}
        }
    }
    items
}

/// The column name that a table item defines, or `None` for a table constraint.
fn column_name(item: &str) -> Option<String> {
    let first = item.split_whitespace().next()?;
    (!CONSTRAINT_KEYWORDS.contains(&first)).then(|| first.to_owned())
}

/// The table name and the column names of every `CREATE TABLE` of normalized SQL.
fn table_columns(normalized: &str) -> Vec<(String, Vec<String>)> {
    normalized
        .split("CREATE TABLE ")
        .skip(1)
        .filter_map(|rest| {
            let (name, body) = rest.split_once(" (")?;
            let columns = body_items(body)
                .into_iter()
                .filter_map(column_name)
                .collect();
            Some((name.to_owned(), columns))
        })
        .collect()
}

#[test]
fn every_line_has_the_form_object_bar_privilege() {
    let lines = grant_lines();

    assert!(!lines.is_empty(), "grants.expected has no grant line");
    for line in &lines {
        let parsed = split_line(line);
        assert!(parsed.is_some(), "unknown privilege or missing bar: {line}");
        let (object, _) = parsed.unwrap();
        assert!(
            is_object(object),
            "object is not table or table.column: {line}"
        );
    }
}

#[test]
fn lines_are_sorted_in_byte_order_without_duplicates() {
    let lines = grant_lines();

    let unordered = lines.windows(2).find(|pair| pair[0] >= pair[1]);

    assert!(
        unordered.is_none(),
        "not sorted or duplicated: {unordered:?}"
    );
}

#[test]
fn every_table_is_a_table_of_section_3_or_sqlx_migrations() {
    let tables = created_names(&document_sql(&SECTIONS, &[])).tables;
    let lines = grant_lines();

    for line in &lines {
        let (object, _) = split_line(line).unwrap();
        let table = object.split_once('.').map_or(object, |(table, _)| table);
        let known = table == MIGRATION_TABLE || tables.iter().any(|name| name == table);
        assert!(known, "table of {line} is not in section 3: {tables:?}");
    }
}

#[test]
fn every_column_line_names_a_column_of_its_table() {
    let columns = table_columns(&document_sql(&SECTIONS, &[]));
    let lines = grant_lines();

    assert_eq!(columns.len(), 17, "section 3 must give 17 tables");
    for (table, names) in &columns {
        assert!(!names.is_empty(), "no columns parsed for {table}");
    }
    for line in &lines {
        let (object, _) = split_line(line).unwrap();
        let Some((table, column)) = object.split_once('.') else {
            continue;
        };
        let known = columns
            .iter()
            .find(|(name, _)| name == table)
            .is_some_and(|(_, names)| names.iter().any(|name| name == column));
        assert!(known, "{column} is not a column of {table} in section 3");
    }
}
