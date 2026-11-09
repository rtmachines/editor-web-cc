# Tasks

Marks after the scope:

- `[sonnet]`: a Sonnet subagent with the effort `high` does the task.
- `[opus]`: an Opus subagent does the check task. It changes no file.
- `[mechanical]`: a Haiku subagent does the task.

Each task is one agent run. Each task of a crate starts with a failing test and shows the red step before the fix. The red step and the fix stay in one task. The tree must pass `cargo test` after each task (design, "Findings that differ from the plan", item 4).

A pinning test passes before its task. Its red step uses one mutation:

1. Copy the file to the scratch directory.
2. Apply the mutation and run the test. The test fails on the mutation.
3. Revert the mutation and run `cmp <file> <copy>`. `cmp` gives no output.

The report of each task gives the test name, the red step, and the failure line.

The tree passes `cargo test --workspace --all-features --locked` before task 1.1 and after each task. Each code task runs the full tests of each crate that it changes. The orchestrator runs the workspace command after the last code task.

## Order

The agents share one working tree. A red step of one task breaks the build of each other task in the same crate. The code tasks therefore run one after the other:

1. 1.1, 1.2, 1.3, 1.4, 3.1, 3.2, 3.3, in this order. Each task starts after the previous task ends with a green crate test.
2. Tasks 2.3, 4.1, and 4.2 change no file of a crate. They can run in parallel with the code tasks.
3. Task 4.3 starts after 4.1 and 4.2. Task 5.1 starts after 1.3. Task 5.2 starts after every other task.

Plan section 5.2 tasks 1 to 3 have no task here. D5-2 does not reproduce, and the owner dropped the fix on 2026-11-08 (design, "Findings that differ from the plan", item 1). The tests `lang_next_approval_for_the_content_sets_the_switch_to_the_version_page` and `lang_next_parameter_removal_for_the_content_sets_the_switch_to_the_version_page` already exist.

## 1. Shown name in the diff sections (D5-3, D5-4)

- [x] 1.1 (recipe-domain) [sonnet] Carry the translated names in the part of an added or a removed parameter (ADR-0152 (b), (d), design D1).
  - Test file: `crates/recipe-domain/src/recipe/diff/tests.rs`.
  - Test: an added pair whose new side has the `pl` translation `Posuw` gives `translated_names` with the one entry `pl` to `Posuw`.
  - Test: a removed pair takes the names of the old side. A pair without a translation gives an empty map.
  - Red step: the test does not compile, because the field does not exist.
  - Fix: add the field to `DiffPart::Parameter` in `diff/rows.rs` and fill it in `parameter_row` of `diff/values.rs`.
  - Update the struct literal `speed_row` of `diff/tests.rs`.
  - Add the field to the pattern of `crates/recipe-web/src/pages/diff_view.rs` line 624 as `translated_names: _`, so the workspace compiles.
  - The patterns of `diff/type_tests.rs` and `diff_view/faults.rs` use `..` and need no change.
- [x] 1.2 (recipe-web) [sonnet] Show the shown name in two sections of the diff page (ADR-0152 (a), (e) to (g), design D3).
  - Test file: `crates/recipe-web/src/pages/diff_name_tests.rs`. Copy the world of `diff_lang_tests.rs`.
  - Test, `pl`: an added parameter with the `pl` translation `Posuw` shows `Posuw`, no marker, and no `Feed rate`.
  - Test, `pl`: an added parameter without a translation shows `<span lang="en">Feed rate</span>` and then the marker outside the span.
  - Test, `pl`: a removed parameter with the `pl` translation `Prędkość linii` shows `Prędkość linii`, no marker, and no `Line speed`.
  - Test, `en`: an added parameter shows `Feed rate` without the marker. Its cell has no `lang` attribute.
  - Test, `en`: a removed parameter shows `Line speed` without the marker. Its cell has no `lang` attribute.
  - Red step: the first test fails, because the cell shows `Feed rate`.
  - Fix: the argument `translation` of `DiffView::new`, the field `marker` of `Cell`, the call of `shown_name`, and the span in `templates/components/diff.html`.
  - Fix: `pages/diff.rs` and `pages/approval/preview.rs` pass the translation language to `DiffView::new`.
  - Change `approval_text_xss_name_en_is_escaped_in_the_added_row_and_the_definition_row` of `approval_text_xss_tests.rs` (design, finding 7):
    1. On the English pages, assert one `text_cell` of the payload (the definition row). Assert one `<td>` cell with the escaped payload (the added or the removed row).
    2. Add a run of the same pages in `pl` through `world.get(&world.data.admin, "pl", path)`. Assert `assert_safe` on each answer.
    3. On the `pl` pages, assert one `text_cell` of the payload and one `<span lang="en">` with the escaped payload.
    4. Assert that the marker `(en)` follows outside the span.
  - Keep each `assert_safe` call and each count of that test. Keep `diff_lang_tests.rs` green.
