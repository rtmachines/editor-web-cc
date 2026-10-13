# Changelog

This file records the notable changes of the recipe editor. The format follows [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Cargo workspace with the six crates of `overview.md` section 5. Each crate is a skeleton without application code.
- Workspace lints of `reliability.md` section 2.1 and the release profile of section 2.4.
- `deny.toml` with six bans from the dependency rule of `overview.md` section 5, a license allow list, and advisory checks.
- CI workflow for format, clippy, tests, documentation, `cargo deny`, coverage, and the STE lint.
- `tools/coverage-gate.py`, which applies the coverage target of 80 % lines to each crate.
- A test in `recipe-server` that runs the skeleton `main`.
- Nightly `cargo audit` workflow.
- ADR-0023 for the scope and the location of the STE lint.
- `tools/ste-lint/`, a copy of the script `ste-lint.py` under the MIT license.
- ADR-0024 with the codes `E-PLT-021 DuplicateMemberName` and `E-RCP-054 S7AddressWidthMismatch`.
- ADR-0025 for the parse reasons of value objects without an own error code.
- ADR-0026 with the platform ranges of `DATE` and `DATE_AND_TIME`.
- The Plant Model types of `recipe-domain`: value objects, `Machine`, `PlcType`, `TypeTree`, `S7Layout`, `IecLiteral`, `PlantError`, and the error code registry.

### Changed

- `reliability.md` section 8 and the R40 row of `overview.md` section 7 refer to the lint scope of ADR-0023.
- `reliability.md`, `domain-model.md`, `data-model.md`, and `api.md` list the codes of ADR-0024. `reliability.md` sections 1.1 and 4.3 hold the rule of ADR-0025. The base-unit table of `domain-model.md` section 2.1 holds the ranges of ADR-0026.
