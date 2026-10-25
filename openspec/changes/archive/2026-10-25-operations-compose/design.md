# Design

## Context

See `proposal.md` (Why). The state at `eeca681`:

- `recipe-server` runs `migrate` and `create-admin`. `serve` returns an error (`crates/recipe-server/src/main.rs`). The crate has a binary target only.
- `recipe-db::Database` gives four ports: unit of work, security event sink, session store, login attempt store. It has one pool and no `close`.
- `recipe-web` has the router, `ReadOnlyMode`, and the JSON error answer of `/export/v1/`. It has no panic layer, no timeout layer, and no health route.
- `recipe-app` has no port `MaintenanceStore`, no job, no production clock, and no `E-EXP-002` counter.
- `migrations/` holds `0001` to `0005`. `app_setting`, `security_event_purge()`, and the grants of the jobs exist.
- `deploy/` holds `db/initdb/01-roles.sh` and two placeholders.

Normative sources:

- `reliability.md` sections 1.1, 1.2, 2, 3, 5, 6, 7, 8.
- `data-model.md` sections 3.6 to 3.8, 3.15 to 3.17, 4.9, 4.12, 5, 6 (row `cleanup`), 8.
- `domain-model.md` section 5 (commit paths) and section 8.
- `api.md` sections 1, 2.10, 3.2, 5.1, 5.3, 7.1, 9.
- `overview.md` sections 3, 5, 7, 9. `design-system.md` sections 8 to 10.
- ADR-0014, ADR-0015, ADR-0020, ADR-0022, ADR-0025, ADR-0029, ADR-0031, ADR-0033, ADR-0034, ADR-0035, ADR-0053, ADR-0056, and the new ADR-0057 to ADR-0087 of D2.

The research of step 1 is in `target/p41-agents/R1.md` to `R4.md`. The check of the recommendations is `VQ.md`. The plan review is in `PR1.md` to `PR4.md` and `plan-review-triage.md`. These files are not normative. The binding decisions of the archived designs of steps P2.1 to P3.5 stay valid.

## Goals / Non-Goals

Goals:

- A `serve` subcommand that runs the router on PostgreSQL with the start order, the drain, and the exit codes of D4.
- Panic containment, timeouts, health endpoints, supervised jobs, read-only mode, and the startup settings comparison.
- A `deploy/` tree that starts the seven services from zero, takes backups, runs the restore test, and archives WAL.
- A CI job that proves the stack, and a runbook for the operator.

Non-Goals:

- The items of `proposal.md` (Out of scope).
- A new error code. `E-WEB-009`, `E-WEB-010`, and `E-AUD-002` cover the new failures. `WebhookFault`, `DiskFault`, and `BackupFault` are reason types without a code (ADR-0025, ADR-0073 (b)).
- A new capability.
- A change of a numeric value of `reliability.md` 7.11 that exists today. Two exceptions come from decisions of the user: the memory limit of `backup` (128 MiB to 512 MiB) and the default of `http.trusted_proxies`. The change also adds rows and changes the text of some rows (D17).

## Decisions

### D1. Answers of the user (2026-10-23)

Every answer took the recommended option. A verifier (T1-O, read only, `VQ.md`) checked the recommendations before the questions.

| Topic | Answer | Record |
|---|---|---|
| Panic layer | `CatchPanicLayer::custom` directly inside `render_errors`. A second instance directly inside the layer `request_id` as a backstop. One panic gives two `ERROR` lines | ADR-0057 |
| Server and timeouts | Own accept loop with hyper `http1`, `header_read_timeout`, `GracefulShutdown`, `JoinSet`. Typed request timeout (`AppError::Timeout { stage: Request }`). `RequestBodyDeadlineLayer`. A late body gives `E-WEB-009` | ADR-0058 |
| Health | Subcommand `healthcheck`, std only, shared with the watchdog, budget `watchdog.probe_timeout` (3 s). `not ready` has status 503. Ready when every migration of the binary is applied with its checksum and no failed row exists | ADR-0059 |
| Health router layers | The health router gets `request_id`, the security headers, and the request span. Its finish line has the level `DEBUG`. No `client_address`, no session, no CSRF. MODIFIED "Event log format" and "Log level by HTTP status" | ADR-0059 |
| Jobs | Ports and one iteration of each job in `recipe-app`. SQL in `recipe-db`. Supervisor, schedules, watchdog, panic hook, `statvfs`, and the wiring in `recipe-server`. Internal operations without a permission | ADR-0060 |
| Library target | `recipe-server` gets `src/lib.rs` with every module and a thin `main.rs`. The crate is not published. The workspace table of CLAUDE.md and `overview.md` 5 say "bin + lib" | ADR-0060 |
| Start order | D4. `SettingChanged` at startup is written also in read-only mode | ADR-0061 |
| Configuration | The operator list of D7. `RECIPE_*` values in `compose.yaml`. Secrets only as files. `.env` only for Compose interpolation | ADR-0062 |
| Webhook | Secret file `alerts.webhook_url_file`. Fingerprint `sha256:<hex>` in `app_setting` and the audit. HTTPS only, ring, webpki roots or the CA file. One `POST` per active condition and run | ADR-0063 |
| Images | Distroless `static-debian13:nonroot`, musl, `rust:1.99.1-slim-trixie` by digest, cargo-chef. `docker save` and `docker load` with version tags | ADR-0064 |
| TLS of the proxy | Plant PKI files as secrets. `tls internal` in dev and CI. Caddy to `app` is HTTP. The default trusted proxy is the `/32` of `proxy` | ADR-0065 |
| Networks and capabilities | Network `edge` for `proxy` and the published ports. Network `egress` for `app` only, for the webhook (answer O1). Network `backend` with `internal: true` for `app`, `db`, `backup`, `walarchive`, `migrate`, `backup-premigrate`, and `proxy`. `cap_drop: [ALL]` everywhere. Caddy gets `NET_BIND_SERVICE` only. `no-new-privileges` everywhere | ADR-0065, ADR-0070 |
| Database client rules | `01-roles.sh` replaces the catch-all line of the image. The roles `recipe_*` connect only from the `backend` subnet. `postgres` connects only through the socket. Runbook: host firewall check | ADR-0069, ADR-0070 |
| Backup | Bind-backed named volumes on the second disk with a marker file. supercronic. One set with one manifest. Retention 30 days by age, the newest daily set and the newest `_premigrate` set stay | ADR-0066 |
| Restore test | In `backup`, a private scratch cluster, counts of the dump snapshot, chain check, random role passwords and a login per role | ADR-0067 |
| WAL archive | `pg_receivewal --synchronous` without a slot. Weekly `pg_basebackup --wal-method=fetch` in plain format, `pg_verifybackup`, 4 base backups, `pg_archivecleanup`. `restore_command` with the `.partial` fallback | ADR-0068 |
| Database access | `ssl_mode = disable`. Options without the pgpass file. `serve`, `migrate`, and `create-admin` refuse to start when any `PG*` variable is set. `pg_read_all_stats` moves from `recipe_app` to `audit_owner` behind a function. Index on `security_event.occurred_at`. `migrate.connect_timeout` | ADR-0069 |
| Refusal of `PG*` | One `ERROR` line with the name of the variable and not its value. Exit code 1. sqlx 0.8.6 cannot ignore `PGOPTIONS`, and `remove_var` is unsafe in edition 2024 | ADR-0069 |
| Limits and stack test | Hard memory limits without swap, the keys of D17, host at least 8 GiB and 4 vCPU, Docker Engine 25, Compose 2.24. CI job `compose` on `ubuntu-24.04` on `workflow_dispatch` and on a push to `main` | ADR-0070, ADR-0094 |
| Read-only banner | A notice with the icon `alert-warning` at the start of `main` | ADR-0071 |
| Catch-up | One extra audit chain check `audit.catchup_delay` (10 min) after a start when the last check is missing or older than `audit.catchup_age` (24 h). No alert for a missing dump or check during `backup.max_age_alert` and `audit.max_check_age` (26 h) after the first start | ADR-0060 |
| Deferred items | In P4.1: S7, Q9, C7, C6 of the reviews of P2.4 and P2.5, the key file check, the sink test of `audit_chain_broken`. To P4.2: R2-5, R2-3 | D18 |

Decisions of the orchestrator, recorded in the named ADR:

- The plan text "pierwszy admin z env" of the operator prompt yields to `reliability.md` 5. CI runs `create-admin --password-file` (ADR-0062).
- `RUST_LOG` through `EnvFilter`. A bad directive stops the start with one `ERROR` line from a fallback logger (ADR-0061, step 1).
- The webhook body is `{"alert":"<name>"}` with the names of ADR-0075 (ADR-0063). A `WARN` line of `low_disk` also holds the field `volume`.
- The internal router has no `client_address` layer, so the healthcheck of `proxy` works without `X-Forwarded-For` (ADR-0059).
- The scripts of `backup` and `walarchive` write one JSON object per line (ADR-0066).
- Caddy sets `protocols h1 h2`, because UDP 443 is not published (ADR-0065).
- Each run of a job has its own `request_id`. Only the rows of the run hold it (ADR-0060).
- `http.public_port` and `http.internal_port` become configuration keys, so a test can use free ports (ADR-0062).
- The ADR plan stays. Each ADR holds lettered items of one topic, as ADR-0029 and ADR-0035 do.
- The webhook URL holds a token of at least 128 random bits. This is a runbook rule without a code check (ADR-0063).
- Task 6.5 merges into task 6.2. The wave order follows `PR1.md` question 6 where it keeps the rules of D16.
- `.gitignore` gets `/deploy/.env` and `/deploy/secrets/`. The file is outside the commit sets, so the orchestrator asks the user at the commit.
- The orchestrator asks the user about the free space of `C:` before the first image build.

### D2. ADR plan and document changes

Each ADR copies the shape of an existing ADR and holds lettered items of one topic. Task 1.1 and task 1.2 write them. Tasks 1.3 and 1.4 change the documents in the same change.

