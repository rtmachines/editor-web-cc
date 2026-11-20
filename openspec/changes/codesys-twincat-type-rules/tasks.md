# Tasks

Each task is for one agent. Sonnet implements each code task. Haiku does each `[mechanical]` task. A Sonnet agent runs the `ste-lint.py` step of each doc task.

The ADRs 0199 to 0209 have the status "Proposed". The owner chose the options on 2026-11-19 and answered the plan review on 2026-11-20. The owner did not yet accept the text of the ADRs and of this file. Task 1.0 sets the status "Accepted". The changes of `docs/adr/`, `docs/architecture/`, and `docs/ops/runbook.md` are part of this change. Only task 1.0 and task 9.1 edit them.

Each code task first writes a failing test and runs the RED command. Then it writes the change and runs the GREEN command. A test name starts with the ADR, for example `adr_0201_`, so the filter of each command finds only the tests of the task. Each code task ends with `cargo test --workspace --all-features --locked`. A test of another crate that pins an earlier rule changes in the same task.

Some kinds of task have no RED step. Each such task names its kind:

- A pin task (1.1 to 1.7, 10.3) pins behavior that the code has now. Its test passes with no code change. A failure is a defect: the agent stops and reports it.
- An integration task (3.3, 6.2, 7.2, 10.1, 10.2, 11.20) tests the use cases of a rule that an earlier task added. Its tests pass after that task.
- The check task 9.2 tests a query of `docs/ops/runbook.md`. Its test passes with no code change.
- The fixture task 2.3 writes a full dump of a database with the stored rows of the matrix of `design.md`. It uses the code before this change.
- Task 1.0 and task 9.1 change docs only. Task 1.8 changes three comments only. `cargo doc` checks task 1.8.
- Task 1.9 and task 12.1 are verification only. They change no file.

Eleven tests fail at the start of this change. The worktree already holds the normative docs of the ADRs:

- `plant::error::tests::registry_matches_reliability` of `recipe-domain`. `reliability.md` §1.2 holds the rows `E-PLT-053` and `E-PLT-054`, and the code has no variant. Task 2.1 fixes it.
- `error::status::tests::error_mapping_status_table_equals_the_registry_of_the_document` of `recipe-web`, for the same rows. Task 2.1 fixes it.
- `error_registry_every_pair_of_row_and_variant_has_a_sample` of `recipe-server --test error_registry`, for the same rows. Task 2.1 fixes it.
- `auth::matrix_doc::the_table_and_the_catalog_name_the_same_operations` of `recipe-app`. The row of `SetMachineStringEncoding` in `api.md` §3.3 has no catalog entry. Tasks 1.0 and 2.2 fix it.
- `auth::matrix_doc::every_mark_of_the_table_equals_the_catalog_result` of `recipe-app`, for the same row. Tasks 1.0 and 2.2 fix it.
- In `recipe-db --test storage`, `constraints::every_named_constraint_and_unique_index_of_section_3_is_covered_or_exempt`. The two checks of `string_encoding` have no test. Task 2.4 fixes it.
- In `recipe-db --test storage`, `schema::migration_0001_equals_the_sql_blocks_of_sections_3_0_to_3_17` and `schema::every_name_of_section_3_exists_in_the_catalog`. Section 3.1 holds the new column and checks. Task 2.4 fixes them.
- In `recipe-db --test storage`, `grants::migration::migration_0003_equals_the_sql_block_of_section_5_2`, `migration_0012_holds_the_second_sql_block_of_section_5_2`, and `migration_0013_holds_the_third_sql_block_of_section_5_2`. Section 5.2 holds a fourth SQL block. Task 2.5 fixes them.

Other tests of wave 1 fail only after a task adds a new item. They are the sample tests of `plant_samples`, the catalog counts, and `restore_code`. The exact `machine` images of the contract tests also fail after `0017`. The task that adds the item fixes them.

The row of `api.md` §3.3 writes `STRING` in backticks. The parser of `crates/recipe-app/src/auth/matrix_doc.rs` reads each backticked name of column one as an operation, so it reads `STRING` as an operation too. Task 1.0 deletes these backticks. The parser does not change.

## Run order

The orchestrator starts a wave after the previous wave merges. Each chain of a wave runs in its own git worktree (`isolation: worktree`), from the same commit. The tasks of one chain run in series in that worktree. Two chains of one wave edit no common file and use no common integration target. Chains of one wave can run `cargo test -p recipe-domain --lib` or `cargo test -p recipe-app --lib` with different filters, as in `siemens-type-rules`. In wave 1, chain B runs `recipe-app --lib auth`, and chain C runs `recipe-app --lib memory`. The orchestrator merges the chains in the listed order. After each merge, it runs `cargo test --workspace --all-features --locked`. A merge conflict stops the run, and the orchestrator reports it.

In wave 1, the end command of a chain still shows the start failures that another chain of wave 1 fixes. The end command of chain B also shows `values_every_operation_has_its_key_in_both_bundles` of `recipe-web`. Task 2.1 of chain A adds the key of the new operation. After the merge of the three chains, the workspace passes.

The work of OQ03 (ADR-0202) has two parts: wave 1 (codes, catalog, storage) and wave 4 (rules and use cases).

0. Step 0 runs when the owner accepts ADR-0199 to ADR-0209 with this file. It runs task 1.0, then task 1.9. No code task starts before it.
1. Wave 1 has three chains. Only OQ03 work and its fixture run in it:
   - Chain A: 2.1. It edits `recipe-domain` (`plant/`, `error_codes.rs`, `plant/error/tests.rs`), `recipe-web` (error mapping, `en.ftl`, `pl.ftl`), and `crates/recipe-server/tests/error_registry/plant.rs`.
   - Chain B: 2.2. It edits `crates/recipe-app/src/auth/` only.
   - Chain C: 2.3, then 2.4, then 2.5. Task 2.3 writes the fixture before the migration `0017` exists. Chain C owns `migrations/`, `crates/recipe-db/tests/`, `crates/recipe-db/tests/grants.expected`, `deploy/pgtools/restore-roles.expected`, `deploy/pgtools/restore-code.expected`, and `crates/recipe-server/tests/migrate.rs`. Chain C also owns `crates/recipe-app/src/memory/images.rs` and `crates/recipe-app/src/memory/plant/tests.rs`.
2. Wave 2 has five chains:
   - Chain D: 1.1, 1.2, then 7.1. They edit `plant/type_expr.rs`, `plant/type_expr/tests.rs`, and `plant/string_length/tests.rs` of `recipe-domain`.
   - Chain E: 1.6, then 6.1. Both edit the module `plant/iec_literal` of `recipe-domain`.
   - Chain F: 1.4, 1.7, 3.1, 3.2, 4.1, 5.1. They share `crates/recipe-web/i18n/en.ftl`, `crates/recipe-web/i18n/pl.ftl`, `crates/recipe-web/src/error/plant.rs`, `recipe/value/leaf.rs`, `recipe/value/leaf/tests.rs`, `plant/error.rs`, `plant/error/tests.rs`, `plant/iec_names.rs`, `crates/recipe-export/tests/golden_types.rs`, and the files `types_*.json` of `crates/recipe-export/tests/golden/`.
   - Chain G: 1.3, 1.5, 1.8. They edit `recipe/value/parse/tests.rs`, `recipe/limit/parse/tests.rs`, `plant/string_length.rs`, and `plant/primitive.rs` of `recipe-domain`.
   - Chain H: 8.1. It owns `crates/recipe-export/schema/export-v2.schema.json`, `crates/recipe-export/tests/schema.rs`, and `crates/recipe-export/tests/schema/`.
3. Wave 3 has three chains:
   - Chain I: 3.3, 3.4, 4.2, 4.3, 5.2, 5.3, 6.2, 7.2, 7.3. They share the test targets `recipe-app --lib`, `recipe-web --lib`, and `recipe-db --test storage`. Tasks 4.2 and 5.2 edit `plant/current_type.rs` and `crates/recipe-db/tests/storage/rebind_shape.rs`. Tasks 4.3 and 5.3 edit `deploy/db/check-identifier-rules.sql`.
   - Chain J: 9.1. It edits `docs/ops/runbook.md` only and builds no crate.
   - Chain K: 10.2, then 10.3. They edit `crates/recipe-export/tests/schema/platform_rules.rs`, `crates/recipe-export/tests/golden.rs`, and `crates/recipe-export/tests/golden/` only.
4. Wave 4 is chain L: tasks 11.1 to 11.20 in series. They change the machine, the carrier of the encoding, the value rule, the use cases, the pages, and the export of every crate.
5. Wave 5 is chain M: task 10.1, then task 9.2. Both use `recipe-db --test storage`.
6. Wave 6 is task 12.1, the gate of the change.

## 1. Acceptance, pin tests, and comments (review items R02 to R10, ADR-0209)

- [x] 1.0 (docs/adr) [mechanical] When the owner accepts ADR-0199 to ADR-0209 with this file, make these edits:
  - Set the status of ADR-0199 to ADR-0209 to "Accepted". Keep the text "Amends ..." after the status.
  - In `docs/adr/README.md`, change "Proposed" to "Accepted" in the index rows of ADR-0199 to ADR-0209. In the decision map, delete "proposed, " after 0199 to 0209.
  - In each note "Amended by ADR-0NNN (proposed, ...)" of ADR-0199 to ADR-0209, delete "proposed, ". These notes stand in the older ADRs and in their index rows.
  - The older ADRs are ADR-0028, ADR-0109, ADR-0119, ADR-0120, ADR-0137, ADR-0143, ADR-0166, ADR-0180, ADR-0181, and ADR-0189. The notes of ADR-0028 and ADR-0137 come from the fix of the plan review (ADR-0202).
  - In the row of `SetMachineStringEncoding` in `docs/architecture/api.md` §3.3, write the word STRING without backticks.
  - Check: `grep -rnE '0(199|20[0-9]) \(proposed' docs/adr` prints nothing. Before the task, it prints 24 lines.
  - Check: `grep -nE '^\| \[0(199|20[0-9])\].*\| Proposed' docs/adr/README.md` prints nothing. Before the task, it prints 11 lines.
  - Check: `ste-lint.py` on the changed files reports 0 hard violations.
