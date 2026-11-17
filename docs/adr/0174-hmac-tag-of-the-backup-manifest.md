# ADR-0174: HMAC tag of the backup manifest

Status: Accepted. Amended by ADR-0176 (item (aj): delete the container of `backup` before command 1)
Date: 2026-11-15
Deciders: project owner

## Context

ADR-0095 (h) records a known limit of the restore test. A writer of the backup disk can replace a set and compute a new manifest.

1. The manifest `recipe_<stamp>.dump.sha256` holds only SHA-256 values (ADR-0066 (f)). Any writer of the disk can compute them again.
2. `pg_restore` runs the definition text of each entry of the dump as the scratch superuser. Step 2 of ADR-0067 compares only the names of three entry kinds (ADR-0076 (b)).
3. A crafted entry of another kind, for example `TABLE`, therefore runs SQL in `pre-data`, before the comparison of step 4.
4. `restore-volume` runs the same gates and then restores the dump into the cluster that becomes production (ADR-0103 (i), (o)).
5. Today `backup-once` computes the manifest from the published files on the backup disk (`backup-once` line 315). The manifest therefore can cover bytes that a writer changed after the publish.

ADR-0095 names option D: an HMAC of the manifest with a key outside the backup disk. On 2026-11-15 the project owner chose option D for the follow-up ADR of ADR-0095 (h). A restore refuses a set whose manifest has a bad or missing tag. The refusal comes before any SQL of the set runs.

On the same day the owner decided the threat model and the four open questions of the change `backup-manifest-hmac`. This ADR records these decisions.

The other candidate of ADR-0095 (h), a comparison of the output of `pg_restore --file`, is not part of this decision.

## Decision drivers

- No SQL of a backup set runs before a check that an offline writer of the backup disk cannot forge (R31, R38).
- The key never lies on the backup disk, in an argument list, in an environment value, or in a shell variable (ADR-0066 (o), `reliability.md` §7.7).
- The image `recipe-pgtools` gets no new package (ADR-0064).
- A missing or bad key stops a run with one `ERROR` line. No run continues without the check (ADR-0031).

## Considered options

### Option A: Perl with the module `Digest::SHA` of the base image

- Pros: the base image `postgres:16.15-bookworm` has `/usr/bin/perl` (package `perl-base`) and `Digest/SHA.pm` (package `libperl5.36`). The program reads the key from the file itself. The key is in no argument and in no shell variable.
- Cons: Perl is a new language in the repository. The shell scripts call one small program.

### Option B: `openssl dgst -mac HMAC` or `openssl mac`

- Pros: `/usr/bin/openssl` 3.0.22 is in the base image.
- Cons: OpenSSL 3.0 takes the key only as an argument (`-hmac`, `-macopt key:` or `-macopt hexkey:`). The key is then visible in `/proc/<pid>/cmdline`. This breaks ADR-0066 (o).

### Option C: HMAC in bash from `sha256sum` and `printf`

- Pros: no new language.
- Cons: the key bytes pass through shell variables for the XOR with the pads. This breaks ADR-0066 (o). A hand-written HMAC is custom cryptography.

### Option D: `pgcrypto` `hmac()` in the scratch cluster

- Pros: no new tool.
- Cons: the scratch server must start before the check. The check must come before any PostgreSQL tool runs (ADR-0095 (a)).

## Decision

The project uses **Option A**.

Threat model:

- (a) The tag protects against an **offline writer** of the backup disk. An offline writer changes a set while no run of `backup`, `restore-test`, or `restore-volume` works on the disk.
  - Examples: an off-site copy that comes back, a detached disk, and a copy on another host.
- (b) An **online writer** is a known limit of this ADR. An online writer changes the disk while a run works on it.
  - It is a process with uid 999 or root on the host, or a container with uid 999.
  - Known limits (ag) and (ah) name the points where an online writer defeats the tag.

Scope:

