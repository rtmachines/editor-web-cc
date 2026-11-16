# Tasks

Each task is for one agent. Codex `gpt-6.1-sol` implements each task in `deploy/` and `deploy/ci/`. Sonnet implements each task in `crates/`. Haiku does each `[mechanical]` task. Opus writes the content of each other doc task. A Sonnet agent runs the `ste-lint.py` step of each doc task and fixes only the form.

Each code task first writes a failing test and runs it (RED). Then it writes the change that makes the test pass (GREEN). A pgtools suite runs inside the image `recipe-pgtools`, as `deploy/ci/pgtools.sh` does. Build the image first with `docker compose -f deploy/compose.yaml -f deploy/compose.build.yaml build backup`.

The letters in parentheses after ADR-0174 name its items.

## Run order

1. Task 1.1 runs when the owner accepts ADR-0174 with this file. No code task starts before it.
2. Group 2 runs before groups 3 and 4. Tasks 2.4 and 2.5 land in one push, because the test of task 2.4 pins the `Dockerfile`.
3. Groups 3 and 4 can run in parallel. Inside each group, the tasks run in their order, because they edit the same files.
4. Group 5 lands in one push with task 6.1. The reason: the job `compose` needs the secret file and the mounts together. Task 5.5 runs after task 5.4.
5. Group 6 starts after groups 3, 4, and 5. Task 6.5 needs tasks 5.5 and 6.4. Tasks 6.5 to 6.8 edit `deploy/ci/upgrade.sh` and run in their order.
   - Tasks 6.7 and 6.8 also edit `deploy/ci/upgrade-common.sh`. Task 6.7 also edits `deploy/ci/test_upgrade_manifest.py`.
   - Task 6.9 runs after task 6.8, and task 6.10 runs after task 6.9. Both edit `deploy/ci/test_upgrade_runbook.py` and `docs/ops/runbook.md`.
   - Tasks 6.9 and 6.10 write each runbook text that their tests read, the links too. Group 7 keeps that text, so no test of group 6 waits for group 7.
6. Group 7 runs after group 6. The tasks that edit `docs/ops/runbook.md` or `reliability.md` run one after the other. Task 7.9 is the gate of the change.

## 1. Acceptance

- [x] 1.1 (docs/adr) [mechanical] When the owner accepts ADR-0174, make these edits:
  - Set the status of ADR-0174 to "Accepted".
  - In `docs/adr/README.md`, change "Proposed" to "Accepted" in the index row of ADR-0174. Delete "proposed: " from its decision map row.
  - Delete the mark "(proposed)" from the note "Amended by ADR-0174" of ADR-0066, ADR-0067, ADR-0074, ADR-0095, and ADR-0103.
  - Check: `grep -rn 'ADR-0174 (proposed)' docs/adr` prints nothing, and `ste-lint.py` on the seven files reports 0 hard violations.

## 2. Tag library

- [x] 2.1 (deploy/pgtools) First write `deploy/pgtools/tests/lib/manifest-tag-cases.sh` and source it from `deploy/pgtools/tests/backup-test`. Its cases fail first:
  - The vector of ADR-0174 (j): the program prints the tag `e047af28abfb9ccdd1867c6813ea85e66e50bd453b35726b79a1cccee8e6dd4d` and the key id `e60535bd89336ac1`.
  - A round trip of sign and check. A changed byte of the manifest, an unknown key id, and a malformed tag line each give their fault name.
  - With two key lines, the program signs with the key of the first line. The key id of the tag file shows it.
  - With two key lines, the check accepts a tag of the second line.
  - Then write `deploy/pgtools/lib/manifest-tag.pl` and `deploy/pgtools/lib/manifest-tag.sh` of ADR-0174 (e) to (l). Check: the suite `backup-test` of `deploy/ci/pgtools.sh` passes.
- [x] 2.2 (deploy/pgtools) First add failing cases to `manifest-tag-cases.sh` for the key file of ADR-0174 (n) and (p):
  - A missing file, an empty file, and a line of 63 or 65 characters.
  - An upper-case character, a carriage return, a NUL byte, and three lines.
  - The line ends: one line with a final line feed passes. One line without a final line feed passes. One line with two final line feeds stops.
  - The location check: a key on `/dev/shm` passes, and a key on the filesystem of `BACKUP_DIR` stops.
  - The size limits of ADR-0174 (k) at the values of `backup_manifest.max_tag_bytes` and `backup_manifest.max_file_bytes`. A file of the limit passes, and a file of one byte more stops.
  - No byte of the key in stdout or stderr. No `-hmac`, `-macopt`, or `hexkey` in a file of `deploy/pgtools/bin/` or `deploy/pgtools/lib/`.
  - Then write the checks in `lib/manifest-tag.pl` and `lib/manifest-tag.sh`. Check: the suite `backup-test` passes.
- [x] 2.3 (deploy/pgtools) First add failing cases to `manifest-tag-cases.sh` for the mode `key-ids` of ADR-0174 (r). The mode prints one key id per line of the key file, in the order of the lines. Its output holds no byte of the key. Then write the mode. Check: the suite `backup-test` passes.
- [x] 2.4 (recipe-server) First change the pinned `COPY` line in `crates/recipe-server/tests/deploy_values.rs` (line 97). Change the allow list of `tests/deploy_values/file_mutations/docker.rs` too. Both name `lib/manifest-tag.sh` and `lib/manifest-tag.pl`. Run `cargo test -p recipe-server --test deploy_values` and see it fail.
- [x] 2.5 (deploy/pgtools) Add the two files to line 25 of `deploy/pgtools/Dockerfile` and to `deploy/pgtools/.dockerignore`. Check: `cargo test -p recipe-server --test deploy_values` passes, and the image build passes.
- [x] 2.6 (deploy/ci) First add `perl -c deploy/pgtools/lib/manifest-tag.pl` to `deploy/ci/check.sh`. Add a line with a syntax fault to a scratch copy of the program and see the check fail. Check: `bash deploy/ci/check.sh` passes on the real program.

## 3. Write path

- [x] 3.1 (deploy/pgtools) First add failing cases to `deploy/pgtools/tests/lib/once-cases.sh`:
  - A run writes `recipe_<stamp>.dump.sha256.hmac`, and its tag passes the check of the library.
  - The run publishes the tag file before the manifest.
  - A missing key file and a key on the backup filesystem stop the run before the stub `pg_dump` runs.
  - Then change step 2 and step 5 of `deploy/pgtools/bin/backup-once` as ADR-0174 (s) and (u) give. Check: the suite `backup-test` passes.
- [x] 3.2 (deploy/pgtools) First add a failing case to `once-cases.sh` for ADR-0174 (t). A test hook changes one byte of the published dump before the run writes the manifest. The manifest must still hold the hash of the bytes of the stub `pg_dump`. Then hash the temp files in `backup-once`. Check: the suite `backup-test` passes.
- [x] 3.3 (deploy/pgtools) First add failing cases to `once-cases.sh` and `deploy/pgtools/tests/lib/delete-cases.sh`:
  - A failed publish of the manifest deletes the published tag file.
  - The retention deletes the tag file of an old set after its manifest.
  - An existing tag file or temp tag file of the new set stops the run before step 2 (ADR-0174 (u)). A link with one of these names also stops it.
  - Then change `tmp_files`, the cleanup, `delete_set`, and the name check of lines 266 to 272 of `backup-once`. Write the temp tag with `O_CREAT` and `O_EXCL`. Check: the suite `backup-test` passes.
- [x] 3.4 (deploy/pgtools) Extend `case_entry` of `once-cases.sh` (line 313) with two cases. The cases are a missing key file and a key line of 63 characters. Each case expects one `ERROR` line and no start of supercronic. Then change `deploy/pgtools/bin/backup-entry` (ADR-0174 (v)). Check: the suite `backup-test` passes.
- [x] 3.5 (recipe-server) Add the name `recipe_20261023_0200.dump.sha256.hmac` to the refused list of `crates/recipe-server/src/backups/tests.rs` (lines 288 to 293). Then prove that the case can fail:
  - Change the suffix match of `MANIFEST_SUFFIX` (`backups.rs` line 27) in a scratch edit. Make a name that ends in `.dump.sha256.hmac` count as a set.
  - Run `cargo test -p recipe-server backups` and see the new case fail. Then restore the code.
  - Check: `git diff crates/recipe-server/src/backups.rs` prints nothing, and `cargo test -p recipe-server backups` passes.
- [x] 3.6 (deploy/pgtools) Add a case to `deploy/pgtools/tests/lib/health-cases.sh`:
  - The fixture is one set with all four files. Its manifest is older than the age limit, and its tag file is new.
  - `backup-health` must read the age of the manifest and stop with "no complete set is younger than the maximum age" (line 70).
  - Then prove that the case can fail. A scratch edit of the glob alone is not enough, because the regex of line 56 still refuses `.hmac`.
  - In a scratch edit, widen the glob of line 54 to match `*.dump.sha256.hmac`. Also widen the regex of line 56 to accept the suffix `.hmac`.
  - Run the suite `backup-test` and see the new case fail. The changed script reads the age of the new tag file and writes "backup is healthy". Then restore both lines.
  - Check: `git diff deploy/pgtools/bin/backup-health` prints nothing, and the suite `backup-test` passes.

