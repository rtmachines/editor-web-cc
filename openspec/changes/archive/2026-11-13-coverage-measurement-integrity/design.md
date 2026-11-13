# Design

## Context

See [proposal.md](proposal.md) for the scope and the observed fault.
This change (A) implements audit task T00 and the file-accounting part of audit task T01.
[follow-up.md](follow-up.md) holds the deferred scope of change B.

Normative references:

- [Testing strategy](../../../docs/architecture/reliability.md#6-testing-strategy).
- [ADR-0127](../../../docs/adr/0127-test-gates.md), items (a) through (i), (v), and (w).
- [Coverage requirements](../../specs/operations/spec.md#requirement-coverage-gate-on-production-code).
- [Contributor checks](../../../CONTRIBUTING.md#local-checks).

Current state on `main` (revision `091216d`):

- `source_scan` in `tools/coverage-gate.py` (line 282) records the production crates. It keeps no set of expected files.
- `workspace_file` (lines 302 to 309) matches the last `crates/<crate>` pair of a report path. It has no root.
- `line_counts` (line 312) sums each entry that the report supplies. It adds duplicate entries.
- `tools/test_coverage_gate.py` (line 486) covers a missing crate, but not a missing module in a reported crate.
- The `Coverage` job (`.github/workflows/ci.yml`, lines 762 to 796) exports a summary with `--summary-only`. It applies 80 % lines per crate.

The audit reproduced an omitted module in a temporary workspace. The result was exit 0 with 100 %.
This is evidence about the gate, not evidence of a missing file in the actual CI report.
The CI run [43016323332](https://github.com/rtmachines/editor-web-cc/actions/runs/43016323332) reported 36,797 covered lines of 37,668.

## Goals / Non-Goals

**Goals:** complete file accounting for the line gate, strict report input, one exit-code table, and a retained full report.

**Non-Goals:** branch results, evidence bundles, extra measurement configurations, a tool upgrade, production changes, wider application tests, and enforcement of 100 %.
A coverage percentage does not prove semantic correctness.

## Decisions

The proposed [ADR-0162](adr/0162-coverage-measurement-integrity.md) records D1 through D4.
Its status stays Proposed until the owner approves this plan.
Task 1.2 adds the accepted ADR to `docs/adr/` and updates the index and the decision map.
The number 0162 is the next free number on 2026-11-13. If another change takes 0162 first, task 1.2 uses the next free number.

### D1. Inventory the source independently

The inventory comes from the source, never from the LLVM report.

1. The gate reads the tracked files with `git ls-files -z -- crates`. It keeps each `crates/<crate>/src/**/*.rs` path.
2. The gate reads the untracked files with `git ls-files -z --others --exclude-standard -- crates`.
3. An untracked `.rs` file under `crates/*/src/` gives exit 1 and names the file.
4. Outside a Git work tree, the gate stops with exit 2.

The existing classifier of ADR-0127 (a) through (g) marks test files. It stays unchanged.
Each inventory entry has a repository-relative path, a SHA-256 of the file bytes, a crate, and a classification.

| Classification | Source | Report rule |
|---|---|---|
| `test_only` | ADR-0127 (a) through (d) | The gate removes the file before it adds lines (ADR-0127 (h)) |
| `declaration_only` | Source detection, see below | No report entry is necessary |
| `other_configuration` | Exact-path entry in `tools/coverage-policy.json` | The file must not occur in the report |
| `required` | Every other production file | One report entry with a positive line count |

A file is `declaration_only` when every item is in this list:

- `mod` declarations, and inline modules whose content follows these rules,
- `use`, `pub use`, and `extern crate` declarations, with any visibility,
- outer and inner attributes, doc comments, comments, and blank lines,
- `struct`, `enum`, and `union` definitions, and `type` aliases,
- `trait` definitions with no default body for an associated function or constant,
- `const` and `static` items whose initializer is a literal, a path, or an array of literals and paths.

A closure, a block, a call, or a macro call in an initializer makes the file `required`.
Any other item also makes the file `required`.
If the report gives a positive line count for a `declaration_only` file, the gate counts the file as `required`.
Thus the detection can never hide executable lines.
A `declaration_only` file needs no policy entry.

`tools/coverage-policy.json` holds exact-path entries for target-gated files and files of another configuration only.
The file holds `{"schema_version": 1, "entries": [...]}`. An empty list is valid. An absent file is an empty list, because then more files are `required`.

Each entry has exactly the keys `path`, `sha256`, `configuration`, `reason`, and `evidence`.
`configuration` names the configuration that compiles the file. `evidence` names the source line that gates the file, for example a `cfg` attribute.
These policy faults give exit 2:

- a wildcard, a duplicate path, a stale hash, or an empty field,
- an entry for a `declaration_only` or `test_only` file,
- an unused entry: its file is absent from the inventory or occurs in the report.

The gate never creates an entry from a missing report entry.
An `other_configuration` file has no coverage claim. The summary lists it with its configuration.

The inventory holds the top-level list `build_scripts` with the tracked `crates/<crate>/build.rs` files. The summary lists them as build-time code outside the runtime metric.
SQL, shell, Python, JavaScript, templates, and vendored dependencies stay outside this Rust metric.
ADR-0127 notes that test-only items inside a production file count as production lines. This limitation stays.

### D2. Reject incomplete or inconsistent reports

The gate reads the full LLVM JSON export, not a summary. It checks the input before it adds any counter.

**Root.** The gate resolves each report path against one root:

- With `--inventory`, the root is the `root` field of the inventory file.
- Without `--inventory`, the root is the directory of `cargo_llvm_cov.manifest_path` when the report has that field.
- Else the root is the current directory.
- With `--inventory`, a `manifest_path` directory that differs from the inventory root gives exit 2.
- In the legacy form, the gate builds the inventory in the current directory. Thus a `manifest_path` directory that differs from the current directory gives exit 2.
- With `--inventory`, the gate rebuilds the inventory in the current directory. An inventory file that differs from it, also in its `root` field, gives exit 2.
- A present `cargo_llvm_cov` key whose value is not an object (also `null`) gives exit 2. An empty or non-text `manifest_path` also gives exit 2.

**Path mapping.**

- The gate accepts an absolute path inside the root and a relative path without `..`. It keeps Windows drive letters and separators.
- The gate never matches a `crates` suffix. A path outside the root is third-party code. The summary lists its line count and does not add it.
- A path inside the root and outside `crates/*/src/` is outside the metric. The summary lists it and does not add it.
- A path under `crates/*/src/` that is not in the inventory gives exit 2.
- A path with a `..` segment, and two entries for one normalised path, give exit 2.
- The root sets the path style. A root with a drive letter or a backslash uses Windows rules. Other roots use POSIX rules.
- A path is under `crates/*/src/` when its first part is `crates` and its third part is `src`.
- An untracked source file is not in the inventory. Thus a report entry for it gives exit 2. Without a report entry, it gives exit 1 (D1).
- A relative path with a drive or a root anchor, and an empty path, give exit 2.
- The duplicate rule applies to every entry: source, outside-metric, and third-party entries.

**Report shape.**

- The report must have exactly one export object in `data`. Another count gives exit 2.
- Each line counter must be an `int` (not a Boolean) with `0 <= covered <= count`. Another value gives exit 2.
- A missing key or a wrong type gives exit 2.
- A `float` counter gives exit 2, also the value `10.0`.
- One export object with an empty `files` list is valid. A crate without production files then shows `no code`.
- A `count` of 0 is a valid counter. The accounting rule then gives exit 1 for a `required` file.
- The gate checks the shape, then the paths, then the duplicates. It adds counters only after the three checks pass.

**Inventory file.** The gate checks the type of each inventory entry before it uses the entry.
A file item has a str `path`, `crate`, `classification`, and `sha256`, and an optional str `configuration`.
The items of `untracked` and `build_scripts` are strings.
A bad entry gives exit 2.

**Accounting.** Each `required` file needs one report entry with a positive line count.
A missing `required` file or a `required` file with zero lines gives exit 1 and names the path.

**Exit codes.** The gate uses one table in its docstring, the spec, the tests, and `CONTRIBUTING.md`:

| Exit | Meaning | Examples |
|---|---|---|
| 0 | Every check passes | Complete accounting and at least 80 % lines in each crate |
| 1 | Coverage deficit or incomplete file accounting | A crate below 80 %, or a missing or zero-line `required` file. An untracked source file without a report entry, or a source rule of ADR-0127 (e) or (f) |
| 2 | Input or policy error | Unreadable or malformed report, bad counter, bad path, duplicate entry, a report entry for an untracked file. Stale inventory, bad inventory entry, policy fault, `#[path]` (ADR-0127 (g)), bad arguments, no Git work tree |

When the gate finds an input or policy error, it stops before it computes a percentage. Thus exit 2 takes precedence over exit 1.
The source rules of ADR-0127 (e) and (f) keep exit 1, because each one makes the production line count unreliable.

### D3. Keep the line floor and show the distance to 100 %

The 80 % line floor per crate of `reliability.md` section 6 stays.
The gate adds the lines of the `required` files of each crate once.
For each crate, the summary shows covered lines, total lines, the percentage, and the uncovered lines to 100 %.
For each `required` file with uncovered lines, the summary shows the path and the uncovered line count.
The summary also lists `declaration_only` paths, outside-metric and third-party paths, and build-time code.
It lists each `other_configuration` file as `<path> (<configuration>)`.
The 100 % line goal stays a separate, unmet goal. No result claims that goal while the accounting is incomplete.

The crate table has the columns crate, covered, lines, percent, uncovered, and result.
A crate at or above 80 % shows `ok`. A crate below 80 % shows `BELOW TARGET`.
A crate at or above 80 % with a missing or zero-line `required` file shows `incomplete`, and the gate exits 1.
The lists follow the table. The gate omits an empty list. A source rule violation keeps exit 1 and the table.
Uncovered line locations and branch results belong to change B.

### D4. Change one step of the `Coverage` job

The job keeps the name `Coverage`, its 45-minute timeout, and `cargo-llvm-cov@0.6.24` (ADR-0127 (v)).

1. Run the five coverage test files `tools/test_coverage_*.py` with `python3 -I -B` (extends ADR-0127 (w)).
2. Write the inventory: `python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json`.
3. Measure: `cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json`.
4. Apply the gate: `python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json`.
5. Upload the report and the inventory with `actions/upload-artifact`, pinned by its commit SHA.

The gate rebuilds the inventory in step 4. A difference from the inventory file gives exit 2. Thus a source change during the measurement fails.

The upload step has `if: always()`. No other step gets `if: always()`, `continue-on-error`, or `--ignore-run-fail`.

The artifact name holds the revision, for example `coverage-report-${{ github.sha }}`. The artifact has `retention-days: 14`.
The upload uses `if-no-files-found: warn`, because a failed measurement can leave no report. The gate step already fails in that case.

A contract test in `deploy/ci/test_coverage_workflow.py` reads `ci.yml` and checks these rules. `deploy/ci/check.sh` runs it.

### D5. Keep file ownership and interfaces explicit

| File | Responsibility |
|---|---|
| `tools/coverage-gate.py` | Existing classifier, path root, report checks, line results, CLI |
| `tools/coverage_inventory.py` | Git file lists, declaration-only detection, policy checks, inventory file |
| `tools/coverage-policy.json` | Reviewed exact-path entries for other configurations |
| `tools/test_coverage_gate.py` | Existing regressions, omitted-module regression, root and path tests |
| `tools/test_coverage_inventory.py` | Inventory, declaration-only, and policy tests |
| `tools/test_coverage_report.py` | Report shape, counter, and duplicate tests |
| `tools/test_coverage_summary.py` | Line floor, summary, and exit precedence tests |
| `tools/test_coverage_cli.py` | CLI form, inventory file, and argument tests |
| `deploy/ci/test_coverage_workflow.py` | Contract test of the `Coverage` job |
| `.github/workflows/ci.yml` | Full report, inventory step, gate arguments, artifact upload |
| `CONTRIBUTING.md` | Commands, exit codes, policy review, artifact |

The tools use Python standard-library modules only.
The gate loads `coverage_inventory.py` by an explicit file path, because `python3 -I` removes the script directory from `sys.path`.
Each function has at most 50 lines. Each file has at most 800 lines.

```python
source_inventory(root: Path, policy: list[dict], classify: Callable | None = None) -> dict
validate_report(report: dict, inventory: dict) -> dict
```

The tokenizer and the declaration-only detection live in `coverage_inventory.py`. The ADR-0127 classifier stays in the gate.
The gate passes `classify`, a callback that returns the `test_only` subset of the tracked paths. Thus the two files do not load each other.
The inventory also holds the key `untracked`: the untracked source files. Each one gives exit 1.

The inventory and the result dictionaries use schema version 1.
An input or policy error raises `ValueError`. The CLI maps it to exit 2.
A coverage deficit or incomplete accounting gives a status value. The CLI maps it to exit 1.

The CLI keeps two existing forms and adds two:

- `coverage-gate.py <report>`: the legacy form. It builds the inventory itself and uses the root rule of D2.
- `coverage-gate.py --source-only`: the source rules without a report. It applies D1 without the report rules.
- `coverage-gate.py --write-inventory <path>`: writes the inventory file and creates its missing parent directory.
- `coverage-gate.py --report <report> --inventory <path>`: the CI form.

The inventory file is JSON with sorted keys, an indent of 2, and a final newline. Two writes of one tree give identical bytes.
`--write-inventory` exits 0 also when the inventory holds untracked files. The `--report` form then gives exit 1.
Bad arguments, and a mix of two forms, give exit 2 with the usage text.

## Risks / Trade-offs

- A handwritten classifier can miss Rust syntax. The declaration-only detection treats an unknown form as `required`. A positive report count overrides it.
- Two functions carry `#[cfg(not(unix))]`: `crates/recipe-server/src/disk.rs` line 99 and `crates/recipe-server/src/serve/process.rs` line 170. The measured Linux build does not compile them. A file-level policy entry cannot express an item-level gate. ADR-0162 records this as a known limit, and the gate does nothing for these two functions.
- A policy entry can hide code. Exact paths and source hashes force a review after each source change.
- The full report is larger than the summary. Task 1.1 measures its size. The artifact keeps it for 14 days.
- The Git file lists need a Git work tree. The gate stops with exit 2 without one.
- A coverage target can encourage weak tests. This change tests the measurement only.

## Migration Plan

1. Measure the full report locally and review the source (task 1.1).
2. Accept the ADR (task 1.2).
3. Add failing tests, then the inventory, the policy, and the report checks (groups 2 and 3).
4. Change the `Coverage` job and the contributor docs (group 4).
5. Review and run the checks (group 5).

This change needs no database or deployment migration.
A rollback restores the old gate and the old job. It must not describe the old gate as complete.

## Open Questions

Task 1.1 finds the declaration-only files, the target-gated files, and the size of the full report.
If a file needs a policy entry, task 2.4 adds it with source evidence.
If the full report does not fit the runner disk or the 45-minute budget, task 1.1 stops and reports to the owner.
