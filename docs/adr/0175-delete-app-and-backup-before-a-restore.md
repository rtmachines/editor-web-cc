# ADR-0175: Delete the containers of app and backup before a restore or a recovery

Status: Accepted. Amended by ADR-0176 (items (h), (i), (j), (k): delete `app` in the major upgrade, `walarchive` in the recovery, and `backup` in the rollback)
Date: 2026-11-16
Deciders: project owner

## Context

Two procedures stop `app` and `backup` and keep their containers:

1. Step 2 of the restore order of ADR-0103 (b) runs `docker compose stop app backup`.
2. Step 1 of the recovery order of ADR-0106 (i) runs the same command.

`compose.yaml` gives `app` and `backup` the policy `restart: always` (ADR-0003). Docker starts a stopped container with this policy again when the Docker daemon starts. A reboot of the host or a restart of Docker therefore starts both services inside the window of the procedure.

- `app` then writes to the old cluster. The restored or recovered cluster does not get these writes. The lost period grows, and no message shows it.
- `backup` then runs its crontab again. `backup-once` runs at 02:00 UTC with its retention. `restore-test` runs on Sunday at 05:00 UTC and can hold the lock that `restore-volume` takes (ADR-0103 (i)).

ADR-0103 already states this fact for the switch: a switch deletes the old containers and does not only stop them. ADR-0174 (aj) applies the same rule to `app` in step 1 of the rollback with `docker compose rm -s -f app`.

The project owner chose the delete for both procedures. This ADR records that decision.

## Decision drivers

- No write of `app` reaches the old cluster after the start of a restore or a recovery (R31).
- No retention and no restore test of `backup` runs during a restore or a recovery (R31, R34).
- A reboot inside the window of a restore or a recovery starts no `app` and no `backup` on the old volume.
- Each start after the delete creates a new container from the files and the tag that fit the volume (ADR-0105 (a)).

## Considered options

### Option A: delete the two containers with `docker compose rm -s -f app backup`

- Pros: Docker starts no deleted container after a reboot. The switch of ADR-0103 (f) and the rollback of ADR-0174 (aj) use the same command form. `compose.yaml` does not change.
- Cons: each later start must create the containers with `docker compose up -d --no-deps`. `docker compose start` then fails, because no container exists.

### Option B: the policy `restart: unless-stopped` for `app` and `backup`

- Pros: one change in `compose.yaml` covers each manual stop.
- Cons: it amends ADR-0003. A service that the operator stopped for another reason then stays down after a reboot.

### Option C: `docker update --restart=no` before the stop

- Pros: the containers stay, so `docker compose start` still works.
- Cons: a later `docker compose up -d --no-deps` can keep the container and its changed policy. The service can then lose `restart: always` without a message.

## Decision

The project uses **Option A**.

- (a) Step 2 of ADR-0103 (b) becomes: the operator deletes the containers of `app` and `backup` with `docker compose rm -s -f app backup`. The reasons of that step stay unchanged. This item amends ADR-0103 (b) step 2.
- (b) Step 1 of ADR-0106 (i) becomes: the operator deletes the containers of `app` and `backup` with `docker compose rm -s -f app backup`. This item amends ADR-0106 (i) step 1.
- (c) Step 5 of the CI job `pitr-recovery` of ADR-0106 (p) deletes the two containers with the same command. This item amends ADR-0106 (p) step 5.
- (d) The CI job `restore-switch` deletes the two containers with the same command in its step `restore`.
  - `deploy/ci/restore-switch.sh` and `deploy/ci/pitr-recovery.sh` copy the runbook command.
  - A test of `deploy/ci/` compares each copy with the runbook command.
- (e) After the delete, each start of `app` or `backup` in these two procedures uses `docker compose up -d --no-deps`. No step uses `docker compose start` for them.
  - This rule covers the start after the switch and each path that ends a failed attempt before the switch.
- (f) The major upgrade runs steps 2 to 5 of the restore order (`reliability.md` §7.10 step 8). Item (a) therefore applies to the major upgrade too.
- (g) The step 1 of the switch of ADR-0103 (f) stops and deletes `app` and `backup` again. That step then finds no container of the two services.
  - On the host of the proposal, Docker Compose 5.7.1 ended `stop` and `rm -f` with the exit code 0 for a service without a container.
  - Nobody checked the lowest supported Compose version 2.24 (runbook section "Check the host"). The CI jobs `restore-switch` and `pitr-recovery` prove the result only for the Compose version of the runner.
