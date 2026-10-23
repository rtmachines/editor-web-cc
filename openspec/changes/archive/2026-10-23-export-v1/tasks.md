# Tasks

The table "Waves" of design D13 gives the agent, the class, the model, the effort, and the file set of each task. Each code task starts with a failing test. Tasks 6.1 and 7.4 are test tasks only.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0052, ADR-0053, ADR-0054, ADR-0055, the index rows, and the decision map. Add the "Amended by" note of ADR-0049. Check: `ste-lint.py` on the scope of ADR-0023 reports 0 hard violations.
- [x] 1.2 (docs/architecture) Put the decisions of ADR-0052 to ADR-0055 into `api.md` sections 2.7, 5.1, 5.3, and 7.3. Add the bullet of ADR-0055 to `.claude/rules/web.md`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.3 (AGENTS.md) Name `target/p35-brief.md` and the design of this change in line 9. Check: `ste-lint.py AGENTS.md` reports 0 hard violations.
- [x] 1.4 (CHANGELOG.md) Add the features of this change under "Added" and "Changed". Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.
- [x] 1.5 (docs/user) Write the how-to guide `check-export-document.md` of design D10. Link the golden files of task 4.3. Check: `ste-lint.py` on the file reports 0 hard violations.
- [x] 1.6 (docs/adr, docs/architecture) Write ADR-0056, its index row, and the decision map. Add the row `export.permits` to `reliability.md` section 7.11. Name the wait for the permit in the row "Export generation" of section 7.4. Check: `ste-lint.py` on the scope of ADR-0023 reports 0 hard violations.

## 2. Texts

- [x] 2.1 (recipe-web) [mechanical] Write a failing test in `src/i18n/export_texts_tests.rs` that reads the keys `error-E-EXP-001` to `error-E-EXP-003` of design D9 in `en` and `pl`. Add `E-EXP-001` to `E-EXP-003` to `codes_with_a_message`. Then copy these three rows of the table D9 into `i18n/en.ftl` and `i18n/pl.ftl`. Task 7.1 adds the two link keys.

## 3. Domain

- [x] 3.1 (recipe-domain) [hard] Write a failing test that the serialized struct value of `ZONE1` holds `setpoint` before `label`. Add tests with a struct inside an array and with a 2-D array of a struct, from a `TypeNode` that the test builds by hand. Add a test that a key outside `members[]` comes after the members in byte order. The stored hash vectors stay equal. Then write the member order of design D3 (a).
- [x] 3.2 (recipe-domain) [hard] Write failing tests of the accessors of `HashedBody` and of its entries. Add tests of the time text of design D3 (b): an offset `+02:00`, a fraction of a second, and a year outside the range. Then add the accessors, `DocumentFacts`, `ExportStatus`, `SCHEMA_VERSION`, and the doc comments of design D3 (c). Add no feature and no fixture module.
- [x] 3.3 (recipe-domain) [hard] Write failing tests of design D11 row T16. The accessor of a type reference gives a domain type of a type reference, not text. Then replace the text in `ParameterEntry::type_id`, `TypeEntry.id`, `TypeEntry.element_type_id`, and `Member.type_id` (design D3 (c)). Update the writer in `recipe-export/src/document.rs`. The stored hash vectors and both golden files stay equal.

## 4. Export crate

- [x] 4.1 (recipe-export) [hard] Write failing schema tests in `tests/schema.rs` for the valid document and for each bad case of design D11 row T3. The valid document is the example of `api.md` 6.2 with the real hash. Then add the dev-dependency and the schema file of design D4. Run `cargo deny --locked check`.
- [x] 4.2 (recipe-export) Write failing tests of `file_name` with and without the prefix `RETIRED_`. Write failing tests of `error_body` with the order of its three fields. Then add both functions of design D5 (b) and (c) and the dependencies `serde` and `serde_json`.
- [x] 4.3 (recipe-export) [hard] Write the failing golden tests of design D11 row T5. Build the two bodies in `tests/golden.rs` through the public constructors of `recipe-domain` (design D14). When a constructor is not public, stop and report. Then write `write_document` of design D5 (a). Each golden document passes the schema, and the JSON block of `api.md` 6.2 equals the first one.

## 5. Flow of the export

- [x] 5.1 (recipe-app) [hard] Write failing tests of design D11 rows T6 and T7 with a writer double that records the facts. Seed the user rows of the approvers in the memory fixtures. Fail the task with a writer double that panics. Then add the port `DocumentWriter`, the constructors with the writer, and the flow of design D6 (a) to (h). Update every caller of design D13 and the tests of design D12.
- [x] 5.2 (recipe-app) [hard] Write failing tests of design D11 row T15. A second export waits while `export.permits` exports hold a permit. A wait past `timeout.export` gives `E-WEB-009` and no entry. A task that runs after the limit keeps its permit. Then add the gate, the setting, and the flow of design D15 (ADR-0056).

## 6. Storage tests

- [x] 6.1 (recipe-db) Write the PostgreSQL tests of design D11 row T8. A test changes the data under the stored hash as the superuser with the triggers off. Make the shared helpers of `tests/storage/atomic/recipe.rs` visible, and do not copy them. The first run of a test-only task can pass.

## 7. Web

- [x] 7.1 (recipe-web) Write failing tests of design D11 row T9 for each role and each state. Then add the export link and the notice of design D8 to `version_actions` and `version.html`, and the two link keys of design D9. Update the tests of design D12 for W3.
- [x] 7.2 (recipe-web) [hard] Write failing tests of design D11 row T10 through a test route under `/export/v1/`. Then write the JSON answer of ADR-0053 in the error layer with `error_body` of `recipe-export`, and the message arguments of design D7 (d).
- [x] 7.3 (recipe-web) [hard] Write failing tests of design D11 rows T11 and T14. Then add the two routes, the handlers of design D7 (a) and (c), the headers, the adapter `ExportWriter`, and the `HEAD` rule of ADR-0054. Update the page sweeps of design D12.
- [x] 7.4 (recipe-web) Write the tests of design D11 row T12 for both export routes and the six principals. The first run of a test-only task can pass.
- [x] 7.5 (recipe-web) [hard] Write failing tests of design D11 row T13 for both routes: `Sec-Fetch-Site` `cross-site`, `same-site`, `none`, absent, and `HX-Request: true`. Check the `csrf_rejected` event and no `VersionExported` entry. Then add the guard of ADR-0055 (design D7 (f)) with the prefix constant of task 7.2.

## 8. Verification

- [x] 8.1 (workspace) Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo doc` with `-D warnings`, `cargo llvm-cov` with the gate of 80 %, `cargo deny --locked check`, `ste-lint.py` on the scope of ADR-0023, and `openspec validate --all --strict`. This task runs again after tasks 3.3 and 5.2.
