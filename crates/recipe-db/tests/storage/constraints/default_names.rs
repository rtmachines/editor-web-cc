//! The names that PostgreSQL gives to inline constraints of `CREATE TABLE`.
//!
//! A primary key without a `CONSTRAINT` clause is `<table>_pkey`. A column with an inline
//! `REFERENCES` clause gets the foreign key `<table>_<column>_fkey`. The coverage meta-test
//! derives these names from the SQL of the document, so a new inline key needs a test.

/// Split text at the commas that are outside parentheses.
fn split_top_level(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth: u32 = 0;
    for character in text.chars() {
        match character {
            '(' => {
                depth = depth.checked_add(1).unwrap();
                current.push(character);
            }
            ')' => {
                depth = depth.saturating_sub(1);
                current.push(character);
            }
            ',' if depth == 0 => parts.push(std::mem::take(&mut current)),
            _ => current.push(character),
        }
    }
    parts.push(current);
    parts
}

/// The words that start a table constraint instead of a column definition.
const TABLE_CLAUSES: [&str; 5] = ["CONSTRAINT", "PRIMARY", "FOREIGN", "UNIQUE", "CHECK"];

/// Return the default names of the primary keys and inline foreign keys in normalized SQL.
pub fn default_constraint_names(sql: &str) -> Vec<String> {
    let mut names = Vec::new();
    for statement in sql.split(';') {
        let Some((_, rest)) = statement.split_once("CREATE TABLE ") else {
            continue;
        };
        let Some((head, body)) = rest.split_once('(') else {
            continue;
        };
        let table = head.trim();
        let Some(body) = body.trim_end().strip_suffix(')') else {
            continue;
        };
        let parts = split_top_level(body);
        if parts.iter().any(|part| part.contains("PRIMARY KEY")) {
            names.push(format!("{table}_pkey"));
        }
        for part in &parts {
            let Some(column) = part.split_whitespace().next() else {
                continue;
            };
            if !TABLE_CLAUSES.contains(&column) && part.contains(" REFERENCES ") {
                names.push(format!("{table}_{column}_fkey"));
            }
        }
    }
    names
}

#[test]
fn inline_primary_keys_and_foreign_keys_get_their_default_names() {
    let sql = "CREATE TABLE parent ( id UUID PRIMARY KEY DEFAULT gen_random_uuid(), \
               CONSTRAINT parent_name CHECK (name IN ('a', 'b')) ); \
               CREATE TABLE child ( parent_id UUID NOT NULL REFERENCES parent (id), \
               other_id UUID REFERENCES parent (id), ordinal INT, \
               PRIMARY KEY (parent_id, ordinal), \
               CONSTRAINT child_pair FOREIGN KEY (parent_id, ordinal) REFERENCES child (a, b) ); \
               CREATE INDEX child_index ON child (ordinal);";

    let names = default_constraint_names(sql);

    assert_eq!(
        names,
        [
            "parent_pkey",
            "child_pkey",
            "child_parent_id_fkey",
            "child_other_id_fkey"
        ]
    );
}