## 4. Check path

- [x] 4.1 (deploy/pgtools) First add failing cases to `deploy/pgtools/tests/lib/restore-set-cases.sh`:
  - A missing tag file, a link as the tag file, a malformed tag line, an unknown key id, and a different tag.
  - A manifest computed again after a changed dump, with the old tag.
  - Each case stops with one `ERROR` line, calls no stub PostgreSQL tool, and writes the result `failed`. It selects no older set.
  - A set with a bad tag whose dump, globals file, and counts file are directories. The run must stop with the tag `ERROR` line, not with a read fault.
  - A swap of the manifest after the tag check. A stub on `PATH` wraps the tag program. The stub runs the real program and then writes a changed manifest over the file on the disk.
  - The swap case must pass the manifest check, because the run reads the private copy. A run that reads the disk stops with the manifest fault.
  - Then add the tag check to `deploy/pgtools/lib/restore-steps.sh`. Add the private copies of the manifest, the tag file, and the counts file there.
  - Then add the order of ADR-0174 (w) to `deploy/pgtools/bin/restore-test`. Check: the suite `restore-test-test` passes.
- [x] 4.2 (deploy/pgtools) First add the refusal cases of task 4.1 to `deploy/pgtools/tests/lib/restore-volume-gate-cases.sh`. These are all cases of 4.1 except the swap case:
  - Each case expects the `ERROR` line of task 4.1 and an exit code other than 0.
  - Each case expects no `initdb` call and no changed file of `/backups`.
  - Then change `run_gates` of `deploy/pgtools/bin/restore-volume` and its header comment. Check: the suite `restore-volume-test` passes.
- [x] 4.3 (deploy/pgtools) First add the swap case of task 4.1 to `restore-volume-gate-cases.sh`. The stub on `PATH` writes a changed manifest over the file on the disk after the tag check. The expected end:
  - The run passes the manifest check, because it reads the private copy.
  - The run calls the stub `initdb`, and the target holds `PG_VERSION`.
  - The run exits with the code 0 and writes no `ERROR` line. It ends with the line "restore volume completed", as `case_volume_completed_line` of `restore-volume-stop-cases.sh` expects (lines 81 and 82).
  - Then prove that the case can fail. In a scratch edit, make the manifest check read the file on the disk. Run the suite and see the case stop with the manifest fault. Then restore the code.
  - Check: the code holds no scratch edit, and the suite `restore-volume-test` passes.
- [x] 4.4 (deploy/pgtools) First add failing cases to `restore-volume-gate-cases.sh` for the mode `--check-only <stamp>` of ADR-0174 (w):
  - A tagged set without a target directory passes. The mode writes one `INFO` line "the set passed the check" and no line "restore volume started". It exits with the code 0.
  - A target directory that is not empty does not stop the mode, because the mode skips the target checks (`restore-volume` lines 162 to 165).
  - Run each refusal case of task 4.2 under `--check-only`. Each case expects the same `ERROR` line as the full restore and an exit code other than 0.
  - Also run `case_volume_gate_manifest`, `case_volume_gate_counts`, and `case_volume_gate_globals` under `--check-only`. Each of their sets has a valid tag of the test key.
  - Only `check_manifest`, `check_counts`, or `check_globals` can then stop the case. Each case expects the same `ERROR` line as the full restore and an exit code other than 0.
  - Each case calls no stub PostgreSQL tool and no `pg_restore --list`, and changes no file outside the work directory.
  - Then add the mode to `deploy/pgtools/bin/restore-volume` (line 347) and its header comment. The mode calls `check_inputs` without the target checks, `take_lock`, and `run_gates` up to `check_globals`. It does not call `check_names`.
  - Then prove that each gate case of the mode can fail. In three scratch edits, one at a time, make the mode skip `check_manifest`, `check_counts`, or `check_globals`.
  - Each edit must make its case fail. Then restore the code.
  - Check: the code holds no scratch edit, and the suite `restore-volume-test` passes.

## 5. Compose and the values of §7.11

- [x] 5.1 (docs/architecture) [mechanical] Add four rows to `reliability.md` §7.11:
  - `secrets.backup_manifest_key_file` (`/run/secrets/backup_manifest_key`) and `backup_manifest.max_keys` (2).
  - `backup_manifest.max_tag_bytes` (4096) and `backup_manifest.max_file_bytes` (65536).
  - Check: `ste-lint.py docs/architecture/reliability.md` reports 0 hard violations.
- [x] 5.2 (recipe-server) First change the checks of `crates/recipe-server/tests/deploy_values/`:
  - `secrets.rs`: `SECRET_LISTS` gets `backup`, `backup-premigrate`, and `restore-volume` with `secrets.backup_manifest_key_file`.
  - `topology.rs`: `SECRETS` gets `backup_manifest_key`. The rule of line 208 permits only this secret on `restore-volume`.
  - `isolation_mutations.rs`: the test of line 61 keeps a fault for every other secret on `restore-volume`.
  - `defaults.rs`: `SCRIPT_DEFAULTS` gets `BACKUP_MANIFEST_KEY_FILE` of `backup-once`, `backup-entry`, `restore-test`, and `restore-volume`.
  - `defaults.rs`: the two size limits of `lib/manifest-tag.pl` must equal the rows of task 5.1.
  - Run `cargo test -p recipe-server --test deploy_values` and see it fail.
- [x] 5.3 (deploy) Declare the secret `backup_manifest_key` in `deploy/compose.yaml` and mount it in `backup`, `backup-premigrate`, and `restore-volume`. Change the comments of `restore-volume` (lines 350 to 356). Check: `cargo test -p recipe-server --test deploy_values` passes, and `docker compose -f deploy/compose.yaml config --quiet` passes with the CI values.
- [x] 5.4 (recipe-server) First add an `Allowed` rule for `deploy/compose.previous.yaml` to `crates/recipe-server/tests/deploy_values/overrides.rs` and its path to `inputs.rs`:
  - The rule permits the key `secrets` of `backup`, `backup-premigrate`, and `restore-volume`.
  - It also permits `environment` with only `BACKUP_CRONTAB`, and one read-only mount of `deploy/crontab.previous`, on `backup`.
  - A test fails when the file names `backup_manifest_key`.
  - A test fails when `deploy/crontab.previous` holds a line of `restore-test`, or a line of `backup-once` other than the line of `deploy/pgtools/crontab.backup`.
  - Run `cargo test -p recipe-server --test deploy_values` and see it fail.
- [x] 5.5 (deploy) Write `deploy/compose.previous.yaml` and `deploy/crontab.previous` of ADR-0174 (aa), each with a header comment that names the rule. Check: `cargo test -p recipe-server --test deploy_values` passes. `docker compose -f deploy/compose.yaml -f deploy/compose.previous.yaml config` shows no `backup_manifest_key` on the three services.

## 6. Stack tests

- [x] 6.1 (deploy/ci) First add a check to `deploy/ci/prepare.sh`: `stat -c '%a %u'` of `backup_manifest_key` must print `400 999`. Run it and see it fail. Then create the file with `openssl rand -hex 32`, as lines 25 to 31 do. Check: the job `compose` starts `backup` with a healthy state.
- [x] 6.2 (deploy/ci) First add two steps to `deploy/ci/recovery.sh` of the job `compose`:
  - After the passed `restore-test`, expect the tag file of the `_now` set and the passed real `pg_restore` of that set in the log.
  - Then copy the `_now` set to a new stamp with `sudo install -o 999 -g 999 -m 0600`, as `deploy/ci/upgrade.sh` line 313 does. The check then stops at the tag `ERROR` line and not at a read fault.
  - Change one byte of the dump of the copy, and compute its manifest again.
  - Expect the `ERROR` line "the tag of the set does not match the manifest" and the result `failed`. Then delete the copy.
  - Check: the job `compose` passes, and the log shows the pass and the refusal.
- [x] 6.3 (deploy/ci) First add four cases to `deploy/ci/restore-switch.sh` of the job `restore-switch` (`.github/workflows/ci.yml` line 439):
  - The real `restore-volume` of a tagged set passes.
  - `restore-volume` for a set without its tag file stops before `initdb`, with one `ERROR` line.
  - `restore-volume --check-only` passes for the tagged set and stops for a set with a changed dump.
  - Check: the job `restore-switch` passes, and its log shows each case.
- [x] 6.4 (deploy/ci) First add failing cases to `deploy/ci/test_upgrade_checks.py` and `deploy/ci/test_upgrade_manifest.py` for a new subcommand of `deploy/ci/upgrade_checks.py`. The subcommand takes the probe result and prints the Compose files and the expected key state:
  - The result "no check" gives `compose.yaml` and `compose.previous.yaml`, and "no key".
  - The result "check" gives only `compose.yaml`, and "key".
  - A second subcommand reads the output of `docker compose config --format json`. It prints "key" when `backup`, `backup-premigrate`, and `restore-volume` each have `backup_manifest_key`, and "no key" when none has it.
  - A configuration where only some of the three services have the key is a fault.
  - Then write the two subcommands. Check: `bash deploy/ci/check.sh` passes.