- (c) The tag covers each set of the backup disk: the dump, the globals file, the counts file, and the manifest (ADR-0066 (e), (f)).
  - The manifest holds the SHA-256 and the name of the other three files. A valid tag therefore binds all four files and the stamp of the set.
- (d) A base backup and the WAL archive get no tag in this ADR. They lie on the WAL archive disk (ADR-0066 (a)).
  - `pitr-check` and `pitr-volume` run no SQL text from a file of that disk. `pg_verifybackup` checks each base backup against its `backup_manifest` (ADR-0068, ADR-0106).
  - `pitr-volume` stays a separate trust boundary. Known limit (ak) names its path.

Tag:

- (e) The algorithm is HMAC-SHA-256 with a key of 32 bytes.
- (f) The message is the ASCII text `recipe-backup-manifest-v1`, one line feed, and then each byte of the manifest file. The label separates this use of the key from every other use.
- (g) The tag file is `recipe_<stamp>.dump.sha256.hmac` in the same directory as the manifest. Its mode is `0600`, as every set file (ADR-0066 (m)).
- (h) The tag file holds one line and one line feed: `hmac-sha256-v1 <key id> <tag>`.
  - `<key id>` is the first 16 hex characters of HMAC-SHA-256 with the same key over the ASCII text `recipe-backup-key-id-v1`.
  - `<tag>` is 64 lower-case hex characters.
  - The first field names the algorithm and the format version, so a later change can add a version.
- (i) The tag file name does not match the set grammar of ADR-0074 (c). The tag file therefore never counts as a set. The adapter of `BackupDirectory`, `backup-health`, and the job `alerts` ignore it.
- (j) Test vector. Python `hmac` and `openssl dgst -sha256 -mac HMAC` gave the same tag for these inputs:
  - Key: `000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f`.
  - Manifest of 278 bytes: three lines. Each line holds 64 hex characters, two spaces, a name, and a line feed.
  - Line 1: 64 times `0` and `recipe_20261115_0200.dump`. Line 2: 64 times `1` and `globals_20261115_0200.sql`. Line 3: 64 times `2` and `recipe_20261115_0200.counts`.
  - Tag: `e047af28abfb9ccdd1867c6813ea85e66e50bd453b35726b79a1cccee8e6dd4d`.
  - Key id: `e60535bd89336ac1`.

Tool:

- (k) One Perl program `deploy/pgtools/lib/manifest-tag.pl` computes a tag, checks a tag, and prints the key id of each line of a key file. It uses `hmac_sha256_hex` of `Digest::SHA`.
  - The program has `use strict` and `use warnings`. Each `open` has three arguments.
  - It takes the path of the key file as an argument, never the key. It reads the key file itself. It writes no byte of the key to stdout, stderr, or a file.
  - It reads at most `backup_manifest.max_tag_bytes` (4096) bytes of a tag file. It reads at most `backup_manifest.max_file_bytes` (65536) bytes of a manifest or a key file.
  - A larger file is a fault. The new rows of `reliability.md` §7.11 hold both values. The container of a restore has 128 MiB of memory or more.
  - It compares the two tags in constant time.
  - On a fault it exits with a code other than 0 and writes one fixed fault name. The calling script maps that name to its `ERROR` line.
- (l) One shell file `deploy/pgtools/lib/manifest-tag.sh` holds the wrappers. `backup-once`, `backup-entry`, and `lib/restore-steps.sh` source it. The `Dockerfile` copies both files into `/etc/pgtools`.

Key:

- (m) The key is the Docker secret `backup_manifest_key`. The host file is `${RECIPE_SECRETS_DIR}/backup_manifest_key` with the mode `0400` and the uid 999.
  - The container file is `/run/secrets/backup_manifest_key`, the value of the new key `secrets.backup_manifest_key_file` of `reliability.md` §7.11.
  - The services `backup`, `backup-premigrate`, and `restore-volume` mount it. No other service of `compose.yaml` mounts it.
  - One CI service is an exception: `upgrade-rollback` of `deploy/ci/upgrade.compose.yaml` mounts it only when the previous image checks the tag (aa).
  - A script reads the path from `BACKUP_MANIFEST_KEY_FILE`. The default is the value of §7.11. Only the test scripts set another value.
