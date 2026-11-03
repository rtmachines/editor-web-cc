# Tasks

Each code task starts with a failing test. A step marked "regression" adds a test for behavior that holds today. That test passes at once and guards the behavior.

All paths of a `(recipe-web)` task are under `crates/recipe-web/`. All paths of an `(e2e)` task are under `e2e/`. The line `Files:` of a task names every file that the task changes, also its test files.

The tasks of one group change disjoint files and can run in parallel, one agent for each task. The groups run in order (design D8).

Some tests stay red across groups on purpose:

- The browser checks of tasks 1.1, 1.2, and 2.1 fail until the code tasks that they check pass.
- The source guard of task 1.8 fails until group 4 ends. After each task that migrates a call site, the guard no longer lists that site.
- The `textarea` assertions of task 2.7 fail until group 3 ends.

## 1. Browser checks and shared parts

- [x] 1.1 (e2e) Add two tests to `tests/translations.spec.ts` (ADR-0111).
  - Test A, the deletion:
    1. Save the row `de`. Type a new name into `#translation-pl-name` and do not save it.
    2. Open the delete disclosure of `de` and press Enter on `#translation-de-delete`.
    3. Check that `#translation-pl-name` keeps the typed text and that the `h3` of `#translation-de` has the focus.
    4. Check that the field `row_version` of the form `pl` holds the value of the new form `de`. That value is larger than before the deletion.
    5. Save `pl` without a page load. Check the toast and an empty `#error-panel`, so no `409` occurred.
    6. Delete `pl`, which is now the last stored translation. Check that `#translation-pl` holds an empty form and no delete disclosure.
  - Test B, the base text: type into `#translation-pl-name`. Change `#name_en` of the edit form and save the definition. Check that `#translations-base` shows the new name and that `#translation-pl-name` keeps the typed text.
  - Run `npm test -- translations.spec.ts`. Test A fails before task 2.3. Test B fails before task 4.7.
  - Files: `e2e/tests/translations.spec.ts`
- [x] 1.2 (e2e) Add `tests/multi-line.spec.ts` (ADR-0112). On the page of the seeded parameter, fill the `textarea` `#description_en` with `Line 1`, a line end, and `Line 2`. Save and load the page again. Check the value of the `textarea` and two lines in the summary text. Change only a value, save, load again, and check the line end again. Run `npm test -- multi-line.spec.ts` and see it fail before group 3.
  - Files: `e2e/tests/multi-line.spec.ts`
- [x] 1.3 (recipe-web) Add the read-only switch to `examples/dev_server.rs` (design D7).
  - Add a failing test for a free async function that reads the switch file with `tokio::fs::try_exists`. It gives true for an existing file and false for a missing file.
  - Then add the variable `RECIPE_DEV_READ_ONLY_FILE` and the named constant `READ_ONLY_POLL` of 250 ms.
  - With the variable set, a task checks the file once per `READ_ONLY_POLL` and calls `ReadOnlySwitch::set` with the result. The task never calls a blocking file function.
  - Without the variable, the task does not start. Update the doc comment of the example.
  - Run `cargo test -p recipe-web --example dev_server`.
  - Files: `crates/recipe-web/examples/dev_server.rs`
- [x] 1.4 (recipe-web) Add failing tests for `PageRequest::may_run` in a test module of `src/pages/request.rs` (ADR-0113 (a)).
  - A ProcessEngineer gets `Ok(true)` for `UpdateParameterDefinition` in normal mode and `Ok(false)` in read-only mode.
  - An Operator gets `Ok(false)` for `UpdateParameterDefinition` in both modes.
  - An Operator gets `Ok(true)` for `ExportApprovedVersion`, `Logout`, and the query `GetVersion` in read-only mode.
  - A request without a session gets `Err` with `E-WEB-004`.
  - Then add `pub(crate) fn may_run(&self, operation: Operation) -> Result<bool, WebError>`. It returns the error of `auth()` unchanged. Its doc comment cites ADR-0113 (a).
  - Files: `crates/recipe-web/src/pages/request.rs`
- [x] 1.5 (recipe-web) Add failing tests to the new module `src/form_text.rs`. CRLF becomes LF. A lone LF stays. A lone CR stays. A text without a line end comes back unchanged. Then add the free function and declare the module in `src/lib.rs` (ADR-0112 (g)).
  - Files: `crates/recipe-web/src/form_text.rs`, `crates/recipe-web/src/lib.rs`