- [x] 6.5 (deploy/ci) First add a failing check to `deploy/ci/upgrade.sh` after line 274. A container of the previous image must have the key state that the subcommand of task 6.4 gives. Then make these changes:
  - Probe the previous image for `/etc/pgtools/manifest-tag.pl`. Use the Compose files of the subcommand for each run of the previous image with the new `compose.yaml`. `UPGRADE_COMPOSE_DIR` is `$GITHUB_WORKSPACE`.
  - `start_previous` (lines 46 to 66) uses the previous tree. Its `compose.yaml` declares no `backup_manifest_key`, so its runs need no overlay. That tree also holds no `deploy/crontab.previous`.
  - Do not use an absolute path to the new overlay there. That path merges a file of the new release into the old `compose.yaml`, and no operator command does this.
  - Copy the tag file with the four files of lines 311 to 318.
  - Mount the key in `upgrade-rollback` of `deploy/ci/upgrade.compose.yaml` only for the result "check". Then expect the tag check in the rollback log.
  - Check: the job `upgrade` passes, and its log shows the probe result and the key state.
- [x] 6.6 (deploy/ci) The checked rollback for the result "no check" (ADR-0174 (aj), case 2). First add a failing case with a bad set to `deploy/ci/upgrade.sh`:
  - Copy the `_premigrate` set to a new `_premigrate` stamp with `sudo install -o 999 -g 999 -m 0600`, as line 313 does. The check then stops at the tag `ERROR` line and not at a read fault.
  - Change one byte of the dump of the copy and compute its manifest again. Keep the old tag file.
  - Run `restore_previous` with that stamp as its argument, before the good set. Expect that `restore-volume --check-only` writes the tag `ERROR` line and exits with a code other than 0.
  - Expect also that `$RECIPE_BACKUP_DIR/upgrade-rollback` does not exist, so the restore of the previous image did not run. Then delete the bad copy.
  - Then change `restore_previous`. It first runs `compose run --rm restore-volume --check-only <stamp>` with the new image on the original set. It copies the set and runs `upgrade-rollback` only on the exit code 0.
  - The bad case and the good set use this one function. A function that ignores the exit code of the check therefore fails the bad case.
  - The check reads the original set in `/backups`, not the copy. It therefore runs the command of the runbook unchanged: the service `restore-volume` of `compose.yaml`, the default `BACKUP_DIR`, and the lock that `restore-test` created.
  - The copy has no `.scratch/.lock` (`restore-volume` lines 170 to 176), and the service sets no `BACKUP_DIR` (`compose.yaml` lines 364 to 366). A check of the copy needs both only in CI.
  - After the copy, the function compares each copied file with its original with `cmp`.
  - Check: the job `upgrade` passes, and its log shows the refusal of the bad set and the pass of the good set.
- [x] 6.7 (deploy/ci) The failed-upgrade path for the result "no check" (ADR-0174 (aj), case 3). First add a failing check to `deploy/ci/upgrade.sh` after line 275:
  - Record the ID of the container of `backup`. Then force a failure of command 1 in a subshell that sets the trap `restore_tag_on_failure` (`upgrade-common.sh` line 65).
  - The forced command is `compose -f "$GITHUB_WORKSPACE/deploy/compose.previous.yaml" up -d backup-premigrate`, then `wait_job backup-premigrate`.
  - The new image then has no key, so `backup-once` stops before the snapshot session (ADR-0174 (s)). Expect an exit code other than 0 of the subshell and no new set.
  - Expect a new container of `backup` with the image `recipe-pgtools:ci-previous`. In `docker inspect`, `BACKUP_CRONTAB` names the read-only mount of `deploy/crontab.previous`.
  - Expect that this file in the container holds the line `0 2 * * * backup-once` and no line of `restore-test`. Expect no file `/run/secrets/backup_manifest_key`.
  - For the result "check", expect the recorded ID of the container of `backup`.
  - Then change `restore_tag_on_failure`. On the result "no check", it runs `RECIPE_VERSION=ci-previous compose -f "$GITHUB_WORKSPACE/deploy/compose.previous.yaml" up -d --no-deps --force-recreate backup`.
  - It runs this command with `UPGRADE_COMPOSE_DIR=$GITHUB_WORKSPACE`, before it selects the previous tree. The operator also stays in the new `deploy/` (`reliability.md` §7.10 step 2).
  - The wrapper puts `compose.previous.yaml` after `upgrade.compose.yaml`, so its `BACKUP_CRONTAB` replaces `/dev/null` (`upgrade.compose.yaml` line 10).
  - The forced failure and a real failure of command 1 or 2 use this one trap. A trap without the command `--force-recreate` keeps the old container with `BACKUP_CRONTAB=/dev/null`.
  - The trap runs the command `--force-recreate` only after a failure of command 1 or 2. `upgrade.sh` sets the marker variable with the line `UPGRADE_STEP3_STARTED=true` just before `compose up -d backup-premigrate` (line 277).
  - The subshell of the forced failure sets the same variable. Without the variable, the trap replaces no container.
  - Thus a refused name check (line 269) replaces no container, as `reliability.md` §7.10 step 2 (line 921) requires. The trap is active from line 262.
  - Add a second subshell that sets the trap without the variable and forces a failure with `false`. Expect an exit code other than 0 and the recorded ID of the container of `backup`.
  - The two subshells do not prove the place of the marker in the real `upgrade()`. Add a unit test to `deploy/ci/test_upgrade_manifest.py` for that place.
  - The test reads the real `upgrade()` with `shell_function("upgrade")` (line 12 of `test_upgrade_manifest.py`). That reader reads `deploy/ci/upgrade.sh` only.
  - The test expects exactly one line `UPGRADE_STEP3_STARTED=true` in `upgrade()`. That line comes after the line that holds only `check_volume_name`.
  - The test also expects that the next line after the assignment is `compose up -d backup-premigrate`.
  - After the case, on the result "no check", create `backup` again in the state of `start_previous`. Run `compose up -d --no-deps --force-recreate backup` in a subshell.
  - That subshell sets `RECIPE_VERSION=ci-previous` and `UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR`. The previous tree declares no key. The wrapper puts `upgrade.compose.yaml` last.
  - Thus `BACKUP_CRONTAB` is `/dev/null` again, and no scheduled job runs beside the jobs of CI (`upgrade.compose.yaml` line 7).
  - Expect `BACKUP_CRONTAB=/dev/null` in `docker inspect`, the image `recipe-pgtools:ci-previous`, and no file `/run/secrets/backup_manifest_key`. Task 6.8 puts its case 1 before this command.
  - Prove that the check can fail. In a scratch edit, delete the command `--force-recreate` from the trap and see the check fail.
  - In a second scratch run, put `upgrade.compose.yaml` after `compose.previous.yaml`. See the check fail, because `BACKUP_CRONTAB` is then `/dev/null`.
  - In a third scratch run, delete the test of the marker variable from the trap. See the second subshell fail.
  - In a fourth scratch run, delete the command that creates `backup` again after the case. See the expectation of `BACKUP_CRONTAB=/dev/null` fail.
  - In a fifth scratch edit, delete only the assignment of the marker from `upgrade()`. See the unit test fail.
  - In a sixth scratch edit, move only that assignment before the line `check_volume_name` (line 269). See the unit test fail. Then restore the code.
  - Check: the code holds no scratch edit. `bash deploy/ci/check.sh` passes. The job `upgrade` passes. Its log shows the forced failure, the crontab lines of the container, and the state after the case.
