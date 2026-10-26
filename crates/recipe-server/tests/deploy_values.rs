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
fn deploy_values_ci_same_filesystem_drops_all_capabilities() {
    let script = include_str!("../../../deploy/ci/same-filesystem.sh");
    let script = script.replace("\\\r\n", " ").replace("\\\n", " ");
    let lines = lines::code_lines(&script);
    let runs: Vec<_> = lines
        .iter()
        .filter(|line| line.contains("docker run "))
        .collect();
    assert_eq!(runs.len(), 2, "retain both fixture and healthcheck runs");
    for run in runs {
        let options = run.split(r#""recipe-pgtools:"#).next().unwrap();
        let words: Vec<_> = options.split_whitespace().collect();
        assert!(
            words.windows(2).any(|pair| pair == ["--cap-drop", "ALL"]),
            "each run must drop all capabilities: {run}"
        );
        assert!(
            words
                .windows(2)
                .any(|pair| pair == ["--security-opt", "no-new-privileges:true"]),
            "each run must prevent new privileges: {run}"
        );
        let added: Vec<_> = words
            .windows(2)
            .filter_map(|pair| (pair[0] == "--cap-add").then_some(pair[1]))
            .collect();
        if words.windows(2).any(|pair| pair == ["--user", "0:0"]) {
            assert_eq!(added, ["CHOWN"], "the fixture needs only CHOWN");
        } else {
            assert!(added.is_empty(), "backup-health needs no capabilities");
        }
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

fn deploy_scripts(directory: &std::path::Path) -> Vec<(std::path::PathBuf, String)> {
    let mut scripts = Vec::new();
    for entry in std::fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            scripts.extend(deploy_scripts(&path));
            continue;
        }
        let bytes = std::fs::read(&path).unwrap();
        let extension = path.extension().and_then(|value| value.to_str());
        if bytes.starts_with(b"#!")
            || matches!(
                extension,
                Some("sh" | "bash" | "py" | "ps1" | "cmd" | "bat")
            )
        {
            scripts.push((path, String::from_utf8(bytes).unwrap()));
        }
    }
    scripts
}

fn volume_deletions(script: &str) -> Vec<String> {
    let script = script.replace("\\\r\n", " ").replace("\\\n", " ");
    lines::code_lines(&script)
        .into_iter()
        .filter_map(|line| {
            let words: Vec<_> = line.split_whitespace().collect();
            let deletion = words
                .windows(2)
                .any(|pair| matches!(pair, ["volume", "rm" | "prune"] | ["system", "prune"]));
            let down = words.iter().position(|word| *word == "down");
            let volumes = down.is_some_and(|position| {
                words.iter().skip(position).any(|word| {
                    *word == "--volumes"
                        || (word.starts_with('-') && !word.starts_with("--") && word.contains('v'))
                })
            });
            (deletion || volumes).then(|| words.join(" "))
        })
        .collect()
}

#[test]
fn deploy_values_ci_volume_deletions_stay_in_throwaway_scripts() {
    let deploy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy");
    let mut actual = Vec::new();
    for (path, script) in deploy_scripts(&deploy) {
        for command in volume_deletions(&script) {
            actual.push((path.strip_prefix(&deploy).unwrap().to_path_buf(), command));
        }
    }
    let mut expected = vec![
        (
            "ci/cleanup.sh".into(),
            "compose down --volumes --remove-orphans || exit 1".into(),
        ),
        (
            "ci/cleanup.sh".into(),
            r#"docker volume rm "$volume" >/dev/null"#.into(),
        ),
        (
            "ci/upgrade.sh".into(),
            "compose --profile upgrade-rollback down -v --remove-orphans".into(),
        ),
    ];
    actual.sort();
    expected.sort();
    assert_eq!(
        actual, expected,
        "deploy scripts must retain only the CI deletions"
    );
}

#[test]
fn deploy_values_ci_volume_removal_keeps_label_and_name_guards() {
    let cleanup = include_str!("../../../deploy/ci/cleanup.sh");
    let code: Vec<_> = lines::code_lines(cleanup)
        .into_iter()
        .map(str::trim)
        .collect();
    assert!(code.starts_with(&[
        "set -euo pipefail",
        "umask 077",
        r#"source "$(dirname "${BASH_SOURCE[0]}")/common.sh""#,
        "compose down --volumes --remove-orphans || exit 1",
        "volume=${COMPOSE_PROJECT_NAME}-one-filesystem",
        r#"if docker volume inspect "$volume" >/dev/null 2>&1"#,
        "then",
        r#"[[ $(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$volume") == "$COMPOSE_PROJECT_NAME" ]]"#,
        r#"docker volume rm "$volume" >/dev/null"#,
        "fi",
    ]));
    assert!(cleanup.contains(
        "# The label check and run-scoped name restrict deletion to this fixture on a throwaway CI runner.\n  # The operator rule in docs/ops/runbook.md still applies: never run docker volume rm on pgdata."
    ));
    let common = include_str!("../../../deploy/ci/common.sh");
    assert!(lines::code_lines(common).starts_with(&[
        "set -euo pipefail",
        "umask 077",
        r#": "${GITHUB_WORKSPACE:?}" "${RUNNER_TEMP:?}" "${COMPOSE_PROJECT_NAME:?}""#,
        "[[ ${GITHUB_ACTIONS:-} == true ]] || exit 1",
        "[[ $COMPOSE_PROJECT_NAME =~ ^recipe-ci-[0-9]+-[0-9]+$ ]] || exit 1",
    ]));
    assert!(common.contains(r#"docker compose --project-name "$COMPOSE_PROJECT_NAME""#));
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    assert!(workflow.contains(
        "COMPOSE_PROJECT_NAME: recipe-ci-${{ github.run_id }}-${{ github.run_attempt }}"
    ));
}

#[test]
fn deploy_values_ci_upgrade_volume_deletion_keeps_project_guards() {
    let upgrade = include_str!("../../../deploy/ci/upgrade.sh");
    assert!(lines::code_lines(upgrade).starts_with(&[
        "set -euo pipefail",
        r#"source "$(dirname "${BASH_SOURCE[0]}")/upgrade-common.sh""#,
    ]));
    let common = include_str!("../../../deploy/ci/upgrade-common.sh");
    assert!(lines::code_lines(common).starts_with(&[
        "set -euo pipefail",
        r#"source "$(dirname "${BASH_SOURCE[0]}")/common.sh""#,
        r#"[[ ${CI:-} == true && ${RUNNER_OS:-} == Linux ]] || fail "the upgrade test requires Linux CI""#,
        r#"[[ $COMPOSE_PROJECT_NAME == "recipe-ci-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}" ]] \"#,
        r#"  || fail "the upgrade project does not belong to this job""#,
    ]));
    assert!(common.contains(r#"docker compose -p "$COMPOSE_PROJECT_NAME""#));
}

#[test]
fn deploy_values_ci_volume_scan_detects_destructive_variants() {
    for command in [
        "docker volume rm pgdata",
        "sudo docker volume  rm pgdata",
        "docker volume \\\nrm pgdata",
        "docker volume prune -f",
        "docker system prune --volumes",
        "docker compose down -v",
        "docker compose down --volumes",
        "compose --project-name production down --remove-orphans -v",
    ] {
        assert_eq!(volume_deletions(command).len(), 1, "missed {command}");
    }
    assert!(volume_deletions("# docker volume rm pgdata\ndocker volume inspect pgdata").is_empty());
}
