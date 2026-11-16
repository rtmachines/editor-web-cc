# Design

## Context

See `proposal.md`, section Why. ADR-0174 holds the decision and its rules. This design links to the items of ADR-0174 and does not repeat them. It gives the integration map, the facts behind the decision with a source for each fact, and the plan of the work.

The owner chose option D of ADR-0095 on 2026-11-15. On the same day the owner decided the threat model and the four open questions of the first plan. Section D0 lists these decisions.

## Goals / Non-Goals

**Goals:**

- No SQL of a set on the backup disk runs before a check that an offline writer of that disk cannot forge (ADR-0174 (a)).
- The key never lies on the backup disk, in an argument list, in an environment value, or in a shell variable.
- The image `recipe-pgtools` gets no new package.

**Non-Goals:**

- A defense against an online writer of the backup disk (ADR-0174 (b), (ag)).
- A defense against a holder of the key or of a shell as uid 999 in `backup` (ADR-0174 (ah)).
- A tag for the base backups and the WAL archive (ADR-0174 (d), (ak)).
- A change of the residual risk of ADR-0084 and ADR-0103 (o).

## Decisions

### D0. Owner decisions of 2026-11-15

| Topic | Decision | ADR-0174 |
|---|---|---|
| Threat model | The tag protects against an offline writer. An online writer is a known limit | (a), (b), (ag), (ah) |
| Write side | `backup-once` hashes each temp file before its publish and never reads a published file back | (t) |
| A set without a valid tag | Each restore refuses it. No flag skips the check | (x) |
| Old images | Only an image with the check restores a tagged set. An image without the check never gets the key | (z), (aa), (aj) |
| Keys and rotation | At most two key lines. A routine rotation and a compromise rotation | (n), (o), (q) |
| Offline copy of the key | The operator compares its key ids at each copy and at each restore drill | (r) |
| The window after the check | No extra measure. It belongs to the online writer | (ag) item 2 |
| Base backups and WAL | No tag in this change. `pitr-volume` stays a separate trust boundary | (d), (ak) |
| Replay of an older set | No promise through `backup_age`. The readers of the age hold no key | (ai) |
| Rollback across the first release with the tag | A bounded known limit of that transition only. A check of the `_premigrate` set with the new image comes first. The rolled-back `backup` runs no weekly `restore-test`. `COMPOSE_FILE` keeps the file of (aa) on | (w), (aa), (aj) |

### D1. Sets with a manifest, their readers, and the scope of the tag

| Set | Manifest today | Reader that runs code of the set | Tag |
|---|---|---|---|
| Logical set on `/backups`: dump, globals file, counts file | `recipe_<stamp>.dump.sha256`, SHA-256 of the three files (`backup-once` line 315) | `restore-test` and `restore-volume` through `lib/restore-steps.sh`: `psql --file` of the globals copy (line 181), `pg_restore --list` (line 153), `pg_restore --section` (lines 207 to 210) | Yes |
| Base backup `/wal/base/base_<YYYYMMDD_HHMMSS>` | `backup_manifest` of `pg_basebackup --manifest-checksums=SHA256` (`basebackup-once` line 265) | `pg_verifybackup` in `basebackup-once` (line 267) and in `lib/recovery-steps.sh` (line 86). `pitr-check` and `pitr-volume` start a server on a copy | No |
| WAL segments at the top of `/wal` | None. `pg_receivewal` writes them (`reliability.md` §7.7) | The `restore_command` of `lib/recovery-steps.sh` (lines 83 to 85) copies them for the recovery | No |

- The logical set is the one path of ADR-0095 (h). There, a reader runs the SQL text of a file as a superuser.
- The base backup and the WAL archive hold no SQL text that a script runs. The recovery replays physical WAL records, and `pg_verifybackup` checks the files of a base backup.
- The other readers of the backup disk run no code of a set. `backup-health`, the retention, and the adapter of `BackupDirectory` read only names and times (`reliability.md` §7.7).

### D2. Evidence for the tool

The command below ran on the pinned base image of `deploy/pgtools/Dockerfile` line 5. This block holds selected lines of its output:

```text
$ docker run --rm --network none --entrypoint sh postgres@sha256:d81f7b51697014367236d07ec58b54153fc28f828b72bf247fe5c7487520bcb0 -c '...'
openssl: /usr/bin/openssl
python3: missing
perl: /usr/bin/perl
perl Digest::SHA ok
libperl5.36:amd64: /usr/lib/x86_64-linux-gnu/perl/5.36.0/Digest/SHA.pm
perl-base: /usr/bin/perl
```

- The help of OpenSSL 3.0.22 shows that it takes the key only as an argument: `-hmac val`, and `-macopt val` "in n:v form or key". ADR-0174 option B therefore fails ADR-0066 (o).
- The guide of the skill `security-privacy`, paper 064 §8.1, asks for HMAC-SHA-256 and not a plain hash with a secret. It also names a digest without its algorithm and version as a failure mode. ADR-0174 (e) and (h) follow it.
- The test vector of ADR-0174 (j) came from host Python `hmac`. The command `openssl dgst -sha256 -mac HMAC -macopt hexkey:<test key>` printed the same tag.
- The Rust reader of a key file uses `str::lines` (`crates/recipe-domain/src/identity/key_ring.rs` line 84). `crates/recipe-server/src/config/key_file.rs` refuses a carriage return first. ADR-0174 (n) copies these line ends.

### D3. Integration map

| Place | Today | After the change | ADR-0174 |
|---|---|---|---|
| `backup-once` step 2 | `check_secret_file` (line 282) | Also the check of the key file | (s) |
| `backup-once` step 4 and 5 | `sha256sum` of the published files (line 315) | Hash of each temp file after its writer closes it, then the manifest, the tag, and the publish | (t), (u) |
| `backup-once` cleanup | `tmp_files` (line 278), cleanup (lines 98 to 119), `delete_set` (lines 210 to 217) | The temp tag name and the published tag file join each list | (u) |
| `backup-entry` | Marker check (lines 20 to 23) | Also the check of the key file | (v) |
| `lib/restore-steps.sh` | `check_manifest` reads the manifest on the disk at lines 100, 107, and 110 | The private copies, the tag check, and the reads of the copy | (w) |
| `restore-test` | Gates at lines 291 to 298 | The order of the check | (w) |
| `restore-volume` | `run_gates` (lines 182 to 202) | The order of the check | (w), (y) |
| `deploy/pgtools/Dockerfile` | `COPY` of five `lib/` files (line 25) | Also `lib/manifest-tag.pl` and `lib/manifest-tag.sh` | (k), (l) |
| `deploy/compose.yaml` | Secrets at lines 441 to 463 | The secret `backup_manifest_key` and three mounts | (m) |
| New `deploy/compose.previous.yaml` | None | The secrets of three services without the key, and `BACKUP_CRONTAB` of `backup` | (aa) |
| New `deploy/crontab.previous` | None | The line of `backup-once` from `deploy/pgtools/crontab.backup`, without the line of `restore-test` | (aa) |
| `restore-volume` arguments | One argument `<stamp>` (line 347) | Also `--check-only <stamp>` | (w) |
| `deploy/ci/upgrade.sh` | Line 274 runs the previous `restore-test`. Lines 311 to 318 copy four files to `upgrade-rollback` | The probe of the previous image, the file of (aa) only for a probe without the check, and the copy of the tag file | (aa), (aj) |
| `deploy/ci/upgrade-common.sh` | The trap `restore_tag_on_failure` (line 65) selects the previous tag | The command of case 3 of ADR-0174 (aj) only after a failure of command 1 or 2, and the function `leave_previous_overlay` | (aj) |
| `deploy/ci/upgrade_checks.py` | Subcommands of `upgrade_check` (`upgrade.sh` lines 9, 19, 76, 249, 275) | A subcommand that gives the Compose files for each probe result | (aa) |
| `crates/recipe-server/tests/deploy_values/overrides.rs` | `Allowed` rules for `compose.dev.yaml` and `compose.build.yaml` | A rule for `compose.previous.yaml` | (aa) |

Mounts of the secret:

| Service | Today | After the change | Source of the current state |
|---|---|---|---|
| `backup` | `backup_password` | `backup_password`, `backup_manifest_key` | `compose.yaml` lines 263 to 279 |
| `backup-premigrate` | `backup_password` | `backup_password`, `backup_manifest_key` | `compose.yaml` lines 70 to 84 |
| `restore-volume` | no secret | `backup_manifest_key` | `compose.yaml` lines 357 to 376 |