- [x] 6.8 (deploy/ci) The exit from `compose.previous.yaml` (ADR-0174 (aj)). First add failing cases to `deploy/ci/upgrade.sh` that call the function `leave_previous_overlay <directory> <write tag>` before it exists:
  - Codex `gpt-6.1-sol` writes the function in `deploy/ci/upgrade-common.sh`, next to `restore_tag_on_failure`. `upgrade.sh` gets it through `source`, as it gets the trap.
  - `<write tag>` is the tag that the calling step writes into `.env`. Task 6.9 gives its value at each call place.
  - The function changes `<directory>/.env` with one atomic replace. That replace deletes the line `COMPOSE_FILE` and sets `RECIPE_VERSION` to `<write tag>`.
  - The replace has three steps, in this order:
    1. Write the new text into a temp file from `mktemp "<directory>/.env.XXXXXX"`. The temp file is in the same directory, so the rename stays on one file system.
    2. Copy the owner and the mode of `.env` to the temp file with `chown --reference` and `chmod --reference`.
    3. Run `mv -f` to rename the temp file over `.env`. A rename on one file system replaces `.env` in one step.
  - Each line of `.env` other than `COMPOSE_FILE` and `RECIPE_VERSION` stays byte for byte and in its order.
  - An `.env` without a `COMPOSE_FILE` line is the normal state at runbook line 1212. The filter must then give the exit code 0.
  - On a fault of a step, the function deletes the temp file and returns a code other than 0. `.env` then stays unchanged.
  - `.env` holds no secret (`.env.example` line 2). The temp file therefore needs no extra measure, such as a `umask` or a secure delete.
  - Then the function runs `unset COMPOSE_FILE` in the shell of the job.
  - The reason for one replace: with two edits, a failed second edit can leave the previous tag without `COMPOSE_FILE`. Then the previous image gets the key.
  - A direct write such as `cat <temp> > .env` can leave a truncated `.env` on `ENOSPC` or on another I/O fault. The rename never leaves a partial `.env`.
  - One replace also makes the fourth scratch edit below fail case 4. Two edits fail together on the read-only directory, so the check of the second edit hides that edit.
  - The function is also the runbook block of task 6.9, which an operator pastes into a Bash shell. It therefore uses no CI variable and no function of `upgrade-common.sh`.
  - On a fault, it uses `return` with a code other than 0. It does not use `exit`, because `exit` ends the shell of the operator.
  - A pasted copy runs without the `set -euo pipefail` of `upgrade-common.sh` line 3. The function therefore checks the result of each step itself.
  - On the first failed step, it returns a code other than 0 and runs no later step.
  - The function sets no `trap` (ADR-0174 (aj)). A trap in a pasted function replaces the traps of the operator. It also replaces the `EXIT` trap of CI (`deploy/ci/upgrade.sh` line 262).
  - Each case keeps its reference files and its `stat` record in a scratch directory outside the copy.
  - Cases 5, 7, and 8 record the sorted list of the names `.env.*` in the scratch copy before the call. After the call, they compare the list with `cmp`.
  - The reason for the list: the copy of `deploy/` holds `.env.example`. The comparison also does not depend on the length of the random part of the temp name.
  - Read each key state with `docker compose --profile restore config --format json` in a scratch copy of `deploy/`. The second subcommand of task 6.4 gives the state.
  - The scratch copy stands for the `deploy/` of the operator. The cases run the function on it, so they run the commands of the runbook block.
  - Run each read with `env -u RECIPE_VERSION` and with the `COMPOSE_FILE` of the job shell. The wrapper `compose` does not pass `COMPOSE_FILE` (`upgrade-common.sh` line 13), so the real runs do not change.
  - Case 1 is the exit of step 2. It comes after the check of task 6.7 and before the command of task 6.7 that creates `backup` again. The `.env` of the scratch copy holds `ci-previous` and `COMPOSE_FILE` with both files. The job shell exports the same `COMPOSE_FILE`.
  - Read the start state and expect "no key". Run the function with `ci-upgrade`. Expect "key", and expect the same ID of the container of `backup`.
  - Case 2, command 1 of step 5. After line 298, expect the file `/run/secrets/backup_manifest_key` in the container of `backup` and the state `healthy`.
  - Case 3, the exit of step 6, after `restore_previous`. Create the rolled-back `backup` with the command of the trap of task 6.7. Prepare the scratch copy and the shell as in case 1.
  - Run the function with `ci-upgrade` and expect "key". Then run `compose up -d --no-deps --force-recreate backup` with the new tag and without `compose.previous.yaml`.
  - Expect the file `/run/secrets/backup_manifest_key` in the container and the state `healthy`.
  - Case 4, a fault of the function. Prepare a second scratch copy as in case 1. Copy its `.env` to a reference file.
  - Make `.env` and the scratch directory read-only with `chmod a-w`. Then `mktemp` cannot create the temp file, and no rename can change `.env`.
  - Run the function in `bash -c` without errexit. Pass its text with `declare -f leave_previous_overlay`, so the new shell does not run `set -e`. Run it as the user of the job and not with `sudo`, because root ignores the mode.
  - That shell exports `COMPOSE_FILE` before the call and prints the return code and `COMPOSE_FILE` after it.
  - Expect a code other than 0 and an unchanged `COMPOSE_FILE` in that shell. Expect a `.env` equal to the reference file with `cmp`. Then make the scratch copy writable again.
  - Case 5, a successful replace keeps the metadata and the other lines. Prepare a third scratch copy as in case 1. Set the mode of its `.env` to `0644`.
  - The mode `0644` differs from the mode `0600` of a new `mktemp` file, so a lost mode is visible.
  - Then run `chgrp` on `.env` with a group of `id -G` other than the group of `id -g`. `id -G` prints the primary group first. This step needs no root.
  - A new `mktemp` file gets the primary group, so the field `%g` shows a lost `chown --reference`.
  - If the user of the job has no supplementary group in CI, the case logs that fact. The seventh edit below then stays undetected.
  - The owner accepts that limit, because `.env` holds no secret. CI runs no root step to change the owner.
  - After the `chgrp`, record `stat -c '%a %u %g'` of `.env`. Copy each line other than `COMPOSE_FILE` and `RECIPE_VERSION` into a reference file.
  - Then run the function with `ci-upgrade`. Expect the code 0 and the same `stat` output.
  - Expect the same other lines with `cmp`. Expect one line `RECIPE_VERSION=ci-upgrade`, no line `COMPOSE_FILE`, and the same list of names `.env.*`.
  - Case 6, an `.env` without a `COMPOSE_FILE` line. Prepare a fourth scratch copy with the tag `ci-upgrade` and no `COMPOSE_FILE` line. Copy its other lines into a reference file.
  - Run the function with `ci-upgrade`. Expect the code 0, one line `RECIPE_VERSION=ci-upgrade`, and the same other lines with `cmp`.
  - Case 7, a fault after the write of the temp file starts. Prepare a fifth scratch copy as in case 1, and copy its `.env` to a reference file.
  - CI cannot mount a full file system without root. The case therefore puts a stub directory first in the `PATH` of the `bash -c` shell of case 4.
  - The function writes the temp file with one external tool, such as `awk`, so a stub can replace it. The stub has the name of that tool. It writes the first half of its input and returns 1, as on `ENOSPC`.
  - Expect a code other than 0 and an unchanged `COMPOSE_FILE` in that shell. Expect a `.env` equal to the reference file with `cmp`. Expect the same list of names `.env.*`.
  - Case 8, a fault of the rename. Repeat case 7 with a stub `mv` that changes nothing and returns 1. Expect the same results as in case 7.
  - Then write the function in `deploy/ci/upgrade-common.sh` and the command of case 3 in `deploy/ci/upgrade.sh`.
  - Prove that each step of the exit is necessary. Make eight scratch edits, one at a time.
  - The first three edits each skip a different step. The steps are the delete of `COMPOSE_FILE` in the replace, the `unset COMPOSE_FILE`, and the command `--force-recreate` of case 3.
  - The fourth edit deletes the check of the result of the replace. Then the function runs `unset COMPOSE_FILE` after the failed replace and returns 0.
  - Case 4 then sees the code 0 and a changed `COMPOSE_FILE`, so it fails.
  - The fifth edit makes the stubbed tool write straight into `.env`, for example `awk ... > .env`. Case 7 then finds a truncated `.env`, so it fails.
  - Do not use a temp file followed by `cat <temp> > .env` for this edit. That variant stops at the stub before the write, so case 7 passes.
  - The sixth edit deletes `chmod --reference`. Case 5 then finds the mode `600`, so it fails.
  - The seventh edit deletes `chown --reference`. Case 5 then finds the primary group in the field `%g`, so it fails.
  - The eighth edit skips the delete of the temp file on a fault. Case 7 then finds a left file `.env.*` in the list of names, so it fails.
  - Each edit must make a case fail. Then restore the code.
  - Check: the code holds no scratch edit. The job `upgrade` passes, and its log shows the result of each case.
