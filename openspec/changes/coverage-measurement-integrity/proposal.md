# Proposal

## Why

A coverage report can omit one production module and still pass the current gate with 100 %.
Reliable measurement supports safe maintenance and data protection (R30, R31, R34).
This change uses no new assumption from `overview.md` section 9.

The audit reproduced this fault with a temporary workspace at revision `4bd52e856d6e5f2abcb9be637307e904b9a7e900`.
That experiment does not prove that the current CI report omits a module.
The gate also maps a report path to a workspace file by its last `crates/<crate>` suffix (`tools/coverage-gate.py`, lines 302 to 309).
Thus a third-party path with a `crates` directory can count as workspace code.

The owner split the original plan into two changes.
This change (A) makes the line gate account for each production file.
A later change (B) adds evidence bundles and branch results. [follow-up.md](follow-up.md) lists the scope of B.

## What Changes

- Build a source inventory from the tracked files under `crates/*/src/`, independent of the report.
- Reconcile each full LLVM JSON report with that inventory before the gate applies a threshold.
- Fail when a required production file is missing from the report or has zero lines.
- Fail when an untracked Rust file is in a source directory.
- Detect declaration-only files from their source. They need no policy entry.
- Keep exact-path policy entries only for target-gated files or files of another configuration.
- Reject duplicate entries, invalid counters, ambiguous paths, and paths outside the recorded root.
- Use one exit-code table: 0 pass, 1 coverage deficit or incomplete file accounting, 2 input or policy error.
- Show the remaining distance to 100 % lines for each crate. Keep the 80 % line floor per crate.
- Change the `Coverage` CI job: export the full JSON report, give it and the inventory to the gate, and upload the report.
- Accept ADR-0162 and add an "Amended by" note to ADR-0127.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: complete file accounting for the line coverage gate.

## Impact

- Python tools: `tools/coverage-gate.py`, the new `tools/coverage_inventory.py`, the new `tools/coverage-policy.json`, and their tests.
- CI: the existing `Coverage` job in `.github/workflows/ci.yml` and a new workflow contract test in `deploy/ci/`. The job name stays `Coverage`, because `main` requires that check.
- Docs: `CONTRIBUTING.md`, ADR-0162, the ADR index and decision map, the Status line of ADR-0127, and one sentence of `reliability.md` section 6.
- The change adds no application dependency, database migration, public API, or runtime behavior.

Roles:

- Sonnet (high effort) implements the Python tools and their tests.
- Codex implements the CI workflow task.
- Haiku does the mechanical doc edits.
- Opus reviews the code. Codex does a second review in parallel.
- The orchestrator runs the local full coverage measurement. It commits only on an explicit request of the user.

## Out of scope

- The scope of change B in [follow-up.md](follow-up.md): evidence bundles and metadata, branch results, extra CI legs, the runner `deploy/ci/coverage.py`, and the configuration `stable-default`.
- A change of the `cargo-llvm-cov` version of the `Coverage` job. It stays 0.6.24 (ADR-0127 (v)).
- Application test expansion from audit tasks T02 through T18.
- A mutation campaign or new property-based tests for application behavior.
- Immediate enforcement of 100 % on the existing application.
- Coverage claims for SQL, shell, Python, browser scripts, templates, or third-party code.
- Changes to branch protection, deployment, or the production Rust toolchain.
- Implementation before the owner approves `tasks.md`.
