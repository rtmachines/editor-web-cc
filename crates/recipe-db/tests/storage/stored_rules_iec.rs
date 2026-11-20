//! Stored rows of the earlier release and the rules of `codesys-twincat-type-rules` (`design.md`,
//! section "Stored data matrix").
//!
//! The ignored test `adr_0202_stored_rules_iec_write_the_fixture` builds the six rows of the
//! matrix with the use cases of the code before the change. It runs before the migration `0017`
//! exists. It writes three files:
//!
//! - `fixtures/stored_rules_iec.sql`, a full plain dump of that database, with `_sqlx_migrations`.
//! - `fixtures/stored_rules_iec.hashes`, one line `<reference> <content_sha256> <scope_sha256>`
//!   for each case.
//! - `fixtures/stored_rules_iec.exports.json`, two captures of each case and the value of
//!   `loadable`: one export of the whole version and one export of the unit `U1`.
//!
//! The tests of task 10.1 load the dump into an empty database, run the migrator of this release,
//! and compare the exports with these files. The pattern and the helpers come from
//! `stored_rules.rs`.

use std::path::{Path, PathBuf};

use recipe_app::auth::CurrentUser;
use recipe_app::export::{ExportApprovedInput, ExportApprovedVersion, ExportScope};
use recipe_app::recipe::{GetVersion, GetVersionInput};
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::{StringEncoding, UnitPath};
use recipe_domain::recipe::ReferenceNumber;
use serde_json::{Value, json};

use crate::stored_rules::{
    Case, ENGINEER, FIXTURE_DIRECTORY, approve_case, backup_into_copy, copy_row, length_limit,
    plain_dump, range_limits, restored_and_upgraded_from, stored_rows,
};
use crate::support::app::App;
use crate::support::identity::PASSWORD;
use crate::support::writers::DocumentFactsWriter;

/// The unit of every case. Each recipe has one parameter on it.
const SCOPE_UNIT: &str = "U1";

/// The case of a `WSTRING` value with a character outside the Basic Multilingual Plane.
fn wstring_case(reference: &'static str, platform: &'static str, value: &str) -> Case {
    Case {
        reference,
        platform,
        structs: Vec::new(),
        type_text: "WSTRING(10)",
        address: None,
        value: format!("\"{value}\""),
        limits: vec![length_limit("", 10)],
    }
}