- [x] 6.9 (deploy/ci) The runbook block of the exit (ADR-0174 (aj)). First add a failing test to `deploy/ci/test_upgrade_runbook.py`:
  - The test reads the level-4 section "Leave the file compose.previous.yaml" of `docs/ops/runbook.md`. That section must hold exactly one `bash` block.
  - Add the finder `runbook_heading_section(title, level)` to `deploy/ci/test_upgrade_runbook.py`. Tasks 6.9 and 6.10 use it for each section and each level:
    1. It reads the lines of the runbook with `str.splitlines()`. Thus a CRLF line end changes no result.
    2. A heading line of level n starts with n characters `#` and one space. The finder skips a line inside a fenced code block.
    3. The heading line `<level × #> <title>` must occur exactly once, after the strip of white space at the end. Else the test fails.
    4. The scope starts at that heading line. It ends before the next heading line of the same or a higher level. Without such a line, it ends at the end of the file.
    5. A sub-heading of a lower level inside the scope stays in the scope. Its lines are lines of the section too.
  - The finder skips headings only inside a fence. Thus a fence can make a scope longer but never shorter.
  - Note for the implementer: a fence is a stripped line that starts with three backticks or three tildes. Add a positive case: a `text` fence that holds `# note` before the first block.
  - `runbook_section` of `deploy/ci/test_restore_switch.py` (lines 2530 to 2537) matches only `### ` and stays unchanged.
  - The test compares that block with the text of `leave_previous_overlay` in `deploy/ci/upgrade-common.sh`. It uses the pattern of `runbook_identity_step` (lines 73 to 82) and of the test at line 625.
  - The reader `shell_function` (lines 67 to 70) of `deploy/ci/test_upgrade_checks.py` reads `deploy/ci/upgrade.sh` only and stays unchanged. The copy of `shell_function` in `deploy/ci/test_upgrade_runbook.py` takes a second parameter, the file name, with the default `upgrade.sh`.
  - The test reads the function with `shell_function("leave_previous_overlay", "upgrade-common.sh")`. The other callers stay unchanged.
  - Then add that section to `docs/ops/runbook.md` with the heading `#### Leave the file compose.previous.yaml`. Its one `bash` block is the function, unchanged.
  - The section goes inside `### Rollback` (lines 1471 to 1539), directly after the section `#### Start app after a rollback` (line 1533).
  - After task 6.10, its scope ends before the heading of task 6.10, which has the same level.
  - Add one sentence: in the directory `deploy/`, paste the block, then run `leave_previous_overlay . <write tag>`. Task 7.6 writes the other text of the section.
  - `<write tag>` is the tag that the calling step writes into `.env`. Name its value at each call place:
    1. The upgrade step that sets the new tag (line 1212) and step 7 of "Major upgrade" (line 1679). Both write the new release tag.
    2. The plain rollback at lines 672 and 1511: the new release tag.
    3. Step 3 of "Switch back" (line 1567): the `<tag>` of that section, which is the tag of the old volume.
  - Place 3 always writes the tag of the old volume. After the plain rollback, the old volume runs the new release tag, so place 3 writes that tag.
  - Only after a failed upgrade is the tag of the old volume the previous tag. Give the reason of place 3 for that state.
  - In a later release, each previous image has the tag check, so the exit runs at place 3.
  - After a failed upgrade, the new release tag there makes step 8 start the new `backup` on the schema of the failed `migrate`. ADR-0174 (aj) forbids that start.
  - Prove that the test can fail. Make each of the two scratch edits below alone, and see the test fail each time. Then restore the runbook:
    1. Delete the line `unset COMPOSE_FILE` from the runbook block. The compare with the function catches it.
    2. At the end of the section, add the sub-heading `##### Note` and under it a `bash` block with the line `docker compose up -d`. The scope catches it.
  - Prove that each edit proves one rule. Disable only the rule that the edit names, and see the edit pass:
    1. For edit 1, skip the compare with the function.
    2. For edit 2, make the finder stop at the next heading of any level. The scope then holds one block.
  - The reason for this way: the cases of task 6.8 run the function, and this test makes the runbook block equal to the function.
  - A runbook block that omits a command fails this test. A function that omits it fails a case of task 6.8. The segment check of the upgrade uses the same pattern.
  - Step for a Sonnet agent: run `ste-lint.py docs/ops/runbook.md`. Check: it reports 0 hard violations.
  - Check: the block holds no scratch edit, and `bash deploy/ci/check.sh` passes.