- [x] 1.1 (recipe-domain) Pin task for review item R02. In `crates/recipe-domain/src/plant/type_expr/tests.rs`, add `adr_0209_open_array_pointer_and_reference_fail`.
  - `TypeExpr::parse` gives `DerivedTypeReason::Grammar` for each of these texts: `ARRAY[*] OF INT`, `ARRAY[*,*] OF INT`, `ARRAY[1..2,*] OF INT`, `POINTER TO INT`, and `REFERENCE TO INT`.
  - The parser does not read the platform, so the test has no loop over the platforms.
  - Command: `cargo test -p recipe-domain --lib adr_0209_open_array` (expect pass).
- [x] 1.2 (recipe-domain) Pin task for review item R05. Add `adr_0141_wstring_of_length_zero_fails` in two files.
  - In `plant/type_expr/tests.rs`, `TypeExpr::parse("WSTRING(0)")` gives `DerivedTypeReason::Grammar`. The parser runs once, with no platform loop.
  - In `plant/string_length/tests.rs`, `StringLength::parse(StringKind::Wstring, platform, 0)` gives `StringLengthReason::OutOfRange` on `CodesysIec`, `TwinCat3`, `S7_1200`, and `S7_1500`. This check reads the platform.
  - The two reasons are fixed. `parse_string` of `plant/type_expr.rs` refuses the text `0` with `Grammar`, and `StringLength::parse` refuses the length 0 with `OutOfRange`.
  - Another reason is a defect. The agent stops and reports it.
  - Command: `cargo test -p recipe-domain --lib adr_0141_wstring_of_length_zero` (expect pass).
- [x] 1.3 (recipe-domain) Pin task for review item R03. Add `adr_0117_nan_and_infinity_fail` in `recipe/value/parse/tests.rs` and in `recipe/limit/parse/tests.rs`.
  - The test uses three routes: the JSON value, `ParameterValue::from_texts`, and `ProcessLimit::parse`. Each route uses a `REAL` and an `LREAL` leaf on `CodesysIec` and `S7_1500`.
  - The JSON route accepts the JSON number `1.5`. The numeric path of the leaf reads only `Value::Number`.
  - The JSON route refuses the JSON strings `"NaN"`, `"inf"`, `"-inf"`, `"Infinity"`, and `"-Infinity"`. JSON has no number for these values.
  - The form route and the limit route accept the text `1.5`. They refuse the texts `NaN`, `inf`, `-inf`, `Infinity`, and `-Infinity`.
  - A refused value gives `E-RCP-005`. A refused limit gives `E-RCP-006`.
  - Command: `cargo test -p recipe-domain --lib adr_0117_nan_and_infinity` (expect pass).
- [x] 1.4 (recipe-domain) Pin task for review item R04. In `crates/recipe-domain/src/recipe/value/leaf/tests.rs`, add `adr_0137_last_latin_1_character`.
  - A `STRING(1)` leaf on `S7_1500`, on `CodesysIec`, and on `TwinCat3` accepts `"\u{FF}"`. It gives `Expected::String` for `"\u{100}"`.
  - Every machine of this wave has the encoding `Latin1`. The doc comment of the test says so. Task 11.4 adds the `Utf8` cases in a separate test, `adr_0202_utf8_two_byte_scalars_respect_capacity`.
  - Command: `cargo test -p recipe-domain --lib adr_0137_last_latin_1` (expect pass).
- [x] 1.5 (recipe-domain) Pin task for ADR-0209 (f) and review item R11. In `crates/recipe-domain/src/recipe/value/parse/tests.rs`, add `adr_0209_first_dimension_is_the_outer_json_level`. `ARRAY[1..2,1..3] OF INT` accepts `[[1,2,3],[4,5,6]]`, and the leaf `[2,1]` holds `4`. It refuses `[[1,2],[3,4],[5,6]]` with `E-RCP-005`. Command: `cargo test -p recipe-domain --lib adr_0209_first_dimension` (expect pass).
- [x] 1.6 (recipe-domain) Pin task for review item R09, literal part. In `crates/recipe-domain/src/plant/iec_literal/range/tests.rs`, add `adr_0115_iec_ends_of_time_literals`. Take the ends from `domain-model.md` §2.1, base unit table. On `CodesysIec` and `TwinCat3`, each literal at an end parses:
  - `TIME`: `T#0ms` gives the count 0, and `T#49d17h2m47s295ms` gives 4294967295 milliseconds.
  - `LTIME`: `LT#0ns` gives 0, and `LT#213503d23h34m33s709ms551us615ns` gives 18446744073709551615 nanoseconds.
  - `DATE`: `D#1970-01-01` gives 0, and `D#2106-02-07` gives 49710 days.
  - `DATE_AND_TIME`: `DT#1970-01-01-00:00:00` gives 0, and `DT#2106-02-07-06:28:15` gives 4294967295 seconds.
  - The test asserts each count of the base unit, and not only the result `Ok`.
  - The literal one step outside each end gives `IecLiteralReason::OutOfRange`, for example `T#-1ms`, `T#49d17h2m47s296ms`, and `D#2106-02-08`.
  - Another reason is a defect. The agent stops and reports it.
  - Command: `cargo test -p recipe-domain --lib adr_0115_iec_ends` (expect pass).
- [x] 1.7 (recipe-export) Pin task for review item R09, export part. In `crates/recipe-export/tests/golden_types.rs`, extend `platform_cases` to `CodesysIec` and `TwinCat3`.
  - Add the golden files `types_CodesysIec.json` and `types_TwinCat3.json` beside the S7 files.
  - Add `adr_0199_golden_types_of_codesys_and_twincat`.
  - Check each 64-bit end and each `LREAL` value of the new files by hand against `domain-model.md` §2.1 and `data-model.md` §7. Use only values that the rules of this change accept.
  - Command: `cargo test -p recipe-export --test golden_types adr_0199` (expect pass).
- [x] 1.8 (recipe-domain) [mechanical] Review item R08, comments only.
  - The comment at `crates/recipe-domain/src/plant/primitive.rs:41` gives the unit of a `WSTRING` length as UTF-16 code units, and cites ADR-0137 (i).
  - The comment at `primitive.rs:39` says that a `STRING` holds one byte per character on a `Latin1` machine and UTF-8 bytes on a `Utf8` machine (ADR-0202).
  - The comment at `crates/recipe-domain/src/plant/string_length.rs:61` names the units. A `STRING` length counts characters on a `Latin1` machine and UTF-8 bytes on a `Utf8` machine. A `WSTRING` length counts UTF-16 code units (ADR-0137 (i), ADR-0202).
  - `migrations/0001_schema.sql:95` shipped and does not change.
  - Check: `RUSTDOCFLAGS="-D warnings" cargo doc -p recipe-domain --no-deps` passes, and `grep -n 'two bytes per character' crates/recipe-domain/src/plant/primitive.rs` prints nothing.
- [x] 1.9 (docs) [mechanical] Verification only, for review items R06, R07, R10, and for A51 and A52. It changes no file. A missing text is a defect, and the agent reports it.
  - Review item R06: `data-model.md` §7 holds the size note of ADR-0209 (c).
  - Review item R07: `data-model.md` §7 holds the list of the excluded type families of ADR-0209 (a).
  - Review item R10: the row `parameters[].value` of `api.md` §6.1 holds three examples. They are `16#E9` (`é` on `Latin1`), the two bytes `16#C3` and `16#A9` (`é` on `Utf8`), and `16#20AC` (`€` in a `WSTRING`).
  - The row writes each byte in its own backticks. The two UTF-8 bytes of `é` stand next to each other in the order `16#C3`, `16#A9`.
  - Check: ``grep -n 'parameters\[\]\.value' docs/architecture/api.md | grep '16#E9' | grep -F '`16#C3` `16#A9`' | grep -c '16#20AC'`` prints `1`. The fixed-string search fails for the reversed order.
  - A51 and A52: `docs/ops/runbook.md` holds the section "Check the CODESYS version and the LREAL size before you add a CodesysIec machine".

## 2. STRING encoding: codes, catalog, and storage (ADR-0202, wave 1)

