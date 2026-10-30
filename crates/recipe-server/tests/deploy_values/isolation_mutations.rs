//! Proof that the checks on the isolation of `pitr-check` and `restore-volume` can fail.
//!
//! The two services join no network and mount no secret (design D6 and D10, `compose.yaml`).
//! The service `restore-volume` also has a profile, one read-only mount, and one variable.
//! Each case changes one copy of `compose.yaml` and asserts a fault that names the service.

use crate::inputs::{Inputs, check_all};

const NETWORK_NONE: &str = "    network_mode: none\n";
const ISOLATED: [&str; 2] = ["pitr-check", "restore-volume"];
/// The lines of `restore-volume` after `network_mode` in `compose.yaml`. They are unique in the file.
const RESTORE_MOUNTS: &str = "    environment:\n      COMPOSE_SUBNET: ${COMPOSE_SUBNET:?}\n    volumes:\n      - backups:/backups:ro\n";
const RESTORE_ENTRYPOINT: &str = "    entrypoint: [\"restore-volume\"]\n";
const RESTORE_PROFILE: &str = "    profiles:\n      - restore\n";
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
fn an_isolated_service_without_network_mode_none_is_a_fault() {
    let removed = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, "");
    let bridge =
        Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, "    network_mode: bridge\n");
    for service in ISOLATED {
        assert_fault(&removed, &format!("{service}.network_mode"));
        assert_fault(&bridge, &format!("{service}.network_mode"));
    }
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
fn a_secret_on_an_isolated_service_is_a_fault() {
    let block = format!("{NETWORK_NONE}    secrets:\n      - app_password\n");
    let with_block = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &block);
    let flow = format!("{NETWORK_NONE}    secrets: [app_password]\n");
    let with_flow = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &flow);
    for service in ISOLATED {
        assert_fault(&with_block, &format!("{service}.secrets"));
        assert_fault(&with_flow, &format!("{service}.secrets"));
    }
}

#[test]
fn a_network_list_in_flow_style_is_read() {
    let block = format!("{NETWORK_NONE}    networks: [egress]\n");
    let flow = Inputs::load().with_replaced("compose.yaml", NETWORK_NONE, &block);
    for service in ISOLATED {
        assert_fault(&flow, &format!("{service}.networks"));
    }
}

#[test]
fn a_restore_volume_without_the_profile_restore_is_a_fault() {
    let removed = Inputs::load().with_replaced("compose.yaml", RESTORE_PROFILE, "");
    assert_fault(&removed, "restore-volume.profiles");
    let other = Inputs::load().with_replaced(
        "compose.yaml",
        RESTORE_PROFILE,
        "    profiles:\n      - pitr\n",
    );
    assert_fault(&other, "restore-volume.profiles");
}

#[test]
fn a_restore_volume_backup_mount_must_be_read_only() {
    let writable = RESTORE_MOUNTS.replace("backups:/backups:ro", "backups:/backups");
    let changed = Inputs::load().with_replaced("compose.yaml", RESTORE_MOUNTS, &writable);
    assert_fault(&changed, "restore-volume.volumes");
}

#[test]
fn a_restore_volume_mounts_no_pgdata_and_no_client_rules_file() {
    for extra in [
        "      - pgdata:/var/lib/postgresql/data\n",
        "      - ./db/cluster-access.sh:/etc/pgtools/cluster-access.sh:ro\n",
    ] {
        let more = format!("{RESTORE_MOUNTS}{extra}");
        let changed = Inputs::load().with_replaced("compose.yaml", RESTORE_MOUNTS, &more);
        assert_fault(&changed, "restore-volume.volumes");
    }
}

#[test]
fn a_restore_volume_needs_the_compose_subnet_variable() {
    let variable = "    environment:\n      COMPOSE_SUBNET: ${COMPOSE_SUBNET:?}\n";
    let without = RESTORE_MOUNTS.replace(variable, "");
    let removed = Inputs::load().with_replaced("compose.yaml", RESTORE_MOUNTS, &without);
    assert_fault(&removed, "restore-volume.environment.COMPOSE_SUBNET");
    let literal = RESTORE_MOUNTS.replace("${COMPOSE_SUBNET:?}", "172.28.0.0/16");
    let changed = Inputs::load().with_replaced("compose.yaml", RESTORE_MOUNTS, &literal);
    assert_fault(&changed, "restore-volume.environment.COMPOSE_SUBNET");
}

#[test]
fn a_restore_volume_entrypoint_is_the_script_and_it_has_no_command() {
    let removed = Inputs::load().with_replaced("compose.yaml", RESTORE_ENTRYPOINT, "");
    assert_fault(&removed, "restore-volume.entrypoint");
    let command = format!("{RESTORE_ENTRYPOINT}    command: [\"restore-volume\"]\n");
    let with_command = Inputs::load().with_replaced("compose.yaml", RESTORE_ENTRYPOINT, &command);
    assert_fault(&with_command, "restore-volume.command");
}
