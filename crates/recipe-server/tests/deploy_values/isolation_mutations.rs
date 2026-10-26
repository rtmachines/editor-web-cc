//! Proof that the checks on the isolation of `pitr-check` can fail.
//!
//! The service `pitr-check` joins no network and mounts no secret (design D10, `compose.yaml`).
//! Each case changes one copy of `compose.yaml` and asserts a fault that names the service.

use crate::inputs::{Inputs, check_all};

const NETWORK_NONE: &str = "    network_mode: none\n";
const OTHER_SERVICES: [&str; 7] = [
    "app",
    "backup",
    "backup-premigrate",
    "db",
    "migrate",
    "proxy",
    "walarchive",
];

fn assert_fault(inputs: &Inputs, part: &str) {
    let report = check_all(inputs);
    assert!(
        report.has_fault_for(part),
        "no fault for {part:?}\nfaults:\n{}",
        report.message()
    );
}

#[test]
fn a_pitr_check_without_network_mode_none_is_a_fault() {
    let removed = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, "");
    assert_fault(&removed, "pitr-check.network_mode");
    let bridge =
        Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, "    network_mode: bridge\n");
    assert_fault(&bridge, "pitr-check.network_mode");
}

#[test]
fn a_network_mode_on_another_service_is_a_fault() {
    let changed = Inputs::load().with_replaced(
        "compose.yaml",
        "\n    cap_drop:\n",
        "\n    network_mode: none\n    cap_drop:\n",
    );
    for service in OTHER_SERVICES {
        assert_fault(&changed, &format!("{service}.network_mode"));
    }
}

#[test]
fn a_secret_on_pitr_check_is_a_fault() {
    let block = format!("{NETWORK_NONE}    secrets:\n      - app_password\n");
    let with_block = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &block);
    assert_fault(&with_block, "pitr-check.secrets");
    let flow = format!("{NETWORK_NONE}    secrets: [app_password]\n");
    let with_flow = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &flow);
    assert_fault(&with_flow, "pitr-check.secrets");
}

#[test]
fn a_network_list_in_flow_style_is_read() {
    let block = format!("{NETWORK_NONE}    networks: [egress]\n");
    let flow = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &block);
    assert_fault(&flow, "pitr-check.networks");
}
