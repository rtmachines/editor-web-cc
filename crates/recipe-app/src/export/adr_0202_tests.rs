//! Tests of the export read path for the STRING encoding of the machine (ADR-0202 (m)).

use recipe_domain::identity::Role;

use super::ExportApprovedVersion;
use super::test_doubles::{RecordingWriter, port};
use super::tests::approved_input;
use crate::memory::fixtures::{actor, meta, reference, second_engineer};
use crate::recipe::test_support::{UTF8_LABEL, seed_utf8_label, utf8_label_fixture};
use crate::recipe::{ApproveVersion, ApproveVersionInput};

#[tokio::test]
async fn adr_0202_export_read_path_carries_utf8() {
    let (fixture, machine) = utf8_label_fixture();
    fixture.seed_actors();
    fixture.seed_draft(&machine, 1, "R-1");
    seed_utf8_label(&fixture, 101, UTF8_LABEL);
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

    let exported = ExportApprovedVersion::new(fixture.context(), port(&RecordingWriter::shared()))
        .execute(&actor(Role::Operator), &meta(), approved_input("R-1"))
        .await
        .unwrap();

    assert!(!exported.has_rule_fault);
    assert!(exported.loadable());
}