- [x] 2.1 (recipe-domain) Add the codes of ADR-0202. RED: `cargo test -p recipe-domain --lib registry_matches_reliability`, `cargo test -p recipe-web --lib error_mapping_status_table`, and `cargo test -p recipe-server --test error_registry` fail.
  - Add the value object `StringEncoding` in `crates/recipe-domain/src/plant/string_encoding.rs` with `parse` for `Latin1` and `Utf8`, and with `as_str`. Add `adr_0202_string_encoding_parses_two_values`.
  - Add the function that gives `E-PLT-053` for `Utf8` on `S7_300`, `S7_1200`, and `S7_1500`, with `adr_0202_utf8_fails_on_s7`.
  - Add `PlantError::StringEncodingNotOnPlatform { code, encoding, platform }` and `PlantError::StringEncodingFrozen { code }`. Add their rows to `error_codes.rs`, and change the code ranges of `registry_matches_reliability` to include 53 and 54.
  - Add one sample of each variant to `plant_samples()` in `crates/recipe-domain/src/plant/error/tests.rs`.
  - In `recipe-web`, map both variants to their status, and add the messages `error-E-PLT-053` and `error-E-PLT-054` to `en.ftl` and `pl.ftl`. The English messages equal `reliability.md` §1.2.
  - Also add the key `operation-set-machine-string-encoding` of the operation of task 2.2 to `en.ftl` and `pl.ftl`. The texts are "Set STRING encoding" and "Zmiana kodowania STRING".
  - Chain A owns both bundles in wave 1, so chain B adds no key. A key without its operation breaks no test of chain A, because the unused-key guard reads only `en-approval.ftl` and `pl-approval.ftl`.
  - In `crates/recipe-server/tests/error_registry/plant.rs`, add one sample of each variant.
  - GREEN: the three RED commands, `cargo test -p recipe-domain --lib plant::error`, and `cargo test -p recipe-domain --lib adr_0202` pass.
- [x] 2.2 (recipe-app) Add the operation `SetMachineStringEncoding` to the catalog. It needs `MachineManage` and stops in read-only mode (A24). RED: `cargo test -p recipe-app --lib matrix_doc` fails after task 1.0.
  - Add the entry to `auth/operation/catalog.rs`, to `MATRIX` and `STOPPED_IN_READ_ONLY` in `auth/matrix.rs`, and to `TABLE` and `COMMAND_NAMES` in `auth/operation/tests.rs`.
  - In `auth/operation/tests.rs`, change the counts. Rename `catalog_has_73_distinct_operations` to `catalog_has_74_distinct_operations` and assert 74. The names test asserts 72 in place of 71.
  - Rename `exactly_37_variants_are_commands` to `exactly_38_variants_are_commands` and assert 38 command variants.
  - `COMMAND_NAMES` changes from `[&str; 36]` to `[&str; 37]`. The two export variants share the name `ExportVersion`, so 38 command variants have 37 distinct names.
  - In `auth/operation/catalog.rs`, change the comment "37 commands and 36 queries" to "38 commands and 36 queries", and change `count: 73` to `count: 74`. Change the count 73 of the macro comment too.
  - `STOPPED_IN_READ_ONLY` of `auth/matrix.rs` changes from 32 to 33 entries.
  - Add `adr_0202_set_machine_string_encoding_needs_machine_manage`. The Administrator and the ProcessEngineer pass, and the Operator and the ExportClient fail.
  - GREEN: `cargo test -p recipe-app --lib auth`.
- [x] 2.3 (recipe-db) Fixture task. It runs first in chain C of wave 1, on the code before this change and before `0017` exists.
  - Add `crates/recipe-db/tests/storage/stored_rules_iec.rs` and its line `mod stored_rules_iec;` in `main.rs`. Add the ignored test `adr_0202_stored_rules_iec_write_the_fixture`.
  - In `crates/recipe-db/tests/storage/support/writers.rs`, `DocumentFactsWriter::write` (line 91) and `DocumentFactsWriter::write_scoped` (line 113) capture the body and the facts.
  - Each capture holds the canonical bytes of the hashed body or of the scope body, its hash, `status`, and `has_rule_fault`. It holds no `exported_at`.
  - `write_scoped` returns these bytes in place of the constant `DOCUMENT`. Change each test that asserts `DOCUMENT` after a scoped export. The recording writer keeps `DOCUMENT`.
  - The test builds the six cases of the matrix of `design.md` through the use cases on PostgreSQL (`crate::support::app::App`). Each case is one recipe with one parameter and one version that `ApproveVersion` approves.
  - The struct cases need no rule of this change, so `DefineUdt` accepts them. The `WSTRING` cases use the JSON value, which the code before ADR-0201 accepts.
  - The test runs the export use case once with `ExportScope::Version` and once with `ExportScope::Unit(U1)`. It reads the `scope_sha256` of the unit from the audit entry `VersionExported`.
  - It writes a full plain dump of that database with `_sqlx_migrations`, as `stored_rules_write_the_fixture` does, to `crates/recipe-db/tests/storage/fixtures/stored_rules_iec.sql`.
  - It writes `fixtures/stored_rules_iec.hashes` with one line `<reference> <content_sha256> <scope_sha256>` for each case.
  - It writes `fixtures/stored_rules_iec.exports.json` with the two captures and the `loadable` value of `ExportedVersion` for each case.
  - Command: `cargo test -p recipe-db --test storage adr_0202_stored_rules_iec_write_the_fixture -- --ignored`. Run it once, because a second run gives other UUIDs. The three files go into the commit of the task.
- [x] 2.4 (recipe-db) Add the migrations of `design.md`, section STRING encoding. RED: `cargo test -p recipe-db --test storage schema` and `cargo test -p recipe-db --test storage constraints` fail.
  - Add `migrations/0017_machine_string_encoding.sql` and `migrations/0018_machine_string_encoding_validate.sql`.
  - Add the pieces of `0017` to `PIECES_AFTER_0001` in `crates/recipe-db/tests/storage/support/document.rs`.
  - `RETURNING to_jsonb(machine)` gives the new key `string_encoding` after `0017`. The shared contract compares the exact keys of each `machine` image on both adapters.
  - Add `("string_encoding", Kind::String)` to `machine_shape_with` of `crates/recipe-db/tests/storage/contract/images.rs`.
  - Add the key `string_encoding` with the literal `"Latin1"` to `machine_image` of `crates/recipe-app/src/memory/images.rs`. Task 11.1 replaces the literal with the field of the row.
  - Add the key to `machine_image` of `crates/recipe-app/src/memory/plant/tests.rs`. Chain A adds `StringEncoding` in the same wave, so chain C uses only the literal.
  - Run the shared machine contract on both adapters: `cargo test -p recipe-db --test storage contract::machine` and `cargo test -p recipe-app --lib memory::plant`.
  - Add one violation test for each check: `adr_0202_unknown_string_encoding_fails` and `adr_0202_utf8_on_s7_fails` give SQLSTATE `23514` with the name of the check.
  - Register `machine_string_encoding_value` and `machine_string_encoding_platform` in `plant::COVERED` of `crates/recipe-db/tests/storage/constraints/plant.rs`.
  - Add `migration_0017.rs` and `migration_0018.rs` after the pattern of `migration_0013.rs` and `migration_0015.rs`. A stored machine of each platform gets `Latin1`, and both checks are `NOT VALID` after `0017` and valid after `0018`.
  - In `migration_0017.rs`, an `INSERT` without `string_encoding`, as the previous release writes it, gets `Latin1`.
  - Set `EXPECTED_MIGRATIONS` in `crates/recipe-server/tests/migrate.rs` to 18.
  - `0017` holds the grant. Add the row `machine.string_encoding|UPDATE` to `crates/recipe-db/tests/grants.expected` and the row `GRANT|machine.string_encoding|UPDATE` to `deploy/pgtools/restore-roles.expected`.
  - Run `cargo test -p recipe-db --test storage restore_code` after the migrations exist. Copy the two `CHECK|public machine machine_string_encoding_*` lines of its list "extra" into `deploy/pgtools/restore-code.expected` in byte order.
  - Check: `python -I deploy/ci/migration_rewrite.py migrations` exits with 0. `deploy/ci/upgrade_smoke.py` needs no change, because the column has a default.
  - GREEN: `cargo test -p recipe-db --test storage adr_0202`, `schema`, `constraints`, `contract`, `restore_roles`, and `restore_code`, each with `cargo test -p recipe-db --test storage`.
  - Then run `cargo test -p recipe-app --lib memory` and `cargo test -p recipe-server --test migrate`.
- [x] 2.5 (recipe-db) Compare `0017` with the fourth SQL block of `data-model.md` §5.2. RED: `cargo test -p recipe-db --test storage grants::migration` fails.
  - In `grants/migration.rs`, skip the fourth SQL block of §5.2 by its index 3 in the three tests of `0003`, `0012`, and `0013`.
  - Add `adr_0202_migration_0017_holds_the_fourth_sql_block_of_section_5_2`.
  - GREEN: `cargo test -p recipe-db --test storage grants`.

## 3. WSTRING characters on CODESYS and TwinCAT (ADR-0201)

- [x] 3.1 (recipe-domain) In `recipe/value/leaf/tests.rs`, add the `adr_0201_*` tests of the requirement "WSTRING characters on CODESYS and TwinCAT" in `specs/recipe-management/spec.md`. RED: `cargo test -p recipe-domain --lib adr_0201`.
  - Then add the branch of `design.md` to `recipe/value/leaf.rs` and the variant `Expected::WstringIec { length }`. `ParameterValue::from_stored` under `RuleSet::Earlier` keeps ADR-0137 (b).
  - In `recipe-web`, map the variant to the key `expected-wstring-iec` in `expected_key`, with texts in `en.ftl` and `pl.ftl`.
  - Change each test that expects a character outside the Basic Multilingual Plane to pass on these platforms.
  - Add the `recipe-web` test `adr_0201_value_form_names_the_two_ranges` of the requirement "Texts of the new type reasons and expected forms".
  - GREEN: `cargo test -p recipe-domain --lib adr_0201`, `cargo test -p recipe-web --lib adr_0201_value_form`, and `cargo test -p recipe-web --lib expected`.
  - The end command can show one failure: the test of `types_TwinCat3_wstring_non_bmp.json` in `recipe-export --test golden_types`. Task 3.2, the next task of chain F, fixes it.
