# Design

## Context

See proposal.md, section Why. The review notes in `target/ui-review/` are not normative. Each finding was checked in the source before this plan.

- A3 is true. The delete form in `translation-row.html` targets `#translations` with `innerHTML`. `remove` in `translation_routes.rs` renders every row again from the store.
- A4 is true. `components/field.html` renders only `input`. The four multi-line fields use it. `LongText::parse` in `recipe-domain/src/text.rs` permits LF and CRLF. `update_input` in `pages/parameters/edit.rs` sends `description_en` with each save of a value.
  - Change: the UI could never store a line end, so no stored text loses one today. The loss starts as soon as any path writes a line end.
- A6 is true. `base.html` shows the banner from `layout.read_only`, but no page view reads the mode. `PageRequest::read_only()` and `Operation::runs_in_read_only` exist.
  - Change: after `503 E-WEB-006`, HTMX puts the error into `#error-panel`, and the form keeps the input (ADR-0036). The user loses the effort and the input at the next page load, not at the answer.
  - `UnitTree::build` in `machines/tree.rs` takes the role and decides the add links itself. Its callers are `machines/page.rs`, `machines/tree_page.rs`, and `units/add.rs`.
  - `form_page` and `form_fragment` in `parameters/reply.rs` always render the form of a new parameter.
- B4 is true. `saved` in `pages/parameters/edit.rs` answers the summary and the form. The English base text in `translations.html` keeps the old texts until the next page load.

Documents that apply:

- Translation rows and focus: ADR-0043 (f), ADR-0110, `design-system.md` §10.1 (answers to HTMX).
- Row version protocol: `api.md` §4, ADR-0008, ADR-0015.
- Form field rules: `design-system.md` §10.5, ADR-0041. Long text rules: `reliability.md` §4.3, §7.11, ADR-0028.
- Read-only mode: A24, `reliability.md` §7.8, ADR-0071, ADR-0073, `api.md` §2 and §5.1.
- Permission check of a form route: ADR-0046. Browser tests: ADR-0048.

## Goals / Non-Goals

**Goals:**

- One ADR for each finding: ADR-0111 (A3 and B4), ADR-0112 (A4), ADR-0113 (A6). Each ADR changes `api.md` or `design-system.md` in this change.
- Keep the three scripts, the CSP, the literal `hx-*` values, and the POST-only mutations.

**Non-Goals:**

- No new route, no new error code, no migration, no change of `recipe-domain` or `recipe-app`.
- No change of A24 or of the gate in `gates.rs`.

## Decisions

### D1. Three ADRs, no merge

The decision map of `docs/adr/README.md` names no ADR that covers these points.

- ADR-0043 (f) fixes the rows, but not the answer of a deletion or the focus after it. ADR-0111 adds that answer. It also adds the English base text to the answer of a definition save, because that text stands in the same section.
- ADR-0028 fixes the domain line ends, but not the field or the web conversion. ADR-0112 adds both.
- ADR-0071 fixes the banner, but not the forms. ADR-0113 adds the page rule.

The three decisions touch different files and can change apart, so each one has its own ADR.

### D2. Answer of a translation deletion (ADR-0111)

`remove` in `translation_routes.rs` builds one `TranslationRow` with no stored translation, the same way as `refuse_row` and `saved`. It renders `TranslationRowFragment` with the toast. The flag `listed` comes from `i18n.translation_langs`.

The row macro gets a flag for the focus. Only the answer of a deletion sets it, so the page and the save answer keep their markup. The `GET .../translations` route keeps `TranslationsFragment`.

After this change only the page and `GET .../translations` render `translations.html`. Both have no toast. The field `toast` of `TranslationsBlock` and the toast branch of `translations.html` therefore go away.

ADR-0008 compares the `row_version` of every request with the stored value. The other rows give no `409` after the deletion, because `HX-Trigger` updates their hidden `row_version` fields.

Alternative: send the other rows out of band with their stored texts. Rejected, because that also replaces the unsaved input.

### D3. Line ends in the web layer (ADR-0112)

One free function converts CRLF to LF. It lives in a new module `crates/recipe-web/src/form_text.rs`. The four parse sites call it before `LongText::parse`:

- `pages/parameters/definition.rs` (`description_en`),
- `pages/parameters/translation_routes.rs` (`check_row`, `description`),
- `pages/recipes/form.rs` and `pages/versions/form.rs` (`change_note`).

A refused form shows the sent text again. That text can hold CRLF, and the `textarea` shows it unchanged.

Alternative: convert in `LongText::parse`. Rejected, because ADR-0028 and the doc comment of `parse` state that the parse does not change the input.

### D4. Multi-line text outside a form (ADR-0112 (h))

The class `text-lines` goes on the element that holds the text, not on a parent. A parent with `pre-wrap` would also show the whitespace of the template. The places are `parameter-summary.html`, `translations.html`, `translation-row.html`, `version.html`, `versions-table.html`, and `components/diff.html`.

The class changes the exact markup of a cell. The task that adds the class also changes every test that asserts that markup, for example `diff_lang_tests.rs` and `approval/preview_rows_tests.rs`.

