# Tasks: plc-type-ranges

The owner answered O1 to O4 of `design.md`. ADR-0118, ADR-0119, and ADR-0120 record the answers. No task waits for an answer.

Each code task starts with a failing test. Run it, see it fail, then change the code. After each code task, run the clippy, test, and fmt commands of `openspec/config.yaml`. A `[mechanical]` task goes to a Haiku subagent. Every other task goes to a Sonnet subagent.

## 1. Normative documents and error codes

- [x] 1.1 (docs/architecture) [mechanical] Haiku. In `reliability.md` §1.2, add the rows `E-RCP-055`, `E-RCP-056`, and `E-RCP-057` after `E-RCP-054`. Use the variants, the status 422, and the messages of `design.md` D1. Check: `ste-lint.py` on the file reports 0 hard violations.
- [x] 1.2 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §2.1, change the `DATE` row of the base-unit table to "`S7_300`, `S7_1200`, and `S7_1500`: 1990-01-01 to 2168-12-31 (ADR-0114)". Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.3 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §2.1, add ADR-0115 to the rows of `DATE_AND_TIME` and `DTL`. The `DTL` row ends at 2262-04-11-23:47:16.854775807. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.4 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §2.1, add the digit rule of ADR-0117 to the rows of `REAL` and `LREAL`. Add one sentence with the earlier rule set of ADR-0120 (a). Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.5 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §2.1 `S7Address`, add the end rule and the data block rule of ADR-0116 (a) and (c) after the alignment rule. Add the scope of ADR-0116 (e) in one sentence. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.6 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §5.2, add `E-RCP-055` to `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.7 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §5.2, add `E-RCP-056` and `E-RCP-057` to `AddParameter`, `UpdateParameterDefinition`, `RebindTypeRevision`, and `ApproveVersion`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.8 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §5.2, add the rule sets of ADR-0120 (d) to `SetParameterValue` and `ReplaceLimits`, and ADR-0120 (g) to `CreateVersion`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.9 (docs/architecture) [mechanical] Haiku. In `domain-model.md` §5.2 `ExportVersion`, change the rule of `loadable` to ADR-0119 (c). Cite ADR-0119. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.10 (docs/architecture) [mechanical] Haiku. In `reliability.md` §4.3 and §4.4, add one link to `domain-model.md` §2.1 for the rules of ADR-0116 and ADR-0117. Do not repeat the rules. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.11 (docs/architecture) [mechanical] Haiku. In `data-model.md` §3.12, after `parameter_s7_address_platform`, add one link to `domain-model.md` §2.1 for the address rules of ADR-0116. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.12 (docs/architecture) [mechanical] Haiku. In `data-model.md` §6, add the rule set of ADR-0120 (g) to the row of `CreateVersion`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.13 (docs/architecture) [mechanical] Haiku. In `data-model.md` §6, change the `loadable` text of the row of `ExportVersion` to ADR-0119 (c). Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.14 (docs/architecture) [mechanical] Haiku. In A43 of `overview.md` §9, write "DATE_AND_TIME counts whole seconds" and add ADR-0115 to the source column. Copy the text into the A43 row of `domain-model.md` §8. Check: the two rows are equal.
- [x] 1.15 (docs/architecture) [mechanical] Haiku. In `api.md` §2.6, add the new codes to the error lists of the parameter routes, as in tasks 1.6 and 1.7. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.16 (docs/architecture) [mechanical] Haiku. In `api.md` §2.5 and §2.6, replace each sentence with the code `E-DB-008` for a stored part by the read rule of ADR-0118. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.17 (docs/architecture) [mechanical] Haiku. In `api.md` §2.5, add the notice of ADR-0119 (l) and (m) to the version page. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.18 (docs/architecture) [mechanical] Haiku. In `api.md` §6.1, change the rule of `loadable` to ADR-0119 (c). In the §2.7 rows of the export routes, write "`loadable` follows section 6.1". Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.19 (docs/user) [mechanical] Haiku. In `how-to-export-a-version.md`, change the sentence "It is `true` only for an `Approved` version" to the rule of ADR-0119 (c). Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.20 (CHANGELOG.md) [mechanical] Haiku. Add an entry under the next release for ADR-0114 to ADR-0120 and the codes `E-RCP-055` to `E-RCP-057`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.21 (recipe-domain) [mechanical] After task 1.1, write a failing registry test that expects `E-RCP-055`, `E-RCP-056`, and `E-RCP-057` in `error_codes.rs`. Then add the three variants of design D1. Check: the registry tests of `recipe-domain` and `recipe-server` pass.

## 2. Rule set and DATE range (ADR-0114, ADR-0120)

- [x] 2.1 (recipe-domain) In `plant/iec_literal/range.rs`, write failing tests for the current set: on `S7_1200` and `S7_1500`, 72 683 passes and 72 684 gives `OutOfRange`. Write failing tests for the earlier set: 72 840 passes and 72 841 gives `OutOfRange`. On `S7_300`, 72 684 gives `OutOfRange` in both sets. Write failing draft tests: `D#2168-12-31` stores 72683, and `D#2169-01-01` gives `E-RCP-005`. A limit `max` of `D#2169-06-06` gives `E-RCP-006`. Then add `RuleSet`, `DATE_S7`, and `DATE_S7_EARLIER` (design D2). Check: the tests pass.

