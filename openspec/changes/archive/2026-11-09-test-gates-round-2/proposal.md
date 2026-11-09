# Proposal

## Why

The repo review of 2026-11-08 found four MEDIUM gaps in the test gates. `docs/plan/2026-11-08-fix-plan.md` section 7 (WP4) holds them.

- D6-1: the only test of the refusal of `session.secure=false` runs in the job `compose`. That job never runs on a pull request (ADR-0094).
- D4-1: the rewrite check passes `ADD COLUMN` with a domain type that has a constraint. PostgreSQL 16 rewrites the table, also without a default.
- D6-3: the XSS corpus never sends a payload through the password fields of `POST /login` and `POST /me/password`.
- D6-2: the end-to-end smoke test checks the export hash, but not the exported parameter or its limits.

The change satisfies R08, R09, R21, R30, R31, R34, and R38. It uses Assumption A34.

## What Changes

- The job `test` runs `rust-tests.sh secure` on each pull request. The job `compose` drops that step.
- The new ADR-0153 amends ADR-0136 and ADR-0127 (p). The rewrite check gets the form `add column of a constrained domain`.
- The rewrite check keeps the constrained domains of all earlier files.
- The form `volatile default` also counts for an added column without a default when its domain has a volatile default.
- `data-model.md` §8 lists the new form.
- The XSS corpus sends each payload through `password` of `POST /login` and through `current_password` and `password` of `POST /me/password`.
- A guard test lists each field of each request struct with `deny_unknown_fields`. It fails on a field without a corpus test or a stated reason.
- The smoke suite checks the machine code, the reference number, the version number, and the parameter `SPEED` with its type, value, and limits.
- `CONTRIBUTING.md` lists the secure refusal check. `CHANGELOG.md` gets an `[Unreleased]` entry.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: the requirement "CI rejects a migration that locks or rewrites an existing table" gets the new form. The requirement "End-to-end smoke on the Compose stack" checks the exported content.

## Impact

- CI: `.github/workflows/ci.yml` (jobs `test` and `compose`), `deploy/ci/check.sh`.
- Scripts: `deploy/ci/migration_rewrite.py` and `deploy/ci/test_migration_rewrite_forms.py`.
- Tests: `recipe-web` (`src/pages/xss_tests.rs` and a new guard test file).
- Browser tests: `e2e/compose/end-to-end.spec.ts`, a new helper in `e2e/compose/support/`, and a new test in `e2e/tests/`.
- Documents: ADR-0153, the Status line of ADR-0136, `docs/adr/README.md`, `data-model.md` §8, `CONTRIBUTING.md`, and `CHANGELOG.md`.
- No production code, no migration, no new error code, and no new key of `serve`.

## Out of scope

- A CI step that compares the Status lines of the ADRs with their index rows (plan section 3.1 "Follow-up"). The owner decided Q3: the step is not part of WP4.
- The Soak test level (D6-6). It stays a known gap.
- A run of the job `compose` on each pull request. ADR-0094 stays.
- A check of the constraints of a domain that a migration did not create.