`maxlength` counts UTF-16 code units, and `LongText::parse` counts Unicode characters. A character outside the Basic Multilingual Plane counts two in the browser and one on the server. Both count a line end as one character after the conversion of D3. The server therefore accepts every text that the browser accepts.

### D5. One predicate for the controls (ADR-0113 (a), (b))

`PageRequest::may_run(operation)` replaces each bare `operation.requirement().allows(role)` under `crates/recipe-web/src/pages/`. For a query it returns the same value as today, because the mode permits every query. The navigation in `layout.rs` keeps its own check, because it filters query targets only.

`UnitTree::build` takes `can_add: bool` in place of the role. Each page caller passes `may_run(Operation::AddUnit)`. One task of group 2 changes the constructor and all its callers, so no parallel task of group 3 touches them.

A source guard checks that `pages/` holds no `requirement().allows` outside `pages/request.rs`.

- It ignores all whitespace in the search. rustfmt splits the chain over two lines in `parameters/page.rs` and `audit/list.rs`, and the guard finds both.
- A self-test of the search uses a chain on one line and a chain on two lines.
- It skips the test files: names that end in `_tests.rs`, the name `tests.rs`, and names that start with `test_`.
- It comes in group 1 and fails on the code of today. It passes after group 4. The links of the audit list and the history page lead to queries, so their tests are regression tests.

### D6. Form routes in read-only mode (ADR-0113 (d))

Each form route of a blocked command keeps its query and its permission check. Its view gets a flag from `may_run` of its command. With the flag false and the mode on, the template shows the notice in place of the form. The heading stays, so the content answer keeps its `h1` with `autofocus` (`design-system.md` §10.1).

A role without the permission still gets `403 E-WEB-002` from the query (ADR-0046). The notice therefore shows only to a role that could run the command in normal mode.

The form of a new parameter answers a full page, the content of `HX-Target: content`, and the fragment of `HX-Target: parameter-form`. All three answers show the notice. `ParameterPage` gets a notice, so the task that adds it also changes `parameters/page.rs`. That task therefore runs in its own group after group 4.

### D7. Read-only switch of the dev server (ADR-0113 (h))

`examples/dev_server.rs` reads `RECIPE_DEV_READ_ONLY_FILE`. With the variable set, a task checks the file once per `READ_ONLY_POLL` (250 ms) and calls `ReadOnlySwitch::set` with the result. The check uses `tokio::fs::try_exists`, so it does not block the runtime. Without the variable, the task does not start.

`e2e/tests/support/read-only.ts` holds the one absolute path of the switch file, `e2e/test-results/read-only-mode`. The config and the spec import it.

- `e2e/playwright.config.ts` starts a second instance on the port `RECIPE_DEV_PORT + 1` with that path.
- A second project `read-only` has the base URL of that port and runs only `read-only.spec.ts`. The project `chromium` ignores that spec.
- The fixture `test` of `support/seed.ts` seeds the instance at the base URL of its project. The read-only test therefore needs no new seed helper.
- The seed holds no `Approved` version. The page test of the version page checks the export link in read-only mode.

### D8. Waves of the tasks

The tasks of one group touch disjoint files, so one Sonnet or Haiku agent can run each task in parallel. Each task names every file that it changes, also the test files whose assertions it changes. The groups run in order. The browser checks and the source guard come first and fail until the code tasks pass.

### D9. English base text after a save of the definition (ADR-0111 (i))

The base block of `translations.html` becomes a macro with the literal `id="translations-base"`. The page renders it in place. `saved` in `pages/parameters/edit.rs` appends it with the literal `hx-swap-oob="true"` to the summary and the form. The answer holds no translation row, so the unsaved input of the rows stays.

Alternative: send every translation row out of band with the new base text. Rejected, because that replaces the unsaved input.

## Risks / Trade-offs

- Risk: HTMX 2 does not honor `autofocus` in an `outerHTML` swap. Mitigation: task 1.1 checks the focus in a browser. A failure stops the wave, and the fix needs a new decision.
- Risk: a page keeps a bare permission check and shows a form in read-only mode. Mitigation: the source test of D5 and the read-only browser test find it.
- Risk: the second dev server slows the e2e run. Mitigation: it starts in parallel with the first instance. It holds only the seed of one test.
- Risk: an XSS test reads only the `value` attribute of an input. Mitigation: task 2.7 changes these assertions in `recipe_xss_tests.rs` to read the text of the `textarea`. The preview, diff, and audit checks keep their escaping assertions.
- Risk: the switch file of a failed run stays and turns on the mode at the next run. Mitigation: the file lies in `e2e/test-results/`, and the spec deletes it in `afterAll`.

## Migration Plan

No data migration. A stored text keeps its line ends. A stored CRLF becomes LF at the next save of its form (ADR-0112 (g)). A rollback is the previous image. The stored data stay valid for it, because the domain accepts LF.

## Decisions of the user

- B4 (UI-03) joins this change as one task under ADR-0111 (i), design D9.
- In read-only mode a page hides the controls of a blocked command (ADR-0113). It does not disable them.

## Open Questions

None.
