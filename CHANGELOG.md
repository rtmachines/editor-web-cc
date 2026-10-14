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
- ADR-0027 for the error lists `RecipeErrors` and the error `DraftError` of a draft command.
- ADR-0028 for the version header, the approval token, the load of a draft, and the value and text clarifications.
- The Recipe Management types of `recipe-domain`: `Recipe`, the version state machine, `Draft`, `ParameterValue`, and `ProcessLimit`. Also the S7 checks, the snapshot check, the rebind mapping, and `RecipeError`.
- ADR-0029 for the ownership of the approval token, the audit row images, the scope of the export timeout, and clarifications of the application layer.
- The application layer of `recipe-app`: the ports, the permission check, the audit entries, and the in-memory adapters. Also the Plant Model commands, the Recipe Management commands, `EnsureDerivedType`, and `ExportVersion`.
- The modules `audit` and `export` of `recipe-domain`, and `Permission` and `Role`.
- ADR-0030 for the guard of a version status write.
- ADR-0031 for the configuration of the migrator and the secret files of the roles.
- ADR-0032, which closes the databases `postgres` and `template1` to `PUBLIC`.
- ADR-0033 for the options of the globals dump of the backup.
- ADR-0034, which hardens the audit objects with an event trigger and a qualified operator.
- The migrations `0001_schema.sql` to `0004_seed_plc_types.sql` and the script `deploy/db/initdb/01-roles.sh`.
- The `sqlx` adapters of `recipe-db`, the map of database errors, the connection pool, and the migrator.
- The subcommand `recipe-server migrate`.
- ADR-0035 for the identity commands, the sessions, the login throttle, and the guard of security events.
- The Identity and Access commands and queries of `recipe-app`, the Argon2id hasher behind a semaphore, and their `sqlx` adapters.
- The migration `0005_security_event_guard.sql`, which narrows the insert grant of `security_event` and adds a guard trigger.
- The request pipeline of `recipe-web` with the security headers, the CSRF checks, and the session reload. Also unstyled pages for the routes of `api.md` sections 2.1 and 2.8 in `pl` and `en`.
- The local breached-password list from SecLists and the script `tools/breached-passwords/filter.py`.
- The subcommand `recipe-server create-admin`.

### Changed

- `reliability.md` section 8 and the R40 row of `overview.md` section 7 refer to the lint scope of ADR-0023.
- `reliability.md`, `domain-model.md`, `data-model.md`, and `api.md` list the codes of ADR-0024. `reliability.md` sections 1.1 and 4.3 hold the rule of ADR-0025. The base-unit table of `domain-model.md` section 2.1 holds the ranges of ADR-0026.
- `reliability.md` sections 1.1, 4.3, 4.4, and 6 and `domain-model.md` sections 2.2, 3, 4, 5.2, and 6 hold the decisions of ADR-0027 and ADR-0028. ADR-0008 and ADR-0016 have an "Amended by" note.
- `overview.md` section 5, `domain-model.md` sections 2.4, 5.1, 5.2, and 6, and `data-model.md` sections 6 and 10 hold the decisions of ADR-0029. `api.md` sections 2.2, 2.5, 2.6, and 3.2 and `reliability.md` sections 2.2, 2.4, and 7.4 hold them too. ADR-0005, ADR-0013, ADR-0022, and ADR-0028 have an "Amended by" note.
- `data-model.md` sections 4.9, 5.1, 5.3, and 6 and `reliability.md` sections 7.7 and 7.11 hold the decisions of ADR-0030 to ADR-0034. ADR-0020 has an "Amended by" note.
- `overview.md` section 5, `domain-model.md` section 5.3, and `api.md` sections 2.8, 7.1, 7.3, 7.4, and 8 hold the decisions of ADR-0035.
- `data-model.md` sections 1.1, 1.3, 3.6 to 3.8, 4.11, 5.1, 5.3, 5.4, 6, 8, and 10 and `reliability.md` sections 1.1, 1.2, 7.1, and 7.11 hold them too. ADR-0014, ADR-0022, ADR-0029, and ADR-0034 have an "Amended by" note.

### Security

- `Cargo.lock` holds `rustls` 0.23.46 in place of 0.23.45 (RUSTSEC-2026-0346 to RUSTSEC-2026-0349).
