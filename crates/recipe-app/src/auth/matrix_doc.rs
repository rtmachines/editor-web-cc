//! Test that `api.md` section 3.3 and the operation catalog agree (design D7).
//!
//! The first test compares the names of column 1 with `Operation::ALL`. The second test reads the
//! header, every row, and every mark of the table. It compares each mark with the result of
//! `Operation::requirement` for the role of its column.
//!
//! A mark `✓` and a mark `(c)` both mean "the role holds the permission". The condition of a
//! `(c)` mark is a rule of the domain and not of the catalog. A mark `✗` means "denied".
//! A row names its operations in backticks in column 1 and its permission in column 2. A name
//! belongs to the operations whose requirement uses that permission. This resolves the names
//! that more than one row uses, such as `ListRecipes` and `ListAuditEntries`.

use std::collections::BTreeSet;

use recipe_domain::identity::Role;

use super::{Operation, Requirement};

const API_DOC: &str = include_str!("../../../../docs/architecture/api.md");

/// Words in backticks in column 1 that name a state, a flag, or a route and not an operation.
const NOT_OPERATIONS: [&str; 4] = ["Approved", "Retired", "Draft", "show_deleted"];

/// The role columns of the table: the header text and the role.
const ROLE_COLUMNS: [(&str, Role); 3] = [
    ("Administrator", Role::Administrator),
    ("ProcessEngineer", Role::ProcessEngineer),
    ("Operator", Role::Operator),
];

/// Return the text of section 3.3, from its heading to the next heading.
fn matrix_section() -> &'static str {
    let (_, after_heading) = API_DOC.split_once("### 3.3 Matrix").unwrap();
    after_heading.split("\n#").next().unwrap()
}

/// Return the names in backticks of one cell of column 1.
fn names_in_cell(cell: &str) -> impl Iterator<Item = &str> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .filter(|name| !name.starts_with('/'))
        .filter(|name| !NOT_OPERATIONS.contains(name))
}

/// Return the names in backticks of column 1 of each table row.
fn names_in_column_one(section: &str) -> BTreeSet<String> {
    section
        .lines()
        .filter(|line| line.starts_with('|'))
        .filter_map(|line| line.trim_start_matches('|').split('|').next())
        .flat_map(names_in_cell)
        .map(str::to_owned)
        .collect()
}

fn catalog_names() -> BTreeSet<String> {
    Operation::ALL
        .iter()
        .map(|operation| operation.name().to_owned())
        .collect()
}

#[test]
fn the_table_and_the_catalog_name_the_same_operations() {
    let in_table = names_in_column_one(matrix_section());
    assert!(
        in_table.len() > 40,
        "the table gave {} names",
        in_table.len()
    );
    let in_catalog = catalog_names();
    let only_in_table: Vec<_> = in_table.difference(&in_catalog).collect();
    let only_in_catalog: Vec<_> = in_catalog.difference(&in_table).collect();
    assert!(
        only_in_table.is_empty(),
        "names in api.md without a catalog entry: {only_in_table:?}"
    );
    assert!(
        only_in_catalog.is_empty(),
        "catalog names missing in api.md: {only_in_catalog:?}"
    );
}

/// One row of the table: the operations of column 1, the permission, and the mark of each role.
#[derive(Debug)]
struct DocRow {
    names: Vec<String>,
    permission: String,
    allowed: Vec<(Role, bool)>,
}

/// Return the cells of one table line without the outer bars, trimmed.
fn cells(line: &str) -> Vec<&str> {
    line.trim()
        .trim_start_matches('|')
        .trim_end_matches('|')
        .split('|')
        .map(str::trim)
        .collect()
}

/// Return true for a mark that allows the role, false for a mark that denies it.
///
/// An unknown mark gives `None`.
fn parse_mark(mark: &str) -> Option<bool> {
    match mark {
        "✓" | "(c)" => Some(true),
        "✗" => Some(false),
        _ => None,
    }
}

