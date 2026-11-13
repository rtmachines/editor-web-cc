# ADR-0162: Complete file accounting for the line coverage gate

Status: Accepted. Amends ADR-0127 (h), (i), and (w)
Date: 2026-11-13
Deciders: project owner

## Context

The current gate rejects a missing crate but can accept a missing production module within a reported crate.
The audit reproduced that case at revision `4bd52e856d6e5f2abcb9be637307e904b9a7e900`.

The gate maps a report path by its last `crates/<crate>` suffix. Thus a third-party path can count as workspace code.
The gate adds duplicate report entries. The `Coverage` job exports only a summary and keeps no report.

The owner split the work. This ADR covers the line gate. A later ADR covers evidence bundles and branch results.

## Decision drivers

- Detect incomplete accounting before the gate trusts a percentage (R30, R34).
- Keep the measurement scope visible (R31).
- Reject input that can inflate a total (R38).
- Keep the 80 % line floor, the tool version, and the required check name `Coverage`.
- Use behavioral tests for the measurement tools.

## Considered options

### Option A: Increase the existing percentage threshold only

This does not detect an omitted module or a foreign path.

### Option B: Derive expected files from the coverage report

This repeats the omission in both the report and its supposed inventory.

### Option C: Reconcile an independent source inventory with the full report

This detects omissions and keeps explicit scope decisions.
It needs a reviewed policy file for files of another configuration.

## Decision

The decision selects Option C.

- (a) The gate builds the source inventory from the tracked files under `crates/*/src/`. The inventory does not depend on the report.
- (b) An untracked `.rs` file under `crates/*/src/` fails the gate with exit 1. A report entry for such a file is an input error with exit 2.
- (c) Each required production file needs one report entry with a positive line count. A missing file or a file with zero lines fails with exit 1.
- (d) The gate detects a declaration-only file from its source. Every item of such a file is in this list.
  - `mod` declarations, and inline modules whose content follows these rules.
  - `use`, `pub use`, and `extern crate` declarations, with any visibility.
  - Outer and inner attributes, doc comments, comments, and blank lines.
  - `struct`, `enum`, and `union` definitions, and `type` aliases.
  - `trait` definitions with no default body for an associated function or constant.
  - `const` and `static` items whose initializer is a literal, a path, or an array of literals and paths.

  A closure, a block, a call, or a macro call in an initializer makes the file required. A call or a macro call in an enum discriminant or an array length also makes the file required. Any other item also makes the file required.
- (e) A declaration-only file needs no policy entry. A positive report line count makes it a required file.
- (f) `tools/coverage-policy.json` holds exact-path entries only for target-gated files and files of another configuration.
- (g) Each policy entry has a path, a source hash, a configuration, a reason, and evidence.
- (h) A wildcard, duplicate, stale, unused, or incomplete policy entry fails with exit 2. A missing report entry never creates a policy entry.
- (i) The gate resolves report paths against one root. With an inventory file, the root is the root of that file.
- (j) Without an inventory file, the root is the directory of `cargo_llvm_cov.manifest_path`, else the current directory. The gate never matches a path suffix.
- (k) A path outside the root is third-party code. The gate lists it and does not count it.
- (l) A duplicate entry, a bad counter, a path traversal, or an unknown path under `crates/*/src/` fails with exit 2.
- (m) A report with more than one export object fails with exit 2.
- (n) Exit 0 means pass. Exit 1 means a coverage deficit or incomplete file accounting. Exit 2 means an input or policy error.
- (o) The 80 % line floor per crate stays. The summary shows the uncovered lines of each crate to the separate goal of 100 %.
- (p) The `Coverage` job exports the full JSON report and gives the gate the report and the inventory file.
- (q) The job uploads the report with a pinned `upload-artifact` for 14 days, with the revision in the name. Only the upload step runs after a failure.
- (r) The job runs the tests of the gate and of the inventory before the measurement.
- (s) The job name `Coverage` and the tool version `cargo-llvm-cov@0.6.24` stay.
- (t) Known limit: two functions carry `#[cfg(not(unix))]`. They are in `crates/recipe-server/src/disk.rs` at line 99 and `crates/recipe-server/src/serve/process.rs` at line 170. The deployment target is Linux, so the measured build does not compile them. A file-level policy entry cannot express an item-level gate. The gate does nothing for them.
- (u) The inventory lists the tracked `crates/<crate>/build.rs` files as build-time code outside the metric. The summary lists them. Their lines never count.

## Consequences

### Positive

A missing module or a foreign path cannot produce a passing line result.
Reviewers can reproduce a CI result from the retained report.

### Negative

A source change can need a review of a policy entry.
The full report needs more disk and artifact storage than the summary.
The gate needs a Git work tree.

### Risks and mitigations

The source classifier cannot prove all Rust configuration semantics.
An unknown form makes a file required, and a positive report count overrides the declaration-only result.

## Related

- ADR-0127, coverage accounting and tool pin.
- `reliability.md` section 6, testing strategy.
- `overview.md` section 7: R30, R31, R34, R38.