- [x] 1.6 (recipe-web) [mechanical] Add a failing test to `src/i18n/tests.rs`. The bundles `pl` and `en` hold the key `field-lines-limit` with the argument `max` and the key `readonly-form-unavailable`. Then add the keys to `i18n/pl.ftl` and `i18n/en.ftl`. The English text of `readonly-form-unavailable` says that the read-only mode blocks this form. Run `ste-lint.py` on the English text.
  - Files: `crates/recipe-web/src/i18n/tests.rs`, `crates/recipe-web/i18n/pl.ftl`, `crates/recipe-web/i18n/en.ftl`
- [x] 1.7 (recipe-web) Add failing assertions to `src/style_tests/component_rules.rs` and `src/style_tests/recipe_rules.rs`. Then change both style sheets with tokens only (`design-system.md` sections 1, 10.5, 12).
  - `static/app.css` holds the base field rules. Its base rule, its `:disabled` rule, its `aria-invalid="true"` rule, and its forced-colors rule add `textarea` next to `input` and `select`.
  - `static/components.css` gives `textarea` the value `resize: vertical`. Its class `.text-lines` sets `white-space: pre-wrap` and `overflow-wrap: anywhere`.
  - The `textarea` keeps the text font `--font-sans` of the base rule.
  - Files: `crates/recipe-web/static/app.css`, `crates/recipe-web/static/components.css`, `crates/recipe-web/src/style_tests/component_rules.rs`, `crates/recipe-web/src/style_tests/recipe_rules.rs`
- [x] 1.8 (recipe-web) Add the source guard `src/pages/read_only_guard_tests.rs` and declare it in `src/pages.rs` (design D5).
  - The guard reads every Rust source under `src/pages/`. It ignores all whitespace when it searches for `requirement().allows(`.
  - The guard skips `src/pages/request.rs` and the test files. A test file has a name that ends in `_tests.rs`, the name `tests.rs`, or a name that starts with `test_`.
  - A self-test of the search finds a chain on one line and a chain split over two lines. It finds nothing in `request.may_run(Operation::AddUnit)?`.
  - Run `cargo test -p recipe-web read_only_guard`. The self-test passes. The guard fails on the code of today and lists each file with a call site.
  - The list holds the split chains of `parameters/page.rs` and `audit/list.rs`.
  - Files: `crates/recipe-web/src/pages/read_only_guard_tests.rs`, `crates/recipe-web/src/pages.rs`

## 2. Translation deletion, macro, parse sites, and unit tree

- [x] 2.1 (e2e) Add the read-only browser test (ADR-0113 (h), design D7). This task needs task 1.3.
  - `tests/support/read-only.ts` exports the absolute path of the switch file: `path.resolve(__dirname, '..', '..', 'test-results', 'read-only-mode')`. The config and the spec import it.
  - `playwright.config.ts` gets a second `webServer` entry on the port `RECIPE_DEV_PORT + 1`. Its `env` sets `RECIPE_DEV_READ_ONLY_FILE` to that path.
  - The config gets a second project `read-only` with the base URL of that port and `testMatch: /read-only\.spec\.ts/`. The project `chromium` gets `testIgnore` with the same pattern.
  - `tests/read-only.spec.ts` uses the fixture `test` of `support/seed.ts`. That fixture seeds the instance at the base URL of the project.
  - The spec creates the file and waits for the banner. On the parameter page it checks the read-only grid.
  - It checks that `form[action="/logout"]` and `form[action="/lang"]` are present and that no other `form[method="post"]` is present.
  - It checks the notice on `/recipes/new`. It deletes the file in `afterAll`.
  - The spec does not check the export link, because the seed holds no `Approved` version. Task 4.2 checks that link.
  - Run `npm test -- read-only.spec.ts` and see it fail before group 4.
  - Files: `e2e/playwright.config.ts`, `e2e/tests/read-only.spec.ts`, `e2e/tests/support/read-only.ts`