- (h) The runbook section "After a host reboot" names the window of each procedure. In that window, `app` and `backup` can have no container.
  - The restore window starts at step 2 of the restore. The recovery window starts at step 1 of the recovery. Each window ends when the procedure starts `app` and `backup` again.
  - Steps 1 and 4 of that section cannot pass in full in the window. `proxy` cannot become `healthy`, because its healthcheck calls `http://app:9000/healthz` (`compose.yaml` line 61).
  - Step 4 runs `docker compose exec app`. The operator therefore skips step 4 in the window.
  - Sub-step 1 of the switch (step 5 of the restore) also deletes the containers of `proxy`, `db`, and `walarchive`.
  - Sub-step 8 of step 5 of the restore creates `db` again. Sub-step 7 of step 7 of the restore creates `walarchive` again.
  - Step 8 of the recovery runs step 5 of the restore. Step 10 of the recovery runs step 7 of the restore.
  - A wait for a service without a container never ends. In the window, step 1 therefore waits only for `db` and `walarchive`, and only for a service that `docker compose ps -a` lists.
  - The recovery is also the procedure for a `db` that does not start. Step 8 of the recovery deletes the container of `db` in sub-step 1 of the switch.
  - The drill check of step 8 runs before that sub-step. Until that delete, the operator skips this wait when `db` did not run at step 1 of the recovery.
  - Such a `db` cannot become `healthy`. The healthcheck `walarchive-health` passes only while a process `pg_receivewal` runs.
  - `walarchive-entry` line 154 starts `pg_receivewal` with `--no-loop`, so it ends without a running `db`.
  - Between sub-step 1 and sub-step 8 of the switch, `docker compose ps -a` lists no `db`. Step 1 therefore does not wait for `db`.
  - Sub-step 8 of the switch in step 8 of the recovery creates `db` on the new volume. That `db` can become `healthy`, so only the rule of the listed services applies.
  - The wait for `db` stays, because steps 2 and 3 of the recovery read from a running `db`. A `db` that still starts can look like a `db` that does not run.
  - The wait for `walarchive` stays, because step 4 of the recovery stops `walarchive` only after the flush check of step 3.
  - Step 4 of the recovery stops `walarchive` and does not delete it. A reboot between step 4 and step 8 of the recovery therefore starts `walarchive` again (`restart: always`).
  - After such a reboot, the operator runs step 1 of "After a host reboot" and then step 4 of the recovery again.
  - The wait comes first, because `docker compose ps -a` lists the stopped `walarchive`. A wait for that `walarchive` would never end.
  - This is the mitigation in the runbook. Item (k) keeps the full fix out of scope.
  - After the wait, the operator continues the procedure at the step where it stopped.
  - That section also names two more cases. A reboot during a fill (step 4 of the restore, step 6 of the recovery) leaves a volume that is not complete.
  - The fill refuses a volume that is not empty (`restore-volume` step 1, `check_target_empty` of `pitr-volume`).
  - The operator therefore deletes the new volume and starts again at step 3 of the restore or at step 5 of the recovery.
  - A reboot inside step 7 of the restore or step 10 of the recovery loses the shell variable `target`.
  - The operator then sets `target` to the directory of sub-step 1. If sub-step 1 made no directory, the operator runs sub-step 1 again.
  - The failed rollback restore of ADR-0174 (aj) has a similar rule in that section.

Not part of this decision:

- (i) Step 1 of the rollback of ADR-0174 (aj) stays unchanged. It runs no step 2 of the restore order. It stops `backup` before the check of the set. It deletes `app` only after the check passes.
- (j) Step 1 of the major upgrade stops `app` with `docker compose stop app`. A failed step before step 8 starts it again with `docker compose start app`. This ADR does not change these steps.
  - A reboot between step 1 and step 8 of the major upgrade therefore starts `app` again on the old volume (`restart: always`).
  - The switch at step 8 then loses the writes of `app` after the dump of the `_now` set.
  - A separate change gives the rule for that window.
- (k) Step 4 of the recovery order stops `walarchive` with `docker compose stop walarchive`. This ADR does not change that step.
  - A reboot between step 4 and step 8 of the recovery therefore starts `walarchive` again. Item (h) gives the mitigation: the operator runs step 1 of "After a host reboot" and then step 4 of the recovery again.

## Consequences

### Positive

- A reboot or a restart of Docker during a restore or a recovery starts no `app` and no `backup`.
- The lost period of a restore or a recovery ends at the step of the delete.
- The restore, the recovery, the switch, and the rollback use one command form to take `app` out of service.

### Negative

- `docker compose start app` and `docker compose start backup` fail after the delete. The runbook therefore names `docker compose up -d --no-deps` at each start.
- After a reboot in the window, `docker compose ps -a` can show no `app` and no `backup`. After sub-step 1 of the switch, it can also show no `proxy`, no `db`, and no `walarchive`. The check list of "After a host reboot" must accept that state.
- `rm` deletes the container logs of `app` and `backup` of the driver `json-file`. The runbook therefore gives an optional copy of these logs before the delete.

### Risks and mitigations

- Risk: an operator follows an old printed copy of the runbook and starts a deleted container with `start`. Mitigation: the command fails with a message and changes nothing. The current runbook gives `up -d --no-deps`.
- Risk: a Docker Compose version other than 5.7.1 ends `stop` or `rm -f` with an error for a service without a container. Mitigation: the CI jobs `restore-switch` and `pitr-recovery` run the switch after the delete.

## Related

- Amends ADR-0103 (b) step 2, ADR-0106 (i) step 1, and ADR-0106 (p) step 5.
- ADR-0003 (restart policy), ADR-0103 (f) (switch deletes the containers), ADR-0105 (a) (files and tag of a volume), ADR-0174 (aj) (delete of `app` in the rollback).
- `docs/architecture/reliability.md` §7.7, §7.10.
- `docs/ops/runbook.md` sections "Restore into production", "Recover from the WAL archive", and "After a host reboot".
- Change `restore-delete-app-backup`, capability `operations`.
