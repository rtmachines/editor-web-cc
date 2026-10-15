# Tasks

The table "Waves" of design D18 gives the agent, the model, and the file set of each task. At most two agents that run cargo work at the same time. The waves run in this order:

1. Wave 1: H1 (task 3.1) first. Then A1 (tasks 2.1 to 2.4) and R1 (tasks 4.1 to 4.3).
2. Wave 2: A2 (tasks 5.1 to 5.4) and W1 (tasks 6.1 to 6.4).
3. Wave 3: W2 (tasks 7.1 to 7.4) and W3 (tasks 8.1 to 8.4).
4. Wave 4: W4 (tasks 9.1 to 9.4) and W5 (task 10.1).
5. Wave 5: W6 (tasks 11.1 to 11.3) and H2 (task 11.4).
6. Wave F1: H3 (task 13.1) and D1 (tasks 13.2 to 13.4).
7. Wave F2: W7 (tasks 13.5 to 13.7) and W8 (tasks 13.8 to 13.11).
8. Wave F3: W9 (tasks 13.12 to 13.14) alone.
9. Wave F4: W10 (task 13.15) alone.
10. Wave F5: W11 (task 13.16) and W12 (tasks 13.17 to 13.19).

After each wave, the orchestrator runs the full CI of the workspace. The next wave starts only after a green result.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0037 with the decisions (a) to (y), its index row, and its rows in the decision map. Add the "Amended by" note to ADR-0036. Check with `ste-lint.py` on the scope of ADR-0023.
- [x] 1.2 (docs/architecture) Change `design-system.md` sections 1, 6, 8, 9, 10.1, 10.2, 10.5, 10.8, 11, and 12. Change `api.md` sections 1 and 2.1 to 2.3 and `reliability.md` section 7.11. Check with `ste-lint.py`.
- [x] 1.3 (.claude/rules) Add the rules of `hx-boost`, of the CSRF token in a URL, and of the two style sheets to `web.md`. Add the document entries of `CHANGELOG.md`. Check with `ste-lint.py` on both files.
- [x] 1.4 (CHANGELOG.md) After wave 5, add the entries of the queries, the ports, the pages, and the development server. Check with `ste-lint.py CHANGELOG.md`.
- [x] 1.5 (docs/adr, docs/architecture, .claude/rules) Write ADR-0038 with its index row and its rows in the decision map. Add (z) and the note of `archived` in (x) to ADR-0037. Change `api.md` sections 1 and 5.3, I8 of `domain-model.md`, and `web.md`. Check with `ste-lint.py`.
- [x] 1.6 (CHANGELOG.md) Add the entries of ADR-0038 and of the review fixes. Check with `ste-lint.py CHANGELOG.md`.

## 2. Ports, adapters, and the usage query (wave 1, agent A1)

- [x] 2.1 (recipe-app, recipe-web) Add the three variants of `Missing` and the nine variants of `FieldReason` of design D5 with their keys. First write failing tests in both crates. Check with `cargo test -p recipe-app missing` and `cargo test -p recipe-web error`.
- [x] 2.2 (recipe-app, recipe-db) Add `list_machines`, `MachineFilter`, and `MachineSummary` of design D4 to the port, the memory adapter, `short_rows.rs`, and the sqlx adapter. First write the failing contract cases of D4. Check with `cargo test -p recipe-db --test storage machine_list`.
- [x] 2.3 (recipe-app, recipe-db) Add `type_usage` and `TypeUsageRow` of design D4 with both adapters. First write the failing contract cases of D4, also the case of two members of one struct. Check with `cargo test -p recipe-db --test storage usage`.
- [x] 2.4 (recipe-app) Add `PageSizes` to `AppSettings`, `type_text.rs`, and the query `GetTypeUsage` of design D3. First write failing tests on `MemoryStore` for the spec "Type usage" and the round trip of D3. Check with `cargo test -p recipe-app queries`.

## 3. Texts (wave 1, agent H1, first)