- (n) The key file has the grammar of a key file of `reliability.md` §7.1. Each line holds one key as 64 lower-case hex characters. The first line is the newest key.
  - The file holds at most `backup_manifest.max_keys` lines. The new row of §7.11 sets 2.
  - The line ends follow the Rust reader `KeyRing::parse` (`crates/recipe-domain/src/identity/key_ring.rs` line 84, `str::lines`). A final line feed is optional.
  - Two line feeds at the end give an empty line, and an empty line stops a run.
  - A missing or unreadable file stops a run with one `ERROR` line. An empty file also stops it.
  - A line that is too short or too long stops a run. A character other than `0-9a-f` also stops it.
  - A carriage return, a NUL byte, and a third line also stop a run (ADR-0031 and its note).
  - The `ERROR` line holds no byte of the file. It may name the line number.
- (o) `backup-once` signs with the key of the first line. A check accepts a tag of either key line.
- (p) The key file and `BACKUP_DIR` must lie on two different filesystems. A script compares `stat -c %d` of the key file and of `BACKUP_DIR`, as `backup-health` does for `pgdata` (ADR-0066 (d)).
  - An equal value stops the run with the `ERROR` line "the manifest key file lies on the backup disk".
  - This check proves two filesystems only. It does not prove two physical disks. The runbook check with `df --output=source` names the devices.
- (q) The rotation of this key is separate from `security.key_rotation_overlap` of `reliability.md` §7.1. A key line stays while a kept set, a `_premigrate` set, or an off-site copy needs it.
  - Routine rotation: the operator adds the new key as the first line and keeps the old key as the second line. The operator deletes the old line when no kept set and no off-site copy has its key id.
  - Compromise rotation: the operator writes a file with only the new key. The restores then refuse each set of the old key.
  - The operator runs a compromise rotation after each incident of the class of ADR-0084. The SQL of such a set can read the key (known limit (ah)).
- (r) The operator keeps one offline copy of the key file, apart from the backup disk and the off-site copy. The runbook gives these steps:
  - At each copy and at each restore drill, the operator compares the copy with the host file. The operator then compares the key ids with the key id of the newest tag file.
  - The key ids come from the mode `key-ids` of the program of (k). The key id of a set is the second field of its tag file.
  - On a new host, the operator restores the key file from the copy before the first restore. The file gets the mode and the owner of (m).
  - Each rotation also updates the copy. The copy keeps each key that a kept set or an off-site copy needs.

Write order:

- (s) `backup-once` checks the key file in step 2, before the snapshot session starts.
- (t) `backup-once` hashes each file of the set while the file still has its temp name. It never reads a published file back for the manifest.
  - The run computes the SHA-256 of the temp dump after `pg_dump` closes it. It computes the hashes of the temp counts file and the temp globals file after their writers close them.
  - The run holds the lock of ADR-0066 (h) during these steps. The hash of each temp file comes before its publish.
  - The run writes the manifest lines from these hashes and the final names, in the format of `sha256sum`. `sha256sum -c --strict` therefore still reads the manifest.
- (u) Step 5 of `backup-once` then runs these steps:
  1. It publishes the dump, the globals file, and the counts file with hard links (ADR-0074 (b)).
  2. It writes the manifest under its temp name.
  3. It computes the tag of the temp manifest and writes it under the temp name `.tmp-recipe_<stamp>.dump.sha256.hmac`. The write opens the file with `O_CREAT` and `O_EXCL`. Bash opens an existing target that is not a regular file without `O_EXCL`. Only an online writer can make such a target, and the tag check then fails (a).
  4. It runs `sync` and publishes the tag file.
  5. It publishes the manifest last. The manifest stays the mark of a complete set.
  - `backup-once` lines 266 to 272 refuse an existing file or link with a name of the new set. This check also covers the tag file name and its temp name.
  - A failed step publishes no manifest. The cleanup then deletes the temp names and the published files of the run, the tag file included.
  - The retention deletes the manifest first and then the tag file with the other files of the set.
