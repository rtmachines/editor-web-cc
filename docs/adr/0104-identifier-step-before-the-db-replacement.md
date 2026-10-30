# ADR-0104: Step of the identifier file before the replacement of db

Status: Accepted. Amended by ADR-0105 (item (c) step 2 and item (d): the step reads a `.partial` file without a complete segment. Item (c) step 4: `sync` before and after `ln -T`). Note: the name check and the one replacement of `db` that this ADR cites as ADR-0103 (d) and (z) are ADR-0105 (j) and (k)
Date: 2026-10-30
Deciders: project owner

## Context

ADR-0103 (s) gives one step for an existing installation. The step writes `/wal/.system-identifier` once, before `walarchive` first starts with an image that has the guard.

ADR-0103 (s) pipes the system identifier of `db` into the old `walarchive` container. `reliability.md` section 7.10 put the step into step 5, before command 1. That position is after the one replacement of `db` in step 3 command 1 (ADR-0103 (z)).

The CI run 41056889675 on commit `eba26bf` failed in the job "Upgrade an older volume". The log shows three facts:

- The replacement of `db` disconnects the old `walarchive`. `pg_receivewal` writes `replication stream was terminated before stop point`.
- The old container then restarts in a loop. Each start writes `mktemp: failed to create directory via template '/run/pgpass/run.XXXXXX': Permission denied`.
- `docker compose exec -T walarchive` then fails with `Container ... is restarting`.

The cause of the restart loop is a fault of the `recipe-pgtools` image of 0.1.0:

- The image has no directory `/run/pgpass`. At the first start, runc creates the mount point of the tmpfs with the owner root and the mode 0755.
- runc gives the tmpfs the mode of an existing mount point. After a restart of the container, uid 999 cannot write into `/run/pgpass`.
- The fault affects `walarchive` and `backup` of 0.1.0. In production, both can restart in a loop after any restart of `db` or of the host.

The old stream of the CI run also ended on a `.partial` file. The step then found no complete segment in `/wal`.

## Decision drivers

- The step must work when the old `walarchive` container restarts in a loop, or when it is gone.
- The step runs while the old cluster and its WAL stream stay unchanged.
- The step keeps its check of the identifier. It never trusts the volume name.
- The operator uses one form of the step. No branch depends on the state of the old container.

## Considered options

- Option A: run the step with `docker compose exec` in the old `walarchive` container, before the replacement of `db`. Cons: the step depends on the state of the old container. A container of 0.1.0 can already restart in a loop after a host reboot.
- Option B: run the step in a new container with `docker compose run`, before the replacement of `db`. Cons: the step needs the previous tag for one command.
- Option C: keep the position after the replacement of `db`, in step 5 of `reliability.md` 7.10. Cons: the replacement disconnects the old `walarchive`, and the old container of 0.1.0 can restart in a loop. The CI run shows both faults.

## Decision

The project uses **Option B**. This is the decision of the user of 2026-10-30.

- (a) Order. The step runs before the one replacement of `db`. That replacement is `docker compose up -d backup-premigrate` in step 3 command 1 of `reliability.md` 7.10.
  - The step runs in step 2 of `reliability.md` 7.10, after the name check of ADR-0103 (d) and before the set of the new tag.
  - The step still runs only when `/wal/.system-identifier` is missing.
- (b) Form. The step always runs in a new container: `docker compose run --rm --no-deps -T --entrypoint sh walarchive`.
  - `RECIPE_VERSION` holds the previous tag for this command. In the upgrade, `deploy/.env` still holds the previous tag, because the step runs before the set of the new tag.
  - `--no-deps` keeps `db` unchanged. `-T` gives the identifier of `db` to the container on its standard input.
  - The step never uses `docker compose exec` into the old `walarchive` container. The step does not depend on the state of that container.
  - The step writes only into `/wal`. It does not use `/run/pgpass`, so the fault of the image of 0.1.0 does not affect it.
