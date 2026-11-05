# Proposal: test-gates-round-1

## Why

The review of 2026-11-04 found five gaps in the test gates of `reliability.md` §6 (D6-1, D6-7, D6-5, D6-6, D4-2).
The coverage gate counts test code as crate code.
The gate also passes a crate that is missing from the report.
The route matrix does not check `X-Request-Id` or the security headers.
The grant test does not see a grant to `PUBLIC`.
No CI step rejects a migration that rewrites a table.

Each gap lets a defect pass CI (R30, R31, R34, R38).

## What Changes

- The coverage gate counts production lines only. It finds test modules from their `#[cfg(test)]` declarations (Q-D (1)). CI pins `cargo-llvm-cov` exactly.
- Each inline `#[cfg(test)] mod name { ... }` in `src/` moves to its own file. The gate fails on a new inline test module.
- The gate fails when a crate with production files has no line in the report (D6-7). It also fails on a production file with a test name.
- `assert_want` of the route matrix also checks a version 7 `X-Request-Id` and each security header of `api.md` §9 (D6-5).
- The grant test also compares the rights of `PUBLIC` on the schema `public` and its objects with `grants.expected` (D6-6).
- A new script `deploy/ci/migration_rewrite.py` rejects the three forbidden forms of `data-model.md` §8 and each `DO` block. A pull request job runs it and its tests (D4-2).
- ADR-0127 records the decisions. The architecture documents and `CONTRIBUTING.md` follow it.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: three ADDED requirements for the coverage gate, the grant test, and the migration check.

## Out of scope

- The Load level, the end-to-end smoke suite, and the soak (WP4 `load-and-smoke`).
- A check for `ADD CONSTRAINT` without `NOT VALID`. `data-model.md` §8 names only three forms.
- The grants of other database roles, for example `recipe_backup`.
- A new target for a crate under 80 %. The estimate is at least 95 % per crate.

## Impact

- `tools/coverage-gate.py` and its new test. The CI jobs `coverage` and a new `migration-rewrite`.
- 64 inline test modules move to files. No production code changes.
- The route matrix of `recipe-web` and the grant test of `recipe-db` with `grants.expected`.
- New `deploy/ci/migration_rewrite.py` and its test.
- ADR-0127, its index, `reliability.md`, `data-model.md`, and `CONTRIBUTING.md`.
- After the merge, a branch protection of `main` requires the pull request jobs.
