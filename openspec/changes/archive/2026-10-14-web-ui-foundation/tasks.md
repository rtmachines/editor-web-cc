# Tasks

One agent works at a time, because every batch touches `recipe-web` or a crate that it depends on. Each check command runs one test filter. The batches run in this order:

1. A: tasks 2.1 to 2.4.
2. B: tasks 3.1 and 3.3.
3. C: task 3.2 (Haiku).
4. D: tasks 4.1 and 4.2.
5. E: tasks 5.1 to 5.4.
6. F: tasks 5.5, 6.1, and 6.2.
7. G: tasks 7.1 and 7.2.
8. H: task 8.1 (Haiku).
9. I: tasks 8.2 and 8.3.
10. J: tasks 8.4 and 8.5.
11. K: tasks 8.6 and 8.7.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0036 with the decisions (a) to (p), its index row, and its rows in the decision map. Check with `ste-lint.py` on the scope of ADR-0023.
- [x] 1.2 (docs/architecture) Write `design-system.md`. Add the row A44 and the new row R39 to `overview.md`. Change `api.md` sections 2.1, 4, 5.3, and 9. Check with `ste-lint.py`.
- [x] 1.3 (docs/plan) Add `docs/plan/ux-brief.md` and the entry of `CHANGELOG.md`. Check with `ste-lint.py` on both files.
- [x] 1.4 (.claude/rules) Change the rule of `HX-Reswap` in `web.md` and add the rule of the design system. Check with `ste-lint.py .claude/rules/web.md`.

## 2. Static files and style sheet

- [x] 2.1 (recipe-web) Vendor the 8 font files and the two license files of design D3, with one route row per font. First write failing tests for the SHA-256 constants, each font route, and an unknown font. Check with `cargo test -p recipe-web static_files`.
- [x] 2.2 (recipe-web) Add `static/app.css` with every token of `design-system.md` and its route. First write the failing token test, contrast test, and route test of design D4. Check with `cargo test -p recipe-web style_tests`.
- [x] 2.3 (recipe-web) Write the rules of `app.css` for the page, the top bar, the navigation, and the language switch. Also write the rules of the focus, the targets, the links, and the fonts. First write the failing rule tests of design D4. Check with `cargo test -p recipe-web style_tests`.
- [x] 2.4 (recipe-web) Write the rules of `app.css` for the table, the form, the badge, and the alert. Also write the rules of the toast, the tree, the empty state, and `.htmx-indicator`. First write the failing rule tests of design D4. Check with `cargo test -p recipe-web style_tests`.

## 3. Scripts, texts, and template rules

- [x] 3.1 (recipe-web) Add `static/row-version.js`, `static/tree.js`, and their routes of design D7. First write failing tests for the line limits, the forbidden words, and the routes. Check with `cargo test -p recipe-web static_files`.
- [x] 3.2 (recipe-web) [mechanical] Copy the text table of design D8 into `en.ftl` and `pl.ftl`. First write a failing test that each key of the table has the text of the table in both bundles. Check with `cargo test -p recipe-web i18n`.
- [x] 3.3 (recipe-web) Add the template tests and the architecture tests of design D8 and D10. They cover literal words, the four text attributes, and every unescaped output. Check with `cargo test -p recipe-web template_tests`.

## 4. Components

- [x] 4.1 (recipe-web) Add the macros `icons`, `badge`, `alert`, `empty`, and `toast` of design D6. Make the key scan of `pages/texts/tests.rs` recursive. First write failing tests with test templates in `pl` and `en`. Check with `cargo test -p recipe-web components_tests`.
- [x] 4.2 (recipe-web) Add the macros `field`, `table`, and `tree` of design D6. First write failing tests with test templates for the ARIA attributes of `design-system.md` sections 10.4, 10.5, and 10.8. Check with `cargo test -p recipe-web components_tests`.

## 5. Layout and pages