- (c) These parts of the step stay as ADR-0103 (s) states them:
  1. The operator reads `SELECT system_identifier FROM pg_control_system()` in `db` as `postgres`.
  2. The container finds the newest complete segment in `/wal` by the segment grammar, with `LC_ALL=C`. It reads `xlp_sysid` of the long page header of the first page.
  3. A difference between the two values stops the step with one `ERROR` line. The step then writes no file.
  4. Else the container writes `/wal/.system-identifier` with `umask 077`, a temp name, and `ln -T`.
- (d) No complete segment. When `/wal` holds no complete segment, the step stops with one `ERROR` line and writes no file.
  - The operator then runs `SELECT pg_switch_wal()` in `db` as `postgres`. The operator waits for a complete segment in `/wal` and runs the step again.
  - A new complete segment arrives only while the old `walarchive` streams.
- (e) The CI job `upgrade` runs the step in the form of (b), before `compose up -d backup-premigrate`. The command gets the tag of the previous image.
  - Before the step, the job runs `SELECT pg_switch_wal()` while the old `walarchive` still streams. Then it waits for a complete segment in `/wal`.
  - The job checks the log line of the step against the state of the file before the step. A previous release with the guard already has the file, and the step then does not run.
- (f) This ADR amends ADR-0103 (s). The step pipes the identifier into the new container of (b), not into the old `walarchive` container. The step runs before the replacement of `db` of ADR-0103 (z).

Changes of other documents:

- (g) The follow-up edits of this ADR are:
  - The Status line of ADR-0103 gets "Amended by ADR-0104" with the scope. `docs/adr/README.md` gets a new index row and the decision map.
  - `reliability.md` section 7.7, the bullet of the existing installation, and section 7.10, steps 2 and 5.
  - `docs/ops/runbook.md` section "Upgrade", the segment check and its position, and the section "Messages of walarchive".
  - The OpenSpec change `restore-volume-switch`: `design.md` D8, D11, and the migration plan. Also `tasks.md`.
  - The spec of that change: the requirements "Identifier file of an existing installation" and "Position and form of the identifier step".
  - `CHANGELOG.md`, section `[Unreleased]`.

## Consequences

### Positive

- The step works when the old `walarchive` container restarts in a loop, and when it is gone.
- The step reads a WAL archive that the replacement of `db` did not change yet.
- The operator has one form of the step and no branch for the state of the old container.
- The image of this release creates `/run/pgpass` with the mode 1777 before `USER 999:999`. `walarchive` and `backup` of this release therefore start again after a restart.

### Negative

- The step needs the previous tag for one command.
- The image of 0.1.0 keeps its fault. From the replacement of `db` until step 5 command 1 of `reliability.md` 7.10, the old `walarchive` and `backup` can restart in a loop.
- In that time, no WAL reaches the archive. `db` keeps `pg.wal_keep_size`, and step 5 command 1 starts the new `walarchive`. A longer time can make a gap in the WAL stream (ADR-0099 (a)).

### Risks and mitigations

- Risk: the operator sets the new tag before the step. Mitigation: the step uses only `bash` and the tools of the image. The check of `xlp_sysid` does not depend on the tag.
- Risk: the newest complete segment is old, because the old `walarchive` restarts in a loop. Mitigation: each segment in `/wal` comes from the same cluster, so the check of (c) stays valid.
- Risk: `/wal` holds no complete segment. Mitigation: the step writes no file, and (d) gives the next action.

## Related

- Amends ADR-0103 (s) (form and position of the step of an existing installation).
- ADR-0103 (d) (name check), ADR-0103 (z) (one replacement of `db`), ADR-0068 (WAL archive).
- ADR-0083 (image of `walarchive` in the upgrade), ADR-0097 (h) (order of step 3).
- `docs/architecture/reliability.md` sections 7.7 and 7.10.
- `docs/ops/runbook.md` sections "Upgrade" and "Messages of walarchive".
- `overview.md` section 7: R31. Section 9: A35.