- [x] 3.1 (recipe-web) [mechanical] Copy the text table of design D13 into `en.ftl` and `pl.ftl`. First write a failing test in `i18n/plant_texts_tests.rs` that each key has the text of the table in both bundles. Check with `cargo test -p recipe-web i18n`.

## 4. Style sheets, route areas, and test support (wave 1, agent R1)

- [x] 4.1 (recipe-web) Split the style sheet into `app.css` and `components.css` of design D9. Serve the new file, link it in `base.html`, and check the order in `page_tests.rs`. First write the failing tests of D9. Check with `cargo test -p recipe-web style_tests` and `cargo test -p recipe-web static_files`.
- [x] 4.2 (recipe-web) Add `Fixture::seed_plant` of design D12 in `test_support/plant.rs`. First write a failing test that `seed_plant` holds the data of D12. Check with `cargo test -p recipe-web seed_plant`.
- [x] 4.3 (recipe-web) Move the matrix into `routes/matrix.rs`, add the three route areas of design D12, and turn `PAGES` into a function. Give `xss_tests.rs` its own list. First write a failing test that the table collects the rows of each area. Check with `cargo test -p recipe-web routes`, `sweep`, and `xss`.

## 5. Queries (wave 2, agent A2)

- [x] 5.1 (recipe-app) Add `ListMachines`, `GetMachine`, and `GetMachineForm` of design D3 and the source test of design D14. First write failing tests for the specs "Machine list", "Archive message", and "Unit tree of a machine". Check with `cargo test -p recipe-app queries::machines`.
- [x] 5.2 (recipe-app) Add `GetUnitTree`, `GetUnitForm`, and `GetUnit` of design D3. First write failing tests for the specs "Unit tree of a machine" and "Proposal of a new unit". Check with `cargo test -p recipe-app queries::units`.
- [x] 5.3 (recipe-app) Add `ListTypes` and `GetTypeTree` of design D3. First write failing tests for the specs "Type list of a machine" and "Type tree of a revision". Check with `cargo test -p recipe-app queries::types`.
- [x] 5.4 (recipe-app) Add `GetUdtForm` with the `datalist` proposals of design D3 and the first-statement tests of the three form queries. First write failing tests for the spec "Form of a new revision". Check with `cargo test -p recipe-app queries` and `cargo test -p recipe-app first_statement`.

## 6. Web foundation (wave 2, agent W1)

- [x] 6.1 (recipe-web) Add the content answer of design D6, `see_other_path`, and `pages/path_values.rs` of design D5. First write failing tests for the spec "Content answer of a page" on a test page and for each parser. Check with `cargo test -p recipe-web layout` and `cargo test -p recipe-web path_values`.
- [x] 6.2 (recipe-web) Add the tree macros, the archived badges, the danger disclosure, and the heading of design D10 with their style rules. First write failing tests with test templates. Check with `cargo test -p recipe-web components_tests` and `cargo test -p recipe-web component_rules`.
- [x] 6.3 (recipe-web) Add `select`, `check`, `listed`, `member_row`, and `pager` of design D10 with their style rules. First write failing tests with test templates, also for the encoded pager link. Check with `cargo test -p recipe-web components_tests`.
- [x] 6.4 (recipe-web) Add the field names of design D5, `INPUT_FIELDS`, and `WebError::BadRow` with its error line. First write failing tests for the spec "Field error lines". Check with `cargo test -p recipe-web error`.

## 7. Machine pages (wave 3, agent W2)

