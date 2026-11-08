# Design

## Context

See `proposal.md`, section "Why". ADR-0152 holds the new rule. ADR-0126 (a) to (f), ADR-0050 (d) and (f), ADR-0049 (e), ADR-0041 (e), and `api.md` section 8 hold the context. This design links to them and does not repeat them.

The state of the code on `main` at `0519b9e`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Domain part of an added or a removed parameter | `DiffPart::Parameter` holds `name_en`, `unit`, `plc_type`, `scalar`, and `leaves`. It holds no translation | `crates/recipe-domain/src/recipe/diff/rows.rs` line 188, `diff/values.rs` line 81 |
| Translations of a diff side | `Parameter::translations` gives the stored translations of each language | `crates/recipe-domain/src/recipe/draft/parameter.rs` line 230 |
| Pair read of the diff page | `pairs` reads the parameters of the page keys with their limits and translations (`SELECT_TRANSLATIONS`) | `crates/recipe-db/src/transaction/diff_reads.rs` lines 127, 215, 238 |
| Transaction of the diff page | `DiffVersions` opens `Isolation::RepeatableRead` and calls `read_page` with `read_new` set to `true` | `crates/recipe-app/src/recipe/queries/diff.rs` lines 90, 146 |
| Transaction of the preview | `GetApprovalPreview` opens `Isolation::RepeatableRead`. It loads the draft with its translations and calls `read_page` with `read_new` set to `false` | `crates/recipe-app/src/recipe/queries/approval_preview.rs` lines 120 to 142, 201 |
| Name cell of the two sections | `in_language(Lang::En.as_str(), text_cell("", name_en))`. It puts `lang="en"` on the cell in each UI language and shows no marker | `crates/recipe-web/src/pages/diff_view.rs` line 631 |
| Cell of a section | `Cell` has `lang` and no marker. `table_cell` puts `lang` on the `td` | `diff_view.rs` line 178, `templates/components/diff.html` line 12 |
| View builder | `DiffView::new(t, page, links)`. The diff page and the preview call it | `diff_view.rs` line 332, `pages/diff.rs` line 127, `pages/approval/preview.rs` line 326 |
| Shown name | `shown_name(t, translated, name_en)` gives the text and the marker. The parameter table and the new row call it | `crates/recipe-web/src/pages/versions/parameter_table.rs` lines 62, 316, `pages/parameters/new.rs` line 101 |
| Translation language | `Lang::translation_lang_or_internal(request_id)` gives `Some(pl)` or `None` | `crates/recipe-web/src/i18n.rs` line 102, `pages/versions/page.rs` line 241 |
| Name cell of the parameter table | `<span lang="en">{{ text }}</span> {{ marker }}` when a marker applies | `templates/components/parameter.html` line 4 |
| Parameter form | `fields::named` of `name_en` and `fields::lines` of `description_en` pass no `lang` | `templates/parameter-form.html` lines 13, 14 |
| Push of the version address | `with_push_url(response, href, request_id)` sets `HX-Push-Url` | `crates/recipe-web/src/pages/draft_reply.rs` line 160 |
| Switch target of a pushed answer | `version_layout` calls `Layout::with_pushed_path(base)`. A content answer to a `POST` then sets `#lang-next` | `pages/versions/page.rs` lines 121 to 138, `crates/recipe-web/src/layout.rs` line 284 |
| Tests of the switch target | The approval, the retire, and the removal assert `#lang-next` and `HX-Push-Url` | `crates/recipe-web/src/pages/versions/lang_next_tests.rs` lines 48, 59, 86 |
| Own password form | The submit button has no `id`. `refuse_password` swaps `#password-form` out of band | `templates/password.html` line 12, `crates/recipe-web/src/pages/own.rs` line 212 |
| New user form | The submit button has no `id`. `NewUserForm::sent` sets `oob` | `templates/user-new.html` line 10, `crates/recipe-web/src/pages/users/templates.rs` line 172 |

### Findings that differ from the plan

1. D5-2 does not reproduce at the review HEAD `3258863` or at `0519b9e`. `version_layout` sets `#lang-next` through `with_pushed_path` at `pages/versions/page.rs` line 136. The review missed that line.
   - Evidence: `lang_next_tests.rs` lines 48, 59, and 86 pin the switch target of the approval, the retire, and the removal. Line 70 pins the approval without HTMX.
   - The owner decided on 2026-11-08: record D5-2 as "not reproduced", drop the refactor of D5, and keep the check of task 2.3.
   - The requirement "Language switch after a pushed address" records the current rule. These tests already satisfy its scenarios.
