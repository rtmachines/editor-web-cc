# Coverage Measurement Integrity Implementation Plan (change A)

**Goal:** The line coverage gate accounts for each production file and rejects input that can inflate a total.

**Architecture:** Extend the existing Python gate with an independent source inventory, a policy file, and strict report checks.
Change one step of the existing `Coverage` job.

**Tech Stack:** Python standard library, cargo-llvm-cov 0.6.24, GitHub Actions.

**Spec:** [operations delta](specs/operations/spec.md), [design](design.md), [proposed ADR](adr/0162-coverage-measurement-integrity.md).
Change B: [follow-up.md](follow-up.md).

## Global Constraints

- All tasks stay unchecked during planning. The owner approves these tasks before implementation starts.
- The cycle is `openspec-apply-change` with `superpowers:test-driven-development`.
- After the code tasks, `code-review` runs on Opus, with a parallel Codex review.
- Each code task starts with a failing behavioral test and names its area. The implementer runs the test and sees the failure first.
- Each task takes at most 2 h. A larger task gets a split before execution.
- One subagent gets one task.
- No task changes production behavior, migrations, or application test expectations.
- The 80 % line floor per crate stays. The 100 % line goal stays a separate, unmet goal.
- The job name `Coverage` and `cargo-llvm-cov@0.6.24` stay.
- The exit-code table of design D2 applies to every task: 0 pass, 1 coverage deficit or incomplete file accounting, 2 input or policy error.
- The orchestrator commits only on an explicit request of the user.

Models:

- Sonnet (high effort) implements the Python tools and their tests.
- Codex implements the CI workflow task and its contract test.
- Haiku does the mechanical doc edits. A Sonnet agent runs the STE checks of docs.
- Opus reviews the code and the source classification.
- The orchestrator runs Cargo with Docker.

## 1. Source review and ADR

- [x] 1.1 (measurement, orchestrator and Opus) Measure the full report and review the source.
  - The orchestrator runs `cargo llvm-cov --workspace --all-features --locked --json --output-path <scratch>/llvm-cov-report.json` locally. Docker works on this host.
  - The orchestrator records the exit code, the duration, the report size, and `cargo_llvm_cov.manifest_path`.
  - An Opus agent lists each production file that the report omits or reports with zero lines.
  - For each such file, the agent records the source-backed class: declaration-only, other configuration, or required.
  - The agent records the evidence in `source-review.md` in this change directory. It creates no policy entry from report absence alone.
  - Stop and report to the owner if the run fails or the report size can exceed the runner disk.
  - Estimate: 2 h, plus the run time.
- [x] 1.2 (docs/adr, Haiku, [mechanical]) Accept the ADR.
  - Copy the ADR draft to `docs/adr/0162-coverage-measurement-integrity.md` with the status Accepted, or to the next free number.
  - Add the index row and update the "Test gates" row of the decision map in `docs/adr/README.md`.
  - Change the Status line of ADR-0127 only: add "Amended by ADR-0162 (coverage file accounting)".
  - Add one sentence to `reliability.md` section 6: the gate reconciles the report with a source inventory first (ADR-0162).
  - A Sonnet agent runs `tools/ste-lint/ste-lint.py` on the changed files and `openspec validate coverage-measurement-integrity --strict`.
  - Estimate: 1 h.

Dependencies: 1.2 needs 1.1, because the review can change a decision of the ADR.

## 2. Independent inventory and policy

Dependencies: 1.1 and 1.2.
Files: `tools/coverage-gate.py`, `tools/coverage_inventory.py`, `tools/coverage-policy.json`, `tools/test_coverage_gate.py`, `tools/test_coverage_inventory.py`.

- [x] 2.1 (tools, Sonnet high) Omitted module in a present crate.
  - Add `test_missing_module_in_present_crate_fails` to `tools/test_coverage_gate.py`. The fixture has `lib.rs` and executable `risk.rs`. The report holds `lib.rs` only.
  - Add a test for a required file with a report line count of 0.
  - Run the tests and see exit 0 from the current gate.
  - Keep the set of required files and reconcile it with the report paths. Require exit 1 and the path.
  - Keep the missing-crate regression.
  - Check: `python3 -I -B tools/test_coverage_gate.py`. Estimate: 2 h.
