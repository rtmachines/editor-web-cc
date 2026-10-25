# Proposal

## Why

The binary has no `serve` subcommand, and `deploy/` holds placeholders. The plant cannot run the editor or restore a backup. The change satisfies R04, R20, R29, R30, R31, R32, R34, R35, R37, R38, and R40.

The change uses A1, A15, A16, A18, A21, A22, A24, A33, A34, A35, A37, and A41.

## What Changes

- `recipe-server serve`: two listeners, an own accept loop with a header read timeout, a bounded drain, a fixed start order. A library target.
- `recipe-server healthcheck`: a probe for the image and the watchdog.
- Panic containment: a panic hook, two `CatchPanicLayer` instances, supervised jobs.
- Typed request and body timeouts with `E-WEB-009`.
- `/healthz` and `/readyz`. Readiness accepts a newer schema.
- The six jobs of `reliability.md` 2.4 behind ports of `recipe-app`, and the `E-EXP-002` counter.
- The startup comparison of a short key list with `SettingChanged`. The webhook URL is a secret file, and the audit holds its fingerprint.
- Read-only mode from `statvfs`, with a banner.
- Database access: refusal of `PG*` variables, TLS mode `disable`, strict `pg_hba.conf` lines, a statistics function, an index.
- `deploy/`: two images, seven services on two networks, Caddy, and the backup, restore test, and WAL archive scripts.
- A CI job `compose`, `docs/ops/runbook.md`, and ADR-0057 to ADR-0083.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: services, networks, images, health, logs, timeouts, panic, jobs, settings, backups, CI stack test.
- `audit`: the catch-up chain check, the chain break row, and the webhook fingerprint.
- `web-ui`: the read-only banner and the panic and timeout answers.
- `identity-access`: the default trusted proxy.

## Impact

- Crates: all except `recipe-export`.
- New direct dependencies: `hyper`, `hyper-util`, `tokio-util`, `hyper-rustls`, `rustls`, `rustix` (Unix), feature `env-filter`. Only `matchers` is new in `Cargo.lock`.
- Migration `0006` and a change of `01-roles.sh`.
- Changes of five documents in `docs/architecture/`, CLAUDE.md, two rule files, and four Status lines of ADRs. No new error code.

## Out of scope

- The Playwright smoke suite on the stack and the 24 h soak (P4.2, ADR-0048).
- The span name `http.request` (R2-5) and the router fallback answer (R2-3, ADR-0053 (g)): P4.2.
- TLS to `db`, a private registry, a TLS listener in `app`.
- `pg_upgrade`. A major upgrade is a dump and a restore (ADR-0020).
- The cache of static files after an upgrade (P4.2).
