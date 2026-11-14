//! The reader of the rows of `reliability.md` section 1.2.

/// One row of the registry: its code and the `Type::Variant` names of its owner cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub code: String,
    pub variants: Vec<String>,
}

/// Return the rows of the table in section 1.2 of `document`.
///
/// A row is a line that starts with ``| `E-``. The section ends at the next heading of level 3.
pub fn rows_of(document: &str) -> Vec<Row> {
    let section = document
        .split_once("### 1.2 ")
        .map_or("", |(_before, after)| after);
    let section = section
        .split_once("\n### ")
        .map_or(section, |(inside, _after)| inside);
    section
        .lines()
        .filter(|line| line.starts_with("| `E-"))
        .map(row_of)
        .collect()
}

/// Return the row of one table line.
fn row_of(line: &str) -> Row {
    let cells: Vec<&str> = line.split('|').map(str::trim).collect();
    Row {
        code: cells[1].trim_matches('`').to_owned(),
        variants: variants_of(cells[2]),
    }
}

/// Return the `Type::Variant` names in the code spans of an owner cell.
///
/// The text of a span ends at the first character that is not part of a path. A span without `::`
/// is not a name, for example the name of a trigger function.
fn variants_of(owner_cell: &str) -> Vec<String> {
    owner_cell
        .split('`')
        .skip(1)
        .step_by(2)
        .map(|span| {
            span.chars()
                .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':'))
                .collect::<String>()
        })
        .filter(|name| name.contains("::") && name.starts_with(|c: char| c.is_ascii_uppercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = "\
## 1. Errors

### 1.1 Types

| `E-XXX-999` | `Other::Row` | 500 | Text. |

### 1.2 Error code registry

| Code | Owner::Variant { fields } | HTTP | User message |
|---|---|---|---|
| `E-PLT-001` | `PlantError::A { code }` | 409 | First. |
| `E-PLT-020` | `PlantError::B { path }` and `SnapshotItem::C` | 404 | Second. |
| `E-PLT-032` | `StorageError::Constraint`, trigger function `enforce_type_platform` (data-model.md 4.2) | 422 | Third. |
| `E-DB-003` | `StorageError::Constraint { code: \"E-DB-003\", .. }` | 409 | Fourth. |

### 1.3 Next

| `E-YYY-998` | `Other::Row` | 500 | Text. |
";

    #[test]
    fn rows_of_reads_only_the_table_of_section_1_2() {
        let rows = rows_of(FIXTURE);
        let codes: Vec<&str> = rows.iter().map(|row| row.code.as_str()).collect();
        assert_eq!(codes, ["E-PLT-001", "E-PLT-020", "E-PLT-032", "E-DB-003"]);
    }

    #[test]
    fn rows_of_collects_each_name_of_an_owner_cell() {
        let rows = rows_of(FIXTURE);
        assert_eq!(rows[0].variants, ["PlantError::A"]);
        assert_eq!(rows[1].variants, ["PlantError::B", "SnapshotItem::C"]);
    }

    #[test]
    fn rows_of_skips_a_code_span_that_is_not_a_name() {
        let rows = rows_of(FIXTURE);
        assert_eq!(rows[2].variants, ["StorageError::Constraint"]);
        assert_eq!(rows[3].variants, ["StorageError::Constraint"]);
    }
}
