# Design: web-recipe-editor

## Context

See `proposal.md`, section Why. Step P3.2 and the earlier steps left these parts:

- `recipe-app` has the recipe commands of `api.md` sections 2.4 to 2.6. It has the operations and the matrix rows of the recipe queries (`auth/operation/mod.rs`, `auth/matrix.rs`). It has the stub `ListRecipes` in `recipe/queries.rs` and no other recipe query.
- `RecipeRepository` reads a recipe by reference or identifier and all parameters of one version. No port lists recipes, reads a page of parameters, or reads one parameter by key.
- The draft functions of `recipe-domain` take the value as JSON text (`NewParameter.value_json`, `Draft::set_value`, `Retype.value_json`). `ParameterValue::parse` and `ParameterValue::check` stop at the first fault. `ValueFault` and `RecipeError::ValueShapeMismatch` carry `expected` as English text.
- `RebindRequest.new_member_values` maps a key and a leaf path to a `serde_json::Value`.
- `Isolation` has `ReadCommitted`, `Serializable`, and `ReadOnly`. `recipe-db` opens `ReadOnly` with `READ COMMITTED READ ONLY`.
- `AppError::ConcurrentEdit` holds the reference, the number, and the two row versions.
- `recipe-web` applies `http.body_limit.form` to every route (`routes.rs`, `limits.rs`, `csrf.rs`). No code sets `HX-Trigger`, and `conflict.html` does not exist.
- `WebError` holds one field per error (`BadRequest`, `BadRow`). `answer.rs` maps no `RecipeError` argument, so an `E-RCP` message falls back to English.
- The bundles hold no `error-E-RCP-*` key and no `error-E-PLT-019`, `error-E-PLT-020`, or trigger key `error-E-PLT-030` to `error-E-PLT-038`.
- `Fixture::seed_plant` seeds the recipe `R-100` with three versions and one parameter. `examples/dev_server.rs` seeds only the primitive types.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Decisions of this change | ADR-0039 to ADR-0043 |
| Routes, use cases, permissions, errors | `api.md` sections 1, 2.4 to 2.6, 3, 4, 5 |
| Body limits, CSRF, CSP, scripts, `hx-*` rule | `api.md` sections 7.3, 7.6, 9, `.claude/rules/web.md` |
| Value, leaf class, leaf path, IEC literal, S7 address | `domain-model.md` sections 2.1, 2.2, `reliability.md` sections 4.3, 4.4 |
| Lists of errors | ADR-0027, ADR-0041 |
| Locks, transactions, isolation | ADR-0015, ADR-0042, `data-model.md` section 6, `reliability.md` section 7.11 |
| Tokens, components, leaf grid, groups, states | `design-system.md` sections 6 to 10 |
| Settings | `reliability.md` section 7.11 |
| Pages of P3.2, field names, content answer | archived design `web-plant-model` D5, D6, D12, ADR-0037 |
| Layout, components, texts of P3.1 | archived design `web-ui-foundation` D1 to D13 |
| Pipeline, CSRF, error mapping | archived design `identity-and-sessions` D11 to D13 |

## Goals / Non-Goals

**Goals:**

- Each route of `api.md` sections 2.4 to 2.6 in scope answers each role as the matrix of `api.md` section 3.3 says.
- Each query checks its permission first. A draft query reads one snapshot.
- Each new page renders in `pl` and `en` with the layout, the CSP, and no inline script, style, or handler.
- No `hx-*` attribute holds user data, and no URL holds the CSRF token.
- One save of a parameter changes its value and its limits together, and an error answer lists every fault at once.

**Non-Goals:**

- No migration, no new grant, no new dependency, and no change of the CSP or of a script.
- No change of `recipe-export` or `recipe-server`.
- No wall-clock budget in a debug test. The load test of `reliability.md` section 6 measures time in P4.2.
- No error list for the login, password, and user forms (ADR-0041 (g)).

## Decisions

### D1. Answers of the user (2026-10-15)

| Question | Answer | Record |
|---|---|---|
| Editor variant | A: leaf grid, one save through `/definition` | ADR-0039 (i) to (n) |
| a. Value without JavaScript | one field per leaf, `value` and the leaf path | ADR-0039 (a) to (h) |
| b. `PUT` and `DELETE` | `POST` routes | ADR-0040 |
| c. Large arrays | whole value in one form, groups of `form.array_chunk` | ADR-0039 (l) |
| d. Rebind | in P3.3 | ADR-0043 (l) |
| e. Page sizes | `page.recipes` and `page.versions` = 100 | ADR-0043 (a) |
| f. Field errors | every error at once, `RecipeErrors`, line under each input | ADR-0041 |
| g. Translation languages | fixed list `i18n.translation_langs`, one row per tag | ADR-0043 (f) |
| h. Performance test | structural tests in debug | D16 |
| i. S7 address | as A31 | ADR-0043 (g) |
| j. Missing `based_on` | `422 E-WEB-008` | ADR-0043 (h) |
| k. Deleted recipe | Administrator reads, others `409 E-RCP-012` | ADR-0043 (e) |
| l. Isolation of queries | `REPEATABLE READ READ ONLY` | ADR-0042 |

| Plan review, 47 findings (P1 to P47) | all accepted. The orchestrator notes on P4, P20, P36, and P42 hold | ADR-0039, ADR-0041, ADR-0042, ADR-0043, this design |

The fixed list of point g makes a select needless. Each tag of the list gets its own form, because the `action` holds the tag (ADR-0040 (a)).

### D2. Module layout

- `recipe-domain`:
  - `recipe/value/texts.rs` (new): `LeafTexts`, `ParameterValue::from_texts`, `ParameterValue::texts` (D3).
  - `recipe/value/expected.rs` (new): `Expected` (D3). `fault.rs`, `walk.rs`, `leaf.rs`, `parse.rs`, `request_leaf.rs` use it.
  - `recipe/value/check.rs`: the collecting check (D3).
  - `recipe/draft/add.rs`, `add/step_content.rs`, `value.rs`, `definition.rs`: `ValueInput` and the list of faults.
  - `recipe/rebind/mod.rs`, `rebind/value.rs`: `MemberValue`.
  - `recipe/error.rs`: the field `expected` of `ValueShapeMismatch` gets the type `Expected`.
- `recipe-app`:
  - `ports/recipe_reads.rs` (new): the port `RecipeReads` of D5. `ports/unit_of_work.rs`: the accessor `recipe_reads`. `ports/isolation.rs`: `RepeatableRead`.
  - `memory/recipe/reads.rs` (new): the memory adapter. `plant/short_rows.rs` forwards the accessor.
  - `blocking.rs`: `PageSizes` gets `recipes`, `versions`, and `parameters`.
  - `recipe/queries/` (new, it replaces `recipe/queries.rs`): `mod.rs`, `recipes.rs`, `versions.rs`, `parameters.rs`, `parameter_form.rs`, `preview.rs`, `source_tests.rs`, and one test file per module.
  - `recipe/add_parameter.rs`, `update_definition.rs`, `set_value.rs`, `rebind.rs`: the inputs of D4.
- `recipe-db`:
  - `src/unit_of_work.rs`: the statement of `RepeatableRead`.
  - `src/transaction/recipe_list.rs`, `version_reads.rs`, `parameter_reads.rs` (new), declared in `src/transaction.rs`, which also gets the accessor.
  - `tests/storage/contract/recipe_reads/` (new): one file per read, and `repeatable_read.rs` for PostgreSQL only.
- `recipe-web`:
  - `src/error/recipe.rs` (new): the arguments of each `E-RCP` code and the words of D13. `src/error/lists.rs` (new): the field lines of a list.
  - `src/pages/draft_reply.rs` (new): `HX-Trigger`, the conflict answer, and the answer of a form with errors (D10, D9).
  - `src/pages/path_values.rs`: the parsers of `{ref}`, `{n}`, `{key}`, and `{lang}` (D6).
  - `src/pages/recipes.rs` and `src/pages/recipes/`, `src/pages/versions.rs` and `src/pages/versions/`, `src/pages/parameters.rs` and `src/pages/parameters/` (area modules, D12). Each area has `route_tests.rs` with `route_rows()` and `sweep_pages()`.
  - `src/limits.rs`, `src/csrf.rs`, `src/routes.rs`, `src/settings.rs`: the body limits of D11 and the setting `translation_langs`.
  - `src/test_support/recipes.rs` (new): `Fixture::seed_recipes` (D12).
  - `templates/`: the pages and fragments of D7. `templates/components/grid.html` (new): the macros of the leaf grid.
  - `static/components.css` and `src/style_tests/recipe_rules.rs` (new).
- No file passes 800 lines. A new test goes into a new file `<module>/<topic>_tests.rs` when the old file has more than 700 lines.

### D3. Values from texts, typed expectations, and fault lists

- `LeafTexts` holds one text per `LeafPath`. Its constructor `parse` refuses a duplicate path and a path with `[*]`. It keeps the order of the pairs.
- `ParameterValue::from_texts(tree, texts)` walks the leaves of `tree` in preorder.
  - It converts each text by the `LeafClass` of its leaf (ADR-0039 (c)). Then it runs the same canonical walk as `ParameterValue::parse`, so one set of rules checks both inputs.
  - It collects every fault: each missing leaf, each path that the tree does not hold, and each text that does not convert. A missing leaf and an unknown path are `ValueShapeMismatch` (`E-RCP-005`).
