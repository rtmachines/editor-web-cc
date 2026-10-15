# Design: web-plant-model

## Context

See `proposal.md`, section Why. Step P3.1 and the earlier steps left these parts:

- `recipe-app` has the seven plant commands, the operations and matrix rows of the ten queries (`auth/operation/mod.rs`, `auth/matrix.rs`), and no query struct.
- The ports have `machine_by_code`, `archived_machine_by_code`, `machine_by_id`, `live_units`, and `type_rows`. No port lists machines or finds the use of a type.
- Five types implement the plant ports: `PgTransaction` in `recipe-db`, `MemoryTransaction` in `memory/plant/` and `memory/types/`, and the test wrapper `ShortRowsTransaction` in `plant/short_rows.rs`.
- `ListUsers` (`identity/queries/users.rs`) is the pattern of a query: `authorize`, then `begin(Isolation::ReadOnly)`, then the port. Its input counts pages from 0.
- `Missing` has no variant for a malformed machine code, a malformed type name, or an unknown revision.
- `recipe-web` has the route table of `routes.rs`, the sweep list `PAGES`, `test_support.rs`, the components of P3.1, and `static/app.css` with 790 lines.
- `routes/tests.rs` keeps `Want`, `Endpoint`, `world()`, and `answer()` private. `answer()` replaces only `{login}`. `pages/xss_tests.rs` reads `PAGES`.
- `see_other` takes a `&'static str`, so it cannot build `/machines/{code}`.
- A `GET` page answers the full page also for an HTMX request. Only the command answers of the user pages branch on `HX-Request`.
- `axum::extract::Form` reads a body into `Vec<(String, String)>`. No code reads an indexed field such as `members[0].name`.
- `MemoryStore` has public seeds for machines, types, recipes, and parameters under the feature `memory`.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Decisions of this change | ADR-0037 (a) to (y) |
| Routes, use cases, permissions, errors | `api.md` sections 1, 2.2, 2.3, 3, 5 |
| Tokens, components, tree, repeated rows, answers to HTMX | `design-system.md` sections 1, 7 to 12 |
| CSP, scripts, `hx-*` rule | `api.md` section 9, `.claude/rules/web.md` |
| Permission check first | ADR-0013, `api.md` section 3.2 |
| Value objects and their errors | `reliability.md` sections 1.2 and 4.3, ADR-0025 |
| Settings | `reliability.md` section 7.11 |
| Type expression, type tree | `domain-model.md` section 2.1 |
| Layout, components, texts of P3.1 | archived design `web-ui-foundation` D1 to D13 |
| Pipeline, CSRF, error mapping | archived design `identity-and-sessions` D11 to D13 |
| Users and their tasks | `docs/plan/ux-brief.md` (not normative) |

## Goals / Non-Goals

**Goals:**

- Each route of `api.md` sections 2.2 and 2.3 answers each role as the matrix of `api.md` section 3.3 says.
- Each query checks its permission as its first statement and reads in one `ReadOnly` transaction.
- Each new page renders in `pl` and `en` with the layout, the CSP, and no inline script, style, or handler.
- No `hx-*` attribute of a plant template holds user data, and no URL holds the CSRF token.

**Non-Goals:**

- No migration, no new grant, no new dependency, and no change of the CSP or of a script.
- No change of `recipe-domain`, `recipe-export`, or `recipe-server`.
- No JavaScript runtime test. The test in a browser uses `examples/dev_server.rs`.
- No change of the answers of the pages of P3.1, except the second style sheet.

## Decisions

### D1. Answers of the user (2026-10-14)

| Question | Answer | Record |
|---|---|---|
| a. Load of the children of a node | the server renders the whole tree, `tree.js` only toggles | ADR-0037 (a) to (e) |
| b. User data and `hx-*` | form `action` or link `href` with `hx-boost` | ADR-0037 (f), (g) |
| c. Tests of the fragments | structural assertions, no snapshot crate | ADR-0037 (s) |
| d. Test in a browser | `examples/dev_server.rs` on the memory adapters | ADR-0037 (t) |
| e. Style sheet near 800 lines | two files by layer | ADR-0037 (r) |
| f. Member row without JavaScript | the form again with `add=1` | ADR-0037 (h) |
| g. Choice of the member type | `TypeExpr` input with a `datalist` | ADR-0037 (i) |
| `E-PLT-005` cannot occur through HTTP | seeded cycle in a test. The browser test shows `E-PLT-013` and `E-PLT-021` | ADR-0037 (u) |
| Page size and search of `ListMachines` | 100 rows, `q` with no difference of case | ADR-0037 (l) |
| Form of a new revision | `GET .../types/new?from={name}` | ADR-0037 (j) |
| Archived units in the tree | a live machine shows its live units | ADR-0037 (d) |
| Plan review, 48 findings | all accepted. "Add a member" is a `POST`. A message follows an archive. The name `new` is refused | ADR-0037 (h), (v) to (y), this design |

### D2. Module layout

- `recipe-app`:
  - `plant/queries/` (new): `mod.rs`, `machines.rs` (`ListMachines`, `GetMachine`, `GetMachineForm`), `units.rs` (`GetUnitTree`, `GetUnitForm`, `GetUnit`), `types.rs` (`ListTypes`, `GetTypeTree`, `GetUdtForm`), `usage.rs` (`GetTypeUsage`), `type_text.rs`, and one test file per module.
  - `blocking.rs`: `PageSizes` in `AppSettings` (D3).
  - `ports/plant.rs` and `ports/types.rs`: the two read methods of D4. `error/missing.rs`: three variants (D5).
  - `memory/plant/list.rs` and `memory/types/usage.rs` (new): the memory adapters of D4. `plant/short_rows.rs` forwards `list_machines`.
- `recipe-db`:
  - `src/transaction/machine_list.rs` and `src/transaction/type_usage.rs` (new), declared in `src/transaction.rs`. `plant.rs` and `types.rs` delegate to them.
  - `tests/storage/contract/machine_list.rs` and `tests/storage/contract/types/usage.rs` (new).
- `recipe-web`:
  - `src/routes/matrix.rs` (new, test only): the shared matrix of D12.
  - `src/pages/machines.rs`, `src/pages/units.rs`, `src/pages/types.rs` (new area modules, D12), each with a directory for its parts and tests.
  - `src/pages/path_values.rs` (new): the parsers of the path values and the query values of D5.
  - `src/pages/types/form_rows.rs` (new): the reader of indexed fields (D8).
  - `src/test_support/plant.rs` (new): `Fixture::seed_plant` (D12).
  - `templates/`: the pages and fragments of D6. `templates/components/`: the macros of D10.
  - `static/components.css` (new, D9). `src/style_tests/component_rules.rs` (new).
  - `examples/dev_server.rs` (new, D15).
- No file passes 800 lines. A new test goes into a new file `<module>/<topic>_tests.rs` when the old file has more than 700 lines.

### D3. Queries of `recipe-app`

- Each query is a struct with `ctx: AppContext`, `impl UseCase` with its `OPERATION`, `pub const fn new(ctx)`, and `execute(&self, user: &CurrentUser, meta: &RequestMeta, input)`. It has the `tracing::instrument` attribute of `ListUsers`.
- The first statement of `execute` is `authorize(Self::OPERATION, user, meta, self.ctx.security.as_ref()).await?`. Then the query opens one `Isolation::ReadOnly` transaction and reads the ports. It writes no row and no audit entry.
- The query ends the transaction before it resolves a type tree or builds a view (`reliability.md` section 7.4).
- `GetMachineForm` reads no port. It returns the five platforms of `PlcPlatform::ALL`.
- `ListMachines` takes a page that counts from 0, as `ListUsers` does. `recipe-web` maps the URL page `n` to `n - 1`.
  - It takes `archived: Option<MachineCode>`. It returns that code only when an archived machine has the code and no live machine has it.
- `GetMachine` takes `MachineRef::Code(MachineCode)` or `MachineRef::Id(MachineId)`. By code it finds only a live machine. By identifier it finds a live or an archived machine.
  - It takes `archived: Option<UnitPath>` and returns the path only when the machine holds that unit archived.
- `GetUnitTree`, `GetMachine`, and `GetUnit` return the `Machine` of the domain. The view keeps the live units for a live machine and every unit for an archived machine (spec "Unit tree of a machine").
- `GetUnit` finds a live or an archived unit of the live machine. It returns the unit, its parent, its live children, and the count of its live descendants.
- `GetUnitForm` returns the live units of depth 1 to 4 as parents and the proposed segment of the chosen parent.
  - The proposal is the lowest segment from 1 to 64 that no live or archived child uses. It is `None` when all 64 are in use.
  - A live parent of depth 5 gives `PlantError::UnitDepthExceeded` (`E-PLT-002`) with the path of its first child, for example `U1.2.4.5.6.1`.
- `ListTypes` reads `type_rows` and keeps the struct rows of the machine. It sorts them by the bytes of the name and by the revision, the latest first.
- `GetTypeTree` builds `PlatformTypes` and resolves the `TypeTree` of the revision. It also returns the list of revisions and the revision that this one supersedes.
- `GetUdtForm` with `from` returns the members of the latest revision. `type_text.rs` writes the type expression of a row, for example `ARRAY[1..4] OF STRING(16)`.
  - A test parses each text with `TypeExpr::parse` and resolves it to the same type row again.
  - `GetUdtForm` also returns the proposals of the `datalist`. First come the primitives of the platform without the seed rows `STRING` and `WSTRING`. Then come `STRING`, `WSTRING` when the platform has it, and the latest revision of each struct as `<name>@<revision>`.
