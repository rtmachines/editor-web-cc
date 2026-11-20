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

use std::path::PathBuf;

use recipe_app::auth::CurrentUser;
use recipe_app::export::{ExportApprovedInput, ExportApprovedVersion, ExportScope};
use recipe_domain::identity::{Login, Role};
use recipe_domain::plant::UnitPath;
use recipe_domain::recipe::ReferenceNumber;
use serde_json::{Value, json};

use crate::stored_rules::{
    Case, ENGINEER, FIXTURE_DIRECTORY, approve_case, length_limit, plain_dump, range_limits,
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