- (v) `backup-entry` checks the key file with the rules of (n) and (p) at each start, after the marker file. A fault stops the container before supercronic starts.

Check order:

- (w) Step 1 of `restore-test` and step 3 of `restore-volume` run in this order:
  1. The marker, the lock, the code files, and the private work directory in `/run/pgpass`, as before.
  2. The selection of the set. `restore-test` takes the newest manifest. `restore-volume` takes the manifest of its stamp.
  3. A private copy of the manifest and of the tag file in the work directory. A missing tag file or a tag file that is not a regular file stops the run.
  4. The check of the key file of (n) and (p).
  5. The check of the tag over the private copy of the manifest. A malformed line, an unknown key id, or a different tag stops the run.
  6. The checks of the manifest, the counts file, and the globals file, as before. They read the private copy of the manifest. The counts file also gets a private copy.
  7. The name list and the refused kinds of step 2 of ADR-0067.
  - Steps 2 to 5 read only the manifest, the tag file, and the key file. They run no PostgreSQL tool.
  - `restore-test` stops on a bad tag of the newest set. It does not select an older set.
  - The command `restore-volume --check-only <stamp>` runs steps 1 to 6 with the functions of a full restore. It has no copy of a check.
  - The mode runs `check_inputs` without the two target checks (`restore-volume` lines 162 to 165), then `take_lock`, then `run_gates` up to `check_globals`.
  - The mode does not read the target directory, because it writes no target. It therefore needs no target volume. It still checks the user, the stamp, and `COMPOSE_SUBNET`.
  - The mode stops before `check_names`, which runs `pg_restore --list`. It therefore runs no PostgreSQL tool and writes no file outside its work directory.
  - The mode writes no line "restore volume started". On a pass, it writes one `INFO` line "the set passed the check" with the field `set` and exits with the code 0.
  - A fault of steps 1 to 6 stops the mode with an exit code other than 0. The mode writes the same `ERROR` line as a full restore. The rollback of (aj) uses this command.
- (x) A restore always refuses a set without a valid tag. No option of a script skips the check. This includes the sets from before this ADR and the sets of a lost key.
- (y) `restore-volume` now mounts one secret. With the file of (aa), it mounts no secret. It still reads no password, sets no known password, and joins no network.

Images:

- (z) Only an image that checks the tag restores a set with a tag file. An image from before this ADR ignores the tag file of each set of its schema. Known limit (aj) names the exceptions of the first release and their safeguards.
- (aa) An image without the tag check never gets the key. The new Compose file `deploy/compose.previous.yaml` holds the changes for such an image.
  - It keeps `backup_password` for `backup` and `backup-premigrate` with `!override`. It sets no secret for `restore-volume` with `!reset []`. `compose.dev.yaml` uses the same tags for `proxy`.
  - It sets `BACKUP_CRONTAB` of `backup` to the mounted file `deploy/crontab.previous`. That file holds the line of `backup-once` and no line of `restore-test`.
  - The previous `backup-entry` reads `BACKUP_CRONTAB` (`backup-entry` line 12). The rolled-back `backup` therefore runs the nightly dump and no weekly `restore-test`.
  - An image has the tag check when it holds `/etc/pgtools/manifest-tag.pl`. A probe of the image gives the result before each use of the file.
  - Each Compose command that runs one of the three services with an image without the tag check adds this file.
  - An image with the tag check runs without this file and gets the key. The CI test of `deploy/ci/upgrade.sh` covers both results of the probe.

Faults and alerts:

