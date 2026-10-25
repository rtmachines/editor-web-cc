# ADR-0066: Backup disks, file sets, and schedule

Status: Accepted. Amended by ADR-0074 (names of the files on the backup disks)
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 writes the image `recipe-pgtools` and the scripts of the services `backup`, `backup-premigrate`, and `walarchive`. The documents leave these points open:

- `reliability.md` section 7.7 requires a backup path that is not on the filesystem of `pgdata`. On a default Docker host, every named volume is on the root filesystem. A plain named volume `backups` therefore always fails the healthcheck.
- A bind mount of a missing disk gives an empty directory on the root disk. The scripts then write the backup to the wrong disk.
- Section 7.7 says "Retention is 30 daily dumps". Section 7.11 and ADR-0020 say "30 days". No document says if the sets `_premigrate` and `_now` count.
- Section 7.7 says "Each dump has a manifest `<name>.sha256`". No document gives the name or the files that the manifest covers.
- The restore test compares the row counts "with the source" (section 7.7). The source changes while the dump runs.
- Section 7.1 says "with cron". The image `postgres` has no `cron`, and Debian `cron` needs root to change the user.
- A dump holds the Argon2id hashes and the audit log. No document gives the modes of the files or of the directories.

## Decision drivers

- Data safety over performance (R31). A backup is on a second disk, or the healthcheck fails (A35).
- A failed run deletes no good backup.
- The rollback of `reliability.md` section 7.10 step 6 always has a `_premigrate` set.
- No log line and no argument of a process holds a password (`reliability.md` section 3.4, ADR-0031).
- The services `backup` and `walarchive` run as uid 999 and not as root.

## Considered options

### Option A: Bind-backed named volumes with a marker file, supercronic, and retention by age

- Pros: the volume names of section 7.8 stay. Compose stops when the host path does not exist. The marker file stops a write to an empty mount point. supercronic runs as the container user and writes to stdout.
- Cons: the image holds a third-party binary. The operator creates the marker file on each disk.

### Option B: Plain named volumes, Debian `cron` as root, and retention by count

- Pros: no third-party binary.
- Cons: the healthcheck fails on every default host. `cron` runs as root. A count of 30 sets can delete the last `_premigrate` set.

### Option C: Direct bind mounts and a shell loop with `sleep`

- Pros: no scheduler binary.
- Cons: Compose creates a missing host path by default. A loop does not match the schedule rule "with cron" of section 7.1, and it drifts after a slow run.

## Decision

The project uses **Option A**.

Disks:

- (a) `backups` and `walarchive` are named volumes with `driver: local` and `driver_opts: {type: none, o: bind, device: ...}`. The paths come from `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR` in `.env`. The file `.env` holds no password, so section 7.1 "There is no env-file option" stays true.
- (b) The operator creates a marker file on each disk. Each script of `backup` and `walarchive` checks the marker first. Without it, the script writes one `ERROR` line and writes no file.
- (c) On a development host with Windows, each path is a directory on a second drive. CI uses a loop-mounted ext4 image for each path (ADR-0070).
- (d) The script `backup-health` checks three things. `stat -c %d` gives a filesystem of `/backups` that differs from the filesystem of `pgdata`. The marker exists. A complete set is younger than `backup.max_age_alert`. A `_premigrate` set counts, so `backup` is healthy after the first start.

File sets:

- (e) One run writes one set. The names are `recipe_<YYYYMMDD_HHMM><suffix>.dump`, `globals_<YYYYMMDD_HHMM><suffix>.sql`, `recipe_<YYYYMMDD_HHMM><suffix>.counts`, and the manifest `recipe_<YYYYMMDD_HHMM><suffix>.dump.sha256`. `<suffix>` is empty, `_premigrate`, or `_now`.
- (f) The manifest covers the dump, the globals file, and the counts file. The script writes it last. A set without a manifest is not complete.
- (g) The counts file holds the row count of each table in the snapshot of the dump.
  - A psql session opens `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY` and exports the snapshot.
  - It counts the rows in the same transaction, and keeps it open until `pg_dump --snapshot` ends.
  - The counts file holds no row for `session` and `login_attempt`, because the dump excludes their data.
  - The restore test of ADR-0067 compares with this file.