- `ParameterValue::texts(tree)` returns one `LeafText { path, class, text }` per leaf in preorder. A number uses the canonical decimal text. A time uses `IecLiteral::to_string`. A test proves the round trip for each class and with `proptest` on random trees.
- `Expected` replaces the text `expected` of `ValueFault` and of `ValueShapeMismatch`. It has one variant for each phrase of `value/` and `draft/` today, and `Bool`, `Decimal`, `Leaf`, and `KnownLeaf` for the texts. Its `Display` keeps the English phrase for the log. `recipe-web` maps each variant to a key `expected-*` of D13. It writes `$max` of `Expected::Char` as `U+` and four or more upper-case hex digits, for example `U+00FF`.
- `ValueInput` is `Json(String)` or `Texts(LeafTexts)`. The draft functions take it. `Json` keeps the existing tests and the JSON rules of `reliability.md` section 4.4. The web layer always sends `Texts`.
- The draft functions of `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, and `ReplaceLimits` collect every fault of the value and of the limits into `RecipeErrors` and return `DraftError::List` (ADR-0041 (b)).
  - The order is the preorder of the leaves, then the order of the limit rows.
  - A list of one entry stays a list, so the web layer has one path.
- `MemberValue` is `Json(serde_json::Value)` or `Text(String)`. The rebind plan converts a text by the class of the leaf in the new revision.
- The leaf texts of a value of `budget.leaves_per_parameter` leaves convert in one walk. No step sorts the whole tree.
  - After that walk, `canonicalize` in `Mode::Stored` checks the built value once more. This second pass is a guard for data safety and stays (decision of wave 2).

### D4. Commands and queries of `recipe-app`

- `AddParameterInput.value` and `RetypeInput.value` are `ValueInput`. `SetParameterValueInput.value` is `ValueInput`. `RebindTypeRevisionInput.new_member_values` maps to `MemberValue`. The existing tests keep `ValueInput::Json`.
- Each query follows the pattern of the plant queries: a struct with `ctx`, `impl UseCase` with its `OPERATION`, `const fn new(ctx)`, `execute(&self, user, meta, input)`, and the `tracing::instrument` attribute.
- The first statement of `execute` is `authorize(Self::OPERATION, ...)`. `source_tests.rs` reads each file of `recipe/queries/` and checks it.
- A query reads in one transaction and ends it before it resolves a type tree or builds a view (`reliability.md` section 7.4).
  - `GetVersion`, `ListParameters`, `GetParameter`, `GetParameterForm`, `GetLimits`, and `GetTranslations` use `Isolation::RepeatableRead` (ADR-0042).
  - The memory transaction treats `RepeatableRead` as `ReadOnly` (`memory/core/transaction.rs`, the comparison with `Isolation::ReadOnly`).
  - The other queries use `Isolation::ReadOnly`.
- A query on a deleted recipe returns `RecipeError::RecipeDeleted` (`E-RCP-012`) unless `Operation::RestoreRecipe` allows the role (ADR-0043 (e)). A form query (`GetVersionForm`, `GetParameterForm`) returns `E-RCP-012` to every role, because a deleted recipe holds no form (decision of wave 1).
- `ListRecipes` and `ListDeletedRecipes` take `RecipeListInput { text, machine, status, page }`. The page counts from 0. `limit = size + 1` shows a next page (archived D3 of `web-plant-model`).
- `GetRecipe` returns the recipe, its machine, and one page of versions (`page.versions`). `ListVersions` returns the same page alone.
- `GetVersion` returns `VersionView`: the header of D5, the platform of the machine, the outdated parameters, and `LastChange { login, at }` (ADR-0043 (m)).
  - Each outdated parameter also holds the new member leaves that its rebind needs. One public domain function computes them from the type rows of the same snapshot. The command and the query use that one rule, and no layer holds a copy of it (wave 5).
  - D21 narrows this rule: only a caller with `ParameterEdit` on a live draft gets the new member leaves. The caller gets them only while their sum stays at or below the bound of `GetVersionInput.rebind_bound`. `VersionView` gives `rebind_sum`, `rebind_over_bound`, and `rebind_bound`.
  - The rebind form reads no parameter value and runs no query per parameter.
- `ListParameters` takes `unit: Option<UnitPath>` and `page`.
  - Without `unit`, a draft with more than `page.parameters` parameters returns the parameters of its first unit and the count of each unit (`api.md` section 2.6).
  - A malformed `unit` gives the code of `UnitPath`. A unit that the version does not use gives `NotFound` (`E-WEB-005`).
- `ListParameters` also returns `type_texts`, the type text of each `PlcTypeId` on the page. It reads `type_rows` in the same snapshot, so the count of reads stays fixed.
- `GetParameter`, `GetLimits`, and `GetTranslations` take the version and the key. `GetParameter` also returns the `TypeTree`, the machine platform, and `type_text`.
- `GetParameterForm` takes the version, an optional key, an optional `TypeExpr`, and an optional `unit`.
  - It returns the live units, the platform, the type proposals of `GetUdtForm`, and the parameter of the key with its `type_text`.
  - With a `TypeExpr`, it returns the preview tree of `preview.rs`.
  - A version that is not a live draft gives `RecipeError::VersionNotEditable` (`409 E-RCP-002`). Only a live draft gets the forms (ADR-0043 (k)).
- `type_text` comes from `plant/queries/type_text.rs` after the end of the transaction.
- `preview.rs` resolves a `TypeExpr` without a write. It reads `type_rows`, adds each missing derived row in memory, and runs `TypeTree::resolve`. It returns the errors of `EnsureDerivedType` (`E-PLT-013`, `E-PLT-018`). A test compares the leaves of the preview with the leaves after `EnsureDerivedType`.
- `PageSizes` gets `recipes`, `versions`, and `parameters`, each 100 by default (`reliability.md` section 7.11).

### D5. Read port `RecipeReads` and its adapters

The signatures are in D19. Each method is one statement, or a fixed number of statements that does not grow with the count of rows.

- `list_recipes(filter, offset, limit)`: rows `RecipeSummary { reference, name, machine_code, deleted, approved: Option<u32>, draft: Option<u32> }`.
  - The SQL keeps a row when `$q IS NULL` or `strpos(lower(reference_number), lower($q)) > 0 OR strpos(lower(name), lower($q)) > 0`.
  - `status` uses `EXISTS` on `recipe_version`, because no index covers `status` (`data-model.md` section 3).
  - The order is `reference_number COLLATE "C"`. The memory adapter uses `to_lowercase` and the same order.
- `version_page(recipe, offset, limit)`: rows `VersionSummary { number, state, change_note, created_at, approved_at }`, newest number first.
- `version_detail(version)`: `VersionDetail` with the header, `change_note`, `created_by` and `approved_by` as logins, `created_at`, `approved_at`, `updated_at`, and `row_version`.
- `last_change(recipe, at)`: the login of the actor of the audit entry with `aggregate_id = $recipe` and `occurred_at = $at`, highest `id` first. The index `audit_log_aggregate_idx` serves it.
- `parameter_page(version, unit, offset, limit)` and `unit_counts(version)`: rows in the order unit path, ordinal, and tag (`COLLATE "C"`).
  - The unit path orders by its numbers, as `UnitPath::cmp` does: `U1.2` comes before `U1.10`. The SQL orders by `string_to_array(substr(path, 2), '.')::integer[]`.
  - `ParameterRow` holds no value. The table does not show it, and 100 values of up to 1 MiB each would load the memory for nothing.
- The memory store records the commit time of each audit entry, so `last_change` matches the time exactly on both harnesses (decision of wave 1).
- `parameter_by_key(version, key)`: the parameter with its limits and its translations.
- `offset` and `limit` reach `sqlx` through `i64::try_from` (`window.rs`). A failed conversion is a `StorageError`.
- `recipe_app` has `SELECT` on each table, so `grants.expected` does not change. The grant test proves it.
- Each behavior has a contract case of `tests/storage/contract/recipe_reads/` that runs on the memory harness and on the PostgreSQL harness.
- `repeatable_read.rs` runs on PostgreSQL only. A second connection commits a draft command between two reads of one `RepeatableRead` transaction. The second read sees the old state.
- The search of `list_recipes` relies on the case mapping of `lower()` under the `lc-ctype` `C.UTF-8` of `reliability.md` section 7.6. The contract case with Polish letters runs on the pinned image with that `initdb` setting.

### D6. Path values, request fields, and field names

- `path_values.rs` gets `reference` (`{ref}`, a malformed value is `Missing::MalformedRecipe`, `E-WEB-005`), `version_number` (`{n}`, `Missing::MalformedVersion`), `parameter_key` (`{key}`, `Missing::MalformedParameter`), and `lang` (`{lang}`, `E-RCP-027`).
- `Missing` gets `MalformedRecipe`, `MalformedVersion`, `MalformedParameter`, and `Parameter { reference, number, key }`. They hold no malformed text.
- `FieldReason` gets the variants of D13 (`reason-*`): `ReferenceNumber`, `ChangeNote`, `Description`, `TranslationDescription`, `Tag`, `Ordinal`, `S7Address`, `BasedOn`, `Status`, `ValueKey`, `LimitRow`, `MaxLength`, `LangList`, and `Reload`. Task 7.1 adds them.
- A malformed `unit_path` gives the code of `UnitPath`, and an unknown unit gives `404 E-PLT-020` (`api.md` section 1). No field reason covers the unit.
- The field names of a form give the input `id`. The `name` attribute keeps the field of `api.md`:

| Form | Field names (`id`) | `name` attributes |
|---|---|---|
| Recipe search | `recipe_q`, `recipe_machine`, `recipe_status`, `show_deleted` | `q`, `machine`, `status`, `show_deleted` |
| New recipe | `machine_code`, `reference_number`, `recipe_name`, `change_note` | `machine_code`, `reference_number`, `name`, `change_note` |
| Rename of a recipe | `recipe_name` | `name` |
| New version | `based_on`, `change_note` | `based_on`, `change_note` |
| Parameter form | `unit_path`, `tag`, `name_en`, `description_en`, `type`, `s7_address`, `ordinal`, `value-<n>`, `limits-<i>-min`, `limits-<i>-max`, `limits-<i>-max_length` | `unit_path`, `tag`, `name_en`, `description_en`, `type`, `s7_address`, `ordinal`, `value<leaf path>`, `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` |
| Translation row | `translation-<lang>-name`, `translation-<lang>-description` | `name`, `description` |
| Rebind form | `new-<k>-<n>` | `new_member_values[<key>]<leaf path>` |

- `<n>` is the preorder index of the leaf, and `<i>` the index of the limit row. `<k>` is the index of the parameter across all rebind forms of the page, so each id stays unique (wave 5).
- Every draft form holds the hidden fields `csrf_token` and `row_version`. A missing or malformed `row_version` is `E-WEB-008` with `FieldReason::Reload`.
- The parameter form also holds the hidden field `value_type` with the `TypeExpr` of the shown grid (ADR-0039 (i)). A malformed `value_type` is `E-WEB-008` with `FieldReason::Reload`.
- On a `POST` route with `{key}`, `fields=1` or `limit_mode` runs `GetParameterForm`. Its `Missing::Parameter` then gives `404 E-RCP-051`, as the save does (`api.md` section 2.6).

### D7. Pages and fragments

A page route answers the three shapes of ADR-0037 (w). Every new form has `hx-boost="true"` and `hx-sync="closest form:drop"`.

| Route | Template | Answer |
|---|---|---|
| `GET /recipes` | `recipes.html` | search form (`method="get"`), table, pager, link New recipe for `RecipeCreate` |
| `GET /recipes/new` | `recipe-new.html` | form of D6. Machine select with an empty first option |
| `POST /recipes` | - | `303` to `/recipes/{ref}/versions/1` |
| `GET /recipes/{ref}` | `recipe.html` | `h1`, description list, rename form for `RecipeEdit`, version table with pager, link New version for `VersionCreate` |
| `POST /recipes/{ref}/rename` | `recipe-renamed.html` | new `#recipe-name`, toast, empty `#error-panel`. Without HTMX: `303` |
| `GET /recipes/{ref}/versions` | `versions-table.html` | the version table with pager. Without HTMX: the recipe page |
| `GET /recipes/{ref}/versions/new` | `version-new.html` | form of D6 |
| `POST /recipes/{ref}/versions` | - | `303` to the new draft |
| `GET .../versions/{n}` | `version.html` | header, outdated list with rebind forms (W5), unit links, `#parameter-table`, link Add a parameter, empty `#parameter-form` |
| `GET .../parameters` | `parameter-table.html` | the table of `ListParameters`. Without HTMX: the version page |
| `GET .../parameters/new` | `parameter-form.html` | `HX-Target: parameter-form`: the form. Otherwise the page of the form |
| `POST .../parameters` | `parameter-row.html` | the new row (`hx-swap="beforeend"` on the table body), an empty `#parameter-form`, the toast, an empty `#error-panel`. With `fields=1` or `limit_mode`: `200` with the form |
| `GET .../parameters/{key}` | `parameter.html` | definition, leaf grid, translation rows. On a draft with `ParameterEdit`: the edit form, the translation forms, the danger disclosure |
| `POST .../{key}/definition` | `parameter-summary.html` | the new `#parameter-summary`, the toast, an empty `#error-panel`. With `fields=1` or `limit_mode`: `200` with the form |
| `POST .../{key}/value` | `grid-values.html` | the read-only grid of the value |
| `GET` and `POST .../{key}/limits` | `grid-limits.html` | the read-only rows of the limits |
| `GET .../{key}/translations` | `translations.html` | the translation rows |
| `POST .../translations/{lang}` | `translation-row.html` | the new row, the toast, an empty `#error-panel` |
| `POST .../translations/{lang}/delete` | `translations.html` | the rows, the toast |
| `POST .../{key}/delete` | `parameter-table.html` | `HX-Target: content`: the content of the version page with `HX-Push-Url` of the version. Otherwise the table |
| `POST .../versions/{n}/rebind` | `parameter-table.html` | the table, the new outdated list out of band, the toast |

- Every success of a draft command sets `HX-Trigger: {"rowVersion": <new>}` (D10).
- A success answer of a form also swaps the form out of band with the saved texts and no error line (ADR-0041 (h)). This covers the edit form, the translation rows, and the rename forms.
- Each submit button of a form that an answer renders again has a fixed `id`, so HTMX keeps the focus (ADR-0041 (e)). After `AddParameter`, the link of the new row has `autofocus`.
- An answer with `fields=1`, `limit_mode`, or a mismatch of `value_type` keeps every sent text and the sent `row_version` (ADR-0039 (h)).
- A page shows a form or a link only when the role holds the permission of its command. The version must also be a live `Draft` (archived D6 of `web-plant-model`).

### D8. Leaf grid

- `pages/parameters/grid.rs` builds `GridView` from the `TypeTree`, the texts of D3, the limits, and the sent texts of an error answer.
  - The view is a flat list of rows in preorder (askama refuses recursion). A row is a leaf row, a row `[*]`, or a toggle row of a group.
  - Each row holds its `id`, its path, its class, its texts, its error lines, and the flags `open` and `limit_inputs`.
- The macros of `components/grid.html` write each row kind. The grid stands in `<div class="tree">` inside the edit form.
- The columns come in the order path, value, minimum, maximum, maximum length, and type (ADR-0039 (j)). At 320 px the frame scrolls sideways, and a focused input stays in view.
  - The grid stands inside a `.table-wrap` frame. The table has no fixed minimum width (wave 3).
  - The path column scrolls with the other columns at every width. A sticky path column hid focused inputs while the frame scrolled sideways (WCAG 2.4.11, task 16.2, findings D16-1 and D16-9).
  - Each input names its row header in `aria-labelledby`, so the accessible name keeps the path when the path cell is out of view.
- Hints stand above the grid, one per leaf class that the grid holds, each with a fixed `id`. Each input names its hint and its error line in `aria-describedby`.
  - The hint keys are `grid-hint-integer`, `grid-hint-number`, `grid-hint-duration`, `grid-hint-date`, `grid-hint-date-time`, `grid-hint-time-of-day`, and `grid-hint-string`.
  - A time hint gets an example with the prefix of the type, for example `T#1m30s`, `LT#1m30s`, or `S5T#1s`.
- A number input has no `inputmode`, because a decimal keypad can offer only a comma. A `String` input has `maxlength` of the type length. A `Bool` leaf is a select with `true` and `false`, labelled `grid-true` and `grid-false`.
  - A `Bool` leaf without a text gets an empty first option and `aria-required="true"`, without the attribute `required` (`design-system.md` section 10.5). Such a leaf belongs to a new parameter, or it is a new leaf after a type change. The user then picks a value, and the browser does not select `TRUE` alone (decision of wave 4).
- An array inside a group of an outer array forms no groups of its own, because a `tbody` cannot hold a `tbody` (wave 4). The user kept this rule in step 7. ADR-0045 records it (D22).
- The limit mode of each array level follows ADR-0039 (k).
  - One button stands in the row after the row `[*]` of the level. It has `name="limit_mode"`, the leaf path of the level as `value`, and the label `grid-mode-split` or `grid-mode-join` with the level.
  - A text `grid-mode-current-all` or `grid-mode-current-each` names the current mode.
  - An element row in the mode "one limit for all elements" shows `grid-limit-from-all` with the minimum, the maximum, and the path of the row `[*]`.
  - The inputs of a row `[*]` also name the cell "every element" in `aria-labelledby`.
- The groups follow ADR-0039 (l) and `design-system.md` section 10.8. The `id` of a group is `grid-group-<g>`. The toggle names the level, the index range, and the count of elements (`grid-group`). A toggle cell has the padding `--space-1`, so two toggles keep the gap `--target-gap`.
- The save button is the first submit button of the form (ADR-0039 (i)). It stands in the action row at the start of the form, before the definition fields. The button `parameter-show-fields` and the mode buttons come after it in the tree order. No CSS rule changes the visual order of these buttons (WCAG 1.3.2, 2.4.3).
- The class names are `.leaf-grid`, `.row-all`, `.grid-note`, `.grid-toggle`, `.field-error`, and `.field-error-code`. Their rules live in `components.css`, also a rule for `.leaf-grid tbody[hidden]`.

