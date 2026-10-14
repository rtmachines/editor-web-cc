use std::fs;
use std::path::Path;

use recipe_domain::error_codes::{ErrorCode, REGISTRY};

use super::user_message;

/// The 16 trigger codes and the code of the check constraint without a trigger code.
const TRIGGER_CODES: [&str; 17] = [
    "E-PLT-030",
    "E-PLT-031",
    "E-PLT-032",
    "E-PLT-033",
    "E-PLT-034",
    "E-PLT-035",
    "E-PLT-036",
    "E-PLT-037",
    "E-PLT-038",
    "E-RCP-030",
    "E-RCP-031",
    "E-RCP-032",
    "E-RCP-033",
    "E-RCP-034",
    "E-RCP-035",
    "E-RCP-036",
    "E-DB-003",
];

/// The sentence that section 1.2 appends to every user message.
const REFERENCE_SUFFIX: &str = " Reference: {request_id}.";

/// One table row of section 1.2.
struct Row {
    code: String,
    source: String,
    message: String,
}

/// Return the text of section 1.2 of `reliability.md`.
fn registry_section() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture/reliability.md");
    let text = fs::read_to_string(&path).unwrap();
    let start = text.find("### 1.2 Error code registry").unwrap();
    let rest = text.get(start..).unwrap();
    let end = rest.find("### 1.3").unwrap();
    rest.get(..end).unwrap().to_owned()
}

/// Parse the table rows of section 1.2. Each row starts with a code cell.
fn registry_rows() -> Vec<Row> {
    registry_section()
        .lines()
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells.get(1)?;
            if !code.starts_with("`E-") {
                return None;
            }
            let last = cells.len().checked_sub(2)?;
            Some(Row {
                code: code.trim_matches('`').to_owned(),
                source: (*cells.get(2)?).to_owned(),
                message: (*cells.get(last)?).to_owned(),
            })
        })
        .collect()
}

#[test]
fn user_message_is_the_row_text_without_the_reference_sentence() {
    let rows = registry_rows();
    for text in TRIGGER_CODES {
        let code = ErrorCode::find(text).unwrap();
        let row = rows.iter().find(|row| row.code == text).unwrap();
        let expected = row.message.strip_suffix(REFERENCE_SUFFIX).unwrap();
        assert_eq!(user_message(code), Some(expected), "{text}");
    }
}

#[test]
fn trigger_codes_are_exactly_the_rows_that_name_a_trigger_function_or_e_db_003() {
    let mut documented: Vec<String> = registry_rows()
        .into_iter()
        .filter(|row| row.source.contains("trigger function") || row.code == "E-DB-003")
        .map(|row| row.code)
        .collect();
    documented.sort();
    let mut listed: Vec<String> = TRIGGER_CODES
        .iter()
        .map(|text| (*text).to_owned())
        .collect();
    listed.sort();
    assert_eq!(documented, listed);
}

#[test]
fn codes_outside_the_trigger_set_have_no_user_message() {
    assert_eq!(user_message(ErrorCode::find("E-RCP-001").unwrap()), None);
    for code in REGISTRY {
        if !TRIGGER_CODES.contains(&code.as_str()) {
            assert_eq!(user_message(*code), None, "{code}");
        }
    }
}