/// The case of a parameter of one stored struct.
fn struct_case(
    reference: &'static str,
    platform: &'static str,
    (name, members): (&str, &[(&'static str, &str)]),
    (type_text, value): (&'static str, &str),
    ranged_leaves: &[&str],
) -> Case {
    Case {
        reference,
        platform,
        structs: vec![(
            name.to_owned(),
            members
                .iter()
                .map(|(member, member_type)| (*member, (*member_type).to_owned()))
                .collect(),
        )],
        type_text,
        address: None,
        value: value.to_owned(),
        limits: range_limits(ranged_leaves, "0", "100"),
    }
}

/// The six rows of the matrix, in the order of `design.md`.
fn matrix() -> Vec<Case> {
    vec![
        wstring_case("SR-IEC-WSTRING-TC", "TwinCat3", "A\u{1F600}"),
        wstring_case("SR-IEC-WSTRING-CS", "CodesysIec", "Z\u{1D11E}"),
        struct_case(
            "SR-IEC-RESERVED-STRUCT",
            "TwinCat3",
            ("VERSION", &[("major", "INT"), ("minor", "INT")]),
            ("VERSION@1", r#"{"major":1,"minor":2}"#),
            &[".major", ".minor"],
        ),
        struct_case(
            "SR-IEC-KEYWORD-MEMBER",
            "CodesysIec",
            ("ST_Coil", &[("END_IF", "BOOL"), ("S", "BOOL")]),
            ("ST_Coil@1", r#"{"END_IF":true,"S":false}"#),
            &[],
        ),
        struct_case(
            "SR-IEC-ONE-MEMBER",
            "CodesysIec",
            ("ST_One", &[("value", "INT")]),
            ("ST_One@1", r#"{"value":3}"#),
            &[".value"],
        ),
        Case {
            reference: "SR-IEC-STRING-LATIN1",
            platform: "CodesysIec",
            structs: Vec::new(),
            type_text: "STRING(4)",
            address: None,
            value: "\"\u{E9}\"".to_owned(),
            limits: vec![length_limit("", 4)],
        },
    ]
}

/// One export of a case: the value of `loadable` and the capture of the writer.
struct Capture {
    /// The value of `ExportedVersion::loadable`.
    loadable: bool,
    /// The document of `DocumentFactsWriter`, parsed. It holds no `exported_at`.
    document: Value,
}

impl Capture {
    /// The capture as the JSON of the fixture file.
    fn to_json(&self) -> Value {
        json!({ "loadable": self.loadable, "capture": self.document })
    }
}

/// Export the approved version of a reference with the test writer and capture the result.
///
/// The function also asserts the state of the rows before the change: the version is approved,
/// loadable, and has no rule fault, and the hash of its body equals the stored hash.
async fn export_case(app: &App, reference: &str, scope: ExportScope) -> Capture {
    let input = ExportApprovedInput {
        reference: ReferenceNumber::parse(reference).unwrap(),
        machine: None,
        scope,
    };
    let exported = ExportApprovedVersion::new(app.context(), DocumentFactsWriter::port())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    assert!(
        exported.loadable(),
        "{reference} is loadable before the change"
    );
    assert!(!exported.has_rule_fault, "{reference} has no rule fault");
    let document: Value = serde_json::from_slice(&exported.document).unwrap();
    assert_eq!(document["loadable"], Value::Bool(true));
    assert_eq!(document["has_rule_fault"], Value::Bool(false));
    assert_eq!(document["status"], "Approved");
    assert_eq!(
        document["content_sha256"],
        exported.content_sha256.to_string()
    );
    Capture {
        loadable: exported.loadable(),
        document,
    }
}

/// The lines `<reference> <content_sha256> <scope_sha256>`, ordered by reference.
///
/// The content hash is the stored hash of the version. The scope hash comes from the audit entry
/// `VersionExported` of the unit export.
async fn hash_lines(app: &mut App) -> String {
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT r.reference_number, v.content_sha256, a.after_state->>'scope_sha256' \
         FROM audit_log a \
         JOIN recipe_version v ON v.id = a.entity_id \
         JOIN recipe r ON r.id = v.recipe_id \
         WHERE a.action = 'VersionExported' AND a.after_state->>'scope_sha256' IS NOT NULL \
         AND v.status = 'approved' ORDER BY r.reference_number",
    )
    .fetch_all(app.admin())
    .await
    .unwrap();
    rows.iter()
        .map(|(reference, content, scope)| format!("{reference} {content} {scope}\n"))
        .collect()
}

/// Write the fixture of task 10.1. Run it once with the code before the change:
/// `cargo test -p recipe-db --test storage adr_0202_stored_rules_iec_write_the_fixture -- --ignored`.
/// A second run gives other UUIDs.
#[tokio::test]
#[ignore = "writes the fixture files once, with the code before the change"]
async fn adr_0202_stored_rules_iec_write_the_fixture() {
    let mut app = App::create().await;
    let id = app
        .seed_user(ENGINEER, Role::ProcessEngineer, PASSWORD)
        .await;
    let engineer = CurrentUser::new(id, Login::parse(ENGINEER).unwrap(), Role::ProcessEngineer);
    let cases = matrix();
    for case in &cases {
        approve_case(&mut app, &engineer, case).await;
    }

    let scope = ExportScope::Unit(UnitPath::parse(SCOPE_UNIT).unwrap());
    let mut references: Vec<&str> = cases.iter().map(|case| case.reference).collect();
    references.sort_unstable();
    let mut exports = Vec::new();
    for reference in references {
        let version = export_case(&app, reference, ExportScope::Version).await;
        let unit = export_case(&app, reference, scope.clone()).await;
        exports.push(json!({
            "reference": reference,
            "version": version.to_json(),
            "unit": unit.to_json(),
        }));
    }

    let hashes = hash_lines(&mut app).await;
    assert_eq!(hashes.lines().count(), cases.len());
    for (line, export) in hashes.lines().zip(&exports) {
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(Value::from(fields[0]), export["reference"]);
        assert_eq!(
            Value::from(fields[1]),
            export["version"]["capture"]["content_sha256"]
        );
        assert_eq!(
            Value::from(fields[1]),
            export["version"]["capture"]["body_sha256"]
        );
        assert_eq!(
            Value::from(fields[2]),
            export["unit"]["capture"]["scope_sha256"]
        );
        assert_eq!(
            Value::from(fields[2]),
            export["unit"]["capture"]["body_sha256"]
        );
    }
    let mut exports_text = serde_json::to_string_pretty(&exports).unwrap();
    exports_text.push('\n');
    let dump = plain_dump(app.database_name());

    let directory = PathBuf::from(FIXTURE_DIRECTORY);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(directory.join("stored_rules_iec.sql"), dump).unwrap();
    std::fs::write(directory.join("stored_rules_iec.hashes"), hashes).unwrap();
    std::fs::write(
        directory.join("stored_rules_iec.exports.json"),
        exports_text,
    )
    .unwrap();
}

// Task 10.1: the stored rows of the matrix on the migrator of this release.

/// The stored fixture: the full plain dump of the earlier release.
const FIXTURE_DUMP: &str = "stored_rules_iec.sql";
/// The stored fixture: one line `<reference> <content_sha256> <scope_sha256>` for each case.
const FIXTURE_HASHES: &str = "stored_rules_iec.hashes";
/// The stored fixture: the captures of each case.
const FIXTURE_EXPORTS: &str = "stored_rules_iec.exports.json";

/// What a row of the matrix expects after the upgrade.
struct Expectation {
    /// The reference number of the recipe.
    reference: &'static str,
    /// The value of `loadable` in the matrix.
    loadable: bool,
}

/// The stored content hash and scope hash of a reference, from the fixture file.
fn stored_hashes(reference: &str) -> (String, String) {
    let text = std::fs::read_to_string(Path::new(FIXTURE_DIRECTORY).join(FIXTURE_HASHES)).unwrap();
    let fields = text
        .lines()
        .map(|line| line.split(' ').collect::<Vec<_>>())
        .find(|fields| fields.first() == Some(&reference))
        .ok_or_else(|| format!("the fixture has no hashes for {reference}"))
        .unwrap();
    assert_eq!(fields.len(), 3);
    (fields[1].to_owned(), fields[2].to_owned())
}

/// The fixture entry of a reference: the keys `reference`, `unit`, and `version`.
fn stored_exports(reference: &str) -> Value {
    let text = std::fs::read_to_string(Path::new(FIXTURE_DIRECTORY).join(FIXTURE_EXPORTS)).unwrap();
    let entries: Vec<Value> = serde_json::from_str(&text).unwrap();
    entries
        .into_iter()
        .find(|entry| entry["reference"] == reference)
        .ok_or_else(|| format!("the fixture has no exports for {reference}"))
        .unwrap()
}

/// Export the approved version of a reference after the upgrade and capture the result.
async fn capture_after(app: &App, reference: &str, scope: ExportScope) -> Capture {
    let input = ExportApprovedInput {
        reference: ReferenceNumber::parse(reference).unwrap(),
        machine: None,
        scope,
    };
    let exported = ExportApprovedVersion::new(app.context(), DocumentFactsWriter::port())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    let document: Value = serde_json::from_slice(&exported.document).unwrap();
    assert_eq!(document["loadable"], Value::Bool(exported.loadable()));
    assert_eq!(
        document["has_rule_fault"],
        Value::Bool(exported.has_rule_fault)
    );
    Capture {
        loadable: exported.loadable(),
        document,
    }
}

/// The capture as JSON without the two fields that ADR-0201 (i) changes.
fn without_fault_fields(mut capture: Value) -> Value {
    if let Some(fields) = capture.as_object_mut() {
        fields.remove("loadable");
    }
    if let Some(document) = capture["capture"].as_object_mut() {
        document.remove("loadable");
        document.remove("has_rule_fault");
    }
    capture
}

/// Assert one capture against the fixture, with the rule of ADR-0201 (i) or ADR-0202 (j2).
fn assert_capture(reference: &str, name: &str, actual: &Capture, expected: &Value, loadable: bool) {
    assert_eq!(actual.loadable, loadable, "{reference} {name}: loadable");
    let machine = &actual.document["body"]["machine"];
    assert!(
        machine.get("string_encoding").is_none(),
        "{reference} {name}: the machine has no field string_encoding"
    );
    let canonical = actual.document["canonical_body"].as_str().unwrap();
    assert!(!canonical.contains("string_encoding"), "{reference} {name}");
    let actual_json = actual.to_json();
    if loadable {
        assert_eq!(&actual_json, expected, "{reference} {name}");
    } else {
        assert_eq!(actual.document["has_rule_fault"], Value::Bool(true));
        assert_eq!(
            without_fault_fields(actual_json),
            without_fault_fields(expected.clone()),
            "{reference} {name}"
        );
    }
}

/// Step 5: both exports and both hashes equal the fixture.
async fn assert_exports(app: &App, expected: &Expectation) {
    let reference = expected.reference;
    let (content, scope_hash) = stored_hashes(reference);
    let fixture = stored_exports(reference);
    let scope = ExportScope::Unit(UnitPath::parse(SCOPE_UNIT).unwrap());
    let version = capture_after(app, reference, ExportScope::Version).await;
    let unit = capture_after(app, reference, scope).await;
    assert_eq!(version.document["content_sha256"], content.as_str());
    assert_eq!(version.document["body_sha256"], content.as_str());
    assert_eq!(unit.document["scope_sha256"], scope_hash.as_str());
    assert_eq!(unit.document["body_sha256"], scope_hash.as_str());
    assert_capture(
        reference,
        "version",
        &version,
        &fixture["version"],
        expected.loadable,
    );
    assert_capture(
        reference,
        "unit",
        &unit,
        &fixture["unit"],
        expected.loadable,
    );
}

/// Step 4: `GetVersion` reads the stored row and the machine keeps the encoding `Latin1`.
async fn read_row(app: &mut App, expected: &Expectation) {
    let input = GetVersionInput {
        reference: ReferenceNumber::parse(expected.reference).unwrap(),
        number: 1,
        rebind_bound: 0,
    };
    let view = GetVersion::new(app.context())
        .execute(&app.user, &app.meta, input)
        .await
        .unwrap();
    assert_eq!(
        view.detail.content_sha256.map(|hash| hash.to_string()),
        Some(stored_hashes(expected.reference).0)
    );
    assert_eq!(view.rule_codes.is_empty(), expected.loadable);
    assert_eq!(view.machine.string_encoding(), StringEncoding::Latin1);
}

/// The eight steps of task 9.2 of `siemens-type-rules` for one row of the matrix.
async fn check_stored_row(expected: Expectation) {
    let (mut app, before) = restored_and_upgraded_from(FIXTURE_DUMP, expected.reference).await;
    read_row(&mut app, &expected).await;
    assert_exports(&app, &expected).await;
    assert_eq!(stored_rows(app.admin(), expected.reference).await, before);
    copy_row(&mut app, expected.reference).await;
    let restored = backup_into_copy(&app).await;
    assert_exports(&restored, &expected).await;
}

#[tokio::test]
async fn adr_0201_stored_wstring_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-WSTRING-TC",
        loadable: false,
    })
    .await;
}

#[tokio::test]
async fn adr_0201_stored_wstring_codesys_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-WSTRING-CS",
        loadable: false,
    })
    .await;
}

#[tokio::test]
async fn adr_0203_stored_reserved_struct_name_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-RESERVED-STRUCT",
        loadable: true,
    })
    .await;
}

#[tokio::test]
async fn adr_0203_stored_keyword_member_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-KEYWORD-MEMBER",
        loadable: true,
    })
    .await;
}

#[tokio::test]
async fn adr_0204_stored_one_member_struct_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-ONE-MEMBER",
        loadable: true,
    })
    .await;
}

#[tokio::test]
async fn adr_0202_stored_latin1_string_exports_with_its_hash() {
    check_stored_row(Expectation {
        reference: "SR-IEC-STRING-LATIN1",
        loadable: true,
    })
    .await;
}