### D9. Error lists of a form

- `WebError::Fields { lines: NonEmpty<FieldError>, form: Option<RenderedForm> }` replaces the one-field answers of the forms of ADR-0041 (g).
  - `FieldError` holds the code, the message key and its arguments, and the target: a field `id` or no input.
  - `code()` returns the code of the first line.
  - `RenderedForm` holds the HTML of the form. Its `Debug` writes only the length, because the form holds the CSRF token.
- `error/lists.rs` builds the lines from `BadRequest`, `BadRow`, `RecipeErrors`, `DraftError`, and `E-RCP-020` items.
  - A leaf line takes its input `id` from a function `Fn(&LeafPath) -> Option<String>` that the caller passes. V2 builds that function from the `GridView` of the form, so `lists.rs` does not depend on the grid.
  - The parameter forms use `command_lines` in `pages/parameters/reply.rs` with a function `Fn(&RecipeError) -> Option<String>`. `GridView::input_of` picks the value input for `E-RCP-004`, `E-RCP-005`, and `E-RCP-028`. It picks the first limit input of the limit row for every other leaf fault (step-7 fix CR-5).
- `answer.rs` renders `field-errors.html`.
  - The element with `role="alert"` holds only the icon, the code, and the sentence `errors-count`.
  - The list after it shows at most 20 lines, then the line `errors-more` with the count of the others. Each line shows its code, its message, and a link to its input.
- For an HTMX request, `answer.rs` appends the pre-rendered form after the fragment in the body. The form element has its fixed `id` and the literal `hx-swap-oob="true"`. No template uses `|safe` for it.
- `draft_reply.rs` gives each form handler one function that renders the form with the sent texts, the sent `row_version`, and the lines, and returns `WebError::Fields`.
- The UDT form, the machine form, the unit form, and the rename forms of the plant pages use the same path (W7).

### D10. Row version and conflict

- `draft_reply::success(output, ...)` sets `HX-Trigger: {"rowVersion": <n>}` from `DraftOutput.row_version`. The value is an integer, so no user text enters the header.
- On `AppError::ConcurrentEdit`, the draft handler runs `GetVersion` with the user of the request. It returns `WebError::Conflict { error, view }`. `error` is the `ConcurrentEdit`, and `view` holds the server `row_version`, the login, and the time of `LastChange`.
  - When `GetVersion` fails, `view` holds that failure, and the answer is still `409 E-WEB-001` without the login and the time. The mapping of `WebError` to HTTP logs the failure once (`reliability.md` section 1.1).
- `conflict.html` shows the code, the sentence of `error-E-WEB-001`, the lines of `conflict-*`, the warning `conflict-reload-warning`, the link Reload version, and the `request_id`. It goes into `#error-panel` with the headers of `api.md` section 5.3. A conflict answer holds no out-of-band form, so the input stays.
- The time uses the format of `design-system.md` section 5.

### D11. Body limits per route

- One layer reads `MatchedPath` and selects the limit: `http.body_limit.value` for the four value routes, `http.body_limit.translation` for `POST .../translations/{lang}`, and `http.body_limit.form` for every other route.
- `DefaultBodyLimit` of the router takes the largest limit. The layer refuses a larger `Content-Length` and a longer body with `413 E-WEB-003`.
- The buffer of the CSRF check uses the same limit for the route. A body with `Transfer-Encoding: chunked` above the limit gets `413 E-WEB-003` from that buffer.
- The reader of the value fields counts the pairs and the segments of a leaf path (ADR-0039 (b)). It stops at the first excess and answers one line `E-WEB-008`.
- `WebSettings` gets `translation_langs: Vec<LangCode>` with the default `pl`.

### D12. Route areas and shared test support

- Wave 3 (W2) extends `table()` with `pages::recipes::rows()`, `pages::versions::rows()`, and `pages::parameters::rows()`. `recipes::rows()` takes the row `GET /recipes` from `routes.rs`. Later waves add rows.
- W2 also creates `route_tests.rs` in each of the three areas with `route_rows()` and `sweep_pages()`, and wires them into `routes/matrix.rs`, `routes/tests.rs`, and `pages/sweep_tests.rs`.
  - Each area agent adds the matrix rows and the sweep pages of its routes in its own wave. The test that compares the mutating rows of `table()` with `matrix()` then stays green after each wave.
- W2 owns `src/components_tests.rs` in wave 3 and V2 in wave 4. Each adds the `mod` line of its component tests.
- `routes/matrix.rs`: `fill()` replaces `{ref}` with `R-100`, `{n}` with `3`, `{key}` with the key of `LINE`, and `{lang}` with `pl`. `world()` also calls `seed_recipes()`.
- `test_support/recipes.rs` holds `Fixture::seed_recipes()` with fixed identifiers. It calls `seed_plant()` and adds:
  - the live machine `M20` (`TwinCat3`) with the units `U1` and `U2`,
  - in the draft version 3 of `R-100`: `SPEED` (`INT`, limit 0 to 3000), `LABEL` (`STRING(20)`), `HOLD` (`TIME`), `SPEED_TABLE` (`ARRAY[1..8] OF INT`, limit `[*]`), `GRID` (`ARRAY[0..1,1..2] OF REAL`), `LINE_A` (struct in a struct), and `ZONES` (`ARRAY[1..4] OF ST_Zone@2`),
  - the struct `ST_Zone@3` with one more member, so `ZONES` is outdated,
  - the recipe `R-200` of `M20` with one draft, and the deleted recipe `R-300`.
- W2 writes `Fixture::seed_large_draft()` in task 8.3: a draft with 4 000 parameters in 40 units. The shared fixture stays small. W3 uses it in task 9.4 and W4 in the test of `E-RCP-021`.

### D13. Texts

The Haiku task copies this table into `en.ftl` and `pl.ftl`. The English text follows ASD-STE100. The Polish text is the translation. "Draft" is "wersja robocza" in a sentence and "Szkic" as a state.