- (ab) The scripts of `recipe-pgtools` write `ERROR` lines and no error code. `reliability.md` §1.2 therefore gets no new row.
  - A failed `restore-test` writes the result `failed`. The job `alerts` raises `restore_test` (ADR-0151 (h)).
  - A failed `backup-once` publishes no manifest. The job `alerts` raises `backup_age` after `backup.max_age_alert` (`reliability.md` §3.7).
  - `reliability.md` §7.11 gets four rows: `secrets.backup_manifest_key_file`, `backup_manifest.max_keys`, `backup_manifest.max_tag_bytes`, and `backup_manifest.max_file_bytes`.

Amended items:

- (ac) This ADR closes the path of ADR-0095 (h) for an offline writer of the backup disk. It amends ADR-0095 (h).
- (ad) It amends ADR-0066 (e), (f), and (h) step 5. A set gets the tag file. The run hashes the temp files and publishes the tag file before the manifest.
- (ae) It amends ADR-0074 (b) and (c). The tag file has a temp name, and it is no set.
- (af) It amends ADR-0067 step 1 with the check order of (w). It amends ADR-0103 (h) and (i) step 3. `restore-volume` mounts `backup_manifest_key` and runs the tag check.

Known limits:

- (ag) An online writer of (b) defeats the tag at three points. The project adds no measure for them.
  1. On the write side, it can change a temp file. This happens between the close by its writer and the hash of (t).
  2. After the check, it can write into the dump file in place. `pg_restore` reads the dump on the disk four times after the check (`lib/restore-steps.sh` lines 153 and 207 to 210).
  3. The scratch cluster of `restore-test` lies on the backup disk in `/backups/.scratch` (`restore-test` lines 191 to 208). An online writer can change its files while the scratch server runs.
- (ah) The key lies in the container where the SQL of a set runs as a superuser. Code of the class of ADR-0084 can read the key with `pg_read_file` and write it to `/backups`.
  - A holder of the key, or of a shell as uid 999 in `backup`, can sign a crafted set.
  - The attacker of ADR-0084 holds the migrator credential and changes production. `backup-once` then signs the dump of that state. ADR-0084 and ADR-0103 (o) stay unchanged.
  - The runbook therefore asks for a compromise rotation after each such incident. It also forbids a host account and another container with uid 999.
- (ai) The tag does not cover the modification time of a file. `backup-health` (line 64) and the adapter of `BackupDirectory` (`crates/recipe-server/src/backups.rs` line 134) read the age from that time.
  - A writer can delete the newer sets and change the times. The alert `backup_age` then gives no proof of a replay.
  - The readers of the age hold no key. The stamp in the signed manifest therefore gives them no better proof. Only the tag check of a restore proves a set.
