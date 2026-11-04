//! Tests of the export of an approved version whose parameter has no type tree.
//!
//! The hash step runs before the rule check, so the answer is `E-EXP-002` and not `E-DB-008`.

use recipe_domain::error_codes::ErrorCode;
use recipe_domain::identity::Role;
use recipe_domain::plant::PlcPlatform;
use recipe_domain::recipe::StoredParameter;

use super::test_doubles::{RecordingWriter, port};
use super::{ExportApprovedInput, ExportApprovedVersion, ExportScope};
use crate::memory::fixtures::{Fixture, actor, meta, parameter_on, reference, tid, uid, vid};

/// A type identifier that no type row of the fixture holds.
const UNRESOLVABLE_TYPE: u128 = 9_999;

#[tokio::test]
async fn a_parameter_with_an_unresolvable_type_gives_exp_002_and_no_entry() {
    let fixture = Fixture::new();
    fixture.seed_actors();
    let machine = fixture.seed_machine_on(PlcPlatform::S7_1500, 1, "M1");
    fixture.seed_approved_and_draft(&machine, 1, "R-1");
    let lost = StoredParameter {
        plc_type: tid(UNRESOLVABLE_TYPE),
        ..parameter_on(PlcPlatform::S7_1500, 1, "LOST", uid(102), 0)
    };
    fixture.store.seed_parameter(vid(101), &lost);
    let writer = RecordingWriter::shared();

    let error = ExportApprovedVersion::new(fixture.context(), port(&writer))
        .execute(
            &actor(Role::Operator),
            &meta(),
            ExportApprovedInput {
                reference: reference("R-1"),
                machine: None,
                scope: ExportScope::Version,
            },
        )
        .await
        .unwrap_err();

    assert_eq!(error.code(), ErrorCode::EXP_002);
    assert!(fixture.store.audit_entries().is_empty());
    assert!(writer.calls().is_empty());
}
