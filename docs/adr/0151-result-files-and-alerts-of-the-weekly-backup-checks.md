# ADR-0151: Result files and alerts of the weekly backup checks

Status: Accepted. Amends ADR-0060 (a) (ports) and (y) (start grace), ADR-0066 (m) (mode of `/wal`), and ADR-0067 (step 9). Also amends ADR-0068 (g), ADR-0083 (b) (step 5), and ADR-0103 (s) (step 4)
Date: 2026-11-08
Deciders: project owner

## Context

The full repo review of 2026-11-08 found that no alert reads the result of the two weekly backup checks (finding D7-1). These facts shape the decision:

- `restore-test` runs in `backup` every Sunday at `restore_test.time`. `basebackup-once` runs in `walarchive` every Sunday at `basebackup.time` (ADR-0067 (a), ADR-0068 (f)).
- A failed run writes one `ERROR` line to the container log only. No condition of `reliability.md` §3.7 reads it.
- `backup_age` checks only that a complete set exists. `replication` checks only the WAL stream.
- At an incident, the newest set can fail to restore. Or the newest base backup can be old, so the replay can exceed the RTO of 1 hour (ADR-0020).
- `app` runs as uid 65532. It mounts `backups` and `walarchive` read-only (`deploy/compose.yaml`). The scripts run as uid 999 with `umask 077`.
- The top directory of `backups` has the mode `0755`. The top directory of `walarchive` has the mode `0700`, because `statvfs` needs no read right (ADR-0066 (m)). `app` therefore cannot open a file in `/wal` today.
- The scan of the backup directory has no time limit (finding D2-1). ADR-0133 gives the rule of a time limit for the disk adapter only.

The project owner chose a new alert condition on 2026-11-08 (decision Q-G (1) of `docs/plan/2026-11-08-fix-plan.md`).

## Decision drivers

- Data safety comes first (R31, `CLAUDE.md` "Purpose").
- The operator learns of a failed check without a person who reads the logs (R37, `reliability.md` §3.7).
- A script never gives a production secret or a host path to `app`.
- No new service, no new network, and no new key of `serve` (ADR-0020, ADR-0062).
- Every number comes from `reliability.md` §7.11.
- No read of a volume can stop the job `alerts` (R30, ADR-0133).

## Considered options

### Option A: Each check writes a result file, and the job `alerts` reads it

- Pros: no new service and no new connection. The read-only mounts of `app` already exist. The webhook path of §3.7 sends the alert.
- Cons: one new file on each backup disk. `app` needs a read right in `/wal`.

### Option B: A known gap and a weekly manual check of the logs

- Pros: no code.
- Cons: a person must read the logs each week. The owner rejected this option (decision Q-G (2)).

### Option C: Each script calls the webhook

- Pros: no new file.
- Cons: `backup` and `walarchive` have no network `egress` and no webhook secret. A second sender of alerts breaks the one place of ADR-0063.

## Decision

The project uses **Option A**.

Result files:

- (a) `restore-test` writes `/backups/restore-test.result`. `basebackup-once` writes `/wal/base-backup.result`.
- (b) A file holds one JSON line with exactly two keys: `result` and `finished_at`. `result` is `passed` or `failed`.
- (b1) `finished_at` is the end time in UTC in exactly one form: `YYYY-MM-DDTHH:MM:SSZ`. The `T` and the `Z` are uppercase. The form has no fraction of a second and no offset.
- (c) The file holds no path and no secret. It has at most `check_result.max_bytes` bytes. That limit is a row of §7.11 and no key of `serve`.
- (d) Neither name matches the set grammar or the base backup grammar of `reliability.md` §7.7, or the name of a WAL segment. The retention, `BackupDirectory`, and `pg_archivecleanup` therefore ignore both files.

Write rule:

- (e) A run writes its result file only while it holds its lock. A run without the marker file or without its lock writes no result file. The run that holds the lock writes the result.
- (f) The exit trap writes the file last, after the cleanup of the run. The exit code 0 gives `passed`. Every other exit code gives `failed`. A run that the kernel kills writes no file. The age limit of (i) covers that case.
- (f1) Each script sets its own traps for `TERM`, `INT`, and `HUP`. They exit with the codes 143, 130, and 129, as `restore-test` does today. A signal therefore never gives the exit trap the status 0.
- (g) The script writes the temp name `.tmp-<final name>` in the same directory, sets the mode `0644`, and renames it to the final name. The rename replaces the old result. ADR-0074 (b) still applies to the files of a set.

Alert conditions:

- (h) The job `alerts` tests two new conditions directly after `backup_age`: `restore_test` and `base_backup`. Their names follow ADR-0075.
- (i) A condition is active in four cases. The newest result is `failed`. The result is `passed` and older than its limit. The file is missing after the start grace. The file cannot be read or parsed.
- (j) The limits are the new rows `restore_test.max_age_alert` and `basebackup.max_age_alert` of §7.11, 8 days each. A weekly check needs a limit above 7 days. Both limits are no keys of `serve`.
- (j1) A read fault makes the condition active also during the start grace. The job writes one `WARN` line of the fault. This follows the rule of `backup_age` (ADR-0073 (c)).
- (j2) The job compares a result time with a comparison time. The comparison time is `database_now` plus the monotonic time from the snapshot to the end of the read.
- (j2a) A result time later than the comparison time by more than `alerts.interval` is untrusted, and the condition stays active (ADR-0093 (a)). Without (j2), a result that a run writes during the earlier reads can count as untrusted.
- (j3) The new reason type of a read fault owns no error code, like `BackupFault` (ADR-0025, ADR-0073 (b)). No row of §1.2 changes.