- [x] 2.2 (recipe-web) Add failing tests to `src/components_tests.rs` for the macro `lines` of `templates/components/field.html`. This task needs task 1.6.
  - The `textarea` has the id, the name, `rows="4"`, `maxlength`, `lang`, and the required mark.
  - The limit line has the id `<id>-limit`. `aria-describedby` names the hint, the limit line, and the error line in this order.
  - A value with a leading LF, a trailing LF, and `<b>` parses back unchanged from the HTML.
  - Then add the macro (ADR-0112 (a) to (f)).
  - Files: `crates/recipe-web/src/components_tests.rs`, `crates/recipe-web/templates/components/field.html`
- [x] 2.3 (recipe-web) Add failing tests to `src/pages/parameters/translation_tests.rs` (ADR-0111, design D2).
  - The delete form has `hx-target="closest .translation-row"` and `hx-swap="outerHTML"`.
  - The HTMX answer of a deletion of `de` holds `#translation-de` with an empty form and no other row. Its `h3` has `tabindex="-1"` and `autofocus`.
  - The deletion of the last stored translation gives its row with an empty form and no delete disclosure.
  - A tag outside the list gives the row with `translation-none`.
  - The answer sets `HX-Trigger` with the new `row_version` and holds the toast out of band. A `409` changes no row.
  - Then change `templates/translation-row.html` and `remove` in `src/pages/parameters/translation_routes.rs`.
  - Drop the dead toast parts: the field `toast` of `TranslationsBlock`, its value in `src/pages/parameters/page.rs`, and the toast branch of `templates/translations.html`.
  - Update the doc comments of `src/pages/parameters/translations.rs` and the comments of `translations.html` and `translation-row.html`. No comment says that a deletion answers every row.
  - Files: `crates/recipe-web/src/pages/parameters/translation_tests.rs`, `crates/recipe-web/src/pages/parameters/translation_routes.rs`, `crates/recipe-web/src/pages/parameters/translations.rs`, `crates/recipe-web/src/pages/parameters/page.rs`, `crates/recipe-web/templates/translation-row.html`, `crates/recipe-web/templates/translations.html`
- [x] 2.4 (recipe-web) Add a failing test to `src/pages/parameters/edit_tests.rs`. A save of the edit form with `description_en` `Line 1` CRLF `Line 2` stores `Line 1` LF `Line 2`. Add a regression assertion: a lone CR gives `422` with a field error of `description_en`. Then call the function of task 1.5 in `src/pages/parameters/definition.rs` (design D3).
  - Files: `crates/recipe-web/src/pages/parameters/edit_tests.rs`, `crates/recipe-web/src/pages/parameters/definition.rs`
- [x] 2.5 (recipe-web) Add a failing test to `src/pages/recipes/form_tests.rs` (design D3).
  - `POST /recipes` with a change note of 3 990 letters and 10 CRLF line ends creates the recipe. The stored note has 4 000 characters.
  - Add a regression case: a change note of 4 000 characters `𝄞` (U+1D11E) creates the recipe. A note of 4 001 such characters gives a field error of `change_note`.
  - Then call the function of task 1.5 in `src/pages/recipes/form.rs`.
  - Files: `crates/recipe-web/src/pages/recipes/form_tests.rs`, `crates/recipe-web/src/pages/recipes/form.rs`
- [x] 2.6 (recipe-web) Add a failing test to `src/pages/versions/form_tests.rs`. `POST /recipes/R-100/versions` with a change note of `Step 1` CRLF `Step 2` stores `Step 1` LF `Step 2`. Then call the function of task 1.5 in `src/pages/versions/form.rs` (design D3).
  - Files: `crates/recipe-web/src/pages/versions/form_tests.rs`, `crates/recipe-web/src/pages/versions/form.rs`
- [x] 2.7 (recipe-web) Change the form assertions of the multi-line fields in `src/pages/recipe_xss_tests.rs` (ADR-0112 (e)).
  - The fields are `description_en`, `translation-pl-description`, and `change_note` of `POST /recipes` and of `POST .../versions`.
  - Each payload must come back escaped as the text of the `textarea`, not as a `value` attribute. The single-line fields keep `assert_input_value`.
  - The preview and diff checks of `approval_text_xss_tests.rs` and the audit checks of `approval_xss_tests.rs` do not change in this task.
  - Run `cargo test -p recipe-web recipe_xss` and see these assertions fail before group 3. Edit no template.
  - Files: `crates/recipe-web/src/pages/recipe_xss_tests.rs`