- [x] 7.1 (recipe-web) Add the navigation link of design D11 and `GET /machines` of design D6 with the archive message, the pager, and the checkbox. First write failing tests for the specs "Machine list page" and "Navigation by permission". Check with `cargo test -p recipe-web machines`.
- [x] 7.2 (recipe-web) Add `GET /machines/new` and `POST /machines` of design D6. First write failing tests for the spec "Machine form and creation", each field reason of the form, and the content answer. Check with `cargo test -p recipe-web machines`.
- [x] 7.3 (recipe-web) Add the machine page and the page of an archived machine of design D6 with their forms. First write failing tests for the spec "Machine page" and the unit message of the spec "Archive message". Check with `cargo test -p recipe-web machines`.
- [x] 7.4 (recipe-web) Add the tree view model of design D7 and `GET /machines/{code}/units`. First write failing tests for the spec "Unit tree with the disclosure pattern" and the scenario "State of an archived unit". Check with `cargo test -p recipe-web machines`.

## 8. Type pages (wave 3, agent W3)

- [x] 8.1 (recipe-web) Add `GET /machines/{code}/types` of design D6 with its matrix rows and sweep pages. First write failing tests for the type list of the spec "Type pages". Check with `cargo test -p recipe-web types`.
- [x] 8.2 (recipe-web) Add `form_rows.rs` and `GET /machines/{code}/types/new` of design D8. First write failing tests for each rule of D8 and the scenarios "Form of a new revision" and "CSRF token in the query". Check with `cargo test -p recipe-web types`.
- [x] 8.3 (recipe-web) Add both `POST` routes of `DefineUdt` with `add=1` of design D8. First write failing tests for the spec "UDT definition through the form", the scenario "Add a member row", and each error code of `api.md` section 2.3. Check with `cargo test -p recipe-web types`.
- [x] 8.4 (recipe-web) Add the type tree page and the usage page of design D6. First write failing tests for the spec "Type pages", the type tree groups of D10, and the pager. Check with `cargo test -p recipe-web types`.

## 9. Unit pages and machine commands (wave 4, agent W4)

- [x] 9.1 (recipe-web) Add `GET /machines/{code}/units/new` of design D6 as a page and as a fragment. First write failing tests for the scenarios "Unit form in the machine page" and "All numbers in use", and for a parent of depth 5. Check with `cargo test -p recipe-web units`.
- [x] 9.2 (recipe-web) Add `POST /machines/{code}/units` of design D6 with the focus unit of D7. First write failing tests for the spec "Addition of a unit" and each error code of the route. Check with `cargo test -p recipe-web units`.
- [x] 9.3 (recipe-web) Add the unit page, its rename, and its archive of design D6. First write failing tests for the spec "Unit page", the scenario "Rename form of a unit", and each error code of the two routes. Check with `cargo test -p recipe-web units`.
- [x] 9.4 (recipe-web) Add the rename and the archive of a machine of design D6. First write failing tests for the spec "Rename and archive of a machine" and each error code of the two routes. Check with `cargo test -p recipe-web machines`.

## 10. Development server (wave 4, agent W5)

- [x] 10.1 (recipe-web) Add `examples/dev_server.rs` of design D15. First write failing unit tests in the example for the bind address, the three seeded users, a missing password variable, and the signing key. Check with `cargo test -p recipe-web --example dev_server`.

## 11. Security and access (wave 5, agents W6 and H2)

- [x] 11.1 (recipe-web) Send the XSS corpus through each field and path value of design D14. This task adds tests for existing code, so the first run can pass. Check with `cargo test -p recipe-web plant_xss`.
- [x] 11.2 (recipe-web) Check each new page and fragment in `pl` and `en` for the `hx-*` rule and the CSRF token rule of D14. Check with `cargo test -p recipe-web plant_sweep`.
- [x] 11.3 (recipe-web) Send each new `POST` route without a CSRF token and in read-only mode. Check with `cargo test -p recipe-web plant_security`.
- [x] 11.4 (recipe-web) [mechanical] Write `routes/plant_access_tests.rs` with each route of `api.md` sections 2.2 and 2.3 and each of the six principals. Take each code from the matrix of `api.md` section 3.3, and check the `forbidden` event. Check with `cargo test -p recipe-web plant_access`.

## 13. Fixes of the code review (design D20, waves of D21)