- (aj) An image from before this ADR ignores the tag file of each set of its schema, not only of a `_premigrate` set. This limit holds only for the transition from the release before this ADR to the first release with it.
  - The previous image runs only in step 3 of `reliability.md` §7.10, in a failed upgrade, and in a rollback. Three cases follow.
  1. `restore-test` of step 3 (ADR-0105 (g)). The newest set then comes from the previous image and has no tag.
  2. A rollback `restore-volume` with the `_premigrate` set (ADR-0103 (q)). The new image wrote and signed that set, and the previous image does not check the tag.
  3. The `backup` service of the previous image after a rollback or a failed upgrade.
  - Safeguard of case 2: the rollback restore runs these three commands in this order.
    1. In the new `deploy/`, the operator runs `env -u COMPOSE_FILE RECIPE_VERSION=<new tag> docker compose -f compose.yaml run --rm restore-volume --check-only <stamp>` of (w).
    2. Only on the exit code 0, the operator sets the previous tag in `deploy/.env`. When the probe of (aa) finds no tag check, the operator also sets `COMPOSE_FILE` there.
    3. Then the operator runs `restore-volume <stamp>` with the previous image, in the order of `reliability.md` §7.7.
  - Command 1 is the same in two states of `deploy/.env`. In the rollback of §7.10 step 6, `deploy/.env` still holds the new tag.
  - A failed upgrade can end in the `_premigrate` restore. Then command 3 of §7.10 step 3 already set the previous tag and `COMPOSE_FILE`, and command 2 is already done.
  - The option `-f compose.yaml` and `env -u COMPOSE_FILE` ignore each `COMPOSE_FILE`. The variable `RECIPE_VERSION` selects the new image in both states.
  - `<new tag>` is the tag of the new release. The operator notes it from the release notes in step 1 of §7.10. Only command 1 uses it.
  - Without these parts, the second state runs the check with the previous image. That image stops with `usage: restore-volume <stamp>` (`restore-volume` line 347). This stop looks like a failed check.
  - The order matters. The new image with the file of (aa) has no key, so its check stops. The previous image has no mode `--check-only`.
  - The operator stops `backup` before command 1. After a pass of command 1, the operator deletes the container of `app` with `docker compose rm -s -f app`. Both steps still come before `restore-volume <stamp>`, as §7.7 requires.
  - A weekly `restore-test` of a running `backup` can hold the lock of `restore-volume` line 176. Command 1 then stops with a fault that the set does not cause.
  - `app` stops after the check, so a failed check leaves the running `app` unchanged.
  - `app` has `restart: always` (`compose.yaml` line 126). A stopped container starts again after a reboot or a restart of Docker.
  - Its writes then make the lost period of the rollback longer, and no message shows them. After a failed upgrade, the previous `app` also starts on the schema of the failed `migrate`.
  - The longer lost period applies to both states of `deploy/.env`. The delete therefore applies to the rollback of §7.10 step 6 too. A later start of `app` with `docker compose up -d --no-deps app` creates the container again.
  - In this restore, `backup` stays stopped in the order of §7.7. The last command of that order starts it, with the file of (aa) when the probe finds no tag check.
  - A failed check stops the rollback. The next step depends on the state of `deploy/.env`.
  - In the rollback of §7.10 step 6, the bullet of that step sets the new tag again. Then the operator starts `app` and `backup`. This bullet and its exit from the file of (aa) apply only to that rollback.
  - After a failed upgrade, a failed check or a failed `restore-volume <stamp>` keeps the previous tag and `COMPOSE_FILE` in `deploy/.env`. No step then starts a container of the new image.
  - After a failed check, the operator starts only `backup` again. `app` keeps running, because it stops only after a pass of command 1.
  - After a failed `restore-volume <stamp>`, the operator also starts only `backup` again, with the previous tag. `app` stays stopped.
  - In that state, the operator first runs `docker compose rm -s -f app` again. The command changes nothing after the delete of the check step. It keeps the runbook section of that state free of a link to an earlier step.
  - After a failed check, `app` keeps running, so this state needs no change. Step 1 of the switch back deletes the containers, so the switch back needs no change either.
  - The switch back of ADR-0103 (g) after a failed upgrade follows the same rule. Its last step then starts only `backup`.
  - Its check after the start then inspects only `backup`. It checks the WAL stream in `db` and does not wait for the alert `replication`, because the job `alerts` runs in `app`.
  - `app` stays stopped until a forward correction of ADR-0083, a new restore, or a restore from the WAL archive with `pitr-volume` (ADR-0106).
  - While `app` stays stopped, no job `alerts` runs. The operator therefore runs two checks once a day, after `backup.time` plus `backup.max_age_alert` minus 24 h (`reliability.md` §7.11). With the values of §7.11, this time is 04:00 UTC.
    1. The stream check: a query of `pg_stat_replication` in `db` must show one row with the state `streaming` and a `flush_lsn`.
    2. The dump check: a complete daily set must have a stamp with the current UTC day.
  - The stamp is `<YYYYMMDD_HHMM>` in UTC, and a daily set has no suffix (ADR-0066 (e)). The manifest is the mark of a complete set.
  - In this state, `backup` runs the previous image (case 3 of (aj)). That image writes the manifest last (ADR-0066 (h) step 5, ADR-0074 (b)). An image with the check also writes it last (step 5 of (u)).
  - The dump check lists `recipe_<day>_[0-9][0-9][0-9][0-9].dump.sha256` in `/backups`. `<day>` is the current UTC day, and ADR-0074 (c) gives the time four digits.
  - The dump check sets `LC_ALL=C` for the match in `backup`, as ADR-0074 (c) requires.
  - Thus a `_premigrate` set, a `_now` set, or a foreign name such as `recipe_<day>_abcd.dump.sha256` does not count. A foreign name would hide a missing set of the day.
  - The dump check reads the stamp and not the age. `backup-health` takes the age from the modification time of the manifest, and the run writes the manifest last.
  - At 04:00 UTC, the age of the set of the day before is then 26 h minus the dump time. An age check therefore hides a failed dump until 04:00 UTC plus the dump time.
  - The check time keeps one reason. The daily run starts at `backup.time`, so a check before the end of the dump gives a false fault.
  - The check time gives the dump the margin of the alert `backup_age`, 2 h with the values of `reliability.md` §7.11.
  - At the later start of `app`, the operator waits until the alert `replication` is inactive.
  - The operator starts `app` earlier only after the operator checks that the schema of the old volume works with the previous image. An example is a migration of the expand pattern (`reliability.md` §7.10 step 3).
  - The owner chose this rule for three reasons:
    1. The old volume holds the schema of the failed `migrate`. That migration did not follow the expand pattern, so the `_premigrate` restore runs. ADR-0083 warns of a schema that matches no image.
    2. Each write after a restart of `app` makes the lost period longer. This holds at a retry or at a restore with `pitr-volume`.
    3. The project gives data safety priority over availability.
  - Thus the new `app` and the new `backup` never start on the schema of a failed `migrate`. The previous `app` starts on it only after that confirmation.
  - Safeguard of case 3: the file of (aa) turns off the weekly `restore-test` of the previous image until the next upgrade.
  - In the failed-upgrade path of command 3 of §7.10 step 3, no Compose command creates `backup` again. `backup` depends on `db` only (`compose.yaml` lines 4 and 5).
  - The running container of the previous image therefore keeps its weekly `restore-test` (`crontab.backup` line 6). The signed `_premigrate` set of command 1 can then be the newest set.
  - When the probe finds no tag check, the operator edits `deploy/.env` and then runs `docker compose up -d --no-deps --force-recreate backup`.
  - Safeguard of each case: while `deploy/.env` holds the tag of a previous image without the tag check, `COMPOSE_FILE` there names `compose.yaml` and `compose.previous.yaml`.
  - The previous image therefore never gets the key. Each step that sets `COMPOSE_FILE` runs only when the probe of (aa) finds no tag check.
  - A later previous image with the tag check gets no `COMPOSE_FILE`. It therefore gets the key, and its `backup-entry` starts (v).
  - Exit from the file of (aa): each edit that writes a tag with the tag check into `deploy/.env` also deletes `COMPOSE_FILE` from it.
  - The exit changes `deploy/.env` with one atomic replace. The replace writes a temp file in `deploy/` with the owner and the mode of `.env`, then renames it over `.env`. Each other line stays unchanged.
  - The exit sets no `trap`, because a trap in a pasted function replaces the traps of the operator and the `EXIT` trap of CI. After an interrupt, the operator deletes a left file `.env.??????`, and `.env` stays unchanged.
  - Without this exit, the new image gets no key, and `backup-entry` stops (v).
  - Three places write such a tag. The first is the bullet of §7.10 step 6 that sets the new tag again, in the rollback of §7.10 step 6 only. After that exit, the operator runs the command of case 3 again.
  - The second place is step 2 of the next upgrade, because that step copies `deploy/.env`. A major upgrade (§7.10 step 8) runs the same step. There the exit does not create `backup` again.
  - Command 1 of §7.10 step 5 then gives `backup` the new image and the key.
  - A new container in step 2 starts the new image before command 1 of step 3 and before the hash of command 5. That breaks the rule of step 2 to keep the running stack.
  - Its weekly `restore-test` also checks the newest set, which has no tag. This test can give a false alert `restore_test`.
  - The third place is step 3 of the switch back of ADR-0103 (g). That step writes the tag of the old volume into `deploy/.env`. The exit runs only when that tag has the tag check.
  - There the exit does not create `backup` again. Step 1 of the switch back deletes the containers, and its last step creates `backup` with the key.
  - At each place, the exit writes the tag of that place. The first two places write the new tag. The third place always writes the tag of the old volume.
  - After the rollback of §7.10 step 6, the old volume runs the new tag, so the third place writes the new tag.
  - Only after a failed upgrade is the tag of the old volume the previous tag. There the new tag would start the new `backup` on the schema of a failed `migrate`.
  - A `COMPOSE_FILE` in the shell of the operator takes precedence over `deploy/.env`. The exit therefore also runs `unset COMPOSE_FILE`. Then `docker compose --profile restore config` must show `backup_manifest_key` on the three services.
  - The residual risk is the time between command 1 and command 3 of case 2. The operator runs the commands one after the other.
  - The owner puts this short time into the class of the online writer of (b). The project adds no other measure for it.
  - The retention of the previous image deletes the four files of an old set and keeps its tag file. Such a tag file has no manifest, so no reader takes it for a set (i).
  - These tag files are harmless. The operator can delete them.
  - The limit ends with the second release that holds this ADR, because its previous image has the tag check.
