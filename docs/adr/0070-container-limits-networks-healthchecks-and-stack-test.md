# ADR-0070: Container limits, networks, healthchecks, and the stack test

Status: Accepted. Amended by ADR-0081 (the temporary container of `pitr-check` in `container.small_cpus`). Amended by ADR-0087 (the term `shutdown.internal_drain` of the first sum rule of (v)). Amended by a note (item (u)). Amended by ADR-0094 (item (w): the trigger rule of the job `compose`). Amended by ADR-0103 (item (j): the temporary container of `restore-volume` in `container.small_cpus`).
Date: 2026-10-23
Deciders: project owner

## Context

Step P4.1 writes `deploy/compose.yaml` and the CI job that starts the stack. The documents leave these points open:

- `reliability.md` section 7.11 has no memory or CPU key for `migrate` and `backup-premigrate`, and no CPU key for `proxy`, `backup`, and `walarchive`.
- Section 7.11 has no stop grace period and no `/dev/shm` size for `db`. The image `postgres` has a default `/dev/shm` of 64 MB.
- A Compose healthcheck has a timeout, a start period, and a start interval. Section 7.11 has no row for them. The Docker default timeout of 30 s is longer than the interval of 10 s.
- All services share one network. `ssl_mode = disable` (ADR-0069) sends every row in clear text on it. Docker gives a root process the capability `NET_RAW` by default.
- Below Docker Engine 28, a routed plant host can reach an unpublished port of a container when the policy of the chain `FORWARD` is `ACCEPT`.
- `pg_isready` without `-h` checks the socket. During its init, the image `postgres` runs a temporary server on the socket only. `db` then becomes healthy before `01-roles.sh` ends.
- Docker delivers log lines in blocking mode by default. A slow log driver then blocks the writer thread of `app` and the answer of `/healthz`.
- `compose.yaml` repeats many numbers of section 7.11. Nothing checks that they stay equal.
- Section 6 has no test level that starts the whole stack.

## Decision drivers

- 24/7 operation on one host (R30). One container cannot take the memory of another.
- Least privilege of every service (R04, R34).
- Every number of `deploy/` equals section 7.11.
- CI proves the start from zero, the backup, the restore, the recovery, and the stop.

## Considered options

### Option A: Hard limits, an internal network, no capabilities, and a stack test on every push

- Pros: each service has a bound. A compromised `proxy` has no capability to attack `db`. CI finds a broken `compose.yaml` before a merge.
- Cons: more keys in section 7.11. Each push runs a longer CI job.

### Option B: Limits of section 7.11 only, one default network, and a manual stack test in the QA step

- Pros: a short Compose file and a short CI.
- Cons: the one-shot services have no limit. Every service can reach `db`. A broken file reaches the plant.

### Option C: Option A with the stack test only at night

- Pros: a push runs a shorter CI.
- Cons: a broken file can reach `main` before the night run.

## Decision

The project uses **Option A**.

Networks and capabilities:

- (a) The network `backend` has `internal: true`, the subnet `${COMPOSE_SUBNET}`, and `ip_range: ${COMPOSE_IP_RANGE}` (ADR-0065 (k), (l)). It holds the seven services: `app`, `db`, `backup`, `walarchive`, `migrate`, `backup-premigrate`, and `proxy`. ADR-0065 gives the networks `edge` and `egress` and the fixed address of `proxy`.
- (b) `db` is only on `backend`. No service except `proxy` publishes a port.
- (c) Every service gets `cap_drop: [ALL]` and `security_opt: [no-new-privileges:true]`. `proxy` gets `cap_add: [NET_BIND_SERVICE]` only.
- (d) `db` runs as `user: 999:999`. `app` and `proxy` get `read_only: true`. `proxy` mounts the volume `caddy` on `/data`, and the Caddyfile sets the global option `persist_config off`. `backup`, `backup-premigrate`, and `walarchive` get `tmpfs: /run/pgpass` (ADR-0066).
- (e) The runbook checks the host firewall. Below Docker Engine 28, a rule in the chain `DOCKER-USER` drops routed traffic to the Compose subnets.

Limits:

- (f) Each service gets `mem_limit` equal to `memswap_limit`, so the limit is hard and has no swap. Each service gets `cpus`.
- (g) The values come from section 7.11. `app` has `container.app_memory` and `container.app_cpus`, and `db` has `container.db_memory` and `container.db_cpus`.
- (h) The new row `container.oneshot_memory` gives 256 MiB each to `migrate` and `backup-premigrate`. ADR-0067 gives `container.backup_memory` 512 MiB to `backup`.
- (i) The row `container.small_memory` changes to 128 MiB each for `proxy` and `walarchive`. `backup` leaves this row.
- (j) The new row `container.small_cpus` gives 0.5 each to `proxy`, `backup`, `walarchive`, `migrate`, and `backup-premigrate`.
- (k) `db` gets `stop_grace_period` from the new row `db.stop_grace_period` (60 s) and `shm_size` from the new row `db.shm_size` (256 MiB).
- (l) `db` gets the `command:` list of section 7.6 and `max_connections` from `db.max_connections`.
- (m) The host has at least 8 GiB of memory and 4 vCPU. It runs Docker Engine 25 or later and Compose 2.24 or later, because `start_interval` needs Engine 25. The long-running limits sum to 3 328 MiB, and the two one-shot services add 512 MiB before `app` starts.

Healthchecks:

- (n) Each healthcheck uses `health.check_interval` (10 s) and `health.check_failures` (3) of section 7.11.
- (o) Each healthcheck also uses the new rows `health.check_timeout` (5 s), `health.start_period` (30 s), and `health.start_interval` (1 s).
- (p) The healthcheck of `app` is the subcommand `healthcheck` of ADR-0059. The healthcheck of `proxy` sends `GET http://app:9000/healthz` (section 7.1).
- (q) The healthcheck of `db` is `["CMD", "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "pg_isready", "-d", "recipe"]`. `db` is therefore healthy only after its init.
  - The probe uses the `pg_hba.conf` line of the name `pg_isready` (ADR-0069 (s)). No role of that name exists, so no login passes.
  - The probe ends at the password request, so `db` writes no `FATAL` line for it.
- (r) The healthcheck of `backup` is `backup-health` (ADR-0066). The healthcheck of `walarchive` is `walarchive-health` (ADR-0068).

Logs:

- (s) One logging anchor serves the seven services. It sets the driver `json-file` with `log.max_size` and `log.max_file` (section 3.5).
- (t) The anchor also sets `mode: non-blocking` and `max-buffer-size: 4m`. The new rows `log.mode` and `log.max_buffer_size` hold the values. A full buffer drops lines, which agrees with the event log of section 3.6 (best effort).

Checks of the values:

- (u) The test `recipe-server/tests/deploy_values.rs` compares each number of section 7.11 in `deploy/` with the table. It checks that each key `pg.*`, `db.max_connections`, `container.*`, and `log.*` has its value in `compose.yaml`.
  - Note: the key `log.session_ref_length` is a format (16 hex characters), not a Compose value. The test `deploy_values` does not require it in `compose.yaml`. Every other key `log.*` keeps its value in `compose.yaml`.
- (v) The same test also checks both sum rules of `reliability.md` section 7.3: `shutdown.drain + shutdown.pool_close + shutdown.runtime_stop < app.stop_grace_period` and `http.request_timeout <= shutdown.drain`.

Stack test:

- (w) The CI job `compose` runs on `ubuntu-24.04` on every push and every pull request. `ci.yml` also gets `workflow_dispatch`. The job has `timeout-minutes: 45`. Amended by ADR-0094: the job runs on `workflow_dispatch` and on a `push` to `main`, not on every push.
- (x) The job builds both images and starts the stack from zero. It requires `/readyz` with `ok` within 60 s.
- (y) Then the job runs these checks:
  - `create-admin --password-file` and a login through Caddy with the pre-session token.
  - `404` for each variant of the health paths (ADR-0065 (h)), and the redirect from HTTP to HTTPS.
  - The uid 65532 of `app` (ADR-0064).
  - A login as `postgres` over TCP from `backup` to `db`. The server refuses it with the reject line of `pg_hba.conf` (ADR-0069 (r)).
  - `backup-now`, `restore-test`, `basebackup-once`, and `pitr-check`, with the modes of a new set (ADR-0066 to ADR-0068).
  - The test scripts of `deploy/pgtools/tests/`, and a search of the scripts for `set -x` that finds nothing.
  - `docker compose stop app` with a request in flight, then the exit code 0 of `app`.
  - A failed backup healthcheck on one filesystem.
- (z) A separate CI job `musl-unwind` runs the panic tests of `recipe-web` on the target `x86_64-unknown-linux-musl`. It proves the unwind of the image of ADR-0064.
- (aa) `reliability.md` section 6 gets the level "Stack" for the CI job `compose`.

## Consequences

### Positive

- A memory leak or a large restore cannot take the memory of `db`.
- A compromised `proxy` holds only `NET_BIND_SERVICE`. It cannot open a raw socket against the traffic of `db`.
- A slow log driver cannot block `/healthz`.
- A change that breaks the stack fails CI before the merge.

### Negative

- A full log buffer drops event log lines.
- Each push runs a CI job of up to 45 minutes. A cold build of the images has no cache on a branch before the first merge.
- The host must have at least 8 GiB of memory.

### Risks and mitigations

- Risk: a routed plant host reaches `db` on an older Engine. Mitigation: the runbook checks the host firewall (e).
- Risk: the one-shot limits stop a large `_premigrate` dump. Mitigation: the CI job measures the peak, and a change of the row changes only the setting.

## Related

- ADR-0020 (seven services), ADR-0059 (healthcheck subcommand), ADR-0064 (images), ADR-0065 (networks `edge` and `egress`).
- ADR-0066 (backup disks), ADR-0067 (backup memory), ADR-0068 (WAL archive), ADR-0069 (client rules of `db`).
- `reliability.md` sections 3.5 (log delivery mode), 6 (level "Stack"), 7.1 (networks, capabilities, healthchecks), 7.8 (limits).
- `reliability.md` section 7.11: the new rows `health.check_timeout` 5 s, `health.start_period` 30 s, `health.start_interval` 1 s, `db.stop_grace_period` 60 s, `db.shm_size` 256 MiB.
- `reliability.md` section 7.11: the new rows `container.oneshot_memory` 256 MiB, `container.small_cpus` 0.5, `log.mode` `non-blocking`, `log.max_buffer_size` 4m. The changed row `container.small_memory` holds 128 MiB for `proxy` and `walarchive`.
- Baseline requirements: `operations` "Seven Compose services" and "Healthcheck and restart policy".
- The delta adds `operations` "Network isolation of the services", "Container limits", and "Stack test in CI".
- `overview.md` section 7: R04, R30, R31, R34.
