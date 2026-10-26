#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]
// Test code may use unwrap, expect, and indexing (reliability.md section 2.3).

//! Compares the values in the files of `deploy/` with `docs/architecture/reliability.md` section 7.11
//! (design D14, row T23 of the change `operations-compose`).
//!
//! The test reads files only. It needs no Docker. Each check takes the text of a file as an argument,
//! so the mutation tests feed a changed copy and assert a fault.

// The modules live in `tests/deploy_values/`. A module of a test root file looks beside the root file,
// so each `mod` line names its path.
#[path = "deploy_values/canon.rs"]
mod canon;
#[path = "deploy_values/command_mutations.rs"]
mod command_mutations;
#[path = "deploy_values/compose.rs"]
mod compose;
#[path = "deploy_values/compose_mutations.rs"]
mod compose_mutations;
#[path = "deploy_values/db_command.rs"]
mod db_command;
#[path = "deploy_values/default_mutations.rs"]
mod default_mutations;
#[path = "deploy_values/defaults.rs"]
mod defaults;
#[path = "deploy_values/edge_guards.rs"]
mod edge_guards;
#[path = "deploy_values/fault.rs"]
mod fault;
#[path = "deploy_values/file_mutations.rs"]
mod file_mutations;
#[path = "deploy_values/files.rs"]
mod files;
#[path = "deploy_values/inputs.rs"]
mod inputs;
#[path = "deploy_values/isolation_mutations.rs"]
mod isolation_mutations;
#[path = "deploy_values/lines.rs"]
mod lines;
#[path = "deploy_values/list_mutations.rs"]
mod list_mutations;
#[path = "deploy_values/override_mutations.rs"]
mod override_mutations;
#[path = "deploy_values/overrides.rs"]
mod overrides;
#[path = "deploy_values/port_mutations.rs"]
mod port_mutations;
#[path = "deploy_values/rule_mutations.rs"]
mod rule_mutations;
#[path = "deploy_values/rules.rs"]
mod rules;
#[path = "deploy_values/schedule_mutations.rs"]
mod schedule_mutations;
#[path = "deploy_values/secret_mutations.rs"]
mod secret_mutations;
#[path = "deploy_values/secrets.rs"]
mod secrets;
#[path = "deploy_values/table.rs"]
mod table;
#[path = "deploy_values/topology.rs"]
mod topology;
#[path = "deploy_values/unit_mutations.rs"]
mod unit_mutations;
#[path = "deploy_values/units.rs"]
mod units;

use inputs::{Inputs, check_all};

#[test]
fn deploy_values_equal_the_settings_table() {
    let report = check_all(&Inputs::load());
    assert!(
        report.faults.is_empty(),
        "{} value(s) of deploy/ differ from reliability.md 7.11:\n{}",
        report.faults.len(),
        report.message()
    );
}

#[test]
fn pgtools_creates_a_searchable_config_directory_before_copy() {
    let dockerfile = include_str!("../../../deploy/pgtools/Dockerfile");
    let lines = lines::code_lines(dockerfile);
    let position = |instruction| {
        lines
            .iter()
            .position(|line| line.trim() == instruction)
            .expect("pgtools Dockerfile must retain the config directory, copies, and user")
    };
    // COPY uses its file mode for a missing destination directory too.
    let directory = position("RUN mkdir -p -m 0755 /etc/pgtools");
    let user = position("USER 999:999");
    assert!(directory < user, "root must create /etc/pgtools");
    for instruction in [
        "COPY --chmod=0644 crontab.backup crontab.walarchive /etc/pgtools/",
        "COPY --chmod=0644 restore-code.sql restore-code.expected /etc/pgtools/",
    ] {
        let copy = position(instruction);
        assert!(
            directory < copy && copy < user,
            "create /etc/pgtools with mode 0755 before {instruction} and USER"
        );
    }
}

#[test]
fn deploy_values_ci_memory_logs_each_cgroup_metric() {
    let script = include_str!("../../../deploy/ci/memory.sh");
    let lines = lines::code_lines(script);
    for instruction in [
        r#"peak=$(sudo -n cat "$path/memory.peak" 2>/dev/null) || peak="#,
        r#"anon=$(sudo -n awk '$1 == "anon" { print $2 }' "$path/memory.stat" 2>/dev/null) || anon="#,
        r#"file=$(sudo -n awk '$1 == "file" { print $2 }' "$path/memory.stat" 2>/dev/null) || file="#,
        r#"oom_kill=$(sudo -n awk '$1 == "oom_kill" { print $2 }' "$path/memory.events" 2>/dev/null) || oom_kill="#,
        r#"memory_log "memory.peak $service peak=$peak limit=$limit anon=$anon file=$file oom_kill=$oom_kill""#,
    ] {
        assert!(
            lines.iter().any(|line| line.trim() == instruction),
            "missing {instruction}"
        );
    }
    for metric in ["peak", "limit", "anon", "file", "oom_kill"] {
        let guard = format!("[[ ${metric} =~ ^[0-9]+$ ]] || {metric}=unavailable");
        assert!(
            lines.iter().any(|line| line.trim() == guard),
            "missing {guard}"
        );
    }
}

#[test]
fn deploy_values_ci_memory_fails_only_on_oom_kills() {
    let script = include_str!("../../../deploy/ci/memory.sh");
    assert!(script.contains(
        r#"if [[ $oom_kill =~ ^[0-9]+$ && $oom_kill =~ [1-9] ]]; then
    memory_log "memory.oom_kill $service oom_kill=$oom_kill: OOM kill detected"
    return 1
  fi
  return 0"#
    ));
    let reader = script.split("memory_cgroup() {").nth(1).unwrap();
    let reader = reader.split("\n}").next().unwrap();
    assert_eq!(reader.matches("return 1").count(), 1);
    assert!(!reader.contains("(("), "the peak must remain log-only");
}

#[test]
fn deploy_values_ci_memory_propagates_oom_failures() {
    let script = include_str!("../../../deploy/ci/memory.sh");
    for instruction in [
        r#"memory_cgroup "$service" "${path:-/sys/fs/cgroup/unavailable}" "$limit""#,
        r#"memory_service "$service" || status=$?"#,
        r#"memory_peak "$service" "$id" || status=$?"#,
        r#"memory_peak pitr-check "$name" || status=$?"#,
        r#"wait "$watcher" || status=$?"#,
    ] {
        assert!(script.contains(instruction), "missing {instruction}");
    }
    assert!(!script.contains(r#"wait "$watcher" || true"#));
}
