# Proposal

## Why

The full repository review of 2026-11-13 (`docs/plan/review/2026-11-13-full-repo-review.md`) found six faults in `deploy/` and CI, F-24 to F-29. Five faults are still present at `8db04c7`. This change fixes them.

- F-24: `01-roles.sh` accepts a NUL byte in a secret file.
- F-26: four `rust-cache` steps save a cache on each pull request.
- F-27: no pull request job checks the code with the toolchain version of the image.
- F-28: a CI test string holds the invalid escape `\$`.
- F-29: the header of `deploy/Caddyfile` names two of three differences from `Caddyfile.dev`.

F-25 is also present. The owner chose to keep the mounts. Change B `review-design-fixes` records this choice in a new ADR.

The change satisfies R31, R34, R38, and R40. It uses A1.

## What Changes

- F-24: `01-roles.sh` stops before its first change when a secret file holds a NUL byte, as `backup-once` does. Change B adds this fourth case to ADR-0031 with an "Amended by" note.
- F-26: each `rust-cache` step saves only on `main`. A new CI script test rejects a step without this condition.
- F-27: a new pull request job `release-toolchain` checks `recipe-server` with the version of `RUSTUP_TOOLCHAIN` in `deploy/Dockerfile`.
- F-28: the string uses `\\$`. `check.sh` compiles each Python file of `deploy/ci/` with warnings as errors.
- F-29: the header names all three differences. A `deploy_values` test compares the code lines of the two files.
- Documents: `CONTRIBUTING.md` names the new job. `CHANGELOG.md` gets entries under `[Unreleased]` (`reliability.md` §8).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the init script refuses a bad password file. A new requirement adds the check with the release toolchain on each pull request.

## Impact

- `deploy/db/initdb/01-roles.sh`, `deploy/Caddyfile`, `deploy/ci/check.sh`, `deploy/ci/rust-tests.sh`, `deploy/ci/test_pitr_recovery.py`, and two new tests in `deploy/ci/`.
- `.github/workflows/ci.yml`: four `rust-cache` steps and one new job.
- Tests: `recipe-db` `tests/storage/grants/init_script.rs` and `recipe-server` `tests/deploy_values/files/caddy.rs`.
- No change of a route, a migration, a grant, a role, a setting, or an error code.
- The owner adds the job `release-toolchain` to the branch protection of `main`.

## Out of scope

- F-25. Change B `review-design-fixes` writes the ADR that keeps the mounts of `db` and accepts the risk.
- The NUL check of other secret readers, such as `recipe-server`. The review did not report them.
- The findings of changes A and B of the fix plan.
- DEP-01 and the other open items of section 5 of the fix plan.
