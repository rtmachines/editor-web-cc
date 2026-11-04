# Proposal

## Why

The review of 2026-11-04 found three runtime safety faults.

- D7-1 (HIGH): `log_min_duration_statement` is live while `01-roles.sh` runs. A slow `CREATE ROLE` can write a plain password into the log of `db`.
- D4-1 (MEDIUM): a cancelled `BEGIN` can return a connection with an open server transaction to the pool. The next user runs inside it.
- D2-1 (MEDIUM): when an accept loop ends before the stop signal, the process stays alive and serves nothing. The watchdog probes only the internal listener.

The change satisfies R29, R30, R34, and R37. It uses A1, A15, and A41. A15 stays unchanged at sqlx 0.8.

## What Changes

- `01-roles.sh` sets `log_min_duration_statement = -1` in its session before the first statement with a password. A runbook step finds a password in an existing log.
- An `after_release` hook of the pool asks the server through the simple query protocol. It closes a released connection with an open transaction.
- `serve` waits for the stop signal and for the end of each accept loop. When a loop ends first, the server stops, and `main` writes one `ERROR` line and returns the exit code 70. Docker then restarts the container.
- `reliability.md` section 2.4 rule 6 names two more places that catch a panic. They are the join of an accept loop and of the watchdog thread. Rule 5 and section 7.3 get the loop end as a second start of the stop.
- New ADRs: ADR-0123 (amends ADR-0031), ADR-0124 (amends ADR-0058 (m)), and ADR-0125 (amends ADR-0057 (h), ADR-0058 (l) and (o), ADR-0090 (b), and `reliability.md` section 2.3).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: three new requirements (init script log, released connection, end of an accept loop).

## Impact

- Code: `01-roles.sh`, `pool_options` and the storage test harness of `recipe-db`, `serve/` and `main.rs` of `recipe-server`.
- Documents: `data-model.md` section 5.1, `reliability.md` sections 2.3, 2.4, 7.3, and 7.5, `CLAUDE.md`, `.claude/rules/rust.md`, `docs/ops/runbook.md`, `CHANGELOG.md`.
- No migration, no new error code, no new setting in section 7.11, and no new dependency.

## Out of scope

- An upgrade to sqlx 0.9 (decision Q-A).
- A new opening of a listener after the end of its accept loop (decision Q-C).
- A rotation of passwords. The runbook step tells the operator when to rotate.
- The other findings of the review (WP2 to WP4).