- [x] 2.8 (recipe-web) Add failing tests to `src/pages/machines/tree_page_tests.rs` (ADR-0113 (b)).
  - In read-only mode the full page `GET /machines/{code}/units` shows no add link to an Administrator. Its HTMX answer shows none either.
  - Then change `UnitTree::build` in `src/pages/machines/tree.rs`. It takes `can_add: bool` in place of the role.
  - Change every caller. A page caller passes `request.may_run(Operation::AddUnit)?`. The test callers pass a literal.
  - Files: `crates/recipe-web/src/pages/machines/tree_page_tests.rs`, `crates/recipe-web/src/pages/machines/tree.rs`, `crates/recipe-web/src/pages/machines/tree_tests.rs`, `crates/recipe-web/src/pages/machines/tree_page.rs`, `crates/recipe-web/src/pages/machines/page.rs`, `crates/recipe-web/src/pages/units/add.rs`

## 3. Multi-line fields and read-only pages of the plant and the users

- [x] 3.1 (recipe-web) Add failing tests to `src/pages/parameters/page_tests.rs` (ADR-0112).
  - The edit form holds a `textarea` `description_en` with the stored line end.
  - The summary and the English base text of `#translations` show the description in an element with the class `text-lines`.
  - Change the assertion of the base texts in both blocks to the new markup.
  - Then use `fields::lines` in `templates/parameter-form.html`. Add the class in `templates/parameter-summary.html` and in the base block of `templates/translations.html`.
  - Run `cargo test -p recipe-web recipe_xss` and see the assertions of task 2.7 for `description_en` pass.
  - Files: `crates/recipe-web/src/pages/parameters/page_tests.rs`, `crates/recipe-web/templates/parameter-form.html`, `crates/recipe-web/templates/parameter-summary.html`, `crates/recipe-web/templates/translations.html`
- [x] 3.2 (recipe-web) Add failing tests to `src/pages/parameters/translation_tests.rs` (ADR-0112).
  - The row `de` holds a `textarea` `translation-de-description` with `lang="de"`.
  - The read-only row shows the description with the class `text-lines`. A save with CRLF stores LF.
  - Change the existing assertions of the `input` and of the `dd` count to the new markup.
  - Then change `templates/translation-row.html` and `check_row` in `src/pages/parameters/translation_routes.rs`.
  - Run `cargo test -p recipe-web recipe_xss` and see the assertions of task 2.7 for `translation-pl-description` pass.
  - Files: `crates/recipe-web/src/pages/parameters/translation_tests.rs`, `crates/recipe-web/src/pages/parameters/translation_routes.rs`, `crates/recipe-web/templates/translation-row.html`
- [x] 3.3 (recipe-web) Add failing tests for the change notes (ADR-0112).
  - The forms of a new recipe and a new version hold a `textarea` `change_note`. Change the existing `input` assertions of both form tests.
  - The version page and the version table show the note with the class `text-lines`.
  - Then change `templates/recipe-new.html`, `templates/version-new.html`, `templates/version.html`, and `templates/versions-table.html`.
  - Run `cargo test -p recipe-web recipe_xss` and see the assertions of task 2.7 for `change_note` pass.
  - Files: `crates/recipe-web/src/pages/recipes/form_tests.rs`, `crates/recipe-web/src/pages/versions/form_tests.rs`, `crates/recipe-web/src/pages/versions/page_tests.rs`, `crates/recipe-web/src/pages/versions/list_tests.rs`, `crates/recipe-web/templates/recipe-new.html`, `crates/recipe-web/templates/version-new.html`, `crates/recipe-web/templates/version.html`, `crates/recipe-web/templates/versions-table.html`
- [x] 3.4 (recipe-web) Add a failing test to `src/pages/diff/page_tests.rs`. The cells of `description_en` and of a translation description use the class `text-lines` (ADR-0112 (h)).
  - Then change `templates/components/diff.html` and `src/pages/diff_view.rs`.
  - Change the exact cell markup in `diff_lang_tests.rs`, `approval/preview_rows_tests.rs`, and `approval_text_xss_tests.rs` to the class. Keep every escaping check.
  - Files: `crates/recipe-web/src/pages/diff/page_tests.rs`, `crates/recipe-web/src/pages/diff_lang_tests.rs`, `crates/recipe-web/src/pages/approval/preview_rows_tests.rs`, `crates/recipe-web/src/pages/approval_text_xss_tests.rs`, `crates/recipe-web/templates/components/diff.html`, `crates/recipe-web/src/pages/diff_view.rs`
