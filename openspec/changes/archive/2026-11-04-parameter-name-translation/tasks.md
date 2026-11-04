# Tasks

Each task is for one agent. The mark after the scope names the agent: `[sonnet]` (effort high) or `[mechanical]` (Haiku). `design.md` D5 gives the run order of tasks 1.2 to 4.3. A `[mechanical]` agent runs no `ste-lint.py`. Task 6.1 runs the STE checks.

## 1. Decision records and normative documents

- [x] 1.1 (docs/adr) [mechanical] After the owner accepts ADR-0126, set its Status to "Accepted". Add its row to the index of `docs/adr/README.md`. Add ADR-0126 to the decision map row "Translations". Add "Amended by ADR-0126 (marker of the English base text)" to the Status line of ADR-0012 and to its index row. Check: `git diff --stat -- docs/adr` names only ADR-0012, ADR-0126, and `README.md`.
- [x] 1.2 (docs/architecture) [mechanical] In `api.md` section 8, add three sentences after the sentence of the marker `(en)`. Copy them exactly:
  - The rule applies to a parameter name or description without a label. A labeled field of the English base text shows the English base text in each UI language (ADR-0126 (f)).
  - With the UI language `en`, the English base text appears without the marker (ADR-0126).
  - In a page of another language, the English base text has `lang="en"`, and the marker stands outside it.
  - Check: `git diff --stat -- docs/architecture` names only `api.md`.
- [x] 1.3 (docs/architecture) [mechanical] In `api.md` section 3, fix the answer column of the row `POST .../parameters/{key}/definition` (`UpdateParameterDefinition`). Replace the first sentence of the cell with the new sentence below. Keep the rest of the cell.
  - Old sentence: `200` parameter row fragment.
  - New sentence: `200` parameter summary fragment and the clean form out of band (ADR-0041 (h), ADR-0126 (h)).
  - Run this task after task 1.2, because both tasks change `api.md`.
  - Check: `git diff --stat -- docs/architecture` names only `api.md`, and `git diff -- docs/architecture/api.md` shows no other changed row of section 3.

## 2. Application

- [x] 2.1 (recipe-app) [sonnet] Write a test of the memory adapter in `memory/recipe/reads_tests/parameters.rs`. A version holds one parameter with a stored `pl` translation and one without. `parameter_page` with `Some(pl)` gives `translated_name` for the first row and `None` for the second. `parameter_page` with `None` gives `None` for both. Write a test of `ListParameters` in `queries/parameters_tests.rs` that passes `lang: Some(pl)` and finds the translated name in `rows`. Run them and see them fail. Then apply `design.md` D1 and the `recipe-app` part of D3. Fill the memory adapter from the stored `translations`. The new field and the new argument break the callers below. Update each one. Before the edit, run `rg -n "ListParametersInput \{|\.parameter_page\(|ParameterRow \{|fn parameter_page" crates` and add each new hit to this list.
  - `recipe-app` code: `recipe/queries/parameters.rs:361` passes `lang`. `recipe/queries/versions.rs:487` passes `None`. `recipe/queries/approval_preview.rs:316` sets `translated_name: None`.
  - `recipe-app` memory adapter: `memory/recipe/reads.rs:233` (`table_row`) and `:378` (`parameter_page`).
  - `recipe-app` tests: `recipe/queries/parameters_tests.rs:181`, `:368`, `:374`, `:378`, and `recipe/queries/parameter_value_tests.rs:76`, `:187`.
  - `recipe-app` tests: `recipe/queries/count_tests.rs:205` and `:215` (the counting adapter), and `:508`.
  - `recipe-app` tests: the seven calls of `memory/recipe/reads_tests/parameters.rs` at `:67`, `:82`, `:110`, `:115`, `:120`, `:145`, `:151`.
  - `recipe-db` code: `src/transaction.rs:189` takes the argument. `src/transaction/parameter_reads.rs:95` returns `translated_name: None`. Task 3.1 replaces that body.
  - `recipe-db` tests: `tests/storage/contract/recipe_reads/parameter_reads.rs:29` and `:77` (a `ParameterRow` literal), and `repeatable_read.rs:26`.
  - `recipe-web` code: `pages/versions/page.rs:239`, `pages/parameters/table.rs:71`, and `pages/versions/rebind.rs:264` pass `lang: None`. Tasks 4.2 and 4.3 replace that value.
  - Check: `cargo test -p recipe-app parameter_page` and `cargo test -p recipe-app list_parameters` pass, and `cargo check --workspace --all-targets` passes.

## 3. Storage