## 3. Ranges narrower than the vendor range (ADR-0115)

- [x] 3.1 (recipe-domain) Write tests for the scenarios of the requirement "Ranges narrower than the vendor range". Include the last `DTL` instant 2262-04-11-23:47:16.854775807 and the next nanosecond. The tests pin the current behavior, so they pass at once. Add no code. Check: `cargo test -p recipe-domain` passes, and each test names ADR-0115.

## 4. Significant digits (ADR-0117)

- [x] 4.1 (recipe-domain) In `recipe/value/decimal.rs`, write failing tests for a function that counts significant digits. Expect 1 for `1000000000`, 5 for `0.00012345`, 8 for `16777217`, and 0 for `0`. Then add the function and let `canonical` use it (design D3). Check: the tests pass.
- [x] 4.2 (recipe-domain) In `recipe/value/leaf.rs`, write failing tests. In the current set, `REAL` `16777217` and `LREAL` `0.1234567890123456` give a precision fault, and `REAL` `1234.567` passes. In the earlier set, `REAL` `16777217` passes. An approval of a stored `REAL` `16777217` gives `E-RCP-020` with the item `E-RCP-055`. Then add the check (design D3). Check: the tests pass.
- [x] 4.3 (recipe-domain) In `recipe/limit/parse.rs`, write failing tests. A request `REAL` limit with `max` `100.00001` gives `E-RCP-055`. A stored `REAL` limit with `min` `-100.00001` fails in the current set and passes in the earlier set. A stored `LREAL` limit with `max` `0.1234567890123456` behaves the same. An approval of a stored `REAL` limit with `max` `100.00001` gives the item `E-RCP-055`. Then add the check in `finish` (design D3). Check: the tests pass.

## 5. Address rules (ADR-0116)

- [x] 5.1 (recipe-domain) In `recipe/draft/s7.rs`, write failing tests: on `S7_300`, `DB16000.DBW0` passes and `DB16001.DBW0` gives `E-RCP-057`. On `S7_1500`, `DB16001.DBW0` passes. Then add the data block rule to `check_definition` and to `check_for_approval` (design D4). Check: the tests pass.
- [x] 5.2 (recipe-domain) In `recipe/draft/s7.rs`, write failing tests: `STRING(254)` at `DB1.DBB65280` passes, and at `DB1.DBB65282` gives `E-RCP-056`. `DINT` at `DB1.DBD65534` gives `E-RCP-056`. `BOOL` at `DB1.DBX65535.7` passes with one bit. Then add the end rule (design D4). Check: the tests pass.
- [x] 5.3 (recipe-domain) Write two failing tests. The approval of a stored `DINT` at `DB1.DBD65534` gives `E-RCP-056`. A rebind to a larger struct revision that ends after byte 65535 gives `E-RCP-056`. Then run both rules in `rebind/apply.rs` for each rebound parameter. Check: the tests pass.
- [x] 5.4 (recipe-domain) Keep the generators of `recipe/draft/s7/prop.rs` inside the new rules. Write a draft test: a draft holds `SPEED` of type `DINT` at `DB1.DBD65534`. `AddParameter` of `FEED` at `DB2.DBW0` passes, and `SetParameterValue` on `SPEED` passes. The test pins ADR-0116 (e1) and passes at once. Check: the property tests pass.

