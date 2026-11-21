//! Tests of the export read path for the STRING encoding of the machine (ADR-0202 (m)).

use recipe_domain::audit::AuditAction;
use recipe_domain::identity::Role;
use recipe_domain::plant::{PlcPlatform, StringEncoding, UnitPath};
use serde_json::{Value, json};

use super::test_doubles::{RecordingWriter, port};
use super::tests::approved_input;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportedVersion};
use crate::memory::fixtures::{Fixture, actor, meta, reference, second_engineer};
use crate::recipe::test_support::{
    UTF8_LABEL, seed_label_row, seed_utf8_label, utf8_label_fixture,
};
use crate::recipe::{ApproveVersion, ApproveVersionInput};

/// Store the approved version 1 of `R-1` on the `Utf8` machine `M1` with the label `"Łódź"`.
fn approved_utf8_fixture() -> Fixture {
    let (fixture, machine) = utf8_label_fixture();
    fixture.seed_actors();
    fixture.seed_draft(&machine, 1, "R-1");
    seed_utf8_label(&fixture, 101, UTF8_LABEL);
    fixture
}

/// Approve the draft of `R-1`, then export it as an Operator through `writer`.
async fn approve_and_export(
    fixture: &Fixture,
    writer: &std::sync::Arc<RecordingWriter>,
    scope: ExportScope,
) -> ExportedVersion {
    ApproveVersion::new(fixture.context())
        .execute(
            &second_engineer(),
            &meta(),
            ApproveVersionInput {
                reference: reference("R-1"),
                number: 1,
                expected_row_version: 1,
                allow_outdated: false,
            },
        )
        .await
        .unwrap();
    let input = ExportApprovedInput {
        scope,
        ..approved_input("R-1")
    };
    ExportApprovedVersion::new(fixture.context(), port(writer))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap()
}

#[tokio::test]
async fn adr_0202_export_read_path_carries_utf8() {
    let fixture = approved_utf8_fixture();
    let writer = RecordingWriter::shared();

    let exported = approve_and_export(&fixture, &writer, ExportScope::Version).await;

    assert!(!exported.has_rule_fault);
    assert_eq!(writer.last().1.string_encoding, StringEncoding::Utf8);
}

#[tokio::test]
async fn adr_0202_v1_of_utf8_is_not_loadable() {
    let fixture = approved_utf8_fixture();
    let writer = RecordingWriter::shared();

    let exported = approve_and_export(&fixture, &writer, ExportScope::Version).await;

    assert!(!exported.has_rule_fault);
    assert!(exported.status.loadable());
    assert!(!exported.loadable());
    let facts = writer.last().1;
    assert!(facts.loadable());
    assert!(!facts.v1_loadable());
}

#[tokio::test]
async fn adr_0202_v2_of_utf8_keeps_loadable() {
    let fixture = approved_utf8_fixture();
    let writer = RecordingWriter::shared();
    let scope = ExportScope::Unit(UnitPath::parse("U1").unwrap());

    let exported = approve_and_export(&fixture, &writer, scope).await;

    assert!(!exported.has_rule_fault);
    assert!(exported.loadable());
    let (scope_body, facts) = writer.scoped_calls().pop().unwrap();
    assert_eq!(scope_body.string_encoding(), StringEncoding::Utf8);
    assert!(facts.document.loadable());
}

/// Store the approved version 1 of `R-1` on a `Latin1` `CodesysIec` machine with the label `"é"`.
fn approved_latin1_fixture() -> Fixture {
    let fixture = Fixture::new();
    let machine = fixture.seed_machine_on(PlcPlatform::CodesysIec, 1, "M1");
    seed_label_row(&fixture, &machine);
    fixture.seed_actors();
    fixture.seed_draft(&machine, 1, "R-1");
    seed_utf8_label(&fixture, 101, "é");
    fixture
}

/// Export the approved `R-1` with the scope `U1` once more as an Operator through `writer`.
async fn export_unit_u1(fixture: &Fixture, writer: &std::sync::Arc<RecordingWriter>) {
    let input = ExportApprovedInput {
        scope: ExportScope::Unit(UnitPath::parse("U1").unwrap()),
        ..approved_input("R-1")
    };
    ExportApprovedVersion::new(fixture.context(), port(writer))
        .execute(&actor(Role::Operator), &meta(), input)
        .await
        .unwrap();
}

/// The `scope_sha256` of each `VersionExported` entry, as text, in the order of the journal.
fn exported_scope_hashes(fixture: &Fixture) -> Vec<String> {
    fixture
        .store
        .audit_entries()
        .iter()
        .filter(|entry| entry.action() == AuditAction::VersionExported)
        .filter_map(|entry| {
            entry
                .after()?
                .get("scope_sha256")?
                .as_str()
                .map(str::to_owned)
        })
        .collect()
}

/// The canonical text of the last scope body that `writer` got, and its hash as text.
fn last_scope(writer: &RecordingWriter) -> (String, String) {
    let (body, facts) = writer.scoped_calls().pop().unwrap();
    assert_eq!(facts.scope_sha256, body.sha256().unwrap());
    (
        body.canonical_text().unwrap(),
        facts.scope_sha256.to_string(),
    )
}

#[tokio::test]
async fn adr_0202_unit_scope_of_a_utf8_machine_holds_the_encoding_after_the_platform() {
    let fixture = approved_utf8_fixture();
    let writer = RecordingWriter::shared();
    let scope = ExportScope::Unit(UnitPath::parse("U1").unwrap());

    approve_and_export(&fixture, &writer, scope).await;

    let (text, hash) = last_scope(&writer);
    assert!(
        text.contains(r#""plc_platform":"CodesysIec","string_encoding":"Utf8"}"#),
        "{text}"
    );
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        body["machine"],
        json!({ "code": "M1", "plc_platform": "CodesysIec", "string_encoding": "Utf8" })
    );
    assert_eq!(exported_scope_hashes(&fixture), vec![hash]);
}

#[tokio::test]
async fn adr_0202_unit_scope_of_a_latin1_machine_holds_no_encoding_and_keeps_its_hash() {
    let fixture = approved_latin1_fixture();
    let writer = RecordingWriter::shared();
    let scope = ExportScope::Unit(UnitPath::parse("U1").unwrap());
    approve_and_export(&fixture, &writer, scope).await;
    let (text, first_hash) = last_scope(&writer);

    export_unit_u1(&fixture, &writer).await;

    assert!(!text.contains("string_encoding"), "{text}");
    let body: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        body["machine"],
        json!({ "code": "M1", "plc_platform": "CodesysIec" })
    );
    let (second_text, second_hash) = last_scope(&writer);
    assert_eq!(second_text, text);
    assert_eq!(second_hash, first_hash);
    assert_eq!(
        exported_scope_hashes(&fixture),
        vec![first_hash.clone(), first_hash]
    );
}

#[tokio::test]
async fn adr_0202_unit_export_of_a_latin1_codesys_machine_with_a_latin_1_value_has_no_rule_fault() {
    let fixture = approved_latin1_fixture();
    let writer = RecordingWriter::shared();
    let scope = ExportScope::Unit(UnitPath::parse("U1").unwrap());

    let exported = approve_and_export(&fixture, &writer, scope).await;

    assert!(!exported.has_rule_fault);
    assert!(exported.loadable());
    let (body, _facts) = writer.scoped_calls().pop().unwrap();
    assert_eq!(body.string_encoding(), StringEncoding::Latin1);
}
