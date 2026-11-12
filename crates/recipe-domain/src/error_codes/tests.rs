use super::*;

#[test]
fn text_form_and_display_show_the_code() {
    let code = ErrorCode::new("E-PLT-001");
    assert_eq!(code.as_str(), "E-PLT-001");
    assert_eq!(code.to_string(), "E-PLT-001");
}

#[test]
fn codes_order_by_text() {
    assert!(ErrorCode::new("E-PLT-001") < ErrorCode::new("E-PLT-002"));
    assert!(ErrorCode::new("E-PLT-009") < ErrorCode::new("E-PLT-010"));
}

#[test]
fn range_check_accepts_codes_inside_the_ranges() {
    for code in [
        "E-PLT-001",
        "E-PLT-049",
        "E-PLT-050",
        "E-PLT-069",
        "E-RCP-050",
        "E-RCP-069",
        "E-WEB-019",
    ] {
        assert!(in_registry_range(code), "{code}");
    }
}

#[test]
fn range_check_rejects_codes_outside_the_ranges() {
    for code in [
        "E-PLT-070",
        "E-RCP-070",
        "E-IDN-020",
        "E-PLT-1",
        "E-XYZ-001",
        "E-PLT-0010",
        "X-PLT-001",
        "PLT-001",
        "E-PLT001",
    ] {
        assert!(!in_registry_range(code), "{code}");
    }
}

#[test]
fn every_registry_row_is_inside_its_range() {
    for code in REGISTRY {
        assert!(in_registry_range(code.as_str()), "{code}");
    }
}

#[test]
fn registry_is_sorted_and_unique() {
    for pair in REGISTRY.windows(2) {
        assert!(pair[0] < pair[1], "{} then {}", pair[0], pair[1]);
    }
}

/// The codes that tasks 2.1 adds, in text order.
const ADDED_CODES: [ErrorCode; 36] = [
    ErrorCode::AUD_001,
    ErrorCode::AUD_002,
    ErrorCode::DB_001,
    ErrorCode::DB_002,
    ErrorCode::DB_003,
    ErrorCode::DB_004,
    ErrorCode::DB_005,
    ErrorCode::DB_006,
    ErrorCode::DB_007,
    ErrorCode::DB_008,
    ErrorCode::EXP_001,
    ErrorCode::EXP_002,
    ErrorCode::EXP_003,
    ErrorCode::EXP_004,
    ErrorCode::PLT_030,
    ErrorCode::PLT_031,
    ErrorCode::PLT_032,
    ErrorCode::PLT_033,
    ErrorCode::PLT_034,
    ErrorCode::PLT_035,
    ErrorCode::PLT_036,
    ErrorCode::PLT_037,
    ErrorCode::PLT_038,
    ErrorCode::RCP_030,
    ErrorCode::RCP_031,
    ErrorCode::RCP_032,
    ErrorCode::RCP_033,
    ErrorCode::RCP_034,
    ErrorCode::RCP_035,
    ErrorCode::RCP_036,
    ErrorCode::WEB_001,
    ErrorCode::WEB_002,
    ErrorCode::WEB_004,
    ErrorCode::WEB_005,
    ErrorCode::WEB_009,
    ErrorCode::WEB_011,
];

#[test]
fn added_codes_are_in_registry() {
    for code in ADDED_CODES {
        assert!(REGISTRY.contains(&code), "{code}");
    }
}

/// The five codes that `WebError` of `recipe-web` owns (`reliability.md` section 1.1).
#[test]
fn web_error_codes_are_in_registry() {
    let owned = [
        (ErrorCode::WEB_003, "E-WEB-003"),
        (ErrorCode::WEB_006, "E-WEB-006"),
        (ErrorCode::WEB_007, "E-WEB-007"),
        (ErrorCode::WEB_008, "E-WEB-008"),
        (ErrorCode::WEB_010, "E-WEB-010"),
    ];
    for (code, text) in owned {
        assert_eq!(code.as_str(), text);
        assert_eq!(ErrorCode::find(text), Some(code));
    }
}

/// `E-WEB-012` is the code of `WebError::MissingBasicCredentials` (`reliability.md` section 1.2).
#[test]
fn web_012_is_in_registry() {
    assert!(
        ErrorCode::find("E-WEB-012").is_some(),
        "E-WEB-012 is missing from the registry"
    );
}

#[test]
fn find_returns_the_registry_entry_of_a_text() {
    assert_eq!(ErrorCode::find("E-RCP-031"), Some(ErrorCode::RCP_031));
}

#[test]
fn find_returns_none_for_an_unknown_text() {
    assert_eq!(ErrorCode::find("E-XYZ-001"), None);
}

#[test]
fn find_returns_every_registry_entry_for_its_text() {
    for code in REGISTRY {
        assert_eq!(ErrorCode::find(code.as_str()), Some(*code));
    }
}

/// The codes of the change `plc-connection-config` (`reliability.md` section 1.2).
#[test]
fn plc_connection_codes_are_in_registry() {
    let added = [
        ("E-PLT-022", ErrorCode::PLT_022),
        ("E-PLT-023", ErrorCode::PLT_023),
        ("E-PLT-024", ErrorCode::PLT_024),
        ("E-PLT-025", ErrorCode::PLT_025),
        ("E-PLT-026", ErrorCode::PLT_026),
        ("E-PLT-027", ErrorCode::PLT_027),
        ("E-PLT-028", ErrorCode::PLT_028),
        ("E-PLT-029", ErrorCode::PLT_029),
        ("E-WEB-013", ErrorCode::WEB_013),
        ("E-WEB-014", ErrorCode::WEB_014),
    ];
    for (text, code) in added {
        assert_eq!(code.as_str(), text);
        assert_eq!(ErrorCode::find(text), Some(code), "{text}");
    }
}

/// The codes of the change `machine-data-block-number` (`reliability.md` section 1.2).
#[test]
fn machine_data_block_codes_are_in_registry() {
    let added = [
        ("E-PLT-050", ErrorCode::PLT_050),
        ("E-PLT-051", ErrorCode::PLT_051),
    ];
    for (text, code) in added {
        assert_eq!(code.as_str(), text);
        assert_eq!(ErrorCode::find(text), Some(code), "{text}");
    }
}
