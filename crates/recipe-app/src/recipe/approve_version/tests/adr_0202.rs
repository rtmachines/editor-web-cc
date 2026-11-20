//! Tests of `ApproveVersion` for the STRING encoding of the machine (ADR-0202 (m), (k)).

use recipe_domain::recipe::ApprovalOutcome;

use super::run;
use crate::recipe::test_support::{UTF8_LABEL, seed_utf8_label, utf8_label_fixture};

#[tokio::test]
async fn adr_0202_version_read_paths_approve_checks_the_draft_as_utf8() {
    let (fixture, machine) = utf8_label_fixture();
    fixture.seed_draft(&machine, 1, "R-1");
    seed_utf8_label(&fixture, 101, UTF8_LABEL);

    let outcome = run(&fixture, 1, 1, false).await.unwrap();

    assert!(
        matches!(outcome, ApprovalOutcome::Approved { .. }),
        "{outcome:?}"
    );
}