| ADR | Title | Documents that change | Amends |
|---|---|---|---|
| 0057 | Panic containment layers | `reliability.md` 2.4 rules 3 and 6. `api.md` 5.1 to 5.3 (the backstop exception). `.claude/rules/web.md` (the exception line). `.claude/rules/rust.md` (count of places) | ADR-0020, ADR-0053 (a), ADR-0036 (p), ADR-0037 (z), ADR-0047 (b), ADR-0049 (k) |
| 0058 | HTTP serving loop and request timeouts | `reliability.md` 1.1 (row of the timeout mapping), 2.4 rule 6 (the `JoinSet`), 7.3 (the sum rule of the stop), 7.4, 7.11. `.claude/rules/rust.md` (count of places) | none |
| 0059 | Health probe subcommand, readiness rule, and health router layers | `overview.md` 5, CLAUDE.md (workspace table), `api.md` 2.10, `api.md` 9 (headers of the health router), `reliability.md` 3.3 (finish line `DEBUG`), 7.1, 7.2, 7.11 | ADR-0020 |
| 0060 | Supervised jobs, their ports, the catch-up, and the library target | `domain-model.md` 5 (commit paths, ports). `overview.md` 5 (dependencies, "bin + lib") and the list of internal operations. CLAUDE.md (workspace table). `api.md` 3.2 (internal operations). `reliability.md` 1.1 (`WebhookFault`, one `WARN` line of a handled fault). `reliability.md` 2.4 (attempt purge interval), 3.2 (`request_id` of a job run), 3.3, 3.7. `reliability.md` 6 (row "Permission matrix"), 7.9, 7.11, 8. `data-model.md` 4.9 (the catch-up) | none |
| 0061 | Start order of serve | `reliability.md` 3.1 (`RUST_LOG`), 7.8, 7.12, `overview.md` A24, `domain-model.md` section 8 (copy of A24) | ADR-0020 |
| 0062 | Configuration keys of serve, their delivery, and the secret files | `reliability.md` 7.1, 7.11 (the column "Used by", the sentence on the configuration file), 7.12 | ADR-0022 (item (c)) |
| 0063 | Webhook of the alerts | `reliability.md` 1.1 (`WebhookFault`), 3.4, 3.7, 7.11, 7.12. `data-model.md` 3.17 (the derived key and its fingerprint) | ADR-0022 |
| 0064 | Images and their distribution | `reliability.md` 7.1, 7.10 (the sentence "Until then, this section is the runbook") | none |
| 0065 | TLS of the proxy and the default trusted proxy | `overview.md` A18, `api.md` 7.1, `reliability.md` 7.1, 7.11 | ADR-0020 |
| 0066 | Backup disks, file sets, and schedule | `reliability.md` 7.1, 7.7, 7.8, 7.11 | ADR-0020 |
| 0067 | Restore test in the backup service | `reliability.md` 7.7, 7.11, `overview.md` 3 (`backup` with the restore test) | none |
| 0068 | WAL archive and base backups | `reliability.md` 7.6, 7.7, 7.11 | none |
| 0069 | Hardening of the database access | `data-model.md` 3.8 (prose only), 5.1 (the purposes of `SET ROLE audit_owner`), 5.3 (routine grants), a new 5.5 (DDL and grant of `0006`), 8. `reliability.md` 3.7, 5, 7.7 (the source of the replication condition), 7.11 | ADR-0020, ADR-0034 |
| 0070 | Container limits, networks, healthchecks, and the stack test | `reliability.md` 3.5 (log delivery mode), 6, 7.1, 7.8, 7.11 | none |
| 0071 | Read-only banner | `design-system.md` 10.1, `reliability.md` 7.8 | ADR-0036 (i) |
| 0072 | Export routes outside the request timeout | `reliability.md` 7.4 | ADR-0058 (g), (h) |
| 0073 | Read faults of the jobs and the audit chain alert | `reliability.md` 1.1, 1.2 (`E-WEB-006`), 2.4, 3.7, 7.8, 8. `data-model.md` 3.8, 3.17, 4.9. `overview.md` A24, `domain-model.md` section 8 (copy of A24) | ADR-0020, ADR-0060 (aa), ADR-0061 (b), ADR-0071 (d) |
| 0074 | Names of the files on the backup disks | `reliability.md` 3.7, 7.7 | ADR-0066 (b), (f), (h), (k), (l) |
| 0075 | Names of the alert conditions | `reliability.md` 3.7 (column "Name", the field `volume` of `low_disk`, the webhook body) | ADR-0063 (l), ADR-0073 (f) |
| 0076 | Name list of the restore test | `reliability.md` 7.7 (steps 2, 4, 5) | ADR-0067 (i), steps 2 and 4 |
| 0077 | Marker entry of the recovery check, WAL cutoff, and scratch constants | `reliability.md` 7.7 (layout, cutoff, recovery check, runbook step), 7.11 | ADR-0068 (j), (m) |
| 0078 | Trigger state and refused kinds of the restore test | `reliability.md` 7.7 (steps 2, 5) | ADR-0076 (a), (b) |
| 0079 | Row of the startup settings comparison | `data-model.md` 6 (row `CompareSettings`), `reliability.md` 7.12 | none |
| 0080 | Blank RUST_LOG and the exit text of main | `reliability.md` 2.1, 3.1, 3.3 | ADR-0061 (c) |
| 0081 | CPU limit of the recovery check | `reliability.md` 7.7, 7.8, 7.11 (row `container.small_cpus`) | ADR-0068 (o), ADR-0070 (j) |
| 0082 | Upgrade and rollback without the dependencies of app | `reliability.md` 7.10 (steps 4 and 6) | ADR-0020 |
| 0083 | Image of `backup` and `walarchive` in the upgrade and the rollback | `reliability.md` 7.7 (the restore test in steps 5 and 6, the image of the schema), 7.10 (steps 5 and 6) | ADR-0020, ADR-0067 (a), (j), ADR-0082 (b) |
| 0084 | Code of a generated column in the restore of `pre-data` | `reliability.md` 7.7 (step 4, the limit of the detection), `AGENTS.md`, `.claude/rules/sql.md` | ADR-0067 (k), first risk. ADR-0078 (the `DEFAULT` risk) |
| 0085 | Pause after an accept fault | `reliability.md` 7.11 (row `http.accept_error_pause`) | ADR-0058 |
| 0086 | Bounds and anchor of the audit chain check | `data-model.md` 4.9 (step 1, its SQL, break rules, end check, value of `audit.last_check_seq`). `reliability.md` 7.11 (rows `audit.check_statement_timeout`, `audit.last_check_seq`). `data-model.md` 4.9 (the batch column `last_clean`, the order of the state writes), 5.1 (the claim on a rewrite) | ADR-0022 |
| 0087 | Drain of the internal listener | `reliability.md` 7.3 (steps 4 to 6, the first sum rule), 7.11 (row `shutdown.internal_drain`) | ADR-0058 (c), (m), (n), (o), (p). ADR-0070 (v) |

- Task 1.3 writes the Status line "Amended by ADR-NNNN" of each ADR of the column "Amends" and its index row. Task 1.3 also adds the line for each other accepted ADR whose rule a new ADR changes.
- `docs/adr/README.md` gets 31 index rows. The decision map row "Deployment, migrations, backup, panic policy, health" lists ADR-0057 to ADR-0070, ADR-0073 to ADR-0078, and ADR-0080 to ADR-0085, and ADR-0087. A new row "Read-only banner" lists ADR-0071 and ADR-0073. The row "Export" adds ADR-0072. The row "Errors of value objects without an own code" adds ADR-0073. The row "Audit log" adds ADR-0086.
- The row "Stored settings, security event retention" adds ADR-0062, ADR-0063, and ADR-0079. The row "Sessions, login, passwords, bootstrap" adds ADR-0065 (the default trusted proxy).
- The SQL of `0006` goes into the new `data-model.md` 5.5, as 5.4 holds `0005`. Section 3.8 gets prose only, so the document test of `0001` stays green.

### D3. Module map

New items by crate. A file name is a proposal. The agent keeps the style of the crate.

`recipe-domain`:

- `src/setting.rs` (new): `SettingId`, `SettingKey` (grammar of `app_setting_key_format`, `data-model.md` 3.17), `SettingValue` (at most `setting.value_max` characters), `SettingValue::fingerprint(text)` (`sha256:` and 64 lower-case hex characters, `sha2` exists).
- `src/audit/target.rs`: `AuditTarget::app_setting(SettingId)`. `src/audit/entry/`: the builder of `SettingChanged` with `actor = None` and the full row before and after (`data-model.md` 10).
- `src/audit/security.rs`: `SecurityEvent::audit_chain_broken(request_id)` with no client address (`data-model.md` 3.8).

`recipe-app`:

- `src/ports/` (new files): `MaintenanceStore`, `AuditChainReads`, `OperationsReads`, `ReadinessProbe`, `DiskSpace`, `BackupDirectory`, `AlertWebhook`, `ReadOnlySwitch`. A new `src/ports/setting.rs` holds `SettingRepository`, which joins `Transaction` in `unit_of_work.rs`.
- `WebhookFault`, `DiskFault`, and `BackupFault` are reason types without `code()`, as ADR-0025 permits. `reliability.md` 1.1 lists them like `ValueFaults` (ADR-0073 (b)).
- `src/operations/` (new): `AlertCounters` (two `Arc<AtomicU64>`: panics and `E-EXP-002`), `ReadinessState` (atomics of `reliability.md` 8), `CompareSettings` (the internal operation of `reliability.md` 7.12).
- `src/jobs.rs` and `src/jobs/` (new): one file per job with `run_once`. Task 3.1 writes `jobs.rs` with the six `mod` lines and six files that hold only their doc comment. Each job task then owns one file.
- `src/memory/`: the in-memory adapters of the new ports (task 3.10). `memory/core/state.rs` gets the rows of the new ports.
- The three test wrappers of `Transaction` forward the new accessor: `src/export/tests.rs`, `src/plant/short_rows.rs`, and `src/recipe/queries/count_tests.rs`.
- `AppContext::with_counters(counters)`. `AppContext::new` stays, so no caller changes. `ExportVersion` increments the `E-EXP-002` counter.

`recipe-db`:

- `src/maintenance.rs`, `src/audit_chain.rs`, `src/operations.rs` (new): the adapters of the new ports. `Database` gets one accessor per port and `close`.
- `OperationsReads::state` returns a typed time that `recipe-db` parses, so `recipe-app` keeps its closed dependency list (ADR-0035 (g)).
- `src/transaction/settings.rs` (new): `SettingRepository` of the transaction.
- `src/options.rs`: options without the pgpass file, every field set by code, and `ssl_mode = disable`. `src/migrator.rs`: the connect timeout and the list of embedded migrations for the readiness rule.
- `src/settings.rs`: `PoolSettings::wait_warn` (default `db.pool_wait_warn`, 1 s) calls `acquire_slow_threshold`. The health pool is a second `Database` with `size = db.health_pool_size`.
- `migrations/0006_operations.sql` (new), `tests/grants.expected`.
- `tests/storage/support/`: a test-only init script puts the host lines of the harness at the top of `pg_hba.conf` (D8).

`recipe-web`:

- `src/panic.rs` (new): the two `CatchPanicLayer` instances (ADR-0057).
- `src/timeout.rs` (new): the request timeout and the body deadline (ADR-0058). The request timeout skips the export paths (ADR-0072). `src/csrf.rs` maps a body timeout to `E-WEB-009`.
- `src/health.rs` (new): `health_router(probe, state)` with `/healthz` and `/readyz`, the readiness cache, the layers of D5, and no `client_address`.
- `src/gates.rs` puts the read-only flag into a request extension. `src/pages/request.rs` reads it. `src/layout.rs`, `templates/base.html`, and the bundles show the banner. `Layout::for_login` and `Layout::bare` and their callers also get the flag. `render_errors` runs outside `gates`, so the error page reads the flag from `WebState` (ADR-0071 (e)).
- `src/state.rs`: `ReadOnlyMode` implements `ReadOnlySwitch`.
- `src/settings.rs`: `request_timeout`, `body_read_timeout`, and the default trusted proxy `172.28.0.2/32`.

`recipe-server`:

- `src/lib.rs` (new, task 7.2): every module of the crate. `src/main.rs` parses the arguments and calls the library. `Cargo.toml` sets `publish = false`.
- `src/pg_env.rs` (new, task 4.2): the refusal of a set `PG*` variable for `serve`, `migrate`, and `create-admin`.
- `src/migrate.rs` (new, task 7.2): the run of the subcommand `migrate`. `src/create_admin.rs` gets the run of `create-admin` (task 7.2).
- `src/config/`: the operator keys of D7, the key files, and the webhook file.
- `src/logging.rs`, `src/panic_hook.rs`, `src/probe.rs`, `src/watchdog.rs`, `src/listener.rs`, `src/supervisor.rs`, `src/schedule.rs`, `src/clock.rs`, `src/disk.rs`, `src/backups.rs`, `src/webhook.rs`, `src/serve.rs` (new).
- `src/listener.rs`: `ListenerSettings::accept_error_pause` takes `http.accept_error_pause` (ADR-0085). `ListenerSettings::drain_deadline` takes `shutdown.drain` for the public listener and `shutdown.internal_drain` (0 s) for the internal listener (ADR-0087).
- `tests/support/` (new): the shared harness of `tests/migrate.rs` and `tests/create_admin.rs`.
- `tests/logging_start.rs` and `tests/bounded_run/` (new, task 7.2): the child test of a bad `RUST_LOG` and the shared function `run_bounded`.

New direct dependencies (ADR-0060, ADR-0063):

