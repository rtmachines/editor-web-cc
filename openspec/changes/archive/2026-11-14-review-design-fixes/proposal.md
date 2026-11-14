# Proposal

## Why

The full repository review of 2026-11-13 (`docs/plan/review/2026-11-13-full-repo-review.md` §4) reports findings that need a design decision. `docs/plan/review/2026-11-13-fix-plan.md` §3.2 gives them to this change. §1 of the fix plan holds the binding decisions of the owner.

The change satisfies R10, R17, R18, R20, R21, R29, R30, R31, R32, R34, R35, R36, R38, and R39. It uses A4, A17, and A33.

## What Changes

- F-01: a failed read after a committed command gives a success answer with a fixed notice. The handler logs the read fault once at `WARN`.
- F-02: the export of the approved version picks the version once more after a concurrent approval.
- F-12 and the F-03 note: `PlantErrors` and `CheckResultFault` get `code()`. `reliability.md` §1.1 and §1.2 name `SnapshotItem`.
- F-13: the documents name the second constructor `from_stored` of the stored parsers.
- F-14 and F-24: notes on the Status lines of ADR-0013 and ADR-0031.
- F-15, F-17, F-18: three corrections of `data-model.md` to the shipped code and schema.
- F-16: the audit guard function keeps its narrow `search_path` as a named exception.
- F-19: a new migration refuses an insert of a version that is not a draft (`E-RCP-037`).
- F-25: an ADR keeps the secret mounts of `db` and records the accepted risk.
- F-09: `session.secure=false` in a `dev` build removes `Secure` and the prefix `__Host-` from the cookies.
- F-06, F-07, F-22: a decision for each dropped future after a committed step. F-07 gets a detached task for the security event.
- Ten new ADRs, 0163 to 0172.
- `CHANGELOG.md` gets one line for each changed behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the notice after a failed read, the language cookie in a `dev` build, and the create form of a user.
- `export`: the second pick of the approved version.
- `identity-access`: the cookies in a `dev` build and the security event after a dropped request.
- `recipe-management`: a new version row is a draft.

## Impact

- `docs/adr/`, `docs/architecture/` (`api.md`, `data-model.md`, `reliability.md`, `domain-model.md`), `CLAUDE.md`, `.claude/rules/`.
- `recipe-domain`, `recipe-app`, `recipe-db`, `recipe-web`, `recipe-server`.
- `migrations/0016_recipe_version_insert.sql`, `crates/recipe-db/tests/grants.expected`, and `deploy/pgtools/restore-code.expected`, and `e2e/`.

## Out of scope

- The findings of change A `review-code-fixes` and change C `review-deploy-fixes`.
- DEP-01, SEC-01, and the semantic review of fix plan §5.
- A change of `deploy/` for F-25.