- [x] 6.10 (deploy/ci) The runbook blocks of a failed rollback restore after a failed upgrade (ADR-0174 (aj)). First add a failing test to `deploy/ci/test_upgrade_runbook.py`:
  - The test reads the level-4 section "Failed rollback restore after a failed upgrade" of `docs/ops/runbook.md` with the finder of task 6.9. That section must hold exactly three `bash` blocks.
  - The first block is for a failed check and for step 8 of "Switch back". The test expects it to equal the one line `docker compose up -d --no-deps backup`.
  - The second block is for a failed `restore-volume <stamp>`. The test expects it to equal two lines in this order:
    1. `docker compose rm -s -f app`.
    2. `docker compose up -d --no-deps backup`.
  - The third block is the dump check of task 7.3. The test expects it to equal the one line `docker compose exec -T -e LC_ALL=C backup sh -c 'ls /backups/recipe_$(date -u +%Y%m%d)_[0-9][0-9][0-9][0-9].dump.sha256'`.
  - The option `-e LC_ALL=C` sets the locale of the match in `backup`, as ADR-0074 (c) requires. `docker compose exec` accepts `-e` (`compose_exec.md` of Docker Compose).
  - The text compare cannot show what the glob matches. The test therefore also runs the glob of that expected line. This run holds the negative case of the four digits of ADR-0074 (c):
    1. The test takes the argument of `sh -c` and replaces `/backups` with `.`. It runs the result with `sh -c`, `LC_ALL=C`, and the `cwd` set to the temp directory.
    2. The text of `sh -c` thus holds no host path. On Windows, a temp path in that text loses its backslashes.
    3. The test finds `sh` with `shutil.which("sh")`, as `deploy/ci/test_upgrade_checks.py` finds `bash` (line 41). The glob run is a separate test method.
    4. When no `sh` exists, that method calls `self.skipTest("sh is not available")`, as `deploy/ci/test_upgrade_checks.py` does for `bash` (line 43). The other checks of this task still run.
    5. A stub directory outside the temp directory comes first on the `PATH` of `sh`. This is like the stubbed tools of task 6.8, cases 7 and 8. Its stub `date` prints the fixed day `20260102`.
    6. `<day>` is that fixed day, and the test makes its files for it. Without the stub, a run across 00:00 UTC makes the two days differ.
    7. The temp directory holds one file per case.
    8. With `recipe_<day>_0200.dump.sha256`, the run prints the name and exits with the code 0.
    9. With `recipe_<day>_abcd.dump.sha256` alone, the run exits with a code other than 0. The same holds for `recipe_<day>_0200_premigrate.dump.sha256` alone.
  - The test also applies an allowlist to the section. The list of the section holds exactly the three lines of the blocks above.
  - The allowlist does not parse Markdown. It reads the raw lines of the section and applies three rules:
    1. Each raw line that holds the word `docker` in any case must equal one line of the list. The test strips the white space at both ends first. White space includes a tab and a CR.
    2. No raw line ends with `\` after the strip.
    3. Therefore the prose of the section holds no word `docker`. It names a command in plain text or links to a section.
  - A word is a run of ASCII letters, digits, and `_`. Each other character ends a word. The test matches `\bdocker\b` with the flags `re.ASCII` and `re.IGNORECASE`.
  - Thus `docker-compose`, `docker.io`, and `/usr/bin/docker` hold the word `docker`. `dockerd` and `Dockerfile` do not hold it.
  - The reason of rule 2: a `\` at the end of a line joins the next line in the shell. A list line that ends with `\` already fails rule 1.
  - Rule 2 therefore catches a line without the word `docker` before a list line. The line `: \` before the `rm` line makes the `rm` an argument of `:`, so the `rm` never runs.
  - Each other check of this task also compares the lines after the strip. Thus a copy of the runbook with CRLF line ends passes.
  - The test parses no option and no service name. Each raw line with the word `docker` that is not in the list fails the test.
  - Thus each raw line of the section with the word `docker` is a line of the list. A `stop` line cannot replace the `rm` line.
  - A heading, a link text, an HTML tag, and each line of a code block are raw lines too. Rule 1 applies to each of them.
  - The test reads four steps of `docs/ops/runbook.md`. Each step must hold a link to the section.
  - The test finds each step in its level-3 section with the finder of task 6.9 and the fixed start of the step. The step pattern of `deploy/ci/test_restore_switch.py` (line 2603) does the same.
  - The level-3 scope of "Rollback" holds its level-4 sections, also the two sections of tasks 6.9 and 6.10.
  - A step starts at the line that starts with its fixed start at the indent 0. It ends before the next line that starts with the next step number and `.` at the indent 0.
  - A sub-step line has an indent, so it does not end a step. When no such line follows in the level-3 scope, the test fails.
  - A step number alone matches two steps of "Rollback" (lines 1482 and 1510). The fixed start is unique:
    1. In "Restore into production", the step that starts with `4. Fill the new volume.` (line 663).
    2. In "Rollback", the step that starts with `3. Fill the new volume.` (line 1510).
    3. In "Switch back", the step that starts with `8. Start` (line 1586).
    4. In "Rollback", the step that starts with `1. Stop` (line 1506). The other step 1 of "Rollback" starts with `1. Run` (line 1475).
  - Step 1 of "Rollback" must also hold exactly one line `docker compose rm -s -f app` in a `bash` block (ADR-0174 (aj)).
  - The scope of step 1 of "Rollback" starts at the line `1. Stop` (line 1506). It ends before the line that starts with `2.` at the indent 0 (line 1508).
  - The test applies the same three rules to the raw lines of that scope with the same function as for the section. Its list has three lines from tasks 7.3 and 7.6:
    1. `docker compose stop backup`.
    2. `env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>`.
    3. `docker compose rm -s -f app`.
  - The reason for one rule: a refusal check of step 1 misses a global option, such as `docker compose -f compose.yaml stop app`.
  - Step 1 keeps its own list, because it needs the stop of `backup` and command 1. The section needs neither line.
  - Then add that section to `docs/ops/runbook.md` with the heading `#### Failed rollback restore after a failed upgrade`. It goes inside `### Rollback`, directly after the section of task 6.9.
  - Its scope ends before `### Switch back` (line 1540). Add its three blocks and one sentence before each block that names its case.
  - Note for the implementer: assert that the 6.10 scope ends at `### Switch back` and that the 6.9 scope ends at the heading of 6.10.
  - A case sentence and the heading hold no word `docker` (rule 3 of the allowlist).
  - Also add the link to that section in each of the four steps. Task 7.6 runs after this task, so this task writes the links and task 7.6 keeps them.
  - This task first adds one `bash` block to step 1 of "Rollback". The block has two lines, one line each: `docker compose stop backup` and command 1.
  - Also add to step 1 of "Rollback" one `bash` block with the line `docker compose rm -s -f app`. One sentence before it says: run it only after command 1 exits with the code 0.
  - Task 7.6 writes the other text of step 1 and keeps that block. Its new text keeps the fixed start `1. Stop`.
  - Prove that the test can fail. Make each of the 29 scratch edits below alone, and see the test fail each time. Then restore the runbook and the test.
  - Edits 2, 3, 8 to 11, 14 to 16, 23, 24, and 29 add an inline code span. It goes into a sentence of the section.
  - Edits 19, 20, 22, and 28 add lines to the section. Edits 1, 7, 18, and 26 change a block of the section.
  - Edits 13, 21, and 27 change the block of step 1 of "Rollback". Each edit names the one rule that catches it.
  - The rules are the three block compares, rule 1, and rule 2. They also include the link check, the `rm` check of step 1, the glob run, and the scope.
  - The edits are:
    1. Replace the line of the first block with `docker compose rm -s -f app`. Rule: the compare of the first block.
    2. Add the span `docker compose start app`. Rule: rule 1 of the section.
    3. Add the span `docker compose up -d`. Rule: rule 1 of the section.
    4. Delete the link from step 4 of "Restore into production". Rule: the link check.
    5. Delete the link from step 3 of "Rollback". Rule: the link check.
    6. Delete the link from step 8 of "Switch back". Rule: the link check.
    7. Swap the two lines of the second block. Rule: the compare of the second block.
    8. Add the span `docker compose start`. Rule: rule 1 of the section.
    9. Add the span `docker compose restart`. Rule: rule 1 of the section.
    10. Add the span `docker compose restart --timeout 30`. Rule: rule 1 of the section.
    11. Add the span `docker compose start --wait-timeout 60`. Rule: rule 1 of the section.
    12. Delete the link from step 1 of "Rollback". Rule: the link check.
    13. Add a second line `docker compose rm -s -f app` to the block of step 1 of "Rollback". Rule: the `rm` check.
    14. Add the span `docker compose up -d --wait --wait-timeout 60`. Rule: rule 1 of the section.
    15. Add the span `docker compose -f compose.yaml up -d`. Rule: rule 1 of the section.
    16. Add the sentence "Run `docker compose up -d` to start the stack." Rule: rule 1 of the section.
    17. Add the span `docker compose -f compose.yaml stop app` to a sentence of step 1 of "Rollback". Rule: rule 1 of step 1.
    18. Replace the line of the third block with `docker compose up -d --no-deps backup`. Rule: the compare of the third block.
    19. Add the line `<pre>docker compose up -d</pre>`. Rule: rule 1 of the section.
    20. Add a sentence with a span across two lines. One line ends with `` `docker compose ``, and the next line starts with `` up -d` ``. Rule: rule 1 of the section.
    21. Add the line `: \` directly before the line `docker compose rm -s -f app` in the block of step 1 of "Rollback". Rule: rule 2 of step 1.
    22. Add the sentence "Docker starts the container again after a reboot." Rule: rule 1 of the section.
    23. Add the span `docker-compose up -d`. Rule: rule 1 of the section.
    24. Add the span `docker start deploy-app-1`. Rule: rule 1 of the section.
    25. Replace `[0-9][0-9][0-9][0-9]` with `????` in the third block and in the expected line of the test. Rule: the glob run.
    26. Delete the line `docker compose rm -s -f app` from the second block. Rule: the compare of the second block.
    27. Delete the `bash` block of the `rm` line from step 1 of "Rollback". Rule: the `rm` check.
    28. At the end of the section, add the sub-heading `##### Note` and under it the raw line `docker compose up -d`, outside a block. Rule: the scope.
    29. Add the span `/usr/bin/docker compose up -d`. Rule: rule 1 of the section.
  - Note for the implementer: assert that the `rm` block of step 1 comes after the block with command 1. Add an edit that swaps the two blocks. The test must fail.
  - In edit 25, the list of rule 1 uses the same expected line, so rule 1 still passes. The run of `recipe_<day>_abcd.dump.sha256` exits with the code 0.
  - In edits 1, 13, and 18, each new line is a line of the list, so rule 1 passes. In edit 21, the line `: \` holds no word `docker`.
  - Rule 1 and rule 2 run in one function for the section and for step 1. Edit 2 and edit 17 prove rule 1 in each scope.
  - Prove that each edit proves one rule. For each edit, disable only the rule that it names in a scratch edit of the test. See the edit pass, then restore the test.
  - To disable the scope, make the finder stop at the next heading of any level. Then the line of edit 28 is outside the scope.
  - Prove three positive cases. The test passes in each case:
    1. The section and step 1 of "Rollback" hold the text of tasks 6.10 and 7.6. A scope that reads on into `### Switch back` fails this case. That section holds the line `docker compose up -d --no-deps app` (line 1589).
    2. A scratch edit changes the line ends of the runbook to CRLF.
    3. A scratch edit adds a copy of the line `docker compose up -d --no-deps backup` to the section. The copy follows a blank line and has one tab before it.
  - Then restore the runbook.
  - Step for a Sonnet agent: run `ste-lint.py docs/ops/runbook.md`. Check: it reports 0 hard violations.
  - Check: the runbook and the test hold no scratch edit, and `bash deploy/ci/check.sh` passes.

## 7. Documents

- [x] 7.1 (docs/architecture) Change `reliability.md` §7.1:
  - Name `backup_manifest_key`, its three readers, its owner uid 999, and the rule of ADR-0174 (p).
  - State that the rotation of ADR-0174 (q) is separate from `security.key_rotation_overlap`.
  - Step for a Sonnet agent: run `ste-lint.py docs/architecture/reliability.md`. Check: it reports 0 hard violations.
- [x] 7.2 (docs/architecture) Change `reliability.md` §7.7:
  - The tag file, the write order, step 1 of the restore test, the mode `--check-only`, and the secret of `restore-volume` (line 788).
  - The known limits of ADR-0174 (ag) to (ak).
  - Step for a Sonnet agent: run `ste-lint.py docs/architecture/reliability.md`. Check: it reports 0 hard violations.