- [x] 13.1 (recipe-web) [mechanical] Copy the table "Texts of the review fixes" of design D13 into `en.ftl` and `pl.ftl`. Change the texts of `field-query`. First write a failing test in `i18n/review_texts_tests.rs`. It checks that each key has the text of the table in both bundles. Check with `cargo test -p recipe-web review_texts`.
- [x] 13.2 (recipe-domain, recipe-app) Add the check of the struct names of ADR-0038 and run it in `DefineUdt` (U28). First write failing tests for the scenarios "Earlier revision of the same struct" and "One struct on two paths". Keep the test of a cycle by identifier. Check with `cargo test -p recipe-domain cycle` and `cargo test -p recipe-app define_udt`.
- [x] 13.3 (recipe-app) Leave the revised struct out of the type proposals of `GetUdtForm` (U30). Keep one copy of `page_window`, `missing_type`, `live_machine`, and the revision lookup (U11). First write a failing test for the scenario "Proposals without the revised struct". The refactor keeps the other tests green. Check with `cargo test -p recipe-app queries`.
- [x] 13.4 (recipe-db, recipe-app) Use `UNION ALL` in `edges` (U20). Add one offset helper with a unit test of its error (U21). Name the error in the port documents, and return the same error in the memory adapters. First write a failing unit test of the helper and a failing test of a memory adapter. Check with `cargo test -p recipe-db window` and `cargo test -p recipe-db --test storage usage`.
- [x] 13.5 (recipe-web) Map the fields of `PlantError` into the arguments of the messages (U1). First write failing tests in `i18n/plant_error_tests.rs`: the English text of each code equals its registry row, and `pl` has a text. Also write a failing test for the scenario "Plant error in Polish". Check with `cargo test -p recipe-web plant_error`.
- [x] 13.6 (recipe-web) Set `HX-Push-Url: false` on each HTMX error answer (U3), and give a `{path}` that is not UTF-8 the code `E-PLT-017` (U27). First write failing tests for the scenario "Refused machine form keeps the URL" and for the path. Check with `cargo test -p recipe-web error` and `cargo test -p recipe-web path_values`.
- [x] 13.7 (recipe-web) Move `Texts::with_arg` to `texts.rs`, and let the key test read the literal keys of the `.rs` files (U18). First write the failing test of the reader. Check with `cargo test -p recipe-web texts`.
- [x] 13.8 (recipe-web) Add `#unit-tree` and `#unit-form` to the full tree page (U2). Add the parameter `value` to `fields::named`, and show the current name in each rename input (U9). First write failing tests for the scenarios "Add link on the full tree page" and "Rename form with the current name". Check with `cargo test -p recipe-web machines` and `cargo test -p recipe-web units`.
- [x] 13.9 (recipe-web) Add the empty platform option (U4) and the search text of spaces only (U17). Add the text of a page after the end (U16). First write failing tests for the scenarios "No platform chosen" and "Search text of spaces only". Then write one for "Page after the end", also for the usage page. Check with `cargo test -p recipe-web machines` and `cargo test -p recipe-web types`.
- [x] 13.10 (recipe-web) Add the gaps and widths of U5, U6, U8, and U16 to the style sheets and to `unit.html`. First write a failing style test for each rule of design D20. Check with `cargo test -p recipe-web style_tests` and `cargo test -p recipe-web component_rules`.
- [x] 13.11 (recipe-web) Change the tree row, the member row, the revision links, and the type tree of U7, U15, and U32. First write failing tests for each rule and each template change. Check with `cargo test -p recipe-web component_rules` and `cargo test -p recipe-web types`.
- [x] 13.12 (recipe-web) Keep one copy of `row`, `query_values`, and `instant` (U12). The refactor keeps the tests green. Check with `cargo test -p recipe-web types` and `cargo test -p recipe-web routes`. U14 moved to task 13.15, because it needs `recipe-app`.
- [x] 13.13 (recipe-web) Move the plant test world and the copied test helpers into `test_support/plant.rs` and `pages/test_helpers.rs` (U13). This task moves tests, so each test stays green. Check with `cargo test -p recipe-web --lib`.
- [x] 13.14 (recipe-web) Make the weak tests exact (U19). Send the XSS corpus into `next` of each plant page that accepts a query (U10). This task changes tests only, so the first run can pass. Check with `cargo test -p recipe-web plant_` and `cargo test -p recipe-web xss`.