- `recipe-server` gets `axum`, `hyper` (`server`, `http1`), `tokio-util`, `tower`, and `time`.
- It gets `hyper-util` (`tokio`, `server`, `server-graceful`, and `http1` from task 7.4, and `service` and `client-legacy` from task 7.7).
- It gets `hyper-rustls` (no default features, `http1`, `tls12`, `ring`, `webpki-tokio`) and `rustls` (no default features, `ring`, `std`, `tls12`).
- It gets `http-body-util`, `rustix` (`cfg(unix)`, `fs`, `std`), `tracing-subscriber` feature `env-filter`, and `async-trait`. The ports `Clock`, `DiskSpace`, and `BackupDirectory` are `#[async_trait]` traits.
- `tokio` of `recipe-server` gets `net`, `time`, `macros`, `sync`, and `signal`. Dev-dependencies: `tokio` `test-util` and `tower-http` with `catch-panic`. Task 7.4 adds them. Task 7.8 adds the dev-dependency `recipe-app` with `memory`, because its start tests use stubs of the ports.
- `recipe-web` gets the features `catch-panic` and `timeout` of `tower-http` and `tokio` with `time`. Its dev `tokio` gets `test-util`. Its dev-dependencies also get `http-body-util` with `channel` for a request body that never ends (task 6.2).
- Only `matchers` 0.2.0 is new in `Cargo.lock`. `deny.toml` keeps `sqlx` in `recipe-db` only.

Owners of `Cargo.lock`: W1 (wave 1, `catch-panic`), W2 (wave 2, `timeout`), S2 (wave 3), S4 (wave 5), S6 (wave 7), S7 (wave 9). H1 (wave 0b) owns it only when `cargo metadata` changes it.

### D4. Start and stop of `serve`

ADR-0061 holds the order. ADR-0058 holds the stop. The order:

1. Fallback logger, then `init_logging` with `RUST_LOG` (default `info,sqlx=warn,hyper=warn`). Panic hook with a clone of `AlertCounters`. The hook calls `record_panic`.
2. Refusal of a set `PG*` variable. Configuration, both key files (`KeyRing::parse`, the carriage return rule of ADR-0031 and ADR-0035), the webhook file. A failure writes one `ERROR` line and returns exit code 1.
3. Watchdog thread, not armed. Its handle has `arm`, `disarm`, and `stop`. Runtime with `worker_threads(runtime.worker_threads)`.
4. Signal stream (`SIGTERM` on Unix, Ctrl-C on both).
5. Main pool (`db.pool_size`) and health pool (a second `Database` with `db.health_pool_size`). Role check (`reliability.md` 5).
6. `WebState` with its read-only flag. `serve` clones `read_only()` for the jobs. The supervisor, with no loop yet. `Supervisor::run_at_start` runs the first `disk_check` iteration inside the span `job` with `iteration` 1. It sets the read-only flag. An `Err` or a panic of that run fails the start (ADR-0061 (b), ADR-0089 (b)).
7. `CompareSettings` in one transaction. It writes `SettingChanged` also in read-only mode.
8. Internal listener on `0.0.0.0:<http.internal_port>`. Arm the watchdog.
9. The loops of the supervised jobs on the supervisor of step 6. `clock_check` runs once at once. The loop of `disk_check` continues with `iteration` 2 and first waits one `health.disk_check_interval` (ADR-0089 (b)).
10. Public listener on `0.0.0.0:<http.public_port>`. Readiness turns on after the first successful probe.

- Both listeners use the same accept loop, the same header read timeout, and the same accept pause. Only the drain deadline differs (ADR-0087 (c)). The loop puts `ConnectInfo<SocketAddr>` into each request, because `client_address` gives `E-WEB-010` without it.
- The loop collects finished connection tasks with `join_next` in its `select!`.
- A signal before step 10 ends the start. The server closes what it opened and exits with code 0.

The stop on the signal:

1. Readiness off. The watchdog stops. The public listener stops accepting. The jobs get the cancel.
2. Drain of the public connections and of the open job iterations, together, bounded by `shutdown.drain`. The supervisor handle has `drain(deadline) -> DrainOutcome`. The loop of the connections aborts the rest without a wait (ADR-0087 (d)). After the deadline, the supervisor aborts the open job iterations with no wait and counts them in its outcome (ADR-0060 (j)). An open transaction rolls back.
3. Close both pools within `shutdown.pool_close`. The pool close waits for the connections of the aborted tasks. For a close that returns `CloseOutcome::TimedOut`, `serve` writes one `WARN` line that names the pool, and the stop continues (ADR-0090 (b)). Then the internal listener stops with a drain of 0 s and cuts its connections at once (ADR-0087).
4. `runtime.shutdown_timeout(shutdown.runtime_stop)`. Exit code 0, also after a cut drain or a pool close that timed out. When one count is above 0, one `WARN` line gives three counts. The line names `cut_public` and `cut_internal` for the connections of the two listeners. It names `cut_jobs` for the cut job iterations. When all three counts are 0, the stop writes no such line (ADR-0087 (d), ADR-0088 (e), ADR-0089 (c)).

Sum rules of `reliability.md` 7.3: `shutdown.drain + shutdown.pool_close + shutdown.internal_drain + shutdown.runtime_stop < app.stop_grace_period` and `http.request_timeout <= shutdown.drain`. T23 checks both.

### D5. Layers of `recipe-web`

The order, outer first. The new layers are in bold:

`request_id` → **backstop `CatchPanicLayer`** → `TraceLayer` → CSP → nosniff → Referrer-Policy → HSTS → Cache-Control → `render_errors`. Then **`CatchPanicLayer`** → **request timeout** → **`RequestBodyDeadlineLayer`** → `client_address` → `body_limit` → `DefaultBodyLimit`. Then `resolve_access` → `session_layer` → `csrf_layer` → `gates` → handler.

- The inner panic handler ignores the payload and returns a `500` with a marker in the response extensions. `render_errors` turns the marker into `WebError::Internal` (`E-WEB-010`) with the `request_id`. It writes the page, the HTMX fragment, or the JSON body (ADR-0053 (a), AD-7 of P3.5).
- The backstop answers a plain `500` with `Content-Type: text/plain`. The body holds the code `E-WEB-010` and the English text of its row in `reliability.md` 1.2 without the reference. `request_id` still adds `X-Request-Id`. The answer has no `HX-*` header and no JSON body.
- The backstop writes its own `ERROR` line. The line holds no `request_id`, because the handler gets only the panic payload. The panic hook line holds it.
- ADR-0057 names the backstop as the one exception to `api.md` 5.1 to 5.3, `.claude/rules/web.md`, and the web-ui requirement "Internal error response". It is also the exception to ADR-0053 (a), ADR-0036 (p), ADR-0037 (z), ADR-0047 (b), and ADR-0049 (k).
- The backstop answer has only `X-Request-Id` of the headers of `api.md` 9. Caddy adds HSTS, CSP, `nosniff`, and `Referrer-Policy` as defaults (ADR-0057 (j), ADR-0065 (f)).
- The request timeout wraps `next.run` in `tokio::time::timeout(http.request_timeout)`. It returns `AppError::Timeout { stage: Request }`. `render_errors` adds `Retry-After` (`http.retry_after`). The request timeout skips the export paths (ADR-0072).
- `csrf.rs` walks the `source()` chain of a body error. A `tower_http::timeout::TimeoutError` gives `E-WEB-009`. Every other body error stays `E-WEB-003`.
- The health router is a separate `Router`. It has `request_id`, the security headers of `api.md` 9, and `TraceLayer` with the finish line at `DEBUG`. It has no `client_address`, no session, and no CSRF layer. The internal listener serves it.
- The health router has no `CatchPanicLayer`. A panic in a health handler ends only its connection (ADR-0058 (f)), and the probe counts a failure.

### D6. Ports and jobs

ADR-0060 holds the rules. The contracts:

| Port | Methods (sketch) | Adapter | Memory adapter |
|---|---|---|---|
| `MaintenanceStore` | `delete_expired_sessions`, `purge_login_attempts(retention, batch) -> u64`, `purge_security_events() -> u64`, `record_state(value: &StateValue)`. `StateValue` is a closed enum of the three state keys with a typed value (`ChainSeq` or `Timestamp`) | `recipe-db` | yes |
| `AuditChainReads` | `head() -> ChainHead` (head, bounds, anchor of ADR-0086 (g)), `check_batch(after, head, batch, bytes) -> BatchOutcome` (SQL of `data-model.md` 4.9) | `recipe-db` | yes |
| `OperationsReads` | `snapshot(windows) -> OperationsSnapshot` in one `READ ONLY` transaction: `database_now`, `replication_state` (function of ADR-0069), failed logins, security alerts, `audit_chain_broken_at`, `audit_last_check_at`, `first_setting_at`. Also `database_now()` for `clock_check`, `role_check()` for step 5 of D4, and `state(key: TimeStateKey) -> Option<Timestamp>`. `TimeStateKey` lists the two time keys `audit.last_check_at` and `heartbeat.last_at` | `recipe-db` | yes |
| `ReadinessProbe` | `check(timeout) -> Readiness` (ping and schema rule of ADR-0059) | `recipe-db` on the health pool | yes |
| `DiskSpace` | `free_space(path) -> FreeSpace` (`f_bavail`, `f_blocks`) | `recipe-server` (`rustix`, `Unsupported` on Windows) | yes |
| `BackupDirectory` | `newest_complete_set() -> Result<Option<Timestamp>, ..>` (a manifest file, an unreadable directory gives `Err`) | `recipe-server` | yes |
| `AlertWebhook` | `send_all(conditions) -> Vec<Result<(), WebhookFault>>` | `recipe-server` | yes |
| `ReadOnlySwitch` | `set(bool)`, `is_on()` | `recipe-web::ReadOnlyMode` | yes |
| `SettingRepository` (in `Transaction`) | `lock_rows()`, `upsert(key, value)` | `recipe-db` | yes |

The time of a backup set is the modification time of its manifest, the same time that `backup-health` reads with `stat -c %Y`. A set is complete when its manifest exists (ADR-0074 (c)).

Each job has `run_once(&self, request_id: RequestId, kind: RunKind) -> Result<(), AppError>`. `RunKind` (`Scheduled` or `CatchUp`) lives in the module `jobs` of `recipe-app` (ADR-0088 (b), (d)). A job writes its `WARN` lines itself. It logs a handled fault, for example a `WebhookFault`, once at `WARN`. It returns an `Err` only for an error of `reliability.md` 3.3 level `ERROR`, and `alerts` also for a failed snapshot read. The supervisor logs that `Err` once with the field `job`, at `WARN` for `alerts` and at `ERROR` for every other job (ADR-0089 (d)).

The run state of a job between runs lives as atomics in the job (`reliability.md` 8, ADR-0060 (aa)). It holds the start of the last security event purge and the two due times of `cleanup` (ADR-0088 (c)). It also holds the time of the last low-disk `WARN` line. It holds the time of the last read-fault `WARN` line and the `Unsupported` flag of `disk_check`. It also holds the counter totals and the panic slots of `alerts` (ADR-0073 (d), (g)).

Each run of a job gets a `request_id`. The rows of the run hold it, for example the row of `audit_chain_broken`. The log lines of a job get the field `job` from the span `job` and hold no `request_id` (`reliability.md` 3.2, ADR-0088 (a)).

| Job | Schedule | Notes |
|---|---|---|
| `cleanup` | one loop with the shorter of `session.cleanup_interval` and `login.attempt_cleanup_interval`. The job keeps a due time for the sessions and one for the attempts (ADR-0088 (c)) | The security event purge runs when `security_event.purge_interval` has passed since its last run. Batches of `data-model.md` 6. A failed batch keeps the batches before it, and the next run continues |
| `disk_check` | every `health.disk_check_interval`, and once in step 6 of D4 | `pgdata` only sets the mode. `backups` and `walarchive` give the warning of 3.7. Integer compare with `checked_mul`. `Unsupported` gives one `WARN` line per process. `Unreadable` on `pgdata` turns the mode on (ADR-0073 (d)) |
| `audit_chain_check` | daily at `audit.check_time`, and once after `audit.catchup_delay` (D1) | The nightly run and the catch-up share one loop and never overlap. A run of kind `CatchUp` reads `audit.last_check_at` again. It skips when the value is newer than the value that the start read. A run of kind `Scheduled` never skips (ADR-0088 (b)). Writes `audit_chain_broken` through `SecurityEventSink`, then returns `AppError::Audit(AuditError::ChainBroken)` with `E-AUD-002` (ADR-0089 (a)). Applies the rules of ADR-0086 (c) to (e). |
| `heartbeat` | every `heartbeat.interval` | `record_state(StateValue::HeartbeatLastAt(now))` |
| `clock_check` | once at start, then every `clock.check_interval` | `WARN` above `clock.skew_warn` |
| `alerts` | every `alerts.interval` | The ten conditions of 3.7, the start grace of D1, the webhook of ADR-0063. One `OperationsReads::snapshot` per run. The `POST` requests of one run go out at the same time under one `alerts.webhook_timeout`. The rules of ADR-0073 (a), (c), (f), and (g). A failed snapshot read ends the run with that error, with no alert and no webhook call (ADR-0089 (d)). A connected receiver without a lag makes the condition `replication` active (ADR-0090 (a)) |