- [x] 7.3 (docs/architecture) Change `reliability.md` §7.10 and lines 855, 865, and 866 of §7.8:
  - Create the key file before step 1.
  - Step 1 (line 912): note the new release tag from the release notes. Only command 1 of the `_premigrate` restore uses it, as `<new tag>` (ADR-0174 (aj)).
  - Step 2: the edit that sets the new tag also deletes `COMPOSE_FILE` from `deploy/.env` and runs `unset COMPOSE_FILE`. The copied `.env` can hold it from a rollback (ADR-0174 (aj)).
  - Each exit links to the runbook section "Leave the file compose.previous.yaml" of task 6.9 for its commands. §7.10 does not repeat them.
  - Step 2 does not create `backup` again. Command 1 of step 5 gives `backup` the new image and the key.
  - Give the reason: a new container in step 2 breaks the rule of line 913. Its `restore-test` can give a false alert `restore_test`.
  - Step 3 adds `deploy/compose.previous.yaml` when the probe finds no tag check in the previous image (ADR-0174 (aa)).
  - Command 3 of step 3 (the failed upgrade): set the previous tag in `deploy/.env`. When the probe finds no tag check, also set `COMPOSE_FILE` there.
  - Then, for the same probe result, run `docker compose up -d --no-deps --force-recreate backup` (ADR-0174 (aj)).
  - A failed upgrade can end in the `_premigrate` restore (line 931). That restore uses the rollback bullet below with its command 1.
  - The rollback bullet of the `_premigrate` restore: the three commands of ADR-0174 (aj), case 2, in their order, with the reason of the order.
  - Command 1 is `env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>`. Command 2 sets `COMPOSE_FILE` only when the probe finds no tag check.
  - Name the two states of `deploy/.env` for command 1. In the plain rollback, it still holds the new tag.
  - After command 3 of step 3, it holds the previous tag, and command 2 is already done.
  - Give the reason of `-f`, `env -u`, and `RECIPE_VERSION`. Without them, the second state runs the previous image, and it stops with `usage: restore-volume <stamp>`.
  - The order: stop `backup` before command 1. Only after a pass of command 1, delete the container of `app` with `docker compose rm -s -f app`.
  - Both steps come before `restore-volume <stamp>` (line 784). This order applies to both states of `deploy/.env`.
  - Give the reason of the order: a weekly `restore-test` of a running `backup` can hold the lock (`restore-volume` line 176). A failed check then leaves the running `app` unchanged.
  - Give the reason of `rm`: `app` has `restart: always` (`compose.yaml` line 126). A stopped container starts again after a reboot (runbook line 463).
  - Its writes then make the lost period of the rollback longer (runbook line 1494). That reason applies to the plain rollback too.
  - In the plain rollback, the later start of `app` with `docker compose up -d --no-deps app` creates the container again.
  - A failed check stops the rollback. Name the next step of each state of `deploy/.env`, as the bullets below give it.
  - The bullet that sets the new tag again (line 946): delete `COMPOSE_FILE` from `deploy/.env` and run `unset COMPOSE_FILE`. Then run `docker compose up -d --no-deps --force-recreate backup`.
  - State that this bullet and its exit apply only to the plain rollback. That bullet then starts `app` and `backup`.
  - After a failed upgrade, a failed check or a failed `restore-volume` keeps the previous tag and `COMPOSE_FILE` in `deploy/.env`. No step then starts a container of the new image.
  - In both cases, start only `backup` again with `docker compose up -d --no-deps backup`. After a failed check, `app` keeps running. After a failed `restore-volume`, `app` stays stopped.
  - After a failed `restore-volume`, first run `docker compose rm -s -f app` again. It changes nothing after the delete of step 1, and the runbook section of that state needs no earlier step.
  - Give the reason: a stopped container of the previous `app` starts again after a reboot (runbook line 463), on the schema of the failed `migrate`.
  - After a failed check, `app` keeps running, so that state needs no change. The switch back needs no `rm`, because its step 1 deletes the containers.
  - `app` stays stopped until a forward correction of ADR-0083, a new restore, or a restore with `pitr-volume` (ADR-0106).
  - While `app` stays stopped, no job `alerts` runs. Name the two checks of ADR-0174 (aj). The operator runs them once a day after `backup.time` plus `backup.max_age_alert` minus 24 h (§7.11):
    1. The stream check of `pg_stat_replication` in `db`: one row, the state `streaming`, and a `flush_lsn`.
    2. The dump check: a complete daily set has a stamp with the current UTC day (ADR-0066 (e)).
  - The dump check lists `recipe_<day>_[0-9][0-9][0-9][0-9].dump.sha256`. The four digits of the time follow ADR-0074 (c), so a foreign name such as `recipe_<day>_abcd.dump.sha256` does not count.
  - The dump check sets `LC_ALL=C` for the match, as ADR-0074 (c) requires. The runbook line of task 6.10 does this with `docker compose exec -T -e LC_ALL=C`.
  - Give the reason of the stamp: the age comes from the modification time of the manifest (`backup-health` line 64). An age check hides a failed dump until 04:00 UTC plus the dump time.
  - Give the time with the values of §7.11, 04:00 UTC, and its reason of ADR-0174 (aj). An earlier check can run before the end of the dump of the day and give a false fault.
  - At the later start of `app`, wait until the alert `replication` is inactive (§3.7).
  - The operator starts `app` earlier only after the operator checks that the schema of the old volume works with the previous image. An example is a migration of the expand pattern (line 931).
  - Give the three reasons of ADR-0174 (aj). Link to the runbook section of task 6.10 for this state.
  - Name the three places of the exit. They are step 2, the bullet of line 946, and the step of the switch back that sets the tag (§7.8, line 865).
  - The exit writes the tag of its place. At step 2 and at the bullet of line 946, it writes the new release tag. At line 865, it always writes the tag of the old volume.
  - After the plain rollback, the tag of the old volume is the new release tag. Only after a failed upgrade is it the previous tag.
  - At line 855, add: after a failed upgrade, only `backup` starts again, by the rule above. `deploy/.env` keeps the previous tag and `COMPOSE_FILE`.
  - At line 865, add this text. When the tag of the old volume has the tag check, that step also runs the exit. The switch back creates no `backup` again, because its last step creates `backup`.
  - At line 866, add this text. After a failed upgrade, the switch back starts only `backup` at its end, by the rule above. `app` and `proxy` stay stopped.
  - At line 866, also add: after a failed upgrade, the check after the start inspects only `backup`. It checks the WAL stream in `db` and does not wait for the alert `replication`.
  - Give the reason: the job `alerts` runs in `app` (line 905), and `app` stays stopped.
  - After each exit, `docker compose config` must show `backup_manifest_key` on the three services.
  - Step for a Sonnet agent: run `ste-lint.py docs/architecture/reliability.md`. Check: it reports 0 hard violations.
- [x] 7.4 (docs/ops) Change `docs/ops/runbook.md`, sections "Create the secret files" and "Copy the backups off site":
  - The ninth random value, its mode and owner, and one offline copy apart from the backup disk.
  - The check `df --output=source` of the key file and of the backup disk, next to line 177.
  - The tag file in the table of the off-site copy.
  - Step for a Sonnet agent: run `ste-lint.py docs/ops/runbook.md`. Check: it reports 0 hard violations.
- [x] 7.5 (docs/ops) Change `docs/ops/runbook.md`, section "Key rotation" and the disaster recovery steps:
  - The routine rotation and the compromise rotation of ADR-0174 (q). A compromise rotation follows each incident of the class of ADR-0084.
  - The comparison of the offline copy of ADR-0174 (r) at each copy and at each restore drill.
  - The restore of the key file from the copy, with its mode, owner, and mount, before the first restore on a new host.
  - The rule that no host account and no other container uses uid 999.
  - Step for a Sonnet agent: run `ste-lint.py docs/ops/runbook.md`. Check: it reports 0 hard violations.
