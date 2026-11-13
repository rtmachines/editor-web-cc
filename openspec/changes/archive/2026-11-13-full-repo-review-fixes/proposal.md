# Proposal

## Why

The full repository review of 2026-11-11 (`docs/plan/review/2026-11-11-full-repo-review.md`) found six faults, R-01 to R-06. Each fault is still present on `main`.

- R-01: `recipe-server` uses `saturating_*` for values that the UI does not show. `reliability.md` §2 forbids this.
- R-02: the drop test of `SecretField` passes also when the field is a plain `String`.
- R-03: ADR-0133 (d) permits calls side by side before the limit. Its consequences promise one blocking thread per path. These statements disagree.
- R-04: five test files have more than 800 lines. `SetPlcConnection::execute` has more than 50 lines.
- R-05: the doc comment of `lock_derived_types` says that the caller sorts the keys. ADR-0144 gives this work to the adapter.
- R-06: the runbook omits `ExportPlcConnection` from the commands of the read-only mode.

The change satisfies R30, R32, R34, and R40. It uses A24.

## What Changes

- R-01: checked arithmetic, a constant, or a slice concatenation replaces each `saturating_*` call in `limited_read.rs`, `credential_cipher.rs`, `create_admin.rs`, and `check_results.rs`.
- R-02: a compile-time assertion requires `zeroize::ZeroizeOnDrop` on the field of `SecretField`. A plain `String` fails the build.
- R-03: new ADR-0161 states the true bound. A path holds at most one stalled call for each pair of a job and an adapter. No production code change.
- R-04: the five test files split into modules. One step of `execute` moves into a private function. The transaction, the lock order, and the audit entry stay.
- R-05 and R-06: the doc comment follows ADR-0144. The runbook follows A24.
- Documents: ADR-0161, the Status line of ADR-0133, `docs/adr/README.md`, `reliability.md` §7.8 and §8, `docs/ops/runbook.md`, and `CHANGELOG.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the two requirements of the time limit of a read state the bound of stalled calls per path.

## Impact

- `recipe-server`: four source files, the tests of `limited_read`, and `tests/deploy_values/`.
- `recipe-web`: `pages/machines/connection_request.rs` and its test file.
- `recipe-app`: `plant/set_plc_connection.rs`, its tests, `ports/types.rs`, and the tests of the job `alerts`.
- `recipe-db`: two test files of `tests/storage/`.
- No change of a route, a migration, a setting, or an error code.

## Out of scope

- Other `saturating_*` calls outside `recipe-server`. The review did not report them.
- A shared read of one path for several jobs (option (b) of R-03).
- The open scope of section 10 of the review, such as the soak test.
- `openspec/changes/coverage-measurement-integrity`.
