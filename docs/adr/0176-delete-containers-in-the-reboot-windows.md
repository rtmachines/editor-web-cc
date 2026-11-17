# ADR-0176: Delete the containers of app, walarchive, and backup in the major upgrade, the recovery, and the rollback

Status: Accepted
Date: 2026-11-16
Deciders: project owner

## Context

ADR-0175 deletes the containers of `app` and `backup` in a restore and in a recovery. Its items (i), (j), and (k) keep three stop steps:

1. Step 1 of the runbook section "Major upgrade" reads "Run `docker compose stop app`. The next dump then holds every change."
   The text before step 5 of that section reads "If a step before step 8 fails, change into the directory of `<old tag>`. Then run `docker compose start app`" (ADR-0175 (j)).
2. Step 4 of "Recover from the WAL archive" reads "Stop `walarchive`. The `.partial` file then stays stable" and runs `docker compose stop walarchive` (ADR-0106 (i) step 4, ADR-0175 (k)).
3. Step 1 of "Rollback" reads "Stop `backup`. Run the check of the set below, which is command 1. Only after it passes, delete `app`." It runs `docker compose stop backup` (ADR-0174 (aj), ADR-0175 (i)).

`compose.yaml` gives `app`, `backup`, and `walarchive` the policy `restart: always` (ADR-0003). Docker starts a stopped container with this policy again when the Docker daemon starts. A reboot of the host or a restart of Docker therefore starts the stopped service inside the window of the procedure.

- Major upgrade: `app` writes to the old cluster after the dump of the `_now` set of step 2. The switch at step 8 loses these writes, and no message shows them.
- Recovery: `walarchive` streams into `/wal` again, so the `.partial` file does not stay stable. ADR-0175 (h) gives a mitigation: the operator runs step 4 of the recovery again. That mitigation needs an operator who knows of the restart. A restart of the Docker daemon alone, for example by a package update, can go unnoticed.
- Rollback: `backup` runs its crontab again on the old volume until sub-step 1 of the switch deletes it. `backup-once` runs at 02:00 UTC with its retention. `restore-test` runs on Sunday at 05:00 UTC and can hold the lock that `restore-volume` takes (ADR-0103 (i)). Command 1 or the fill then stops with a fault that the set does not cause.

The section "After a host reboot" also names no window of the major upgrade and no window of the rollback. In both windows, `app` can have no container, so steps 1 and 4 of that section cannot pass.

## Decision drivers

- No write of `app` reaches the old cluster after the dump of a major upgrade (R31).
- The `.partial` file stays stable from step 4 of the recovery without an action of the operator (R31, A35).
- No retention and no restore test of `backup` runs during a rollback restore (R31, R34).
- The procedures use one command form to take a service with `restart: always` out of service (ADR-0175).

## Considered options

### Option A: delete each container with `docker compose rm -s -f <service>`

- Pros: Docker starts no deleted container after a reboot. ADR-0175, the switch of ADR-0103 (f), and the rollback of ADR-0174 (aj) use the same command form. `compose.yaml` does not change.
- Cons: each later start must create the container with `docker compose up -d --no-deps`. `rm` deletes the container log.

### Option B: the policy `restart: unless-stopped` for `app`, `backup`, and `walarchive`

- Pros: one change in `compose.yaml` covers each manual stop.
- Cons: it amends ADR-0003. A service that the operator stopped for another reason then stays down after a reboot. ADR-0175 rejected this option for the same reason.

### Option C: keep the stop steps and add a rule to "After a host reboot"

- Pros: no command of a procedure changes.
- Cons: the rule needs an operator who knows of the restart. A cron run of `backup` or a write of `app` can come before the action of the operator.

## Decision

The project uses **Option A**.

Major upgrade:

- (a) Step 1 of the major upgrade becomes: the operator deletes the container of `app` with `docker compose rm -s -f app`. The reason "The next dump then holds every change" stays. This item amends ADR-0175 (j).
- (b) When a step before step 8 of the major upgrade fails, the operator changes into the directory of `<old tag>` and runs `docker compose up -d --no-deps app`. No step of the major upgrade uses `docker compose start app`.
- (c) Step 8 of the major upgrade runs step 2 of "Restore into production", `docker compose rm -s -f app backup`. That command then finds no container of `app`.
  - ADR-0175 (g) reports only `stop` and `rm -f` for a service without a container. It does not report `rm -s -f`.
  - A local probe on Docker Compose 5.7.1 ran `rm -s -f` for a service without a container. It printed `No stopped containers` and exited with the code 0.
  - No CI job runs `rm -s -f` for a service without a container, and `ci.yml` pins no Compose version. The result on the runner is therefore not proven.

Recovery:

- (d) Step 4 of the recovery order becomes: the operator deletes the container of `walarchive` with `docker compose rm -s -f walarchive`. The reason "The `.partial` file then stays stable" stays. This item amends ADR-0106 (i) step 4 and ADR-0175 (k).
- (e) Step 5 of the CI job `pitr-recovery` deletes `walarchive` with the same command after the `SIGKILL` of `db`. This item amends ADR-0106 (p) step 5.
  - `deploy/ci/pitr-recovery.sh` copies the runbook command. A test of `deploy/ci/` compares the copy with the runbook command.