The supervisor runs one `tokio::spawn` per iteration inside the span `job` with the fields `job`, `iteration`, and `outcome`. It passes the `request_id` and the `RunKind` of the fired trigger. A restart keeps the kind. Its method `run_at_start` runs the first run of `disk_check` in step 6 of D4 inside the span with `iteration` 1. The loop of that job continues with 2 (ADR-0089 (b)). It restarts after `job.restart_delay` on a panic. It starts the next run one interval after the end of the last run, so a late run gives no burst. It stops on the cancel token. Its handle has `drain(deadline) -> DrainOutcome` for step 2 of the stop (R4 Q5). `DrainOutcome::cut` gives `cut_jobs` of stop step 4.

The jobs and `CompareSettings` are internal operations without a permission and without a row of the matrix. `api.md` 3.2 and `reliability.md` 6 list them next to `EnsureDerivedType`.

### D7. Configuration of `serve`

ADR-0062 holds the list of 18 keys. The keys, their environment names, and the canonical text in `app_setting`:

| Key | Environment name | Canonical text |
|---|---|---|
| `db.host`, `db.port`, `db.name`, `db.app_user` | `RECIPE_DB__HOST`, `RECIPE_DB__PORT`, `RECIPE_DB__NAME`, `RECIPE_DB__APP_USER` | As given. The port as a decimal number |
| `secrets.app_password_file` | `RECIPE_SECRETS__APP_PASSWORD_FILE` | The path |
| `security.login_hmac_key_file`, `security.csrf_signing_key_file` | `RECIPE_SECURITY__LOGIN_HMAC_KEY_FILE`, `RECIPE_SECURITY__CSRF_SIGNING_KEY_FILE` | The path |
| `http.trusted_proxies` | `RECIPE_HTTP__TRUSTED_PROXIES` (comma list) | The CIDR texts joined by `,` in the given order |
| `http.public_port`, `http.internal_port` | `RECIPE_HTTP__PUBLIC_PORT`, `RECIPE_HTTP__INTERNAL_PORT` | The port as a decimal number |
| `approval.separation_of_duties` | `RECIPE_APPROVAL__SEPARATION_OF_DUTIES` | `true` or `false` |
| `alerts.webhook_url_file`, `alerts.webhook_ca_file` | `RECIPE_ALERTS__WEBHOOK_URL_FILE`, `RECIPE_ALERTS__WEBHOOK_CA_FILE` | The path, or empty |
| `alerts.webhook_url` | none, derived from the file | `sha256:` and 64 hex characters, or empty (ADR-0063) |
| `session.secure` | `RECIPE_SESSION__SECURE` | `true` or `false`. `false` stops the start without the feature `dev` |
| `session.idle_timeout`, `session.absolute_timeout` | `RECIPE_SESSION__IDLE_TIMEOUT`, `RECIPE_SESSION__ABSOLUTE_TIMEOUT` (whole seconds) | Whole seconds and `s`, for example `1800s` |
| `i18n.translation_langs` | `RECIPE_I18N__TRANSLATION_LANGS` (comma list) | The tags joined by `,` |

- `serve` records only these 18 keys. The keys of `migrate` stay out of `app_setting`. This narrows ADR-0022 and `reliability.md` 7.12.
- `healthcheck` reads `RECIPE_HTTP__INTERNAL_PORT` and no other value.
- Every other numeric value of 7.11 that `serve` uses is a code default. The existing unit tests compare the defaults with 7.11.
- `create-admin` and `migrate` keep their own key lists and `deny_unknown_fields`.
- A refusal of the webhook file names the key only. The refusals are a URL that does not parse, a scheme other than `https`, a carriage return, and a second line. A missing file at a set path is a refusal too. An empty key or an empty file means no webhook and gives one `INFO` line. No log line and no `Display` or `Debug` text holds a part of the URL. An unset key and a missing default file also mean no webhook.
- `session.idle_timeout` and `session.absolute_timeout` have no upper bound. The list of step P4.2 keeps this point.
- A port key is a decimal number from 1 to 65535 with no sign and no space. `serve` and `healthcheck` use one parser, `parse_port` in `port.rs`.
- Without a CA file, the connector uses `with_provider_and_webpki_roots` with `ring`. With a CA file, it uses `with_tls_config` with an own `ClientConfig` that holds only the certificates of the file. Both use `https_only()`.
- Secret files (ADR-0062, runbook): a root-owned directory `${RECIPE_SECRETS_DIR}` outside the repository with mode `0700`. Each file with one reader has mode `0400` and the uid of that reader. `tls_cert` and `tls_key` belong to uid 0, the user of the Caddy image.
- `app_password` and `migrator_password` have two readers and get a POSIX ACL for uid 999 and uid 65532 with the mask `r--`. Their mode then shows `0440`, and the owning group gets no right. The runbook makes each value with `openssl rand -hex 32`.

### D8. Database changes

ADR-0069 holds the rules.

- `01-roles.sh`: `GRANT pg_read_all_stats TO audit_owner` (with `INHERIT TRUE`). The grant to `recipe_app` goes away. The script header names ADR-0069.
- `01-roles.sh` writes a new `pg_hba.conf` in place of the catch-all line of the image.
  - The socket line admits `postgres` only, with the method `peer` (ADR-0069 (r), user decision). A test that runs `psql` through `docker exec` runs it as the system user `postgres`.
  - The `host` lines admit `recipe_migrator`, `recipe_app`, and `recipe_backup` to `recipe`, and `recipe_replicator` to `replication`, from `${COMPOSE_SUBNET}` only.
  - The probe line `host recipe pg_isready 127.0.0.1/32 scram-sha-256` serves the healthcheck. No role `pg_isready` exists, so the probe ends at the password request and the log gets no `FATAL` line.
  - A final line rejects every other connection.
- The guard list of `audit_guard.audit_objects_dropped()` in `01-roles.sh` gets `public.replication_state()`. Only the superuser can replace the guard function.
- `0006_operations.sql` runs the five steps of `data-model.md` 5.1 in one file:
  1. `GRANT CREATE ON SCHEMA public TO audit_owner`.
  2. `SET ROLE audit_owner`.
  3. `CREATE INDEX security_event_occurred_at_idx ON security_event (occurred_at)` and `CREATE FUNCTION replication_state()`. `REVOKE EXECUTE ... FROM PUBLIC`, `GRANT EXECUTE ... TO recipe_app`.
  4. `RESET ROLE`.
  5. `REVOKE CREATE ON SCHEMA public FROM audit_owner`.
- `replication_state()` is `SECURITY DEFINER` with `search_path = pg_catalog, public, pg_temp` (`.claude/rules/sql.md`). It qualifies every name.
- The function filters on `usename = 'recipe_replicator'` and `application_name = 'walarchive'`. It returns one row `connected boolean, lag_bytes numeric`.
- A plain `CREATE INDEX` is allowed, because `security_event` is small at this release (`.claude/rules/sql.md`, rewrite rule).
- `grants.expected` gets routine rows, for example `replication_state()|EXECUTE` and the missing row of `security_event_purge()`. The format test reads them, and the grant test reads `information_schema.role_routine_grants`.
- `has_function_privilege` checks that `PUBLIC`, `recipe_backup`, and `recipe_migrator` cannot run `replication_state()`.
- The guard tests change: `audit_hardening.rs` counts 8 objects of `audit_owner`. `grants/checks.rs` and `audit_guard.rs` list the new function.
- The document test of `0006` reads `data-model.md` 5.5 in `tests/storage/operations_schema.rs`. `tests/storage/schema.rs` does not change.
- The storage harness mounts a test-only init script from `tests/storage/support/`. It puts the host lines of the harness at the top of `pg_hba.conf`. The production script holds no test switch.
- `options.rs`: options without the pgpass file, every field set by code, `ssl_mode(PgSslMode::Disable)`.
- `recipe-server/src/pg_env.rs`: before any other step, `serve`, `migrate`, and `create-admin` refuse a set `PG*` variable. The `ERROR` line names the variable and not its value. The exit code is 1.

### D9. Layout of `deploy/`

| File | Content |
|---|---|
| `deploy/Dockerfile` | ADR-0064: stages `chef`, `planner`, `builder` (musl, build image by digest), runtime `distroless/static-debian13:nonroot` by digest, `USER 65532:65532`, `ENTRYPOINT ["/usr/local/bin/recipe-server"]`, `CMD ["serve"]`, no `HEALTHCHECK` |
| `deploy/pgtools/Dockerfile` | `postgres:16.15-bookworm` by digest, supercronic v0.2.51 with an own SHA-256 checked before `chmod +x`. It copies `bin/` as a directory, both crontab files, `restore-code.sql`, and `restore-code.expected` (ADR-0067 (j)). `USER 999:999` |
| `deploy/pgtools/restore-code.sql`, `restore-code.expected` | The query builds one line for each function, `CHECK` constraint, generated column, trigger (`NOT tgisinternal`, with `tgenabled`, ADR-0078 (a)), and event trigger outside `pg_catalog` and `information_schema`. A line holds the kind, the name, the owner, and the SHA-256 of the definition (ADR-0067 (i), ADR-0076 (a)). The file holds the lines of the release. The storage test `tests/storage/restore_code.rs` of `recipe-db` runs the query with `include_str!` on the harness database and compares the lines with the file |
| `deploy/pgtools/bin/` | `backup-entry`, `backup-once`, `backup-now`, `backup-health`, `restore-test`, `walarchive-entry`, `basebackup-once`, `walarchive-health`, `pitr-check` (bash, `set -euo pipefail`, JSON lines) |
| `deploy/pgtools/tests/` | One test script per script task (T25). The CI job runs them in a container of the pgtools image |
| `deploy/pgtools/crontab.backup`, `crontab.walarchive` | `CRON_TZ=UTC`. Dump at 02:00, restore test Sunday 05:00, base backup Sunday 03:30. The image copies both files to `/etc/pgtools/`. The entry scripts start supercronic with `-json -passthrough-logs` |
| `deploy/compose.yaml` | Seven services of `reliability.md` 7.1, the limits of D17, the secrets, the volumes of ADR-0066, the networks below |
| `deploy/compose.build.yaml` | `build:` for `recipe-server` and `recipe-pgtools` (dev and CI) |
| `deploy/compose.dev.yaml` | `Caddyfile.dev` and the published port `127.0.0.1:8443:443` |
| `deploy/Caddyfile`, `deploy/Caddyfile.dev` | ADR-0065. `reverse_proxy app:8080 { transport http { keepalive 5s } }` with `proxy.upstream_keepalive`. The global option `persist_config off`. The production file also sets `auto_https disable_certs` (ADR-0065 (b)). Two nested snippets hold the default headers (ADR-0065 (f)). `handle_errors` sets the HTMX headers of ADR-0065 (q) |
| `deploy/.env.example` | `RECIPE_VERSION`, `RECIPE_BACKUP_DIR`, `RECIPE_WALARCHIVE_DIR`, `RECIPE_SECRETS_DIR`, `COMPOSE_SUBNET=172.28.0.0/16`, `COMPOSE_IP_RANGE=172.28.1.0/24`, `PROXY_ADDRESS=172.28.0.2`, `SITE_ADDRESS`. No password |
| `deploy/ci/` | The scripts of the CI job (D11) |
| `.dockerignore`, `.gitattributes` | ADR-0064. `.dockerignore` is an allow list of the build inputs. `eol=lf` for `Caddyfile*`, `Dockerfile`, `crontab.*`, `deploy/pgtools/bin/*`, `deploy/pgtools/tests/*` |

Rules of `compose.yaml`:

- Network `edge` holds `proxy` and the published ports. Network `egress` holds `app` only and publishes no port. The runbook names a host firewall rule that limits its traffic to the webhook host. Network `backend` has `internal: true`, the subnet `${COMPOSE_SUBNET}`, and the `ip_range` `${COMPOSE_IP_RANGE}`, which excludes `${PROXY_ADDRESS}`. A change of the subnet edits the three values together (runbook).
- `backend` holds `app`, `db`, `backup`, `walarchive`, `migrate`, `backup-premigrate`, and `proxy` with the fixed address `${PROXY_ADDRESS}`.
- `app` gets `RECIPE_HTTP__TRUSTED_PROXIES: ${PROXY_ADDRESS:?}/32` from the same variable.
- Every service gets `cap_drop: [ALL]` and `security_opt: [no-new-privileges:true]`. `proxy` gets `cap_add: [NET_BIND_SERVICE]` only. `db` runs as `user: 999:999`. `app` and `proxy` get `read_only: true`. `proxy` mounts the volume `caddy` on `/data`.
- Each secret file is `${RECIPE_SECRETS_DIR:?}/<name>` on the host and `/run/secrets/<name>` in the container.
- `db` gets the `command:` list of `reliability.md` 7.6 and `db.max_connections`. The list holds `fsync`, `synchronous_commit`, `full_page_writes`, `wal_level`, `archive_mode`, `max_wal_senders`, and `wal_keep_size`.
- The list also holds `checkpoint_timeout`, `shared_buffers`, `max_connections`, `log_min_duration_statement`, `log_lock_waits`, `default_transaction_isolation`, and `timezone`.
- The healthcheck of `db` is `["CMD", "pg_isready", "-h", "127.0.0.1", "-p", "5432", "-U", "pg_isready", "-d", "recipe"]`, so `db` is healthy only after its init (D8 probe line).
- `backup`, `backup-premigrate`, and `walarchive` get `tmpfs: /run/pgpass`.
- The logging anchor of the seven services sets `mode: non-blocking` and `max-buffer-size: 4m` (`log.mode`, `log.max_buffer_size`).

Every value of 7.11 in these files has a check in `recipe-server/tests/deploy_values.rs` (task 9.2).

### D10. Backup, restore test, and WAL archive

ADR-0066, ADR-0067, and ADR-0068 hold the rules. Rules of every pgtools script:

- `umask 077` is the first command. Files get `0600` and directories `0700`. Only the top directory of `backups` gets `0755`, so uid 65532 can list the names. The top directory of `walarchive` stays `0700`.
- A delete uses `rm -- "${path:?}"` after a check of the path prefix. The marker files of ADR-0074 (a), the lock file `.lock`, and the scratch directory stay.
- The pgpass file lives in `/run/pgpass`. The scripts escape `\` and `:` (libpq rule) and keep each password out of argv and out of shell variables. No script enables `xtrace`.
- Layout of the volume `walarchive`, mounted at `/wal` (ADR-0077 (d)): the WAL segments at the top, `base/base_<YYYYMMDD_HHMMSS>`, the temp name `base/.tmp-base_<...>`, `.walarchive-marker`, `.lock`, and `.pitr-scratch/run.XXXXXX`.

The scripts:

- `backup-once <suffix>`: marker check, `flock` on `/backups/.lock`, a pgpass file from `/run/secrets/backup_password` (the one-line rule of ADR-0031). A psql session opens `BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY`, exports the snapshot, and counts the rows of each table except `session` and `login_attempt`. It stays open until `pg_dump --snapshot` ends. Then `pg_dump --snapshot`, `pg_dumpall --globals-only --no-role-passwords -l recipe`, `sync`, the publish of each file from its temp name `.tmp-<name>` with `ln -T` (ADR-0074 (b)), the manifest last, then the retention of ADR-0066. A failed run deletes only its own temp names and its own published files without a manifest. The counts file holds one line `schema.table|count` per table, in the order of schema and table name. An empty counts file means that the database has no table.
  - After `flock`, an existing file with a final name or a temp name of the new set gives one `ERROR` line. The script then writes no file and deletes no file. A publish never replaces a file (ADR-0066 (h), ADR-0074 (b)).
- `backup-health`: the filesystem check (`PG_VERSION` in the data directory, then `stat -c %d`), the marker, and a complete set younger than `backup.max_age_alert`. A set counts only by the grammar of ADR-0074 (c).
- `restore-test`: ADR-0067 in a private cluster under the backup disk. The cluster gets `initdb --auth-local=peer --auth-host=scram-sha-256` with a random superuser password file.
  - The scratch server listens on `127.0.0.1` only, with its socket in a private `0700` directory. It uses the `scratch.*` settings of D17, also `fsync=off`.
  - Each role logs in over `127.0.0.1` with its random password, and a wrong password must fail.
  - The script compares the `FUNCTION`, `TRIGGER`, and `EVENT TRIGGER` entries of `pg_restore -l` with the names of the lines of these kinds in `restore-code.expected` (ADR-0076 (b)). The lines of `audit_guard.audit_objects_dropped()` and its event trigger come from `deploy/db/initdb/01-roles.sh`. An unknown entry stops the test with an `ERROR` line. An entry of the kind `PROCEDURE`, `AGGREGATE`, `MATERIALIZED VIEW`, or `OPERATOR` also stops the test (ADR-0078 (b)).
  - The script restores `--section=pre-data` first. It runs `restore-code.sql` (`pg_get_functiondef`, `pg_get_constraintdef`, `pg_get_expr`, `pg_get_triggerdef`, `sha256`) and compares the function, `CHECK`, and generated column lines with `restore-code.expected`. A missing, extra, or changed line stops the test before the data load. Then it restores `data` and `post-data`. After `post-data`, it compares the lines of every kind. A difference stops the test before the row counts (ADR-0076 (c)).
  - A `trap` deletes the cluster on each exit.
- `walarchive-entry`: marker check, pgpass file, `pg_receivewal --synchronous -d "... application_name=walarchive"`, and supercronic. `wait -n` ends the container on the first child exit.
- `basebackup-once`: `flock -n` on `/wal/.lock`, `pg_basebackup --wal-method=fetch -Fp --checkpoint=fast --manifest-checksums=SHA256` into `/wal/base/.tmp-base_<YYYYMMDD_HHMMSS>`, `pg_verifybackup --quiet`, the rename to `base_<YYYYMMDD_HHMMSS>`, and retention 4. Then `pg_archivecleanup` runs with the lowest start segment of all kept base backups (ADR-0077 (f)). A failed `pg_verifybackup` deletes no base backup and no WAL segment.
- `pitr-check`: the CI recovery test of ADR-0077 (d), (e). It takes `/wal/.lock`, copies the newest `base_*` into `/wal/.pitr-scratch/run.XXXXXX`, and recovers from the copy with `restore_command = 'cp /wal/%f %p || cp /wal/%f.partial %p'`. It opens no connection to `db`, mounts no secret, and joins no network. It runs with `container.backup_memory`, `container.small_cpus`, and the `scratch.*` settings (ADR-0081).

ADR-0067 names two residual risks. A dump runs code as the restoring superuser. The name list and the code check of ADR-0067 (h) to (l) bound this risk. They do not bound the code of a generated column in the restore of `pre-data` (ADR-0084). That code runs as the scratch superuser. It can write files as uid 999, hide its change from the comparison of step 4, or keep the restore from ending. `backup` runs as uid 999 and can read the raw files of `pgdata`, also the SCRAM verifiers.

### D11. CI job `compose`

ADR-0070 holds the level and ADR-0094 holds the trigger rule (`workflow_dispatch` and a push to `main`). The job in `.github/workflows/ci.yml`, `runs-on: ubuntu-24.04`, `timeout-minutes: 45`. `ci.yml` also gets `workflow_dispatch`, so the WIP branch gets a run after the user permits a push.

1. Loop-mounted ext4 images for `RECIPE_BACKUP_DIR` and `RECIPE_WALARCHIVE_DIR`, with the marker files.
2. Secrets from `openssl rand -hex 32` (one line, no carriage return) in a directory of `$RUNNER_TEMP`, outside the build context. `RECIPE_SECRETS_DIR` names it. The TLS pair of `tls internal` needs no file.
3. `docker/setup-buildx-action` and `docker/build-push-action` by commit SHA, `load: true`, GitHub cache with one scope per image, cache writes from `main` only.
4. `docker compose -f compose.yaml -f compose.build.yaml -f compose.dev.yaml up -d --no-build`. A bounded poll of 60 s for the healthy services and the two one-shot exit codes.
5. `create-admin --password-file` and a login through Caddy with the pre-session token. Then `404` on `/healthz`, `/readyz?x=1`, `/HEALTHZ`, `/healthz/`, `//healthz`, and `/readyz%2F`.
6. A permanent redirect from HTTP to HTTPS. The user id 65532 of `app` from `docker inspect`. A login as `postgres` over TCP from `backup` to `db`. The server refuses it with the reject line of `pg_hba.conf`.
7. `backup-now`, `restore-test`, and `basebackup-once`. Then the marker entry: `POST /me/password` with the session of step 5 (`ChangeOwnPassword`, ADR-0077 (a)), and `303` to `/recipes`. Then `docker compose exec -T -u postgres db psql -d recipe` reads `chain_seq`, `entry_hash`, and `action` of the newest `audit_log` row and `pg_current_wal_lsn()`. A bounded poll of 60 s, one read each second, waits until `flush_lsn` of `walarchive` in `pg_stat_replication` reaches that position. The segment of the position exists only as `.partial`. Then `pitr-check` with `PITR_CI=1`, `PITR_MARKER_SEQ`, and `PITR_MARKER_HASH`. The modes of a new set with `stat -c %a`.
8. The test scripts of `deploy/pgtools/tests/` (T25). A search of the scripts for `set -x` finds nothing.
9. `docker compose stop app` with a request in flight, then the exit code 0 from `docker inspect`.
10. The backup healthcheck on one filesystem (a second run with a plain volume) fails.
11. On a failure: `docker compose ps -a` and `docker compose logs`.

The job reads `memory.peak` of `backup`, `walarchive`, `pitr-check`, and the `_premigrate` dump, and writes each value to the log. It sets no threshold (ADR-0067, ADR-0070).

A separate job `musl-unwind` runs `cargo test --target x86_64-unknown-linux-musl -p recipe-web panic` with `Swatinem/rust-cache`. It proves the unwind on musl.

### D12. Read-only banner

ADR-0071 holds the place. The banner is `alert::notice("warning", text)` directly after `<main id="content">`, before `#error-panel`. It stands in the full page and in the content answer of `HX-Target: content`. It has no role, no button, and no link. The login page and the error page show it too.

| Key | en | pl |
|---|---|---|
| `readonly-banner` | Read-only mode: the free disk space is low or unknown. Changes are blocked. You can view, export, log in, and log out. | Tryb tylko do odczytu: brakuje miejsca na dysku lub nie można go zmierzyć. Zmiany są zablokowane. Możesz przeglądać, eksportować, logować się i wylogować. |

### D13. Runbook

`docs/ops/runbook.md` is one how-to page (Diátaxis, STE, scope of ADR-0023). A step has at most 20 words. The page links to `reliability.md` and the ADRs and does not repeat their rules. Sections:

- Host preparation: disks, marker files, the secret files of D7, Docker versions, and the host firewall check. Below Engine 28, a `DOCKER-USER` rule drops routed traffic to the Compose subnets.
- First start, first administrator, and image transfer. The operator compares the SHA-256 of each image file before `docker load` and the image ID after it (ADR-0064).
- Backup now, restore test, and restore into production (a new volume).
- Upgrade, rollback, major upgrade of PostgreSQL, key rotation, certificate rotation, and subnet change. The upgrade follows `reliability.md` 7.10. Steps 4 and 6 use `docker compose up -d --no-deps app` (ADR-0082). Steps 5 and 6 use the three `recipe-pgtools` commands (ADR-0083). A subnet change edits `COMPOSE_SUBNET`, `COMPOSE_IP_RANGE`, and `PROXY_ADDRESS` together.
- A gap in the WAL stream: move the old segments aside, take a new base backup, and start the stream again (ADR-0068 (e)).
- A plant device in front of Caddy: its CIDR goes into the Caddy `trusted_proxies` and `http.trusted_proxies`, one `/32` for each device (ADR-0065 (m)).
- An older volume: the two grant statements, the new guard function, and the new `pg_hba.conf` of ADR-0069, as the superuser.
- Alarms (one row per condition of 3.7 with the first action), read-only mode, and restart loop. The webhook URL holds a token of at least 128 random bits.
- Destructive commands: the runbook never uses `down -v` or `docker volume rm` on `pgdata`. Each step that deletes backup data names what it deletes and runs only after a passed restore test. Removing a secret key line or replacing a certificate deletes no backup data, so this rule does not apply.
- The check of `client_ip` of the first login in the security events.