| Key | en | pl |
|---|---|---|
| `error-E-RCP-001` | Reference number { $reference } is already in use. | Numer referencyjny { $reference } jest już używany. |
| `error-E-RCP-002` | Version { $number } of recipe { $reference } is { $status }. Create a new draft to make changes. | Wersja { $number } receptury { $reference } ma stan { $status }. Utwórz nową wersję roboczą, aby wprowadzić zmiany. |
| `error-E-RCP-003` | Parameter { $tag } needs a PLC address. The machine uses an S7-300 controller. | Parametr { $tag } wymaga adresu PLC. Maszyna ma sterownik S7-300. |
| `error-E-RCP-004` | Value { $value } of { $tag }{ $leaf } is outside the process limit { $min } to { $max }. | Wartość { $value } parametru { $tag }{ $leaf } jest poza limitem procesowym od { $min } do { $max }. |
| `error-E-RCP-005` | Value of { $tag }{ $leaf } does not match the type. Expected { $expected }. | Wartość parametru { $tag }{ $leaf } nie pasuje do typu. Oczekiwano: { $expected }. |
| `error-E-RCP-006` | Process limit of { $tag }{ $leaf } is outside the range of the data type. | Limit procesowy parametru { $tag }{ $leaf } jest poza zakresem typu danych. |
| `error-E-RCP-007` | Version { $number } of recipe { $reference } is not a draft. | Wersja { $number } receptury { $reference } nie jest wersją roboczą. |
| `error-E-RCP-008` | Version { $number } of recipe { $reference } has no parameters. Add at least one parameter before approval. | Wersja { $number } receptury { $reference } nie ma parametrów. Przed zatwierdzeniem dodaj co najmniej jeden parametr. |
| `error-E-RCP-009` | Version { $number } of recipe { $reference } is not approved. | Wersja { $number } receptury { $reference } nie jest zatwierdzona. |
| `error-E-RCP-010` | Recipe { $reference } has an approved version. An Administrator can retire the version. | Receptura { $reference } ma zatwierdzoną wersję. Administrator może wycofać tę wersję. |
| `error-E-RCP-011` | Recipe { $reference } already has draft version { $number }. Edit or discard that draft. | Receptura { $reference } ma już wersję roboczą { $number }. Edytuj albo odrzuć tę wersję roboczą. |
| `error-E-RCP-012` | Recipe { $reference } is deleted. An administrator can restore it. | Receptura { $reference } jest usunięta. Administrator może ją przywrócić. |
| `error-E-RCP-013` | Parameter { $tag } uses type { $type_name }. This type does not exist on { $platform }. | Parametr { $tag } używa typu { $type_name }. Ten typ nie istnieje na platformie { $platform }. |
| `error-E-RCP-014` | Parameter tag { $tag } is already used in version { $number }. | Znacznik parametru { $tag } jest już używany w wersji { $number }. |
| `error-E-RCP-015` | User { $login } created or edited draft { $number } of recipe { $reference }. A different user must approve it. | Użytkownik { $login } utworzył lub edytował wersję roboczą { $number } receptury { $reference }. Zatwierdzić ją musi inny użytkownik. |
| `error-E-RCP-016` | { $tag }{ $leaf } has no process limit. Set a limit before approval. | { $tag }{ $leaf } nie ma limitu procesowego. Przed zatwierdzeniem ustaw limit. |
| `error-E-RCP-017` | Path { $leaf } does not exist in the type of { $tag }. | Ścieżka { $leaf } nie istnieje w typie parametru { $tag }. |
| `error-E-RCP-018` | The minimum { $min } of { $tag }{ $leaf } is greater than the maximum { $max }. | Minimum { $min } parametru { $tag }{ $leaf } jest większe niż maksimum { $max }. |
| `error-E-RCP-019` | { $tag }{ $leaf } does not accept this limit. A BOOL, CHAR, or WCHAR leaf has no limit. A string leaf has only a maximum length. | { $tag }{ $leaf } nie przyjmuje tego limitu. Pole BOOL, CHAR lub WCHAR nie ma limitu. Pole tekstowe ma tylko maksymalną długość. |
| `error-E-RCP-020` | { $count } parameters are not valid. See the list. | Liczba niepoprawnych parametrów: { $count }. Zobacz listę. |
| `error-E-RCP-021` | The version exceeds the limit of { $limit } { $budget }. Current count: { $actual }. | Wersja przekracza limit { $limit } { $budget }. Obecna liczba: { $actual }. |
| `error-E-RCP-022` | Address { $address } of { $tag } overlaps with { $other_tag }. | Adres { $address } parametru { $tag } nakłada się na { $other_tag }. |
| `error-E-RCP-023` | Address { $address } of { $tag } must start at an even byte. | Adres { $address } parametru { $tag } musi zaczynać się od parzystego bajtu. |
| `error-E-RCP-024` | { $tag } uses revision { $current } of { $type_name }. The newest revision is { $newest }. Rebind the parameter. | { $tag } używa rewizji { $current } typu { $type_name }. Najnowsza rewizja to { $newest }. Przepnij parametr. |
| `error-E-RCP-025` | Recipe { $reference } is not deleted. | Receptura { $reference } nie jest usunięta. |
| `error-E-RCP-026` | Recipe { $reference } has a retired version. Only an administrator can delete it. | Receptura { $reference } ma wycofaną wersję. Tylko administrator może ją usunąć. |
| `error-E-RCP-027` | { $lang } is not a language code. Use a code such as pl or de-AT. | { $lang } nie jest kodem języka. Użyj kodu takiego jak pl lub de-AT. |
| `error-E-RCP-028` | { $tag }{ $leaf } is a new member. Give it a value. | { $tag }{ $leaf } to nowy składnik. Podaj jego wartość. |
| `error-E-RCP-029` | { $tag }{ $leaf } has a limit but no longer exists in the new revision. Delete the limit first. | { $tag }{ $leaf } ma limit, ale nie istnieje w nowej rewizji. Najpierw usuń limit. |
| `error-E-RCP-030` | A parameter row cannot move to another version. | Wiersz parametru nie może przejść do innej wersji. |
| `error-E-RCP-031` | The version is not a live draft. | Wersja nie jest aktywną wersją roboczą. |
| `error-E-RCP-032` | The unit does not belong to the machine of the recipe. | Jednostka nie należy do maszyny receptury. |
| `error-E-RCP-033` | The version status or a frozen column of the version cannot change in this way. | Stan wersji ani zamrożona kolumna wersji nie mogą zmienić się w ten sposób. |
| `error-E-RCP-034` | The reference number or the machine of a recipe cannot change while a version uses it. | Numer referencyjny ani maszyna receptury nie mogą się zmienić, gdy używa ich wersja. |
| `error-E-RCP-035` | The version has more than 4 000 parameters. | Wersja ma więcej niż 4 000 parametrów. |
| `error-E-RCP-036` | The version has more than 32 768 limit rows. | Wersja ma więcej niż 32 768 wierszy limitów. |
| `error-E-RCP-050` | Parameter { $tag } in version { $number } of recipe { $reference } has no translation for { $lang }. | Parametr { $tag } w wersji { $number } receptury { $reference } nie ma tłumaczenia dla { $lang }. |
| `error-E-RCP-051` | Parameter { $key } does not exist in version { $number } of recipe { $reference }. | Parametr { $key } nie istnieje w wersji { $number } receptury { $reference }. |
| `error-E-RCP-052` | The rebind of { $type_name } is not possible: { $reason }. | Przepięcie { $type_name } nie jest możliwe: { $reason }. |
| `error-E-RCP-053` | Parameter { $tag } cannot have a PLC address on platform { $platform }. Delete the address. | Parametr { $tag } nie może mieć adresu PLC na platformie { $platform }. Usuń adres. |
| `error-E-RCP-054` | Address { $address } of { $tag } does not match the type of { $tag }. Use the width { $width }. | Adres { $address } parametru { $tag } nie pasuje do typu { $tag }. Użyj szerokości { $width }. |
| `error-E-PLT-019` | Machine { $code } does not exist. Select an existing machine. | Maszyna { $code } nie istnieje. Wybierz istniejącą maszynę. |
| `error-E-PLT-020` | Unit { $path } does not exist on this machine. Select an existing unit. | Jednostka { $path } nie istnieje na tej maszynie. Wybierz istniejącą jednostkę. |
| `error-E-PLT-030` | The PLC platform of a machine with recipes cannot change. | Platforma PLC maszyny z recepturami nie może się zmienić. |
| `error-E-PLT-031` | The type does not exist on the platform of the version. | Typ nie istnieje na platformie wersji. |
| `error-E-PLT-032` | The type belongs to another machine. | Typ należy do innej maszyny. |
| `error-E-PLT-033` | A type definition cannot change. Create a new revision. | Definicja typu nie może się zmienić. Utwórz nową rewizję. |
| `error-E-PLT-034` | The members of a type cannot change after the type is created. | Składniki typu nie mogą się zmienić po utworzeniu typu. |
| `error-E-PLT-035` | Only a struct type has members. | Tylko typ struktury ma składniki. |
| `error-E-PLT-036` | A struct needs at least one member. | Struktura musi mieć co najmniej jeden składnik. |
| `error-E-PLT-037` | The machine code cannot change while a version that is not a draft uses it. | Kod maszyny nie może się zmienić, gdy używa go wersja, która nie jest wersją roboczą. |
| `error-E-PLT-038` | The unit path or the machine of a unit cannot change while a version uses the unit. | Ścieżka jednostki ani maszyna jednostki nie mogą się zmienić, gdy wersja używa jednostki. |
| `expected-json` | valid JSON text | poprawny tekst JSON |
| `expected-depth` | a value with at most { $levels } levels | wartość o najwyżej { $levels } poziomach |
| `expected-object` | an object with the members { $members } | obiekt ze składnikami { $members } |
| `expected-array` | an array | tablica |
| `expected-array-count` | an array of { $count } elements | tablica z { $count } elementami |
| `expected-index` | an index inside the INTEGER range | indeks w zakresie INTEGER |
| `expected-scalar` | a scalar value | wartość skalarna |
| `expected-wstring` | a string of at most { $length } UTF-16 units | tekst o najwyżej { $length } jednostkach UTF-16 |
| `expected-string` | a string of at most { $length } characters from Latin-1 | tekst o najwyżej { $length } znakach z Latin-1 |
| `expected-char` | one character from U+0000 to { $max } | jeden znak od U+0000 do { $max } |
| `expected-literal` | an IEC literal of type { $type_name } | literał IEC typu { $type_name } |
| `expected-count` | a count of type { $type_name } | liczba jednostek typu { $type_name } |
| `expected-integer` | an integer of type { $type_name } | liczba całkowita typu { $type_name } |
| `expected-number-range` | a number of type { $type_name } from { $low } to { $high } | liczba typu { $type_name } od { $low } do { $high } |
| `expected-number-digits` | a number of type { $type_name } with at most { $digits } significant digits | liczba typu { $type_name } z najwyżej { $digits } cyframi znaczącymi |
| `expected-parameter-type` | a value of the type of the parameter | wartość typu parametru |
| `expected-new-value` | a new value for the new type | nowa wartość dla nowego typu |
| `expected-bool` | TRUE or FALSE | TRUE albo FALSE |
| `expected-decimal` | a decimal number with a dot, for example 1.5 | liczba dziesiętna z kropką, na przykład 1.5 |
| `expected-leaf` | a value for this field | wartość tego pola |
| `expected-known-leaf` | a path of the type of the parameter | ścieżka z typu parametru |
| `budget-parameters` | parameters | parametrów |
| `budget-leaves` | scalar leaves | wartości skalarnych |
| `budget-limit-rows` | limit rows | wierszy limitów |
| `budget-version-numbers` | version numbers | numerów wersji |
| `rebind-reason-source-not-struct` | the source type is not a struct | typ źródłowy nie jest strukturą |
| `rebind-reason-target-not-struct` | the target type is not a struct | typ docelowy nie jest strukturą |
| `rebind-reason-target-not-found` | the target type does not exist on the machine of the draft | typ docelowy nie istnieje na maszynie wersji roboczej |
| `rebind-reason-other-struct` | the target is a revision of another struct | cel jest rewizją innej struktury |
| `rebind-reason-not-newer` | the target revision is not higher than the source revision | rewizja docelowa nie jest wyższa niż rewizja źródłowa |
| `rebind-reason-not-used` | no parameter of the draft uses the source revision | żaden parametr wersji roboczej nie używa rewizji źródłowej |
| `rebind-reason-plan-mismatch` | the plan does not match the draft or the trees of the new types | plan nie pasuje do wersji roboczej ani do drzew nowych typów |
| `status-deleted` | Deleted | Usunięta |
| `status-outdated` | Outdated | Nieaktualny |
| `status-discarded` | Discarded | Odrzucona |
| `missing-malformed-recipe` | Recipe | Receptura |
| `missing-malformed-version` | Version | Wersja |
| `missing-malformed-parameter` | Parameter | Parametr |
| `missing-parameter` | Parameter { $key } of version { $number } of recipe { $reference } | Parametr { $key } wersji { $number } receptury { $reference } |
| `operation-list-deleted-recipes` | List deleted recipes | Lista usuniętych receptur |
| `reason-reference-number` | a reference number has 1 to 50 characters from A to Z, 0 to 9, underscore, and hyphen. It starts with a letter or a digit | numer referencyjny ma od 1 do 50 znaków z zakresu A do Z, 0 do 9, podkreślenie i łącznik. Zaczyna się literą lub cyfrą |
| `reason-change-note` | a change note has 1 to 4000 characters and no control character except a line end | opis zmiany ma od 1 do 4000 znaków i nie ma znaków sterujących poza końcem wiersza |
| `reason-description` | a description has 1 to 4000 characters and no control character except a line end | opis ma od 1 do 4000 znaków i nie ma znaków sterujących poza końcem wiersza |
| `reason-translation-description` | a description has 0 to 4000 characters and no control character except a line end | opis ma od 0 do 4000 znaków i nie ma znaków sterujących poza końcem wiersza |
| `reason-tag` | a tag has 1 to 64 characters from A to Z, 0 to 9, and underscore. It starts with a letter | znacznik ma od 1 do 64 znaków z zakresu A do Z, 0 do 9 i podkreślenie. Zaczyna się literą |
| `reason-ordinal` | the order is a whole number from 0 to 100000 | kolejność to liczba całkowita od 0 do 100000 |
| `reason-s7-address` | a PLC address has the form DB1.DBX0.0, DB1.DBB0, DB1.DBW0, or DB1.DBD0 | adres PLC ma postać DB1.DBX0.0, DB1.DBB0, DB1.DBW0 lub DB1.DBD0 |
| `reason-based-on` | choose a version from the list | wybierz wersję z listy |
| `reason-status` | choose a state from the list | wybierz stan z listy |
| `reason-value-key` | each value field names one path of the type once, with no [*] | każde pole wartości wskazuje jedną ścieżkę typu jeden raz, bez [*] |
| `reason-limit-row` | each limit row has a path and a minimum, a maximum, or a maximum length | każdy wiersz limitu ma ścieżkę oraz minimum, maksimum albo maksymalną długość |
| `reason-max-length` | the maximum length is a whole number from 1 to the length of the type, at most 16382 | maksymalna długość to liczba całkowita od 1 do długości typu, najwyżej 16382 |
| `reason-lang-list` | choose a language from the list | wybierz język z listy |
| `reason-reload` | reload the page and try again | odśwież stronę i spróbuj ponownie |
| `field-machine-code` | Machine | Maszyna |
| `field-reference-number` | Reference number | Numer referencyjny |
| `field-recipe-name` | Name | Nazwa |
| `field-change-note` | Change note | Opis zmiany |
| `field-recipe-q` | Search | Wyszukiwanie |
| `field-recipe-machine` | Machine | Maszyna |
| `field-recipe-status` | State | Stan |
| `field-show-deleted` | Show deleted recipes | Pokaż usunięte receptury |
| `field-based-on` | Base version | Wersja bazowa |
| `field-unit-path` | Unit | Jednostka |
| `field-tag` | Tag | Znacznik |
| `field-name-en` | English name | Nazwa angielska |
| `field-description-en` | English description | Opis angielski |
| `field-type` | Type | Typ |
| `field-s7-address` | PLC address | Adres PLC |
| `field-ordinal` | Order in the unit | Kolejność w jednostce |
| `field-row-version` | Change number | Numer zmiany |
| `field-value-type` | Type of the value fields | Typ pól wartości |
| `field-limit-mode` | Limit mode | Tryb limitów |
| `field-value-leaf` | Value { $leaf } | Wartość { $leaf } |
| `field-limit-min` | Minimum { $leaf } | Minimum { $leaf } |
| `field-limit-max` | Maximum { $leaf } | Maksimum { $leaf } |
| `field-limit-max-length` | Maximum length { $leaf } | Maksymalna długość { $leaf } |
| `field-translation-name` | Name ({ $lang }) | Nazwa ({ $lang }) |
| `field-translation-description` | Description ({ $lang }) | Opis ({ $lang }) |
| `field-new-member` | New member { $leaf } of { $tag } | Nowy składnik { $leaf } parametru { $tag } |
| `recipes-caption` | Recipe list | Lista receptur |
| `recipes-search-hint` | A part of the reference number or of the name. | Część numeru referencyjnego albo nazwy. |
| `recipes-any-machine` | Every machine | Każda maszyna |
| `recipes-any-status` | Every state | Każdy stan |
| `recipes-filter-submit` | Show | Pokaż |
| `recipes-new-link` | New recipe | Nowa receptura |
| `recipes-col-reference` | Reference number | Numer referencyjny |
| `recipes-col-name` | Name | Nazwa |
| `recipes-col-machine` | Machine | Maszyna |
| `recipes-col-approved` | Approved version | Wersja zatwierdzona |
| `recipes-col-draft` | Draft | Wersja robocza |
| `recipes-none` | none | brak |
| `recipe-new-title` | New recipe | Nowa receptura |
| `recipe-reference-hint` | 1 to 50 characters: A to Z, 0 to 9, underscore, and hyphen. The system changes small letters to capital letters. Example: R-100. | Od 1 do 50 znaków: A do Z, 0 do 9, podkreślenie i łącznik. System zmienia małe litery na wielkie. Przykład: R-100. |
| `recipe-change-note-hint` | Write the reason for this version. | Podaj powód tej wersji. |
| `recipe-create-submit` | Create recipe | Utwórz recepturę |
| `recipe-title` | Recipe { $reference } | Receptura { $reference } |
| `recipe-machine` | Machine | Maszyna |
| `recipe-name` | Name | Nazwa |
| `recipe-rename-submit` | Rename recipe | Zmień nazwę receptury |
| `recipe-renamed` | Recipe { $reference } has the new name. | Receptura { $reference } ma nową nazwę. |
| `versions-caption` | Versions | Wersje |
| `versions-col-number` | Version | Wersja |
| `versions-col-state` | State | Stan |
| `versions-col-change-note` | Change note | Opis zmiany |
| `versions-col-created` | Created at | Utworzono |
| `versions-col-approved` | Approved at | Zatwierdzono |
| `version-new-link` | New version | Nowa wersja |
| `version-new-title` | New version of recipe { $reference } | Nowa wersja receptury { $reference } |
| `version-empty-draft` | Empty draft | Pusta wersja robocza |
| `version-based-on-hint` | The new draft copies the parameters of the base version. | Nowa wersja robocza kopiuje parametry wersji bazowej. |
| `version-create-submit` | Create version | Utwórz wersję |
| `version-title` | Version { $number } of recipe { $reference } | Wersja { $number } receptury { $reference } |
| `version-state` | State | Stan |
| `version-change-note` | Change note | Opis zmiany |
| `version-created-by` | Author | Autor |
| `version-created-at` | Created at | Utworzono |
| `version-approved-by` | Approver | Zatwierdzający |
| `version-approved-at` | Approved at | Zatwierdzono |
| `version-updated-at` | Last change | Ostatnia zmiana |
| `version-platform` | PLC platform | Platforma PLC |
| `parameters-caption` | Parameters | Parametry |
| `parameters-col-tag` | Tag | Znacznik |
| `parameters-col-name` | Name | Nazwa |
| `parameters-col-type` | Type | Typ |
| `parameters-col-unit` | Unit | Jednostka |
| `parameters-col-address` | PLC address | Adres PLC |
| `parameters-col-state` | State | Stan |
| `parameters-empty` | The version has no parameters. | Wersja nie ma parametrów. |
| `parameters-units-label` | Units of the version | Jednostki wersji |
| `parameters-unit-link` | { $path } (parameters: { $count }) | { $path } (parametry: { $count }) |
| `parameter-add-link` | Add a parameter | Dodaj parametr |
| `parameter-new-title` | New parameter | Nowy parametr |
| `parameter-tag-hint` | Capital letters, digits, and underscores. Start with a letter. At most 64 characters. | Wielkie litery, cyfry i podkreślenia. Na początku litera. Najwyżej 64 znaki. |
| `parameter-s7-hint` | The width follows the type: X for BOOL, B for 1 byte, W for 2 bytes, and D for 4 bytes. A string, a struct, and an array use B. Example: DB10.DBW0. | Szerokość zależy od typu: X dla BOOL, B dla 1 bajtu, W dla 2 bajtów i D dla 4 bajtów. Tekst, struktura i tablica mają B. Przykład: DB10.DBW0. |
| `parameter-ordinal-hint` | The page shows the parameters of a unit in this order. | Strona pokazuje parametry jednostki w tej kolejności. |
| `parameter-show-fields` | Show value fields | Pokaż pola wartości |
| `parameter-fields-first` | Enter the type and then show the value fields. | Wpisz typ, a potem pokaż pola wartości. |
| `value-type-notice` | The type changed. Enter the values of the new type. | Typ się zmienił. Wpisz wartości nowego typu. |
| `parameter-add-submit` | Add parameter | Dodaj parametr |
| `parameter-added` | Parameter { $tag } is added. | Parametr { $tag } został dodany. |
| `parameter-title` | Parameter { $tag } | Parametr { $tag } |
| `parameter-key` | Key | Klucz |
| `parameter-save-submit` | Save parameter | Zapisz parametr |
| `parameter-saved` | Parameter { $tag } is saved. | Parametr { $tag } został zapisany. |
| `parameter-delete-summary` | Delete the parameter | Usuń parametr |
| `parameter-delete-warning` | The system deletes the parameter with its value, its limits, and its translations from this draft. | System usunie parametr z wartością, limitami i tłumaczeniami z tej wersji roboczej. |
| `parameter-delete-submit` | Delete parameter { $tag } | Usuń parametr { $tag } |
| `parameter-deleted` | Parameter { $tag } is deleted. | Parametr { $tag } został usunięty. |
| `grid-caption` | Values and process limits | Wartości i limity procesowe |
| `grid-col-path` | Path | Ścieżka |
| `grid-col-type` | Type | Typ |
| `grid-col-value` | Value | Wartość |
| `grid-col-min` | Minimum | Minimum |
| `grid-col-max` | Maximum | Maksimum |
| `grid-col-max-length` | Maximum length | Maks. długość |
| `grid-all-elements` | every element { $range } | wszystkie elementy { $range } |
| `grid-limit-from-all` | { $min } to { $max } (from { $path }) | { $min } do { $max } (z { $path }) |
| `grid-length-from-all` | at most { $length } characters (from { $path }) | najwyżej { $length } znaków (z { $path }) |
| `grid-limit-none-from-all` | no limit yet (from { $path }) | jeszcze bez limitu (z { $path }) |
| `grid-not-applicable` | not applicable | nie dotyczy |
| `grid-group` | Elements { $range } of { $level } | Elementy { $range } z { $level } |
| `grid-mode-split` | Set a limit per element of { $level } | Ustaw limit dla każdego elementu { $level } |
| `grid-mode-join` | Set one limit for all elements of { $level } | Ustaw jeden limit dla wszystkich elementów { $level } |
| `grid-mode-current-all` | { $level }: one limit for all elements | { $level }: jeden limit dla wszystkich elementów |
| `grid-mode-current-each` | { $level }: a limit per element | { $level }: limit dla każdego elementu |
| `grid-mode-join-notice` | The elements of { $level } have different limits. Enter one limit for all elements. | Elementy { $level } mają różne limity. Wpisz jeden limit dla wszystkich elementów. |
| `grid-hint-integer` | A whole number, for example 1200. | Liczba całkowita, na przykład 1200. |
| `grid-hint-number` | A decimal number with a dot, for example 1.5. | Liczba dziesiętna z kropką, na przykład 1.5. |
| `grid-hint-duration` | A duration as an IEC literal, for example { $example }. | Czas trwania jako literał IEC, na przykład { $example }. |
| `grid-hint-date` | A date as an IEC literal, for example { $example }. | Data jako literał IEC, na przykład { $example }. |
| `grid-hint-date-time` | A date and a time as an IEC literal, for example { $example }. | Data i godzina jako literał IEC, na przykład { $example }. |
| `grid-hint-time-of-day` | A time of day as an IEC literal, for example { $example }. | Godzina jako literał IEC, na przykład { $example }. |
| `grid-hint-string` | Text of at most { $length } characters. | Tekst o najwyżej { $length } znakach. |
| `grid-true` | TRUE | TRUE |
| `grid-false` | FALSE | FALSE |
| `grid-limits-empty` | No process limits. | Brak limitów procesowych. |
| `recipe-machine-choose` | Choose a machine | Wybierz maszynę |
| `outdated-col-struct` | Outdated struct | Nieaktualna struktura |
| `outdated-nested-hint` | { $tag } uses { $type } inside the struct { $holder }. Create a new revision of { $holder } that uses { $type }@{ $newest }. Then rebind { $tag }. | { $tag } używa { $type } wewnątrz struktury { $holder }. Utwórz nową rewizję { $holder }, która używa { $type }@{ $newest }. Potem przepnij { $tag }. |
| `error-E-RCP-029` (changed in step 7) | { $tag }{ $leaf } has a limit, but the new revision has no such member or gives the member another type. Delete the limit first. After the rebind, set the limit again. | { $tag }{ $leaf } ma limit, ale nowa rewizja nie ma tego elementu albo nadaje mu inny typ. Najpierw usuń limit. Po przepięciu ustaw limit ponownie. |
| `error-E-RCP-054` (pl changed in step 7) | unchanged | Adres { $address } parametru { $tag } nie pasuje do typu parametru { $tag }. Użyj szerokości { $width }. |
| `rebind-too-large` | The rebinds of this draft need at least { $count } new values. One page shows at most { $max } new values. | Przepięcia tej wersji roboczej wymagają co najmniej { $count } nowych wartości. Jedna strona pokazuje najwyżej { $max } nowych wartości. |
| `translations-caption` | Translations | Tłumaczenia |
| `translations-col-lang` | Language | Język |
| `translations-col-base` | English base text | Tekst bazowy po angielsku |
| `translation-none` | No translation | Brak tłumaczenia |
| `translation-save-submit` | Save translation { $lang } | Zapisz tłumaczenie { $lang } |
| `translation-saved` | The translation { $lang } is saved. | Tłumaczenie { $lang } zostało zapisane. |
| `translation-delete-summary` | Delete the translation { $lang } | Usuń tłumaczenie { $lang } |
| `translation-delete-warning` | The system deletes the name and the description in this language. | System usunie nazwę i opis w tym języku. |
| `translation-delete-submit` | Delete translation { $lang } | Usuń tłumaczenie { $lang } |
| `translation-deleted` | The translation { $lang } is deleted. | Tłumaczenie { $lang } zostało usunięte. |
| `outdated-caption` | Outdated parameters | Nieaktualne parametry |
| `outdated-hint` | A newer revision of a struct exists. Rebind the parameters to the newest revision. | Istnieje nowsza rewizja struktury. Przepnij parametry na najnowszą rewizję. |
| `rebind-title` | Rebind { $from } to { $to } | Przepięcie { $from } na { $to } |
| `rebind-new-members` | Values of the new members | Wartości nowych składników |
| `rebind-no-new-members` | The new revision adds no member. | Nowa rewizja nie dodaje składników. |
| `rebind-submit` | Rebind parameters | Przepnij parametry |
| `rebind-done` | The parameters use { $to }. | Parametry używają { $to }. |
| `conflict-row-version` | Current change number: { $row_version } | Obecny numer zmiany: { $row_version } |
| `conflict-last-change` | Last change: { $login }, { $time } | Ostatnia zmiana: { $login }, { $time } |
| `conflict-no-editor` | The system cannot show the last change. | System nie może pokazać ostatniej zmiany. |
| `conflict-reload-warning` | Your changes are not saved. The reload discards them. | Zmiany nie są zapisane. Odświeżenie je usunie. |
| `conflict-reload` | Reload version | Odśwież wersję |
| `errors-count` | Errors: { $count }. Change the marked fields. | Liczba błędów: { $count }. Popraw oznaczone pola. |
| `errors-more` | Other errors: { $count }. Each one stands under its field. | Pozostałe błędy: { $count }. Każdy stoi pod swoim polem. |