- (h) The script `backup-once <suffix>` runs these steps:
  1. It checks the marker and takes `flock`, so two runs cannot write at the same time. When a file with a name of the new set exists, it writes one `ERROR` line and stops. It then writes no file and deletes no file.
  2. It writes a pgpass file in the tmpfs `/run/pgpass` from `/run/secrets/backup_password`. It applies the one-line rule of ADR-0031.
  3. It exports the snapshot and writes the counts file.
  4. It runs `pg_dump --snapshot` and `pg_dumpall --globals-only --no-role-passwords -l recipe` (ADR-0033).
  5. It runs `sync` and renames the files to their final names. A rename never replaces a file. Then it writes the manifest.
  6. It runs the retention of (k).
- (i) The daily run has no suffix. `backup-premigrate` uses `_premigrate`. The command `backup-now` runs in the running container through `docker compose exec backup backup-now` and uses `_now`.

Schedule:

- (j) supercronic v0.2.51 runs the schedule as uid 999 with `CRON_TZ=UTC`, under `init: true`. The image checks an own SHA-256 of the binary before `chmod +x`. The dump runs at `backup.time` (02:00 UTC). ADR-0067 and ADR-0068 add their own schedule lines.

Retention:

- (k) After a successful new set, the script deletes every set older than `backup.retention` (30 days). The rule covers the daily sets, `_premigrate`, and `_now`. The newest daily set and the newest `_premigrate` set always stay.
- (l) A failed run deletes nothing. A delete uses `rm -- "${path:?}"` after a check of the path prefix. The marker file and the scratch directory of ADR-0067 always stay.

Rules of every pgtools script:

- (m) `umask 077` is the first command. Files get `0600`, and directories get `0700`. Only the top directory of `backups` gets `0755`, so uid 65532 in `app` can list the names for the job `alerts`. The top directory of `walarchive` stays `0700`, because `statvfs` needs no read right.
- (n) A script writes one JSON object per line to stdout.
- (o) A script escapes `\` and `:` in the pgpass file, as libpq requires. It keeps each password out of the argument list and out of shell variables. No script enables `xtrace`.

## Consequences

### Positive

- A backup cannot land on the root disk without a failed healthcheck or an `ERROR` line.
- A rollback after an upgrade always finds the newest `_premigrate` set, also after 30 days.
- The restore test compares with counts of the same snapshot as the dump.
- A host user and the mount in `app` see the names of the sets, but not their content.

### Negative

- The image `recipe-pgtools` holds supercronic, a binary outside Debian. The project pins its hash.
- The operator prepares two directories and two marker files before the first start.
- `stat -c %d` compares filesystems, not physical disks. Two partitions of one disk pass the check.

### Risks and mitigations

- Risk: a defect in the retention deletes good sets. Mitigation: the test script of `deploy/pgtools/tests/` builds old sets with `touch -d` and checks (k) and (l) in CI.
- Risk: a changed byte in a set stays unseen. Mitigation: the restore test checks the manifest first (ADR-0067).

## Related

- Amends ADR-0020 (the retention of 30 days covers every set, and the newest daily and `_premigrate` sets stay).
- ADR-0031 (secret files), ADR-0033 (globals dump), ADR-0067 (restore test), ADR-0068 (WAL archive), ADR-0070 (CI job and limits).
- `reliability.md` section 7.1 (the service `backup` with supercronic), 7.7 (file set, manifest, retention, modes), 7.8 (the volumes `backups` and `walarchive`). Section 7.11 keeps `backup.time` 02:00 UTC, `backup.retention` 30 days, and `backup.max_age_alert` 26 h. This ADR adds no row.
- Baseline requirements: `operations` "Nightly dump" and "Dump before a migration". The delta adds `operations` "Disks of the backup and the WAL archive" and "Dump on demand".
- `overview.md` section 7: R31. Section 9: A16, A35.