- `GetTypeUsage` reads `type_rows` to find the revision, runs the usage statement, and reads `type_rows` again for the type texts. Type rows are append-only (ADR-0006), so the last read sees each type of the usage rows.
- `ListMachines` and `GetTypeUsage` compute `offset = page * size` and `limit = size + 1` with `checked_*`. One row more than the size means a next page.
- `AppSettings` gets `pages: PageSizes { machines: u32, type_usage: u32 }`. `Default` gives 100 and 100 (`reliability.md` section 7.11). A test changes a size with `..AppSettings::default()`.

### D4. Read ports and adapters

- `PlantRepository::list_machines(&mut self, filter: &MachineFilter, offset: u64, limit: u32) -> Result<Vec<MachineSummary>, StorageError>`.
  - `MachineFilter { text: Option<Text>, include_archived: bool }`. `MachineSummary { id, code, name, platform, archived_at }`.
  - The SQL keeps a row when `$1 IS NULL` or when `strpos(lower(code), lower($1)) > 0 OR strpos(lower(name), lower($1)) > 0`.
  - It orders by `substr(code, 2)::integer`, `archived_at DESC NULLS FIRST`, and `id`. The memory adapter uses `to_lowercase` and the same order.
- `TypeCatalog::type_usage(&mut self, machine: MachineId, type_id: PlcTypeId, offset: u64, limit: u32) -> Result<Vec<TypeUsageRow>, StorageError>`.
  - `TypeUsageRow { reference, number, status, tag, parameter_type }`.
  - The SQL builds one edge set of the member links and the element links. A recursive CTE with `UNION` then collects the type and each type that holds it. The recursive term reads the CTE once.
  - The query joins `parameter`, `recipe_version`, and `recipe` of the machine. It keeps a version that is not discarded and a recipe that is not deleted.
  - It orders by `reference_number COLLATE "C"`, `number DESC`, and `tag COLLATE "C"`. The column names come from `data-model.md` section 3.
  - The memory adapter walks the same edges with a set of visited types, so a seeded cycle ends.
- `offset` and `limit` reach sqlx through `i64::try_from`. A failed conversion is a `StorageError` of the adapter.
- The existing indexes `plc_type_member_member_type_id_idx`, `plc_type_element_type_id_idx`, and `parameter_plc_type_id_idx` serve the CTE. The machine table is small, so the list needs no index.
- `recipe_app` already has `SELECT` on each table, so `grants.expected` does not change.
- A contract case of `tests/storage/contract/` runs each behavior on the memory harness and on the Postgres harness:
  - the order of the code number, the search with Polish letters in another case (`Łódź` and `łÓDŹ`), and no search text. Also `include_archived` and offset with limit.
  - the use through a member, through an array element, through two levels, and through two members of one struct. Also a deleted recipe, a discarded draft, and the order.

### D5. Path values, request fields, and field names

- `Missing` gets `MalformedMachine` (a malformed `{code}` or `{uuid}`), `MalformedType` (a malformed `{name}`, `from`, or `rev`), and `TypeRevision { code, name, revision }`.
  - The keys are `missing-malformed-machine`, `missing-malformed-type`, and `missing-type-revision` (D13).
  - `MalformedMachine` and `MalformedType` hold no text, so no malformed value reaches a message or a log line.
- A malformed `{path}`, `parent_path`, or `archived` path gives the `PlantError` of `UnitPath::parse` (`E-PLT-017` or `E-PLT-002`), as `api.md` section 1 says.
- `pages/path_values.rs` holds one parser for each path value and query value. Each area module uses it.
- `FieldReason` gets 9 variants, each with its key `reason-<name>`. `FieldReason::ALL` grows to 20:

| Variant | Field | Rule |
|---|---|---|
| `MachineCode` | `code`, `archived` of `/machines` | `MachineCode::parse` fails |
| `Name` | `machine_name`, `main_unit_name`, `unit_name` | `Text::parse` fails |
| `Platform` | `plc_platform` | not one of the five platforms |
| `TypeName` | `type_name` | `TypeName::parse` fails, or the name is `new` (ADR-0037 y) |
| `MemberName` | a member row | `MemberName::parse` fails |
| `MemberRows` | a member row | the row has a name and no type, or a type and no name |
| `MemberCount` | `members` | the form holds more than `type.max_members` rows |
| `Flag` | `show_archived`, `add` | a value other than `1` |
| `SearchText` | `machine_q` | a text that is not empty and that `Text::parse` refuses |

- A malformed index of a row, a repeated key, or an unknown key is `FieldReason::Form`.
- A bad `TypeExpr` of a member is `E-PLT-013`, because `TypeExpr` has its own code.
- The field names of a form give the input `id` and the key `field-<name>` (ADR-0037 v). The `name` attribute keeps the field of `api.md`:

| Form | Field names (`id`) | `name` attributes |
|---|---|---|
| New machine | `code`, `machine_name`, `plc_platform`, `main_unit_name` | `code`, `name`, `plc_platform`, `main_unit_name` |
| Rename of a machine | `machine_name` | `name` |
| New unit | `parent_path`, `segment`, `unit_name` | `parent_path`, `segment`, `name` |
| Rename of a unit | `unit_name` | `name` |
| UDT form | `type_name`, `members-<i>-name`, `members-<i>-type` | `name`, `members[i].name`, `members[i].type` |
| Machine search | `machine_q`, `show_archived` | `q`, `show_archived` |

- `error/view.rs` adds `code`, `machine_name`, `plc_platform`, `main_unit_name`, `parent_path`, `segment`, `unit_name`, `type_name`, `machine_q`, and `show_archived` to `INPUT_FIELDS`. The users page keeps `q`.
- `WebError` gets the variant `BadRow { row: u16, field: RowField, reason: FieldReason }` with `RowField::Name` and `RowField::Type`. Its code is `E-WEB-008`.
  - Its line shows the label `udt-member-name` or `udt-member-type` with the number `row + 1`. It links to `#members-<row>-name` or `#members-<row>-type`.
  - `MemberCount` uses `BadRequest` with the field `members`. Its line links to `#members-0-name`.

### D6. Pages and fragments

A page route answers three shapes (ADR-0037 w):

1. A request without `HX-Request`, or with a target other than `content`, gets the full page.
2. An HTMX request with `HX-Target: content` gets the content of `main`, the `title` element, and the out-of-band field `next` of the language switch. Its `h1` has `tabindex="-1"` and `autofocus`.
3. An HTMX request to a fragment route gets the fragment of the table below.

| Route | Template | Answer |
|---|---|---|
| `GET /machines` | `machines.html` | table of D10 with the pager. Search form `method="get"` with `machine_q` and the checkbox `show_archived`. The archive message of `archived` in `#toast` |
| `GET /machines/new` | `machine-new.html` | form with `code`, `machine_name`, the select `plc_platform`, and `main_unit_name`. The hints of D13 |
| `POST /machines` | - | `303` to `/machines/{code}` |
| `GET /machines/{code}` | `machine.html` | `h1`, description list, link to the types, `#unit-tree` with `unit-tree.html`, `#unit-form` (empty), the rename form, and the archive form in `details`. The archive message of `archived` in `#toast` |
| `GET /machines/by-id/{uuid}` | `machine.html` | an archived machine: the large badge "Archived", a tree without links, no form, and no link under `/machines/{code}`. A live machine: the page of `GET /machines/{code}` |
| `POST /machines/{code}/rename` | `machine-renamed.html` | the new `#machine-name`, the toast, and the empty `#error-panel` out of band. Without HTMX: `303` to `/machines/{code}` |
| `POST /machines/{code}/archive` | - | `303` to `/machines?archived={code}` |
| `GET /machines/{code}/units` | `unit-tree.html`, `unit-tree-page.html` | HTMX: the whole tree (D7). Without HTMX: the page of the tree |
| `GET /machines/{code}/units/new` | `unit-form.html`, `unit-form-page.html` | `HX-Target: unit-form`: the form with an `h2` of the parent and `autofocus` on its first input. Otherwise the page of the form |
| `POST /machines/{code}/units` | `unit-added.html` | the tree with the focus unit, the toast, the empty `#error-panel`, and a new `#unit-form` for the same parent out of band, with no `autofocus`. Without HTMX: `303` to `/machines/{code}` |
| `GET /machines/{code}/units/{path}` | `unit.html` | `h1`, description list, live children, rename form, archive form in `details` with the count of descendants |
| `POST .../units/{path}/rename` | `unit-renamed.html` | as the machine rename. Without HTMX: `303` to the unit page |
| `POST .../units/{path}/archive` | - | `303` to `/machines/{code}?archived={path}` |
| `GET /machines/{code}/types` | `types.html` | table of the revisions with the mark "latest" |
| `GET /machines/{code}/types/new` | `udt-form.html` | form of D8 |
| `POST /machines/{code}/types` and `.../revisions` | `udt-form.html` | `303` to `/machines/{code}/types/{name}` or to `...?rev={n}`. With `add=1`: `200` with the form of D8 |
| `GET /machines/{code}/types/{name}` | `type.html` | `h1` with `<name>@<revision>`, revision links, the type tree, the link to the usage, the link to a new revision |
| `GET .../types/{name}/usage` | `type-usage.html` | table with the version badge, the pager, the empty state |