- A message argument that names a state, a budget, a platform, a rebind reason, or an expected form gets the text of its key. It never gets the English `Display` (`error/recipe.rs`).
- `{ $leaf }` follows `{ $tag }` with no space, as in `reliability.md` section 1.2, for example `SPEED_TABLE[3]`.
- A message of the bundles leaves out the sentence "Reference: { request_id }" of the registry. The fragment shows the `request_id` on its own line, as for `error-E-WEB-010`. A test compares each `E-RCP` and `E-PLT` message of `en` with its registry row without that sentence.
- An agent that needs a key outside this table reports it. The orchestrator adds the key to both bundles and to this table.

### D14. XSS and security

Each check has one owner:

- The area agents test each error code of their routes in its fragment with `HX-Retarget` and `HX-Push-Url: false`, and the matrix rows of D12.
- H2 tests each new route and each principal of `api.md` section 3.3 with the code in the body and the `forbidden` security event.
- W6 tests the XSS corpus, the `hx-*` rule, the CSRF token, and read-only mode.

The checks of W6:

- The corpus of `tests/web/xss-corpus.txt` goes into these fields:
  - `reference_number`, `name`, `change_note`, `tag`, `name_en`, `description_en`, `type`, `s7_address`, and `ordinal`,
  - each value field, `limits[i].min`, `limits[i].max`, and `limits[i].max_length`,
  - the translation `name` and `description`, `based_on`, `q`, `machine`, `status`, `page`, `unit`, and `limit_mode`,
  - `machine_code`, `unit_path`, `limits[i].leaf_path`, `new_member_values`, `from_type_id`, `to_type_id`, `row_version`, `value_type`, `fields`, and `show_deleted`.
- The payload also goes into the name of a value field, for example `value.<payload>`. The test reads the answer and the out-of-band form.
- It also goes into the path values `{ref}`, `{n}`, `{key}`, and `{lang}`, and into the field `next` of the language switch on each new page.
- The test asserts the escaped text in `value` for a field that the server echoes into a form. An example is the sent texts of an error answer.
- For a value that the server refuses, the test asserts the error code. The answer holds no raw payload, also not in the out-of-band form.
- A test reads every new page and fragment. Each `hx-*` value of an element, also one that it gets from an ancestor, is one of a fixed list of literals. `hx-headers` of `<body>` is the one exception.
- A test reads every URL of every answer: each `href`, `action`, `Location`, `HX-Push-Url`, and the field `next`. None of them holds `csrf_token`.
- A test sends each new `POST` route without a CSRF token and gets `403 E-WEB-007`. In read-only mode each new `POST` route gets `503 E-WEB-006`. Each `GET` route works in read-only mode.
- The sweep gets each new `GET` page in both languages and checks the CSP, the inline script, the style, and the handlers.

Queries:

- `source_tests.rs` reads each file of `recipe/queries/`. Each `execute` body starts with `authorize(Self::OPERATION`.
- The first-statement test of `auth/first_statement.rs` covers the form queries and `ListDeletedRecipes`.

### D15. Development server and the seed script

- `examples/dev_server.rs` does not change. It seeds the primitive types and no machine.
- The orchestrator writes `scratchpad/seed_recipes.py` for the review and the QA. It logs in through HTTP with the pre-session token and sends `POST` requests with `csrf_token`.
- The script creates the machines `M2` (`S7_300`) and `M20` (`TwinCat3`) and their units. It also creates the structs `ST_Zone@1`, `ST_Zone@2`, and `ST_Line@1`, and the recipes `R-100` and `R-200` with the parameters of D12.

### D16. Test plan of the required states

| State | Test |
|---|---|
| Every primitive class in the grid: `INT`, `REAL`, `BOOL`, `CHAR`, `STRING(20)`, `WSTRING(10)`, `TIME`, `DATE`, `DATE_AND_TIME` | one grid test per class: input kind, hint, `maxlength`, the canonical text |
| `ARRAY[1..8] OF INT` with `[*]` | eight leaf rows and one row `[*]` with limit inputs |
| `ARRAY[0..1,1..2] OF REAL` | rows `[0,1]`, `[0,2]`, `[1,1]`, `[1,2]` |
| Struct in a struct, array of structs | the paths `.zone.setpoint` and `[1].setpoint` to `[4].setpoint`. The limit modes of ADR-0039 (k) |
| Array of 1 000 elements | ten groups, the first open, `aria-controls` on each toggle |
| Round trip of the texts | domain: each class and `proptest` on random trees up to depth 6 |
| Partial value | `E-RCP-005` with a line for the missing leaf |
| Two faults at once | domain list of two entries. Web: two summary lines and two lines under the inputs |
| S7 address on `S7_300` and none on `TwinCat3` | form tests on `M7` and `M20` |
| 409 with two sessions | W4: two `TestSession` values on one draft. The second save gets `conflict.html` with the login and the time of the first, and no out-of-band form. Also for `AddParameter`, a translation, the removal, and the rebind (W5) |
| Form rendered again after another save | W4: `fields=1`, `limit_mode`, and a `value_type` mismatch keep the sent `row_version`. The next save gets `409 E-WEB-001` |
| Limit modes | V2: the switch to a limit per element copies the `[*]` limit. The switch back with unequal limits leaves `[*]` empty with `grid-mode-join-notice` |
| Clean form after a save | W4 and W7: the success answer swaps the form with the saved texts and no error line |
| Deleted recipe on a parameter route | W4: a ProcessEngineer gets `409 E-RCP-012` on each `GET` and `POST` route of a parameter of `R-300`. An Operator gets `409 E-RCP-012` on each data `GET` and `403 E-WEB-002` on the form route and on each `POST`, because the permission check comes first (ADR-0013, wave 5) |
| Flood of pairs, long key | V2: 100 000 extra pairs and a key of 1 MiB give one line `E-WEB-008` |
| Chunked body | W2: a chunked body above `http.body_limit.value` gives `413 E-WEB-003` |
| Enter saves | test in a browser: Enter in a value input sends neither `fields` nor `limit_mode` |
| Success header | each draft command sets `HX-Trigger` with the new `row_version` |
| Each `POST` error code of `api.md` sections 2.4 to 2.6 in scope | one HTMX test per code with `HX-Retarget` and `HX-Push-Url: false` |
| `E-RCP-020` list | one line per item |
| Each code of a new route has a message in `pl` and `en` | `i18n/recipe_texts_tests.rs`: each code of D13 in both bundles. The key test reads templates and `.rs` files |
| Each page in `pl` and `en` | the sweep of D14 |
| 4 000 parameters | A2: `ListParameters` reads a fixed number of statements for 40 and for 4 000 parameters. W3: the page shows at most 100 rows and stays under a fixed byte bound |
| Budget | `E-RCP-021` at the parameter budget through the web route |
| Read-only mode | each new command gets `503 E-WEB-006` |
| Width of 320 CSS px, keyboard path through the grid, focus after a swap | the test in a browser (task 16.2), in `pl`, on the grid of `ZONES` and `LINE_A` |