- (ak) The server of `pitr-check` and `pitr-volume` reads the configuration files and the catalog of a base backup. An online writer of `/wal` has the same power as the online writer of (ag). An offline writer of `/wal` can change a base backup. This ADR adds no check for it.

## Consequences

### Positive

- An offline writer of the backup disk cannot make `restore-test` or `restore-volume` run the SQL of a crafted set.
- The check covers the globals file and every entry of the dump, so it also covers the path of ADR-0095 (h).
- The manifest covers the bytes that the run wrote, not the bytes on the disk after the publish.
- The image gets no new package. The key stays out of every argument list and shell variable.

### Negative

- One more secret file, one more mount in three services, and two more operator procedures for the key.
- A lost key makes every set that it signed unusable for a restore. The offline copy of (r) is the only path.
- An online writer stays a known limit at the points of (ag) and (ah).
- Perl is a new language in `deploy/pgtools/`.

### Risks and mitigations

- Risk: the operator stores the key on the backup disk. Mitigation: the check of (p) stops each run, and the runbook check with `df` names the devices.
- Risk: the key is lost with the host. Mitigation: the offline copy and its key-id comparison of (r).
- Risk: the off-site copy leaves out the tag files. Mitigation: the runbook table of the off-site copy names the tag file.
- Risk: an image without the tag check gets the key. Mitigation: the file of (aa), `COMPOSE_FILE` in `deploy/.env` (aj), and the probe in `deploy/ci/upgrade.sh`.
- Risk: a later release changes the manifest format. Mitigation: the label of (f) and the version field of (h) change in the same change.

## Related

- Amends ADR-0066 (e), (f), (h), ADR-0067 step 1, ADR-0074 (b), (c), ADR-0095 (h), and ADR-0103 (h), (i).
- ADR-0031 (secret files), ADR-0035 (key file format), ADR-0064 (images), ADR-0076 (name list), ADR-0084 (generated column in `pre-data`), ADR-0105 (g), ADR-0151 (result files and alerts).
- `docs/architecture/reliability.md` §3.7, §7.1, §7.7, §7.8, §7.10, §7.11.
- `docs/plan/review/2026-11-13-full-repo-review.md` DEP-01, `docs/plan/review/2026-11-13-fix-plan.md` §5.
- Change `backup-manifest-hmac`, capability `operations`.
- `overview.md` §7: R31, R34, R38. `overview.md` §9: A16.