- `walarchive`, `pitr-check`, `pitr-volume`, `app`, `db`, `migrate`, and `proxy` do not mount the key.
- `RECIPE_SECRETS_DIR` is a host directory outside the repository, by default `/etc/recipe-editor/secrets` (`deploy/.env.example` line 20). The backup disk is a second disk (ADR-0066 (a)).
- `backup-health` uses `stat -c %d` for `pgdata` (lines 44 and 45). ADR-0174 (p) uses the same test. `docs/ops/runbook.md` line 177 already runs `df --output=source,target` for the disks.
- `compose.dev.yaml` line 15 uses `secrets: !reset []` for `proxy`. The file of ADR-0174 (aa) follows this precedent.

### D4. Faults, alerts, and the documents

- The scripts of `recipe-pgtools` write `ERROR` lines and no error code. The rows of `reliability.md` §1.2 name only Rust error types. The app treats `BackupFault` as a reason without a code (`crates/recipe-app/src/lib.rs` line 7).
- New `ERROR` lines. Each holds a fixed text and the set name only.
  - "the manifest key file is missing" and "the manifest key file has a bad line", with the field `line`.
  - "the manifest key file holds a carriage return" and "the manifest key file holds a NUL byte".
  - "the manifest key file has too many lines" and "the manifest key file lies on the backup disk".
  - "the newest set has no tag file" and "the tag file of the set has a bad line".
  - "the tag of the set comes from an unknown key" and "the tag of the set does not match the manifest".
  - "cannot read the manifest key file" and "the manifest key file exceeds the size limit".
  - "cannot check the filesystem of the manifest key file" and "the manifest tag program failed".
  - "cannot read the tag file of the set" and "the tag file of the set exceeds the size limit".
  - "cannot read the manifest" and "the manifest exceeds the size limit".
  - "cannot load the manifest tag library" and "cannot write the manifest tag file".
  - "cannot copy the manifest" and "cannot copy the manifest tag file".
  - "cannot set the manifest copy permissions" and "cannot set the manifest tag copy permissions".
  - "cannot read the manifest size limit" and "cannot record the manifest size fault".
  - "the checked manifest argument is missing" and "cannot read the manifest of the newest set".
  - "the counts copy does not match the manifest" and "the newest set lacks a regular file".
  - "cannot copy the counts file" and "cannot set the counts copy permissions".
  - "cannot hash the counts copy".
- Only "the manifest key file has a bad line" carries the field `line`. The mapper `manifest_tag_message` gives no other message a line number.
- `restore-volume` writes no result file. The operator reads its `ERROR` line in the console (ADR-0103 (i)).
- `reliability.md` changes:
  - §7.1 names the secret, its readers, its location rule, and its rotation apart from `security.key_rotation_overlap`.
  - §7.7 names the tag file, the write order, and the new step 1. Its line "`restore-volume` ... mounts no secret" (line 788) changes.
  - §7.10 step 3 gets the key file before the upgrade and the file of ADR-0174 (aa).
  - §7.11 gets the four rows of ADR-0174 (ab).
- `docs/ops/runbook.md` changes these sections:
  - "Create the secret files" (line 191): the ninth random value, the offline copy, and the `df` check.
  - "Copy the backups off site" (line 509): the tag file in the table.
  - "Key rotation" (line 1712): the routine rotation and the compromise rotation.
  - "Messages of backup-once" (line 2495): the new `ERROR` lines.
  - "Known gaps" (line 2550): ADR-0174 (ag) to (ak).
  - The upgrade steps, the failed-upgrade path, the rollback steps, and the disaster recovery steps.
  - These steps get the key file, the probe of the image, the check of the `_premigrate` set, and `COMPOSE_FILE` in `deploy/.env`.