### D17. Known changes of test expectations

The implementer reports each change with this reason. Any other change of an expectation needs its own reason.

| Test | Old expectation | New expectation | Reason |
|---|---|---|---|
| `routes::tests::expected_routes` and the matrix | the routes of P3.2 | also the rows of the three recipe areas | D12 |
| The count of the mutating routes in the coverage test | the count of P3.2 | the count of the mutating rows of the matrix | D12 |
| Tests of `ValueShapeMismatch` that compare `expected` with a text | a `String` | an `Expected` variant | D3 |
| Tests of a draft command with two faults | the first fault | a `RecipeErrors` list | D3, ADR-0041 |
| `recipe/queries` tests of the stub `ListRecipes` | a unit result | `RecipeList` with rows | D4 |
| `error::reason` tests of `FieldReason::ALL` | 20 variants | 34 variants | D6 |
| Tests of a field error answer of a plant form | one line | every line and the out-of-band form | D9, ADR-0041 |
| Tests of the body limit | 256 KiB on every route | the limit of the route | D11 |
| The tests of the recipe list page of P3.1 | the empty state only | the table, the pager, and the search form | D7 |
| `ListRecipes` in the sweep | an empty list | the rows of `seed_recipes` | D12 |
| Tests that build a command input with `value_json` or `new_member_values` | a `String` or a `serde_json::Value` | `ValueInput::Json` or `MemberValue::Json` with the same text | D3, D4 |
| Tests with a literal of `PageSizes` | two fields | five fields | D4 |
| `error::value_tests::every_missing` | the variants of P3.2 | also `MalformedRecipe`, `MalformedVersion`, `MalformedParameter`, and `Parameter` | D6 |
| `error::tests::error_mapping_code_without_a_bundle_key_falls_back_to_english` | the case `trigger` with `E-PLT-030` | the new case `trigger-audit` with `E-AUD-003`, because D13 gives `E-PLT-030` a key (orchestrator, wave 1) | D13 |
| `settings::tests::default_body_limits_equal_the_table` | 1 MiB for `http.body_limit.value` | 4 MiB (orchestrator, wave 1) | ADR-0039 (g) |
| `pages::xss_tests::xss_the_query_pages_are_every_get_page_of_the_table_without_a_query_extractor` | `/recipes` in the list | `/recipes/new` and `/recipes/{ref}/versions/new` in the list, without `/recipes`, which now reads its filters (W3, wave 4) | D7 |
| `pages::versions::page_tests::versions_page_of_a_process_engineer_on_the_draft_holds_the_add_link` | an empty `<div id="outdated-list"></div>` | `<div id="outdated-list">` with the list of the draft (W5, wave 5) | D7, ADR-0043 (l) |
| `pages::types::route_tests::route_rows`, the row of a new `ST_Zone` revision | a redirect to `?rev=3` | a redirect to `?rev=4`, because the matrix world seeds `ST_Zone@3` (W2, wave 3) | D12 |
| `pages::parameters::add_error_tests::parameters_add_errors_of_the_value_and_the_limits_stand_under_the_leaf` | the lines of `E-RCP-006` and `E-RCP-018` under `value-0` | the lines under `limits-0-min`, and the summary link and `aria-invalid` on the same input (F1, step 7) | ADR-0039 (n), ADR-0041 (e) |
| `pages::parameters::add_error_tests::parameters_add_a_limit_on_a_bool_leaf_gives_e_rcp_019` | the input `value-0` | no input, because a BOOL row has no limit input. The line stands only in the summary (F1, step 7) | ADR-0041 (e) |
| `pages::parameters::grid_tests::grid_the_input_of_a_leaf_path_is_its_value_input_or_its_limit_input` | `input_id(path)` | `input_id(path, LeafInput)`. `[*]` with `Value` gives no input, `[3]` with `Limit` gives `limits-0-min` (F1, step 7) | ADR-0039 (n) |
| `pages::parameters::grid_tests::grid_an_array_of_1000_elements...`, `components_tests::grid::grid_component_a_group_has_a_toggle_body_and_a_closed_body` | the toggle label without a count | the label with " (elements: 100)" (F1, step 7) | design-system.md 10.8 |
| `recipe/queries/outdated/tests.rs`, the helper `leaves_of_get_version` of the two rebind-leaf tests | the Operator gets the new member leaves | a process engineer gets them, the Operator gets `None` (F5, step 7) | ADR-0044 (a), D21 |
| Tests that build `GetVersionInput` (`version_tests.rs`, `test_support/recipes/tests.rs`) | two fields | three fields with `rebind_bound` (F5, step 7) | D21 |
| `pages::machines::form_tests::machines_create_with_the_code_of_a_live_machine_gives_409_in_the_error_panel`, now `..._puts_e_plt_001_under_the_code` | the fragment holds no form | the out-of-band form with the sent texts and one line `E-PLT-001` under `code` (F3, step 7) | ADR-0041 (e) |
| `pages::units::add_tests::units_add_lists_every_bad_field_and_swaps_the_form_with_the_sent_texts` | "Errors: 2." with lines under `parent_path` and `unit_name` | "Errors: 3." with a line under `segment` too (F3, step 7) | ADR-0041 (a) |
| `pages::recipes::form_tests::recipes_create_with_an_unknown_machine_gives_e_plt_019` | no `hx-swap-oob` in the answer | the out-of-band form with one line `E-PLT-019` under `machine_code` (F3, step 7) | ADR-0041 (e) |
| `pages::recipes::list_tests::recipes_list_filter_of_a_machine_without_a_live_row_keeps_its_code_selected`, now `recipes_list_filter_of_a_code_without_a_live_machine_gives_e_web_005` | 200 with `M99` as an extra selected option | `404 E-WEB-005` and no `M99` option (F4, step 7) | ADR-0043 (b), api.md section 1 |
| `pages::parameters::new_tests::parameters_new_form_without_htmx_is_a_page_with_a_plain_form`, now `..._with_a_stand_alone_form` | the plain form tag without `hx-*` attributes | the tag with `hx-boost="true" hx-target="#content" hx-push-url="true" hx-sync="closest form:drop"`, and no `#parameter-rows` in `main` (F7, step 7) | D7, `.claude/rules/web.md` |
| `style_tests::recipe_rules::recipe_rules_each_expected_declaration_has_the_value_of_the_design`, entry `.leaf-grid th[scope="row"]` `overflow-wrap` | `anywhere` | `break-word` (F6, step 7) | design-system.md 5 |
| Seven tests that expect the fault text of a bad integer leaf (`draft/add/content.rs` two, `draft/definition/tests.rs` two, `draft/value/tests.rs`, `value/texts_tests.rs`, `rebind/value/tests.rs` now `..._expects_an_integer`) | `Expected::Decimal`, "a decimal number with a dot" | `Expected::Integer { type_name }` (M1, step 7) | DR-4, D3 |
| `error::fragment_tests::fragment_error_is_one_alert_with_icon_code_sentence_and_reference` | the body ends with `</span></div>` | the same check after the trailing empty `#toast` element (M2, step 7) | ADR-0047 (b) |
| `error::summary_tests::summary_an_answer_without_a_form_holds_no_form` | no `hx-swap-oob` | no `<form`, and one `hx-swap-oob`: the empty `#toast` at the end (M2, step 7) | ADR-0047 (b) |
| `pages::draft_reply::tests::draft_reply_an_htmx_error_answer_swaps_the_form_out_of_band_after_the_summary` | ends with `</form>`, one `hx-swap-oob` | ends with the empty `#toast` element, two `hx-swap-oob` (M2, step 7) | ADR-0047 (b) |
| `pages::draft_reply::conflict_tests`, two tests of the conflict answer | no `hx-swap-oob`, one ` hx-` | one `hx-swap-oob` (the empty toast), the ` hx-` count of the body without the toast element (M2, step 7) | D10, ADR-0047 (b) |
| `pages::versions::rebind_tests::rebind_with_an_old_row_version_from_a_second_session_gives_the_conflict` | no `hx-swap-oob` | no `<form`, one `hx-swap-oob`, and the body ends with the empty `#toast` element (orchestrator for M2, step 7) | D10, ADR-0047 (b) |
| `plant::queries::units_tests::an_operator_gets_forbidden_from_the_form_and_the_store_stays_unread` | `Forbidden { GetUnitForm }` | `Forbidden { AddUnit }` (M3, step 7) | ADR-0046 (b) |
| `auth::first_statement`, the six tests `get_{machine,unit,udt,recipe,version,parameter}_form_checks_the_permission_first` | the operation of the form query | replaced by `auth::purpose_tests`, one check per purpose that expects the command of the purpose (M3, step 7) | ADR-0046 (b) |
| Three tests of `recipe-domain` `rebind/limit/tests.rs` that compare the message of `E-RCP-029` | "has a limit but no longer exists in the new revision" | the new registry row of `reliability.md` 1.2 (M4, step 7) | QA-3 |
| `recipe/queries/source_tests.rs`, `plant/queries/source_tests.rs`, the check of the first statement | the raw start of the body | the start of the body without white space, because rustfmt splits the arguments of `authorize` (orchestrator, step 7) | ADR-0013, ADR-0046 (b) |
| Tests in `recipe-app` and `recipe-web` that build the input of a form query (`UnitFormInput`, `UdtFormInput`, `GetVersionFormInput`, `GetParameterFormInput`, new `MachineFormInput`, `RecipeFormInput`) | no purpose | the field `purpose` (M3, step 7) | ADR-0046 (b) |
| Tests that build `OutdatedParameter` | no holder | the field `holder` (M4, step 7) | D20, K7 |
| `pages::parameters::value_tests::parameters_limits_reads_the_rows_through_get_limits` | split of the source on `"\n}\n"` | the lines up to the closing brace, because a CRLF checkout breaks the split (orchestrator, step 7, RR1-3) | ADR-0046 (d) |
| `style_tests::recipe_rules::recipe_rules_the_error_line_shrinks_and_wraps_inside_a_narrow_cell`, `recipe_rules_the_error_code_never_breaks_and_the_sentence_still_wraps` | `.field-error` `overflow-wrap: anywhere` | `break-word`, and the message wraps below the code (G3, step 7, RQ-4) | design-system.md 5 |
| The form-tag constants of `parameters/add_tests.rs`, `new_tests.rs`, `edit_tests.rs`, `page_tests.rs` | `hx-swap="beforeend"` and `hx-swap="innerHTML"` | the same values with ` show:none focus-scroll:true` (G1, step 7, RQ-2) | D16 |
| `pages::recipe_sweep_tests`, the `hx-swap` literal list | `innerHTML`, `outerHTML`, `beforeend` | `innerHTML`, `outerHTML`, and the two values with ` show:none focus-scroll:true`. No template uses plain `beforeend` any more (G1, step 7) | ADR-0037 (f) |
| `style_tests::recipe_rules::recipe_rules_each_expected_declaration_has_the_value_of_the_design`, entries `.leaf-grid th[scope="row"]` `position` and `left` | `sticky` and `0` | no entries. The new test `recipe_rules_no_rule_of_components_css_is_sticky_so_no_cell_can_hide_a_focused_input` replaces `recipe_rules_only_the_row_header_of_the_grid_is_sticky_and_the_header_row_is_not` (G4, task 16.2, D16-1, D16-9) | D8, WCAG 2.4.11 |
| `pages::types::form_tests`, the test of a CSRF token in the query of the types form | the token text `abc` | the token text `tok-zz-qq`, because a random request id can hold `abc` and the old test failed at random (W2 fix round, wave 3) | D14 |

### D18. Waves

At most two agents that run cargo work at the same time. A shared file has one owner in each wave. A wave starts after the green CI of the wave before it. After a wave with `[hard]` tasks, it also waits for a verifier report without CRITICAL and HIGH.

Classes: T1-F `fable` xhigh, T1-C `opus` high, T1-O `opus` max, T2 `opus` xhigh, T3 `sonnet` high, T4 `haiku` medium.

- T1-F does only critical reviews, hard planning, and debugging after five failed attempts of other models. It writes no code (rule of the user, set in wave 2).
- From wave 5 on, no agent uses `fable` (rule of the user). Each wave verifier is `opus` xhigh. The code reviews of step 7 are T1-O (`opus` max). Debugging after five failed attempts goes to T1-O.
- T1-C writes the code of the `[hard]` tasks from task 3.5 on. D1, A2, and V1 for tasks 3.1 to 3.4 ran as T1-F high before that rule.