- [x] 3.2 (recipe-export) In `crates/recipe-export/tests/golden_types.rs`, change the test of `types_TwinCat3_wstring_non_bmp.json` (ADR-0201 (i), ADR-0202 (j2)).
  - Build the value through `ParameterValue::from_stored` under `RuleSet::Earlier`, because `ParameterValue::parse` now refuses it. Rename the test to start with `adr_0201_`.
  - The affected file gets `loadable` `false` and a rule fault. Its hashed body bytes and its `content_sha256` equal the values before the change. Only `loadable` changes.
  - Add `adr_0201_unaffected_golden_is_byte_identical`. The file `types_TwinCat3.json` of task 1.7 keeps its exact bytes.
  - RED: `cargo test -p recipe-export --test golden_types adr_0201` fails before the change of the golden file. GREEN: the same command and `cargo test -p recipe-export`.
- [x] 3.3 (recipe-app) Integration task for task 3.1. Add the use case tests `adr_0201_*` of the requirement "WSTRING characters on CODESYS and TwinCAT". They cover these cases: `SetParameterValue`, `AddParameter`, the rebind, a stored value in a draft, `ApproveVersion` with `E-RCP-020`, `CreateVersion`, and the export with `loadable` `false`. Command: `cargo test -p recipe-app --lib adr_0201`.
- [x] 3.4 (recipe-db) In `crates/recipe-db/tests/storage/platform_ranges.rs`, add `adr_0201_iec_wide_text_lists_only_iec_rows`.
  - Seed a stored `WSTRING` value outside the Basic Multilingual Plane on `TwinCat3` and on `S7_1500`. Expect only the `TwinCat3` row in the part `iec_wide_text`.
  - RED: `cargo test -p recipe-db --test storage platform_ranges::adr_0201`.
  - Then add the part `iec_wide_text` to `deploy/db/check-platform-ranges.sql` with a header comment that names ADR-0201 (k). GREEN: the same command.

## 4. Reserved names on CODESYS and TwinCAT (ADR-0203)

- [x] 4.1 (recipe-domain) Add the lists and the check of ADR-0203. RED: `cargo test -p recipe-domain --lib adr_0203`.
  - Add `crates/recipe-domain/src/plant/iec_names.rs` with the 15 struct names and the 2 extra keywords, as `design.md` places them.
  - Add `is_iec_reserved_struct_name`, `is_iec_reserved_member_name`, and `check_iec_names(platform, name, members)` for ADR-0203 (a) and (d). The struct row constructor does not change.
  - `check_iec_names` checks only the names. Task 5.1 adds the count of the members as a separate function.
  - Add `adr_0203_struct_names_are_new`: no name of the 15 is in a reserved set of ADR-0143 or ADR-0186, compared without case.
  - Add `adr_0203_member_keywords_are_the_keywords_of_adr_0143_and_two_more`: the 184 member keywords equal the 182 keywords of ADR-0143 plus `UNION` and `END_UNION`.
  - Add `DerivedTypeReason::ReservedMemberName`. Extend `derived_type_reasons_have_their_phrase` in `plant/error/tests.rs`.
  - Map the reason in `crates/recipe-web/src/error/plant.rs` to the key `type-reason-reserved-member-name`, with texts in `en.ftl` and `pl.ftl`. Add a key test.
  - GREEN: `cargo test -p recipe-domain --lib adr_0203` and `cargo test -p recipe-web --lib plant`.
- [x] 4.2 (recipe-app) Enforce ADR-0203 in the use cases, as `design.md` places it (ADR-0203 (i1) to (i3), (j), (k), (l)).
  - First add the `adr_0203_*` use case tests of the requirement "Reserved names on CODESYS and TwinCAT" in `specs/plant-model/spec.md`. RED: `cargo test -p recipe-app --lib adr_0203` fails.
  - The two preview scenarios of that requirement are not part of this task. They are "Stored reserved struct in the preview of the parameter form" and "Stored keyword member in the preview of the parameter form". Task 7.3 tests them.
  - The tests cover `AddParameter`, a changed and an unchanged `UpdateParameterDefinition`, and `DefineUdt` with a struct name, a member name, and a member type.
  - They also cover both proposal lists, `RebindTypeRevision`, and the valid name `ST_Coil@1` whose stored revision has the member `END_IF`.
  - Then add the pure function `check_iec_struct(types, expr)` in `plant/current_type.rs`. It finds the stored revision that `expr` names directly in `types` and calls `check_iec_names` on it.
  - Add `require_iec_struct(open, expr)` in `recipe/definition_steps.rs`. It calls `check_iec_struct(&open.types, expr)`. Call it in `add_parameter.rs` and `update_definition.rs`.
  - In `plant/define_udt.rs`, call `check_iec_names` on the new struct in `require_current_names`. That function gets no stored row.
  - In `ensure_members` of `plant/define_udt.rs`, the base of a member type can be `BaseExpr::Udt`, also as the element of an array. Find its stored row in `derived.rows()` and call `check_iec_names` on that row.
  - Change `is_refused_struct_name` to `is_refused_struct(platform, row)` in `plant/current_type.rs`. It calls `check_iec_names`. Call it in `recipe/queries/parameter_form.rs` and `plant/queries/types.rs`.
  - This task does not check the count of the members. Thus a one-member struct on `CodesysIec` still passes after this task.
  - Add the `recipe-web` test `adr_0203_struct_form_names_the_reserved_member_name` of the requirement "Texts of the new type reasons and expected forms".
  - A test of any crate that defines such a struct through `DefineUdt` changes in this task. It gets another name, or it builds the row through `from_stored`.
  - GREEN: `cargo test -p recipe-app --lib adr_0203` and `cargo test -p recipe-web --lib adr_0203`.
- [x] 4.3 (recipe-db) In `crates/recipe-db/tests/storage/identifier_rules.rs`, add the `adr_0203_*` tests.
  - Seed a stored struct `VERSION@1` on `TwinCat3`, `VERSION@1` on `S7_1500`, and a stored member `END_IF` on `CodesysIec`. Expect only the IEC rows in the parts `iec_reserved_struct_name` and `iec_keyword_member_name`.
  - Add `adr_0203_sql_lists_equal_the_domain_lists`.
  - RED: `cargo test -p recipe-db --test storage identifier_rules::adr_0203`. Then add both parts to `deploy/db/check-identifier-rules.sql`. GREEN: the same command.

## 5. Two members of a struct on CODESYS (ADR-0204)

- [x] 5.1 (recipe-domain) Add the check of ADR-0204 (a). RED: `cargo test -p recipe-domain --lib adr_0204`.
  - Add `check_codesys_members(platform, members)` in `plant/iec_names.rs`. It is separate from `check_iec_names` of task 4.1, and no use case calls it in this task.
  - Add `adr_0204_one_member_fails_on_codesys` and `adr_0204_one_member_passes_on_twincat` for `check_codesys_members`.
  - Add `DerivedTypeReason::TooFewMembers`. Map it in `recipe-web` to the key `type-reason-too-few-members`, with texts in `en.ftl` and `pl.ftl`.
  - The struct row constructor does not change. An empty member list still gives `E-PLT-016`.
  - GREEN: `cargo test -p recipe-domain --lib adr_0204` and `cargo test -p recipe-web --lib plant`.
- [x] 5.2 (recipe-app) Enforce ADR-0204 in the use cases, as `design.md` places it (ADR-0204 (c1) to (c3), (d), (e), (f)).
  - First add the `adr_0204_*` use case tests of the requirement "Two members of a struct on CODESYS". RED: `cargo test -p recipe-app --lib adr_0204` fails.
  - The scenario "Stored one-member struct in the preview of the parameter form" is not part of this task. Task 7.3 tests it.
  - The tests cover the cases of task 4.2 with the stored struct `ST_One@1` with one member on `CodesysIec`. They include a member of type `ST_One@1` in `DefineUdt`.
  - The RED tests fail, because no use case calls `check_codesys_members` before this task.
  - Then call `check_codesys_members` beside each call of `check_iec_names` of task 4.2. The places are `check_iec_struct`, `is_refused_struct`, `require_current_names`, and `ensure_members`. `require_iec_struct` gets the count check through `check_iec_struct`.
  - `RebindTypeRevision` does not call it (ADR-0204 (e)).
  - Add the `recipe-web` test `adr_0204_struct_form_names_too_few_members`.
  - A test of any crate that defines a one-member struct on `CodesysIec` through `DefineUdt` changes in this task. It gets a second member, or it builds the row through `from_stored`.
  - Candidates: `crates/recipe-db/tests/storage/rebind_shape.rs`, `plant/queries/udt_form_tests.rs`, `plant/define_udt/tests/adr_0177.rs`, `adr_0186.rs`, and `recipe/queries/parameter_form_tests.rs` of `recipe-app`.
  - GREEN: `cargo test -p recipe-app --lib adr_0204`, `cargo test -p recipe-web --lib adr_0204`, and `cargo test -p recipe-db --test storage rebind_shape`.
- [x] 5.3 (recipe-db) In `identifier_rules.rs`, add `adr_0204_one_member_struct_lists_only_codesys`.
  - Seed a stored struct with one member on `CodesysIec` and on `TwinCat3`. Expect only the `CodesysIec` row in the part `codesys_one_member_struct`.
  - RED: `cargo test -p recipe-db --test storage identifier_rules::adr_0204`. Then add the part to `deploy/db/check-identifier-rules.sql`. GREEN: the same command.

