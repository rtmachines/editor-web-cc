use recipe_domain::error_codes::REGISTRY;

use super::*;

const RELIABILITY: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../docs/architecture/reliability.md"
);

/// Return the code and the status of each row of section 1.2, with `None` for a row without HTTP answer.
fn documented_rows() -> Vec<(String, Option<u16>)> {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    rows_of(&document)
}

/// Return the code and the first number of the HTTP column for each row that has an HTTP answer.
fn documented_statuses() -> Vec<(String, u16)> {
    let document = std::fs::read_to_string(RELIABILITY).unwrap();
    statuses_of(&document)
}

/// Return the code and the status of each row of a registry text. The cell `-` gives `None`.
fn rows_of(document: &str) -> Vec<(String, Option<u16>)> {
    document
        .lines()
        .filter(|line| line.starts_with("| `E-"))
        .map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells[1].trim_matches('`').to_owned();
            if cells[3] == "-" {
                return (code, None);
            }
            let number: String = cells[3].chars().take_while(char::is_ascii_digit).collect();
            (code, Some(number.parse().unwrap()))
        })
        .collect()
}

/// Return the rows of a registry text that have an HTTP answer.
fn statuses_of(document: &str) -> Vec<(String, u16)> {
    rows_of(document)
        .into_iter()
        .filter_map(|(code, status)| status.map(|number| (code, number)))
        .collect()
}

const FIXTURE: &str = "\
| Code | Variant | HTTP | Text |
|---|---|---|---|
| `E-WEB-001` | `WebError::A` | 409 | First. |
| `E-WEB-015` | `WebError::B` | - | No HTTP answer. |
| `E-WEB-099` | `WebError::C` | 503 | Third. |
";

#[test]
fn status_parser_skips_a_row_without_http_answer() {
    let rows = statuses_of(FIXTURE);
    assert_eq!(
        rows,
        vec![("E-WEB-001".to_owned(), 409), ("E-WEB-099".to_owned(), 503)]
    );
}

#[test]
fn status_parser_keeps_the_code_of_a_row_without_http_answer() {
    let rows = rows_of(FIXTURE);
    assert_eq!(rows.len(), 3);
    assert!(rows.contains(&("E-WEB-015".to_owned(), None)));
}

#[test]
#[should_panic(expected = "ParseIntError")]
fn status_parser_rejects_a_value_that_is_not_a_status() {
    statuses_of("| `E-WEB-001` | `WebError::A` | none | Text. |\n");
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
    let documented = documented_rows();
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
fn error_mapping_range_and_address_codes_give_status_422() {
    for text in ["E-RCP-055", "E-RCP-056", "E-RCP-057"] {
        let code = ErrorCode::find(text).unwrap();
        assert_eq!(status_of(code), StatusCode::UNPROCESSABLE_ENTITY, "{text}");
        assert!(!is_database_rule(code), "{text}");
    }
}

#[test]
fn error_mapping_unchanged_revision_gives_status_409() {
    let code = ErrorCode::find("E-PLT-052").unwrap();
    assert_eq!(status_of(code), StatusCode::CONFLICT);
    assert!(!is_database_rule(code));
}

#[test]
fn error_mapping_connection_codes_give_status_422_or_404() {
    for text in [
        "E-PLT-022",
        "E-PLT-023",
        "E-PLT-024",
        "E-PLT-025",
        "E-PLT-026",
        "E-PLT-028",
    ] {
        let code = ErrorCode::find(text).unwrap();
        assert_eq!(status_of(code), StatusCode::UNPROCESSABLE_ENTITY, "{text}");
        assert!(!is_database_rule(code), "{text}");
    }
    for text in ["E-PLT-027", "E-PLT-029"] {
        let code = ErrorCode::find(text).unwrap();
        assert_eq!(status_of(code), StatusCode::NOT_FOUND, "{text}");
        assert!(!is_database_rule(code), "{text}");
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
