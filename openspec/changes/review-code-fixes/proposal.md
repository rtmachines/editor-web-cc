# Proposal

## Why

The full repository review of 2026-11-13 (`docs/plan/review/2026-11-13-full-repo-review.md` §4) reports code faults that need no design decision. `docs/plan/review/2026-11-13-fix-plan.md` §3.1 gives them to this change. The design checks each fault at HEAD `8db04c7`.

The change satisfies R30, R32, R34, R35, R36, R38, and R39. It uses A44.

## What Changes

- F-03: a new test in `recipe-server` collects `code()` of every variant of the error types of `reliability.md` §1.1. It compares the result with the rows of §1.2.
- F-04: the 11 calls of `Uuid::new_v4` in 7 files of `recipe-app` use `random_uuid`. A failed read of random bytes gives `E-WEB-011` and no panic.
- F-05: one shared function builds the pager links of the four lists. The user list encodes `q` like the other lists. The user list also reads `page` with the reader of the other lists, so `page=01` gives `422 E-WEB-008`.
- F-08: a UDT member row with one empty input also checks its other input. The answer gives one line for each bad input.
- F-10: dead code and its stale `expect(dead_code)` attributes leave `recipe-web`.
- F-11: `FieldLines`, `Choice`, and `TokenFields` move into one shared form module.
- F-20: a panic of a connection task gives one `ERROR` line, the line of the panic hook.
- F-21: `create-admin` gives the "too long" message for each input above the limit, also when the input is not valid UTF-8 at the cut.
- F-23: the guard of `see_other_path` refuses each control character.
- F-30 to F-33: four tests become deterministic or assert the parsed value. F-30 also changes the twin test in `recipe-server` `src/probe/tests.rs`.
- `CHANGELOG.md` gets one line under `[Unreleased]` for each changed behavior (`reliability.md` §8).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the requirement "Every field error at once" gets a scenario for a half-empty UDT member row.

## Impact

- `recipe-app`: seven use case files, `identity/issue.rs`, a new source test, and `blocking/tests.rs`.
- `recipe-web`: `pages/render.rs`, the four list pages, `pages/users/query.rs`, `pages/types/define.rs`, `error/lists.rs`, `pages/parameters/grid.rs`, `pages/versions/parameter_table.rs`, `pages/versions/table.rs`, `pages/xss_coverage_tests/registry.rs`, and the importers of the moved form items.
- `recipe-server`: `listener.rs`, `create_admin.rs`, `tests/healthcheck.rs`, `src/probe/tests.rs`, and a new registry test.
- `recipe-domain`: two test files.
- `CHANGELOG.md`: the `[Unreleased]` section. Change C also adds lines there, so the second change to merge rebases this file.
- No change of a route, a migration, a setting, an error code, or a document in `docs/`.

## Out of scope

- F-09 (`session.secure`) and F-22 (a dropped `read_with_limit`). Both need a design decision. `design.md` gives the reason. Change B `review-design-fixes` carries both.
- The §1.2 row of `E-PLT-020` for `SnapshotItem::UnknownUnit`. `design.md` D1 gives the decision. Change B carries this text.
- The findings of change B (`review-design-fixes`) and change C (`review-deploy-fixes`).
