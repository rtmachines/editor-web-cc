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
        "COPY --chmod=0644 restore-code.sql restore-code.expected restore-roles.sql restore-roles.expected /etc/pgtools/",
        "COPY --chmod=0644 lib/globals-grammar.sh lib/recovery-steps.sh lib/restore-steps.sh lib/wal-identity.sh /etc/pgtools/",
        "COPY --chmod=0644 --from=db cluster-access.sh /etc/pgtools/cluster-access.sh",
    ] {
        let copy = position(instruction);
        assert!(
            directory < copy && copy < user,
            "create /etc/pgtools with mode 0755 before {instruction} and USER"
        );
    }
}

#[test]
fn pgtools_creates_the_pgpass_mount_point_before_user() {
    // runc gives a tmpfs the mode of an existing mount point. Without the directory in the image,
    // a restart finds the mount point of the first start with mode 0755 and the owner root.
    let dockerfile = include_str!("../../../deploy/pgtools/Dockerfile");
    let lines = lines::code_lines(dockerfile);
    let position = |instruction| lines.iter().position(|line| line.trim() == instruction);
    let directory = position("RUN mkdir -p -m 1777 /run/pgpass")
        .expect("pgtools Dockerfile must create the mount point /run/pgpass with mode 1777");
    let user = position("USER 999:999").expect("pgtools Dockerfile must set USER 999:999");
    assert!(directory < user, "root must create /run/pgpass before USER");
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

fn function_body<'a>(common: &'a str, name: &str) -> Vec<&'a str> {
    let header = format!("{name}() {{");
    lines::code_lines(common)
        .into_iter()
        .map(str::trim)
        .skip_while(|line| *line != header)
        .take_while(|line| *line != "}")
        .collect()
}

fn delete_ci_volumes_body(common: &str) -> Vec<&str> {
    function_body(common, "delete_ci_volumes")
}

fn delete_labelled_volume_body(common: &str) -> Vec<&str> {
    function_body(common, "delete_labelled_volume")
}