- [x] 13.15 (recipe-app, recipe-web) `GetTypeTree` gives the text of each member type, written with `type_text`. The type tree page shows that text (U14). First write a failing test in `recipe-app` for the text of a member of the global type `STRING`. Then change the leaf `("label", "STRING(254)")` of `types/tree_tests.rs` to `("label", "STRING")`. Check with `cargo test -p recipe-app queries` and `cargo test -p recipe-web types`.

- [x] 13.16 (recipe-web) Give `.tree-group` the top gap `--space-4` (RQ-1), and let a link of `.link-list` keep its own width (RQ-4). Mark the platform list as required (RQ-2). Show no previous link on a page after the end (RQ-5), and fix the comment of `unit-form.html` (RR-8). First write a failing test for each rule. Check with `cargo test -p recipe-web style_tests`, `machines`, and `types`.
- [x] 13.17 (recipe-app, recipe-db) Compute the page offset with `checked_mul` (RR-1). Return the storage error of U21 from the memory adapter of the user list, and name it in the port document (RR-2). First write a failing test of the memory adapter. Check with `cargo test -p recipe-app memory` and `cargo test -p recipe-db --test storage users`.
- [x] 13.18 (recipe-web) Check `{code}` before `{path}` (RR-4), and give a `{name}` that is not UTF-8 `Missing::MalformedType` (RR-5). Let the key test read `t.with_arg("` (RR-3). Turn `samples()` into a table (RR-6), and fix the permission name of `plant_access_tests.rs` (RR-9). First write failing tests for RR-4 and RR-5. Check with `cargo test -p recipe-web path_values`, `texts`, and `error`.
- [x] 13.19 (recipe-web) Add HTTP tests for the proposals of the form of a new revision and of its `add=1` answer (RR-7). Add one for `E-PLT-005` of a revision that holds an earlier revision of itself. This task adds tests only, so the first run can pass. Check with `cargo test -p recipe-web types`.

## 12. Verification

- [x] 12.1 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` with `cargo llvm-cov --jobs 2`. Also run `cargo deny --locked check` and `openspec validate web-plant-model --strict`. Check with the output of each command.

## Workflow follow-up

- The code review runs `code-review`, `design-review`, and `accessibility-audit` on the tree with 5 levels and 64 units and on the UDT form. It also runs a security review and a review of `recipe-db`.
- The test in a browser uses `examples/dev_server.rs`. It creates the S7-300 machine, the units down to `U1.2.4.5.6`, and the path `U1.2.4.5.6.7` (`E-PLT-002`).
- It then defines a UDT, the member `ST_A@1` inside `ST_A` (`E-PLT-013`), a duplicate member (`E-PLT-021`), a new revision, and the usage page.
- It also checks the focus after each swap and the address bar after each `303`. It checks the `__Host-` cookie on `127.0.0.1` and the tree at 320 CSS px in `pl`.
- Before the archive, run `ste-lint.py` on each base spec merged with its delta.
- Just before the archive, the orchestrator runs `cargo test --workspace` and shows the result.
- Archive with the `openspec` CLI. A hook runs `cargo test --workspace` first.
- After wave F2, the orchestrator checks U2, U3, U5 to U8, and U32 again in the browser with `examples/dev_server.rs`.
- Step P3.3 adds the error line under an input, also for a member row (U25, U33).
- Step P4.2 takes the deferred findings U22, U23, U26, U29, RQ-3, and RR-4 of design D20. Step P4.2 decides whether an error answer empties `#toast` (plan review F39).