/// Parse the first table of `section`. The header row gives the columns of the roles.
fn parse_table(section: &str) -> Vec<DocRow> {
    let mut lines = section
        .lines()
        .skip_while(|line| !line.starts_with('|'))
        .take_while(|line| line.starts_with('|'));
    let header = cells(lines.next().unwrap());
    let column = |title: &str| header.iter().position(|cell| *cell == title).unwrap();
    let permission_column = column("Permission");
    let role_columns: Vec<(usize, Role)> = ROLE_COLUMNS
        .iter()
        .map(|(title, role)| (column(title), *role))
        .collect();
    lines
        .skip(1)
        .map(|line| {
            let row = cells(line);
            let cell = |index: usize| *row.get(index).unwrap();
            DocRow {
                names: names_in_cell(cell(0)).map(str::to_owned).collect(),
                permission: cell(permission_column).replace('`', ""),
                allowed: role_columns
                    .iter()
                    .map(|(index, role)| (*role, parse_mark(cell(*index)).unwrap()))
                    .collect(),
            }
        })
        .collect()
}

/// Return the labels that a row of column 2 can show for a requirement.
fn requirement_labels(requirement: Requirement) -> Vec<String> {
    match requirement {
        Requirement::Permission(permission) => vec![format!("{permission:?}")],
        Requirement::AnyOf(first, second) => vec![format!("{first:?}"), format!("{second:?}")],
        Requirement::Authenticated => vec!["authenticated".to_owned()],
        Requirement::NoSession => vec!["none".to_owned()],
    }
}

/// The result of the comparison: the count of compared cells and the differences.
#[derive(Debug, Default)]
struct Comparison {
    cells: usize,
    differences: Vec<String>,
}

/// Compare every mark of `rows` with the catalog result for the operations `operations`.
///
/// An operation is allowed for a role when any row that names it and shows one of its permission
/// labels marks the role as allowed. A name without an operation, and an operation without a row,
/// are differences.
fn compare(rows: &[DocRow], operations: &[Operation]) -> Comparison {
    let mut result = Comparison::default();
    for row in rows {
        for name in &row.names {
            let known = operations.iter().any(|operation| {
                operation.name() == name
                    && requirement_labels(operation.requirement()).contains(&row.permission)
            });
            if !known {
                result.differences.push(format!(
                    "row `{name}` with permission {}: no operation of the catalog",
                    row.permission
                ));
            }
        }
    }
    for operation in operations {
        let labels = requirement_labels(operation.requirement());
        let matching: Vec<&DocRow> = rows
            .iter()
            .filter(|row| row.names.iter().any(|name| name == operation.name()))
            .filter(|row| labels.contains(&row.permission))
            .collect();
        if matching.is_empty() {
            result
                .differences
                .push(format!("{operation:?}: no row in the table"));
            continue;
        }
        for (_, role) in ROLE_COLUMNS {
            let in_doc = matching.iter().any(|row| {
                row.allowed
                    .iter()
                    .any(|(row_role, allowed)| *row_role == role && *allowed)
            });
            let in_catalog = operation.requirement().allows(role);
            result.cells = result.cells.checked_add(1).unwrap();
            if in_doc != in_catalog {
                result.differences.push(format!(
                    "{operation:?} for {role:?}: api.md says {in_doc}, the catalog says {in_catalog}"
                ));
            }
        }
    }
    result
}

#[test]
fn every_mark_of_the_table_equals_the_catalog_result() {
    let rows = parse_table(matrix_section());
    assert!(rows.len() > 20, "the table gave {} rows", rows.len());
    let comparison = compare(&rows, &Operation::ALL);
    assert!(
        comparison.differences.is_empty(),
        "differences between api.md and the catalog: {:#?}",
        comparison.differences
    );
    let expected = Operation::ALL
        .len()
        .checked_mul(ROLE_COLUMNS.len())
        .unwrap();
    assert_eq!(comparison.cells, expected);
}

/// Compare each form query of `rows` with its purposes (ADR-0046 (c)).
///
/// A row that names a form query must also name a purpose of that query whose requirement shows
/// the permission of the row. Each other row gives one difference.
fn compare_purposes(rows: &[DocRow], operations: &[Operation]) -> Vec<String> {
    let names =
        |row: &DocRow, operation: &Operation| row.names.iter().any(|n| n == operation.name());
    let mut differences = Vec::new();
    for row in rows {
        let forms = operations
            .iter()
            .filter(|operation| !operation.purposes().is_empty() && names(row, operation));
        for form in forms {
            let served = form.purposes().iter().any(|purpose| {
                names(row, purpose)
                    && requirement_labels(purpose.requirement()).contains(&row.permission)
            });
            if !served {
                differences.push(format!(
                    "row `{form}` with permission {}: no purpose of the row needs it",
                    row.permission
                ));
            }
        }
    }
    differences
}