- Every form that sends `POST` holds `csrf_token`. No form that holds `csrf_token` sends `GET`.
- A create form, an archive form, and the UDT form have `hx-boost="true"`, `hx-target="#content"`, `hx-push-url="true"`, and `hx-sync="closest form:drop"`.
- A rename form has `hx-boost="true"`, `hx-target="#machine-name"` or `hx-target="#unit-name"`, `hx-swap="innerHTML"`, `hx-push-url="false"`, and `hx-sync="closest form:drop"`.
- The unit form has `hx-boost="true"`, `hx-target="#unit-tree"`, `hx-swap="innerHTML"`, `hx-push-url="false"`, and `hx-sync="closest form:drop"`.
- The link "Add a sub-unit" has `hx-boost="true"`, `hx-target="#unit-form"`, `hx-swap="innerHTML"`, and `hx-push-url="false"`.
- An answer with `add=1` sets `HX-Push-Url: false`, so the address bar keeps the URL of the form.
- A page shows a form or a link to it only when the role holds the permission of its command. The page checks `Operation::requirement().allows(role)`, as the navigation does.
  - The commands are `CreateMachine`, `RenameMachine`, `ArchiveMachine`, `AddUnit`, `RenameUnit`, `ArchiveUnit`, and `DefineUdt`.
- `POST /machines/{code}/units` runs `AddUnit`, then `GetUnitTree` and `GetUnitForm`. Each one checks its permission first.
- A `POST` with `add=1` runs `GetUdtForm` and no command. Its route keeps the operation `DefineUdt`, so read-only mode refuses it with `E-WEB-006`.

### D7. Unit tree

- `pages/machines/tree.rs` builds the view model. It takes the machine, the scope (live units or every unit), the role, and `focus: Option<&UnitPath>`. W4 uses it for the answer of `AddUnit`.
- The view model is a tree of nodes, `UnitTree { roots }`, and each node holds its children. Each node holds the unit identifier, the path, the name, the depth, and the state. It also holds the flags `expanded`, `focus`, `link`, and `can_add`.
- Askama refuses a macro inside its own expansion. So `unit-tree.html` writes one macro per level, and `type.html` loops over a flat list in preorder. Tests pin both to the markup of the macros of D10.
- `expanded` is true for a node of depth 1 or 2 and for each ancestor of the focus unit.
- `can_add` is true for a live node of depth 1 to 4 when the role holds `AddUnit`. `link` is false on the page of an archived machine.
- The macro `tree::unit_node` writes one row with the button, the link to the unit page, and the optional link "Add a sub-unit". The macro `tree::unit_leaf` writes a row without children.
  - The row is a flex row with the gap `--target-gap`, and two rows keep the same gap. Each link has `.target-link`.
  - The row wraps at a small width. The unit path has `white-space: nowrap`.
- A link "Add a sub-unit" has the visible text of `tree-add-sub-unit` and the `aria-label` of `tree-add-sub-unit-to` with the path.
- The link of the focus unit has `autofocus`. A full page has no focus unit.
- `tree.js` does not change.

### D8. UDT form

- `GET /machines/{code}/types/new` reads its query with `#[serde(deny_unknown_fields)]`. It accepts only `from`. A request with `csrf_token` in the query gets `E-WEB-008`.
- `pages/types/form_rows.rs` reads the pairs of a `POST` body. It accepts `csrf_token`, `name`, `add`, `members[<i>].name`, and `members[<i>].type`. Any other key is `FieldReason::Form`.
- An index is a decimal number of 1 to 4 digits with no leading zero, except the index `0`. It is below `type.max_members`.
- Two keys for one field of one row are `FieldReason::Form`. The reader finds them on the parsed index.
- The reader keeps the rows in the order of the index and numbers them again from 0. A gap of indices is permitted.
- The `POST` of a definition skips a row whose name and type are both empty. A row with only one of them is `BadRow` with `MemberRows`.
- With `add=1`, the server renders the sent rows and one empty row. With `from`, `GET` renders the members of the latest revision and no empty row. A new form has one empty row.
  - The first input of the new empty row has `autofocus`. The `h1` of that answer has no `autofocus`, because the first `autofocus` of a page takes the focus.
  - Enter in a text input sends the first submit button, "Add a member". This follows the plan review A4.
  - An index of 1024 or more gives `FieldReason::MemberCount`.
  - A form at `type.max_members` rows shows the notice `udt-max-rows` in place of the button "Add a member".
- The member rows stand in one `fieldset` with `id="members"` and the `legend` of `field-members`. Each row has the label `udt-member-row` with the row number.
- The hint `udt-type-hint` stands once above the rows. Each type input names it in `aria-describedby`.
- The type input has `list="type-choices"`, `class="data"`, `autocomplete="off"`, `autocapitalize="none"`, and `spellcheck="false"`. The page holds one `datalist` with `id="type-choices"`.
- The button "Add a member" stands before the create button. It has `name="add"` and `value="1"`.
- With `from`, the name is plain text in the `h1`, and the form `action` is `/machines/{code}/types/{name}/revisions`.
- A struct name `new` gives `E-WEB-008` with `FieldReason::TypeName` (ADR-0037 y).

### D9. Style sheets

- `app.css` keeps lines 1 to 498 and 766 to 790 of today. These lines hold the header, `:root`, the fonts, the page, the skip link, the targets, and the focus. They also hold the top bar, the base rules of the buttons and fields, `.htmx-indicator`, and the forced colors of these.
- `components.css` takes the sections "Panel, table, empty state", "Badge", "Alert, field lines, toast", and "Tree". It has its own `@media (forced-colors: active)` block for its rules.
- New rules of P3.2 go into `components.css`. They are the pager, the danger disclosure, the select, the checkbox, the member rows, the tree row, and the large badge.
- `.tree-group` indents by `var(--space-4)`. `.tree-node` loses `width: 100%`.
- `static_files.rs` serves the file with the media type `text/css; charset=utf-8`. `base.html` links it after `app.css`.
- The style tests parse both files with the parser of `style_tests/css.rs`. The token test reads only `app.css`. The color rule and the target rule read both files.
- The new rule tests go into `style_tests/component_rules.rs`, because `style_tests/rules.rs` has 727 lines.
- A test checks that each style sheet has at most 800 lines and that only `app.css` holds `:root`.
- `summary` gets the rule of a target. `details[open]` shows the icon `tree-expanded`, and a closed `details` shows `tree-collapsed`.

### D10. Components

- `field.html` gets `select(t, name, label, hint, options, selected)`, `check(t, name, label, checked)`, and `listed(t, name, label, hint_id, list_id, value)`.
- `badge.html` gets `badge::archived(word)` and `badge::archived_large(word)` with the icon `status-retired`.
- `tree.html` gets `unit_node`, `unit_leaf`, `type_node`, and `type_leaf` of D7. The group `id` of a type node is `tree-group-<uuid>-<index>` (ADR-0037 e).
- `pager.html` (new): `pager(t, previous, next, current)` in a `nav` with the `aria-label` of `pager-label`. Each link has the class `.target-link`.
  - A query value in a pager link is percent-encoded. The implementer uses the askama filter `urlencode_strict` when the build has it, or a small encoder in `recipe-web`. The change adds no crate.
- `rows.html` (new): `member_row(t, index, number, name, type_text, focus)` for one member row.
- `danger.html` (new): `danger(t, summary, warning, button, action, csrf)` for an archive form inside `details`.
- `heading.html` (new): `h1(layout, text)`. It writes `tabindex="-1"` and `autofocus` only for the content answer of D6.
- A component gets a test template in `templates/test/`, and `components_tests/plant.rs` renders it.

### D11. Navigation

- `NavTarget` gets `Machines` with the path `/machines`, the key `nav-machines`, and the query `ListMachines`. The order of `ALL` is `Recipes`, `Machines`, `Users`, `Me`.
- The current link of every page under `/machines` is `Machines`.

### D12. Route areas and shared test support

Wave 1 prepares the route table and the tests, so that the agents of wave 3 and wave 4 change disjoint files:

- `routes.rs` extends `table()` with `pages::machines::rows()`, `pages::units::rows()`, and `pages::types::rows()`. Each function returns no row at first.
- `src/routes/matrix.rs` (test only) takes `Principal`, `PRINCIPALS`, `Want`, `Endpoint`, `endpoint`, `world`, `answer`, and `assert_want` from `routes/tests.rs`. They become `pub(crate)`.
  - `Want` gets `Redirect(&'static str)` for a `303` to a path and `Status(StatusCode, &'static str)` for a status with a code.
  - `answer()` replaces `{login}` with `user2`, `{code}` with `M7`, `{path}` with `U1.2`, and `{name}` with `ST_Zone`. It replaces `{uuid}` with the fixed identifier of the archived machine `M12`.
  - `world()` also calls `Fixture::seed_plant()`.