- [x] 5.1 (recipe-web) Add `Layout`, `NavTarget`, `NavLink`, and `PageRequest::path_and_query` of design D5. First write failing unit tests of the links for each role, for `must_change_password`, and for a request that is not `GET`. Check with `cargo test -p recipe-web layout`.
- [x] 5.2 (recipe-web) Rewrite `base.html` with the layout regions and move every page struct and the error page to `layout`. First write failing page tests for the spec "Page layout" in `pl` and `en`. Check with `cargo test -p recipe-web page_tests`.
- [x] 5.3 (recipe-web) Add the navigation and the language switch to the top bar. First write failing page tests for the specs "Navigation by permission", "Language switch", and "Language labels". Check with `cargo test -p recipe-web page_tests`.
- [x] 5.4 (recipe-web) Build the user pages with the components, the user badge, and the toast of design D6. Change `FORBIDDEN_MARKUP` of design D10. First write failing page tests in `pl` and `en`. Check with `cargo test -p recipe-web pages::users`.
- [x] 5.5 (recipe-web) Build `error.html` and `field-errors.html` with the macro `alert`, the field labels, and `HX-Reswap` of design D5. First write failing tests for the specs "Field error lines" and "HTMX error headers". Check with `cargo test -p recipe-web error`.

## 6. Recipe list skeleton

- [x] 6.1 (recipe-app) Add the query `ListRecipes` and the type `RecipeList` of design D9 and D12. First write failing tests on `MemoryStore` that each role gets a list without rows. Check with `cargo test -p recipe-app recipe::queries`.
- [x] 6.2 (recipe-web) Add the route `GET /recipes`, its handler, `recipes.html`, and its rows in the route tests of design D9. First write failing tests for each scenario of the spec "Recipe list page". Check with `cargo test -p recipe-web pages::recipes`.

## 7. Security and page tests

- [x] 7.1 (recipe-web) Extend the XSS tests of design D10. They cover the field `next`, the fixed icon tags, the new corpus line, and the handler rules. Check with `cargo test -p recipe-web xss`.
- [x] 7.2 (recipe-web) Send each `GET` page of the route table in `pl` and `en`. Each answer has the CSP, `no-store`, the `htmx-config`, the layout, and no inline script, style, or handler. Check with `cargo test -p recipe-web page_tests`.

## 8. Code review fixes

- [x] 8.1 (recipe-web) [mechanical] Copy the text tables of design D13 into `en.ftl` and `pl.ftl`. First write a failing test that each key of the tables has its text in both bundles. Check with `cargo test -p recipe-web i18n`.
- [x] 8.2 (recipe-web) Map each field reason, missing entity, password reason, task stage, and operation to its key of design D13 (R2). First write failing tests for each variant in both bundles. Check with `cargo test -p recipe-web error`.
- [x] 8.3 (recipe-web) Give the field line links the class `.target-link` (R1). Serve the four static files from one table (R3). First write failing tests. Check with `cargo test -p recipe-web static_files`.
- [x] 8.4 (recipe-web) Make the template text test reject string literals and `value` texts (R7). Move the duplicated test helpers to one place (R8). Check with `cargo test -p recipe-web template_tests`.
- [x] 8.5 (recipe-web) Fix R9 to R13 of the code review: fields, data class, table data, empty state, tree node, and form edge. First write failing tests. Check with `cargo test -p recipe-web page_tests`.
- [x] 8.6 (recipe-web) Harden the test helpers and extend the scenario tests (R14, R15). The failing helper tests come first. Check with `cargo test -p recipe-web xss`.
- [x] 8.7 (recipe-web) Fix R16 and R17 of the code review: buttons, description list, dates, tree leaf, unused classes, and macro branches. First write failing tests. Check with `cargo test -p recipe-web components_tests`.

## 9. Verification

- [x] 9.1 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` with `cargo llvm-cov --jobs 2`. Also run `cargo deny --locked check` and `openspec validate web-ui-foundation --strict`. Check with the output of each command.

## Workflow follow-up

- The code review runs `code-review`, `design-review`, `accessibility-audit` on `/login` and `/recipes`, and a review of the CSP, XSS, `hx-*`, and `|safe`.
- The code review loads both scripts on a test page in a browser and records the result.
- Before the archive, run `ste-lint.py` on the base spec `web-ui` merged with the delta.
- Just before the archive, the orchestrator runs `cargo test --workspace` and shows the result.
- Archive with the `openspec` CLI. A hook runs `cargo test --workspace` first.
- Step P3.2 adds the HTMX load of the tree children. Step P3.3 replaces the skeleton of `/recipes` and adds the error line under an input.
- Step P4.3 adds the zoom of 150 % on a machine panel to the operator guide.
- Step P4.2 decides how the success message of an HTMX password change reaches the page after `HX-Refresh` (code review R6).