#[test]
fn every_row_of_a_form_query_names_a_purpose_with_the_permission_of_the_row() {
    let rows = parse_table(matrix_section());
    let differences = compare_purposes(&rows, &Operation::ALL);
    assert!(differences.is_empty(), "{differences:#?}");
}

const PURPOSE_TABLE: &str = "\
| Operation | Permission | Administrator | ProcessEngineer | Operator | Condition |
|---|---|---|---|---|---|
| Rename (`RenameRecipe`). Form query `GetRecipeForm` with the purpose `RenameRecipe` | `RecipeEdit` | ✓ | ✓ | ✗ | R25 |
";

#[test]
fn a_form_query_under_the_permission_of_another_purpose_is_a_difference() {
    let right = parse_table(PURPOSE_TABLE);
    assert!(compare_purposes(&right, &Operation::ALL).is_empty());
    let wrong = PURPOSE_TABLE.replace("| `RecipeEdit` |", "| `RecipeCreate` |");
    let differences = compare_purposes(&parse_table(&wrong), &Operation::ALL);
    assert_eq!(
        differences,
        vec![
            "row `GetRecipeForm` with permission RecipeCreate: no purpose of the row needs it"
                .to_owned()
        ]
    );
}

const SMALL_TABLE: &str = "\
| Operation | Permission | Administrator | ProcessEngineer | Operator | Condition |
|---|---|---|---|---|---|
| Rename recipe (`RenameRecipe`) | `RecipeEdit` | ✓ | ✓ | ✗ | R25 |
| Delete recipe (`DeleteRecipe`) | `RecipeDelete` | (c) | (c) | ✗ | A13 |
";

#[test]
fn the_parser_reads_the_header_the_names_and_the_marks() {
    let rows = parse_table(SMALL_TABLE);
    assert_eq!(rows.len(), 2);
    let first = rows.first().unwrap();
    assert_eq!(first.names, vec!["RenameRecipe".to_owned()]);
    assert_eq!(first.permission, "RecipeEdit");
    assert_eq!(
        first.allowed,
        vec![
            (Role::Administrator, true),
            (Role::ProcessEngineer, true),
            (Role::Operator, false)
        ]
    );
    let conditional = rows.get(1).unwrap();
    assert!(
        conditional
            .allowed
            .iter()
            .take(2)
            .all(|(_, allowed)| *allowed)
    );
}

#[test]
fn a_correct_small_table_has_no_difference() {
    let rows = parse_table(SMALL_TABLE);
    let comparison = compare(&rows, &[Operation::RenameRecipe, Operation::DeleteRecipe]);
    assert_eq!(comparison.cells, 6);
    assert!(
        comparison.differences.is_empty(),
        "{:?}",
        comparison.differences
    );
}

#[test]
fn one_wrong_cell_gives_exactly_one_difference() {
    let wrong = SMALL_TABLE.replace(
        "| `RecipeEdit` | ✓ | ✓ | ✗ |",
        "| `RecipeEdit` | ✓ | ✓ | ✓ |",
    );
    assert_ne!(wrong, SMALL_TABLE);
    let rows = parse_table(&wrong);
    let comparison = compare(&rows, &[Operation::RenameRecipe, Operation::DeleteRecipe]);
    assert_eq!(comparison.cells, 6);
    assert_eq!(
        comparison.differences,
        vec!["RenameRecipe for Operator: api.md says true, the catalog says false".to_owned()]
    );
}

#[test]
fn a_flipped_denied_mark_and_a_missing_row_are_differences() {
    let flipped = SMALL_TABLE.replace(
        "| `RecipeEdit` | ✓ | ✓ | ✗ |",
        "| `RecipeEdit` | ✓ | ✗ | ✗ |",
    );
    let rows = parse_table(&flipped);
    let comparison = compare(&rows, &[Operation::RenameRecipe, Operation::CreateRecipe]);
    assert_eq!(comparison.differences.len(), 3);
    assert!(
        comparison
            .differences
            .iter()
            .any(|text| text.starts_with("RenameRecipe for ProcessEngineer"))
    );
    assert!(
        comparison
            .differences
            .iter()
            .any(|text| text == "CreateRecipe: no row in the table")
    );
    assert!(
        comparison
            .differences
            .iter()
            .any(|text| text.starts_with("row `DeleteRecipe`"))
    );
}
