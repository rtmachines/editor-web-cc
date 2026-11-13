# Follow-up: change B

The owner split the original plan of `coverage-measurement-integrity` into two changes.
Change A is this change. It covers file accounting for the line gate.
This file lists the scope of change B. Change B is a new OpenSpec change with its own proposal, design, tasks, and ADR.
Nothing in this file is normative. Change A does not implement it.

## Deferred scope

1. Evidence bundle and metadata: one directory for each configuration with `report.json`, `inventory.json`, `metadata.json`, `summary.json`, and `missing.json` (old design D4, old task group 4).
2. Metadata: revision, source digest, tool versions, command argument arrays, features, targets, and exit codes, with a fixed allowlist of environment variables.
3. Stale-evidence checks: a wrong revision, a changed source, a changed lockfile, or replaced report bytes fail validation.
4. Branch results with the statuses `unmeasured` and `not_applicable` (branch parts of old design D3, old task 1.1, old task group 5).
5. Uncovered line locations and uncovered branch outcomes in `missing.json`.
6. Extra CI legs and the runner `deploy/ci/coverage.py` with its tests (old design D5, old task group 6).
7. The configuration `stable-default`, see the review findings below.

## Toolchain decision

- The `branch-all` leg uses cargo-llvm-cov 0.9.1 and a dated nightly with rustc 1.99 or later.
- The ADR of change B overrides ADR-0127 (v) for the `branch-all` leg only.
- That ADR adds filter tests for ADR-0127 item (f) for each tool version. The default test filter of 0.9.1 can differ from 0.6.24.
- The `stable-all` gate stays on cargo-llvm-cov 0.6.24.

## Review findings for change B

1. The gate takes the `unmeasured` status from the run metadata: the `--branch` argument and the toolchain.
   - The JSON report alone cannot separate `unmeasured` from `not_applicable`. A run without `--branch` also reports zero branches.
   - Change B adds a control report without `--branch` to its fixtures.
2. Branch rows repeat for each instantiation of a function.
   - The gate merges the rows by location and outcome. It never adds the rows.
   - After the merge, the two-arm fixture gives an empty missing list. The one-arm fixture gives exactly one missing outcome.
3. Each diagnostic leg is a separate job that `main` does not require. Thus the required check name `Coverage` stays.
4. The qualification command calls an explicit binary path and checks its exact version before the run.
5. `stable-default` differs from `stable-all` only by the `dev` feature of `recipe-server`. Change B defers it as speculative.
6. A `mod` declaration with `cfg(feature = ...)` can compile in one configuration and not in another. Change B needs a rule for these declarations in each configuration.
7. Change B regenerates the fixtures and commits their JSON reports. The earlier fixtures in `/tmp` no longer exist.

## Earlier evidence

[qualification.md](qualification.md) records the branch experiments of 2026-11-11.
Its status block gives the limits of that evidence. The fixture pair does not build the workspace.