- [x] 3.1 (recipe-db) [sonnet] Write the contract case `parameter_page_gives_the_translated_name_of_the_requested_language` for the memory and the PostgreSQL adapter in `tests/storage/contract/recipe_reads/parameter_reads.rs`. One parameter has a `pl` and a `de-AT` translation, and one has none. On both adapters, the case sends four requests:
  - A request for `pl` gives the `pl` name for the first row and `None` for the second row.
  - A request for `de-AT` gives the `de-AT` name for the first row and `None` for the second row.
  - A request for a language tag without a stored translation, for example `cs`, gives `None` for both rows.
  - A request with `None` gives `None` for both rows. The page cut of two rows still holds two parameters.
  - Add `translated_name` to `PAGE_COLUMNS`. Run the tests and see them fail. Then write the join and the column of `design.md` D2 in `SELECT_PAGE` and its record.
  - Check: `cargo test -p recipe-db parameter_reads` and `cargo test -p recipe-db parameter_page` pass. The tests need Docker.

## 4. Web

- [x] 4.1 (recipe-web) [sonnet] Write unit tests of `shown_name` for the three cases of `design.md` D4. The cases are a translation, no translation with `Pl`, and no translation with `En`. Write a render test of `ParameterRowFragment` that asserts the cell `<td><span lang="en">Line speed</span> (en)</td>`. Keep the case `<b>Speed</b>` of `parameter_table_tests.rs` for both arms of `ShownName`, so a translated name and `name_en` are both escaped. Run the tests and see them fail. Then add `Texts::lang()`, `ShownName`, `shown_name`, the template branch of `components/parameter.html`, and the bundle key of D4. `row_view` and `added` call `shown_name`. `added` passes `None`. Update `parameter_table_tests.rs`. The test of `pages/texts/tests.rs` already checks the bundle key, because `shown_name` names the key as a literal in `t.get`. Check: `cargo test -p recipe-web parameter_table` and `cargo test -p recipe-web shown_name` pass.
- [x] 4.2 (recipe-web) [sonnet] Write the HTTP tests of the five scenarios of the requirement "Translation fallback" in the spec delta. Give each test name the prefix `translation_fallback_`. Store the `pl` translation through the route `POST .../translations/pl` or the fixture. Write a unit test of `Lang::translation_lang` for both arms. `Pl` gives `Some(pl)`, so the test parses the literal `pl`. `En` gives `None`. Run the tests. See the scenario "Stored Polish translation" and the unit test fail. Then add `Lang::translation_lang` and its error mapping of `design.md` D3. Pass its value in `pages/versions/page.rs:239`. Check: `cargo test -p recipe-web translation_fallback` passes and reports at least five tests, and `cargo test -p recipe-web translation_lang` passes.
- [x] 4.3 (recipe-web) [sonnet] Write three HTTP tests with the cookie `recipe_lang` set to `pl` and a stored `pl` translation. Give each test name the prefix `translation_fallback_`. Each test asserts the translated name in the row and no marker `(en)`:
  - The HTMX table fragment `GET .../parameters?page=1` (`pages/parameters/table.rs:71`).
  - The table fragment of the answer of `RemoveParameter` (`pages/parameters/table.rs:1` to `:5`).
  - The table of the rebind answer (`pages/versions/rebind.rs:264`).
  - Add a test of the large draft with the cookie `recipe_lang` set to `pl` that keeps the body under `BYTE_BOUND` (`pages/versions/large_tests.rs`). Its comment records the measured size of the `pl` page.
  - Run the tests and see the first three fail. Then pass the value of `Lang::translation_lang` in `table.rs:71` and `rebind.rs:264`.
  - Check: `cargo test -p recipe-web translation_fallback` passes and reports at least eight tests, and `cargo test -p recipe-web versions_large_draft` passes.

## 5. Release notes

- [x] 5.1 (CHANGELOG.md) [mechanical] Add one line to the section "Unreleased" under "Fixed". The parameter table and the row of a new parameter show the translation for the UI language (ADR-0012). Without it, they show the English name with the marker `(en)` and `lang="en"` (ADR-0126). Do not change the section of an earlier release. Check: `git diff --stat -- CHANGELOG.md` names one file, and `git diff -- CHANGELOG.md` shows the new line under "Unreleased".

## 6. Checks

- [x] 6.1 (docs) [sonnet] Run `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'` and the same linter on the files of this change. Fix each hard violation without a change of meaning. Check: `ste-lint.py` reports 0 hard violations, and `openspec validate parameter-name-translation --strict` passes.
- [x] 6.2 (workspace) [sonnet] Run `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, and `cargo doc --no-deps` with `RUSTDOCFLAGS="-D warnings"`. Check: the four commands pass, and the output is in the report.

## Workflow follow-up

- Code review with the skill `code-review` (Opus) and a Codex second review. Fix every CRITICAL and HIGH finding.
- `superpowers:verification-before-completion`, then `openspec-archive-change` after `cargo test` and `openspec validate` pass.