| Wave | Agent | Class | Model | Effort | Tasks | File set |
|---|---|---|---|---|---|---|
| 1, first | H1 | T4 | haiku | medium | 2.1 | `recipe-web`: `i18n/en.ftl`, `i18n/pl.ftl`, `src/i18n/recipe_texts_tests.rs`, the `mod` line in `src/i18n.rs` |
| 1, after H1 | A1 | T3 | sonnet | high | 4.1 to 4.4 | Ports of `recipe-app`: `ports/recipe_reads.rs`, `ports/mod.rs`, `ports/isolation.rs`, `ports/unit_of_work.rs`. Memory: `memory/recipe/` (also `reads.rs`), `memory/mod.rs`, `memory/core/transaction.rs`, `plant/short_rows.rs`. Queries: `recipe/mod.rs`, `recipe/queries/` without the three files of A2, `error/missing.rs`, `auth/first_statement.rs`, `lib.rs`, `blocking.rs`. The accessor only: `export/tests.rs`. The literals of `PageSizes` only: `blocking/tests.rs`, `plant/queries/machines_tests.rs`, `plant/queries/usage_tests.rs`. In `recipe-web`, the call of `ListRecipes` only: `src/pages/recipes.rs`, `src/pages/recipes/tests.rs`. In `recipe-web`, the new `Missing` variants only: `src/error/values.rs`, `src/error/value_tests.rs` |
| 1, after task 4.1 | D1 | T1-F | fable | high | 5.1 to 5.4 `[hard]` | `recipe-db`: `src/unit_of_work.rs`, `src/transaction.rs`, `src/transaction/recipe_list.rs`, `version_reads.rs`, `parameter_reads.rs`, `tests/storage/contract/mod.rs`, `tests/storage/contract/recipe_reads/`, `tests/storage/unit_of_work.rs`, `tests/grants.expected` (read only) |
| 2 | V1 | T1-F, T1-C for 3.5 | fable, opus for 3.5 | high | 3.1 to 3.5 `[hard]` | `recipe-domain`: `src/recipe/value/`, `src/recipe/error.rs`, `src/recipe/draft/`, `src/recipe/rebind/`, `src/recipe/mod.rs`, `src/lib.rs`. `recipe-app`: `recipe/add_parameter.rs`, `add_parameter_errors.rs`, `update_definition.rs`, `update_definition/`, `definition_steps.rs`, `set_value.rs`, `replace_limits.rs`, `rebind.rs`, `rebind/`, `draft_session.rs`, and their tests, `memory/fixtures/scenarios.rs`, `plant/ensure_derived/tests.rs`. `recipe-db`: `tests/storage/race/order.rs`, `tests/storage/race/approve.rs`, `tests/storage/atomic/draft.rs` (the command inputs only) |
| 2 | A2 | T1-F | fable | high | 6.1 to 6.3 `[hard]` | `recipe-app`: `recipe/queries/parameters.rs`, `parameter_form.rs`, `preview.rs`, their tests, `recipe/queries/mod.rs`, `recipe/mod.rs`, `lib.rs`, `auth/first_statement.rs` |
| 3 | W1 | T1-C | opus | high | 7.1 to 7.4 `[hard]` | `recipe-web`: `src/error.rs`, `src/error/` (also `recipe.rs`, `lists.rs`), `src/pages/draft_reply.rs` and its tests, `templates/field-errors.html`, `templates/conflict.html`, `templates/components/alert.html`, `templates/components/field.html`, `templates/test/` (new error templates), `src/components_tests/errors.rs` |
| 3 | W2 | T3 | sonnet | high | 8.1 to 8.4 | Paths and limits of `recipe-web`: `src/pages/path_values.rs` and its tests, `src/limits.rs`, `src/csrf.rs`, new test files of both, `src/settings.rs`. Routes: `src/routes.rs`, `src/routes/matrix.rs`, `src/routes/tests.rs`, `src/pages.rs`, `src/pages/recipes.rs` (the `rows()` only), `src/pages/versions.rs`, `src/pages/parameters.rs`. Area tests: `src/pages/{recipes,versions,parameters}/route_tests.rs`, `src/pages/sweep_tests.rs`, `src/components_tests.rs`. Fixtures: `src/test_support.rs`, `src/test_support/recipes.rs`. Style: `static/components.css`, `src/style_tests.rs`, `src/style_tests/recipe_rules.rs`, `templates/components/badge.html` |
| 4 | W3 | T1-C | opus | high | 9.1 to 9.4, 9.3 and 9.4 `[hard]` | `recipe-web`: `src/pages/recipes.rs`, `src/pages/recipes/`, `src/pages/versions.rs`, `src/pages/versions/`, `templates/recipes.html`, `recipe-new.html`, `recipe.html`, `recipe-renamed.html`, `versions-table.html`, `version-new.html`, `version.html`, `parameter-table.html`, `parameter-row.html`, `templates/components/parameter.html` |
| 4 | V2 | T1-C | opus | high | 10.1 to 10.3 `[hard]` | `recipe-web`: `src/pages/parameters.rs`, `src/pages/parameters/fields.rs`, `grid.rs`, their tests, `templates/components/grid.html`, `templates/grid-values.html`, `grid-limits.html`, `templates/test/grid*.html`, `src/components_tests.rs`, `src/components_tests/grid.rs` |
| 5 | W4 | T1-C | opus | high | 11.1 to 11.5 `[hard]` | `recipe-web`: `src/pages/parameters.rs`, `src/pages/parameters/` except `fields.rs` and `grid.rs`, `templates/parameter-form.html`, `parameter.html`, `parameter-summary.html`, `translations.html`, `translation-row.html` |
| 5 | W5 | T1-C | opus | high | 12.1, 12.2 `[hard]` | `recipe-web`: `src/pages/versions.rs`, `src/pages/versions/` (also `rebind.rs` and its tests, `route_tests.rs`), `templates/version.html`, `templates/rebind-form.html` |
| 6 | W7 | T1-C | opus | high | 13.1, 13.2 `[hard]` | Modules of `recipe-web`: `src/pages/machines/form.rs`, `machines/commands.rs`, `src/pages/units/form.rs`, `units/add.rs`, `units/commands.rs`, `src/pages/types/define.rs`, `types/form_rows.rs`, and new test files. Tests that assert the changed answer: `types/define_error_tests.rs`, `types/define_tests.rs`, `types/form_rows_tests.rs`, `machines/form_tests.rs`, `machines/commands_tests.rs`. Also `units/form_tests.rs`, `units/add_tests.rs`, `units/commands_tests.rs`, `pages/plant_xss_tests.rs`. Templates: `machine-new.html`, `machine.html`, `machine-renamed.html`, `unit-form.html`, `unit.html`, `unit-renamed.html`, `udt-form.html` |
| 6 | H2 | T4 | haiku | medium | 15.1 `[mechanical]` | `recipe-web`: `src/routes/recipe_access_tests.rs`, one `mod` line in `src/routes.rs` |
| 7 | W6 | T3 | sonnet | high | 14.1 to 14.3 | `recipe-web`: `src/pages/recipe_xss_tests.rs`, `recipe_sweep_tests.rs`, `recipe_security_tests.rs`, `tests/web/xss-corpus.txt` (read only), three `mod` lines in `src/pages.rs` |

- An item with `#[cfg_attr(not(test), expect(dead_code, ...))]` loses that attribute in the wave that first calls it. The agent of that call deletes the attribute line, also in a file outside its set. It changes nothing else there.
- Second step-7 fix wave additions to the sets, with the consent of the orchestrator. M1 added one `mod` line in `pages/parameters.rs`. M1 also changed the caller `value/request_leaf.rs` and three `draft/*` test files. M3 changed the `UnitFormInput` literal in `pages/units/form.rs` and added `MachineFormInput` and `RecipeFormInput` to the exports of `plant/mod.rs` and `recipe/mod.rs`. The orchestrator wrote the entry of M2 in `pages/versions/rebind_tests.rs`. G1 changed one constant in `parameters/add_tests.rs`, `new_tests.rs`, `edit_tests.rs`, `page_tests.rs`, and the list of `pages/recipe_sweep_tests.rs`.
- Step-7 fix wave additions to the sets: F5 added one line `rebind_bound` to the `GetVersionInput` literals in `pages/versions/rebind.rs` (two), `pages/parameters/table.rs`, `pages/draft_reply.rs`, and `test_support/recipes/tests.rs`. The orchestrator gave its consent.
- Wave 6 additions to the sets: W7 also changed `src/pages/machines/page.rs`, `machines/machine_tests.rs`, `src/pages/units/page.rs`, `units/page_tests.rs`, `src/pages/types/form.rs`, `types/form_tests.rs`, and `templates/components/rows.html`. These files hold the rename forms and the out-of-band UDT form (orchestrator, wave 6).
- Wave 4 additions to the sets: W1 (T1-C) fixes the verifier findings of wave 3 in its set of wave 3, before W3 starts. V2 adds `form.array_chunk` to `src/settings.rs` and its tests.
- Wave 3 additions to the sets: W1 added the `mod` lines in `src/pages.rs` and `src/components_tests.rs` in its step 1. W2 changed one expectation in `src/pages/types/route_tests.rs` (D17).
- Wave 2 additions to the sets: V1 changed one call in `recipe-app` `src/audit/diff/tests.rs`. A3 (T1-C) fixes the verifier findings of A2 in the set of A2. V3 (T1-C) does task 3.5 in the `recipe-app` and `recipe-db` part of the set of V1.
- Wave 1 additions to the sets (verifier V1-5): A1 also changed `memory/core/store.rs` (the commit time of each audit entry). D1 also changed the visibility in `recipe-db` `src/transaction/parameter_rows.rs`.
- A1 starts with task 4.1, the port and the memory adapter. D1 starts when task 4.1 is green. A1 then continues with 4.2 to 4.4.
- Task 4.1 changes `Isolation` and `Transaction`. `recipe-db` does not compile until D1 adds the statement and the accessor. The orchestrator runs the full CI after both agents finish.
- In wave 2, A2 owns `recipe/mod.rs` and `lib.rs` of `recipe-app`. V1 reports each entry that it needs there.
- V1 works in three steps with a report after each: task 3.2 first, then 3.1, 3.3, and 3.4, then 3.5. The orchestrator resumes it with `SendMessage`. Each step ends with green tests of the changed crates.
- W1 works in three steps with a report after each: 7.1 and 7.2, then 7.3, then 7.4.
- Each area agent adds the matrix rows and the sweep pages of its routes in its own `route_tests.rs` (D12). H2 then adds the test of the `forbidden` event for each new route, in the shape of `routes/plant_access_tests.rs`.
- In wave 3, W2 owns `src/pages.rs`, `src/routes.rs`, `src/test_support.rs`, and `static/components.css`. W1 reports each rule and each `mod` line that it needs.
- In wave 4, W3 owns `version.html` and the parameter table. V2 owns `src/pages/parameters.rs`.
- In wave 5, W5 owns `version.html` and `src/pages/versions/`. W4 does not change them.
- Each verifier after a wave with `[hard]` tasks is a fresh agent with effort xhigh (`fable` up to wave 4, `opus` from wave 5). It reads only and runs no cargo.
- A T3 task that fails twice goes to T1-C with both reports. A defect that resists five attempts of other models goes to T1-F for debugging. A refusal or an unavailable Fable gives the task to T1-O once.

### D19. Signatures