- The test module of each area has `route_rows()` (matrix rows) and `sweep_pages()` (sweep requests). Both return nothing at first. `routes/tests.rs` and `pages/sweep_tests.rs` collect them.
- The count of the mutating routes in the coverage test becomes the count of the mutating rows of the matrix.
- `pages/sweep_tests.rs` turns `PAGES` into a function. The sweep fixture calls `seed_plant()`.
- `pages/xss_tests.rs` keeps its own list of the six pages of P3.1 that accept any query. It no longer reads the sweep list.
- `test_support/plant.rs` holds `Fixture::seed_plant()` with fixed identifiers:
  - the primitive types, and the live machine `M7` (S7-300) with 64 live units down to `U1.2.4.5.6`,
  - the struct `ST_Zone` in the revisions 1 and 2, and the struct `ST_Line@1` with a member of the type `ARRAY[1..4] OF ST_Zone@2`,
  - the recipe `R-100` with a draft that holds the parameter with the tag `LINE` of the type `ST_Line@1`,
  - the archived machine `M12` with the live unit `U1` and the archived unit `U2`.

### D13. Texts

The Haiku task copies this table into `en.ftl` and `pl.ftl`. The English text follows ASD-STE100. The Polish text is the translation. The Polish word for a struct member is "składnik".

| Key | en | pl |
|---|---|---|
| `nav-machines` | Machines | Maszyny |
| `status-archived` | Archived | Zarchiwizowana |
| `field-code` | Machine code | Kod maszyny |
| `field-machine-name` | Machine name | Nazwa maszyny |
| `field-plc-platform` | PLC platform | Platforma PLC |
| `field-main-unit-name` | Name of the main unit | Nazwa jednostki głównej |
| `field-parent-path` | Parent unit | Jednostka nadrzędna |
| `field-segment` | Last number of the unit path | Ostatni numer ścieżki jednostki |
| `field-unit-name` | Unit name | Nazwa jednostki |
| `field-type-name` | Type name | Nazwa typu |
| `field-members` | Members | Składniki |
| `field-machine-q` | Code or name contains | Kod lub nazwa zawiera |
| `field-show-archived` | Show the archived machines | Pokaż zarchiwizowane maszyny |
| `field-archived` | Archived object | Zarchiwizowany obiekt |
| `field-add` | Add a member | Dodaj składnik |
| `reason-machine-code` | a machine code is M and a number from 1 to 9999999 | kod maszyny to M i liczba od 1 do 9999999 |
| `reason-name` | a name has 1 to 200 characters and no control character | nazwa ma od 1 do 200 znaków i nie ma znaków sterujących |
| `reason-platform` | choose a PLC platform from the list | wybierz platformę PLC z listy |
| `reason-type-name` | a type name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore. It is not new or a primitive name | nazwa typu ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem. Nie może to być new ani nazwa typu prostego |
| `reason-member-name` | a member name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore | nazwa składnika ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem |
| `reason-member-rows` | each member row needs a name and a type. Clear both fields of a row that you do not want | każdy wiersz składnika wymaga nazwy i typu. Wyczyść oba pola wiersza, którego nie chcesz |
| `reason-member-count` | the form holds at most 1024 member rows | formularz ma najwyżej 1024 wiersze składników |
| `reason-flag` | the value must be 1. Reload the page and try again | wartość musi wynosić 1. Odśwież stronę i spróbuj ponownie |
| `reason-search-text` | the search text has at most 200 characters and no control character | szukany tekst ma najwyżej 200 znaków i nie ma znaków sterujących |
| `missing-malformed-machine` | Machine | Maszyna |
| `missing-malformed-type` | Type | Typ |
| `missing-type-revision` | Revision { $revision } of type { $name } of machine { $code } | Rewizja { $revision } typu { $name } maszyny { $code } |
| `platform-codesys-iec` | CODESYS (IEC 61131-3) | CODESYS (IEC 61131-3) |
| `platform-twincat3` | TwinCAT 3 | TwinCAT 3 |
| `platform-s7-1200` | Siemens S7-1200 | Siemens S7-1200 |
| `platform-s7-1500` | Siemens S7-1500 | Siemens S7-1500 |
| `platform-s7-300` | Siemens S7-300 | Siemens S7-300 |
| `column-code` | Code | Kod |
| `column-name` | Name | Nazwa |
| `column-platform` | PLC platform | Platforma PLC |
| `column-state` | State | Stan |
| `column-revision` | Revision | Rewizja |
| `column-members` | Members | Składniki |
| `column-recipe` | Recipe | Receptura |
| `column-version` | Version | Wersja |
| `column-tag` | Tag | Tag |
| `column-type` | Type | Typ |
| `pager-label` | Pages | Strony |
| `pager-previous` | Previous page | Poprzednia strona |
| `pager-next` | Next page | Następna strona |
| `pager-current` | Page { $page } | Strona { $page } |
| `machines-title` | Machines | Maszyny |
| `machines-caption` | Machines of the line | Maszyny linii |
| `machines-empty` | No machine matches. Archived machines show only with the filter. | Żadna maszyna nie pasuje. Zarchiwizowane maszyny widać tylko z filtrem. |
| `machines-search-submit` | Search | Szukaj |
| `machines-new-link` | New machine | Nowa maszyna |
| `machine-archived-toast` | Machine { $code } is archived. | Maszyna { $code } została zarchiwizowana. |
| `machine-new-title` | New machine | Nowa maszyna |
| `machine-code-hint` | M and a number from 1 to 9999999, for example M7. | M i liczba od 1 do 9999999, na przykład M7. |
| `machine-platform-hint` | The platform cannot change after the first recipe. | Platformy nie można zmienić po pierwszej recepturze. |
| `machine-main-unit-hint` | The system creates the main unit U1 with this name. | System tworzy jednostkę główną U1 o tej nazwie. |
| `machine-create-submit` | Create machine | Utwórz maszynę |
| `machine-title` | Machine { $code } | Maszyna { $code } |
| `machine-types-link` | Open the UDT types | Otwórz typy UDT |
| `machine-units-title` | Unit tree | Drzewo jednostek |
| `machine-rename-submit` | Rename machine | Zmień nazwę maszyny |
| `machine-renamed` | Machine { $code } has the new name. | Maszyna { $code } ma nową nazwę. |
| `machine-archive-summary` | Archive the machine | Archiwizuj maszynę |
| `machine-archive-warning` | The machine and its unit tree become read-only. You cannot undo this. The recipes stay readable, and a new machine can use the code { $code }. | Maszyna i jej drzewo jednostek będą tylko do odczytu. Tej operacji nie można cofnąć. Receptury pozostaną dostępne, a nowa maszyna może użyć kodu { $code }. |
| `machine-archive-submit` | Archive machine { $code } | Archiwizuj maszynę { $code } |
| `machine-archived-notice` | This machine is archived. The page is read-only. | Ta maszyna jest zarchiwizowana. Strona jest tylko do odczytu. |
| `machine-archived-at` | Archived on | Data archiwizacji |
| `tree-add-main-unit` | Add a main unit | Dodaj jednostkę główną |
| `tree-add-sub-unit` | Add a sub-unit | Dodaj podjednostkę |
| `tree-add-sub-unit-to` | Add a sub-unit to { $path } | Dodaj podjednostkę do { $path } |
| `unit-tree-title` | Unit tree of machine { $code } | Drzewo jednostek maszyny { $code } |
| `unit-new-title` | New unit of machine { $code } | Nowa jednostka maszyny { $code } |
| `unit-new-under` | New unit under { $path } | Nowa jednostka w { $path } |
| `unit-new-main` | New main unit | Nowa jednostka główna |
| `unit-parent-none` | None. The unit is a main unit. | Brak. Jednostka jest jednostką główną. |
| `unit-segment-hint` | A number from 1 to 64. The form proposes the first free number. | Liczba od 1 do 64. Formularz proponuje pierwszy wolny numer. |
| `unit-no-free-segment` | All numbers from 1 to 64 under this parent are in use. | Wszystkie numery od 1 do 64 w tej jednostce są zajęte. |
| `unit-create-submit` | Add unit | Dodaj jednostkę |
| `unit-added` | Unit { $path } is added. | Jednostka { $path } została dodana. |
| `unit-archived-toast` | Unit { $path } and its sub-units are archived. | Jednostka { $path } i jej podjednostki zostały zarchiwizowane. |
| `unit-title` | Unit { $path } | Jednostka { $path } |
| `unit-machine` | Machine | Maszyna |
| `unit-path` | Unit path | Ścieżka jednostki |
| `unit-children-title` | Sub-units | Podjednostki |
| `unit-children-empty` | The unit has no sub-unit. | Jednostka nie ma podjednostek. |
| `unit-rename-submit` | Rename unit | Zmień nazwę jednostki |
| `unit-renamed` | Unit { $path } has the new name. | Jednostka { $path } ma nową nazwę. |
| `unit-archive-summary` | Archive the unit | Archiwizuj jednostkę |
| `unit-archive-warning` | The unit and all its sub-units become archived. You cannot undo this. The system keeps their history. | Jednostka i wszystkie jej podjednostki zostaną zarchiwizowane. Tej operacji nie można cofnąć. System zachowa ich historię. |
| `unit-archive-count` | Sub-units that the system also archives: { $count }. | Podjednostki, które system także zarchiwizuje: { $count }. |
| `unit-archive-submit` | Archive unit { $path } | Archiwizuj jednostkę { $path } |
| `types-title` | UDT types of machine { $code } | Typy UDT maszyny { $code } |
| `types-caption` | Struct revisions | Rewizje struktur |
| `types-empty` | The machine has no UDT type. | Maszyna nie ma typów UDT. |
| `types-new-link` | New UDT type | Nowy typ UDT |
| `type-latest` | latest | najnowsza |
| `udt-new-title` | New UDT type | Nowy typ UDT |
| `udt-revision-title` | New revision of { $name } | Nowa rewizja { $name } |
| `udt-name-hint` | A letter or an underscore first, then letters, digits, and underscores. At most 64 characters. | Najpierw litera lub podkreślenie, potem litery, cyfry i podkreślenia. Najwyżej 64 znaki. |
| `udt-member-row` | Member { $number } | Składnik { $number } |
| `udt-member-name` | Name of member { $number } | Nazwa składnika { $number } |
| `udt-member-type` | Type of member { $number } | Typ składnika { $number } |
| `udt-type-hint` | A type expression, for example INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL. | Wyrażenie typu, na przykład INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL. |
| `udt-empty-rows-hint` | The system skips a row with an empty name and an empty type. Clear both fields of a member that you do not want. | System pomija wiersz z pustą nazwą i pustym typem. Wyczyść oba pola składnika, którego nie chcesz. |
| `udt-add-member` | Add a member | Dodaj składnik |
| `udt-max-rows` | The form holds the maximum of 1024 members. | Formularz ma maksymalną liczbę 1024 składników. |
| `udt-create-submit` | Create type | Utwórz typ |
| `udt-revision-submit` | Create revision | Utwórz rewizję |
| `type-title` | Type { $name }@{ $revision } | Typ { $name }@{ $revision } |
| `type-revisions-title` | Revisions | Rewizje |
| `type-tree-title` | Type tree | Drzewo typu |
| `type-supersedes` | Supersedes | Zastępuje |
| `type-usage-link` | Show the use of this revision | Pokaż użycie tej rewizji |
| `type-new-revision-link` | New revision | Nowa rewizja |
| `usage-title` | Use of { $name }@{ $revision } | Użycie { $name }@{ $revision } |
| `usage-caption` | Parameters that use the revision | Parametry, które używają rewizji |
| `usage-empty` | No parameter uses this revision. | Żaden parametr nie używa tej rewizji. |