## 6. Long prefixes of IEC literals (ADR-0205)

- [x] 6.1 (recipe-domain) In the tests of `plant/iec_literal`, add `adr_0205_long_prefixes_give_the_short_count`. RED: `cargo test -p recipe-domain --lib adr_0205`.
  - Each of the 8 long prefixes gives the count of its short prefix, and `Display` writes the short prefix.
  - These texts fail: `ltime#1us` (lower case) and `S5TIME#1s` (no long prefix of `S5T#`). The test asserts the reason that the code gives for an unknown prefix.
  - On a `DTL` leaf of `S7_1500`, `DATE_AND_TIME#2024-01-01-00:00:00` fails with `IecLiteralReason::Prefix`, because the long prefix names `DATE_AND_TIME` (ADR-0205 (b)).
  - Then change `literal_prefix` in `plant/iec_literal/mod.rs`. GREEN: `cargo test -p recipe-domain --lib iec_literal`.
- [x] 6.2 (recipe-app) Integration task for task 6.1. Add the `adr_0205_*` use case tests of the requirement "Long prefixes of IEC literals": a value, a limit through `ReplaceLimits`, and a refused lower-case prefix. Command: `cargo test -p recipe-app --lib adr_0205`.

## 7. Vendor aliases of type names (ADR-0206)

- [x] 7.1 (recipe-domain) In `crates/recipe-domain/src/plant/type_expr/tests.rs`, add `adr_0206_aliases_give_the_catalog_name`. RED: `cargo test -p recipe-domain --lib adr_0206`.
  - Under `TypeExpr::parse`, the 4 aliases give the catalog name, also as the element of an array.
  - `TypeExpr::parse` gives `DerivedTypeReason::ReservedName` for `DT@1`, `TOD@1`, `LTIME_OF_DAY@1`, and `LDATE_AND_TIME@1` (ADR-0206 (d1), (d2)).
  - `T`, `D`, and `LD` stay grammar errors. `TypeExpr::from_stored("TOD", RuleSet::Earlier)` gives `DerivedTypeReason::Grammar`.
  - In `type_expr_malformed_texts_are_grammar_errors`, delete the text `DT`, because a bare `DT` now maps to `DATE_AND_TIME`.
  - Then change `parse_base` in `plant/type_expr.rs`, as `design.md` places it. GREEN: `cargo test -p recipe-domain --lib type_expr`.
- [x] 7.2 (recipe-app) Integration task for task 7.1. Add the `adr_0206_*` use case tests of the requirement "Vendor aliases of type names". A request with `TOD` stores `TIME_OF_DAY`, and the export of the approved version holds `TIME_OF_DAY`. A request with `DT@1` gives `E-PLT-013` with the reason "reserved name". Command: `cargo test -p recipe-app --lib adr_0206`.
- [x] 7.3 (recipe-web, recipe-app) Add the parse path of a request type text and the check of the form query (ADR-0206 (d3)). `design.md` places both. RED: `cargo test -p recipe-web --lib adr_0206` and `cargo test -p recipe-app --lib -- adr_0206_preview adr_0203_preview adr_0204_preview` fail.
  - First add the `adr_0206_*` HTTP tests: the preview `fields=1`, `AddParameter`, an edit, the element of an array, and the type of a struct member. Each sends `TOD` and gets `TIME_OF_DAY`.
  - An edit form that sends the unchanged stored reference `Int@1` keeps it.
  - The machine of one HTTP test has no struct `DT@1`. The preview `fields=1` and the save of the add form send `DT@1`. Each answer shows `E-PLT-013` with the reason "reserved name", and not "unknown struct".
  - A stored row with the type text `TOD` stays a corrupt row on the page.
  - The preview `fields=1` of the add form with `INT@1` shows `E-PLT-013` with the reason "unknown struct" of the request parser.
  - The helper `input` of `recipe/queries/parameter_form_tests.rs` uses `TypeExpr::parse` and the purpose `AddParameter`. It cannot build these requests.
  - Add a helper beside it that takes the purpose, the key, and the text. It parses the text with `TypeExpr::from_stored(text, RuleSet::Earlier)`, as `request_type_expr` does after a refusal.
  - Each edit case uses the purpose `UpdateParameterDefinition` and the key of the edited parameter. Each test asserts the reason of the returned `PlantError::DerivedTypeInvalid`, or a returned preview tree.
  - Add these `recipe-app` tests of the requirement "Reserved struct reference in the preview of the parameter form". Each name starts with `adr_0206_preview_`:
    - `adr_0206_preview_of_a_missing_reserved_struct_gives_reserved_name`: `M9` has no struct `DT@1`. The add form with `DT@1` and with `Int@9` gives "reserved name". A missing `ST_Zone@9` gives "unknown struct".
    - `adr_0206_preview_of_a_stored_reserved_struct_needs_an_unchanged_text`: `M9` holds a stored struct `DT@1` with two members from before ADR-0143, built through `from_stored`. The parameter `STAMP` has the type `DT@1`, and the parameter `START` has the type `INT`.
      The add form with `DT@1` gives "reserved name". The edit form of `START` with `DT@1` gives "reserved name". The edit form of `STAMP` with `DT@1` gives the grid of `DT@1`.
      The parameter `STAMPS` has the type `ARRAY[0..7] OF DT@1`. Its edit form with `ARRAY[-0..007] OF DT@1` gives the grid, because `expr_names_row` compares the canonical text.
    - `adr_0206_preview_of_a_siemens_reserved_struct_on_s7_only`: a missing `BEGIN@9` gives "reserved name" on `M5` (`S7_1500`) and "unknown struct" on `M8` (`TwinCat3`).
  - Add two `recipe-app` tests of the plant-model scenarios on a stored reserved struct and a stored one-member struct in the preview:
    - `adr_0203_preview_of_a_stored_reserved_revision_needs_an_unchanged_text`: the stored `VERSION@1` on `M8` (`TwinCat3`). The add form and the changed edit of `START` give "reserved name". The unchanged edit of `VER` gives the grid.
    - `adr_0203_preview_of_a_stored_keyword_member_needs_an_unchanged_text`: the stored `ST_Coil@1` with the members `END_IF` and `S` on `M9` (`CodesysIec`). The add form and the changed edit of `START` give "reserved member name". The unchanged edit of `COIL` gives the grid.
    - `adr_0204_preview_of_a_stored_one_member_struct_needs_an_unchanged_text`: the stored `ST_One@1` on `M9` (`CodesysIec`). The add form and the changed edit of `START` give "too few members". The unchanged edit of `ONE` gives the grid.
  - `type_expr_primitive_name_in_exact_case_stays_unknown_struct_under_both_rules` keeps its `INT@1` case. Such a text fails `request_type_expr` and never reaches the form query.
  - Then add `request_type_expr(text)` in `crates/recipe-web/src/pages/parameters/`. Call it at `definition.rs:141`, `prepare.rs:83`, and `fields.rs:407`.
  - Then change `GetParameterForm::execute` of `crates/recipe-app/src/recipe/queries/parameter_form.rs`. Before `preview_tree`, it calls `require_current_type(expr, machine.platform())`.
  - After that call, it calls `check_iec_struct(&types, expr)` of task 4.2, which task 5.2 extends with the count check.
  - It skips both calls for the purpose `UpdateParameterDefinition` when the form edits a stored parameter and `expr_names_row(&types, stored.plc_type, expr)` holds. `update_definition.rs:167` uses the same call.
  - A test that previews a stored reserved struct on the add form or with a changed type text changes in this task. An unchanged type text keeps its grid.
  - Candidates: `pages/parameters/edit_tests.rs` (the unchanged `Int@1` of `ZONE` keeps passing), `pages/parameters/stored_s7_tests.rs`, and `recipe/queries/parameter_form_tests.rs`.
  - `base_id` of `crates/recipe-app/src/recipe/queries/preview.rs` does not change. Thus the parity test `a_bad_expression_gives_the_same_code_and_reason_as_ensure_derived_type` keeps its cases.
  - GREEN: `cargo test -p recipe-web --lib adr_0206`, `cargo test -p recipe-web --lib parameters`, `cargo test -p recipe-app --lib -- adr_0206_preview adr_0203_preview adr_0204_preview`, and `cargo test -p recipe-app --lib preview`.

## 8. Rules of the export schema v2 (ADR-0208)

- [x] 8.1 (recipe-export) Add the module `platform_rules` to `crates/recipe-export/tests/schema.rs` with `adr_0208_*` tests. RED: `cargo test -p recipe-export --test schema adr_0208`.
  - Each failing document starts from a valid document and changes only one type position of ADR-0208 (a1): `parameters[].type_id`, `types[].members[].type_id`, or `types[].element_type_id`.
  - Every reference of a test document is a valid UUID of its `types` list, and it passes (ADR-0208 (a3)). A bare `STRING` and a bare `WSTRING` fail (ADR-0208 (a2)).
  - Each other item of ADR-0208 (b) to (d) gets one passing and one failing document.
  - A test compares the enum of each platform with the catalog of `recipe-domain`. Each golden v2 document still passes.
  - Then add the rules to `crates/recipe-export/schema/export-v2.schema.json`. GREEN: `cargo test -p recipe-export`.

## 9. Runbook and rollback check