```rust
// recipe-domain
pub struct LeafTexts { /* ordered pairs, one per leaf path */ }
impl LeafTexts {
    pub fn parse(pairs: Vec<(LeafPath, String)>) -> Result<Self, LeafTextsError>;
}
pub enum ValueInput { Json(String), Texts(LeafTexts) }
pub enum MemberValue { Json(serde_json::Value), Text(String) }
pub struct LeafText { pub path: LeafPath, pub class: LeafClass, pub text: String }
impl ParameterValue {
    pub fn from_texts(tree: &TypeTree, texts: &LeafTexts) -> Result<Self, ValueFaults>;
    pub fn texts(&self, tree: &TypeTree) -> Result<Vec<LeafText>, ValueFault>;
}
pub enum Expected { /* one variant per phrase of D3 and D13 */ }
pub struct RebindLeaves { /* the planned tree and its new member leaves (wave 5) */ }
pub fn rebind_new_leaves(tree: &TypeTree, from: PlcTypeId, to: &TypeTree)
    -> Result<Option<RebindLeaves>, PlantError>;
// recipe-app: OutdatedParameter gets `rebind_leaves: Option<RebindLeaves>` (wave 5).
// recipe-app (D21): GetVersionInput gets `rebind_bound: usize`.
//   VersionView gets `rebind_sum: usize, rebind_over_bound: bool, rebind_bound: usize`.

// recipe-app ports
pub enum Isolation { ReadCommitted, Serializable, ReadOnly, RepeatableRead }
#[async_trait]
pub trait RecipeReads: Send {
    async fn list_recipes(&mut self, filter: &RecipeFilter, offset: u64, limit: u32)
        -> Result<Vec<RecipeSummary>, StorageError>;
    async fn version_page(&mut self, recipe: RecipeId, offset: u64, limit: u32)
        -> Result<Vec<VersionSummary>, StorageError>;
    async fn version_detail(&mut self, version: RecipeVersionId)
        -> Result<Option<VersionDetail>, StorageError>;
    async fn last_change(&mut self, recipe: RecipeId, at: Timestamp)
        -> Result<Option<Login>, StorageError>;
    async fn parameter_page(&mut self, version: RecipeVersionId, unit: Option<UnitId>,
        offset: u64, limit: u32) -> Result<Vec<ParameterRow>, StorageError>;
    async fn unit_counts(&mut self, version: RecipeVersionId)
        -> Result<Vec<UnitCount>, StorageError>;
    async fn parameter_by_key(&mut self, version: RecipeVersionId, key: ParameterKey)
        -> Result<Option<StoredParameter>, StorageError>;
}
// Transaction gets: fn recipe_reads(&mut self) -> &mut dyn RecipeReads;
// Row types (wave 1): RecipeFilter { text: Option<Text>, machine: Option<MachineCode>,
//   status: Option<VersionState>, include_deleted: bool }. RecipeSummary { reference, name,
//   machine_code, deleted, approved: Option<u32>, draft: Option<u32> }. VersionSummary { number,
//   state, change_note, created_at, approved_at }. VersionDetail { id, recipe, number, state,
//   change_note, created_by: Login, approved_by: Option<Login>, created_at, approved_at,
//   updated_at, row_version: i64 }. ParameterRow { key, unit, unit_path, tag, name_en, ordinal,
//   plc_type, s7_address }. UnitCount { unit, path, count: u32 }.
// Query views (wave 2): ParameterList { recipe, machine, detail, units: Vec<UnitCount>,
//   unit: Option<UnitPath>, rows: Vec<ParameterRow>, type_texts: BTreeMap<PlcTypeId, String>,
//   page: u32, has_next: bool }. ParameterRef { reference, number, key }. ParameterView { recipe,
//   machine, detail, unit_path, parameter: StoredParameter, tree: TypeTree, type_text: String }.
//   LimitsView { detail, key, tag, limits }. TranslationsView { detail, key, tag, name_en,
//   description_en, translations }. GetParameterFormInput { reference, number,
//   key: Option<ParameterKey>, type_expr: Option<TypeExpr>, unit: Option<UnitPath> }.
//   ParameterForm { recipe, machine, detail, units: Vec<Unit>, unit, choices: Vec<String>,
//   parameter: Option<StoredParameter>, type_text: Option<String>, preview: Option<TypeTree> }.

// recipe-web
pub enum WebError { /* existing variants */
    Fields { lines: NonEmpty<FieldError>, form: Option<RenderedForm> },
    Conflict { error: Box<AppError>, view: ConflictView }, // boxed for clippy::result_large_err (wave 3)
}
pub struct RenderedForm(String); // Debug writes only the length
pub fn leaf_lines(errors: &RecipeErrors, input_id: &dyn Fn(&LeafPath) -> Option<String>)
    -> Vec<FieldError>;
// pages/parameters/reply.rs (step-7 fix CR-5): the parameter forms map a fault to its input
fn command_lines(/* error */, input_of: &dyn Fn(&RecipeError) -> Option<String>) -> Vec<FieldError>;
pub enum LeafInput { Value, Limit } // GridView::input_id(path, LeafInput), GridView::input_of(&RecipeError)
```

The agents keep these names. A change of a signature needs a reason in the report, and the orchestrator updates this section.

### D20. Fixes of the code review

The reviews of step 7 are in `target/p33-agents/` (not in the repository). The fixes follow the findings and the decisions of the user.

| Finding | Fix | Decision |
|---|---|---|
| CR-1, RR2-1 | The limit set keeps only rows of the tree, copies only to resolved paths, and finds the levels with an index in one set | D8 |
| CR-2 = SEC-1 | The value routes read the pairs lazily and stop after `max_pairs()` | ADR-0039 (b) |
| CR-3 = QA-2 | Every converted leaf gets its range check when another leaf fails | ADR-0041 (b) |
| CR-4 | A saved definition shows the stored canonical texts | ADR-0041 (h), A43 |
| CR-5 = DR-3 | A limit fault stands under its limit input | ADR-0039 (n) |
| CR-6, DB-7 | The filter `machine` resolves the live machine, an unknown code gives `404 E-WEB-005` | ADR-0043 (b) |
| CR-7, K12, RR1-2 | Every error answer of a form swaps the form out of band with the lines of that answer | ADR-0041 (e) |
| CR-8, CR-9 | The table body travels in `<template>`. The page of a new parameter is a stand-alone form | D7 |
| CR-10, CR-11 | The mode comes from every sent path. The toggle names the count | design-system.md 10.8 |
| DR-1, DR-2, RR1-4 | The grid uses the width of `main`, the fields keep `--width-form`, and an error code does not break | design-system.md 5, 6 |
| SEC-2 | Bound of the rebind forms | D21, ADR-0044 |
| K9 | No groups inside a group | D22, ADR-0045 |
| SEC-3, K13, K4, K10 | The form query with its purpose runs before the fields | D23, ADR-0046 |
| K5, DR-5, RR1-1 | Page of a refused form, empty toast after an error, removal toasts | D24, ADR-0047 |
| DR-4, QA-3, QA-4 | Integer hint. New text of `E-RCP-029` in `reliability.md` 1.2. New pl text of `E-RCP-054` | D13 |
| K7, K8 | Hint of a nested outdated struct. `E-RCP-005` for an unknown member path. Read-only translation rows outside the list | D13 |

The user moved the MEDIUM items of code quality and tests and every LOW item to step P4.2.

### D21. Bound of the rebind forms

ADR-0044 amends ADR-0043 (l) and sets the bound `form.rebind_leaves` (`reliability.md` section 7.11). This decision names the files of the code. It narrows the outdated parameters of `GetVersion` in D4.

| Crate | File | Content |
|---|---|---|
| `recipe-app` | `src/recipe/queries/outdated.rs` | `rebind_new_leaves` runs only for a caller with `ParameterEdit` on a live `Draft`. A running sum stops the work when it passes the bound |
| `recipe-app` | `src/recipe/queries/versions.rs` | The input of `GetVersion` takes the bound. `VersionView` tells if the sum passed the bound, and gives the sum and the bound. The bound comes back because `rebind.rs` builds its form from the view and has no access to the settings |
| `recipe-web` | `src/settings.rs` | The setting `form.rebind_leaves` with the default 16 384, next to `form.array_chunk` |
| `recipe-web` | `src/pages/versions/page.rs` | The page gives the setting to `GetVersion`. `rebind_form.rs` shows no rebind form to a reader without new member leaves |
| `recipe-web` | `src/pages/versions/rebind_form.rs` | Over the bound, each form shows the notice and holds no field of a new member and no submit button |
| `recipe-web` | `i18n/en.ftl`, `i18n/pl.ftl` | The key `rebind-too-large` |

| Key | en | pl |
|---|---|---|
| `rebind-too-large` | The rebinds of this draft need at least { $count } new values. One page shows at most { $max } new values. | Przepięcia tej wersji roboczej wymagają co najmniej { $count } nowych wartości. Jedna strona pokazuje najwyżej { $max } nowych wartości. |

- `{ $count }` is the sum that the query reached when it stopped. `{ $max }` is the value of `form.rebind_leaves`.
- The command `RebindTypeRevision` and its route do not change.

### D22. Nested arrays inside a group

ADR-0045 amends ADR-0039 (l). It answers the question of D8 about an array inside a group.

- `pages/parameters/grid/walk.rs` keeps the rule. Only an array that does not stand inside a group splits into groups of `form.array_chunk` elements.
- An array inside a group shows all its rows in that group. It gets no toggle row and no `tbody` of its own.
- The rule holds at every depth. An array of the top level and an array member of a struct outside a group still split.
- A new test in `pages/parameters/nested_group_tests.rs` uses the type `ARRAY[1..120] OF ST_Zone`, where `ST_Zone` holds `profile : ARRAY[1..120] OF INT`.
  - The grid holds two groups of the outer array and no other group.
  - The first group holds the 120 rows of `profile` of each of its elements.
  - No `tbody` holds a `tbody`, and each toggle names exactly one group.
- No text key changes.

### D23. Permission check of a form route

ADR-0046 amends ADR-0041 (a) and refines ADR-0013. A route of a form runs its form query before it reads its fields. A caller without the permission gets `403 E-WEB-002` and one `forbidden` event that names the command.

| Crate | File | Content |
|---|---|---|
| `recipe-app` | `src/recipe/queries/recipes.rs` | The input of `GetRecipeForm` takes its purpose `CreateRecipe` or `RenameRecipe`. `authorize` takes the operation of the purpose |
| `recipe-app` | `src/recipe/queries/parameter_form.rs` | `GetParameterFormInput` takes its purpose: one of the eight commands of the `ParameterEdit` row of `api.md` section 3.3 |
| `recipe-app` | `src/recipe/queries/versions.rs` | `GetVersionFormInput` takes the purpose `CreateVersion` |
| `recipe-app` | `src/recipe/queries/parameters.rs` | `GetLimits` returns the type tree that the limits grid needs (ADR-0046 (d)) |
| `recipe-app` | `src/plant/queries/machines.rs`, `units.rs`, `types.rs` | `GetMachineForm`, `GetUnitForm`, and `GetUdtForm` take their purpose |
| `recipe-app` | `src/auth/first_statement.rs` | One test per purpose: an Operator gets `Forbidden` with the operation of the command and one `forbidden` event |
| `recipe-app` | `src/recipe/queries/source_tests.rs`, `src/plant/queries/source_tests.rs` | The source tests of the first statement accept `authorize` with the operation of the purpose |
| `recipe-app` | `src/auth/matrix.rs`, `src/auth/matrix_doc.rs` | The catalog holds the permission of each purpose. The row "Rename recipe" of `api.md` section 3.3 names `GetRecipeForm` with `RecipeEdit` |
| `recipe-web` | `src/pages/machines/form.rs`, `src/pages/machines/commands.rs` | The create route and the rename route run `GetMachineForm` before they read the fields |
| `recipe-web` | `src/pages/units/add.rs`, `src/pages/units/commands.rs` | The add route and the rename route run `GetUnitForm` before they read the fields |
| `recipe-web` | `src/pages/types/define.rs` | The routes of a new type and of a new revision run `GetUdtForm` before they read the fields |
| `recipe-web` | `src/pages/recipes/form.rs`, `src/pages/recipes/rename.rs` | The create route and the rename route run `GetRecipeForm` with their purpose before they read the fields |
| `recipe-web` | `src/pages/versions/form.rs`, `src/pages/versions/rebind.rs` | The create route runs `GetVersionForm`. The rebind route runs `GetParameterForm` with the purpose `RebindTypeRevision` |
| `recipe-web` | `src/pages/parameters/` | Each `POST` route runs `GetParameterForm` with the purpose of its command before it reads the fields |
| `recipe-web` | `src/pages/parameters/value.rs` | `GET .../limits` runs `GetLimits` in place of `GetParameter` |

- A route parses its path values before the form query, because the form query needs them. A bad path value keeps its code of `api.md` section 1.
- A route test sends a bad field and an unknown field as an Operator to each route of the table. It expects `403 E-WEB-002`, one `forbidden` event, and no form in the body.
- The archive routes and the user routes are not in this decision (ADR-0046, risks).
- No text key changes.

### D24. Refused form page and toast

ADR-0047 amends ADR-0036 (i), (m), and (o). A request without HTMX with field errors and a session gets the page of the refused form. Every HTMX error answer empties `#toast`, and `#toast` sticks at the top of the view.

| Crate | File | Content |
|---|---|---|
| `recipe-web` | `src/layout.rs` | `SessionFrame` holds the navigation, the CSRF token, and the login of the session. It has no `Debug`. `Layout::refused` builds the layout of the page from it, with no language switch |
| `recipe-web` | `src/error/form.rs` | `RenderedForm` holds the session parts in a box. `page_html` returns the form without any `hx-*` attribute (RR1-1). `Debug` writes only the length |
| `recipe-web` | `src/pages/draft_reply.rs` | `refused` copies the session parts for a request without HTMX |
| `recipe-web` | `src/error/answer.rs` | The page of the refused form, with the form before the end tag of `main`. Each HTMX error answer ends with `<div id="toast" hx-swap-oob="innerHTML"></div>` |
| `recipe-web` | `templates/refused-form-page.html` | The layout of the session, `field-errors.html` in `#error-panel`, and one `h1` with the key `error-page-title` |
| `recipe-web` | `static/app.css` | `#toast` with `position: sticky`, `top: 0`, and `z-index: 1`. `html` with `scroll-padding-top: calc(var(--target) + var(--space-4))` |
| `recipe-web` | `src/pages/parameters/remove.rs`, `templates/parameter-removed-toast.html` | The removal of a parameter sends the toast `parameter-deleted` with the tag. The table answer sends the toast out of band. The content answer puts it into `#toast` |

- The status of the page of a refused form is the status of the error.
- Every other error, and a request without a session, keeps the full error page.
- The removal of a translation row keeps its toast `translation-deleted`.
- No text key changes.

## Risks / Trade-offs

- [Risk] A panel PC renders a value of 16 384 leaves slowly. → Closed groups, and the load test of P4.2 measures the page.
- [Risk] `recipe-db` does not compile between task 4.1 and the end of D1. → D1 starts at once after task 4.1, and the full CI runs after both agents.
- [Risk] The change of `expected` from text to `Expected` touches many domain tests. → D17 lists the change, and V1 keeps the English phrase in `Display`.
- [Risk] The out-of-band form doubles the size of an error answer. → The answer holds only the form of the request.
- [Risk] A body of the four value routes floods the reader with pairs. → The reader counts pairs and path segments and stops at the first excess (D11).
- [Risk] A form rendered again carries a newer `row_version` than its data. → Every answer without a write keeps the sent `row_version` (ADR-0039 (h)).
- [Risk] Two audit entries share one `occurred_at`. → `last_change` takes the highest `id`.
- [Risk] Seven waves take long. → Read-only reviews and verifiers run in parallel with the CI of each wave.

## Migration Plan

No migration. The change adds no table, column, index, or grant. A rollback is the previous image.

## Open Questions

None.