## 6. Application and web layers for the codes

- [x] 6.1 (recipe-app) In `recipe/queries/approval_preview.rs`, write a failing test: the blockers of `E-RCP-056` and `E-RCP-057` name the tag. Then add both variants to the tag match. Check: the test passes.
- [x] 6.2 (recipe-web) [mechanical] Write a failing test that the three codes give status 422. Then add them to `error/status.rs`. Check: the test passes.
- [x] 6.3 (recipe-web) Write failing tests for the field lines of ADR-0041. `E-RCP-056` and `E-RCP-057` give a line under the field `s7_address`. `E-RCP-055` gives a line under the value field or the limit field of its leaf. Then change `error/recipe.rs`, `pages/parameters/reply.rs`, `pages/parameters/grid.rs`, and `pages/versions/rebind_form.rs`. Check: the tests pass.
- [x] 6.4 (recipe-web) Write a failing test: the approval blockers list the three codes. Then change `pages/approval/blockers.rs`. Check: the test passes.
- [x] 6.5 (recipe-web) [mechanical] Write a failing test in `i18n/recipe_texts_tests.rs` for the three codes. Then add the texts to `i18n/pl.ftl` and `i18n/en.ftl`. Check: the test passes.
- [x] 6.6 (recipe-domain, recipe-web) Write failing tests. A `ReplaceLimits` with a `REAL` `max` of `100.00001` gives `E-RCP-055` with `field` `LimitMax`, and the line stands under the `max` input. A value with 8 digits gives `field` `Value`. Then add `PrecisionField` and the field to `RecipeError::PrecisionExceeded`. Check: the tests pass.

## 7. Commands with the earlier rule set (ADR-0120)

- [x] 7.1 (recipe-app) In `recipe/create_version/tests.rs`, write a failing test for `CreateVersion` from an approved base. The base holds the stored `DATE` 72700, the `REAL` `16777217`, a `REAL` limit `max` `100.00001`, and a `DINT` at `DB1.DBD65534`. The draft holds each part unchanged. A base with the stored `DATE` 72900 gives `E-RCP-020` and no draft. Check: the test fails until task 7.2.
- [x] 7.2 (recipe-domain) Write failing tests for `check_snapshot`. `VersionSnapshot::check` passes the base of task 7.1 and refuses 72900. `check_for_approval` gives the item `E-RCP-005` for the value 72700 and `E-RCP-006` for a limit `max` 72700. Then pass the rule set of design D6 to `check_snapshot` and the stored parsers. Update the comment at the start of `snapshot/check.rs`. Check: the tests of tasks 7.1 and 7.2 pass.
- [x] 7.3 (recipe-domain) Write failing draft tests. A draft holds `DUE` with the stored value 72700 and the stored limit `max` 72700. `SetParameterValue` on `DUE` with `D#2168-12-31` passes. Then `ReplaceLimits` on `DUE` with `max` `D#2168-12-31` passes. Check: the tests fail before the change.
- [x] 7.4 (recipe-domain) In the tests of task 7.3, add the other order. `ReplaceLimits` on `DUE` without `max` passes first. Then `SetParameterValue` with `D#2168-12-31` passes. A new value `D#2169-01-17` gives `E-RCP-005`. A new value above a stored `max` still gives `E-RCP-004`. Then change `set_value` and `replace_limits` (design D6). Check: the tests of tasks 7.3 and 7.4 pass.
- [x] 7.5 (recipe-domain) In the tests of task 7.3, add: a stored `REAL` limit `max` `100.00001` lets `SetParameterValue` with `50` pass. `RemoveParameter` on `DUE` passes. Check: the tests pass.