Read right in `/wal`:

- (k) The top directory of `walarchive` gets the mode `0711` (option K1). `app` can then open a file by its name, but it cannot list the names.
- (k1) `walarchive-entry` sets the mode at each start after the check of the marker. The runbook and the CI preparation create the directory with `0711`. This item amends ADR-0066 (m).

First run after the upgrade:

- (l) The first run of `alerts` after the upgrade does not start a clock. The start grace counts from the first start of the installation, as in ADR-0060 (x). Its length is the limit of each condition. This item amends ADR-0060 (y).
- (m) Step 5 of `reliability.md` §7.10 gets a fourth command: `docker compose exec walarchive basebackup-once`. Command 3 of step 5 already runs `restore-test`, so both files exist after step 5.
- (n) In the first upgrade to a release with this ADR, both alerts can be active from step 4 until step 5 ends. The release notes name these two alerts.

Time limit of the reads:

- (o) The scan of the backup directory and each read of a result file wait at most `health.disk_check_interval`. They follow ADR-0133 (b) to (e) with their own set of stalled paths.
- (p) One free function of `recipe-server` holds the time limit and the set of stalled paths. The disk adapter, the adapter of `BackupDirectory`, and the result reader all call it.

Switch of the WAL line:

- (q) Step 4 of ADR-0103 (s) also moves `/wal/base-backup.result` into the target directory. The result describes the base backups of the old cluster, and those base backups move too.
- (q1) The move deletes nothing, as ADR-0103 (t) requires. A killed first base backup of the new line then gives a missing file, not an old `passed` result.
- (q2) In a gap of the WAL stream (ADR-0099 (a)), the result file stays in `/wal`, because the cluster does not change.

Ports:

- (r) `recipe-app` gets the port `CheckResults`, and `recipe-server` implements it. This item amends the port list of ADR-0060 (a).

## Options for item (k)

The owner chose option K1 on 2026-11-08. The owner rejected options K2 and K3.

### Option K1: The mode `0711` on the top directory of `walarchive` (chosen)

- Pros: no new mount. `app` cannot list the WAL segments. The segments keep the mode `0600`, so `app` cannot read them.
- Cons: a change of ADR-0066 (m). An existing installation gets the new mode at the next start of `walarchive`.

### Option K2: `walarchive` also mounts `backups` and writes its result there (rejected)

- Pros: the mode of `/wal` stays `0700`.
- Cons: `walarchive` gets write access to the dumps. A fault of the script can then change a set.

### Option K3: The mode `0755` on the top directory of `walarchive` (rejected)

- Pros: the same mode as `backups`.
- Cons: `app` can list every WAL segment name. No reader needs that right.

## Consequences

### Positive

- A failed restore test or base backup reaches the webhook within `alerts.interval`.
- A check that stops without a result reaches the webhook after 8 days at most.
- A stalled backup mount cannot stop the job `alerts`.

### Negative

- The webhook can send `restore_test` and `base_backup` during the first upgrade with this ADR.
- Step 5 of the upgrade takes the time of one more base backup.
- The two backup disks hold one more file each.

### Risks and mitigations

- Risk: a result file of the old cluster stays in `/wal` after a switch of the WAL line. Mitigation: step 4 of ADR-0103 (s) moves it (item (q)).
- Risk: the mode `0711` lets every uid of the host open a file in `/wal` by its name. Mitigation: each file except the result file keeps the mode `0600`.
- Risk: a run in the container of the previous image during step 3 of §7.10 writes no result. Mitigation: command 3 of step 5 runs the new image and writes the result.

## Related

- ADR-0020, ADR-0025, ADR-0060 (a), (x), (y), ADR-0062, ADR-0063, ADR-0066 (m), ADR-0067, ADR-0068.
- ADR-0073 (b), (c), ADR-0074 (b), ADR-0075, ADR-0083 (b), ADR-0093 (a), ADR-0099 (a), ADR-0103 (s), (t), ADR-0133.
- `reliability.md` §1.1 (the reason type `CheckResultFault`), §3.7 (two new rows), and §7.7 (the result files and the WAL line).
- `reliability.md` §7.10 step 5 and §8 (the sets of stalled paths).
- `reliability.md` §7.11: the new rows `restore_test.max_age_alert` 8 days, `basebackup.max_age_alert` 8 days, and `check_result.max_bytes` 256 bytes.
- `domain-model.md` §5 (table of the ports of the jobs) and `overview.md` §5 (ports of `recipe-server`): the port `CheckResults`.
- Change `backup-result-alerts`, capability `operations`.
- `overview.md` §7: R30, R31, R37. §9: A16, A35.
