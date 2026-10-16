# Tasks

The table "Waves" of design D18 gives the agent, the class, the model, the effort, and the file set of each task. Each code task starts with a failing test.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0039 to ADR-0043, the index rows, the decision map, and the "Amended by" notes of ADR-0001, ADR-0027, ADR-0029, and ADR-0036. Check: `ste-lint.py` on the scope of ADR-0023 reports 0 hard violations.
- [x] 1.2 (docs/architecture) Put the decisions into `api.md`, `reliability.md`, `domain-model.md`, `data-model.md`, and `design-system.md`. Check: `ste-lint.py` reports 0 hard violations.
- [x] 1.3 (.claude/rules) Add the rules of ADR-0040, ADR-0039, ADR-0041, and ADR-0042 to `web.md` and `sql.md`. Check: `ste-lint.py` on the two files reports 0 hard violations.
- [x] 1.4 (CHANGELOG.md) Add the features of this change under "Added" and "Changed". Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 2. Texts

- [x] 2.1 (recipe-web) [mechanical] Write a failing test in `src/i18n/recipe_texts_tests.rs` that reads each key of design D13 in `en` and `pl`. Then copy the table D13 into `i18n/en.ftl` and `i18n/pl.ftl`.

## 3. Values from texts and fault lists

- [x] 3.1 (recipe-domain) [hard] Write failing tests for `LeafTexts::parse`, `ParameterValue::from_texts`, and `ParameterValue::texts` with every leaf class. Add a round-trip property test. Then build them (design D3).
- [x] 3.2 (recipe-domain) [hard] Write failing tests that compare `expected` of `ValueShapeMismatch` with an `Expected` variant. Then replace the text with the type `Expected` (design D3).
- [x] 3.3 (recipe-domain) [hard] Write failing tests with two bad leaves and two bad limit rows. Then make the draft functions return every fault in one `RecipeErrors` (design D3, ADR-0041).
- [x] 3.4 (recipe-domain) [hard] Write failing tests for a rebind with a text of a new member. Then add `MemberValue` and convert the text by the class of the new leaf.
- [x] 3.5 (recipe-app) [hard] Write failing tests of `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, and `RebindTypeRevision` with texts. Then change their inputs to `ValueInput` and `MemberValue` (design D4).

## 4. Recipe queries and read ports

- [x] 4.1 (recipe-app) Write failing tests of the memory adapter for each method of `RecipeReads` and for `Isolation::RepeatableRead`. Put them into new test files. Then add the port, the accessor, the memory adapter, and `PageSizes` (design D4, D5).
- [x] 4.2 (recipe-app) Write failing tests of `ListRecipes` and `ListDeletedRecipes` for the filters, the order, the page, and the permission. Then write both queries (design D4).
- [x] 4.3 (recipe-app) Write failing tests of `GetRecipe`, `ListVersions`, `GetRecipeForm`, and `GetVersionForm`, also on a deleted recipe. Then write the four queries.
- [x] 4.4 (recipe-app) Write failing tests of `GetVersion` with the header, the outdated parameters, and `LastChange`. Then write the query with `Isolation::RepeatableRead`.

## 5. Adapters of the read ports

- [x] 5.1 (recipe-db) [hard] Write a failing test that `Isolation::RepeatableRead` opens `REPEATABLE READ READ ONLY`. Then add the statement and the accessor `recipe_reads` of `PgTransaction`.
- [x] 5.2 (recipe-db) [hard] Write failing contract cases for `list_recipes` with search, machine, state, deleted rows, Polish letters, and the page. Then write the SQL (design D5).
- [x] 5.3 (recipe-db) [hard] Write failing contract cases for `version_page`, `version_detail`, and `last_change`. Then write the SQL with the index of the audit log.
- [x] 5.4 (recipe-db) [hard] Write failing contract cases for `parameter_page`, `unit_counts`, and `parameter_by_key`, and the case of `repeatable_read.rs` of design D5. Then write the SQL. The grant test stays green.

## 6. Parameter queries

- [x] 6.1 (recipe-app) [hard] Write failing tests of `ListParameters` for the unit filter, the page, and the unit pages. Add a test of a fixed count of reads. Then write the query.
- [x] 6.2 (recipe-app) [hard] Write failing tests of `GetParameter`, `GetLimits`, and `GetTranslations` in a snapshot. Then write the three queries.
- [x] 6.3 (recipe-app) [hard] Write failing tests that the preview tree of `GetParameterForm` equals the tree after `EnsureDerivedType` and writes nothing. Then write `preview.rs` and the query.

## 7. Error lists and conflict

- [x] 7.1 (recipe-web) [hard] Write failing tests for `WebError::Fields` with lines of `BadRequest`, `BadRow`, `RecipeErrors`, and `E-RCP-020` items, and for the 14 new `FieldReason` variants. Then add them and `error/lists.rs` (design D6, D9).
- [x] 7.2 (recipe-web) [hard] Write failing tests for the summary of `field-errors.html` and its cap of 20 lines. Add tests of the out-of-band form and of the clean form after a success. Then write the answer, the error line macro, `RenderedForm`, and `draft_reply.rs`.
- [x] 7.3 (recipe-web) [hard] Write failing tests that each `E-RCP` code and `E-PLT-019`, `E-PLT-020`, and `E-PLT-030` to `E-PLT-038` give their message in `pl` and `en`. Add the test against the registry. Then write `error/recipe.rs`.
- [x] 7.4 (recipe-web) [hard] Write failing tests for `HX-Trigger` on success and for `conflict.html` with `LastChange`, without it, and after a failed `GetVersion`. Then write the success answer and the conflict answer (design D10).

## 8. Web foundation

- [x] 8.1 (recipe-web) Write failing tests for the path values `{ref}`, `{n}`, `{key}`, and `{lang}` and their codes. Then write the parsers (design D6).
- [x] 8.2 (recipe-web) Write failing tests for the body limit of each route group, of the CSRF buffer, and of a chunked body. Put them into new test files. Then write the limit layer and the setting `translation_langs` (design D11).
- [x] 8.3 (recipe-web) Write failing tests that the route table holds the three recipe areas and that `seed_recipes` and `seed_large_draft` seed design D12. Then write the areas, the `route_tests.rs` files, `fill()`, and the fixtures.
- [x] 8.4 (recipe-web) Write failing style tests for the rules of the leaf grid, the error line, and the states Deleted and Outdated. Then write the rules and the badge macros.

## 9. Recipe and version pages

- [x] 9.1 (recipe-web) Write failing tests for the recipe list with its filters, its page, the deleted recipes, and `403 E-WEB-002`. Then write the page.
- [x] 9.2 (recipe-web) Write failing tests for the recipe form, `POST /recipes`, the recipe page, the rename, and the pages of a deleted recipe. Then write them.
- [x] 9.3 (recipe-web) [hard] Write failing tests for the version list, the version form, `POST .../versions`, `422 E-WEB-008` for `based_on`, and the `E-RCP-020` lines. Then write them.
- [x] 9.4 (recipe-web) [hard] Write failing tests for the version page and the unit links with `aria-current`. Add a draft with 4 000 parameters: at most 100 rows and a byte bound. Then write the page.

## 10. Value form

- [x] 10.1 (recipe-web) [hard] Write failing tests for the reader of value fields, limit rows, `new_member_values`, `fields`, `limit_mode`, and `value_type`. Cover each `E-WEB-008` case, the flood of pairs, and a long key. Then write `fields.rs`.
- [x] 10.2 (recipe-web) [hard] Write failing tests for `GridView` of each type of design D16. Add the limit modes with their copy rules, the groups, and the hints. Then write `grid.rs`.
- [x] 10.3 (recipe-web) [hard] Write failing component tests for the macros of the leaf grid, the button order, and the read-only fragments. Then write the templates (design D8).

## 11. Parameter routes

- [x] 11.1 (recipe-web) [hard] Write failing tests for the form of a new parameter, `fields=1`, `limit_mode`, read-only mode, and `AddParameter` with each error code. Then write the routes.
- [x] 11.2 (recipe-web) [hard] Write failing tests for the parameter page and the edit form through `/definition`. Add the clean form after a save and a `value_type` mismatch. Then write them and the routes `/value` and `/limits`.
- [x] 11.3 (recipe-web) [hard] Write failing tests for the translation rows, their `POST` routes, and the two removals. Then write them.
- [x] 11.4 (recipe-web) [hard] Write failing tests for `409 E-WEB-001` with two sessions on each draft route and after a form rendered again. Then complete the handlers.
- [x] 11.5 (recipe-web) [hard] Write failing tests for `409 E-RCP-012` on each parameter route of a deleted recipe and for `503 E-WEB-006` of each route. Then complete the handlers.

## 12. Rebind

- [x] 12.1 (recipe-web) [hard] Write failing tests for the list of outdated parameters with the state Outdated on the version page. Then write the list.
- [x] 12.2 (recipe-web) [hard] Write failing tests for the rebind form, `POST .../rebind`, the lines of `E-RCP-028` and `E-RCP-029`, and `409 E-WEB-001` with two sessions. Then write the form and the route.
- [x] 12.3 (recipe-app, recipe-web) [hard] Write failing tests for the bound of the rebind forms (D21, ADR-0044). An Operator gets no new member leaves and no rebind form. A sum over `form.rebind_leaves` gives the notice `rebind-too-large` without fields. Then write the bound in `GetVersion` and on the version page.

## 13. Error lists of the plant forms

- [x] 13.1 (recipe-web) [hard] Write failing tests for the machine, unit, and rename forms. Each answers every bad field with a line under its input, and a success with a clean form. Then use design D9.
- [x] 13.2 (recipe-web) [hard] Write failing tests for the UDT form. It answers every bad member row with a line under each input (U25, U33). Then use design D9.

## 14. Security and sweep

- [x] 14.1 (recipe-web) Write the XSS tests of design D14 for every new field, field name, and path value. They pass on the first run, or they find a defect.
- [x] 14.2 (recipe-web) Write the sweep tests of design D14 for each new page in `pl` and `en`. Check the literal `hx-*` values and the URLs without a token.
- [x] 14.3 (recipe-web) Write the tests of design D14 for a missing CSRF token and for read-only mode on each new route.

## 15. Routes and roles

- [x] 15.1 (recipe-web) [mechanical] Write a test of the `forbidden` event and the code in the body for each new route. Use the shape of `routes/plant_access_tests.rs`. The area rows of the matrix cover each principal.

## 16. Verification

- [x] 16.1 (workspace) Run each local check of `CONTRIBUTING.md` and `openspec validate web-recipe-editor --strict`. Check: `tools/coverage-gate.py` reports at least 80 % per crate.
- [x] 16.2 (workspace) Run the test in a browser of design D16 on `examples/dev_server.rs`. Check: 320 px without sideways page scroll, the keyboard path through the grid, Enter saves, and the focus after a swap.

## 17. Fixes of the code review

- [x] 17.1 (recipe-web) Write failing tests for an array inside a group (D22, ADR-0045). Then keep the rule in grid/walk.rs.
- [x] 17.2 (recipe-domain, recipe-web) [hard] Write failing tests for the messages and the translation rows. An integer leaf gets an integer hint. Cover E-RCP-029 on a nested revision and the pl text of E-RCP-054. A translation row outside the list is read-only. An unknown member path of a rebind gives E-RCP-005. An Outdated parameter without a form gets a hint.
- [x] 17.3 (recipe-web) [hard] Write failing tests for the answers. A refused save without HTMX renders the form page with its lines and texts. An error answer clears the toast. A success shows its toast in view. A removal sends its toast.
- [x] 17.4 (recipe-app, recipe-web) [hard] Write failing tests for the permission check of the form routes (D23, ADR-0046). Then add the purposes of the form queries and run them before the fields.