- `reliability.md` §7.10 has two places that set the previous tag in `deploy/.env`: command 3 of step 3 and the bullets of step 6. Each place gets the step of `COMPOSE_FILE` when the probe finds no tag check. Three places set a tag with the check again. They are step 2, the bullet of step 6 near line 946, and step 3 of the runbook section "Switch back". Each of them gets the exit of ADR-0174 (aj). Only the bullet of step 6 then creates `backup` again. In step 2, command 1 of step 5 gives `backup` the new image and the key. In the switch back, its last step creates `backup`.
- Step 1 of the rollback deletes the container of `app` with `docker compose rm -s -f app` after a pass of the check, in both states of `deploy/.env` (ADR-0174 (aj)). A reboot then starts no `app` on the old volume.
- After a failed upgrade, a failed rollback restore starts only `backup` again. `app` stays stopped until a forward correction, a new restore, or `pitr-volume` (ADR-0174 (aj)). A failed check also starts only `backup` again, and `app` keeps running. The switch back in that state checks the WAL stream in `db` in place of the alert `replication`, because `app` runs the job `alerts`.

### D5. Tests

The suites of `deploy/pgtools/tests/` run inside the image through `deploy/ci/pgtools.sh` (line 9: `backup-test restore-test-test restore-volume-test walarchive-test pitr-volume-test`).

| Test | Path | Cases |
|---|---|---|
| Tag library | new `deploy/pgtools/tests/lib/manifest-tag-cases.sh`, sourced by `backup-test` | The vector of ADR-0174 (j). Sign with the first key line. Accept a set of the second key line. The stop cases and the line ends of (n). The size limits of (k). No key byte in stdout or stderr |
| Write path | `deploy/pgtools/tests/lib/once-cases.sh` | Tag file exists and passes the check. Tag before manifest. A change of a published file after the hash does not change the manifest. No `pg_dump` call without a key |
| Retention | `deploy/pgtools/tests/lib/delete-cases.sh` | The retention deletes the tag file after the manifest |
| Start check | `deploy/pgtools/tests/backup-test`, case `case_entry` (`once-cases.sh` line 313) | `backup-entry` stops without a valid key |
| Restore test | `deploy/pgtools/tests/lib/restore-set-cases.sh` | Missing, malformed, unknown key, different tag, forged manifest, link as tag file. No stub tool runs. No fallback to an older set |
| Restore volume | `deploy/pgtools/tests/lib/restore-volume-gate-cases.sh` | The refusal cases of the restore test: no `initdb` call, and no file of `/backups` changes. The swap of the manifest after the check: the restore reads the private copy and completes. The mode `--check-only` passes for a tagged set without a target volume and gives the same `ERROR` line for each refusal case. This includes signed sets with a bad manifest, counts file, or globals file. A mutation that skips one of these gates fails its case |
| Age readers | `crates/recipe-server/src/backups/tests.rs` (refused list, lines 288 to 293), `deploy/pgtools/tests/lib/health-cases.sh` | A tag file is no set. A new tag file of an old set does not change the health result. A mutation of the glob and the regex of `backup-health` makes the case fail |
| Compose values | `crates/recipe-server/tests/deploy_values/secrets.rs`, `topology.rs`, `defaults.rs`, `overrides.rs` | The three mounts, the declaration, the script defaults, and the file of ADR-0174 (aa) |
| Stack, job `compose` | `deploy/ci/recovery.sh`, `deploy/ci/prepare.sh` | A real `pg_restore` of a tagged set passes. A set with a changed manifest and no new tag fails before any PostgreSQL tool |
| Stack, job `restore-switch` (`.github/workflows/ci.yml` line 439) | `deploy/ci/restore-switch.sh` | The real `restore-volume` of a tagged set passes. A set without a tag stops before `initdb`. The mode `--check-only` passes and stops |
| Upgrade, job `upgrade` | `deploy/ci/upgrade.sh`, `deploy/ci/upgrade.compose.yaml` | The probe result decides the key and the file of (aa). The rollback set keeps its tag file. A bad set stops the rollback before the previous image runs. A forced failure of command 1 runs the failed-upgrade path. Its new `backup` reads `deploy/crontab.previous`, and the CI file does not mask the check. The exit runs from the rolled-back state and gives the key back. A mutation that skips one step of the exit fails a case |
| Probe decision | `deploy/ci/test_upgrade_checks.py`, run by `deploy/ci/check.sh` | Both probe results give their Compose files. The key state of a Compose configuration. The runbook block of the exit equals `leave_previous_overlay`. The marker of the trap stands just before `backup-premigrate` in the real `upgrade()`. The runbook block of a failed rollback restore after a failed upgrade starts no `app`. The glob of the dump check refuses a foreign name such as `recipe_<day>_abcd.dump.sha256`. Step 1 of the rollback deletes `app` with `rm` |
| Lint | `deploy/ci/check.sh` | `perl -c` of `lib/manifest-tag.pl` |

