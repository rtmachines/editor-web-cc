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
- ADR-0036 and `docs/architecture/design-system.md` for the design system, the typefaces, the icons, and the page layout. `overview.md` sections 7 and 9 and `api.md` sections 2.1, 4, 5.3, and 9 hold the decisions and the new Assumption A44. `docs/plan/ux-brief.md` is the non-normative input of step P3.1.
- The style sheet `static/app.css` with every token of `design-system.md`. Tests compare the tokens with the document and compute the contrast ratio of each pair.
- The typefaces Atkinson Hyperlegible Next and Mono as 8 WOFF2 files under `/static/fonts/`, with a SHA-256 test for each file. The OFL-1.1 license texts are in the same directory.
- The scripts `static/row-version.js` and `static/tree.js`, and one static route for the style sheet, each script, and each font file.
- One layout for every full page. It has a skip link, a top bar with the navigation, the language switch, and the logout button, and `main` with `#error-panel` and `#toast`.
- The askama components for icons, status badges, alerts, notices, form fields, tables, trees, toasts, and the empty state.
- The query `ListRecipes` of `recipe-app` and the skeleton page `GET /recipes` with the empty state.
- The type `FieldReason` of `recipe-web`. Field reasons, missing entities, password reasons, task stages, and operation names in error messages come from the bundles.
- ADR-0037 for the pages of machines, units, and UDT types. It records the whole unit tree, the forms with `hx-boost`, and the member rows of the UDT form. It also records the form of a new revision and the second style sheet.
- The machine queries `ListMachines`, `GetMachine`, and `GetMachineForm` of `recipe-app`. Each query checks the permission in its first statement.
- The unit queries `GetUnitTree`, `GetUnitForm`, and `GetUnit` of `recipe-app`.
- The Plant Model queries `ListTypes`, `GetTypeTree`, `GetUdtForm`, and `GetTypeUsage` of `recipe-app`.
- The port reads `list_machines` and `type_usage` with an in-memory adapter and a `sqlx` adapter. The settings `page.machines` and `page.type_usage` set the page sizes.
- The pages of `api.md` section 2.2 in `pl` and `en`. They show the machine list, the machine form, and the machine page. They also show the unit tree, the unit form, and the unit page.
- The pages of `api.md` section 2.3 in `pl` and `en`: the type list, the UDT form with member rows, the type tree, and the usage page.
- The style sheet `static/components.css`. It holds the rules of the select, the checkbox, the datalist, the member row, and the pager. It also holds the rules of the tree rows, the archived badge, and the danger disclosure.
- The example `examples/dev_server.rs` of `recipe-web`. It serves the pages on `127.0.0.1` with the in-memory adapters for a test in a browser.
- ADR-0038 for a struct cycle across revisions. `DefineUdt` refuses a struct whose type tree holds a struct with the same name.
- The messages of the Plant Model error codes in `pl` and `en`.
- ADR-0039 for the form of a parameter value and the leaf grid. ADR-0040 for the POST routes of translations and removals. ADR-0041 for the error lists of a form.
- ADR-0042 for the snapshot reads of draft queries. ADR-0043 for the pages of the recipe editor.
- The read port `RecipeReads` with an in-memory adapter and a `sqlx` adapter. `Isolation::RepeatableRead` opens a `REPEATABLE READ READ ONLY` transaction.
- The recipe queries `ListRecipes`, `ListDeletedRecipes`, `GetRecipe`, `GetRecipeForm`, `ListVersions`, `GetVersionForm`, and `GetVersion` of `recipe-app`.
- The parameter queries `ListParameters`, `GetParameter`, `GetLimits`, `GetTranslations`, and `GetParameterForm`. `GetParameterForm` shows the tree of a type without a write.
- The settings `page.recipes`, `page.versions`, `page.parameters`, `form.array_chunk`, and `i18n.translation_langs`.
- `LeafTexts`, `ParameterValue::from_texts`, and `ParameterValue::texts` in `recipe-domain`. A value travels as one text per leaf.
- The type `Expected` for the expected form of a value, and `MemberValue` for the new members of a rebind.
- `rebind_new_leaves` in `recipe-domain`. The rebind form and the command use the same rule for the new member leaves.
- The pages of `api.md` section 2.4 in `pl` and `en`: the recipe list with search and filters, the recipe form, the recipe page, and the rename.
- The pages of `api.md` section 2.5: the version list, the version form, and the version page with the parameter table and the unit links.
- The pages of `api.md` section 2.6: the parameter form with the leaf grid, the parameter page, the value, the limits, the translations, and the removals.
- The list of outdated parameters and the rebind form on the version page.
- The error list of a form. `#error-panel` shows at most 20 lines, and the form comes back with a line under each bad input.
- The conflict answer shows the `row_version` of the server, the login of the last editor, and the time of the last change.
- A body limit for each route group. A parameter value can have 4 MiB, and a translation 64 KiB.

### Changed

