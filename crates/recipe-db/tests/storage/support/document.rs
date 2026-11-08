//! The SQL of `data-model.md` and of the migration files, in one comparable form (design D11).
//!
//! A migration file is an exact copy of the SQL blocks of the document. The document test
//! compares both sides after the same normalization. Comments drop out, and each run of white
//! space becomes one space. A file can therefore keep its own header comment.

use std::path::PathBuf;

const DOCUMENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/data-model.md"
);
const MIGRATIONS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../migrations");
const INIT_SCRIPT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../deploy/db/initdb/01-roles.sh"
);
const SQL_FENCE: &str = "```sql";
const CLOSING_FENCE: &str = "```";

/// Drop the `--` comments, make each run of white space one space, and trim.
pub fn normalize(sql: &str) -> String {
    sql.lines()
        .map(|line| line.split("--").next().unwrap_or_default())
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Return the raw SQL blocks under the heading of one section, in document order.
fn section_blocks(document: &str, section: &str) -> Vec<String> {
    let heading = format!("### {section} ");
    let mut blocks = Vec::new();
    let mut current = String::new();
    let mut in_section = false;
    let mut in_block = false;
    for line in document.lines() {
        if in_block {
            if line.trim_end() == CLOSING_FENCE {
                blocks.push(std::mem::take(&mut current));
                in_block = false;
            } else {
                current.push_str(line);
                current.push('\n');
            }
        } else if line.starts_with("## ") || line.starts_with("### ") {
            in_section = line.starts_with(&heading);
        } else if in_section && line.trim_end() == SQL_FENCE {
            in_block = true;
        }
    }
    blocks
}

/// The normalized SQL of the listed sections of `data-model.md`, in the order of the list.
///
/// `skip` names blocks that are not part of a migration: a pair of a section and the index of
/// the block in that section, counted from 0. The call fails when a section has no SQL block
/// or when a skip names a missing block.
pub fn document_sql(sections: &[&str], skip: &[(&str, usize)]) -> String {
    let document = std::fs::read_to_string(DOCUMENT)
        .map_err(|error| format!("cannot read data-model.md: {error}"))
        .unwrap();
    let mut sql = Vec::new();
    for section in sections {
        let blocks = section_blocks(&document, section);
        assert!(!blocks.is_empty(), "section {section} has no SQL block");
        for (section_skipped, index) in skip {
            if section_skipped == section {
                assert!(
                    *index < blocks.len(),
                    "section {section} has no block {index}"
                );
            }
        }
        for (index, block) in blocks.iter().enumerate() {
            if !skip.contains(&(*section, index)) {
                sql.push(block.clone());
            }
        }
    }
    normalize(&sql.join("\n"))
}

/// One piece of `data-model.md` section 3 that a later migration changed after `0001`.
///
/// Migration 0001 shipped before the piece and never changes. A later migration adds the piece:
/// 0009 adds the pieces of ADR-0109 as `NOT VALID` constraints, and 0011 changes `dims_valid`
/// (ADR-0145) and adds two unique indexes (ADR-0143). The document shows the current schema, so
/// the comparison with 0001 must undo each piece first. `current` and `shipped` are normalized
/// text. `current` must occur exactly once in the document text.
pub struct ShippedPiece {
    /// What the piece is, for the failure message.
    pub name: &'static str,
    /// The normalized text in the current document.
    pub current: &'static str,
    /// The normalized text in migration 0001. An empty text removes the piece.
    pub shipped: &'static str,
}

/// The pieces of section 3 that a migration after 0001 changed.
pub const PIECES_AFTER_0001: [ShippedPiece; 5] = [
    ShippedPiece {
        name: "role list of app_user_role_value",
        current: "'operator', 'export_client')",
        shipped: "'operator')",
    },
    ShippedPiece {
        name: "constraint app_user_export_client_password",
        current: " CONSTRAINT app_user_export_client_password CHECK ( role <> 'export_client' OR NOT must_change_password ),",
        shipped: "",
    },
    ShippedPiece {
        name: "start subscript checks of dims_valid (ADR-0145)",
        current: " AND array_lower(lower_b, 1) = 1 AND array_lower(upper_b, 1) = 1",
        shipped: "",
    },
    ShippedPiece {
        name: "index plc_type_struct_name_ci_unique (ADR-0143)",
        current: " CREATE UNIQUE INDEX plc_type_struct_name_ci_unique ON plc_type (machine_id, lower(name), revision) WHERE kind = 'struct';",
        shipped: "",
    },
    ShippedPiece {
        name: "index plc_type_member_name_ci_unique (ADR-0143)",
        current: " CREATE UNIQUE INDEX plc_type_member_name_ci_unique ON plc_type_member (struct_type_id, lower(member_name));",
        shipped: "",
    },
];

/// Turn the normalized text of section 3 into the text of migration 0001.
///
/// Each piece of [`PIECES_AFTER_0001`] must match exactly once. The call fails with the name of
/// the piece otherwise.
pub fn as_shipped_in_0001(normalized: &str) -> String {
    PIECES_AFTER_0001
        .iter()
        .fold(normalized.to_owned(), |text, piece| {
            let count = text.matches(piece.current).count();
            assert_eq!(
                count, 1,
                "piece '{}' must match once in the document, found {count} times",
                piece.name
            );
            text.replacen(piece.current, piece.shipped, 1)
        })
}

/// The text `CONSTRAINT <name> CHECK ( ... )` of a constraint in normalized SQL, up to the
/// matching closing parenthesis. The call fails when the constraint is missing.
pub fn check_constraint(normalized: &str, name: &str) -> String {
    let head = format!("CONSTRAINT {name} CHECK (");
    let found = normalized.find(&head);
    assert!(found.is_some(), "no constraint {name} in the SQL text");
    let start = found.expect("presence asserted above");
    let tail = &normalized[start..];
    let mut depth = 0_u32;
    let mut closing = None;
    for (offset, character) in tail.char_indices() {
        match character {
            '(' => depth = depth.checked_add(1).expect("parenthesis depth overflow"),
            ')' => {
                depth = depth
                    .checked_sub(1)
                    .expect("unbalanced closing parenthesis");
                if depth == 0 {
                    closing = Some(offset);
                    break;
                }
            }
            _ => {}
        }
    }
    assert!(
        closing.is_some(),
        "constraint {name} has no closing parenthesis"
    );
    let offset = closing.expect("presence asserted above");
    tail[..=offset].to_owned()
}

/// The path of a migration file.
fn migration_path(file: &str) -> PathBuf {
    PathBuf::from(MIGRATIONS).join(file)
}

/// The normalized text of a migration file. The call fails with a clear message when the file
/// does not exist.
pub fn migration_sql(file: &str) -> String {
    let path = migration_path(file);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("cannot read migration {}: {error}", path.display()))
        .unwrap();
    normalize(&text)
}

