//! Tests of the export read path for the STRING encoding of the machine (ADR-0202 (m)).

use recipe_domain::identity::Role;
use recipe_domain::plant::{StringEncoding, UnitPath};

use super::test_doubles::{RecordingWriter, port};
use super::tests::approved_input;
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope, ExportedVersion};
use crate::memory::fixtures::{Fixture, actor, meta, reference, second_engineer};
use crate::recipe::test_support::{UTF8_LABEL, seed_utf8_label, utf8_label_fixture};
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