## 8. Reads of a stored part (ADR-0118)

- [x] 8.1 (recipe-domain) Write failing tests for the read check of design D7. A stored `DATE` 72700 on `S7_1500` gives the rule fault `E-RCP-005`. A stored limit `max` 72700 gives `E-RCP-006`. A stored `REAL` `16777217` gives `E-RCP-055`. Then add the read check. Check: the tests pass.
- [x] 8.2 (recipe-domain) In the tests of task 8.1, add: a stored `REAL` limit `min` `-100.00001` gives `E-RCP-055` of kind limit. A `DINT` at `DB1.DBD65534` gives `E-RCP-056`. `DB16001.DBW0` on `S7_300` gives `E-RCP-057`. A stored `DATE` 72900 and a text `abc` give a structure fault. Check: the tests pass.
- [x] 8.3 (recipe-domain) Write failing tests: `texts.rs` `literal_text` of the `DATE` count 72700 gives `D#2169-01-17`. A `DATE` leaf at `.zone.due` in a struct gives the same text. The text of a limit `max` 72700 gives the same text. `pairs.rs` `check_stored` accepts 72700 and refuses 72900. Then select the earlier set there (design D7). Check: the tests pass.
- [x] 8.4 (recipe-app) In `recipe/queries/parameters_tests.rs`, write a failing test for `ListParameters`. The stored `DATE` 72700 in `DUE` gives the value text and the fault `E-RCP-005`. The stored count 72900 still gives `E-DB-008`. Then change `value_texts_of`. Check: the tests pass.
- [x] 8.5 (recipe-app) In `recipe/queries/parameter_form_tests.rs`, write a failing test for the parameter page query. It gives the value fault `E-RCP-055`, the limit fault `E-RCP-006`, and the address fault `E-RCP-056`. Then change `parameter_form.rs`. Check: the test passes.
- [x] 8.6 (recipe-app) In `recipe/queries/diff_page_tests.rs`, write a failing diff test. A stored limit `max` 72700 gives the row with the fault `E-RCP-006`. A stored count 72900 still gives `E-DB-008`. Then change `diff_page.rs`. Check: the tests pass.
- [x] 8.7 (recipe-app) In `recipe/queries/approval_preview_tests.rs`, write failing tests. A draft fixes `DUE` to `D#2168-12-31` on a base with 72700. The preview gives a result with a line `E-RCP-005` on the base side. A draft with the stored 72700 gives the blocker `E-RCP-005`. Then change `approval_preview.rs` (design D7). Check: the tests pass.
- [x] 8.8 (recipe-web) In `pages/versions/parameter_table_value_tests.rs`, write a failing test. The table shows the value of `DUE`, the line `E-RCP-005` under it, and status 200. Then change the table template and its view. Check: the test passes.
- [x] 8.9 (recipe-web) Write failing tests for `pages/parameters/form_view.rs`. The page of a draft shows `RATE` `16777217` in the input and the line `E-RCP-055` under it. It shows the line `E-RCP-006` under the limit `max` and `E-RCP-056` under `s7_address`. Check: the tests fail before the change.
- [x] 8.10 (recipe-web) In the tests of task 8.9, add: the page of an approved version shows the same lines. A draft that `CreateVersion` copied from that version shows the same lines. Then change `form_view.rs`, `parameter-summary.html`, and `parameter-form.html`. Check: the tests of tasks 8.9 and 8.10 pass.
- [x] 8.11 (recipe-web) In `pages/diff/stored_tests.rs`, write a failing test. The diff page shows the limit row of task 8.6 with the line `E-RCP-006` and status 200. Then change the diff templates. Check: the test passes.
- [x] 8.12 (recipe-web) Write a failing test for the approval preview page of task 8.7. The diff shows the line `E-RCP-005` on the base side, and the status is 200. Then change the preview template. Check: the test passes.

