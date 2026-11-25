## ADDED Requirements

### Requirement: Allocator of the musl build
The binary of `recipe-server` SHALL use `mimalloc` as its global allocator with its default options
on each target with `target_env = "musl"`. The target of the shipped image is
`x86_64-unknown-linux-musl`. A build of the binary for another C library SHALL keep the system
allocator. Only `recipe-server` SHALL depend on `mimalloc`. No file of `deploy/` SHALL set or pass
an environment variable whose name starts with `MIMALLOC_` in any letter case. A failed rule SHALL
fail CI (R30, R34).

#### Scenario: The musl build links the allocator
- **GIVEN** the workspace of a pull request
- **WHEN** the job `Dependency policy` reads the dependency tree of `recipe-server` for the target
  `x86_64-unknown-linux-musl`
- **THEN** the tree names `mimalloc` with `recipe-server` as its dependent
- **AND** `crates/recipe-server/src/main.rs` holds one `#[global_allocator]` on one static of type
  `mimalloc::MiMalloc`, under `#[cfg(target_env = "musl")]`

#### Scenario: The gnu build keeps the system allocator
- **GIVEN** the workspace of a pull request
- **WHEN** the job `Dependency policy` reads the dependency tree of `recipe-server` for the target
  `x86_64-unknown-linux-gnu`
- **THEN** the tree names no `mimalloc`

#### Scenario: A musl tree without the allocator
- **GIVEN** a dependency tree of the musl target that names no `mimalloc`
- **WHEN** the CI check reads the tree
- **THEN** the check fails and names the missing allocator of the musl build

#### Scenario: A global allocator without the musl condition
- **GIVEN** a `main.rs` with `#[global_allocator]` and no `#[cfg(target_env = "musl")]` above it
- **WHEN** the CI check reads the file
- **THEN** the check fails and names the missing condition

#### Scenario: A global allocator of another type
- **GIVEN** a `main.rs` with `#[cfg(target_env = "musl")]` and `#[global_allocator]` above
  `static GLOBAL: std::alloc::System = std::alloc::System;`
- **AND** the dependency `mimalloc` stays in the musl tree
- **WHEN** the CI check reads the file
- **THEN** the check fails and names the type of the static

#### Scenario: An unwind test without its global allocator
- **GIVEN** a `tests/mimalloc_unwind.rs` without its `#[global_allocator]` static
- **WHEN** the CI check reads the file
- **THEN** the check fails and names the file

#### Scenario: A lint attribute on the allocator static
- **GIVEN** a `main.rs` with `#[allow(dead_code)]` or `#[expect(dead_code)]` on the static
- **WHEN** the CI check reads the file
- **THEN** the check fails

#### Scenario: A library crate depends on the allocator
- **GIVEN** a workspace in which `recipe-web` depends on `mimalloc`
- **WHEN** CI runs `cargo deny --locked check`
- **THEN** the check fails and names the banned edge of `mimalloc`

#### Scenario: An allocator option in a Compose file
- **GIVEN** a `compose.yaml` that sets `MIMALLOC_PURGE_DELAY=0` in the environment of `app`
- **WHEN** the CI check reads the files of `deploy/`
- **THEN** the check fails and names the variable and its file

#### Scenario: A comment names an allocator option
- **GIVEN** a script of `deploy/` with a comment line that names `MIMALLOC_PURGE_DELAY`
- **AND** no other line of `deploy/` names a variable whose name starts with
  `MIMALLOC_` in any letter case
- **WHEN** the CI check reads the files of `deploy/`
- **THEN** the check passes

### Requirement: Features of the allocator crates
The musl build SHALL resolve exactly the default features of `mimalloc` and no feature of
`libmimalloc-sys`, with and without `--all-features`. A failed rule SHALL fail CI (R30, R34).

#### Scenario: A feature of the allocator crates
- **GIVEN** a musl build in which `libmimalloc-sys` resolves a feature
- **WHEN** the CI check reads `cargo metadata` for the musl target
- **THEN** the check fails and names the crate and its features

### Requirement: Unwind proof with the allocator of the musl build
The CI job `musl-unwind` SHALL run an integration test of `recipe-server` on the target
`x86_64-unknown-linux-musl`. It SHALL run the test once in the dev profile and once in the release
profile. The test SHALL declare `mimalloc::MiMalloc` as its global allocator. It SHALL show that a
panic inside `CatchPanicLayer` unwinds. A failed test, a missing test, or a missing step SHALL fail
CI (R30, A41).

#### Scenario: A panic unwinds with the allocator of the musl build
- **GIVEN** the test binary of `crates/recipe-server/tests/mimalloc_unwind.rs` for the musl target.
  The test builds it in the dev profile and in the release profile.
  It has `mimalloc::MiMalloc` as its global allocator
- **WHEN** a request reaches a service that allocates, holds a value with a `Drop`, and panics
  inside `CatchPanicLayer`
- **THEN** the layer answers `500`
- **AND** the `Drop` of the value runs
- **AND** a second request to a normal service answers `200`

#### Scenario: The mode finds too few passed tests
- **GIVEN** a run of the mode of `deploy/ci/rust-tests.sh` in which one of the two profiles reports
  fewer passed tests than the file holds
- **WHEN** the mode reads the test output
- **THEN** the mode fails and names the expected count and the profile

#### Scenario: The step leaves the job
- **GIVEN** a `ci.yml` in which the job `musl-unwind` has no step that runs the mode
- **WHEN** `deploy/ci/check.sh` reads `ci.yml`
- **THEN** the check fails and names the missing step
