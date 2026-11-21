# Proposal

## Why

The full repository review of 2026-11-21 (`docs/plan/review/2026-11-21-full-repo-review.md`) found V-02 in `deploy/`. The fix plan gives its proof to change C (`docs/plan/review/2026-11-21-fix-plan.md` §3.3).

- V-02: without `db`, `pitr-volume` checks the recovery end only to the segment. No test shows what a damaged record in the newest segment does. Change B needs this result (fix plan §4 step 4).
- Runbook line 1040 says that the script "checks the end of the archive" without `db`. The text must say what the check covers.

The change satisfies R31, R35, and R40. It uses A35.

## What Changes

- V-02 proof: the CI step `refusals` of `deploy/ci/pitr-recovery.sh` gets a fourth run. It damages one record in the newest segment of a copy of `/wal`. Then it runs `pitr-volume` without `PITR_EXPECTED_LSN`.
- The constant `EARLY_END_REFUSAL_REQUIRED` keeps CI green on both results. No run asserts the completion as right.
- The step logs each proof value. A Haiku agent copies them from the log of a dispatch into `design.md` "Proof result".
- V-02 text: `docs/ops/runbook.md` line 1040 says what the end check covers. The text depends on the proof result.
- Documents: `CHANGELOG.md` gets entries under `[Unreleased]` (`reliability.md` §8).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The proof changes a CI test and a runbook line. It changes no behavior of the product.

## Impact

- `deploy/ci/pitr-recovery.sh` and `deploy/ci/test_pitr_recovery.py`. In `.github/workflows/ci.yml`, only the name and the comment of the step `refusals` can change.
- `docs/ops/runbook.md` line 1040.
- No change of a route, a migration, a grant, a role, an error code, a key of `reliability.md` §7.11, or `docs/architecture/`. No new ADR.

## Out of scope

- The fix of V-02. If the proof shows `pitr volume completed`, change B writes an ADR that amends ADR-0106.
- The amendment of ADR-0106 (p) that names the fourth run. The ADR of change B writes it on both results.
- V-06 moves to change B. Its fix keeps a scratch directory after a failed stop. That changes the rule of ADR-0067 (g) and `reliability.md` §7.7.
- V-07 moves to change B. Its fix adds a clock rule for `backup-health` that no normative document states.
- The findings of changes A and B of the fix plan.
- A new full review pass of `deploy/db/check-*.sql` and `.github/workflows/ci.yml` (fix plan §1, "Coverage gaps").