- [x] 9.1 (docs/ops) In `docs/ops/runbook.md`, name the new parts near the existing runbook sections of the two ops queries of `deploy/db/`. The task reads no query file.
  - The parts are `iec_wide_text`, `iec_reserved_struct_name`, `iec_keyword_member_name`, and `codesys_one_member_struct`.
  - Say which ADR each part follows and what a listed row means for an upgrade. Use the two kinds of step 2 of the section Migration Plan in `design.md`.
  - Say that a prohibited struct deeper than the directly named revision is legacy data, and that these parts list it (`design.md`, Decisions).
  - The section "Roll back on the newer schema" already holds the pre-check and the query of ADR-0202 (q). That query is the one executable copy. Do not change it.
  - Check that the query has the two predicates of ADR-0202 (q3): `machine.string_encoding = 'Utf8'` and `audit_log.action = 'MachineStringEncodingSet'`.
  - Check: `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/ops/**/*.md'` reports 0 hard violations.
- [x] 9.2 (recipe-db) Check task for ADR-0202 (q3). In `crates/recipe-db/tests/storage/`, add `adr_0202_rollback_query_counts_utf8_machines_and_encoding_entries`.
  - No test reads a runbook block now. The test adds its own reader of `docs/ops/runbook.md`.
  - The reader finds the heading `#### Roll back on the newer schema`. It takes the lines between the first `<<'SQL'` after that heading and the next line `SQL`. The runbook holds four such blocks.
  - Each case runs in its own empty database.
  - Case 1: a fresh database gives `utf8_machines` 0 and `encoding_entries` 0.
  - Case 2: a `CodesysIec` machine with `Utf8` and a stored `STRING(4)` value `é` gives `utf8_machines` 1. The value has 2 UTF-8 bytes.
  - Case 3: `SetMachineStringEncoding` to `Utf8` and back to `Latin1` gives `utf8_machines` 0 and `encoding_entries` 2.
  - Command: `cargo test -p recipe-db --test storage adr_0202_rollback_query` (expect pass).

## 10. Stored data matrix (design.md, section Stored data matrix)

- [x] 10.1 (recipe-db) Integration task for groups 2 to 5 and 11. In `crates/recipe-db/tests/storage/stored_rules_iec.rs`, add one test for each row of the matrix, for example `adr_0201_stored_wstring_exports_with_its_hash`. Each test does the eight steps of task 9.2 of `siemens-type-rules` on `fixtures/stored_rules_iec.sql`:
  1. Restore into an empty database with `TestDatabase::create_empty`.
  2. Load the dump with `psql --set ON_ERROR_STOP=1 --single-transaction`.
  3. Upgrade with `recipe_db::migrate`. This step runs `0017` and `0018` on the data of the earlier release. Then `App::over` builds the `App`.
  4. Read with `GetVersion`. The stored type text, the stored value, and the encoding `Latin1` of the machine do not change.
  5. Export with `ExportScope::Version` and with `ExportScope::Unit(U1)`. Compare with `fixtures/stored_rules_iec.hashes` and `fixtures/stored_rules_iec.exports.json`.
  6. Copy with `CreateVersion`. The draft holds the same rows.
  7. Back up with `pg_dump --format=custom` into the `DumpDirectory` of the test.
  8. Restore the backup with `pg_restore --exit-on-error --single-transaction` into a second empty database. Step 5 gives the same captures.
  - Step 5 of an unaffected case asserts the exact bytes of both captures, both hashes, and the `loadable` value of the fixture (ADR-0202 (j2)).
  - Step 5 of each `WSTRING` case asserts `loadable` `false`, both stored hashes, and the body bytes of the fixture (ADR-0201 (i)).
  - No captured scope body holds `machine.string_encoding`. `ExportedVersion::loadable()` gives the flag of the matrix.
  - Command: `cargo test -p recipe-db --test storage stored_rules_iec`.
  - Evidence: the report shows the line `test result: ok. 6 passed; 0 failed; 1 ignored;` of this command.
- [x] 10.2 (recipe-export) Integration task for ADR-0208 (f). In `crates/recipe-export/tests/schema/platform_rules.rs`, add `adr_0208_stored_rows_pass_the_schema`. It builds the rows of ADR-0201, ADR-0203, and ADR-0204 of the matrix through `from_stored` under `RuleSet::Earlier`. Each v2 document passes the v2 schema file. Command: `cargo test -p recipe-export --test schema adr_0208_stored_rows`.
- [x] 10.3 (recipe-export) Pin task for ADR-0202 (j2). In `crates/recipe-export/tests/golden.rs`, add `adr_0202_latin1_codesys_unit_document`.
  - The production writer `write_scoped` writes the v2 document of a `CodesysIec` version with fixed export metadata, as the other tests of `golden.rs` do.
  - The version holds a `STRING(4)` parameter with the value `"é"`. On a `Latin1` machine, `é` is one character.
  - The version also holds the two `CodesysIec` struct rows of the matrix, `ST_Coil@1` and `ST_One@1`, built through `from_stored`. The `TwinCat3` rows and the `WSTRING` rows are not in this version.
  - Write the golden file `crates/recipe-export/tests/golden/recipe_R-200_v1_U1_CodesysIec.json`. The test compares the bytes, rebuilds `scope_sha256` from the scope body, and checks the document against the v2 schema file.
  - Each parameter has `s7_address` `null`, and the document holds no layout (A31, A53).
  - Command: `cargo test -p recipe-export --test golden adr_0202_latin1` (expect pass after the file exists).

## 11. STRING encoding: rules, use cases, pages, and export (ADR-0202, wave 4)

- [x] 11.1 (recipe-db) Machine read. Add `adr_0202_stored_machine_reads_its_string_encoding` to the machine adapter tests. RED: `cargo test -p recipe-db --test storage adr_0202_stored_machine`.
  - `Machine` and `StoredMachine` of `recipe-domain` get `string_encoding`.
  - The six statements of `design.md`, section Carrier of the encoding, read the column. The memory adapter of `recipe-app` reads and writes it.
  - `machine_image` of `crates/recipe-app/src/memory/images.rs` writes the field of the row in place of the literal `"Latin1"` of task 2.4.
  - `CreateMachineInput` gets the encoding only in task 11.11. Until then, a test gets a `Utf8` machine through `MemoryStore::seed_machine` (`crates/recipe-app/src/memory/plant/seed.rs:13`) with a `Machine` that holds `Utf8`.
  - The `recipe-web` test worlds already call `seed_machine`, so the tests of tasks 11.5 to 11.8 in `recipe-app` and in `recipe-web` use this path.
  - GREEN: the same command and `cargo test -p recipe-app --lib machine`.
- [x] 11.2 (recipe-db) Draft read. Add `adr_0202_stored_draft_reads_the_string_encoding`. RED: `cargo test -p recipe-db --test storage adr_0202_stored_draft`.
  - `StoredDraft` of `recipe-domain` gets `string_encoding`. `SELECT_HEADER` and `LOCK_HEADER` of `draft_rows.rs` join `machine`, as `design.md` states. `LOCK_HEADER` keeps `FOR UPDATE OF v`.
  - Change every struct literal of `StoredDraft`. Find them with `rg -n 'StoredDraft \{' crates`. The memory draft adapter of `recipe-app` sets the field.
  - GREEN: the same command and `cargo test -p recipe-app --lib draft`.
- [x] 11.3 (recipe-domain) Carrier. Add `adr_0202_type_tree_holds_the_string_encoding`. RED: `cargo test -p recipe-domain --lib adr_0202_type_tree`.
  - Add `PlatformTypes::collect_encoded(machine, platform, string_encoding, rows)` and `TypeTree::string_encoding()`.
  - `collect_encoded` keeps the filter of the owner: a row is kept when it is global or belongs to `machine`. The platform and the encoding cannot tell two machines apart.
  - The old `PlatformTypes::collect` stays until task 11.9 and gives `Latin1`. Its doc comment says that task 11.9 deletes it.
  - GREEN: `cargo test -p recipe-domain --lib type_tree`.
- [x] 11.4 (recipe-domain) Value rule. In `recipe/value/leaf/tests.rs`, add the `adr_0202_*` value tests of the requirement "STRING values on a Utf8 machine" and of the modified requirement on `max_length`. Each test name starts with `adr_0202_utf8_`. RED: `cargo test -p recipe-domain --lib adr_0202_utf8`.
  - Add `adr_0202_utf8_two_byte_scalars_respect_capacity`. In UTF-8, `"\u{FF}"` and `"\u{100}"` have 2 bytes each. `adr_0137_last_latin_1_character` of task 1.4 does not change.
  - On a `Utf8` machine, `STRING(1)` refuses both values with `Expected::StringUtf8`, and `STRING(2)` accepts both values.
  - Then add the rule and `Expected::StringUtf8 { length }` of `design.md`. Map the variant to `expected-string-utf8` in `recipe-web`, with texts in both bundles.
  - Add the `recipe-web` test `adr_0202_expected_string_utf8_names_the_length_in_bytes`. It writes the line of `Expected::StringUtf8 { length: 4 }` in `en` and in `pl`, and each line names the length 4 in bytes.
  - The draft trees of a request get the encoding only in task 11.6. Thus task 11.6 adds the HTMX test of the value form.
  - GREEN: `cargo test -p recipe-domain --lib leaf` and `cargo test -p recipe-web --lib expected`.
