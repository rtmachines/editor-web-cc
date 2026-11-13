## ADDED Requirements

### Requirement: Coverage report accounts for each production file

The coverage gate SHALL build its source inventory from the tracked files under `crates/*/src/`, not from the report. Each required production file SHALL have one report entry with a positive line count. A missing or zero-line required file SHALL fail the gate with exit 1 before the percentage check. An untracked Rust file under `crates/*/src/` SHALL also fail with exit 1. A report entry for an untracked Rust file SHALL fail with exit 2 (R30, R34).

#### Scenario: One module is absent from a passing crate report
- **GIVEN** a crate with the executable production modules `lib.rs` and `risk.rs`
- **AND** a report that covers every line of `lib.rs` and holds no entry for `risk.rs`
- **WHEN** the coverage gate reads the report
- **THEN** the gate exits with code 1 and names `risk.rs`
- **AND** the gate does not show the crate as complete

#### Scenario: A tracked production file is missing from the report
- **GIVEN** a new tracked production file `crates/recipe-export/src/extra.rs` with an executable function
- **AND** a report that holds every other production file and no entry for `extra.rs`
- **WHEN** the coverage gate reads the report
- **THEN** the gate exits with code 1 and names `crates/recipe-export/src/extra.rs`

#### Scenario: A required file has zero lines
- **GIVEN** a required production file with an executable function
- **AND** a report entry for that file with a line count of 0
- **WHEN** the coverage gate reads the report
- **THEN** the gate exits with code 1 and names the file

#### Scenario: An untracked source file exists
- **GIVEN** an untracked file `crates/recipe-app/src/draft.rs` that Git does not ignore
- **AND** the report holds no entry for that file
- **WHEN** the coverage gate builds the source inventory
- **THEN** the gate exits with code 1 and names `crates/recipe-app/src/draft.rs`

#### Scenario: A report names an untracked file
- **GIVEN** an untracked file `crates/recipe-app/src/draft.rs` that Git does not ignore
- **AND** a report entry for `crates/recipe-app/src/draft.rs` with a positive line count
- **WHEN** the coverage gate reads the report
- **THEN** the gate exits with code 2
- **AND** it names `crates/recipe-app/src/draft.rs` as untracked
- **AND** it prints no percentage

### Requirement: Coverage exceptions have reviewable evidence

The coverage gate SHALL detect a declaration-only file from its source and SHALL NOT need a policy entry for it. A policy entry SHALL name an exact path, source hash, configuration, reason, and evidence, and SHALL apply only to a file of another configuration. A missing report entry alone SHALL NOT justify an entry. A stale, unused, duplicate, or wildcard entry SHALL fail the gate with exit 2 (R31, R34).

#### Scenario: A declaration module has no executable code
- **GIVEN** a module file that holds only `mod` and `pub use` declarations and doc comments
- **AND** no policy entry and no report entry for that file
- **WHEN** the coverage gate reconciles the inventory
- **THEN** it lists the file as declaration-only
- **AND** it adds no covered line and no total line for the file

#### Scenario: An executable function changes a declaration module
- **GIVEN** a declaration-only module file
- **WHEN** a change adds an executable function to the file
- **AND** the report holds no entry for the file
- **THEN** the gate classifies the file as required
- **AND** the gate exits with code 1 and names the file

#### Scenario: A module belongs to another target
- **GIVEN** a policy entry with the current source hash of a target-gated module
- **AND** a report without an entry for that module
- **WHEN** the coverage gate reconciles the inventory
- **THEN** the summary names the module and the configuration of the entry
- **AND** the summary makes no coverage claim for that module

#### Scenario: A policy entry has a stale hash
- **GIVEN** a policy entry with a source hash that differs from the current file
- **WHEN** the coverage gate reads the policy
- **THEN** the gate exits with code 2 and names the entry

### Requirement: Coverage input cannot inflate totals silently

The coverage gate SHALL resolve report paths against one recorded root and SHALL NOT match a path suffix. A duplicate entry, an invalid counter, a path traversal, or an unknown path under `crates/*/src/` SHALL fail with exit 2. A report with more than one export object SHALL also fail with exit 2. The gate SHALL list third-party paths and SHALL NOT count them (R30, R38).

#### Scenario: A file occurs twice
- **GIVEN** two report entries for the same normalised workspace path
- **WHEN** the gate reads the report
- **THEN** it exits with code 2 and does not add both counters

#### Scenario: A third-party path contains a crates directory
- **GIVEN** a report path outside the root that ends with `crates/recipe-domain/src/lib.rs`
- **WHEN** the gate resolves the path against the root
- **THEN** it lists the path as third-party code
- **AND** it does not add the lines of that path to `recipe-domain`

#### Scenario: A legacy report has no inventory file
- **GIVEN** a report with `cargo_llvm_cov.manifest_path` and no inventory file
- **WHEN** the gate reads the report
- **THEN** it uses the directory of `manifest_path` as the root

#### Scenario: Counters exceed their valid range
- **GIVEN** a Boolean, negative, non-integer, or non-finite counter, or a covered count above its total
- **WHEN** the gate reads the report
- **THEN** it exits with code 2 without a percentage

### Requirement: Coverage results show the distance to full line coverage

The coverage gate SHALL keep the 80 % line floor per crate. It SHALL show the uncovered lines of each crate to the separate goal of 100 % lines. The `Coverage` CI job SHALL give the gate the full JSON report and the inventory. The job SHALL keep the report as an artifact for 14 days, also after a failed step (R30, R34).

#### Scenario: Valid measurement stays below the goal
- **GIVEN** a complete measurement with 90 % lines in each crate
- **WHEN** CI applies the gate
- **THEN** the gate exits with code 0
- **AND** the summary shows the uncovered lines of each crate without a claim that the goal is complete

#### Scenario: A crate is below the floor
- **GIVEN** a complete measurement with one crate at 79 % lines
- **WHEN** CI applies the gate
- **THEN** the gate exits with code 1 and names the crate

#### Scenario: The gate fails in CI
- **GIVEN** a `Coverage` job with a gate step that exits with a nonzero code
- **WHEN** the job ends
- **THEN** the job fails
- **AND** the artifact holds the full report of that revision