- [x] 3.5 (recipe-web) Add failing tests for the machine pages (ADR-0113).
  - In read-only mode an Administrator gets no link to a new machine and no rename, archive, or add-unit control.
  - `GET /machines/new` shows the notice.
  - Then use `may_run` in `src/pages/machines/list.rs`, `page.rs`, and `form.rs`, and change `templates/machine-new.html`.
  - Files: `crates/recipe-web/src/pages/machines/list_tests.rs`, `crates/recipe-web/src/pages/machines/machine_tests.rs`, `crates/recipe-web/src/pages/machines/form_tests.rs`, `crates/recipe-web/src/pages/machines/list.rs`, `crates/recipe-web/src/pages/machines/page.rs`, `crates/recipe-web/src/pages/machines/form.rs`, `crates/recipe-web/templates/machine-new.html`
- [x] 3.6 (recipe-web) Add failing tests for the unit pages (ADR-0113).
  - In read-only mode the unit page has no rename, archive, or add control.
  - The form route of a new unit shows the notice.
  - Then use `may_run` in `src/pages/units/page.rs` and `form.rs`, and change `templates/unit-form-page.html` and `templates/unit-form.html`.
  - Files: `crates/recipe-web/src/pages/units/page_tests.rs`, `crates/recipe-web/src/pages/units/form_tests.rs`, `crates/recipe-web/src/pages/units/page.rs`, `crates/recipe-web/src/pages/units/form.rs`, `crates/recipe-web/templates/unit-form-page.html`, `crates/recipe-web/templates/unit-form.html`
- [x] 3.7 (recipe-web) Add failing tests for the type pages (ADR-0113).
  - In read-only mode the type list has no link to a new UDT. The type page has no link to a new revision.
  - `GET /machines/{code}/types/new` and `GET /machines/{code}/types/new?from={name}` show the notice.
  - Then use `may_run` in `can_define` of `src/pages/types.rs` and in `src/pages/types/form.rs`, and change `templates/udt-form.html`.
  - Files: `crates/recipe-web/src/pages/types/list_tests.rs`, `crates/recipe-web/src/pages/types/tree_tests.rs`, `crates/recipe-web/src/pages/types/form_tests.rs`, `crates/recipe-web/src/pages/types.rs`, `crates/recipe-web/src/pages/types/form.rs`, `crates/recipe-web/templates/udt-form.html`
- [x] 3.8 (recipe-web) Add failing tests to `src/pages/users/tests.rs` (ADR-0113).
  - In read-only mode the user list has no link to a new user. The user page has no action form and no reset disclosure.
  - `GET /users/new` shows the notice.
  - Then pass the flag in `src/pages/users/templates.rs` and `query.rs`, and change `templates/users.html`, `user.html`, and `user-new.html`.
  - Files: `crates/recipe-web/src/pages/users/tests.rs`, `crates/recipe-web/src/pages/users/templates.rs`, `crates/recipe-web/src/pages/users/query.rs`, `crates/recipe-web/templates/users.html`, `crates/recipe-web/templates/user.html`, `crates/recipe-web/templates/user-new.html`
- [x] 3.9 (recipe-web) Add failing tests to `src/pages/own/tests.rs` (ADR-0113 (g)).
  - In read-only mode `GET /me/password` shows the notice and the logout button, also for a session with `must_change_password`.
  - The page `/me` has no link to the password form. The login form stays.
  - Then change `src/pages/own/templates.rs`, `templates/password.html`, and `templates/me.html`.
  - Files: `crates/recipe-web/src/pages/own/tests.rs`, `crates/recipe-web/src/pages/own/templates.rs`, `crates/recipe-web/templates/password.html`, `crates/recipe-web/templates/me.html`
- [x] 3.10 (recipe-web) Add a failing test to `src/pages/approval/preview_page_tests.rs` (ADR-0113).
  - In read-only mode the approval preview shows the diff and the blockers. It holds no approve form and shows the notice in place of the form.
  - Then change `src/pages/approval/preview.rs` and `templates/approve.html`.
  - Files: `crates/recipe-web/src/pages/approval/preview_page_tests.rs`, `crates/recipe-web/src/pages/approval/preview.rs`, `crates/recipe-web/templates/approve.html`

## 4. Read-only pages of the recipes and the English base text

