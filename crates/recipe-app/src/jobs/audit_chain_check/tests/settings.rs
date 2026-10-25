//! The settings of the job: the defaults equal `reliability.md` section 7.11.

use crate::jobs::audit_chain_check::AuditChainCheckSettings;

#[test]
fn the_defaults_equal_the_table_of_section_7_11() {
    let settings = AuditChainCheckSettings::default();
    assert_eq!(settings.batch_rows, 10_000);
    assert_eq!(settings.batch_bytes, 8 * 1024 * 1024);
}