### D14. Test plan

| Row | Level | Place | Checks |
|---|---|---|---|
| T1 | Unit | `recipe-domain` `setting` | Key grammar, value length, fingerprint vector, `SettingId` |
| T2 | Unit | `recipe-domain` audit | `SettingChanged` entry, `audit_chain_broken` event without client address |
| T3 | Storage | `recipe-db` `tests/storage/` | Migration 0006: index, function, routine grant rows, role test, `recipe_app` without `pg_read_all_stats`, the function as `recipe_app`. `has_function_privilege` of `PUBLIC`, `recipe_backup`, `recipe_migrator`. 8 objects of `audit_owner`. A read of `pg_stat_activity.query` as `recipe_app`. The lines of `pg_hba_file_rules`. `EXPLAIN (FORMAT JSON)` on 50 000 rows: the purge and the two alert sums use the new index |
| T4 | Storage | `recipe-db`, `recipe-server` | Options without the pgpass file. `serve`, `migrate`, and `create-admin` with `PGOPTIONS` in a child process: one `ERROR` line with the name only, exit code 1. Migrator connect timeout. `PoolSettings::wait_warn` equals 7.11 |
| T5 | Storage | `recipe-db` | `MaintenanceStore`: batches of 5 000, expired sessions, purge, state keys without an audit entry. A failure in the second batch keeps the first batch deleted and the failed batch whole |
| T6 | Storage | `recipe-db` | `AuditChainReads` with a break planted as the superuser. Sink test of `audit_chain_broken`. Planted rows at 0, below 0, and above the head, an empty table, and the anchor (ADR-0086). |
| T7 | Storage | `recipe-db` | `OperationsReads` in one transaction, `ReadinessProbe` with a changed checksum, a newer database, a failed row |
| T8 | Storage | `recipe-db` | `SettingRepository` upsert and the audit entry in one transaction, a failure midway writes nothing |
| T9 | HTTP | `recipe-web` | Panic: page, HTMX fragment, JSON on `/export/v1/`, backstop without `HX-*` header, next request `200` |
| T10 | HTTP | `recipe-web` | Request timeout with paused time, an export path without the request timeout (ADR-0072), body deadline gives `E-WEB-009`, `Retry-After`. Default trusted proxy `/32`: a neighbor address keeps its socket address |
| T11 | HTTP | `recipe-web` | Health router: `ok`, `not ready` with 503, cache 5 s, shutting down, no `client_address`, `X-Request-Id` and the security headers, finish line at `DEBUG`. The public router answers `404` for `/healthz` and `/readyz` |
| T12 | HTTP | `recipe-web` | Banner on and off, both languages, login page and error page, the XSS sweep and the template rules |
| T14 | Use case | `recipe-app` | Each job with memory adapters: every branch of D6, the start grace, the catch-up without overlap and with the skip. One snapshot per `alerts` run and the parallel webhook calls |
| T15 | Use case | `recipe-app` | `CompareSettings`: no change, one change, a missing row, read-only mode, the fingerprint, a failed commit |
| T16 | Use case | `recipe-app` | `E-EXP-002` increments the counter. Two exports with a mismatch give 2 |
| T17 | Unit | `recipe-server` | The 18 configuration keys, environment names, canonical text, key file faults, webhook file refusals that name the key only, `session.secure` |
| T18 | Unit | `recipe-server` | Probe against a local listener (200, 503, garbage, silence, a slow readiness answer), the budget of 3 s, `FailureWindow` |
| T19 | Unit | `recipe-server` | Accept loop: drain complete, drain cut, header read timeout on both listeners, keep-alive after a panic, `ConnectInfo` in each request. Also the abort after the deadline without a wait and a drain of 0 s. After 100 closed connections, `JoinSet::len()` returns 0 |
| T20 | Unit | `recipe-server` | Supervisor restart after a panic and after `Err`, cancel, `drain(deadline)`, schedules with paused time, a run of 90 s gives no burst |
| T21 | Unit | `recipe-server` | Panic hook in its own test binary through the library target: one JSON line with the `request_id` of the span, counter 1 |
| T22 | Integration | `recipe-server/tests/serve.rs` | Child process on testcontainers: `/healthz`, `/readyz`, the role check refusal, `SettingChanged` at the first start, a key file fault, `SIGTERM` (Unix only). Free ports through the two port keys |
| T23 | Unit | `recipe-server/tests/deploy_values.rs` | Each 7.11 value in `deploy/` equals the table. Each `pg.*`, `db.max_connections`, `container.*`, and `log.*` key is present in `compose.yaml` (`log.session_ref_length` is excepted, ADR-0070 (u)). Both sum rules of D4. The keep-alive line, the `pg_isready` line, the trusted proxies line, the `ip_range`, `cap_drop`, `cap_add`, and `no-new-privileges` |
| T24 | Stack | CI job `compose` | D11 steps 4 to 10 |
| T25 | Script | `deploy/pgtools/tests/` in the CI job | Retention with backdated sets (`touch -d`): old sets go, the newest daily and `_premigrate` sets stay, a failed dump deletes nothing. A changed byte gives a manifest mismatch. A missing marker. Two parallel runs (`flock`). An existing name of the new set gives an `ERROR` line, and no file changes. A changed function body, an extra function, and a missing line of `restore-code.expected` stop the restore test before the data load. A failed `pg_verifybackup` deletes nothing |
| T26 | Unit | `recipe-server` `serve` | The start order of D4 with stubs of the ports: low disk at startup, a failed comparison, a signal during the start. The stop order of D4: the internal listener stops after the pool close and cuts at once |
| T27 | Unit | `recipe-server` `webhook` | A local TLS listener with a test CA file, an `http` URL, a timeout. `Display` and `Debug` of `WebhookFault` and of the URL holder hold no part of the URL |

The row T13 merges into T10 with task 6.2. The browser checks of the banner (focus, 320 px) go to P4.2 (ADR-0048 (f)).

### D15. Known changes of test expectations

- `recipe-web/src/csrf/tests.rs`: a body that stops gives `E-WEB-009`, not `E-WEB-003`.
- `recipe-web/src/settings/tests.rs`: the default of `http.trusted_proxies` is `172.28.0.2/32`.
- `recipe-db/tests/storage/roles.rs`: `recipe_app` has no `pg_read_all_stats`. `audit_owner` has it.
- `recipe-db/tests/grants.expected`: the routine rows of `replication_state()` and `security_event_purge()`.
- `recipe-db/tests/storage/grants/file.rs` and `checks.rs`: the routine row format and its query. `audit_hardening.rs`: 8 objects. `audit_guard.rs`: the new name.
- `recipe-db/tests/storage/support/`: the test-only init script of the host lines.
- `recipe-server/tests/migrate.rs`: the applied list ends at `0006` (task 4.1, wave 1). The test name says "every migration", not "five".
- `recipe-server/tests/migrate.rs` and `create_admin.rs`: the helpers `applied_migrations` and `query` run `psql` as the system user `postgres` through `gosu` (ADR-0069 (r)).
- `recipe-db/tests/storage/roles.rs`: the socket line is `local all postgres peer`. `grants/init_script.rs`: the subnet check refuses `/0` and a leading zero.
- `recipe-web/src/error/export_json_tests.rs`: two new cases: a panic gives `E-WEB-010`, and a slow export path gets the answer of its handler (ADR-0072).
- `recipe-app`: the three test wrappers of `Transaction` of D3 forward the new accessor.
- `recipe-web/src/error/tests.rs`: the message of `E-WEB-006` is the text of ADR-0073 (h).
- `recipe-server/src/supervisor/tests.rs` and `span_tests.rs`: four tests used the job `alerts` as an incidental failing job. They now use `cleanup`. ADR-0089 (d) gives a failed `alerts` run a `WARN` line.

### D16. Waves

Classes: T1-O `opus` max, T1-C `opus` high, T2 `opus` xhigh, T3 `sonnet` high, T4 `haiku` medium. No agent uses `fable`. An agent that writes only tests is T3. From wave 2, a code row of class T1-C runs as T3 (`sonnet` high), by a decision of the user. By a decision of the user, a simple edit of a comment or a document runs as T4, at most T3. At most three cargo agents run at the same time. The third starts only with at least 8 GB of free RAM. Agents without cargo run beside them.