- [x] 4.1 (recipe-web) Add failing tests to `src/pages/parameters/page_tests.rs` (ADR-0113).
  - In read-only mode the page of a draft parameter shows the read-only grid and the translation texts.
  - The page holds `form[action="/logout"]` and `form[action="/lang"]` and no other form with `method="post"`.
  - `GET .../parameters/{key}/translations` holds no form in read-only mode.
  - Then use `may_run` in `is_editable` of `src/pages/parameters/page.rs` and in `src/pages/parameters/translation_routes.rs`.
  - Files: `crates/recipe-web/src/pages/parameters/page_tests.rs`, `crates/recipe-web/src/pages/parameters/page.rs`, `crates/recipe-web/src/pages/parameters/translation_routes.rs`
- [x] 4.2 (recipe-web) Add failing tests for the version page (ADR-0113).
  - In read-only mode a draft page has no link "Add a parameter", no link "Approve", no rebind form, and no danger disclosure.
  - The compare form and the export link of an approved version stay.
  - Then pass `may_run` to `version_actions` in `src/pages/versions/actions.rs`. Use it in `page.rs`, `rebind_form.rs`, and `rebind.rs`.
  - Files: `crates/recipe-web/src/pages/versions/actions_tests.rs`, `crates/recipe-web/src/pages/versions/page_tests.rs`, `crates/recipe-web/src/pages/versions/rebind_form_tests.rs`, `crates/recipe-web/src/pages/versions/actions.rs`, `crates/recipe-web/src/pages/versions/page.rs`, `crates/recipe-web/src/pages/versions/rebind_form.rs`, `crates/recipe-web/src/pages/versions/rebind.rs`
- [x] 4.3 (recipe-web) Add a failing test to `src/pages/versions/route_tests.rs` (ADR-0113 (d)).
  - In read-only mode `GET /recipes/R-100/versions/new` shows the notice and no form that sends `POST`.
  - The Operator still gets `403 E-WEB-002`.
  - Then change the form route in `src/pages/versions/form.rs` and `templates/version-new.html`.
  - Files: `crates/recipe-web/src/pages/versions/route_tests.rs`, `crates/recipe-web/src/pages/versions/form.rs`, `crates/recipe-web/templates/version-new.html`
- [x] 4.4 (recipe-web) Add failing tests for the recipe list and the recipe page (ADR-0113).
  - In read-only mode the recipe list has no link to a new recipe and no restore form.
  - The recipe page has no rename form, no delete disclosure, and no link to the form of a new version.
  - Then use `may_run` in `src/pages/recipes/list.rs` and `page.rs`.
  - Files: `crates/recipe-web/src/pages/recipes/list_tests.rs`, `crates/recipe-web/src/pages/recipes/page_tests.rs`, `crates/recipe-web/src/pages/recipes/list.rs`, `crates/recipe-web/src/pages/recipes/page.rs`
- [x] 4.5 (recipe-web) Add a failing test to `src/pages/recipes/route_tests.rs` (ADR-0113 (d)).
  - In read-only mode `GET /recipes/new` shows the notice and no form that sends `POST /recipes`.
  - The Operator still gets `403 E-WEB-002`.
  - Then change the form route in `src/pages/recipes/form.rs` and `templates/recipe-new.html`.
  - Files: `crates/recipe-web/src/pages/recipes/route_tests.rs`, `crates/recipe-web/src/pages/recipes/form.rs`, `crates/recipe-web/templates/recipe-new.html`
- [x] 4.6 (recipe-web) [mechanical] Add regression tests to `src/pages/audit/list_tests.rs` and `src/pages/history/tests.rs` (design D5).
  - The links of the audit list and of the history page stay in read-only mode. These links lead to queries, so the tests pass today.
  - Then use `may_run` in `src/pages/audit/list.rs` and `src/pages/history.rs`. The guard of task 1.8 no longer lists these two files.
  - Files: `crates/recipe-web/src/pages/audit/list_tests.rs`, `crates/recipe-web/src/pages/history/tests.rs`, `crates/recipe-web/src/pages/audit/list.rs`, `crates/recipe-web/src/pages/history.rs`