Texts of the review fixes (D20). The English message of each code equals its row in `reliability.md` section 1.2, with `{ $name }` for each `{name}`.

| Key | en | pl |
|---|---|---|
| `error-E-PLT-001` | Machine code { $code } is already in use. Use a different code. | Kod maszyny { $code } jest już używany. Użyj innego kodu. |
| `error-E-PLT-002` | Unit path { $path } has more than 5 levels. Use at most 5 levels. | Ścieżka jednostki { $path } ma więcej niż 5 poziomów. Użyj najwyżej 5 poziomów. |
| `error-E-PLT-003` | The parent unit of { $path } does not exist. Create the parent unit first. | Jednostka nadrzędna dla { $path } nie istnieje. Najpierw utwórz jednostkę nadrzędną. |
| `error-E-PLT-004` | Unit { $path } already exists on this machine. | Jednostka { $path } już istnieje na tej maszynie. |
| `error-E-PLT-005` | Type { $type_name } contains itself. Delete the circular member. | Typ { $type_name } zawiera sam siebie. Usuń składnik, który tworzy cykl. |
| `error-E-PLT-006` | Member { $member } of { $type_name } uses a type that does not exist on { $platform }. | Składnik { $member } typu { $type_name } używa typu, który nie istnieje na platformie { $platform }. |
| `error-E-PLT-008` | Unit { $path } is the last main unit of machine { $code }. A machine needs one main unit. | Jednostka { $path } jest ostatnią jednostką główną maszyny { $code }. Maszyna potrzebuje jednej jednostki głównej. |
| `error-E-PLT-009` | Unit { $path } is archived. Select a live unit. | Jednostka { $path } jest zarchiwizowana. Wybierz aktywną jednostkę. |
| `error-E-PLT-010` | Machine { $code } is archived. | Maszyna { $code } jest zarchiwizowana. |
| `error-E-PLT-011` | Machine { $code } has a draft in recipe { $reference }. Approve or discard the draft first. | Maszyna { $code } ma wersję roboczą w recepturze { $reference }. Najpierw zatwierdź albo odrzuć wersję roboczą. |
| `error-E-PLT-012` | Unit { $path } is used by the draft of recipe { $reference }. Move or delete those parameters first. | Jednostka { $path } jest używana w wersji roboczej receptury { $reference }. Najpierw przenieś albo usuń te parametry. |
| `error-E-PLT-013` | The type { $type_expr } is not valid: { $reason }. Use a type such as INT, STRING(20), ST_Zone@3, or ARRAY[1..8] OF REAL. | Typ { $type_expr } jest niepoprawny: { $reason }. Użyj typu takiego jak INT, STRING(20), ST_Zone@3 lub ARRAY[1..8] OF REAL. |
| `error-E-PLT-014` | Unit { $path } is used by the approved version of recipe { $reference }. An Administrator can retire the version. | Jednostka { $path } jest używana w zatwierdzonej wersji receptury { $reference }. Administrator może wycofać tę wersję. |
| `error-E-PLT-015` | Type { $type_name } already exists on this machine. Create a new revision instead. | Typ { $type_name } już istnieje na tej maszynie. Zamiast tego utwórz nową rewizję. |
| `error-E-PLT-016` | Type { $type_name } has no members. Add at least one member. | Typ { $type_name } nie ma składników. Dodaj co najmniej jeden składnik. |
| `error-E-PLT-017` | { $text } is not a unit path. A unit path is U and 1 to 5 numbers from 1 to 64, with a dot between two numbers. Example: U1.2. | { $text } nie jest ścieżką jednostki. Ścieżka jednostki to U i od 1 do 5 liczb od 1 do 64, z kropką między dwiema liczbami. Przykład: U1.2. |
| `error-E-PLT-018` | Type { $type_name } is larger than the limit { $limit }. Use fewer elements, fewer members, or fewer levels. | Typ { $type_name } przekracza limit { $limit }. Użyj mniej elementów, mniej składników lub mniej poziomów. |
| `error-E-PLT-021` | Type { $type_name } has two members with the name { $member }. Give each member a different name. | Typ { $type_name } ma dwa składniki o nazwie { $member }. Nadaj każdemu składnikowi inną nazwę. |
| `type-reason-grammar` | the text does not match the type grammar | tekst nie pasuje do gramatyki typu |
| `type-reason-unknown-struct` | the struct or the revision does not exist on the machine | struktura albo rewizja nie istnieje na tej maszynie |
| `type-reason-not-on-platform` | the type does not exist on the platform of the machine | typ nie istnieje na platformie maszyny |
| `type-reason-out-of-range` | a bound, a dimension, or a length is outside its range | granica, wymiar albo długość jest poza zakresem |
| `type-reason-row-shape` | the stored type row does not match its kind | zapisany wiersz typu nie pasuje do swojego rodzaju |
| `type-limit-leaves` | { $count } leaves | { $count } elementów skalarnych |
| `type-limit-depth` | { $count } levels | { $count } poziomów |
| `type-limit-members` | { $count } members | { $count } składników |
| `field-platform-choose` | Choose a PLC platform | Wybierz platformę PLC |
| `list-page-after-end` | Page { $page } is after the last page. | Strona { $page } jest za ostatnią stroną. |
| `list-first-page` | Go to the first page | Przejdź do pierwszej strony |

- The key `field-query` of step P3.1 changes its text to "Search" (en) and "Wyszukiwanie" (pl). The old text gave "Field Search fields" and "Pole Pola wyszukiwania" (QA-10).
- `{ $count }` of `type-limit-*` and `{ $page }` get text, not a number, so the bundles add no group separator.
- The existing keys `operation-*`, `missing-machine`, `missing-machine-id`, `missing-unit`, `missing-type`, `field-q`, and `status-*` stay without a change.
- A test checks that each key of this table has the text of the table in both bundles. It lives in `i18n/plant_texts_tests.rs`, because `i18n/tests.rs` has 696 lines.

### D14. XSS and security

Each check has one owner:

- The area agents test each error code of their routes in its fragment with `HX-Retarget`, and the matrix rows of D12.
- H2 tests each route and each principal of `api.md` section 3.3 with the code in the body and the `forbidden` security event.
- W6 tests the XSS corpus, the `hx-*` rule, the CSRF token, and read-only mode.

The checks of W6:

- The corpus of `tests/web/xss-corpus.txt` goes into these fields: `code`, `name`, `main_unit_name`, `segment`, `parent_path`, `q`, `members[i].name`, `members[i].type`, `from`, `rev`, `page`, `show_archived`, `add`, and `archived`.
- It also goes into the path values `{code}`, `{path}`, `{name}`, and `{uuid}`, and into the field `next` of the language switch on each plant page.
- The test asserts the escaped text in `value` for a field that the server echoes into a form. An example is the rows of a `POST` with `add=1`.
- For a value that the server refuses, the test asserts the error code and no raw payload in the answer.
- A test reads every new page and fragment. Each `hx-*` value of an element, also a value that an element gets from an ancestor, is one of a fixed list of literals. `hx-headers` of `<body>` is the one exception.
- A test reads every URL of every answer: each `href`, `action`, `Location`, and the field `next`. None of them holds `csrf_token`.
- A test sends each new `POST` route without a CSRF token and gets `403 E-WEB-007`. In read-only mode each new `POST` route gets `503 E-WEB-006`.
- The tests of P3.1 cover the CSP, the inline script, the style, and the handlers of each page in the sweep. The sweep gets each new `GET` page in both languages.
- `E-PLT-010` of `RenameMachine` and `AddUnit` occurs only in a race. A test of the error mapping covers its status and fragment. The use case tests of `recipe-app` cover the race.

Queries:

- The first-statement test of `auth/first_statement.rs` covers the three form queries. The seven queries with `RecipeView` allow every role, so no test sees their check at run time.
- A source test reads each file of `plant/queries/`. Each `execute` body starts with `authorize(Self::OPERATION`. The code review checks the same rule.

### D15. Development server

- `crates/recipe-web/examples/dev_server.rs` builds `WebState` and `router` with `MemoryStore`, `MemorySink`, and `AppSettings::default()`.
- It listens on `127.0.0.1` and the port of the variable `RECIPE_DEV_PORT`, default 8080. It never binds another address.
- It seeds the users `admin`, `engineer`, and `operator` with the roles of their names and the password of the variable `RECIPE_DEV_PASSWORD`. Without the variable it stops with an error. It never logs the password.
- It makes a new signing key for the pre-session cookie at each start with `recipe_app::identity::random_nonce` or an equal source. A test checks that the key is not a literal.
- It seeds the primitive types and no machine. The test in a browser creates the machines.
- It moves the memory clock to the wall time every second. A frozen clock would keep a login delay and a session for ever.
- It makes a random key for the login HMAC too, so the example holds no key literal.
- `main` returns `Result`. A function that reads the environment takes the lookup as a closure, so a test needs no `env::set_var`. The example uses no `exit`, `print`, or `expect`.
- `Cargo.toml` of `recipe-web` gets `[[example]]` with `name = "dev_server"` and `test = true`, so `cargo test` runs the unit tests of the example.
- Chromium sends `Sec-Fetch-Site: same-origin` to `http://127.0.0.1`, so the CSRF origin check passes (`csrf.rs`, `origin_matches`). The test in a browser checks that Chromium keeps the `__Host-` cookie there.

### D16. Test plan of the required states

| State | Test |
|---|---|
| Empty list of machines, of types, and of usage rows | page test with the empty state sentence |
| Page after the end of the machine list | the text of `list-page-after-end`, a link to page 1, no empty state, and no next page (D20) |
| Tree of 5 levels and 64 units | `GET /machines/M7/units` on `seed_plant`: 64 unit links, the hidden groups of D7 |
| Path with 6 segments | `GET /machines/M7/units/U1.2.4.5.6.7` gives `422 E-PLT-002`. `POST` and `GET .../units/new` with the parent `U1.2.4.5.6` give `422 E-PLT-002` |
| All 64 segments in use | the unit form shows `unit-no-free-segment` and no create button |
| UDT with a cycle | `POST /machines/M7/types` on a store with a seeded cycle gives `422 E-PLT-005`. A new revision of `ST_Zone` with a member of the type `ST_Zone@1` also gives `422 E-PLT-005` (ADR-0038) |
| Each `POST` error code of `api.md` sections 2.2 and 2.3 | one HTMX test per code with `HX-Retarget: #error-panel` and the code in the fragment, except `E-PLT-010` (D14) |
| Each endpoint and each role | the area matrix rows of D12 and `routes/plant_access_tests.rs` of H2 |
| Each page in `pl` and `en` | the sweep of D14 |
| The content answer of D6 | one test per area: no top bar, the `title`, the out-of-band `next`, and the `h1` with `autofocus` |
| Line limits of the scripts | the tests of P3.1 stay |
| Contrast | no new token. The contrast test of P3.1 stays |
| Width of 320 CSS px | the test in a browser, in `pl`, on the tree of 5 levels |

### D17. Known changes of test expectations

The implementer reports each change with this reason. Any other change of an expectation needs its own reason.

| Test | Old expectation | New expectation | Reason |
|---|---|---|---|
| `routes::tests::expected_routes` and `matrix()` | the routes of P3.1 | also the static row of `components.css` and the rows of the three areas | D9, D12 |
| `routes::tests` count of mutating routes | `10` | the count of the mutating rows of the matrix | D12 |
| `pages::sweep_tests::PAGES` | a constant of 7 pages | a function with the pages of the three areas | D12 |
| `pages::xss_tests` query pages | read from `PAGES` | the `GET` rows of the route table that accept a query: the six pages of P3.1 and five plant pages | D12, D20 (U10) |
| `static_files` tests | 4 static files | 5 static files | D9 |
| `page_tests::assert_assets` and the layout tests | one style sheet | `app.css`, then `components.css` | D9, spec "Page layout" |
| Navigation tests | the links `/recipes`, `/users`, `/me` | also `/machines`, after `/recipes` | D11 |
| `error::reason` tests of `FieldReason::ALL` | 11 variants | 20 variants | D5 |
| `style_tests` that read `APP_CSS` for a component rule | `app.css` | `components.css` | D9 |
| The rule test of `.tree-node` | `width: 100%` | no width | D9, ADR-0037 (c) |
| The tests of the field `next` in `page_tests.rs` and `recipes/tests.rs` | `<input type="hidden" name="next" ...>` | the same input with `id="lang-next"` | D6, the out-of-band update of the content answer |
| A test that lists the headers of an HTMX error answer | `HX-Retarget` and `HX-Reswap` | also `HX-Push-Url: false` | D20 (U3), ADR-0037 (z) |
| A test of the text of `field-query` | "Search fields", "Pola wyszukiwania" | "Search", "Wyszukiwanie" | D20 (U31) |
| A test of the empty state on a page after the end | the text of the empty state | the text of `list-page-after-end` | D20 (U16) |
| A test of the platform list of the machine form | the five platforms | an empty first option, then the five platforms | D20 (U4) |
| `routes::plant_access_tests` | each route and each principal | the `forbidden` event and the code in the body. The area rows of the matrix keep each route and each principal | D20 (U19) |
| `types::tree_tests` leaf of the global `STRING` | `STRING(254)` | `STRING` | D20 (U14), task 13.15 |
| `path_values` test of a second value that is not UTF-8 | `MalformedMachine` for each route | `E-PLT-017` for `{path}`, `MalformedType` with `E-WEB-005` for `{name}` | D20 (U27, RR-5) |
| The tests of a page after the end of the machine list and of the usage list | a link to the previous page | no link to the previous page and no pager | D20 (RQ-5) |
| The style test of `.tree-group` | `margin-top: var(--target-gap)` | `margin-top: var(--space-4)` | D20 (RQ-1), ADR-0037 (c) |

### D18. Waves

At most two agents that run cargo work at the same time. A shared file has one owner in each wave. A wave starts after the green CI of the wave before it.

| Wave | Agent | Model | Tasks | File set |
|---|---|---|---|---|
| 1, first | H1 | Haiku | 3.1 | `recipe-web`: `i18n/en.ftl`, `i18n/pl.ftl`, `src/i18n/plant_texts_tests.rs`, the `mod` line in `src/i18n.rs` |
| 1, after H1 | A1 | Sonnet | 2.1 to 2.4 | `recipe-app`: `error/missing.rs`, `ports/plant.rs`, `ports/types.rs`, `ports/mod.rs`, `memory/plant/`, `memory/types/`, `plant/short_rows.rs`, `blocking.rs`, `plant/mod.rs`, `plant/queries/mod.rs`, `plant/queries/usage.rs`, `plant/queries/type_text.rs`, and their tests. `recipe-db`: `src/transaction.rs`, `src/transaction/`, `tests/storage/contract/`. `recipe-web`: `src/error.rs` (one `mod` line), `src/error/values.rs`, `src/error/value_tests.rs`, `src/error/reason.rs`, `src/error/reason_tests.rs` |
| 1, after H1 | R1 | Sonnet | 4.1 to 4.3 | `recipe-web`: `static/app.css`, `static/components.css`, `src/static_files.rs` and its tests, `tests/web/static_files.rs`, `templates/base.html`, `src/style_tests.rs`, `src/style_tests/`, `src/pages/page_tests.rs`, `src/routes.rs`, `src/routes/tests.rs`, `src/routes/matrix.rs`, `src/pages.rs`, `src/pages/{machines,units,types}.rs` and their test files, `src/pages/sweep_tests.rs`, `src/pages/xss_tests.rs`, `src/test_support.rs`, `src/test_support/plant.rs` |
| 2 | A2 | Sonnet | 5.1 to 5.4 | `recipe-app`: `plant/mod.rs`, `plant/queries/` except `usage.rs` and `type_text.rs`, `auth/first_statement.rs` |
| 2 | W1 | Sonnet | 6.1 to 6.4 | `recipe-web`: `templates/base.html`, `src/layout.rs`, `src/layout/tests.rs`, `src/pages/request.rs`, `src/pages/render.rs`, `src/pages/path_values.rs`, `src/pages.rs` (one `mod` line), `templates/components/`, `templates/test/`, `static/components.css`, `src/style_tests/component_rules.rs`, `src/components_tests.rs`, `src/components_tests/`, `src/error.rs`, `src/error/view.rs`, `src/error/answer.rs`, `src/error/row_field_tests.rs`, `templates/field-errors.html` |
| 3 | W2 | Sonnet | 7.1 to 7.4 | `recipe-web`: `src/layout.rs`, `src/layout/tests.rs`, `src/pages/page_tests.rs`, `src/pages/machines.rs`, `src/pages/machines/`, `templates/machines.html`, `templates/machine-new.html`, `templates/machine.html`, `templates/unit-tree.html`, `templates/unit-tree-page.html` |
| 3 | W3 | Sonnet | 8.1 to 8.4 | `recipe-web`: `src/pages/types.rs`, `src/pages/types/`, `templates/types.html`, `templates/udt-form.html`, `templates/type.html`, `templates/type-usage.html` |
| 4 | W4 | Sonnet | 9.1 to 9.4 | `recipe-web`: `src/pages/units.rs`, `src/pages/units/`, `src/pages/machines.rs`, `src/pages/machines/commands.rs`, `templates/unit-form.html`, `templates/unit-form-page.html`, `templates/unit.html`, `templates/unit-added.html`, `templates/unit-renamed.html`, `templates/machine-renamed.html` |
| 4 | W5 | Sonnet | 10.1 | `recipe-web`: `examples/dev_server.rs`, `Cargo.toml` (`[[example]]` only) |
| 5 | W6 | Sonnet | 11.1 to 11.3 | `recipe-web`: `src/pages/plant_xss_tests.rs`, `src/pages/plant_sweep_tests.rs`, `src/pages/plant_security_tests.rs`, `tests/web/xss-corpus.txt`, three `mod` lines in `src/pages.rs` |
| 5 | H2 | Haiku | 11.4 | `recipe-web`: `src/routes/plant_access_tests.rs`, one `mod` line in `src/routes.rs` |