| Wave | Agent | Class | Model | Effort | Task | File set |
|---|---|---|---|---|---|---|
| 0a | DA | T1-C | opus | high | 1.1 | `docs/adr/0057-*.md` to `docs/adr/0064-*.md` (new) |
| 0a | DB | T1-C | opus | high | 1.2 | `docs/adr/0065-*.md` to `docs/adr/0071-*.md` (new) |
| 0b | DC | T1-C | opus | high | 1.3 | `docs/adr/README.md`, `docs/architecture/reliability.md`, the Status lines of `docs/adr/0020-*.md`, `0022-*.md`, `0034-*.md`, `0053-*.md`, and of each other amended ADR |
| 0b | DD | T1-C | opus | high | 1.4 | `docs/architecture/overview.md`, `domain-model.md`, `data-model.md`, `api.md`, `design-system.md`, `CLAUDE.md`, `.claude/rules/rust.md`, `.claude/rules/web.md` |
| 0b | H1 | T4 | haiku | medium | 5.1 `[mechanical]` | Root `Cargo.toml` (`[workspace.dependencies]` only), `Cargo.lock` only when `cargo metadata` changes it |
| 0b | OR | orchestrator | none | none | 1.7 (no cargo) | `AGENTS.md` |
| 1 | X1 | T3 | sonnet | high | 2.1 | `recipe-domain`: `src/setting.rs` (new), its tests, the `mod` line in `src/lib.rs` |
| 1 | D1 | T1-C | opus | high | 4.1 `[hard]` | `migrations/0006_operations.sql` (new), `deploy/db/initdb/01-roles.sh`. `recipe-db`: `tests/grants.expected`, `tests/storage/roles.rs`, a new `tests/storage/operations_schema.rs`, `tests/storage/main.rs`, `tests/storage/grants/file.rs`, `checks.rs`, `audit_hardening.rs`, `audit_guard.rs`, `init_script.rs` (the subnet check), `tests/storage/support/` (the test-only init script and its mount). `recipe-server`: `tests/migrate.rs` (the list of D15 and the mount of the test-only init script), `tests/create_admin.rs` (the same mount) |
| 1 | W1 | T1-C | opus | high | 6.1 `[hard]` | `recipe-web`: `Cargo.toml` (feature `catch-panic`), `src/panic.rs` (new), `src/lib.rs` (the `mod` line), `src/routes.rs` (`assemble`), `src/pipeline.rs` (`wrap`), `src/error/layer.rs` (the marker of the inner layer), `src/error/export_json_tests.rs`, a new `src/panic/tests.rs`. `Cargo.lock` |
| 1 | O1 | T3 | sonnet | high | 8.1 (no cargo) | `deploy/Dockerfile`, `.dockerignore`, `.gitattributes` |
| 1 | O6 | T3 | sonnet | high | 8.6 (no cargo) | `deploy/Caddyfile`, `deploy/Caddyfile.dev` |
| 2 | A1 | T1-C | opus | high | 3.1 `[hard]` | `recipe-app`: the new port files in `src/ports/`, `src/ports/mod.rs`, `src/operations.rs` and `src/operations/` (`AlertCounters`, `ReadinessState`), `src/jobs.rs` and the six doc-only files, `src/lib.rs`. `recipe-web`: `src/state.rs` (the impl of `ReadOnlySwitch`) |
| 2 | D2 | T1-C | opus | high | 4.2 `[hard]` | `recipe-db`: `src/options.rs`, `src/migrator.rs`, `src/settings.rs` and `src/settings/` (connect timeout, `wait_warn`), `src/database.rs` (`acquire_slow_threshold`), `tests/storage/main.rs`, `tests/storage/migrate.rs`. `recipe-server`: `src/pg_env.rs` (new), `src/main.rs` (the call before each subcommand), a new `tests/pg_env.rs` |
| 2 | W2 | T1-C | opus | high | 6.2 `[hard]` | `recipe-web`: `Cargo.toml` (feature `timeout`, `tokio` `time`, dev `test-util`), `src/timeout.rs` (new), `src/lib.rs` (the `mod` line), `src/routes.rs` (`assemble`), `src/settings.rs` (two fields and the default trusted proxy). Also `src/csrf.rs`, `src/csrf/tests.rs`, `src/settings/tests.rs` (also the test of the default trusted proxy), a new `src/timeout/tests.rs`, `src/error/export_json_tests.rs`. `Cargo.lock` |
| 2 | O2 | T1-C | opus | high | 8.2 `[hard]` (no cargo) | `deploy/pgtools/Dockerfile`, `deploy/pgtools/bin/backup-*`, `deploy/pgtools/crontab.backup`, `deploy/pgtools/tests/backup-test` and the sourced files `deploy/pgtools/tests/lib/*.sh` (new) |
| 3 | X2 | T3 | sonnet | high | 2.2 | `recipe-domain`: `src/audit/target.rs`, `src/audit/entry/` (the `SettingChanged` builder), `src/audit/security.rs`, their tests |
| 3 | A2 | T1-C | opus | high | 3.2 `[hard]` | `recipe-app`: `src/ports/unit_of_work.rs` (`SettingRepository`), a new `src/ports/setting.rs`, `src/ports/mod.rs`, `src/memory/core/transaction.rs`, `src/memory/core/state.rs`, their tests, `src/export/tests.rs`, `src/plant/short_rows.rs`, `src/recipe/queries/count_tests.rs`. `recipe-db`: `src/transaction.rs` (the accessor), `src/transaction/settings.rs` (new), a new `tests/storage/settings.rs`, `tests/storage/main.rs`, `src/error.rs` (the variant `ReservedKey`). Also `recipe-app` `src/memory/core/transaction/settings.rs` and its tests |
| 3 | S2 | T1-C | opus | high | 7.2 `[hard]` | `recipe-server`: `Cargo.toml` (feature `env-filter`, `[lib]`, `publish = false`), `src/lib.rs` (new), `src/main.rs`, `src/logging.rs`, `src/panic_hook.rs` (new), a new `tests/panic_hook.rs`. Also `src/create_admin.rs` (`run`), `src/migrate.rs` (new), the new `tests/logging_start.rs` and `tests/bounded_run/mod.rs`. Also the `RUST_LOG` removal in `tests/pg_env.rs`, `tests/create_admin.rs`, and `tests/migrate.rs`. `Cargo.lock` |
| 3 | O3 | T1-C | opus | high | 8.3 `[hard]` (cargo for the storage test only) | `deploy/pgtools/bin/restore-test`, `deploy/pgtools/tests/restore-test-test`, `deploy/pgtools/tests/lib/restore-*.sh` (new), `deploy/pgtools/restore-code.sql`, `deploy/pgtools/restore-code.expected` (new), `deploy/pgtools/Dockerfile` (the copy of the two files). `recipe-db`: a new `tests/storage/restore_code.rs`, `tests/storage/main.rs` |
| 3 | O4 | T1-C | opus | high | 8.4 `[hard]` (no cargo) | `deploy/pgtools/bin/walarchive-*`, `basebackup-once`, `pitr-check`, `deploy/pgtools/crontab.walarchive`, `deploy/pgtools/tests/walarchive-test` (new), `deploy/pgtools/tests/lib/wal-*.sh` (new) |
| 4 | A10 | T3 | sonnet | high | 3.10 | `recipe-app`: the memory adapters in `src/memory/` (new files, `memory/mod.rs`, `memory/core/state.rs`), their tests |
| 4 | D3 | T1-C | opus | high | 4.3 `[hard]` | `recipe-db`: `src/maintenance.rs` (new), `src/database.rs`, `src/lib.rs`, a new `tests/storage/maintenance.rs`, `tests/storage/main.rs` |
| 4 | S3 | T1-C | opus | high | 7.3 `[hard]` | `recipe-server`: `src/probe.rs`, `src/watchdog.rs` (new), `src/config/args.rs` (`healthcheck`), `src/main.rs` (dispatch), `src/lib.rs` (the `mod` lines), their tests |
| 4 | O5 | T1-C | opus | high | 8.5 `[hard]` (no cargo) | `deploy/compose.yaml`, `deploy/compose.build.yaml`, `deploy/compose.dev.yaml`, `deploy/.env.example` |
| 5 | A9 | T1-C | opus | high | 3.9 `[hard]` | `recipe-app`: `src/operations/compare_settings.rs` (new), the `mod` line in `src/operations.rs`, its tests |
| 5 | D4 | T1-C | opus | high | 4.4 `[hard]` | `recipe-db`: `src/audit_chain.rs` (new), `src/database.rs`, `src/lib.rs`, a new `tests/storage/audit_chain.rs`, `tests/storage/main.rs`, `tests/storage/security_events.rs` (the sink test) |
| 5 | S4 | T1-C | opus | high | 7.4 `[hard]` | `recipe-server`: `Cargo.toml` (every `tokio` feature and the dev-dependencies of D3), `src/listener.rs` (new), its tests, the `mod` line in `src/lib.rs`. `Cargo.lock` |
| 6 | A3 | T3 | sonnet | high | 3.3 | `recipe-app`: `src/context.rs` (`with_counters`), `src/export/flow.rs` (the increment), `src/export/tests.rs`, `src/context/tests.rs` |
| 6 | D5 | T1-C | opus | high | 4.5 `[hard]` | `recipe-db`: `src/operations.rs` (new), `src/database.rs`, `src/lib.rs`, `src/migrator.rs` (the embedded list, read only use), a new `tests/storage/operations.rs`, `tests/storage/main.rs` |
| 6 | S5 | T1-C | opus | high | 7.5 `[hard]` | `recipe-server`: `src/supervisor.rs`, `src/schedule.rs` (new), their tests, the `mod` lines in `src/lib.rs` |
| 7 | A4 | T3 | sonnet | high | 3.4 | `recipe-app`: `src/jobs/cleanup.rs` and its tests |
| 7 | W3 | T1-C | opus | high | 6.3 `[hard]` | `recipe-web`: `src/health.rs` (new), `src/lib.rs`, a new `src/health/tests.rs` |
| 7 | S6 | T3 | sonnet | high | 7.6 | `recipe-server`: `src/clock.rs`, `src/disk.rs`, `src/backups.rs` (new), their tests, the `mod` lines in `src/lib.rs`, `Cargo.toml` (`rustix`, `time`). `Cargo.lock` |
| 8 | A5 | T3 | sonnet | high | 3.5 | `recipe-app`: `src/jobs/disk_check.rs` and its tests |
| 8 | A6 | T1-C | opus | high | 3.6 `[hard]` | `recipe-app`: `src/jobs/audit_chain_check.rs` and its tests |
| 8 | S1 | T1-C | opus | high | 7.1 `[hard]` | `recipe-server`: `src/config/` except `args.rs` (serve keys, key files, webhook file), `src/config/tests.rs` |
| 9 | A7 | T3 | sonnet | high | 3.7 | `recipe-app`: `src/jobs/heartbeat.rs`, `src/jobs/clock_check.rs` and their tests |
| 9 | A8 | T1-C | opus | high | 3.8 `[hard]` | `recipe-app`: `src/jobs/alerts.rs` and its tests |
| 9 | S7 | T1-C | opus | high | 7.7 `[hard]` | `recipe-server`: `src/webhook.rs` (new), its tests, the `mod` line in `src/lib.rs`, `Cargo.toml` (`hyper-rustls`, `rustls`, `http-body-util`). `Cargo.lock` |
| 10 | S8 | T1-C | opus | high | 7.8 `[hard]` | `recipe-server`: `src/serve.rs` (new, the start), its tests, the `mod` line in `src/lib.rs`, `src/main.rs`, `src/config/mod.rs` (the wiring call only) |
| 10 | W4 | T1-C | opus | high | 6.4 `[hard]` | `recipe-web`: `src/gates.rs`, `src/pages/request.rs`, `src/layout.rs`, the callers of `Layout::for_login` and `Layout::bare`, `templates/base.html`, `i18n/en.ftl`, `i18n/pl.ftl` (the key of D12), a new `src/layout/banner_tests.rs` |
| 10 | S10 | T3 | sonnet | high | 9.2 | `recipe-server`: `tests/deploy_values.rs` (new) |
| 11 | S11 | T1-C | opus | high | 7.9 `[hard]` | `recipe-server`: `src/serve.rs` (the stop), its tests |
| 11 | C1 | T1-C | opus | high | 8.7 `[hard]` (no cargo locally) | `.github/workflows/ci.yml` (`workflow_dispatch`, the jobs `compose` and `musl-unwind`), `deploy/ci/` (new) |
| 11 | R1 | T3 | sonnet | high | 1.5 (no cargo) | `docs/ops/runbook.md` (new) |
| 12 | S9 | T3 | sonnet | high | 9.1 | `recipe-server`: `tests/support/` (new), `tests/serve.rs` (new), `tests/migrate.rs` and `tests/create_admin.rs` (the use of the shared harness) |
| 12 | CL | T3 | sonnet | high | 1.6 (no cargo) | `CHANGELOG.md` |
| 12 | OR | orchestrator | none | none | 10.1 (after S9) | none |

Rules of the waves:

- A shared file has one owner in each wave. The owner is the agent whose set names the file. Another agent reports the entry that it needs.
- A wave starts after the green CI of the wave before it. The WIP branch gets CI through `workflow_dispatch` after the user permits a push.
- After a wave with `[hard]` tasks, the next wave also waits for a verifier report (T2, read only, no cargo) without CRITICAL and HIGH.
- After each closed wave the orchestrator stops for `/compact` or a new session (user rule).
- A1 creates `src/jobs.rs` with the six `mod` lines and six files with only a doc comment, so A4 to A8 each own one file.
- S2 creates the library target in wave 3. From wave 4, the `mod` line of a new module of `recipe-server` goes into `src/lib.rs`.
- The deploy agents run no image build in waves 1 to 4. The first build runs in step 6 (QA) after the user decides on the free space of `C:`.
- The orchestrator asks the user before the commit of `CLAUDE.md`, `.claude/rules/rust.md`, `.claude/rules/web.md`, and `.gitignore`.
- A T3 task that fails twice goes to a T1-O diagnosis (read only) with both reports. Then a new T3 agent applies the diagnosis. A red CI that one fix does not make green also goes to a T1-O diagnosis (read only).
- Each code agent runs its one task through `/ccx:loop --min-severity low --loops 4` in single mode. Codex only reviews. Then a Claude reviewer CR-<agent> (T2, `opus` xhigh, read only) reviews the files of the agent.

### D17. New and changed rows of `reliability.md` 7.11

ADR-0058 to ADR-0070, ADR-0077, ADR-0085, and ADR-0087 own these rows. Task 1.3 writes them.

