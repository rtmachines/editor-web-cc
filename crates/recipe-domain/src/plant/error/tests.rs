use super::*;
use crate::error_codes::REGISTRY;

#[test]
fn unit_depth_exceeded_has_code_plt_002_and_the_section_1_2_text() {
    let error = PlantError::UnitDepthExceeded {
        path: "U1.2.3.4.5.6".to_owned(),
    };
    assert_eq!(error.code().as_str(), "E-PLT-002");
    assert_eq!(
        error.to_string(),
        "Unit path U1.2.3.4.5.6 has more than 5 levels. Use at most 5 levels."
    );
}

#[test]
fn invalid_unit_path_has_code_plt_017_and_the_section_1_2_text() {
    let error = PlantError::InvalidUnitPath {
        text: "U65".to_owned(),
    };
    assert_eq!(error.code().as_str(), "E-PLT-017");
    assert_eq!(
        error.to_string(),
        "U65 is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2."
    );
}

fn path(text: &str) -> UnitPath {
    UnitPath::parse(text).unwrap()
}

#[test]
fn machine_tree_errors_have_their_code_registry_row_and_text() {
    let code = MachineCode::parse("M7").unwrap();
    let cases = [
        (
            PlantError::ParentNotFound { path: path("U1.2") },
            ErrorCode::PLT_003,
            "E-PLT-003",
            "The parent unit of U1.2 does not exist. Create the parent unit first.",
        ),
        (
            PlantError::DuplicateUnitPath { path: path("U1.2") },
            ErrorCode::PLT_004,
            "E-PLT-004",
            "Unit U1.2 already exists on this machine.",
        ),
        (
            PlantError::LastMainUnit {
                code,
                path: path("U1"),
            },
            ErrorCode::PLT_008,
            "E-PLT-008",
            "Unit U1 is the last main unit of machine M7. A machine needs one main unit.",
        ),
        (
            PlantError::UnitArchived { path: path("U1.2") },
            ErrorCode::PLT_009,
            "E-PLT-009",
            "Unit U1.2 is archived. Select a live unit.",
        ),
        (
            PlantError::MachineArchived { code },
            ErrorCode::PLT_010,
            "E-PLT-010",
            "Machine M7 is archived.",
        ),
    ];
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn machine_edit_errors_have_their_code_registry_row_and_text() {
    let code = MachineCode::parse("M7").unwrap();
    let reference = ReferenceNumber::parse("r-100").unwrap();
    let cases = [
        (
            PlantError::DuplicateMachineCode { code },
            ErrorCode::PLT_001,
            "E-PLT-001",
            "Machine code M7 is already in use. Use a different code.",
        ),
        (
            PlantError::MachineHasDraft { code, reference },
            ErrorCode::PLT_011,
            "E-PLT-011",
            "Machine M7 has a draft in recipe R-100. Approve or discard the draft first.",
        ),
        (
            PlantError::MachineNotFound { code },
            ErrorCode::PLT_019,
            "E-PLT-019",
            "Machine M7 does not exist. Select an existing machine.",
        ),
        (
            PlantError::UnitNotFound { path: path("U1.2") },
            ErrorCode::PLT_020,
            "E-PLT-020",
            "Unit U1.2 does not exist on this machine. Select an existing unit.",
        ),
    ];
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn unit_usage_errors_have_their_code_registry_row_and_text() {
    let reference = ReferenceNumber::parse("r-100").unwrap();
    let cases = [
        (
            PlantError::UnitReferencedByDraft {
                path: path("U1.2"),
                reference: reference.clone(),
            },
            ErrorCode::PLT_012,
            "E-PLT-012",
            "Unit U1.2 is used by the draft of recipe R-100. Move or delete those parameters first.",
        ),
        (
            PlantError::UnitReferencedByApproved {
                path: path("U1.2"),
                reference,
            },
            ErrorCode::PLT_014,
            "E-PLT-014",
            "Unit U1.2 is used by the approved version of recipe R-100. An Administrator can retire the version.",
        ),
    ];
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn derived_type_invalid_has_code_plt_013_and_the_section_1_2_text() {
    let error = PlantError::DerivedTypeInvalid {
        type_expr: "STRING(255)".to_owned(),
        reason: DerivedTypeReason::OutOfRange,
    };
    assert_eq!(error.code(), ErrorCode::PLT_013);
    assert_eq!(error.code().as_str(), "E-PLT-013");
    assert!(REGISTRY.contains(&ErrorCode::PLT_013));
    assert_eq!(
        error.to_string(),
        "The type STRING(255) is not valid: a bound, a dimension, or a length is outside its range. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL."
    );
}

#[test]
fn type_limits_show_the_numbers_of_section_7_11() {
    assert_eq!(TypeLimit::Leaves.to_string(), "16384 leaves");
    assert_eq!(TypeLimit::Depth.to_string(), "32 levels");
    assert_eq!(TypeLimit::Members.to_string(), "1024 members");
}

#[test]
fn type_errors_have_their_code_registry_row_and_text() {
    let name = TypeName::parse("ST_Zone").unwrap();
    let member = MemberName::parse("speed").unwrap();
    let cases = [
        (
            PlantError::TypeCycle {
                type_name: name.clone(),
            },
            ErrorCode::PLT_005,
            "E-PLT-005",
            "Type ST_Zone contains itself. Delete the circular member.",
        ),
        (
            PlantError::MemberTypeNotOnPlatform {
                type_name: name.clone(),
                member: member.clone(),
                platform: PlcPlatform::S7_300,
            },
            ErrorCode::PLT_006,
            "E-PLT-006",
            "Member speed of ST_Zone uses a type that does not exist on S7_300.",
        ),
        (
            PlantError::DuplicateTypeName {
                type_name: name.clone(),
            },
            ErrorCode::PLT_015,
            "E-PLT-015",
            "Type ST_Zone already exists on this machine. Create a new revision instead.",
        ),
        (
            PlantError::StructHasNoMembers {
                type_name: name.clone(),
            },
            ErrorCode::PLT_016,
            "E-PLT-016",
            "Type ST_Zone has no members. Add at least one member.",
        ),
        (
            PlantError::TypeTooLarge {
                type_name: "ST_Zone".to_owned(),
                limit: TypeLimit::Members,
            },
            ErrorCode::PLT_018,
            "E-PLT-018",
            "Type ST_Zone is larger than the limit 1024 members. Use fewer elements, fewer members, or fewer levels.",
        ),
        (
            PlantError::DuplicateMemberName {
                type_name: name,
                member,
            },
            ErrorCode::PLT_021,
            "E-PLT-021",
            "Type ST_Zone has two members with the name speed. Give each member a different name.",
        ),
    ];
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}

#[test]
fn derived_type_reasons_have_their_phrase() {
    let cases = [
        (
            DerivedTypeReason::Grammar,
            "the text does not match the type grammar",
        ),
        (
            DerivedTypeReason::UnknownStruct,
            "the struct or the revision does not exist on the machine",
        ),
        (
            DerivedTypeReason::NotOnPlatform,
            "the type does not exist on the platform of the machine",
        ),
        (
            DerivedTypeReason::OutOfRange,
            "a bound, a dimension, or a length is outside its range",
        ),
        (
            DerivedTypeReason::RowShape,
            "the stored type row does not match its kind",
        ),
    ];
    for (reason, text) in cases {
        assert_eq!(reason.to_string(), text);
    }
}

#[test]
fn registry_contains_both_plant_codes() {
    assert!(REGISTRY.contains(&ErrorCode::PLT_002));
    assert!(REGISTRY.contains(&ErrorCode::PLT_017));
}

/// Return the E-PLT rows of `reliability.md` section 1.2 as (code, variant, template).
fn plant_doc_rows() -> Vec<(String, String, String)> {
    const DOC: &str = include_str!("../../../../../docs/architecture/reliability.md");
    DOC.lines()
        .filter(|line| line.starts_with("| `E-PLT-"))
        .filter_map(|line| {
            let cells: Vec<&str> = line.split('|').map(str::trim).collect();
            let code = cells.get(1)?.trim_matches('`');
            let owner = cells.get(2)?.trim_matches('`');
            let variant = owner
                .strip_prefix("PlantError::")?
                .split([' ', '{', '('])
                .next()?;
            let template = cells.get(4)?;
            Some((code.to_owned(), variant.to_owned(), (*template).to_owned()))
        })
        .collect()
}

/// Replace each `{field}` of a template with the text of its value.
fn fill(template: &str, fields: &[(&str, String)]) -> String {
    fields
        .iter()
        .fold(template.to_owned(), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
}

fn machine_code() -> MachineCode {
    MachineCode::parse("M7").unwrap()
}

fn reference() -> ReferenceNumber {
    ReferenceNumber::parse("r-100").unwrap()
}

fn zone() -> TypeName {
    TypeName::parse("ST_Zone").unwrap()
}

fn speed() -> MemberName {
    MemberName::parse("speed").unwrap()
}

/// One sample of each `PlantError` variant, with the display text of each field.
fn plant_samples() -> Vec<(PlantError, Vec<(&'static str, String)>)> {
    vec![
        (
            PlantError::UnitDepthExceeded {
                path: "U1.2.3.4.5.6".to_owned(),
            },
            vec![("path", "U1.2.3.4.5.6".to_owned())],
        ),
        (
            PlantError::DuplicateMachineCode {
                code: machine_code(),
            },
            vec![("code", machine_code().to_string())],
        ),
        (
            PlantError::ParentNotFound { path: path("U1.2") },
            vec![("path", path("U1.2").to_string())],
        ),
        (
            PlantError::DuplicateUnitPath { path: path("U1.2") },
            vec![("path", path("U1.2").to_string())],
        ),
        (
            PlantError::TypeCycle { type_name: zone() },
            vec![("type_name", zone().to_string())],
        ),
        (
            PlantError::MemberTypeNotOnPlatform {
                type_name: zone(),
                member: speed(),
                platform: PlcPlatform::S7_300,
            },
            vec![
                ("type_name", zone().to_string()),
                ("member", speed().to_string()),
                ("platform", PlcPlatform::S7_300.to_string()),
            ],
        ),
        (
            PlantError::LastMainUnit {
                code: machine_code(),
                path: path("U1"),
            },
            vec![
                ("code", machine_code().to_string()),
                ("path", path("U1").to_string()),
            ],
        ),
        (
            PlantError::UnitArchived { path: path("U1.2") },
            vec![("path", path("U1.2").to_string())],
        ),
        (
            PlantError::MachineArchived {
                code: machine_code(),
            },
            vec![("code", machine_code().to_string())],
        ),
        (
            PlantError::MachineHasDraft {
                code: machine_code(),
                reference: reference(),
            },
            vec![
                ("code", machine_code().to_string()),
                ("reference", reference().to_string()),
            ],
        ),
        (
            PlantError::UnitReferencedByDraft {
                path: path("U1.2"),
                reference: reference(),
            },
            vec![
                ("path", path("U1.2").to_string()),
                ("reference", reference().to_string()),
            ],
        ),
        (
            PlantError::UnitReferencedByApproved {
                path: path("U1.2"),
                reference: reference(),
            },
            vec![
                ("path", path("U1.2").to_string()),
                ("reference", reference().to_string()),
            ],
        ),
        (
            PlantError::DuplicateTypeName { type_name: zone() },
            vec![("type_name", zone().to_string())],
        ),
        (
            PlantError::StructHasNoMembers { type_name: zone() },
            vec![("type_name", zone().to_string())],
        ),
        (
            PlantError::TypeTooLarge {
                type_name: "ST_Zone".to_owned(),
                limit: TypeLimit::Members,
            },
            vec![
                ("type_name", "ST_Zone".to_owned()),
                ("limit", TypeLimit::Members.to_string()),
            ],
        ),
        (
            PlantError::DuplicateMemberName {
                type_name: zone(),
                member: speed(),
            },
            vec![
                ("type_name", zone().to_string()),
                ("member", speed().to_string()),
            ],
        ),
        (
            PlantError::DerivedTypeInvalid {
                type_expr: "STRING(255)".to_owned(),
                reason: DerivedTypeReason::OutOfRange,
            },
            vec![
                ("type_expr", "STRING(255)".to_owned()),
                ("reason", DerivedTypeReason::OutOfRange.to_string()),
            ],
        ),
        (
            PlantError::InvalidUnitPath {
                text: "U65".to_owned(),
            },
            vec![("text", "U65".to_owned())],
        ),
        (
            PlantError::MachineNotFound {
                code: machine_code(),
            },
            vec![("code", machine_code().to_string())],
        ),
        (
            PlantError::UnitNotFound { path: path("U1.2") },
            vec![("path", path("U1.2").to_string())],
        ),
    ]
    .into_iter()
    .chain(connection_samples())
    .collect()
}

/// One sample of each connection variant, with the display text of each field.
fn connection_samples() -> Vec<(PlantError, Vec<(&'static str, String)>)> {
    let code = ("code", machine_code().to_string());
    let address = PlcAddress::parse("plc-1").unwrap();
    vec![
        (
            PlantError::ConnectionFieldNotOnPlatform {
                code: machine_code(),
                field: "rack",
                platform: PlcPlatform::TwinCat3,
            },
            vec![
                code.clone(),
                ("field", "rack".to_owned()),
                ("platform", PlcPlatform::TwinCat3.to_string()),
            ],
        ),
        (
            PlantError::ConnectionFieldMissing {
                code: machine_code(),
                field: "slot",
            },
            vec![code.clone(), ("field", "slot".to_owned())],
        ),
        (
            PlantError::ConnectionDataBlockOutOfRange {
                code: machine_code(),
                number: 16001,
                max: 16000,
            },
            vec![
                code.clone(),
                ("number", "16001".to_owned()),
                ("max", "16000".to_owned()),
            ],
        ),
        (
            PlantError::CommModeNotOnPlatform {
                code: machine_code(),
                mode: CommMode::Ads,
                platform: PlcPlatform::CodesysIec,
            },
            vec![
                code.clone(),
                ("mode", CommMode::Ads.to_string()),
                ("platform", PlcPlatform::CodesysIec.to_string()),
            ],
        ),
        (
            PlantError::AddressNotAmsNetId {
                code: machine_code(),
                address: address.clone(),
            },
            vec![code.clone(), ("address", address.to_string())],
        ),
        (
            PlantError::ConnectionNotConfigured {
                code: machine_code(),
            },
            vec![code.clone()],
        ),
        (
            PlantError::ConnectionFieldNotInMode {
                code: machine_code(),
                field: "login",
                mode: CommMode::ModbusTcp,
            },
            vec![
                code.clone(),
                ("field", "login".to_owned()),
                ("mode", CommMode::ModbusTcp.to_string()),
            ],
        ),
        (
            PlantError::ConnectionRecipeMismatch {
                code: machine_code(),
                reference: reference(),
            },
            vec![code, ("reference", reference().to_string())],
        ),
    ]
}

#[test]
fn registry_matches_reliability() {
    let rows = plant_doc_rows();
    let expected: Vec<String> = (1..=6)
        .chain(8..=29)
        .map(|number| format!("E-PLT-{number:03}"))
        .collect();

    // Assertion 1: the doc has the 28 domain rows, in order.
    let doc_codes: Vec<String> = rows.iter().map(|row| row.0.clone()).collect();
    assert_eq!(doc_codes, expected);

    // Assertion 2: REGISTRY holds the same E-PLT codes.
    let registry: Vec<String> = REGISTRY
        .iter()
        .map(|code| code.as_str().to_owned())
        .filter(|code| {
            code.strip_prefix("E-PLT-")
                .and_then(|digits| digits.parse::<u32>().ok())
                .is_some_and(|number| number < 30)
        })
        .collect();
    assert_eq!(registry, expected);

    // Assertions 3 and 4: each variant has one sample that matches its doc row.
    let samples = plant_samples();
    let mut sample_codes: Vec<String> = Vec::new();
    for (error, fields) in &samples {
        let code = error.code().as_str().to_owned();
        let row = rows
            .iter()
            .find(|row| row.0 == code)
            .expect("the code of a sample has a doc row");
        let debug = format!("{error:?}");
        let debug_variant = debug.split([' ', '{', '(']).next().unwrap_or("");
        assert_eq!(debug_variant, row.1.as_str(), "{code}");
        let text = fill(&row.2, fields);
        assert!(!text.contains('{'), "{code} has an unfilled field");
        assert_eq!(error.to_string(), text, "{code}");
        sample_codes.push(code);
    }
    assert_eq!(samples.len(), rows.len());
    for code in &expected {
        let count = sample_codes.iter().filter(|c| *c == code).count();
        assert_eq!(count, 1, "{code} has {count} samples");
    }
}

#[test]
fn connection_errors_have_their_code_registry_row_and_text() {
    let code = MachineCode::parse("M7").unwrap();
    let reference = ReferenceNumber::parse("r-100").unwrap();
    let address = PlcAddress::parse("plc-1").unwrap();
    let cases = [
        (
            PlantError::ConnectionFieldNotOnPlatform {
                code,
                field: "rack",
                platform: PlcPlatform::TwinCat3,
            },
            ErrorCode::PLT_022,
            "E-PLT-022",
            "The connection of machine M7 has no field rack on platform TwinCat3. Delete the value of rack.",
        ),
        (
            PlantError::ConnectionFieldMissing {
                code,
                field: "slot",
            },
            ErrorCode::PLT_023,
            "E-PLT-023",
            "The connection of machine M7 needs a value in field slot. Enter a value.",
        ),
        (
            PlantError::ConnectionDataBlockOutOfRange {
                code,
                number: 16001,
                max: 16000,
            },
            ErrorCode::PLT_024,
            "E-PLT-024",
            "Data block 16001 of machine M7 is above 16000. The controller of the machine has no such data block.",
        ),
        (
            PlantError::CommModeNotOnPlatform {
                code,
                mode: CommMode::Ads,
                platform: PlcPlatform::CodesysIec,
            },
            ErrorCode::PLT_025,
            "E-PLT-025",
            "Platform CodesysIec of machine M7 does not support the mode Ads. Select a mode of this platform.",
        ),
        (
            PlantError::AddressNotAmsNetId { code, address },
            ErrorCode::PLT_026,
            "E-PLT-026",
            "Address plc-1 of machine M7 is not an AMS Net ID. The mode Ads needs an AMS Net ID.",
        ),
        (
            PlantError::ConnectionNotConfigured { code },
            ErrorCode::PLT_027,
            "E-PLT-027",
            "Machine M7 has no connection settings. Enter the settings on the page of the machine first.",
        ),
        (
            PlantError::ConnectionFieldNotInMode {
                code,
                field: "login",
                mode: CommMode::ModbusTcp,
            },
            ErrorCode::PLT_028,
            "E-PLT-028",
            "The mode ModbusTcp of machine M7 does not use field login. Delete the value of login.",
        ),
        (
            PlantError::ConnectionRecipeMismatch { code, reference },
            ErrorCode::PLT_029,
            "E-PLT-029",
            "Recipe R-100 does not belong to machine M7. Use a recipe of this machine.",
        ),
    ];
    for (error, expected, text, message) in cases {
        assert_eq!(error.code(), expected);
        assert_eq!(error.code().as_str(), text);
        assert!(REGISTRY.contains(&expected), "{text}");
        assert_eq!(error.to_string(), message);
    }
}
