use recipe_domain::error_codes::REGISTRY;

use super::*;

const RELIABILITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// Return the code and the first number of the HTTP column for each row of section 1.2.
fn documented_statuses() -> Vec<(String, u16)> {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    document
        .lines()
        .filter(|line| line.starts_with("| `E-"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells[1].trim_matches('`').to_owned();
            let number: String = cells[3].chars().take_while(char::is_ascii_digit).collect();
            (code, number.parse().unwrap())
        })
        .collect()
}

#[test]
fn error_mapping_status_table_equals_the_registry_of_the_document() {
    let documented = documented_statuses();
    assert!(documented.len() >= 100, "{}", documented.len());
    for (text, expected) in documented {
        let code = ErrorCode::find(&text).unwrap();
        assert_eq!(
            status_of(code).as_u16(),
            expected,
            "status of {text} differs from reliability.md section 1.2"
        );
    }
}

#[test]
fn error_mapping_every_registry_code_has_a_documented_row() {
    let documented = documented_statuses();
    for code in REGISTRY {
        assert!(
            documented
                .iter()
                .any(|(text, _status)| text == code.as_str()),
            "{code} has no row in reliability.md section 1.2"
        );
    }
}

#[test]
fn error_mapping_database_rule_codes_are_defects() {
    for text in [
        "E-DB-003",
        "E-DB-006",
        "E-DB-007",
        "E-PLT-030",
        "E-PLT-038",
        "E-RCP-030",
        "E-RCP-036",
        "E-AUD-003",
    ] {
        assert!(is_database_rule(ErrorCode::find(text).unwrap()), "{text}");
    }
    for text in [
        "E-DB-001",
        "E-PLT-001",
        "E-RCP-050",
        "E-IDN-003",
        "E-WEB-010",
    ] {
        assert!(!is_database_rule(ErrorCode::find(text).unwrap()), "{text}");
    }
}
