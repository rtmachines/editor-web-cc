//! Proof that the check of the Compose override files can fail (design D9, finding VX10-3).
//!
//! `compose.dev.yaml` may change the port, the volumes, and the secrets of `proxy`. `compose.build.yaml`
//! may add `build:` only. Any other key is a controlled setting, so the check reports a fault.

use crate::inputs::{Inputs, check_all};

const DEV: &str = "compose.dev.yaml";
const BUILD: &str = "compose.build.yaml";

/// Assert a fault that names the key and the file of the override.
fn assert_fault(inputs: &Inputs, key: &str, file: &str) {
    let report = check_all(inputs);
    assert!(
        report.has_fault_for(key) && report.has_fault_for(file),
        "no fault for {key:?} in {file:?}\nfaults:\n{}",
        report.message()
    );
}

#[test]
fn a_memory_limit_in_the_dev_override_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        DEV,
        "    secrets: !reset []",
        "    secrets: !reset []\n    mem_limit: 64m\n    memswap_limit: 64m",
    );
    assert_fault(&changed, "proxy.mem_limit", DEV);
    assert_fault(&changed, "proxy.memswap_limit", DEV);
}

#[test]
fn a_memory_limit_in_the_build_override_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        BUILD,
        "  app:\n    build: *server-build",
        "  app:\n    mem_limit: 64m\n    build: *server-build",
    );
    assert_fault(&changed, "app.mem_limit", BUILD);
}

#[test]
fn another_service_in_the_dev_override_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        DEV,
        "    secrets: !reset []",
        "    secrets: !reset []\n  app:\n    cpus: 2",
    );
    assert_fault(&changed, "app", DEV);
}

#[test]
fn a_top_level_section_in_an_override_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        BUILD,
        "services:",
        "networks:\n  backend:\n    internal: false\n\nservices:",
    );
    assert_fault(&changed, "networks", BUILD);
}

#[test]
fn a_service_in_flow_style_in_an_override_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        DEV,
        "    secrets: !reset []",
        "    secrets: !reset []\n  db: {mem_limit: 64m}",
    );
    assert_fault(&changed, "db", DEV);
}