- [x] 1.3 (recipe-web) [sonnet] Pin the shown name in the two sections of the approval preview (ADR-0152 (a), web-ui "Translation fallback").
  - Test file: `crates/recipe-web/src/pages/diff_name_tests.rs`.
  - Test, `pl`: the preview of a draft that adds `FEED` without a translation shows `Feed rate` in `lang="en"` and the marker outside it.
  - Test, `pl`: with the `pl` translation `Posuw`, the preview shows `Posuw` without the marker.
  - Test, `pl`: the preview of a draft that removes `SPEED` with the `pl` translation `Prędkość linii` shows `Prędkość linii` without the marker.
  - Test, `en`: the preview of a draft that removes `SPEED` shows `Line speed` without the marker. Its cell has no `lang` attribute.
  - Red step: the mutation passes `None` as the argument `translation` in `pages/approval/preview.rs`. The second test fails.
- [x] 1.4 (recipe-web) [sonnet] Tag the English base text inputs of the parameter form (D5-4, ADR-0126 (f), design D4).
  - Test file: the form tests of `crates/recipe-web/src/pages/parameters/`.
  - Test, `pl`: the edit form and the new form hold `lang="en"` on the input `name_en` and on the text area `description_en`.
  - Test: the two labels carry no `lang` attribute.
  - Red step: the test fails, because the input has no `lang`.
  - Fix: pass `"en"` as the argument `lang` in `templates/parameter-form.html` lines 13 and 14.

## 2. Switch target of a pushed address (D5-2)

- [x] 2.3 (recipe-web) [opus] Check that no handler sets `HX-Push-Url` to a path outside the known places (plan section 5.2 task 4, design D5).
  - Command: `git grep -n -E "HX_PUSH_URL|hx-push-url|with_push_url|with_pushed_path" -- crates/recipe-web/src ':!*tests*' ':!*test_helpers.rs' ':!*xss_kit.rs'`.
  - Expected: three calls of `with_push_url` set the version path: `approve.rs` line 204, `actions.rs` line 338, and `remove.rs` line 117.
  - Expected: `version_layout` calls `with_pushed_path` at `pages/versions/page.rs` line 136.
  - Expected: each other place that sets a value sets `false`, as the table of design D5 shows.
  - Check that the tests of `lang_next_tests.rs` lines 48, 59, 70, and 86 cover the scenarios of web-ui "Language switch after a pushed address".
  - Report each other place with its file, its line, and its value.

## 3. Focus of the out-of-band forms (D5-1)

- [x] 3.1 (recipe-web) [sonnet] Give the submit button of the own password form the fixed `id` `password-submit` (ADR-0041 (e), design D6).
  - Test file: `crates/recipe-web/src/pages/own/password_tests.rs`.
  - Test: `GET /me/password` holds `id="password-submit"` on the submit button of `#password-form`.
  - Test: the `422` answer of an HTMX `POST /me/password` with a new password of 3 characters swaps the form with the same button `id`.
  - Red step: the first test fails, because the button has no `id`.
  - Fix: add the `id` in `templates/password.html` line 12, after `type="submit"` and before `class`.
  - Change `review_fix_single_action_forms_have_a_primary_button` of `review_fix_tests.rs`. The path `/me/password` expects `<button type="submit" id="password-submit" class="btn btn-primary">`. The other two literals stay.