- The location check needs two filesystems in the suites. A suite puts the test key on a tmpfs such as `/dev/shm` and keeps `BACKUP_DIR` on the container disk. `health-cases.sh` line 16 already tests a second filesystem.
- `deploy/ci/prepare.sh` creates the CI key with `openssl rand -hex 32`, as for the other keys (lines 25 to 31).
- `deploy/ci/upgrade.sh` line 274 runs `restore-test` of the previous image with the new `compose.yaml`. When the probe finds no check, that image needs the file of ADR-0174 (aa), or it gets the key.
- Today each previous image has no check, so the job `upgrade` runs that result. The unit test covers the other result. The job runs that result for real when `upgrade_from` names a release with ADR-0174.
- The service `upgrade-rollback` of `deploy/ci/upgrade.compose.yaml` mounts no secret and joins no network. It gets the key only when the probe finds the check in the previous image.
- The commands of the exit of ADR-0174 (aj) live in one runbook block. That block is the function `leave_previous_overlay` of `deploy/ci/upgrade-common.sh`, and the job `upgrade` runs that function.
- A unit test compares the runbook block with the function. The segment check uses the same pattern (`deploy/ci/test_upgrade_checks.py` lines 73 to 82 and line 625).
- The reason: a runbook step without a command then fails the unit test. A function without that command fails a case of the job. A text check of the prose alone cannot run the commands.
- The trap of `deploy/ci/upgrade-common.sh` line 65 creates `backup` again only after a failure of command 1 or 2. A refused name check therefore replaces no container (`reliability.md` §7.10 step 2). A unit test reads the real `upgrade()` and fixes the place of the marker.
- After the case of the failed upgrade, the job creates `backup` again from the previous tree, with `upgrade.compose.yaml` last. Thus no scheduled job runs beside the jobs of CI.

### D6. Implementers

- Codex `gpt-6.1-sol` implements the tasks in `deploy/` and `deploy/ci/`.
- Sonnet implements the tasks in `crates/`.
- Haiku does the mechanical doc tasks. Opus does the doc tasks that change content.

## Risks / Trade-offs

- [The operator upgrades without the key file] → The upgrade steps of the runbook create the file first. `backup-entry` stops at its start without a valid key file, and `backup-premigrate` publishes no set.
- [The key is lost with the host] → The offline copy and its key-id comparison of ADR-0174 (r).
- [An online writer changes a file during a run] → ADR-0174 (ag). The runbook keeps uid 999 for the stack only.
- [The SQL of a set reads the key] → ADR-0174 (ah). The runbook asks for a compromise rotation after each incident of the class of ADR-0084.
- [The first release runs the previous image on a tagged set] → ADR-0174 (aj). The check with the new image comes first, and `COMPOSE_FILE` keeps the file of (aa) on.
- [Perl is a new language in `deploy/pgtools/`] → One program of about 100 lines, the vector of ADR-0174 (j), `perl -c` in `deploy/ci/check.sh`, and code review.
- [A stub suite cannot prove the real `Digest::SHA` and `pg_restore`] → The vector case runs in the real image. The stack test runs the real `backup-now`, `restore-test`, and `restore-volume`.

## Migration Plan

1. Before the upgrade, the operator creates the key file of ADR-0174 (m) with `openssl rand -hex 32`. The operator stores one offline copy (ADR-0174 (r)).
2. Step 3 of `reliability.md` §7.10 runs `restore-test` of the previous image. When the probe finds no check, the command adds the file of ADR-0174 (aa).
3. `backup-premigrate` of the new image writes the first signed set before `migrate` runs (ADR-0066 (i)).
4. `backup` of the new image signs each later set. The retention of `backup.retention` deletes the unsigned sets. The restores refuse an unsigned set (ADR-0174 (x)).
5. Rollback and failed upgrade: the operator follows ADR-0174 (aj). The extra secret file on the host does no harm.

## Open Questions

None. The owner decided each question of the first plan on 2026-11-15. Section D0 lists the decisions.