| Key | Value | ADR |
|---|---|---|
| `watchdog.probe_timeout` | 3 s. The budget of the subcommand `healthcheck`, above `health.db_check_timeout` | 0059 |
| `runtime.worker_threads` | 2 | 0061 |
| `alerts.webhook_timeout` | 10 s for all `POST` requests of one run | 0063 |
| `alerts.webhook_url_file` | `/run/secrets/alerts_webhook_url`. An empty key or an empty file means no webhook. A missing file at a set path stops the start | 0063 |
| `alerts.webhook_ca_file` | empty, which means the webpki roots | 0063 |
| `alerts.webhook_url` (changed) | The fingerprint `sha256:<64 hex>` of the URL in the file, or empty. Not a configuration key | 0063 |
| `http.public_port` / `http.internal_port` (changed) | 8080 / 9000. Configuration keys, recorded in `app_setting` | 0062 |
| `http.accept_error_pause` | 1 s. A code default | 0085 |
| `shutdown.pool_close` / `shutdown.runtime_stop` | 5 s / 3 s | 0058 |
| `shutdown.internal_drain` | 0 s. A code default | 0087 |
| `health.check_timeout` / `health.start_period` / `health.start_interval` | 5 s / 30 s / 1 s | 0070 |
| `db.stop_grace_period` / `db.shm_size` | 60 s / 256 MiB | 0070 |
| `container.oneshot_memory` | 256 MiB each for `migrate` and `backup-premigrate` | 0070 |
| `container.backup_memory` | 512 MiB for `backup` and the temporary container of `pitr-check` | 0067 |
| `container.small_memory` (changed) | 128 MiB each for `proxy` and `walarchive` | 0070 |
| `container.small_cpus` | 0.5 each for `proxy`, `backup`, `walarchive`, `migrate`, `backup-premigrate`, and the temporary container of `pitr-check` | 0070, 0081 |
| `log.mode` / `log.max_buffer_size` | `non-blocking` / 4m | 0070 |
| `proxy.upstream_keepalive` | 5 s, below `http.body_read_timeout` | 0065 |
| `scratch.shared_buffers` / `scratch.maintenance_work_mem` / `scratch.max_connections` | 64 MB / 64 MB / 10 | 0067 |
| `scratch.max_parallel_workers_per_gather` / `scratch.max_parallel_maintenance_workers` | 0 / 0 | 0067 |
| `scratch.restore_fsync` / `scratch.restore_wal_level` / `scratch.restore_max_wal_senders` | `off` / `minimal` / 0, for the cluster of `restore-test` only | 0067 |
| `scratch.pitr_hot_standby` | `off`, for the cluster of `pitr-check` only | 0068 |
| `scratch.port` / `scratch.stop_timeout` | 5433 / 60 s | 0077 |
| `scratch.restore_start_timeout` / `scratch.pitr_start_timeout` | 60 s / 600 s | 0077 |
| `restore_test.detail_length` | 160 characters | 0077 |
| `pitr_check.log_tail` | 20 lines | 0077 |
| `basebackup.time` / `basebackup.retention` | Sunday 03:30 UTC / 4 base backups | 0068 |
| `restore_test.time` | Sunday 05:00 UTC | 0067 |
| `migrate.connect_timeout` | 10 s | 0069 |
| `audit.catchup_delay` / `audit.catchup_age` | 10 min / 24 h | 0060 |
| `secrets.postgres_password_file` | `/run/secrets/postgres_password`. Read by the image of `db` only | 0069 |
| `http.trusted_proxies` (changed) | CIDR list, default `172.28.0.2/32`, the address of `proxy` | 0065 |

The musl fallback (ADR-0064) uses the soak ceiling and the p99 bound of `reliability.md` 6 as its trigger. It adds no number.

### D18. Deferred items

- In this change: S7 (D8), Q9 (D8), C7 (`migrate.connect_timeout`), S8 (D8). Also C6 (D17), the key file check (D4 step 2), the sink test of `audit_chain_broken` (T6).
- To P4.2: R2-5 (span name `http.request`), R2-3 (router fallback answer, ADR-0053 (g)). Also the cache of static files after an upgrade and the browser checks of the banner.

### D19. Plan review

Sources: `PR1.md` to `PR4.md` and `plan-review-triage.md`. "Accepted" means that the proposed change of the report applies, with the note of the triage.

| Id | Decision | Place of the change |
|---|---|---|
| PR1-1 | Accepted | D15, D16 (D1 owns `tests/migrate.rs`) |
| PR1-2 | Accepted | D8, D15, D16 (D1) |
| PR1-3 | Accepted, routine rows in `grants.expected` | D8, D15, D16 (D1), ADR-0069 |
| PR1-4 | Accepted, new `data-model.md` 5.5 | D2, D8, D16 (D1) |
| PR1-5 | Accepted | D3 (owners of `Cargo.lock`), D16 (W1, W2) |
| PR1-6 | Accepted | D16 (W2) |
| PR1-7 | Accepted | D3, D15, D16 (A2) |
| PR1-8 | Accepted through the library target | D3, D16 (S6, S7) |
| PR1-9 | User decision: library target | D1, D3, D16 (S2), ADR-0060 |
| PR1-10 | User decision: library target | D1, D2, D16 (S2), T21 |
| PR1-11 | Accepted | D3, D16 (S4) |
| PR1-12 | Accepted, task 6.4 `[hard]` | D3, D12, D16 (W4), T12 |
| PR1-13 | Accepted, test scripts in `deploy/pgtools/tests/` | D9, D11, D14 (T25), D16 (O2, O3, O4) |
| PR1-14 | Accepted | D4, T19, ADR-0058 |
| PR1-15 | Accepted. Changed later: the hook takes an `AlertCounters` clone (review of task 3.1) | D4, D16 (S2 in wave 3, A1 in wave 2) |
| PR1-16 | Accepted | D4, D6, T20 |
| PR1-17 | Orchestrator decision: two port keys | D7, D17, T22, ADR-0062 |
| PR1-18 | Accepted | D11, D14 (T3, T11, T24, T26) |
| PR1-19 | Accepted, tasks 3.10 and 7.9 split out | D16, `tasks.md` |
| PR1-20 | Accepted, dev `tower-http` | D3 |
| PR1-21 | Accepted | D3, D4 |
| PR1-22 | Accepted | D9, D16 (`tests/storage/main.rs` in each set, W2 names `src/settings/tests.rs` for the test of the default) |
| PR1-23 | Accepted, H1 in wave 0b, 1.7 to the orchestrator, `workflow_dispatch` | D11, D16 |
| PR1-24 | Accepted | D16, `tasks.md` (2.1, 6.4, 7.2) |
| PR1-25 | Accepted, `login.attempt_cleanup_interval` | D6, ADR-0060 |
| PR1-26 | Accepted | D3, T1 |
| PR1-Q6 | Orchestrator decision: shorter order, 6.5 into 6.2 | D16 (12 code waves) |
| PR2-1 | User decision: refusal of `PG*` | D1, D3, D4, D8, T4, ADR-0069, operations delta |
| PR2-2 | Accepted, also `audit_guard.rs` | D8, D15, D16 (D1) |
| PR2-3 | Accepted, row T25 | D10, D14, ADR-0066, ADR-0068 |
| PR2-4 | User decision: two networks, `cap_drop`, `no-new-privileges` | D1, D9, T23, ADR-0065, ADR-0070 |
| PR2-5 | User decision: new `pg_hba.conf`, firewall check | D1, D8, D13, ADR-0069, ADR-0070 |
| PR2-6 | Accepted | D7, D13, ADR-0062 |
| PR2-7 | Accepted | D10, D11, ADR-0066 |
| PR2-8 | Accepted | D10, ADR-0067 |
| PR2-9 | Accepted | D10, ADR-0067 |
| PR2-10 | Accepted | D7, T17, T27 |
| PR2-11 | Accepted | D9, T23 |
| PR2-12 | Accepted | D13, ADR-0064 |
| PR2-13 | Accepted, `.gitignore` at the commit | D1, D9, D11, D16 |
| PR2-14 | Accepted | D9, D10, D11 |
| PR2-15 | Accepted | D11, ADR-0065 |
| PR2-16 | Accepted in part, `search_path` keeps `public` by `.claude/rules/sql.md` | D8, D13 |
| PR2-17 | Orchestrator decision: runbook rule | D1, D13, ADR-0063 |
| PR2-18 | Accepted | D9 |
| PR2-19 | Accepted | D4, T19, ADR-0058 |
| PR2-20 | Accepted | D13 |
| PR3-1 | Accepted with PR1-3 | D8, D15 |
| PR3-2 | Accepted | D2, D16 (DC) |
| PR3-3 | Accepted | D6, T5 |
| PR3-4 | Accepted | D2, D5, web-ui delta ("Internal error response") |
| PR3-5 | User decision: layers on the health router | D1, D2, D5, T11, operations delta |
| PR3-6 | Accepted | D2, operations delta ("Least privilege of the application role") |
| PR3-7 | Accepted | D2 |
| PR3-8 | Accepted, `audit.catchup_age` 24 h | D1, D6, D17 |
| PR3-9 | Accepted, `watchdog.probe_timeout` 3 s | D1, D17, T18, operations delta |
| PR3-10 | Accepted | D16 (rules), `tasks.md` (1.4) |
| PR3-11 | Accepted | D2, D6 |
| PR3-12 | Accepted | D2, D3, D6 |
| PR3-13 | Orchestrator decision: lettered items, README row | D1, D2 |
| PR3-14 | Accepted | Non-Goals |
| PR3-15 | Accepted with PR1-25 | D6 |
| PR3-16 | Accepted | D2 |
| PR3-17 | Accepted, rows only | D1, D2, D6 |
| PR3-18 | Accepted | D4, D6, D17, T23 |
| PR3-19 | Accepted | D2 |
| PR3-20 | Accepted with PR1-23 | D16 (OR) |
| PR3-21 | Accepted | audit, web-ui, and operations deltas |
| PR3-22 | Accepted | D3, D6 |
| PR4-1 | Accepted, presence test | D9, T23 |
| PR4-2 | Accepted | D9, D17, T23 |
| PR4-3 | Accepted | D4, T19 |
| PR4-4 | Accepted | D4, D17 |
| PR4-5 | Accepted | D6, T14, T20 |
| PR4-6 | Accepted | D3, D16 (D2), T4 |
| PR4-7 | Accepted | D9, T23 |
| PR4-8 | Accepted | D9, D17, ADR-0070 |
| PR4-9 | Accepted | D10, D17 |
| PR4-10 | Accepted, no new number | D17, ADR-0064 |
| PR4-11 | Accepted, own job `musl-unwind` | D11 |
| PR4-12 | Accepted with PR3-9 | D17, operations delta |
| PR4-13 | Accepted | D6, T14, audit delta |
| PR4-14 | Accepted | T3 |
| PR4-15 | Accepted | D4, T23 |

## Risks / Trade-offs

- [The musl allocator is slow under the login flood] → ADR-0064 states the decision rule. The Compose QA runs one `oha` flood on each of two builds: musl and `cc-debian13`. It records logins per second, p99, and `memory.peak`.
- [`backup` with a private cluster runs out of 512 MiB] → the `scratch.*` settings of D17. The CI job measures the peak.
- [The own accept loop is new code on the request path] → T19 tests the drain, the cut, and keep-alive. The fallback is `axum::serve`, which ADR-0058 names.
- [A wrong `PROXY_ADDRESS` gives every client the address of `proxy`] → one variable feeds both lines (D9), and the runbook checks `client_ip` of the first login.
- [The plant has no second disk] → the healthcheck of `backup` fails, and the runbook states the RPO of 24 hours (A35).
- [A host shell sets a `PG*` variable] → `serve`, `migrate`, and `create-admin` refuse to start with a clear `ERROR` line. The runbook names the check.
- [14 waves of agents] → the waves follow the dependency order. Each closed wave is a checkpoint.
- [No image build before step 6] → the deploy agents check their files with `docker compose config` and `bash -n`. The CI job and the QA build the images.

## Migration Plan

- Forward migration `0006_operations.sql`. No rewrite of a table.
- `01-roles.sh` runs only on an empty volume. No installation exists yet. The runbook states the manual steps of ADR-0069 for an older volume.
- Rollback: `RECIPE_VERSION=<previous tag> docker compose up -d --no-deps app`. The previous image stays ready on the newer schema (ADR-0059, ADR-0082). `backup` and `walarchive` keep the new image, because the schema stays newer (ADR-0083 (a), (c)).

## Open Questions

- O1. The network `backend` has `internal: true`. Then the webhook of ADR-0063 cannot reach a host outside the Compose networks. User answer (2026-10-23, recommended): a third network `egress` with `app` as the only member. ADR-0065 records it.