- [x] 7.6 (docs/ops) Change `docs/ops/runbook.md`, sections "Messages of backup-once", the restore entries, the upgrade steps, the rollback steps, and "Known gaps":
  - The new `ERROR` lines of `design.md` D4.
  - The key file before the upgrade and the probe of the previous image.
  - The file of ADR-0174 (aa) for each run of an image without the check.
  - The failed-upgrade path and the rollback: when the probe finds no tag check, `COMPOSE_FILE` in `deploy/.env` names `compose.yaml` and `compose.previous.yaml` with the previous tag.
  - The failed-upgrade path: when the probe finds no tag check, run `docker compose up -d --no-deps --force-recreate backup` after the edit of `deploy/.env`.
  - The rollback with `_premigrate`: the three commands of ADR-0174 (aj), case 2, in their order. Command 1 is the command of task 7.3, with the placeholder `<new tag>`.
  - `<tag>` at line 1503 keeps one meaning: the tag that the old volume runs. It is the value of `RECIPE_VERSION` in `.env` now.
  - In the plain rollback, `<tag>` is the new tag. After a failed upgrade, `<tag>` is the previous tag. Rewrite line 1503 to state both states.
  - Add the placeholder `<new tag>` at line 1503 for command 1 only. It is the new release tag that step 1 of the section "Upgrade" notes. In the plain rollback, it equals `<tag>`.
  - Add to step 1 of the section "Upgrade" (line 1194): note the new release tag from the release notes.
  - Thus sub-step 4 of the switch (line 692) writes the tag of the old volume into the change log. Step 3 of "Switch back" (line 1567) reads that tag from the change log.
  - Step 4 of "Restore into production" (line 672) ends the attempt with the same `<tag>`. No step outside command 1 uses `<new tag>`.
  - The section "Major upgrade" already uses `<new tag>` (lines 1672 and 1694) with the same meaning, the tag of the new release.
  - Name the two states of `deploy/.env` of task 7.3. Step 12 (line 1236) and the fault list (line 1448) link to this command. They link for a `_premigrate` restore after a failed upgrade.
  - Step 1 of the rollback (line 1506): stop `backup` and run command 1. Only after a pass of command 1, delete the container of `app` with the block of task 6.10.
  - Keep the `bash` block of task 6.10 with the lines `docker compose stop backup` and command 1. The prose of step 1 names the stop of `backup` in plain text.
  - Give the reasons of task 7.3. The step applies to both states of `deploy/.env`. Keep the fixed start `1. Stop` and the link of task 6.10.
  - Write the reasons in plain text. The reason of `rm` says "the later start of `app`" and does not copy the command `docker compose up -d --no-deps app` of task 7.3.
  - Do not copy "a restart of Docker" of ADR-0174 (aj). Write "a restart of the container engine" instead.
  - Step 1 then runs only step 3 of "Restore into production". It does not run step 2, because step 2 stops `app` and `backup`. Step 1 of the rollback stops `backup` itself and deletes `app` only after the tag check. The text names step 2 by its number and copies none of its commands.
  - Step 1 has its own list in task 6.10. Each raw line of step 1 with the word `docker` in any case is a line of that list. The test applies the strip of task 6.10 first.
  - The prose of step 1 therefore holds no word `docker`. It names a command in plain text or links to a section. The current text of step 1 (lines 1506 and 1507) holds no word `docker`.
  - The next actions after a fault therefore name the services in plain text or link to a section.
  - The rollback stops on a fault. Name the next action of task 7.3.
  - This is for the plain rollback only. After a failed check, or a failed step 4 of "Restore into production" (lines 672 and 1511), `.env` holds `<tag>`, the new tag. Then the operator starts `app` and `backup`.
  - After a failed upgrade, a failed check or a failed step 4 keeps the previous tag and `COMPOSE_FILE` in `.env`. Then the operator starts only `backup` with the section of task 6.10.
  - Lines 672 and 1511 get this split. Line 672 then starts `app` and `backup` only outside the `_premigrate` restore after a failed upgrade.
  - Line 673 gets the split too. In the plain rollback, the old volume fits `<tag>` only.
  - After a failed upgrade, the old volume holds the schema of the failed `migrate`. That schema can match no image (ADR-0083). Then the section of task 6.10 applies.
  - Step 8 of "Switch back" (line 1586) gets the same split. After a failed upgrade, it starts only `backup` with the section of task 6.10. `app` and `proxy` stay stopped.
  - Step 9 of "Switch back" (line 1593) gets the split too. In every other case, step 9 stays unchanged.
  - After a failed upgrade, step 1 of that section deleted the container of `app`. The job `alerts` runs in `app` (`reliability.md` line 905).
  - Step 9 then runs the `docker inspect` command of sub-step 6 of step 9 of "Restore into production" (line 813). That command uses the service name `backup` only.
  - Step 9 does not wait for the alert `replication` (line 814). Step 9 runs the stream check of step 9 of "A gap in the WAL stream" (line 2105) instead.
  - That check is a query in `db` and needs no `app`. It passes on one row with the state `streaming` and a `flush_lsn`.
  - The reason for this choice: `app` can stay stopped for a long time, and `pitr-volume` needs the WAL archive. A wait at the later start of `app` leaves the stream unchecked until then.
  - The stream check does not compare the lag with `wal.max_lag_bytes`. The later start of `app` therefore waits for the alert `replication`, as below.
  - Write the text of the section "Failed rollback restore after a failed upgrade" around its three blocks. Keep the blocks, the case sentences, and the links of task 6.10 unchanged.
  - Keep the level-4 heading of that section and its place in `### Rollback`. A sub-heading inside the section stays in the scope of the test.
  - Thus the test of task 6.10 passes. Each change of the three steps keeps the fixed start of the step that this test reads.
  - That text names the cases. After a failed check, `app` keeps running, and the first block applies. After a failed full restore, `app` stays stopped, and the second block applies.
  - After a switch back, `app` stays stopped, and the first block applies. Step 1 of "Switch back" already deleted the container of `app`.
  - Give the reason of the `rm` line of task 7.3. A stopped `app` with `restart: always` starts again after a reboot (line 463).
  - State that step 1 of "Rollback" already deleted the container of `app`. The `rm` line of the second block repeats that delete and changes nothing then.
  - It names the next action: a forward correction of ADR-0083, a new restore, or `pitr-volume` (ADR-0106).
  - It names the one condition for an earlier start of `app`. The operator checks that the schema of the old volume works with the previous image. Give the three reasons of ADR-0174 (aj).
  - While `app` stays stopped, no job `alerts` runs. The operator runs two checks once a day after 04:00 UTC, as task 7.3 gives that time (`reliability.md` §7.11):
    1. The stream check of step 9 of "A gap in the WAL stream" (line 2105). The text links to that step and does not copy its command.
    2. The dump check of task 7.3, with the third block of task 6.10. It passes when it prints a name and exits with the code 0. Its option `-e LC_ALL=C` sets the locale of the match (ADR-0074 (c)).
  - Give the reason of the glob. `[0-9][0-9][0-9][0-9]` matches only the four digits of the time of a daily set (ADR-0074 (c)). A `_premigrate` set, a `_now` set, or a foreign name such as `recipe_<day>_abcd.dump.sha256` does not count.
  - A foreign name would hide a missing set of the day and give a false pass. The run of the glob in the test of task 6.10 refuses that name.
  - The prose of the section holds no word `docker` in any case. Each raw line with that word is a line of the list of task 6.10.
  - A heading, a link text, or a reason names a command in plain text. The rule of the `rm` reason and of "a restart of Docker" in step 1 applies here too.
  - It links to "Stop, start, and restart" for the start after the confirmation. Thus the test of task 6.10 passes.
  - Add one line to the section "After a host reboot" (line 458). It names the state of a stopped `app` after a failed rollback restore after a failed upgrade.
  - That line says that steps 1 and 4 cannot pass in that state. It links to the section of task 6.10 and not to a start of `app`.
  - After that start, the operator waits until the alert `replication` is inactive. Line 814 does the same (step 14 of "A gap in the WAL stream").
  - The exit: each edit that writes a tag with the check deletes `COMPOSE_FILE` from `deploy/.env` and runs `unset COMPOSE_FILE`.
  - Write the text of the section "Leave the file compose.previous.yaml" around its block. Keep the block of task 6.9 unchanged, so the test of task 6.9 passes.
  - Keep the level-4 heading of that section and its place in `### Rollback`. A sub-heading inside the section stays in the scope of the test.
  - Add one sentence about an interrupt. After an interrupt, the operator deletes a left file `.env.??????`. `.env` stays unchanged because the `mv` is atomic.
  - Add text about the owner. `chown --reference` of an operator without root fails when another uid owns `.env`, for example root after a `sudo` edit.
  - Then the function keeps `.env` and returns a code other than 0. Give the fix: give `.env` back to the operator, for example with `sudo chown`, and run the function again.
  - The three places of the exit link to that section. Then `docker compose config` must show `backup_manifest_key` on the three services. The places are:
    1. The upgrade step that sets the new tag in the copied `.env` (line 1212). Step 7 of "Major upgrade" (line 1679) writes `<new tag>` with step 6 of "Load the images" (line 301). Step 7 therefore links there too. `<write tag>` is the new release tag.
    2. The rollback step that sets the new tag again (lines 672 and 1511), in the plain rollback only. `<write tag>` is the new release tag, which equals `<tag>` there.
    3. Step 3 of "Switch back" (line 1567). Use it only when the probe of ADR-0174 (aa) finds the tag check in the image of `<tag>`. Else `COMPOSE_FILE` stays. `<write tag>` is the `<tag>` of that section, the tag of the old volume.
  - Each place names its value of `<write tag>`. Place 3 always writes the tag of the old volume. After the plain rollback, that tag is the new release tag.
  - Only after a failed upgrade is that tag the previous tag. Then place 3 must not write the new release tag, for the reason of task 6.9.
  - Only the rollback step then creates `backup` again with the command above. The upgrade step does not, for the reason of task 7.3.
  - Step 3 of "Switch back" does not. Step 1 of that section deletes the containers, and its step 8 creates `backup` with the key.
  - The other places that write `RECIPE_VERSION` into `.env` need no exit. Line 301 at a first installation finds no `COMPOSE_FILE`.
  - Lines 1236, 1447, and 1508 write the previous tag. They get the step of `COMPOSE_FILE` of task 7.3 instead.
  - In the upgrade, the step `docker compose up -d --no-deps backup walarchive` gives `backup` the new image and the key.
  - The rule of ADR-0174 (z): only an image with the check restores a tagged set, except the case of (aj).
  - The known limits of ADR-0174 (ag) to (ak).
  - Step for a Sonnet agent: run `ste-lint.py docs/ops/runbook.md`. Check: it reports 0 hard violations, and `bash deploy/ci/check.sh` passes.
- [x] 7.7 (openspec/specs) [mechanical] Check that the delta `specs/operations/spec.md` still matches ADR-0174. Check: `openspec validate backup-manifest-hmac --strict` passes.
- [x] 7.8 (CHANGELOG.md) [mechanical] Add an entry under "Unreleased" for the tag of the backup manifest. Mark as BREAKING the step that creates `backup_manifest_key` before the upgrade. Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.
- [ ] 7.9 (deploy) Run the gate: `bash deploy/ci/check.sh`, `cargo test --workspace --all-features --locked`, and the five suites of `deploy/ci/pgtools.sh`. Also run the CI jobs `compose`, `restore-switch`, and `upgrade`, `openspec validate --all`, and the `ste-lint.py` command of `CLAUDE.md`. Check: each command passes.

## Changes outside the tasks

- The shared function `manifest_tag_message` in `deploy/pgtools/lib/manifest-tag.sh` maps each fault name of `manifest-tag.pl` to the ERROR text of design.md D4. `backup-entry`, `backup-once`, `restore-test`, and `restore-volume` use it.
- The key check names the exact fault: `key-missing`, `key-nul`, `key-cr`, `key-lines`, and `key-line <n>`. The code follows ADR-0174 (k), (l), and (n).
- The review of the change added the fixes of its findings. The commit message lists them.