- [x] 4.7 (recipe-web) Add a failing test to `src/pages/parameters/edit_tests.rs` (ADR-0111 (i), design D9).
  - An HTMX save of the edit form with a new `name_en` and `description_en` holds `#translations-base` out of band. The block shows the new texts, and the description has the class `text-lines`.
  - The answer holds no element with the class `translation-row`.
  - Then put the base block of `templates/translations.html` into a macro with the literal `id="translations-base"`. The page renders it without `hx-swap-oob`. The answer of the save renders it with the literal `hx-swap-oob="true"`.
  - Add the fragment struct in `src/pages/parameters/translations.rs`. Append the fragment in `saved` of `src/pages/parameters/edit.rs`.
  - Files: `crates/recipe-web/src/pages/parameters/edit_tests.rs`, `crates/recipe-web/src/pages/parameters/edit.rs`, `crates/recipe-web/src/pages/parameters/translations.rs`, `crates/recipe-web/templates/translations.html`

## 5. Form of a new parameter

- [x] 5.1 (recipe-web) Add failing tests to `src/pages/parameters/new_tests.rs` (ADR-0113 (d)). This group runs after group 4, because `src/pages/parameters/page.rs` also builds `ParameterPage`.
  - In read-only mode `GET .../parameters/new` answers `200` with the notice and no form that sends `POST .../parameters`.
  - The test sends the request three times: as a full page, with `HX-Target: content`, and with `HX-Target: parameter-form`.
  - Then change `src/pages/parameters/new.rs`. Change `form_page` and `form_fragment` in `src/pages/parameters/reply.rs`, because both always render the form today.
  - Give `ParameterPage` in `src/pages/parameters/summary.rs` a notice. Set it to `None` in `src/pages/parameters/page.rs`.
  - Show the notice in `templates/parameter.html`. Add `templates/parameter-notice.html` for the fragment of `HX-Target: parameter-form`.
  - Files: `crates/recipe-web/src/pages/parameters/new_tests.rs`, `crates/recipe-web/src/pages/parameters/new.rs`, `crates/recipe-web/src/pages/parameters/reply.rs`, `crates/recipe-web/src/pages/parameters/summary.rs`, `crates/recipe-web/src/pages/parameters/page.rs`, `crates/recipe-web/templates/parameter.html`, `crates/recipe-web/templates/parameter-notice.html`

## 6. Guard and browser run

- [x] 6.1 (recipe-web) Run `cargo test -p recipe-web read_only_guard` and see the guard of task 1.8 pass. A remaining call site gets its own small task with a failing test (design D5).
  - Files: none
- [x] 6.2 (e2e) Run `npm test` in `e2e/` with `E2E_PASSWORD` set. The checks of tasks 1.1, 1.2, and 2.1 and every other spec pass.
  - Files: none

## 7. Docs and close

- [x] 7.1 (CHANGELOG.md) [mechanical] Add to `[Unreleased]`. Cite ADR-0111, ADR-0112, and ADR-0113. Run `ste-lint.py CHANGELOG.md`.
  - Under `Added`: the multi-line fields and the notice of a form route in read-only mode.
  - Under `Changed`: the deletion of a translation keeps the other rows. Read-only pages hide the controls of changes.
  - Under `Fixed`: a save of the definition shows the new English base text above the translations.
  - Files: `CHANGELOG.md`
- [x] 7.2 (docs/user) Update `docs/user/reference-screens.md` and `docs/user/how-to-edit-a-parameter-in-a-draft.md`. Describe the multi-line description field, the deletion of one translation row, the new English base text after a save, and the pages in read-only mode. Run `ste-lint.py` on both files.
  - Files: `docs/user/reference-screens.md`, `docs/user/how-to-edit-a-parameter-in-a-draft.md`
- [x] 7.3 (docs/adr) [mechanical] After the user approves this change, set the status of ADR-0111, ADR-0112, and ADR-0113 to "Accepted". Change each file and the index of `docs/adr/README.md`. Run the `ste-lint.py` command of CLAUDE.md.
  - Files: `docs/adr/0111-deletion-answer-of-one-translation-row.md`, `docs/adr/0112-multi-line-text-field.md`, `docs/adr/0113-read-only-aware-pages.md`, `docs/adr/README.md`
- [x] 7.4 (workspace) Run `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --check`, the `ste-lint.py` command of CLAUDE.md, and `openspec validate web-ui-data-safety --strict`. Show the passing output.
  - Files: none

## Workflow follow-up

- Run `code-review` on the diff and fix every CRITICAL and HIGH finding.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