## 9. Loadable flag (ADR-0119)

- [x] 9.1 (recipe-domain) In `export/facts/tests.rs`, write failing tests: `Approved` with no rule fault gives `loadable` `true`. `Approved` with a rule fault gives `false`. `Retired` gives `false`. Then add the result to `DocumentFacts` (design D8). Check: the tests pass.
- [x] 9.2 (recipe-domain) Write failing tests for the export check of design D8. The stored `DATE` 72700, a limit `max` 72700, the `REAL` `16777217`, and a `REAL` limit `min` `-100.00001` each give a rule fault. Check: the tests fail before the change.
- [x] 9.3 (recipe-domain) In the tests of task 9.2, add: on `S7_300`, a `DINT` at `DB1.DBD65534` and an `INT` at `DB16001.DBW0` give a rule fault. On `S7_1500`, `DB16001.DBW0` passes. The stored `DATE` 72900 gives a structure fault. Then add the function with the read check of task 8.1. Check: the tests pass.
- [x] 9.4 (recipe-app) In `export/tests.rs`, write a failing test for `ExportVersion` of an approved version with the stored `DATE` 72700. It gives `loadable` `false`, the stored `content_sha256`, and one `VersionExported` entry. Check: the test fails before the change.
- [x] 9.5 (recipe-app) In the tests of task 9.4, add: an `S7_300` version with `SPEED` at `DB1.DBD65534` gives `loadable` `false`. A changed hash still gives `E-EXP-002`. The stored `DATE` 72900 gives `E-DB-008`. Then call the check in `document.rs` and change `ExportedVersion::loadable` (design D8). Check: the tests pass.
- [x] 9.6 (recipe-app) Write a failing test: the unit export of `U1.2` gives `loadable` `false` when the affected parameter is in `U1` (ADR-0119 (f)). Then pass the whole-version result to the v2 facts. Check: the test passes.
- [x] 9.7 (recipe-export) In `tests/schema.rs`, write failing tests: an `Approved` document with `loadable` `false` passes the v1 and the v2 schema files. A `Retired` document with `loadable` `true` still fails. Then remove the `else` branch and change the description (ADR-0119 (g)). Check: the golden tests pass.
- [x] 9.8 (recipe-web) In `pages/export/tests.rs` and `pages/export_api/tests.rs`, write failing tests: each export route of an affected approved version gives `200` with `loadable` `false`. The file name has no `RETIRED_` prefix. Change the code only where a test fails. Check: the tests pass.

## 10. Notice on the version page (ADR-0119 (l), (m))

- [x] 10.1 (recipe-app) In `recipe/queries/version_tests.rs`, write a failing test. The version query of an approved version with the stored `DATE` 72700 gives the code `E-RCP-005`. A version without a rule fault gives no code. Then run the check of task 9.3 in `versions.rs` (design D9). Check: the test passes.
- [x] 10.2 (recipe-web) Write a failing test. The version page of task 10.1 shows the notice with `E-RCP-005` in the export area, and the link names it in `aria-describedby`. Then change the version page template and add the notice text to `pl.ftl` and `en.ftl`. Check: the test passes.

## 11. Integration checks

- [x] 11.1 (openspec) Run `openspec validate plc-type-ranges --strict`. Check: the command passes.
- [x] 11.2 (docs) Run `ste-lint.py` on the scope of ADR-0023 from Bash. Check: 0 hard violations.
- [x] 11.3 (workspace) Run `cargo clippy`, `cargo test --workspace --all-features --locked`, and `cargo fmt --check`. Check: all three pass.

## Workflow follow-up

- Review the code with `code-review` and fix every CRITICAL and HIGH finding.
- Archive the change with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