- `reliability.md` section 8 and the R40 row of `overview.md` section 7 refer to the lint scope of ADR-0023.
- `reliability.md`, `domain-model.md`, `data-model.md`, and `api.md` list the codes of ADR-0024. `reliability.md` sections 1.1 and 4.3 hold the rule of ADR-0025. The base-unit table of `domain-model.md` section 2.1 holds the ranges of ADR-0026.
- `reliability.md` sections 1.1, 4.3, 4.4, and 6 and `domain-model.md` sections 2.2, 3, 4, 5.2, and 6 hold the decisions of ADR-0027 and ADR-0028. ADR-0008 and ADR-0016 have an "Amended by" note.
- `overview.md` section 5, `domain-model.md` sections 2.4, 5.1, 5.2, and 6, and `data-model.md` sections 6 and 10 hold the decisions of ADR-0029. `api.md` sections 2.2, 2.5, 2.6, and 3.2 and `reliability.md` sections 2.2, 2.4, and 7.4 hold them too. ADR-0005, ADR-0013, ADR-0022, and ADR-0028 have an "Amended by" note.
- `data-model.md` sections 4.9, 5.1, 5.3, and 6 and `reliability.md` sections 7.7 and 7.11 hold the decisions of ADR-0030 to ADR-0034. ADR-0020 has an "Amended by" note.
- `overview.md` section 5, `domain-model.md` section 5.3, and `api.md` sections 2.8, 7.1, 7.3, 7.4, and 8 hold the decisions of ADR-0035.
- `data-model.md` sections 1.1, 1.3, 3.6 to 3.8, 4.11, 5.1, 5.3, 5.4, 6, 8, and 10 and `reliability.md` sections 1.1, 1.2, 7.1, and 7.11 hold them too. ADR-0014, ADR-0022, ADR-0029, and ADR-0034 have an "Amended by" note.
- The pages of `api.md` sections 2.1 and 2.8 use the layout and the components. The logout form is in the top bar.
- An HTMX error answer sets `HX-Reswap: innerHTML show:#error-panel:top` (ADR-0036).
- A field error line shows the label of the field from the bundles. In an HTMX fragment and on the login page, the line links to its input.
- The success message of a user command goes into `#toast`. The same answer clears `#error-panel`.
- `design-system.md` sections 1, 6, 8, 9, 10.1, 10.2, 10.5, 10.8, 11, and 12 hold the decisions of ADR-0037. `api.md` sections 1, 2.1, 2.2, and 2.3 and `reliability.md` section 7.11 hold them too. ADR-0036 has an "Amended by" note.
- The navigation shows the link to the machines after the link to the recipes. A boosted form or link of a page replaces the content of `main` and the page title.
- An HTMX error answer sets `HX-Push-Url: false`, so a refused form keeps the URL of its page. `api.md` sections 1 and 5.3 and I8 of `domain-model.md` hold the decisions of ADR-0037 (z) and ADR-0038.
- `api.md` sections 1, 2.4 to 2.6, 4, 7.6, and 9 and `reliability.md` sections 4.2, 4.4, and 7.11 hold the decisions of ADR-0039 to ADR-0043. `domain-model.md` sections 2.1 and 5.2, `data-model.md` section 6, and `design-system.md` sections 8, 9, 10.4, 10.5, and 10.8 hold them too.
- ADR-0001, ADR-0027, ADR-0029, and ADR-0036 have an "Amended by" note. `http.body_limit.value` is 4 MiB.
- The draft functions of `recipe-domain` return every fault of one parameter in one `RecipeErrors`. The field `expected` of `ValueShapeMismatch` is an `Expected`.
- `AddParameter`, `UpdateParameterDefinition`, and `SetParameterValue` take a `ValueInput`. `RebindTypeRevision` takes a `MemberValue` for each new member leaf.
- The forms of machines, units, and UDT types answer every bad field at once (ADR-0041).
- The recipe list page of step P3.1 shows the table, the pages, and the search form.
- `design-system.md` sections 9, 10.4, and 10.6 and `api.md` sections 2.5 and 2.6 hold decisions of the implementation. ADR-0036 has a second "Amended by" note.
- ADR-0044 limits the rebind forms of one version page to `form.rebind_leaves` new member fields. Only a user with `ParameterEdit` on a live draft gets them. ADR-0043 has an "Amended by" note.
- ADR-0045 keeps an array inside a group of the leaf grid in that group, with no toggle of its own. `design-system.md` section 10.8 holds the rule. ADR-0039 has an "Amended by" note.
- ADR-0046 runs the form query of a route before the route reads its fields. The form query checks the permission of its purpose. `api.md` section 3 and `domain-model.md` section 5.5 hold the rule. ADR-0041 has an "Amended by" note.
- ADR-0047 shows a refused form without HTMX on a page of the session with the sent texts. Every HTMX error answer empties `#toast`, and `#toast` sticks at the top of the view. The removal of a parameter shows a toast. `design-system.md` section 10 holds the rules. ADR-0036 has an "Amended by" note.

### Security

- `Cargo.lock` holds `rustls` 0.23.46 in place of 0.23.45 (RUSTSEC-2026-0346 to RUSTSEC-2026-0349).