- A1 starts with task 2.1, because its tests read the keys of H1.
- Wave 3 reads `src/pages.rs`, `src/routes.rs`, `src/test_support/`, and the components, but does not change them. An agent that needs a change there reports it.
- W2 renders the rename form and the archive form on the machine page. W4 adds their `POST` routes in wave 4.
- W4 uses the tree view model of W2 with its parameter `focus` and does not change it.
- The texts of D13 exist after wave 1. A later agent that needs a new key reports it, and the orchestrator adds it to both bundles and to D13.

### D19. Signatures

```rust
// recipe-app, ports
pub struct MachineFilter { pub text: Option<Text>, pub include_archived: bool }
pub struct MachineSummary { pub id: MachineId, pub code: MachineCode, pub name: Text,
                            pub platform: PlcPlatform, pub archived_at: Option<Timestamp> }
pub struct TypeUsageRow { pub reference: ReferenceNumber, pub number: u32, pub status: VersionStatus,
                          pub tag: ParameterTag, pub parameter_type: PlcTypeId }
// PlantRepository
async fn list_machines(&mut self, filter: &MachineFilter, offset: u64, limit: u32)
    -> Result<Vec<MachineSummary>, StorageError>;
// TypeCatalog
async fn type_usage(&mut self, machine: MachineId, type_id: PlcTypeId, offset: u64, limit: u32)
    -> Result<Vec<TypeUsageRow>, StorageError>;

// recipe-app, settings
pub struct PageSizes { pub machines: u32, pub type_usage: u32 }   // Default: 100, 100
// AppSettings gets the field `pub pages: PageSizes`.

// recipe-app, query inputs and outputs. `page` counts from 0.
pub enum MachineRef { Code(MachineCode), Id(MachineId) }
pub struct ListMachinesInput { pub filter: MachineFilter, pub page: u32, pub archived: Option<MachineCode> }
pub struct MachineList { pub rows: Vec<MachineSummary>, pub page: u32, pub has_next: bool,
                         pub archived: Option<MachineCode> }
pub struct GetMachineInput { pub machine: MachineRef, pub archived: Option<UnitPath> }
pub struct MachineView { pub machine: Machine, pub archived: Option<UnitPath> }
pub struct MachineForm { pub platforms: [PlcPlatform; 5] }            // GetMachineForm takes no input
pub struct UnitTreeInput { pub code: MachineCode }                     // GetUnitTree returns Machine
pub struct UnitFormInput { pub code: MachineCode, pub parent: Option<UnitPath> }
pub struct UnitForm { pub machine: Machine, pub parents: Vec<Unit>, pub parent: Option<UnitPath>,
                      pub segment: Option<u8> }
pub struct UnitRef { pub code: MachineCode, pub path: UnitPath }
pub struct UnitView { pub machine: Machine, pub unit: Unit, pub parent: Option<Unit>,
                      pub children: Vec<Unit>, pub descendants: usize }
pub struct TypesInput { pub code: MachineCode }
pub struct TypeList { pub machine: Machine, pub rows: Vec<TypeListRow> }
pub struct TypeListRow { pub id: PlcTypeId, pub name: TypeName, pub revision: u32,
                         pub members: usize, pub latest: bool }
pub struct TypeRef { pub code: MachineCode, pub name: TypeName, pub revision: Option<u32> }
pub struct TypeView { pub machine: Machine, pub id: PlcTypeId, pub name: TypeName, pub revision: u32,
                      pub revisions: Vec<u32>, pub supersedes: Option<u32>, pub tree: TypeTree }
pub struct UdtFormInput { pub code: MachineCode, pub from: Option<TypeName> }
pub struct UdtForm { pub machine: Machine, pub from: Option<(TypeName, u32)>,
                     pub members: Vec<(MemberName, String)>, pub choices: Vec<String> }
pub struct TypeUsageInput { pub type_ref: TypeRef, pub page: u32 }
pub struct TypeUsage { pub machine: Machine, pub name: TypeName, pub revision: u32,
                       pub rows: Vec<TypeUsageLine>, pub page: u32, pub has_next: bool }
pub struct TypeUsageLine { pub row: TypeUsageRow, pub type_text: String }

// Each query: new(ctx: AppContext) and
// async fn execute(&self, user: &CurrentUser, meta: &RequestMeta, input: <Input>) -> Result<<Output>, AppError>

// recipe-web
pub(crate) enum RowField { Name, Type }
// WebError::BadRow { row: u16, field: RowField, reason: FieldReason }  -> E-WEB-008
pub(crate) fn see_other_path(location: &str, cookies: impl IntoIterator<Item = HeaderValue>)
    -> Result<Response, WebError>;
```

- The implementer may rename a field of these structs. The implementer keeps the shapes and reports each change.

### D20. Fixes of the code review

The code review of step 6 gave the findings U1 to U34. The user chose the fixes on 2026-10-15. Each fix starts with a failing test.

Errors and answers:

- U1. Each code of `api.md` sections 2.2 and 2.3 gets `error-E-PLT-<n>` (D13). `args_of` gets the fields of `PlantError`. A code, a path, a name, a reference, and a typed text stay data. `platform` comes from `platform-*`, `reason` from `type-reason-*`, and `limit` from `type-limit-*`.
- U3. Each HTMX error answer that sets `HX-Retarget` also sets `HX-Push-Url: false` (ADR-0037 (z)).
- U27. A malformed `archived` of `/machines` keeps `422 E-WEB-008` (ADR-0037 (x)). A `{path}` that is not UTF-8 gives `422 E-PLT-017`, as `api.md` section 1 says. The rejection of axum keeps no bytes, so the text of the error is the replacement character U+FFFD.
- U17. A `q` of spaces only is no search text.
- U18. `Texts::with_arg` moves to `texts.rs`. The key test also reads the literal keys of `.get("`, `.with("`, and `.with_arg("` in each `.rs` file under `src/`.

Pages and style:

- U2. The full page of `GET /machines/{code}/units` holds `#unit-tree` and an empty `#unit-form`, as the machine page does.
- U4. The platform list starts with an empty option with the text of `field-platform-choose`. The option is selected when the form holds no platform. The server check of `reason-platform` stays.
- U5. `.target-link` gets `gap: var(--space-2)`, so the path and the name have a space on the screen.
- U6. The lists of links on the unit page (the parent and the sub-units) are flex columns with `gap: var(--target-gap)`.
- U7 and U32. A wrapped tree row starts its add link at the column of the unit path. Two rows keep the gap `--space-4`, and two targets of one row keep `--target-gap`. The user chose this on 2026-10-15, so `design-system.md` section 10.8 and ADR-0037 (c) changed too. A member row puts the type input below the name input at a small width.
- U32 is met only in part. At 320 CSS px a long type expression scrolls inside its input, because the padding of `design-system.md` section 6 stays. The user accepted this on 2026-10-15.
- U8. A field in `.actions` gets `min-width: 0` and a flexible width, so the search input stays in its panel at 320 CSS px.
- U9. `fields::named` gets a parameter `value`. A rename input holds the current name.
- U15. A link with `aria-current="page"` in the revision list has its own look. The revision links use the face of data. The type tree shows the member type in the muted data face, apart from the member name.
- U16. The large badge and the notice, and the table and the pager, have the gap `--space-4`. The empty state inside a table has no own border. A page after the last page shows `list-page-after-end` and a link to page 1 with the text of `list-first-page`, on `/machines` and on the usage page.

