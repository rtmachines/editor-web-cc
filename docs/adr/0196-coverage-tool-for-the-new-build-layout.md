# ADR-0196: cargo-llvm-cov 0.9.1 for the new build layout of Cargo

Status: Accepted. Amends ADR-0127 (f), (v), and the negative consequence of the exact pin, and ADR-0162 (s)
Date: 2026-11-19
Deciders: project owner

## Context

`rust-toolchain.toml` sets `channel = "stable"`, because `reliability.md` §8 requires the current stable toolchain. The CI runner moved from Rust 1.99.1 to Rust 1.100.0.

The `Coverage` job of CI run 44365740463 used Rust 1.99.1. The `Coverage` job of CI run 44400830778 (pull request #83) used Rust 1.100.0. The logs of the two runs show these facts:

- With Rust 1.99.1, Cargo wrote the test binaries to `target/llvm-cov-target/debug/deps/`. Each crate had more than 95 % line coverage.
- With Rust 1.100.0, Cargo wrote the test binaries to `target/llvm-cov-target/debug/build/<unit>/<hash>/out/`.
- With Rust 1.100.0, every test passed. The gate reported `recipe-app` at 62.72 %, `recipe-server` at 73.71 %, and `recipe-web` at 17.76 %.

The sources of `cargo-llvm-cov` show these facts:

- In 0.6.24, `fn walk_target_dir` in `src/main.rs` skips each directory with the name `build`. With `--include-build-script`, it skips each directory with the name `out`.
- In 0.9.1, `fn walk_target_dir` in `src/report.rs` enters the directory `out` of a build unit when `fingerprint` is next to it and `run` is not.
- In 0.9.1, `fn walk_target_dir` does not skip the directory `deps`.
- The CHANGELOG entry of 0.8.6 says "Add support for unstable new layout of Cargo's `build-dir` (`-Zbuild-dir-new-layout`)".
- The CHANGELOG entry of 0.9.0 says "Fix an issue where build scripts appeared in reports even without `--include-build-script` when Cargo's new build-dir layout is used".

The logs and the sources together indicate that 0.6.24 did not read the object files of the test binaries in the new layout. This link is an inference. The project saw the same layout with a nightly toolchain before. `openspec/changes/archive/2026-11-13-coverage-measurement-integrity/qualification.md`, section "Cause", records it with the same inference.

ADR-0127 (v) requires a comparison of the default filter of a new version with the copy of ADR-0127 (f). The `Coverage` job does not use `--remap-path-prefix`. For that case, `fn ignore_filename_regex` pushes this test filename form in 0.6.24 (`src/main.rs` line 1344) and in 0.9.1 (`src/report.rs` line 947):

```text
{SEPARATOR}rustc{SEPARATOR}([0-9a-f]+|[0-9]+\.[0-9]+\.[0-9]+){SEPARATOR}|^{workspace_root}({SEPARATOR}.*)?{SEPARATOR}(tests|examples|benches){SEPARATOR}|^{workspace_root}({SEPARATOR}.*)?{SEPARATOR}(tests\.rs|[0-9a-zA-Z_-]+[_-]tests\.rs)$
```

The form for `--remap-path-prefix` is also identical in the two versions (`src/main.rs` line 1340, `src/report.rs` line 943):

```text
(^|{SEPARATOR})(rustc{SEPARATOR}([0-9a-f]+|[0-9]+\.[0-9]+\.[0-9]+)|tests|examples|benches){SEPARATOR}|{SEPARATOR}(tests\.rs|[0-9a-zA-Z_-]+[_-]tests\.rs)$
```

A diff of `fn ignore_filename_regex` in the two versions shows two other changes of the pattern. The first change is the excluded member paths under `--remap-path-prefix`. The second change is the branch of `--dep-coverage`. The job uses neither flag and no excluded member.

## Decision drivers

- Keep the current stable toolchain of `reliability.md` §8.
- Measure the test binaries in the old and the new build layout of Cargo (R30, R34).
- Keep the exact pin of ADR-0127 (v) and the filter copy of ADR-0127 (f).
- Keep the 80 % line floor and the job name `Coverage`.

## Considered options

### Option A: Pin the toolchain to Rust 1.99.1

This breaks `reliability.md` §8. The current stable toolchain already has the new layout, so the pin only delays the same fault.

### Option B: Keep 0.6.24 and add `--include-build-script`

With this flag, 0.6.24 skips each directory with the name `out`. The test binaries of the new layout are in such a directory. The flag also adds the build scripts to the report.

### Option C: Install cargo-llvm-cov 0.9.1

The walk of 0.9.1 enters the `out` directory of a compiled unit and keeps the directory `deps`. The test filename filter does not change.

## Decision

The decision selects Option C.

- (a) The `Coverage` job installs exactly `cargo-llvm-cov@0.9.1` through `taiki-e/install-action`. This version replaces `cargo-llvm-cov@0.6.24` in ADR-0127 (v) and ADR-0162 (s).
- (b) The exact-pin rule of ADR-0127 (v) stays. A new version needs a change of `ci.yml`. The review of that change compares the default filter of the tool with the copy of ADR-0127 (f).
- (c) ADR-0127 (f) now refers to the default filter of 0.9.1. The pattern in `tools/coverage-gate.py` stays. Its doc comment names 0.6.24 and 0.9.1.
- (d) `deploy/ci/test_coverage_workflow.py` requires `cargo-llvm-cov@0.9.1` in the job. Its mutation replaces the value with `cargo-llvm-cov@0.9.0` and expects the rule `coverage-version` to fail.
- (e) The gate design of ADR-0127 and ADR-0162 stays. The job does not use `--no-default-ignore-filename-regex`, `--include-build-script`, or `--remap-path-prefix`.
- (f) `rust-toolchain.toml` keeps `channel = "stable"`.
- (g) No CI run with 0.9.1 exists at the date of this ADR. The gate of the first such run checks the fix with Rust 1.100.0.

## Consequences

### Positive

The gate measures the test binaries in the new build layout of Cargo.
The walk of 0.9.1 keeps the directory `deps`, so a toolchain with the old layout also gives a full report.
The toolchain stays on the current stable version.

### Negative

The step from 0.6.24 to 0.9.1 crosses three minor versions. Other changes of the CLI or of the JSON report can affect the job.
The exact pin `cargo-llvm-cov@0.9.1` needs a manual change for each new version of the tool.

### Risks and mitigations

Risk: a later version of Cargo changes the build layout again. Mitigation: the 80 % floor of each crate showed this fault, and it stops a merge.
Risk: 0.9.1 reads a build script as a test binary. Mitigation: the CHANGELOG entry of 0.9.0 names this fault as fixed. ADR-0162 (u) keeps `build.rs` outside the metric.

## Related

- ADR-0127, coverage gate, filter copy, and exact tool pin.
- ADR-0162, complete file accounting of the line coverage gate.
- `reliability.md` §6, testing strategy, and §8, code conventions (current stable toolchain).
- `openspec/changes/archive/2026-11-13-coverage-measurement-integrity/qualification.md`, section "Cause".
- `overview.md` §7: R30, R34.