- (f) After the delete, each start of `walarchive` in the recovery uses `docker compose up -d --no-deps walarchive`.
  - This rule covers the end of a failed attempt. Step 7 of the recovery reads "Start `walarchive`, if you stopped it in step 4" now.
  - When `<recovery tag>` differs from `<tag>`, the operator runs steps 6 to 12 in the directory `deploy/` of `<recovery tag>` (ADR-0105 (a)).
  - `docker compose start` kept the old container and its image. `up` creates a new container from the configuration of the current directory.
  - To end a failed attempt, the operator therefore first changes into the directory `deploy/` of `<tag>` and sets `RECIPE_VERSION` in `.env` to `<tag>`.
  - Only then does the operator create `walarchive` again. The new container then runs the image of the tag of the schema of `db` (ADR-0083 (a)).
  - Step 8 of the recovery refuses a drill volume before sub-step 1 of the switch. That refusal is also an exit after the delete. It does not switch and ends the attempt in the order of step 7.
  - Sub-step 7 of step 7 of "Restore into production" already runs `docker compose up -d --no-deps walarchive`. Step 10 of the recovery runs that step.
  - Sub-step 1 of the switch (step 8 of the recovery) then finds no container of `walarchive`. ADR-0175 (g) applies.

Section "After a host reboot" (this item amends ADR-0175 (h)):

- (g) The section names two more windows. In each window, `app` can have no container:
  - The window of the major upgrade starts at step 1 and ends when the procedure starts `app` again.
  - The window of the rollback starts at step 1 of the restore of the `_premigrate` set in "Rollback". That step deletes `backup`. The window ends when the procedure starts `app` and `backup` again. In it, `app` has no container from the pass of command 1. With item (i), `backup` has no container from step 1.
  - The rules of ADR-0175 (h) for the window of the restore apply to these windows too. `proxy` can stay unhealthy while `app` has no container, and the operator skips step 4 of the section.
- (h) From step 4 of the recovery, `docker compose ps -a` lists no `walarchive` until step 10, or the end of a failed attempt, creates it again.
  - Step 1 of the section waits only for a service that `docker compose ps -a` lists. That rule therefore stays correct without a change.
  - The section drops the case "If the reboot came between step 4 and step 8 of the recovery, `walarchive` runs again. Run step 4 of the recovery again." Two cases stay.
  - The wait for `walarchive` stays before step 4 of the recovery, because the flush check of step 3 needs a running `walarchive`.
  - The skip rule for a `db` that did not run at step 1 of the recovery stays unchanged. Step 8 of the recovery still deletes the container of `db` in sub-step 1 of the switch.

Rollback:

- (i) Step 1 of the rollback deletes the container of `backup` with `docker compose rm -s -f backup` before command 1. It deletes `app` only after command 1 passes, as before. This item amends ADR-0174 (aj) and ADR-0175 (i).
  - Each later start of `backup` in the rollback already uses `docker compose up -d --no-deps backup`, with `--force-recreate` in the plain rollback.
  - The function `check_rollback_runbook` of `deploy/ci/upgrade.sh` copies the command. A test compares the copy with the runbook command.

Logs:

- (j) Before each delete of (a), (d), and (i), the runbook gives an optional copy of the log of the container: `docker compose logs --no-color <service> > <file>`. The model is the log copy of ADR-0175.

Not part of this decision:

- (k) Step 2 of "A gap in the WAL stream" stops `walarchive` with `docker compose stop walarchive` (ADR-0099 (a) step 2). This ADR does not change that step. A reboot between step 2 and step 8 of that section starts `walarchive` again.
- (l) The tests of `deploy/ci/` get more hardening in a separate change.

## Consequences

### Positive

- A reboot or a restart of Docker starts no `app` inside the window of a major upgrade. The lost period of a major upgrade ends at step 1.
- The `.partial` file of the recovery stays stable from step 4 without an action of the operator.
- With item (i), a reboot during a rollback restore starts no `backup` on the old volume.
- The section "After a host reboot" names each window in which `app` can have no container.

### Negative

- `docker compose start app` in the major upgrade and `docker compose start walarchive` in the recovery fail after the delete. The runbook therefore names `docker compose up -d --no-deps` at each start.
- `rm` deletes the container logs of the driver `json-file`. Item (j) gives an optional copy.
- After a reboot in the window of the recovery, `docker compose ps -a` lists no `walarchive`. This holds from step 4 until step 10, or the end of a failed attempt, creates it again.

### Risks and mitigations

- Risk: a Docker Compose version other than 5.7.1 ends `stop`, `rm -f`, or `rm -s -f` with an error for a service without a container. Mitigation: the CI job `pitr-recovery` runs the `stop` and `rm -f` of the switch after the delete of `walarchive`. No CI job runs `rm -s -f` for such a service, and no CI job covers the major upgrade.
- Risk: an operator follows an old printed copy of the runbook and starts a deleted container with `start`. Mitigation: the command fails with a message and changes nothing.

## Related

- Amends ADR-0175 (h), (i), (j), (k), ADR-0106 (i) step 4, and ADR-0106 (p) step 5. With item (i), it also amends ADR-0174 (aj).
- ADR-0003 (restart policy), ADR-0083 (a) (image of the tag of the schema), ADR-0099 (a) (gap in the WAL stream).
- ADR-0103 (f) (switch deletes the containers), ADR-0105 (a) (tag of a data volume), ADR-0105 (c), (e) (directory of the old release).
- `docs/architecture/reliability.md` §7.7, §7.10.
- `docs/ops/runbook.md` sections "Major upgrade", "Recover from the WAL archive", "Rollback", and "After a host reboot".
- Change `restore-reboot-hazards`, capability `operations`.