2. Plan section 5.1 task 2 says that the diff query must read the translation. The pair read and the draft load already read every translation in the snapshot. The gap is in the domain part, so the fix touches `recipe-domain`.
3. The review says that every other out-of-band form has a fixed button `id`. The new user form has none. That form has no `hx-boost`, so the focus loss does not occur. The template test of D7 still fails on it.
4. The plan gives a failing test and its fix as two tasks. The apply guidance of `openspec/config.yaml` requires a green `cargo test` after each task. Each task of `tasks.md` therefore holds its red step and its green step.
5. `templates/user-actions.html` puts `hx-swap-oob` on the wrapper `#user-actions`, not on a form. Its unlock button has no `id`. The proposal puts the user page actions out of scope. The requirement and the scan of D7 therefore cover only a form whose start tag holds `hx-swap-oob`.
6. `parameter-form.html` renders its submit buttons through macros of `components/grid.html`. `actions` renders `id="{{ save_id }}"` and `id="parameter-show-fields"`. `mode_row`, through `grid`, renders `id="{{ row.id }}"`. Each of these ids has a constant source today, but no test fails when one goes away.
7. Two current tests match markup that this change replaces. `approval_text_xss_tests.rs` line 203 counts two `<td lang="en">` cells of the payload on English pages. `review_fix_tests.rs` lines 124 to 137 match a submit button without an `id` on `/me/password` and `/users/new`.

## Goals / Non-Goals

**Goals:**

- The added and the removed sections show the shown name of ADR-0152 on both pages.
- The English base text inputs of the parameter form carry `lang="en"`.
- Each submit button of each form whose start tag holds `hx-swap-oob` has a fixed `id`, and a test keeps it so.

**Non-Goals:**

- A change of the SQL change flags, the pair read, or the draft load.
- A change of the definitions section or the translations section.
- A change of the toast of the parameter removal.
- A code change of the push of the version address (D5-2, finding 1).

## Decisions

### D1: The domain part carries the translated names (ADR-0152 (b), (d))

`DiffPart::Parameter` gets the field `translated_names: BTreeMap<LangCode, Text>`. `parameter_row` fills it from `parameter.translations()` with the `name` of each `Translation`. The added part takes the new side, and the removed part takes the old side, because `pair_parts` already passes that side.

- Alternative: the use case passes the translation language into `diff_pairs`. Rejected, because the pure diff then knows the UI language.
- Alternative: `recipe-app` adds a map from key to names to `DiffPage`. Rejected, because it builds a second view of the pairs next to the domain rows.

### D2: The translation comes from the snapshot (ADR-0152 (c))

The diff page reads both sides with `pairs` (`diff_reads.rs` line 238). The preview reads the old side with `pairs` and loads the draft with `read_draft_of`. All three reads run inside the `RepeatableRead` transaction of `diff.rs` line 146 or `approval_preview.rs` line 201. The transaction ends before the domain diff runs (ADR-0050 (f)). D1 only keeps data that these reads already return. The change adds no statement, no query, and no port method.

### D3: The name cell follows the one function of the shown name (ADR-0152 (e) to (g))

- `DiffView::new` gets the argument `translation: Option<&LangCode>`. `pages/diff.rs` and `pages/approval/preview.rs` pass `request.lang().translation_lang_or_internal(request_id)?`.
- The name cell calls `shown_name(t, translated_names.get(code), name_en)`. A `None` translation language gives `None` as the translation.
- `Cell` gets the field `marker: Option<String>`. Every other builder sets `None`.
- `table_cell` renders `<span lang="en">` with the text and then the marker outside the span, as `components/parameter.html` line 4 does. A cell without a marker keeps its current shape.
- The name cell gets no `lang` on the `td`. A translation of `pl` shows on a `pl` page. `name_en` on an `en` page needs no tag (ADR-0126 (e)).
- Only the pattern of `diff_view.rs` line 624 names each field and gets the new field. The patterns of `diff_view/faults.rs` and `diff/type_tests.rs` use `..` and stay.
- On an English page, the name cell of the two sections renders `<td>` with the text. A definition row keeps `<td lang="en">`.

### D4: The English inputs of the parameter form (ADR-0126 (f))

`parameter-form.html` line 13 passes `"en"` as the argument `lang` of `fields::named`. Line 14 passes `"en"` as the argument `lang` of `fields::lines`. `translations.html` line 5 and `parameter-summary.html` line 12 already tag the same texts with `lang="en"`. The labels stay in the UI language.

### D5: Withdrawn

The owner dropped this decision on 2026-11-08, because D5-2 does not reproduce (finding 1). The change adds no function and moves no header. Task 2.3 checks the current places that set `HX-Push-Url` or the switch target. The command is `git grep -n -E "HX_PUSH_URL|hx-push-url|with_push_url|with_pushed_path"` on `crates/recipe-web/src` without the test files.