- [x] 11.5 (recipe-app) Machine read path group. Move these call sites to `collect_encoded` with the encoding of `Machine`:
  - `plant/queries/types.rs` (3), `plant/queries/usage.rs` (2), `plant/define_udt.rs` (`check_tree`), `recipe/queries/parameters.rs` (3), `recipe/create_version.rs`, and `recipe/queries/preview.rs`.
  - `check_tree` of `plant/define_udt.rs` gets the encoding from the `Machine` of `execute`. `plant/ensure_derived.rs` and `DerivedTypes::new` do not change in this task. Task 11.6 changes them.
  - `preview_tree` gets the encoding as an argument. Its production caller in `GetParameterForm::execute` of `recipe/queries/parameter_form.rs` passes the encoding of the `Machine` of the form.
  - The seven test calls of `preview_tree` in `recipe/queries/preview_tests.rs` pass `StringEncoding::Latin1`. Before this change, they stand at lines 95, 108, 125, 136, 142, 243, and 261.
  - The call of `DerivedTypes::new` in `preview_tests.rs` does not change here. Task 11.6 changes it.
  - RED first: `adr_0202_machine_read_paths_carry_utf8`. On a `Utf8` machine of the memory adapter, the parameter query and `CreateVersion` accept the stored `STRING(10)` value `"Łódź"`. The value has 7 UTF-8 bytes.
  - No use case can write that value before task 11.6. Thus the test seeds the machine, the version, and the value directly in the memory store.
  - The same test asks the add form for the preview of `STRING(10)`, without a parameter. It asserts that `preview.string_encoding()` of the returned tree gives `Utf8`.
  - The edit form checks the stored value through `collect_stored_types`, which task 11.6 moves. Thus this task does not test the edit form.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_machine_read_paths`.
- [x] 11.6 (recipe-app) Draft read path group. Move `recipe/draft_session.rs`, `recipe/approve_version/write.rs`, and `plant/ensure_derived.rs` to `collect_encoded`.
  - `Draft::load` of `crates/recipe-domain/src/recipe/draft/load.rs` copies the encoding from `StoredDraft` into `Draft`. `Draft::string_encoding()` returns it. `write.rs:44` reads it there.
  - `collect_stored_types` of `draft_session.rs` takes the encoding. Its callers pass it: `draft_session.rs:181` and `recipe/rebind.rs:144` from `Draft`, and the call in `GetParameterForm::execute` of `recipe/queries/parameter_form.rs` from `Machine`.
  - `DerivedTypes::new` of `plant/ensure_derived.rs` takes the encoding, and `platform_types()` at `ensure_derived.rs:77` passes it to `collect_encoded`.
  - The three production callers of `DerivedTypes::new` pass the encoding. `recipe/definition_steps.rs:75` and `recipe/rebind/new_types.rs:45` pass `open.draft.string_encoding()`. `plant/define_udt.rs:115` passes the encoding of its `Machine`.
  - The test callers of `DerivedTypes::new` pass `StringEncoding::Latin1`: `plant/ensure_derived/tests.rs`, `recipe/queries/preview_tests.rs`, and `plant/queries/type_text_tests.rs`.
  - RED first: `adr_0202_draft_read_paths_carry_utf8`. On a `Utf8` machine, `SetParameterValue` stores `"Łódź"` (7 UTF-8 bytes) in a `STRING(10)` leaf, and `RebindTypeRevision` keeps the value of a `STRING(10)` struct member.
  - The same test runs `UpdateParameterDefinition` with a retype from `STRING(10)` to `STRING(20)`. The parameter keeps `"Łódź"`. The rule of `Latin1` refuses `Ł`, so this case fails with a wrong encoding in `DerivedTypes`.
  - The same test opens the edit form of the parameter with the stored value `"Łódź"`. The form has no rule fault.
  - Add the `recipe-web` HTMX test `adr_0202_value_form_names_the_length_in_bytes` of the scenario "STRING bytes in the parameter form of a Utf8 machine" of `specs/web-ui/spec.md`.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_draft_read_paths` and `cargo test -p recipe-web --lib adr_0202_value_form`.
- [x] 11.7 (recipe-app) Version read path group. Move `recipe/queries/versions.rs`, `diff.rs`, `outdated.rs`, `approval_preview.rs`, and `recipe/approve_version/prepare.rs` to `collect_encoded`.
  - Each call site reads the encoding from the `Machine` of the version. `prepare.rs:150` does not read it from `ExportHeader` (ADR-0202 (k)).
  - `Snapshot` of `approve_version/prepare.rs` gets the field `string_encoding`. `read_snapshot` sets it from its `Machine` before the blocking closure. `check_and_hash` reads it there.
  - RED first: `adr_0202_version_read_paths_carry_utf8`. `ApproveVersion` approves a draft with the `STRING(10)` value `"Łódź"` (7 UTF-8 bytes) on a `Utf8` machine. The version, diff, and outdated queries read it.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_version_read_paths`.
- [x] 11.8 (recipe-app) Export read path. Move `export/document.rs` to `collect_encoded` with the encoding of the `Machine` of `read_rows`.
  - RED first: `adr_0202_export_read_path_carries_utf8`. The export of an approved version with the `STRING(10)` value `"Łódź"` (7 UTF-8 bytes) on a `Utf8` machine gives `has_rule_fault` `false`.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_export_read_path`.
- [x] 11.9 (workspace) [mechanical] Delete the old `PlatformTypes::collect`. Rename `collect_encoded` to `collect` in every crate.
  - RED first: in the tests of `plant/type_tree`, add `adr_0202_collect_takes_the_string_encoding`. It calls `PlatformTypes::collect(machine, platform, StringEncoding::Utf8, rows)` and asserts `TypeTree::string_encoding()`.
  - RED: `cargo test -p recipe-domain --lib adr_0202_collect` fails to compile, because the old `collect` has three arguments.
  - Then delete the old function and rename the new one. The new `collect` keeps the argument `machine`.
  - Each test and each test support call passes `StringEncoding::Latin1`, or the encoding of its machine.
  - GREEN: `cargo test -p recipe-domain --lib adr_0202_collect` passes.
  - Check: `rg -n 'collect_encoded' crates` prints nothing. `cargo test --workspace --all-features --locked` passes.
- [x] 11.10 (recipe-domain) Add the audit action `MachineStringEncodingSet` and its entry kind with the full `machine` row in both states. Map it in `crates/recipe-web/src/pages/audit/choices.rs`. Change the count of the action tests to 39.
  - Add the key `audit-action-machine-string-encoding-set` to `crates/recipe-web/i18n/en-approval.ftl` and `pl-approval.ftl`, beside `audit-action-machine-data-block-set`. The test `rows_each_action_has_a_word_in_both_bundles_and_none_equals_its_key` reads each action of `AuditAction::ALL`.
  - RED and GREEN: `cargo test -p recipe-domain --lib audit` and `cargo test -p recipe-web --lib audit`.