#[test]
fn deploy_values_ci_cleanup_continues_after_failures() {
    let cleanup = include_str!("../../../deploy/ci/cleanup.sh");
    let code: Vec<_> = lines::code_lines(cleanup)
        .into_iter()
        .map(str::trim)
        .collect();
    assert!(
        !code.iter().any(|line| line.contains("|| exit")),
        "cleanup must save failures and continue"
    );
    assert!(code.windows(2).any(|pair| pair
        == [
            "status=0",
            "compose down --volumes --remove-orphans || status=$?",
        ]));
    assert!(code.contains(&"delete_ci_volumes || status=$?"));
    let common = include_str!("../../../deploy/ci/common.sh");
    let delete = delete_ci_volumes_body(common);
    assert!(!delete.iter().any(|line| line.contains("|| exit")));
    assert!(delete.contains(&r#"delete_labelled_volume "$volume" || status=$?"#));
    assert!(delete.ends_with(&[r#"return "$status""#]));
    assert!(
        !delete_labelled_volume_body(common)
            .iter()
            .any(|line| line.contains("|| exit"))
    );
    assert!(code.ends_with(&[
        "for disk in backups walarchive-disk",
        "do",
        r#"if mountpoint -q "$CI_ROOT/$disk""#,
        "then",
        r#"sudo umount "$CI_ROOT/$disk" || status=$?"#,
        "fi",
        "done",
        r#"exit "$status""#,
    ]));
}

#[test]
fn deploy_values_ci_volume_deletions_stay_in_throwaway_scripts() {
    let deploy = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy");
    let mut actual = Vec::new();
    for (path, script) in deploy_scripts(&deploy) {
        let relative: Vec<_> = path
            .strip_prefix(&deploy)
            .unwrap()
            .components()
            .map(|part| part.as_os_str().to_string_lossy().into_owned())
            .collect();
        for command in volume_deletions(&script) {
            actual.push((relative.join("/"), command));
        }
    }
    let mut expected: Vec<(String, String)> = vec![
        (
            "ci/cleanup.sh".into(),
            "compose down --volumes --remove-orphans || status=$?".into(),
        ),
        (
            "ci/common.sh".into(),
            r#"docker volume rm "$1" >/dev/null"#.into(),
        ),
        (
            "ci/restore-switch.sh".into(),
            r#"compose --profile restore --profile pitr down -v --remove-orphans || fail "keep: the project stop failed.""#.into(),
        ),
        (
            "ci/upgrade.sh".into(),
            "compose --profile upgrade-rollback down -v --remove-orphans || status=$?".into(),
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
        "status=0",
        "compose down --volumes --remove-orphans || status=$?",
        "delete_ci_volumes || status=$?",
        "for disk in backups walarchive-disk",
    ]));
    let common = include_str!("../../../deploy/ci/common.sh");
    assert!(common.contains(
        "# The operator rule in docs/ops/runbook.md still applies: never delete the pgdata volume of a production host."
    ));
    assert!(lines::code_lines(common).contains(&"VOLUME_LABEL_MISMATCH=3"));
    assert!(function_body(common, "has_job_label").contains(
        &r#"[[ $(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$1") == "$COMPOSE_PROJECT_NAME" ]]"#
    ));
    assert_eq!(
        delete_labelled_volume_body(common),
        [
            "delete_labelled_volume() {",
            r#"has_job_label "$1" || return "$VOLUME_LABEL_MISMATCH""#,
            r#"docker volume rm "$1" >/dev/null"#,
        ]
    );
    assert!(delete_ci_volumes_body(common).starts_with(&[
        "delete_ci_volumes() {",
        "local ids status=0 volume",
        r#"ids=$(docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME") || return 1"#,
        "for volume in $ids",
        "do",
        r#"delete_labelled_volume "$volume" || status=$?"#,
        "done",
        r#"return "$status""#,
    ]));
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

#[test]
fn deploy_values_ci_diagnostics_reject_silent_exits() {
    let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/ci");
    for (path, script) in deploy_scripts(&directory) {
        let name = path.file_name().unwrap().to_str().unwrap();
        if name == "cleanup.sh" {
            continue;
        }
        for (index, line) in script.lines().enumerate() {
            if name == "common.sh" && matches!(index, 6 | 7) {
                continue;
            }
            assert!(
                !line.contains("|| exit 1") && !line.contains("*) exit 1"),
                "{} has a silent exit: {line}",
                path.display()
            );
        }
    }
}

#[test]
fn deploy_values_ci_diagnostics_name_mode_failures() {
    let script = include_str!("../../../deploy/ci/modes.sh");
    let script = script.replace("\\\n", " ");
    for check in [
        r#"[[ $(sudo stat -c %a "$RECIPE_BACKUP_DIR") == 755 ]]"#,
        r#"[[ $(sudo stat -c %a "$RECIPE_WALARCHIVE_DIR") == 700 ]]"#,
        r#"test "$(stat -c %a "$path")" = 700"#,
        r#""$found""#,
    ] {
        assert!(
            script
                .lines()
                .any(|line| { line.trim().starts_with(check) && line.contains("|| fail ") }),
            "the mode check needs a diagnostic: {check}"
        );
    }
}

#[test]
fn deploy_values_ci_diagnostics_print_step_evidence() {
    let workflow = include_str!("../../../.github/workflows/ci.yml");
    assert!(workflow.replace("\r\n", "\n").contains(
        "if: failure()\n        timeout-minutes: 3\n        run: bash deploy/ci/diagnostics.sh"
    ));
    let diagnostics = include_str!("../../../deploy/ci/diagnostics.sh");
    for (producer, evidence) in [
        (include_str!("../../../deploy/ci/isolation.sh"), "tcp.log"),
        (
            include_str!("../../../deploy/ci/same-filesystem.sh"),
            "same-filesystem.log",
        ),
    ] {
        assert!(producer.contains(&format!(r#">"$CI_STATE/{evidence}" 2>&1"#)));
        assert!(
            diagnostics
                .lines()
                .any(|line| { line.contains("redact.py") && line.contains(evidence) }),
            "diagnostics must print {evidence} through the redactor"
        );
    }
}
