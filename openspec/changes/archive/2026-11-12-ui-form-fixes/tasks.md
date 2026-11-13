# Tasks

## 1. Add link after the last page (F2, design D1)

- [x] 1.1 (recipe-web) Write a failing page test. `GET /recipes/{ref}/versions/{n}?page=2` of a draft with 3 parameters renders the link Add a parameter with `hx-target="#content"` and `hx-push-url="true"`. The page holds no `#parameter-rows`. The first page keeps `hx-target="#parameter-form"`. Then pass `after_end` to `VersionPage` and add the second literal link to `version.html`.
- [x] 1.2 (recipe-web) Write a guard test. The first page of a draft without parameters holds `#parameter-rows` and the link with `hx-target="#parameter-form"`. The test passes before and after 1.1. Say so in the task report.
- [x] 1.3 (recipe-web) Write a guard test. An HTMX `GET .../parameters/new` with `HX-Target: content` answers the page form with `hx-target="#content"` and without `#parameter-rows`. A save of that form answers `303` to the version page. The test passes at once. Say so in the task report.
- [x] 1.4 (e2e) Write the browser test of the scenario "Save from a stale page in the browser" in `e2e/tests/version-pages.spec.ts`. Collect the console messages and the requests. Assert exactly one `POST` with `fields=1`, exactly one `POST` that saves, no `htmx:targetError`, and the new tag on the version page. Check: `E2E_PASSWORD=... npm test -- version-pages`.

## 2. Order of the UDT buttons and the unchanged revision (F4, design D2 and D4)

- [x] 2.1 (recipe-domain, recipe-web) Write failing tests. `PlantError::RevisionUnchanged { type_name, revision }` gives `E-PLT-052` and a message with the name and the revision. `ErrorCode::PLT_052` stands in the list of all codes. `error/status.rs` maps `E-PLT-052` to `409`. `error-E-PLT-052` exists in `en.ftl` and `pl.ftl`, and the English text equals the registry row. Then add, in one step, the row of ADR-0159 (e) to `reliability.md` section 1.2, the variant, the constant, the status, and the two keys. The registry tests read all four, so they land together. Check: `cargo test --workspace` passes.
- [x] 2.2 (docs) [mechanical] Add `E-PLT-052 RevisionUnchanged` to the errors of `DefineUdt` in `domain-model.md` section 5.1 and in the row `DefineUdt` of `data-model.md` section 6. Add `409 E-PLT-052` to the two `DefineUdt` routes of `api.md` section 2.3. Check: `ste-lint.py` on the three files.
- [x] 2.3 (recipe-app) Write failing tests in `plant/define_udt/tests.rs`. A revision with the members of the latest revision gives `E-PLT-052` and writes no row and no audit entry. A revision with one changed type, one changed name case, one more member, or another order gives the next revision. Then add the comparison of design D4.
- [x] 2.4 (recipe-web) Write a failing page test in `src/pages/types/define_error_tests.rs`. An HTMX `POST .../types/ST_Zone/revisions` with the members of `ST_Zone@2` gives `409 E-PLT-052` with `HX-Retarget: #error-panel`, the sent rows, and no field line. Then complete the error mapping of `pages/types/define.rs` when the test fails.
- [x] 2.5 (recipe-web) Write a failing render test in `src/pages/types/define_tests.rs`. The first `type="submit"` button of `#udt-form` is `udt-define-submit`. That holds in the form of a new type with 1 row and in the revision form of `?from=`. It also holds after `add=1` at 1022 rows (1023 rows) and at 1024 rows. `udt-add-member` follows it at 1 and 1023 rows. Then swap the two buttons in `udt-form.html`.
- [x] 2.6 (e2e) Write the browser tests of the scenario "Enter in the form of a new type in the browser". Put them in a new `e2e/tests/udt-form.spec.ts`. Cover 1, 1023, and 1024 rows, and Enter in `#type_name` and in the type input of a row. Build the large forms with requests, not with clicks. Assert exactly one `POST` and no field `add`. Assert the type page of `ST_<unique>@1` with that count of members. Check: `npm test -- udt-form`.
- [x] 2.7 (e2e) Write the browser tests of the scenario "Enter in a revision form in the browser" in `e2e/tests/udt-form.spec.ts`. Cover 1, 1023, and 1024 rows. Change the type of one row first. Assert exactly one `POST`, no field `add`, and the next revision. Add one test that presses Enter in an unedited revision form and finds `E-PLT-052` in `#error-panel`. Check: `npm test -- udt-form`.
- [x] 2.8 (recipe-app, recipe-web, recipe-db, e2e) Run `cargo test --workspace` and the browser suite. Change each existing test that writes a revision with the same members, so that it changes one member. Check: every test passes.

## 3. Fixed parent of the unit form (F5, design D3)

- [x] 3.1 (recipe-web) Write failing tests in `src/pages/units/form_tests.rs`. `GET .../units/new?parent_path=U1` with `U1.1` gives a hidden `parent_path` `U1`, no `select`, an `h2` with `U1`, a text line with `U1`, and the segment `2`. `GET .../units/new` gives an empty hidden `parent_path`, the text of `unit-parent-none`, and the segment `2`. Then change `unit-form.html` and `FormView`, and remove `ParentOption`. Update each old test that reads the parent `select`.
- [x] 3.2 (recipe-web) Write a guard test in `src/pages/units/add_tests.rs`. An HTMX `POST` with the parent `U1` and the used segment `1` gives `409 E-PLT-004`, and the out-of-band form keeps the hidden `parent_path` `U1`. The test can pass after 3.1. Say so in the task report.
- [x] 3.3 (recipe-web) Write failing tests in `src/pages/units/add_tests.rs`. An HTMX `POST` with the archived parent `U2`, the parent `U9` that does not exist, or the parent `U1..2` gives the summary line of its code. That line has no `href="#parent_path"`. Then pass the target `None` at `units/add.rs:68` and in `add_input` for `ParentNotFound` and `UnitArchived`.
- [x] 3.4 (recipe-web) Write a failing test in `src/pages/units/add_tests.rs`. A `POST` without `HX-Request` with the parent `U1`, the segment `2`, and an empty name gives a page with the text line of `U1` and the hidden `parent_path` `U1`. Then complete the form of `Place::Refused`.
- [x] 3.5 (e2e) Write the browser test of the scenario "Proposed segment saves in the browser" in a new `e2e/tests/unit-form.spec.ts`. Follow "Add a sub-unit" of `U1` and keep the proposed segment. Assert the new node in `#unit-tree` and an empty `#error-panel`. Check: `npm test -- unit-form`.

## 4. Checks of the change

- [x] 4.1 (openspec/changes/ui-form-fixes) Run `openspec validate ui-form-fixes --strict` and `ste-lint.py` on the files of the change and on ADR-0159. Check: both report 0 errors and 0 hard violations.
- [x] 4.2 (docs/adr) Set ADR-0159 to `Accepted` after the owner approves this change. Add the index row and the entry in the decision map row "PLC types, UDT revisions, derived types, S7-300 layout" of `docs/adr/README.md`. Check: `ste-lint.py` on `docs/adr/README.md`.
- [x] 4.3 (e2e) Run the full browser suite with `E2E_PASSWORD=... npm test`. The tests `page-sweep`, `accessibility`, and `focus` cover the three changed forms. Check: every test passes.

## Workflow follow-up

- Review the code with `code-review` (Opus). Fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion` with clippy, test, and fmt of CI.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate --strict` pass.