- [x] 11.11 (recipe-app) `CreateMachineInput` gets `string_encoding`. One task changes every builder of the input, so the task compiles. RED first: `adr_0202_create_machine_*` and `adr_0202_machine_form_*` tests.
  - `CreateMachine` checks the encoding against the platform before its transaction. `MachineCreated` holds the encoding in `after_state`.
  - The builders: `crates/recipe-web/src/pages/machines/form.rs`, `crates/recipe-db/tests/storage/atomic/plant.rs`, `plant/create_machine/tests.rs`, and `memory/fixtures/scenarios.rs` of `recipe-app`.
  - Add `string_encoding` to `INSERT_MACHINE` of `crates/recipe-db/src/transaction/machines.rs` and to its binds in `insert`. Without it, the column default stores `Latin1` for each new machine.
  - Add the `recipe-db` test `adr_0202_create_machine_commits_utf8` to `atomic/plant.rs`. `CreateMachine` creates a `CodesysIec` machine with `Utf8` on PostgreSQL.
  - The test reads `machine.string_encoding` of the row and the `after_state` of `MachineCreated`. Both hold `Utf8`.
  - The form `machine-new.html` gets the field `string_encoding` with `Latin1` selected. `E-PLT-053` gives a line under the field.
  - Add the row `("pages/machines/form.rs", "CreateFields", "string_encoding", ...)` to `crates/recipe-web/src/pages/xss_coverage_tests/registry.rs`.
  - The row uses a `Cover::NoText` constant with a reason, as `ROLE` does. The field holds a closed set of values, and the form never shows the sent text.
  - The web tests follow the requirement "STRING encoding field of the machine form" in `specs/web-ui/spec.md`.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_create_machine`, `cargo test -p recipe-web --lib adr_0202_machine_form`, `cargo test -p recipe-web --lib xss`, and `cargo test -p recipe-db --test storage adr_0202_create_machine`.
- [x] 11.12 (recipe-app) Port and adapters. Add the methods `machine_has_version` and the update of the encoding to `PlantRepository`, with no default body. RED first: the freeze tests.
  - Implement both methods in each implementer: `crates/recipe-db/src/transaction/plant.rs`, `memory/plant/mod.rs`, `plant/short_rows.rs`, and `recipe/queries/count_tests.rs` of `recipe-app`.
  - The `recipe-db` adapter uses the query of `data-model.md` §6. A test double that does not use the method gives an error, not `false`.
  - Add `adr_0202_machine_has_version_*` tests in `recipe-db` for the port method: each row of a discarded draft and of a deleted recipe counts.
  - The audit failure test needs the command of task 11.13, so task 11.13 adds it.
  - GREEN: `cargo test -p recipe-db --test storage adr_0202_machine_has_version` and `cargo test -p recipe-app --lib plant`.
- [x] 11.13 (recipe-app) Add `crates/recipe-app/src/plant/set_machine_string_encoding.rs` after the pattern of `set_machine_data_block.rs`.
  - First write the race test `adr_0202_encoding_change_and_create_recipe_serialize` in `crates/recipe-db/tests/storage/race/`. It covers both orders of the requirement "No new version during a change of the STRING encoding".
  - Then write `adr_0202_set_machine_string_encoding_fails_in_the_audit_insert_and_keeps_the_encoding` in `crates/recipe-db/tests/storage/atomic/plant.rs`. It follows `set_machine_data_block_fails_in_the_audit_insert_and_changes_neither_table`.
  - Also write the `adr_0202_set_machine_string_encoding_*` tests with the memory adapter. RED: the three commands of GREEN fail.
  - The tests cover the change, the freeze, the permission, and `409 E-PLT-010` for a machine that is archived under the lock. A refused command writes no row and no success audit entry.
  - The gate of `crates/recipe-web/src/gates.rs` refuses a command in read-only mode. Thus task 11.14 tests the read-only mode, and this task does not.
  - The use case checks `MachineManage` first and the platform before its transaction. Under the machine lock, it calls `machine_has_version`.
  - Add the operation to `crates/recipe-app/src/auth/first_statement.rs` and to `crates/recipe-app/src/audit/coverage/plant.rs`.
  - GREEN: `cargo test -p recipe-app --lib adr_0202_set_machine_string_encoding`, `cargo test -p recipe-db --test storage adr_0202_encoding_change`, and `cargo test -p recipe-db --test storage adr_0202_set_machine_string_encoding`.
- [x] 11.14 (recipe-web) The machine page shows the encoding. Add the route `POST /machines/{code}/string-encoding`, its form, and the fragment `machine-string-encoding-set.html`, after the pattern of the data block form.
  - The page query gives `has_version`. The page shows the form only on a live `CodesysIec` or `TwinCat3` machine without a version, and not in read-only mode.
  - Add the `adr_0202_machine_page_*` tests of the requirement "STRING encoding on the machine page". RED: `cargo test -p recipe-web --lib adr_0202_machine_page` fails.
  - The route uses the access of the operation `SetMachineStringEncoding`. Thus the gate of `crates/recipe-web/src/gates.rs` gives `503 E-WEB-006` in read-only mode.
  - HTTP tests: an unknown or archived machine gives `404 E-WEB-005` at the lookup. A direct `POST` in read-only mode gives `503 E-WEB-006`.
  - HTTP tests: an Operator with a valid token gives `403 E-WEB-002` with its `security_event` row. A missing or bad token gives `403 E-WEB-007` with a `csrf_rejected` event.
  - After each refused request, the machine keeps its encoding, and the audit log has no new entry.
  - Add the rows of the fields `string_encoding` and `csrf_token` of the new request struct to `pages/xss_coverage_tests/registry.rs`, after the rows of `DataBlockFields`. The row of `csrf_token` uses `CSRF`.
  - The row of `string_encoding` uses the `Cover::NoText` constant of task 11.11.
  - GREEN: the RED command and `cargo test -p recipe-web --lib xss_coverage`.
- [x] 11.15 (recipe-domain) In the module `export`, give `ScopeBody` a machine struct of its own with an optional `string_encoding`, as `design.md` places it. RED: `cargo test -p recipe-domain --lib -- adr_0202_scope_body adr_0202_v1_body` fails.
  - Add `adr_0202_scope_body_holds_utf8_only` and `adr_0202_v1_body_has_no_encoding`. A `Latin1` scope body gives the same bytes as before.
  - `ScopeBody::build(whole, scope, string_encoding)` takes the encoding as a parameter, because `HashedBody` never holds it (ADR-0202 (k)).
  - Change the three callers. `write_scope` of `crates/recipe-app/src/export/document.rs:178` gets a new parameter with the encoding of the `Machine` of `read_rows`.
  - `crates/recipe-export/tests/golden.rs:510` and `crates/recipe-domain/src/export/scope/tests.rs:74` pass `StringEncoding::Latin1`.
  - GREEN: the RED command, `cargo test -p recipe-app --lib export`, and `cargo test -p recipe-export --test golden`.
- [x] 11.16 (recipe-export) The v2 writer writes `machine.string_encoding` directly after `machine.plc_platform` for a `Utf8` machine only. RED: `cargo test -p recipe-export adr_0202` fails.
  - The v2 schema file gets the optional property with the enum `["Utf8"]`. An `if` and `then` pair permits it only on `CodesysIec` and `TwinCat3` (ADR-0202 (j3)). Change its description sentence on a new field, as ADR-0202 (j5) states.
  - Add `adr_0202_*` schema tests. A document without the field passes on each of the five platforms: `CodesysIec`, `TwinCat3`, `S7_300`, `S7_1200`, and `S7_1500`.
  - A document with `Utf8` passes on `CodesysIec` and `TwinCat3`.
  - A document with `Utf8` fails on `S7_300`, `S7_1200`, and `S7_1500`. A document with the explicit value `Latin1` fails.
  - Each golden v2 document stays byte-identical, also `recipe_R-200_v1_U1_CodesysIec.json` of task 10.3 (ADR-0202 (j2)).
  - GREEN: `cargo test -p recipe-export`.
- [x] 11.17 (recipe-domain) The `loadable` flag of a v1 document, as `design.md`, section Export of the encoding, places it (ADR-0202 (l)). The v1 hashed body does not change (ADR-0202 (k)). RED first: the tests of this task fail.
  - `DocumentFacts` gets `string_encoding` and `v1_loadable()`. The v1 writer in `crates/recipe-export/src/document.rs:136` writes `facts.v1_loadable()`. The v2 writer keeps `loadable()`.
  - `write_scope` of `crates/recipe-app/src/export/document.rs` selects the same flag for `Written` and `ExportedVersion`. `has_rule_fault` keeps its meaning.
  - Add `adr_0202_v1_of_utf8_is_not_loadable` and `adr_0202_v2_of_utf8_keeps_loadable` in `recipe-app`.
  - Add `adr_0202_export_bytes_*` HTTP tests in `recipe-web` with the production writer. Each request gets `200` with JSON, and then the test reads `loadable` in the response body.
  - Both production writers call `serde_json::to_vec_pretty` (`crates/recipe-export/src/document.rs:156`, `:205`). Thus the body holds a space after the colon, as line 5 of each golden file shows.
  - The test reads the flag with the helper `field(&reply.body, "loadable")` of `crates/recipe-web/src/pages/export/tests.rs:57`, or of `pages/export_api/tests.rs:105` for a machine route. The helper reads the pretty key `"loadable": `.
  - The tests use one approved version of `R-300` on the `Utf8` machine `M10`. Each path uses the credentials of its route in `api.md` §2.7:
    - An Operator session sends `GET /export/v1/recipes/R-300/approved`. The helper gives `false`.
    - A ProcessEngineer session sends `GET /export/v1/recipes/R-300/versions/1`. The helper gives `false`.
    - The `ExportClient` account `plc-m10` sends `GET /export/v1/machines/M10/recipes/R-300/approved` with HTTP Basic. The helper gives `false`.
    - The `ExportClient` account `plc-m10` sends `GET /export/v2/machines/M10/recipes/R-300/approved/units/U1` with HTTP Basic. The helper gives `true`.
  - GREEN: `cargo test -p recipe-app --lib -- adr_0202_v1_of_utf8 adr_0202_v2_of_utf8` and `cargo test -p recipe-web --lib adr_0202_export_bytes`.
- [x] 11.18 (recipe-export) Add the golden file `crates/recipe-export/tests/golden/recipe_R-300_v1_U1_TwinCat3_Utf8.json` and the test `adr_0202_utf8_twincat_unit_document` in `golden.rs`. RED: the test fails before the file exists.
  - The production writer writes the v2 document of a `TwinCat3` version on a `Utf8` machine with fixed export metadata. The version holds a `STRING(10)` value `"Łódź"`. The value has 7 UTF-8 bytes.
  - The test compares the bytes, rebuilds `scope_sha256` from the scope body, and checks the document against the v2 schema file.
  - Each parameter has `s7_address` `null`, and the document holds no layout (A31, A53).
  - With task 10.3, the golden files cover both encodings.
  - GREEN: `cargo test -p recipe-export --test golden adr_0202`.
- [x] 11.19 (recipe-web) The version page of an Approved version of a `Utf8` machine shows the notice of the v1 document (`specs/web-ui/spec.md`). RED: `cargo test -p recipe-web --lib adr_0202_version_notice` fails.
  - Add the key `version-export-encoding-notice` beside `version-export-rule-notice` in `en.ftl` and `pl.ftl`, with the texts of the requirement.
  - The export link names the notice in `aria-describedby`. A version with a rule fault on a `Utf8` machine shows both notices, and the link names both. The rule notice keeps its rule codes.
  - Tests: a healthy and a faulty version under `Latin1` and under `Utf8`. Each case shows only the notices of its state.
  - GREEN: the RED command.
- [x] 11.20 (recipe-app) Integration task for tasks 11.4 to 11.17. Add the `adr_0202_*` use case tests of the three requirements on `STRING` values in `specs/recipe-management/spec.md`. Add the export tests of the requirement "STRING encoding in the export document v2". Command: `cargo test -p recipe-app --lib adr_0202`.

## 12. Gate

- [x] 12.1 (workspace) Verification only. It changes no file. Run each command and show its output:
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features --locked`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
  - `python -I deploy/ci/migration_rewrite.py migrations`
  - `openspec validate codesys-twincat-type-rules --strict`
  - `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/codesys-twincat-type-rules/**/*.md'`

## Workflow follow-up

- Run `code-review` after group 12. Fix each CRITICAL, HIGH, and MEDIUM finding.
- Commit the review report and `docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md` in the PR of this change.
- Archive the change with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