- [x] 3.2 (recipe-web) [sonnet] Give the submit button of the new user form the fixed `id` `user-new-submit` (ADR-0041 (e), design D6).
  - Test file: `crates/recipe-web/src/pages/users/form_error_tests.rs`.
  - Test: `GET /users/new` and the refused `POST /users` hold `id="user-new-submit"` on the submit button of `#user-new-form`.
  - Red step: the test fails, because the button has no `id`.
  - Fix: add the `id` in `templates/user-new.html` line 10, after `type="submit"` and before `class`.
  - Change `review_fix_single_action_forms_have_a_primary_button` of `review_fix_tests.rs`. The path `/users/new` expects `<button type="submit" id="user-new-submit" class="btn btn-primary">`. The literal of `/login` stays.
- [x] 3.3 (recipe-web) [sonnet] List every out-of-band form and assert a fixed `id` on each submit button (plan section 5.3 task 3, design D7).
  - Test file: `crates/recipe-web/src/components_tests/oob_forms.rs`. Add the module to `components_tests`.
  - Test: the scan of design D7 finds the 15 forms of its table. Each form has at least one submit button, and each submit button has an `id`.
  - The scan counts a `<button` without `type="button"` or `type="reset"` as a submit button. It also counts `<input type="submit">`.
  - The scan expands the macros of `components/grid.html` that a form calls, as design D7 steps 2 and 3 give.
  - Red step: four mutations, one after the other. Each mutation makes the test fail:
    1. Remove `id="password-submit"` from `templates/password.html`. The failure names `password.html`.
    2. Remove `id="{{ save_id }}"` from the macro `actions` of `components/grid.html`. The failure names `parameter-form.html`.
    3. Remove `id="parameter-show-fields"` from the macro `actions`. The failure names `parameter-form.html`.
    4. Remove `id="{{ row.id }}"` from the macro `mode_row`. The failure names `parameter-form.html`.

## 4. Documents

ADR-0152, the Status line of ADR-0050, and the index and the decision map of `docs/adr/README.md` come from the propose step. `ste-lint.py` reports 0 hard violations on them.

- [x] 4.1 (docs/architecture) [mechanical] Name the diff sections in the rule of the shown name (ADR-0152).
  - File: `docs/architecture/api.md` section 8, the item that starts with "Parameter names and descriptions show the translation".
  - Add after its third sentence: "The rule also covers the added and the old-only parameters of the diff and the preview (ADR-0152)."
  - Check: `python -I tools/ste-lint/ste-lint.py docs/architecture/api.md` reports 0 hard violations.
- [x] 4.2 (CHANGELOG.md) [mechanical] Add one entry under `[Unreleased]` for the change `shown-name-and-focus-round-2`.
  - Text: the diff sections show the shown name (ADR-0152), the parameter form tags its English inputs, and two forms keep the focus.
  - Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` reports 0 hard violations.
- [x] 4.3 (docs) [sonnet] Run the STE checks of every document of the change.
  - Command: `python -I tools/ste-lint/ste-lint.py docs/adr/0152-*.md docs/adr/README.md docs/adr/0050-*.md docs/architecture/api.md CHANGELOG.md openspec/changes/shown-name-and-focus-round-2/**/*.md` from Bash with `globstar` and `nullglob`.
  - Check each sentence for at most 25 words and each procedure step for at most 20 words. Report each fault and fix only the wording.

## 5. Check and gate

- [x] 5.1 (workspace) [opus] Check that the shown name reads no data outside the snapshot (ADR-0152 (c), design D2).
  - Command: `git diff main -- crates/recipe-db crates/recipe-app/src`. Expected: no change outside test files.
  - Check that `diff.rs` and `approval_preview.rs` still read the pairs and the draft inside `Isolation::RepeatableRead`.
- [x] 5.2 (workspace) [sonnet] Run each command of the gate and put the output into the report.
  - `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
  - `cargo test --workspace --all-features --locked`
  - `cargo fmt --all --check`
  - `cargo deny check`
  - `openspec validate shown-name-and-focus-round-2 --strict` and `openspec validate --specs --strict`
  - The `ste-lint.py` command of `CLAUDE.md` "Commands"