| Place | Handler | Value |
|---|---|---|
| `pages/approval/approve.rs` line 204 | `approve` | the version path, through `with_push_url` |
| `pages/versions/actions.rs` line 338 | `retire` | the version path, through `with_push_url` |
| `pages/parameters/remove.rs` line 117 | `version_content` of `remove` | the version path, through `with_push_url` |
| `pages/versions/page.rs` line 136 | `version_layout` | the switch target `base`, through `with_pushed_path` |
| `error/answer.rs` line 502 | the HTMX error answer | `false` (ADR-0037 (z)) |
| `pages/parameters/reply.rs` line 405 | the refused parameter form for the content | `false` |
| `pages/types/define.rs` line 294 | the refused type form for the content | `false` |

The definitions of `with_push_url` (`draft_reply.rs` line 160) and `with_pushed_path` (`layout.rs` line 284) set no value of their own. A value `false` pushes no address, so the switch keeps its target. The attribute `hx-push-url="true"` of a boosted form pushes the address of the final answer. That answer is a `GET` after a `303`, so its layout holds its own path.

### D6: The fixed ids of the two buttons (ADR-0041 (e))

`password.html` line 12 gets `id="password-submit"`. `user-new.html` line 10 gets `id="user-new-submit"`. The ids follow the pattern `<form>-submit` of the other forms.

- The `id` stands after `type="submit"` and before `class`, as in `machine-new.html` line 15.
- The button then reads `<button type="submit" id="password-submit" class="btn btn-primary">`.
- `review_fix_single_action_forms_have_a_primary_button` of `review_fix_tests.rs` matches the old literal. Tasks 3.1 and 3.2 change the literal of their path to hold the new `id`. The literal of `/login` stays.

### D7: One template test lists the out-of-band forms (ADR-0041 (e))

The test reads each file under `crates/recipe-web/templates/` except `templates/test/`. For each `<form` start tag that holds `hx-swap-oob`, it reads the body up to `</form>`.

1. The test finds each submit button in the body. A submit button is a `<button` without `type="button"` or `type="reset"`, or an `<input type="submit">`.
2. The test expands each macro call `grids::<name>(` in the body to the body of that macro in `components/grid.html`.
3. The test also expands each call of another macro of that file inside an expanded body, for example `mode_row(` inside `grid`.
4. Each submit button that the test finds must hold `id="`.
5. A form without a submit button fails the test. So a new form or a new macro needs a submit button with an `id`.

The expansion covers the three submit buttons of `parameter-form.html`: `id="{{ save_id }}"` and `id="parameter-show-fields"` of `actions`, and `id="{{ row.id }}"` of `mode_row`. `toggle_row` renders `type="button"`, so it holds no submit button.

Each out-of-band form today, from that rule:

| Template | Form `id` | Submit button `id` |
|---|---|---|
| `approve.html` line 17 | `approve-fields` | `approve-submit` |
| `machine-new.html` line 9 | `machine-new-form` | `machine-create-submit` |
| `machine.html` line 29 | `machine-rename-form` | `machine-rename-submit` |
| `parameter-form.html` line 5 | `parameter-edit-form` or `parameter-new-form` | `{{ save_id }}` and `parameter-show-fields` through `grids::actions`, `{{ row.id }}` through `grids::grid` |
| `password.html` line 8 | `password-form` | none (D6 adds `password-submit`) |
| `rebind-form.html` line 8 | `{{ form.id }}` | `{{ form.id }}-submit` |
| `recipe-new.html` line 9 | `recipe-new-form` | `recipe-create-submit` |
| `recipe.html` line 17 | `recipe-rename-form` | `recipe-rename-submit` |
| `translation-row.html` line 6 | `translation-{{ row.lang }}-form` | `translation-{{ row.lang }}-save` |
| `udt-form.html` line 9 | `udt-form` | `udt-add-member`, `udt-define-submit` |
| `unit-form.html` line 7 | `unit-add-form` | `unit-create-submit` |
| `unit.html` line 27 | `unit-rename-form` | `unit-rename-submit` |
| `user-new.html` line 6 | `user-new-form` | none (D6 adds `user-new-submit`) |
| `user-role-form.html` line 2 | `user-role-form` | `user-role-submit` |
| `version-new.html` line 8 | `version-new-form` | `version-create-submit` |

## Risks / Trade-offs

- Risk: the new field of `DiffPart::Parameter` breaks the struct literal `speed_row` of `diff/tests.rs`. Mitigation: task 1.1 updates it in the same run.
- Risk: D3 changes the name cell, and a current XSS test counts the old cell (finding 7). Mitigation: task 1.2 changes the expected cells and keeps the escape check of each payload.
- Risk: the source scan of D7 misses a form whose `hx-swap-oob` stands on a later line. Mitigation: the scan reads the whole start tag up to its `>`, also across lines and template tags.
- Risk: the scan misses a submit button of a macro. Mitigation: the scan expands the macros of `components/grid.html`, and task 3.3 has one mutation for each id of these macros.
- Risk: the red step of one task breaks the build of a parallel task in the same working tree. Mitigation: the code tasks of `recipe-web` run one after the other (`tasks.md`, "Order").
