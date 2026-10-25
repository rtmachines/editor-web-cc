# ADR-0074: Names of the files on the backup disks

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 writes the scripts of `recipe-pgtools` and the adapter of the port `BackupDirectory` in `recipe-server`. ADR-0066 and `reliability.md` section 7.7 leave three names open:

1. ADR-0066 (b) and section 7.7 require "a marker file" on each disk. They give no name. The runbook, the CI job, and the scripts of `backup` and `walarchive` must use the same name.
2. ADR-0066 (h) gives the order of a run: the files, the final names, and the manifest last. It gives no name for a file before its publish. Every reader of the sets must skip such a file.
3. Section 3.7 says "A set is complete when its manifest exists". No document gives the grammar of a manifest name. The retention, the healthcheck, the restore test, and the job `alerts` must find the same sets.

Two more facts apply:

- A run of `backup-once` has no time limit. A database that does not answer holds the run.
- A publish with a hard link needs a filesystem with hard links.

## Decision drivers

- The project gives data safety priority over performance (R31). A reader never counts a set that a run did not complete.
- A failed step writes one `ERROR` line and stops (R35).
- A file name on a disk is external input. Every reader checks it against one anchored grammar (R38).
- Each name has one normative place, so the scripts, the runbook, the CI job, and `recipe-server` agree.
- No new setting and no new row of section 7.11.

## Considered options

### Option A: The names in a new ADR and in section 7.7

- Pros: every task and the runbook cite one place. A review can compare each script with the document.
- Cons: a change of a name needs a new ADR.

### Option B: The names only in the scripts and in the design of the change

- Pros: no new ADR.
- Cons: the design of a change is not normative, and it moves to the archive with the change. The adapter of `BackupDirectory`, the runbook, and the CI job then copy each name from a script. A second copy can drift.

## Decision

The project uses **Option A**.

Marker files:

- (a) Each disk has one marker file at the top of its mount.
  - The marker of the backup disk is `.backup-marker`. In the container `backup`, its path is `/backups/.backup-marker`.
  - The marker of the WAL archive disk is `.walarchive-marker` at the top of the volume `walarchive`.
  - The operator creates both marker files before the first start (ADR-0066 (b)). The CI job creates them on its loop-mounted images (ADR-0066 (c)).
  - Every delete keeps the marker files (ADR-0066 (l)).

Temp names:

- (b) A run writes each file of a new set first under the temp name `.tmp-<final name>` in the same directory, for example `.tmp-recipe_20261023_0200.dump`.
  - The run publishes a file with a hard link to its final name, through `ln -T` without `-f`. Then it deletes the temp name.
  - `ln` without `-f` fails when the final name exists. A publish therefore never replaces a file. This publish is the rename of ADR-0066 (h) step 5.
  - The run publishes the manifest last, in the same way.
  - Step 1 of ADR-0066 (h) also stops the run when a temp name of the new set exists.
  - A temp name never counts as a set, and the retention never deletes it.
  - A run that fails deletes its own temp names and the files that it published without a manifest. It deletes no file of another set.
  - The temp files of a killed run stay until the operator deletes them. The runbook gives the step.

Grammar of a set:

- (c) A set is complete when its manifest exists (section 7.7). A reader finds a manifest only by this anchored grammar of the whole file name:

  ```text
  ^recipe_[0-9]{8}_[0-9]{4}(_premigrate|_now)?\.dump\.sha256$
  ```

  - `[0-9]` stands for the ASCII digits only. A shell script sets `LC_ALL=C` for the match.
  - A name with a prefix, such as `.tmp-`, or with another suffix never matches.
  - The other three file names of the set come from the matched name (ADR-0066 (e)).
  - The rule holds for the retention of `backup-once`, for `backup-health`, for `restore-test`, and for the adapter of `BackupDirectory`.
  - A name that does not match is not a set. The retention never deletes it.

Accepted risks:

- (d) A run of `backup-once` has no time limit for a database that does not answer. The project adds no setting for it.
  - The run holds `flock`. The next runs and `backup-premigrate` therefore stop with one `ERROR` line.
  - The held run deletes nothing.
  - `backup-health` fails after `backup.max_age_alert`, and the job `alerts` raises the condition of the dump age (section 3.7).
  - A time limit needs a new row of section 7.11 and a new ADR.
- (e) A filesystem without hard links stops each run at the first publish with one `ERROR` line. No set appears.
  - `backup-health` then fails after `backup.max_age_alert`.
  - The runbook names ext4 or xfs for the second disk.

## Consequences

### Positive

- A reader never counts a half-written set, also after a killed run.
- The scripts, the runbook, the CI job, and `recipe-server` use one name for each file.
- A foreign file on the backup disk never counts as a set, and the retention never deletes it.

### Negative

- The temp files of a killed run use disk space until the operator deletes them.
- A hung database stops every backup run until the operator acts.
- The second disk needs a filesystem with hard links.

### Risks and mitigations

- Risk: a reader uses a looser pattern and counts a temp name. Mitigation: the test scripts of `deploy/pgtools/tests/` and the tests of the adapter of `BackupDirectory` put a temp manifest into the directory.
- Risk: the second drive of a Windows development host (ADR-0066 (c)) has no hard links in the container. Mitigation: the QA of step 6 runs `backup-now` on that host.

## Related

- Amends ADR-0066 (b) (names of the marker files) and (h) (temp names and the publish).
- Amends ADR-0066 (f) and (k) (grammar of a set) and (l) (deletes of a failed run).
- ADR-0067 (restore test), ADR-0068 (a) (marker check of `walarchive-entry`), ADR-0070 (CI job), ADR-0073 (c) (unreadable backup directory)
- `docs/architecture/reliability.md` sections 3.7 and 7.7. Section 7.11 keeps `backup.max_age_alert` 26 h. This ADR adds no row.
- Baseline requirement: `operations` "Nightly dump". The delta changes `operations` "Disks of the backup and the WAL archive".
- `overview.md` section 7: R31, R35, R38. Section 9: A16, A35