/// The normalized text of the init script `01-roles.sh`. The call fails with a clear message
/// when the file does not exist.
pub fn init_script_sql() -> String {
    let text = std::fs::read_to_string(INIT_SCRIPT)
        .map_err(|error| format!("cannot read 01-roles.sh: {error}"))
        .unwrap();
    normalize(&text)
}

/// The file name and the raw text of every migration file, sorted by file name.
pub fn migration_texts() -> Vec<(String, String)> {
    let entries = std::fs::read_dir(MIGRATIONS)
        .map_err(|error| format!("cannot list migrations: {error}"))
        .unwrap();
    let mut files: Vec<(String, String)> = entries
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "sql"))
        .map(|path| {
            let text = std::fs::read_to_string(&path)
                .map_err(|error| format!("cannot read {}: {error}", path.display()))
                .unwrap();
            (
                path.file_name().unwrap().to_string_lossy().into_owned(),
                text,
            )
        })
        .collect();
    files.sort();
    files
}

/// The names that a piece of SQL creates.
#[derive(Debug, Default)]
pub struct CreatedNames {
    /// `CREATE TABLE <name>`.
    pub tables: Vec<String>,
    /// `CONSTRAINT <name>`.
    pub constraints: Vec<String>,
    /// `CREATE [UNIQUE] INDEX <name>`.
    pub indexes: Vec<String>,
    /// `CREATE [CONSTRAINT] TRIGGER <name>`.
    pub triggers: Vec<String>,
    /// `CREATE FUNCTION <name>`.
    pub functions: Vec<String>,
}

/// The identifier at the start of a token. A function name can stick to its parenthesis.
fn identifier(token: &str) -> String {
    token.split('(').next().unwrap_or_default().to_owned()
}

/// Find the names that normalized SQL creates.
pub fn created_names(normalized: &str) -> CreatedNames {
    let tokens: Vec<&str> = normalized.split(' ').collect();
    let mut names = CreatedNames::default();
    for window in tokens.windows(4) {
        let [first, second, third, fourth] = window else {
            continue;
        };
        match (*first, *second, *third) {
            ("CREATE", "TABLE", _) => names.tables.push(identifier(third)),
            ("CREATE", "FUNCTION", _) => names.functions.push(identifier(third)),
            ("CREATE", "TRIGGER", _) => names.triggers.push(identifier(third)),
            ("CREATE", "INDEX", _) => names.indexes.push(identifier(third)),
            ("CREATE", "UNIQUE", "INDEX") | ("CREATE", "CONSTRAINT", "TRIGGER") => {
                let target = if *second == "UNIQUE" {
                    &mut names.indexes
                } else {
                    &mut names.triggers
                };
                target.push(identifier(fourth));
            }
            ("CONSTRAINT", name, _) if name != "TRIGGER" => {
                names.constraints.push(identifier(name));
            }
            _ => {}
        }
    }
    names
}