- [x] 2.2 (tools, Sonnet high) Tracked and untracked source files.
  - Add tests with a temporary Git repository. A new tracked file without a report entry gives exit 1.
  - An untracked `.rs` file under `src/` gives exit 1. A directory outside Git gives exit 2.
  - Implement `source_inventory` in `tools/coverage_inventory.py` with `git ls-files`. Load the module by an explicit file path.
  - Check: `python3 -I -B tools/test_coverage_inventory.py` and `python3 -I -B tools/test_coverage_gate.py`. Estimate: 2 h.
- [x] 2.3 (tools, Sonnet high) Declaration-only detection.
  - Add one test for each item kind of the D1 list. A file with only that kind is declaration-only and needs no report entry.
  - The kinds are `mod`, inline `mod`, `use`, `pub use`, `extern crate`, attributes, and comments.
  - The kinds also include `struct`, `enum`, `union`, `type`, and `trait` without default bodies.
  - The last kinds are `const` and `static` with a literal, a path, or an array.
  - Add tests: a closure, a block, a call, or a macro call in an initializer makes the file required. A trait with a default body makes the file required.
  - Add tests: the gate classifies a file with a function as required. A declaration-only file with a positive report line count counts as required (report override).
  - Add a test: an unknown form makes a file required.
  - Keep all existing classifier regressions of ADR-0127 (a) through (g).
  - Check: both coverage test files. Estimate: 2 h.
- [x] 2.4 (tools, Sonnet high) Policy file.
  - Add a test for a valid entry for another configuration.
  - Add fault tests: a stale hash, a duplicate path, a wildcard, an unused entry, an empty field, and an entry for a declaration-only file.
  - Each fault gives exit 2.
  - Implement the policy checks of design D1. Create `tools/coverage-policy.json` with the entries of `source-review.md` only. An empty list is valid.
  - Check: both coverage test files and `python3 -I -B tools/coverage-gate.py --source-only`. Estimate: 2 h.

Acceptance: each source file has a current classification, and no required file leaves the denominator silently.

## 3. Report checks and line results

Dependencies: group 2.
Files: `tools/coverage-gate.py`, `tools/test_coverage_gate.py`, `tools/test_coverage_report.py`, `tools/test_coverage_summary.py`, `tools/test_coverage_cli.py`.
The test files share helpers. No file has more than 800 lines, so tasks 3.2 to 3.4 put their tests in new files.

- [x] 3.1 (tools, Sonnet high) Root and path mapping.
  - Add a test with a third-party path outside the root that ends with `crates/<crate>/src/lib.rs`. See the current gate count its lines.
  - Add root tests: the legacy root from `cargo_llvm_cov.manifest_path`, and the current directory without that field.
  - Add path tests: Windows drive paths, a `..` segment, an unknown path under `crates/*/src/`, and a root that differs from the inventory root.
  - Replace the suffix match of `workspace_file` with the root rule of design D2.
  - Check: both coverage test files. Estimate: 2 h.
- [x] 3.2 (tools, Sonnet high) Report shape and counters.
  - Add tests: a duplicate normalised path (see the current gate add both counters), two export objects, and missing keys.
  - Add counter tests: Boolean, negative, fraction, non-finite, and covered above count.
  - Add a test where a duplicate covered entry cannot hide an uncovered file.
  - Make the gate check the input before it adds any counter. Each fault gives exit 2.
  - Check: both coverage test files. Estimate: 2 h.
- [x] 3.3 (tools, Sonnet high) Line results and exit precedence.
  - Add fixtures with exactly 80 % lines and just below 80 %. Assert literal counters from the fixture source.
  - Add tests: the summary shows the uncovered lines of each crate and of each required file, and lists declaration-only, other-configuration, and third-party paths.
  - Add a test where an input error and a coverage deficit occur together. Require exit 2.
  - Update the module docstring with the exit-code table.
  - Check: both coverage test files. Estimate: 2 h.