Domain and queries:

- U28. `recipe-domain` gets a check of the struct names of a new struct (ADR-0038). `DefineUdt` runs it on the type tree of the new struct. A struct name that occurs twice on one path from the root gives `TypeCycle` with that name. `TypeTree::resolve()` does not change.
- U30. `GetUdtForm` with `from` leaves every revision of that struct out of the type proposals. The answer of `add=1` for a revision does the same.
- U11. `page_window`, `missing_type`, `live_machine`, and the revision lookup of `recipe-app` exist once in `plant/queries/mod.rs`.
- U20. The CTE `edges` of `type_usage` uses `UNION ALL`. The `UNION` of `holders` keeps the end of a cycle.
- U21. One helper of `recipe-db` turns an offset into a bind value for `machine_list`, `type_usage`, and the user list. A unit test covers its error. The port documents name the error, and the memory adapters return the same error.

Refactors and tests:

- U12. One `row` helper (`operation_row` of `routes.rs`), one `query_values` in `path_values.rs`, and one `instant`. `rows()` of `pages/machines.rs` has at most 50 lines.
- U13. One plant test world in `test_support/plant.rs` serves the tests of the three areas. The helpers that the XSS and sweep tests copy (`CORPUS`, `payloads`, `encode`, `askama_escape`, `CSP`, `assert_error`, the `hx-*` reader) live in `pages/test_helpers.rs`.
- U14. The type tree page writes a member type with `type_text` of `recipe-app`. No second writer exists.
- U10. The test of the pages without a query extractor reads the route table. The field `next` of each plant page that accepts a query gets the XSS corpus.
- U19. The refused-value tests assert the exact code. The pager test checks the link to page 2. No assertion compares two literals. `plant_access_tests` keeps the `forbidden` event and the code in the body. The area rows of the matrix keep each route and each principal.
- U31. `field-query` gets the texts of D13.

Deferred: U22 and U23 to P4.2, U25 and U33 to P3.3, U26 and U29 to P4.2. No change: U24, U34.

Fixes of the second check (wave F5). A browser check (RQ) and a review of the fixes (RR) found one MEDIUM and twelve LOW items. The user chose them on 2026-10-15:

- RQ-1. `.tree-group` gets the top gap `--space-4`, so a parent row and its first child row keep the gap of two rows (ADR-0037 (c)).
- RQ-2. The platform list is a required field and shows the mark and `aria-required` of `design-system.md` section 10.5.
- RQ-4. A link in `.link-list` keeps its own width.
- RQ-5. A page after the end shows no link to the previous page.
- RR-1. The page offset uses `checked_mul`, as `rust.md` requires.
- RR-2. The memory adapter of the user list returns the storage error of U21 for an offset above `i64::MAX`, and the port document names it.
- RR-3. The key test of the templates also reads `t.with_arg("`.
- RR-4. A malformed `{code}` gives `404 E-WEB-005` before the check of `{path}`.
- RR-5. A `{name}` that is not UTF-8 gives `Missing::MalformedType`.
- RR-6. The samples of `error/plant_args_tests.rs` are a table, so no function has more than 50 lines.
- RR-7. HTTP tests cover the proposals of the form of a new revision and `E-PLT-005` of a revision that holds an earlier revision of itself.
- RR-8 and RR-9. A stale template comment and a wrong permission name in a test document change.
- Deferred: RQ-3 to P4.2, because a proposal list without the structs that contain the revised struct needs an ADR. No change: RQ-6.
- RR-4 is not done. With a `{path}` that is not UTF-8, axum 0.8.9 keeps no other path value, so `/machines/X7/units/%FF` gives `422 E-PLT-017` and not `404 E-WEB-005`. A fix needs the handlers to read the raw URI. A test pins this limit, and the document of `path_code_unit` names it. Step P4.2 decides it.

### D21. Waves of the review fixes

The rules of D18 apply. A wave starts after the green CI of the wave before it.

| Wave | Agent | Model | Fixes | File set |
|---|---|---|---|---|
| F1, first | H3 | Haiku | 13.1 (U1 texts, U4, U16, U31 keys) | `recipe-web`: `i18n/en.ftl`, `i18n/pl.ftl`, `src/i18n/review_texts_tests.rs`, the `mod` line in `src/i18n.rs` |
| F1 | D1 | Sonnet | 13.2 to 13.4 (U28, U30, U11, U20, U21) | `recipe-domain`: `src/plant/` and its tests. `recipe-app`: `src/plant/define_udt.rs`, `src/plant/queries/`, `src/ports/plant.rs`, `src/ports/types.rs`, `src/memory/plant/`, `src/memory/types/` and their tests. `recipe-db`: `src/transaction/`, `src/transaction.rs`, `src/identity/users.rs`, `tests/storage/contract/` |
| F2 | W7 | Sonnet | 13.5 to 13.7 (U1, U3, U27, U18) | `recipe-web`: `src/error.rs`, `src/error/`, `src/i18n/plant_error_tests.rs` and its `mod` line in `src/i18n.rs`, `src/pages/path_values.rs`, `src/pages/path_values/`, `src/pages/texts.rs`, `src/pages/texts/`, `src/pages/render.rs` |
| F2 | W8 | Sonnet | 13.8 to 13.11 (U2, U4 to U9, U15 to U17, U32) | `recipe-web`: `templates/` except the error templates, `static/app.css`, `static/components.css`, `src/style_tests.rs`, `src/style_tests/`, `src/components_tests.rs`, `src/components_tests/`, `src/pages/machines/` except `test_world.rs`, `src/pages/units/page.rs`, `src/pages/units/page_tests.rs`, `src/pages/types/usage.rs`, `src/pages/types/usage_tests.rs` |
| F3 | W9 | Sonnet | 13.12 to 13.14 (U10, U12 to U14, U19) | `recipe-web`: `src/pages/{machines,units,types}.rs`, `src/routes.rs`, `src/routes/`, `src/pages/path_values.rs`, every test world and test file of the three areas, `src/pages/plant_*_tests.rs`, `src/pages/xss_tests.rs`, `src/pages/sweep_tests.rs`, `src/pages/test_helpers.rs`, `src/test_support.rs`, `src/test_support/`, `src/pages/types/tree.rs` |

- H3 starts first in F1, because W7 and W8 read its keys. D1 starts at the same time, because its files are apart.
- W8 owns `machines/list.rs` for U16 and U17.
- W9 runs alone, because it moves the test worlds of every area.
- Wave F5 has two agents. W11 (Sonnet) does task 13.16 on `templates/` except the error templates, `static/app.css`, `static/components.css`, `src/style_tests/`, `src/components_tests/`, `src/pages/machines/list.rs`, `src/pages/machines/list_tests.rs`, `src/pages/machines/form_tests.rs`, `src/pages/types/usage.rs`, and `src/pages/types/usage_tests.rs`. W12 (Sonnet) does tasks 13.17 to 13.19 on `recipe-app` `src/plant/queries/mod.rs`, `src/memory/identity/`, `src/ports/user.rs`, `recipe-db` `src/transaction/identity/users.rs` and its tests, and `recipe-web` `src/pages/path_values.rs`, `src/pages/path_values/`, `src/pages/texts/tests.rs`, `src/error/plant_args_tests.rs`, `src/pages/types/define_tests.rs`, `src/pages/types/form_tests.rs`, `src/routes/plant_access_tests.rs`.
- Wave F4 has one agent, W10 (Sonnet), with task 13.15 (U14). Its file set is `recipe-app` `src/plant/queries/types.rs`, `src/plant/queries/type_text.rs`, and their tests, and `recipe-web` `src/pages/types/tree.rs`, `templates/type.html`, and `src/pages/types/tree_tests.rs`. U14 needs a member text from `GetTypeTree`, and the file set of W9 had no file of `recipe-app`.

## Risks / Trade-offs

- [The refactor of the route table in wave 1 breaks a test of P3.1] → R1 keeps the existing rows. It changes only the way the table and the tests collect them. The full CI runs after wave 1.
- [Two agents of one wave need the same shared file] → the table of D18 names one owner. The other agent reports the needed entry.
- [`lower()` of PostgreSQL and `to_lowercase` of Rust differ for a rare character] → the contract test covers the Polish letters. The difference changes only a search result, never stored data.
- [The recursive usage query is slow on a large catalog] → the three indexes of D4 serve it, and `db.statement_timeout` stops it. The page shows 100 rows.
- [The type tree of a struct with many nested members is long] → the tree shows the element type of an array once. The server expands only the root group.
- [`autofocus` after an HTMX swap does not move the focus in a browser] → the browser test checks it. The toast still announces the result.
- [HTMX does not push the URL after a `303`] → the browser test checks the address bar after each create and archive.
- [`examples/dev_server.rs` drifts from the real server] → it holds no production setting. Step P4.1 replaces its use.

## Migration Plan

No migration. The release that holds this change serves two style sheets. A browser with the old `app.css` in its cache gets the new file after `static.cache_max_age`.

## Open Questions

None. Each question of the propose step and of the plan review has an answer in D1.