- [x] 3.4 (tools, Sonnet high) CLI forms.
  - Add tests for `--write-inventory <path>` and for `--report <report> --inventory <path>`.
  - Add tests: an inventory file that differs from the rebuilt inventory gives exit 2. The legacy `<report>` form and `--source-only` keep their results.
  - Implement the CLI forms of design D5.
  - Check: both coverage test files. Estimate: 1.5 h.

Oracle: fixture source and literal counter values, not totals that the gate under test computes.
Plausible bugs: a sum of duplicate entries, a suffix path match, and a default of zero for a missing counter.

## 4. CI job and contributor docs

Dependencies: group 3.

- [x] 4.1 (.github/workflows and deploy/ci, Codex) The `Coverage` job.
  - Add `deploy/ci/test_coverage_workflow.py` first. It reads `ci.yml` and checks the rules of design D4. Run it and see it fail.
  - The rules: the job name `Coverage`, no `--summary-only`, and the gate with `--report` and `--inventory`.
  - More rules: the inventory step and the five coverage test files `tools/test_coverage_*.py` run before the measurement.
  - More rules: `upload-artifact` pinned by its SHA, `retention-days: 14`, the revision in the artifact name, and `if: always()` on the upload step only.
  - More rules: no `continue-on-error` and no `--ignore-run-fail` in the job.
  - Change the job. Reuse the SHA of `actions/upload-artifact` that `ci.yml` already pins.
  - Check: `python3 -I -B deploy/ci/test_coverage_workflow.py` and `bash deploy/ci/check.sh`. Estimate: 2 h.
- [x] 4.2 (CONTRIBUTING.md, Haiku, [mechanical]) Contributor docs.
  - Replace the summary commands with the commands of design D4.
  - Add the exit-code table, the rules of a policy entry, and the name and retention of the artifact.
  - A Sonnet agent runs `tools/ste-lint/ste-lint.py CONTRIBUTING.md`. Estimate: 1 h.

## 5. Review and verification

Dependencies: groups 1 through 4.

- [x] 5.1 (review, Opus and Codex) Review the diff with `code-review` on Opus and a parallel Codex review.
  - Check that each scenario of the spec delta has a passing behavioral test.
  - A Sonnet agent fixes each CRITICAL and HIGH finding, one finding per agent. Estimate: 2 h, plus fix rounds.
- [x] 5.2 (verification, orchestrator) Run the checks with `superpowers:verification-before-completion`.
  - Run the full local measurement of task 1.1 again. Apply the gate with `--report` and `--inventory`. Record the result in `source-review.md`.
  - Run the commands of section 6. Show their output before a claim of success.
  - Report the CI result of the `Coverage` job and the artifact name to the owner. Estimate: 1.5 h, plus the run time.

## 6. Commands

```bash
python3 -I -B tools/test_coverage_gate.py
python3 -I -B tools/test_coverage_inventory.py
python3 -I -B tools/test_coverage_report.py
python3 -I -B tools/test_coverage_summary.py
python3 -I -B tools/test_coverage_cli.py
python3 -I -B tools/coverage-gate.py --source-only
python3 -I -B deploy/ci/test_coverage_workflow.py
bash deploy/ci/check.sh

# Local full measurement, with Docker, by the orchestrator.
python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json
cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json
python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json

# Repository checks.
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
openspec validate coverage-measurement-integrity --strict
bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'
```

## 7. Traceability and effort

| Spec requirement | Tasks |
|---|---|
| Coverage report accounts for each production file | 1.1, 2.1, 2.2, 2.3 |
| Coverage exceptions have reviewable evidence | 1.1, 2.3, 2.4 |
| Coverage input cannot inflate totals silently | 3.1, 3.2, 3.4 |
| Coverage results show the distance to full line coverage | 3.3, 4.1, 4.2 |

Estimated effort: 25 h for 14 tasks (1.1 to 5.2).
The estimate excludes the fix rounds of the review, the local run time, and one CI run of the `Coverage` job.
This estimate is not a test runtime measurement.
No task claims that the application reaches 100 % during this change.
