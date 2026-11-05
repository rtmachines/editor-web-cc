# HTTP API

Status: Accepted (design stage, revised 2026-10-12 after review)
Source of requirements: `general-draft.txt`
Related: `domain-model.md`, `data-model.md`, `reliability.md`, ADR-0001, ADR-0008, ADR-0013, ADR-0014, ADR-0015, ADR-0016, ADR-0018, ADR-0019, ADR-0021, ADR-0022, ADR-0109

## 1. Scope

This document defines the HTTP surface of `recipe-web`. It is a hypermedia API (ADR-0001).

- A full page request returns `text/html`.
- An HTMX request (`HX-Request: true`) returns an HTML fragment.
- The export endpoints return `application/json`. No other public endpoint returns JSON.
- The five export endpoints use two schemes. Two use the session cookie. Three use HTTP Basic (section 2.7).
- The health endpoints live on a separate internal listener (section 2.10). They are not part of the public surface.

URL policy:

- HTML routes have no version prefix. Example: `/recipes/R-100/versions/3`.
- The JSON export lives under `/export/v1/` and `/export/v2/`. A breaking change of the export document creates the next prefix (ADR-0019).
- The unit scope document breaks v1, so it lives under `/export/v2/` (section 6.4, ADR-0109 (aa)).

Path values are the human identifiers from `domain-model.md`. The table gives the value object for each path value and the error for a bad value. The rule is in `reliability.md` 4.2. A path value that is a domain value object with its own error code returns that code with 422. A path value that names a resource returns `404 E-WEB-005` when the resource does not exist, also when the text cannot name one. The exemption to this rule follows the table.

| Path value | Value object | Bad value |
|---|---|---|
| `{code}` | `MachineCode` | `404 E-WEB-005` |
| `{uuid}` | `MachineId` (UUID) | `404 E-WEB-005` |
| `{path}` | `UnitPath` | `422 E-PLT-017` for a malformed text or segment. `422 E-PLT-002` for more than 5 segments. A valid path of a unit that does not exist: `404 E-WEB-005` |
| `{name}` | `TypeName` | `404 E-WEB-005` |
| `{ref}` | `ReferenceNumber` | `404 E-WEB-005` |
| `{n}`, `{m}` | version number | `404 E-WEB-005` |
| `{key}` | `ParameterKey` (UUID) | `404 E-WEB-005` for a malformed key (not a UUID), a key of a version that does not exist, or a key that a query does not find. `404 E-RCP-051` for a well-formed key that the existing draft does not hold |
| `{lang}` | `LangCode` | `422 E-RCP-027` for a malformed code. `404 E-WEB-005` for a code of a version that does not exist. `404 E-RCP-050` for a well-formed code that the existing draft does not hold. `422 E-WEB-008` for a code outside `i18n.translation_langs` on the route that writes a translation (ADR-0043) |
| `{login}` | `Login` | `404 E-WEB-005` |
| `{id}` | audit entry id | `404 E-WEB-005` |

No path value holds an S7 address.

Exemption from the `E-WEB-005` rule. `E-WEB-005 NotFound` is for an unknown `{ref}`, `{n}`, `{code}`, or `{login}`, a malformed `{key}` (not a UUID), and a `{key}` or `{lang}` of a version that does not exist. `E-RCP-051 ParameterNotFound` and `E-RCP-050 TranslationNotFound` are for a well-formed `{key}` or `{lang}` that is absent from an existing draft. The draft commands of section 2.6 return them. The two DELETE rows of section 2.6 cite both codes with this rule.

Query values. A query value of a `GET` route can name a resource that the route needs. It then follows the rule of the path value with the same value object, for example `parent_path`, `from`, and `rev` of section 2 (ADR-0037). A query value that only selects a message, such as `archived`, is a request field. A malformed value gives the code of its value object, or `422 E-WEB-008` when the value object has no code. The audit filter `actor` of section 2.9 is an exception (ADR-0051 (d)).

Request fields. `E-WEB-005` is for URL values only. A request field that names a machine that does not exist returns `404 E-PLT-019 MachineNotFound`, for example `machine_code` of `CreateRecipe`. A request field that names a unit that does not exist returns `404 E-PLT-020 UnitNotFound`, for example `unit_path` of `AddParameter`. A missing parent in `AddUnit` returns `422 E-PLT-003 ParentNotFound`.

Mutating requests use `POST`. No route uses `PUT` or `DELETE`, because a form sends only `GET` and `POST` (ADR-0040). Every mutating request carries the CSRF token (section 7). Every mutating request on a `Draft` carries `row_version` (section 4).

Line ends. A browser sends each line end of a `textarea` as CRLF. `recipe-web` converts each CRLF of the fields `description_en`, `description`, and `change_note` to LF before the parse (ADR-0112). A lone CR stays, and the parse refuses it.

The use case names in the tables are the commands from `domain-model.md` section 5 and ADR-0016, and the queries of `domain-model.md` 5.5. A query is also a use case. It checks its permission first, in the same way as a command (R28).

Every `GET` row of the tables of section 2 names its query use case, except the export rows. Four of them run the command `ExportVersion`, and one runs the query `ListExportableRecipes`. The row `GET /recipes/{ref}/compare` names no use case, because it only redirects to the diff route, which runs `DiffVersions` (ADR-0108 (e)). A form or preview query has the permission of the command that it prepares.

## 2. Endpoints

Columns: Method, Path, Use case, Permission, Request, Success, Errors. The error column lists the HTTP status and the error code from `reliability.md` section 1.2. Every endpoint can also return these codes:

- `401 E-WEB-004` (no session), `403 E-WEB-002` (no permission), `403 E-WEB-007` (CSRF). The three transfer routes of section 2.7 return `401 E-WEB-012` or `401 E-IDN-003` in place of `E-WEB-004`.
- `413 E-WEB-003` (request too large), `422 E-WEB-008` (malformed field).
- `503 E-WEB-006` (read-only mode), `503 E-WEB-009` (timeout), `500 E-WEB-010` (internal error), `500 E-WEB-011` (a `spawn_blocking` task failed).
- The storage codes `503 E-DB-001`, `503 E-DB-002`, `409 E-DB-003`, `503 E-DB-004`, `503 E-DB-005`, `409 E-DB-006`, `409 E-DB-007`, `500 E-DB-008`.
- The code of a bad path value (section 1).

The tables do not repeat these codes. In read-only mode every command route returns `E-WEB-006`, except `POST /login`, `POST /logout`, and the five export routes of section 2.7 (section 5.1). In that mode a page shows no control of a blocked command. A form route shows a notice in place of the form (`design-system.md` section 10.1, ADR-0113).

### 2.1 Auth and language

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/login` | `GetLoginForm` | none | - | `200` login page. Sets the pre-session CSRF cookie (section 7.3) | - |
| POST | `/login` | `Login` | none | `login`, `password`, `csrf_pre` | `303` to `/recipes`. `303` to `/me/password` when `must_change_password` is set. Sets the session cookie | `401 E-IDN-003`, `429 E-IDN-005`, `303 E-IDN-007` (the redirect to `/me/password`), `E-IDN-004` (only in the event log and `security_event`. The client gets `401 E-IDN-003`) |
| POST | `/logout` | `Logout` | authenticated | CSRF token | `303` to `/login`. Deletes the session row and clears the cookie | - |
| POST | `/lang` | - (sets the cookie `recipe_lang`, section 8) | none | `lang` in `pl` or `en`, `csrf_pre` or CSRF token, `next` | `303` to `next` when `next` is a same-origin relative path. Otherwise `303` to `/recipes`. Sets the cookie `recipe_lang`. Works without a session and in read-only mode | - |

The response body and the response time of `POST /login` are the same for an unknown login, an inactive user, and a wrong password (section 7.4, A42).

Static files. `GET /static/*` serves the files of section 9, the style sheets `static/app.css` and `static/components.css` (ADR-0037), and the 8 font files of `design-system.md` section 5 under `/static/fonts/`. The route needs no session and has no use case. It is outside the `must_change_password` gate of section 7.4. The response sets `Cache-Control: public, max-age` with the value of `static.cache_max_age` (`reliability.md` 7.11).

### 2.2 Machines and units

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines` | `ListMachines` | `RecipeView` | `q`, `page`, `show_archived`, optional `archived` | `200` list of live machines, `page.machines` rows per page. `show_archived=1` adds the archived machines, each with a link to `/machines/by-id/{uuid}` | - |
| GET | `/machines/new` | `GetMachineForm` | `MachineManage` | - | `200` form | - |
| POST | `/machines` | `CreateMachine` | `MachineManage` | `code`, `name`, `plc_platform`, `main_unit_name` | `303` to `/machines/{code}` | `409 E-PLT-001` |
| GET | `/machines/{code}` | `GetMachine` | `RecipeView` | optional `archived` | `200` page of the live machine with this code, with the unit tree | `404 E-WEB-005` |
| GET | `/machines/by-id/{uuid}` | `GetMachine` | `RecipeView` | - | `200` machine page of a live or an archived machine, with the unit tree. Read-only for an archived machine | `404 E-WEB-005` |
| POST | `/machines/{code}/rename` | `RenameMachine` | `MachineManage` | `name` | `200` fragment. A request without `HX-Request` gets `303` to `/machines/{code}` | `404 E-WEB-005`, `409 E-PLT-010` |
| POST | `/machines/{code}/archive` | `ArchiveMachine` | `MachineManage` | CSRF token | `303` to `/machines?archived={code}` (ADR-0037) | `404 E-WEB-005`, `409 E-PLT-011` |
| GET | `/machines/{code}/units` | `GetUnitTree` | `RecipeView` | - | `200` fragment of the whole unit tree. A request without `HX-Request` gets the page of the tree (ADR-0037) | `404 E-WEB-005` |
| GET | `/machines/{code}/units/new` | `GetUnitForm` | `UnitManage` | optional `parent_path` | `200` form | `404 E-WEB-005` |
| POST | `/machines/{code}/units` | `AddUnit` | `UnitManage` | `parent_path` (empty for a main unit), `segment`, `name` | `200` tree fragment with the new unit as the focus unit. A request without `HX-Request` gets `303` to `/machines/{code}` (ADR-0037) | `404 E-WEB-005`, `422 E-PLT-002`, `422 E-PLT-003`, `409 E-PLT-004`, `409 E-PLT-009`, `409 E-PLT-010`, `422 E-PLT-017` |
| GET | `/machines/{code}/units/{path}` | `GetUnit` | `RecipeView` | - | `200` unit detail | `404 E-WEB-005` |
| POST | `/machines/{code}/units/{path}/rename` | `RenameUnit` | `UnitManage` | `name` | `200` fragment. A request without `HX-Request` gets `303` to `/machines/{code}/units/{path}` | `404 E-WEB-005`, `409 E-PLT-009` |
| POST | `/machines/{code}/units/{path}/archive` | `ArchiveUnit` | `UnitManage` | CSRF token | `303` to `/machines/{code}?archived={path}`. The unit and its descendants are archived (ADR-0037) | `404 E-WEB-005`, `409 E-PLT-009`, `409 E-PLT-012`, `409 E-PLT-014`, `409 E-PLT-008` |

`q` of `ListMachines` is a `Text`. It matches a part of the code or of the name, with no difference between upper and lower case. The rows come in the order of the code number. `page` starts at 1 (ADR-0037).

A malformed `parent_path` of `GetUnitForm` gives the code of `UnitPath` (section 1). A valid `parent_path` of a unit that is not live gives `404 E-WEB-005`. A live parent of depth 5 gives `422 E-PLT-002`, because its child would have six levels.

`archived` holds the code or the unit path of the last archive. The page shows a success message only when the query finds that machine or unit archived (ADR-0037).

`/machines/{code}` and every route under it resolve to the live machine with that code. An archived machine keeps its history at `/machines/by-id/{uuid}`, because a new machine can use its code (A32, ADR-0021).

`ArchiveMachine` is blocked while a recipe of the machine has a live `Draft` (`E-PLT-011`). The recipes stay readable. `ArchiveUnit` is blocked while the live `Draft` of a recipe references the unit or a descendant (`E-PLT-012`). It is also blocked while the `Approved` version of a recipe references them (`E-PLT-014`), and when the unit is the last live main unit (`E-PLT-008`). A unit that is already archived gives `E-PLT-009` (ADR-0029).

### 2.3 Types

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines/{code}/types` | `ListTypes` | `RecipeView` | - | `200` list of UDTs, one row per revision. Inside one name the latest revision comes first | `404 E-WEB-005` |
| GET | `/machines/{code}/types/new` | `GetUdtForm` | `UdtManage` | optional `from` (ADR-0037) | `200` form. With `from`, the form of a new revision of `from` with the members of its latest revision | `404 E-WEB-005` |
| POST | `/machines/{code}/types` | `DefineUdt` | `UdtManage` | `name`, `members[i].name`, `members[i].type` (a `TypeExpr`), optional `add` (ADR-0037) | `303` to the new type. With `add=1`, the query `GetUdtForm` runs in place of `DefineUdt`. Then `200` with the form, the sent rows, and one more empty row | `422 E-PLT-005`, `422 E-PLT-006`, `422 E-PLT-013`, `409 E-PLT-015`, `422 E-PLT-016`, `422 E-PLT-018`, `422 E-PLT-021` |
| GET | `/machines/{code}/types/{name}` | `GetTypeTree` | `RecipeView` | optional `rev` | `200` type tree of the latest revision, or of `rev`. A machine on `S7_300`, `S7_1200`, or `S7_1500` also gets the S7 layout (ADR-0108 (h)) | `404 E-WEB-005` |
| POST | `/machines/{code}/types/{name}/revisions` | `DefineUdt` | `UdtManage` | same as `POST /types`. The server sets `supersedes` | `303` to the new revision | same as `POST /types` |
| GET | `/machines/{code}/types/{name}/usage` | `GetTypeUsage` | `RecipeView` | optional `rev`, `page` | `200` list of recipes and parameters that use the revision, directly or through another type. `page.type_usage` rows per page | `404 E-WEB-005` |

A malformed `from` gives `404 E-WEB-005`, as a malformed `{name}` does. A `rev` that is not a revision number, or a revision that does not exist, gives `404 E-WEB-005`. The usage list shows the versions `Draft`, `Approved`, and `Retired` of the recipes that are not deleted (ADR-0037).

The struct name `new` gives `422 E-WEB-008`, because the route `/types/new` would hide the page of the struct (ADR-0037).

A UDT is immutable. A second `POST` creates a new revision (ADR-0006). The usage query is the "where used" view that the rebind workflow needs (ADR-0018).

The type page of a machine on `S7_300`, `S7_1200`, or `S7_1500` shows the table S7 layout after the type tree (ADR-0108 (h)). The table has one row per struct member in the order of the tree. An array is one row. Each row has the path, the type, and the offset as `<byte>.<bit>`. A sentence before the table says that the offsets apply to a data block with standard access. A sentence after the table gives the size in bytes.

A machine on `CodesysIec` or `TwinCat3` gets no S7 layout. The placement rules are in ADR-0017 and ADR-0108 (g).

The fields `members[i].type` of `DefineUdt` and `type` of section 2.6 carry a `TypeExpr` (`domain-model.md` 2.1). Examples: `INT`, `STRING(20)`, `ST_Zone@3`, `ARRAY[1..8] OF REAL`, `ARRAY[0..1,1..2] OF ST_Zone@3`. The internal operation `EnsureDerivedType` resolves the expression and creates a missing `STRING(n)`, `WSTRING(n)`, or array type (`domain-model.md` 5.1). A bad expression returns `422 E-PLT-013`. The exception is a member of `DefineUdt` whose type is not on the platform, which returns `422 E-PLT-006`. A type above a size limit returns `422 E-PLT-018` (`domain-model.md` 2.1).

### 2.4 Recipes

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes` | `ListRecipes` | `RecipeView` | `q`, `machine`, `status`, `page`, `show_deleted` | `200` list, `page.recipes` rows per page. Deleted recipes are hidden. `show_deleted=1` runs `ListDeletedRecipes` with `RecipeRestore` and adds the deleted recipes (ADR-0043) | - |
| GET | `/recipes/new` | `GetRecipeForm` | `RecipeCreate` | - | `200` form | - |
| POST | `/recipes` | `CreateRecipe` | `RecipeCreate` | `machine_code`, `reference_number`, `name`, `change_note` | `303` to `/recipes/{ref}/versions/1` | `409 E-RCP-001`, `409 E-PLT-010`, `404 E-PLT-019` |
| GET | `/recipes/{ref}` | `GetRecipe` | `RecipeView` | `page` | `200` recipe page with version list, `page.versions` rows per page | `404 E-WEB-005`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/rename` | `RenameRecipe` | `RecipeEdit` | `name` | `200` fragment | `404 E-WEB-005`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/delete` | `DeleteRecipe` | `RecipeDelete` (conditional, section 3) | CSRF token | `303` to `/recipes` | `404 E-WEB-005`, `409 E-RCP-010`, `409 E-RCP-026`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/restore` | `RestoreRecipe` | `RecipeRestore` | CSRF token | `303` to `/recipes/{ref}` | `404 E-WEB-005`, `409 E-RCP-025`, `409 E-PLT-010` |

`DeleteRecipe` discards the live `Draft` in the same transaction.

`q` is a `Text`. It matches a part of the reference number or of the name, with no difference between upper and lower case. `machine` follows the rule of the path value `{code}` (section 1). `status` is `draft`, `approved`, or `retired` and keeps a recipe with a version in that state. The rows come in the byte order of the reference number (ADR-0043).

A deleted recipe. An Administrator reads its recipe page and its version pages with the state "Deleted" and no form.

Every other `GET` route with `{ref}` of sections 2.4 to 2.6 answers `409 E-RCP-012` to every other role. A form route, such as `.../versions/new` or `.../parameters/new`, answers `409 E-RCP-012` to every role with the permission of its command (ADR-0043, ADR-0046). The approval preview is a form route. The diff route of section 2.5 and the history route of section 2.9 follow the rule of the other `GET` routes. An Administrator reads them, and every other role gets `409 E-RCP-012` (ADR-0050, ADR-0051).

The recipe page holds the delete form in a danger disclosure for a role with `RecipeDelete`. The list with `show_deleted=1` holds a restore form in each deleted row. The pages of a deleted recipe keep no form that writes (ADR-0049, ADR-0108 (d)). The version page keeps the compare form, because it sends a `GET` request.

### 2.5 Versions

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes/{ref}/versions` | `ListVersions` | `RecipeView` | `page` | `200` version list fragment, `page.versions` rows per page | `404 E-WEB-005`, `409 E-RCP-012` |
| GET | `/recipes/{ref}/versions/new` | `GetVersionForm` | `VersionCreate` | - | `200` form with `based_on` and `change_note` | `404 E-WEB-005`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/versions` | `CreateVersion` | `VersionCreate` | `based_on` (version number, optional), `change_note` | `303` to the new `Draft`. The draft page lists only the outdated parameters, which need a rebind. A base version can reference an archived unit, an archived machine, or a type that is not on the platform. Then the server returns `422 E-RCP-020` with the items and creates no draft | `404 E-WEB-005`, `409 E-RCP-011`, `409 E-RCP-012`, `409 E-PLT-010`, `422 E-RCP-020`, `422 E-RCP-021`, `422 E-WEB-008` (`based_on`) |
| GET | `/recipes/{ref}/versions/{n}` | `GetVersion` | `RecipeView` | `page` and `unit` for the parameter table (ADR-0043 (j)) | `200` version page. Editable forms only for a `Draft` and a user with `ParameterEdit`. The header of an `Approved` or a `Retired` version shows `content_sha256`. The page shows the compare form when the recipe has two or more versions (ADR-0108 (c), (d)) | `404 E-WEB-005`, `409 E-RCP-012`, `503 E-WEB-009` the check of an `Approved` version passes `timeout.export`, `500 E-WEB-011` the task of that check fails (ADR-0122) |
| GET | `/recipes/{ref}/versions/{n}/approve` | `GetApprovalPreview` | `VersionApprove` | `kind`, `page`, `rv` (ADR-0050) | `200` approve page. Shows the diff against the current `Approved` version. Limit changes are highlighted. Lists the outdated parameters and offers the flag `allow_outdated`. Shows the blockers of the approval (ADR-0049) | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/versions/{n}/approve` | `ApproveVersion` | `VersionApprove` | `row_version`, `allow_outdated` (default `false`), CSRF token | `200` version page. Shows the retired version number when one exists. An already approved version with `row_version == expected + 1` returns the same page without a change | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-RCP-015`, `409 E-WEB-001`, `422 E-RCP-008`, `422 E-RCP-016`, `422 E-RCP-020`, `422 E-RCP-022`, `422 E-RCP-023`, `422 E-RCP-024`, `422 E-RCP-056`, `422 E-RCP-057`, `500 E-EXP-004` |
| POST | `/recipes/{ref}/versions/{n}/retire` | `RetireVersion` | `VersionRetire` | `row_version`, CSRF token | `200` version page | `404 E-WEB-005`, `409 E-RCP-009`, `409 E-RCP-012`, `409 E-WEB-001` |
| POST | `/recipes/{ref}/versions/{n}/discard` | `DiscardDraft` | `VersionDiscard` | `row_version`, CSRF token | `303` to `/recipes/{ref}` | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-WEB-001` |
| POST | `/recipes/{ref}/versions/{n}/rebind` | `RebindTypeRevision` | `ParameterEdit` | `row_version`, `from_type_id`, `to_type_id`, one field `new_member_values[<key>]<leaf path>` per new member leaf (ADR-0039) | `200` parameter table fragment | `404 E-WEB-005`, `409 E-RCP-002`, `409 E-RCP-012`, `409 E-WEB-001`, `422 E-RCP-005`, `422 E-RCP-021`, `422 E-RCP-022`, `422 E-RCP-028`, `422 E-RCP-029`, `422 E-RCP-052`, `422 E-RCP-055`, `422 E-RCP-056`, `422 E-RCP-057`, `422 E-PLT-018` |
| GET | `/recipes/{ref}/versions/{n}/diff/{m}` | `DiffVersions` | `RecipeView` | `kind`, `page` (ADR-0050) | `200` parameter diff between versions `n` and `m` | `404 E-WEB-005`, `409 E-RCP-012` |
| GET | `/recipes/{ref}/compare` | - (redirect only) | authenticated | `n`, `m` (version numbers, ADR-0108 (e)) | `303` to `/recipes/{ref}/versions/{max(n,m)}/diff/{min(n,m)}` | `404 E-WEB-005` (malformed `{ref}`), `422 E-WEB-008` |

A version with `based_on` empty starts with no parameters. The approve `POST` is idempotent (section 4). It returns `422 E-RCP-024` for an outdated parameter unless `allow_outdated` is `true`. The audit entry `VersionApproved` records the flag (A29, ADR-0018).

`RebindTypeRevision` moves every parameter of the draft whose type uses `from_type_id` to `to_type_id` (`domain-model.md` 5.2). A failed rebind returns one `E-RCP-028` or `E-RCP-029` line per failing parameter and changes nothing.

The request holds one field per new member leaf. The field name is `new_member_values[<key>]` followed by the leaf path, where `<key>` is the `ParameterKey`. The field holds the text of the leaf (ADR-0039). A duration or instant leaf is an IEC literal. A new member leaf without a field gives `E-RCP-028`.

Only a user with `ParameterEdit` on a live `Draft` gets the rebind forms, with at most `form.rebind_leaves` new member fields on one page (ADR-0044).

`based_on` of `CreateVersion` is a request field. A number that names no version of the recipe gives `422 E-WEB-008` with the field `based_on` (ADR-0043).

The approve page is the review step of the approval (ADR-0049). It runs the checks of the approval without a write and shows each blocker. With a blocker, it shows no approve form. The retire and the discard forms stand in danger disclosures on the version page. An HTMX success of the approval and of the retirement sets `HX-Push-Url` to the path of the version page.

The diff compares the definition, the value per leaf, the limit rows, and the translations of each parameter, matched by `key`. In `.../diff/{m}`, `m` is the old side. The page groups the changes by kind. `kind` names one section, and `page` counts the pages of that section, with `page.diff` parameters per page (ADR-0050). The preview carries the `row_version` of its first page in `rv`.

The compare form of the version page has the lists `n` and `m`, which hold each version number of the recipe. It sends `GET /recipes/{ref}/compare` and no CSRF token (ADR-0037, ADR-0108 (d)). The compare route runs no use case, reads no data, and writes nothing. A missing key, an unknown key, or a value that is not a version number gives `422 E-WEB-008`. The diff route then checks the permission and the versions.

The header of an `Approved` or a `Retired` version shows the field `content_sha256` of section 6. A `Draft` shows no content hash (ADR-0108 (c)).

The version page of an `Approved` version shows a notice when a value, a limit, or an S7 address breaks a current rule (ADR-0119 (l)). The notice gives the code of each broken rule. The export link names the notice in `aria-describedby`, and the page reads the values of every parameter to run the check (ADR-0119 (m)). The check takes a permit of `export.permits` and runs inside `timeout.export` (ADR-0122).

### 2.6 Parameters

All routes in this group are under `/recipes/{ref}/versions/{n}`. A mutating route works only when version `n` is a live `Draft` (I3). Otherwise the server returns `409 E-RCP-002`.

Every mutating route in this group can also return `409 E-WEB-001`. Every route of this group returns `409 E-RCP-012` when the recipe is deleted. The exception is a `GET` route that shows data, for a user with `RecipeRestore` (ADR-0043). Every mutating route with `{key}` can also return `404 E-RCP-051` when the draft does not hold the key (section 1). The tables do not repeat these codes, except in the two removal rows.

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `.../parameters` | `ListParameters` | `RecipeView` | `unit` (path filter), `page` | `200` parameter table fragment, grouped by unit, 100 rows per page | `404 E-WEB-005`, `503 E-WEB-009` the read check passes `timeout.validation`, `500 E-WEB-011` the task of the read check fails (ADR-0122) |
| GET | `.../parameters/new` | `GetParameterForm` | `ParameterEdit` | `unit` | `200` empty parameter form. The server generates `key` at `AddParameter` (A40) | `404 E-WEB-005`, `409 E-RCP-002` (ADR-0043 (k)) |
| POST | `.../parameters` | `AddParameter` | `ParameterEdit` | `row_version`, `unit_path`, `tag`, `name_en`, `description_en`, `type` (a `TypeExpr`), `s7_address`, `ordinal`, the value fields (one per leaf, required), `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` (`limits[]` can be empty). Optional: `fields` and `limit_mode` (ADR-0039) | `200` parameter row fragment with the new `key`. With `fields=1` or `limit_mode`, the query `GetParameterForm` runs in place of `AddParameter`. Then `200` with the form and the leaf rows of the sent type | `409 E-RCP-014`, `422 E-RCP-003`, `422 E-RCP-004`, `422 E-RCP-005`, `422 E-RCP-006`, `422 E-RCP-017`, `422 E-RCP-018`, `422 E-RCP-019`, `422 E-RCP-021`, `422 E-RCP-022`, `422 E-RCP-023`, `422 E-RCP-053`, `422 E-RCP-054`, `422 E-RCP-055`, `422 E-RCP-056`, `422 E-RCP-057`, `409 E-PLT-009`, `422 E-PLT-013`, `422 E-PLT-018`, `404 E-PLT-020` |
| GET | `.../parameters/{key}` | `GetParameter` | `RecipeView` | - | `200` parameter detail or edit form | `404 E-WEB-005` |
| POST | `.../parameters/{key}/definition` | `UpdateParameterDefinition` | `ParameterEdit` | `row_version`, `unit_path`, `tag`, `name_en`, `description_en`, `type` (a `TypeExpr`), `s7_address`, `ordinal`. When `type` names a different type than the current one: also the value fields and `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length`, all required. Otherwise both are optional (ADR-0029). Optional `fields` and `limit_mode` (ADR-0039) | `200` parameter summary fragment and the clean form out of band (ADR-0041 (h), ADR-0126 (h)). The HTMX answer also replaces the English base text `#translations-base` out of band and no translation row (ADR-0111 (i)). With `fields=1` or `limit_mode`, the query `GetParameterForm` runs in place of the command. Then `200` with the form | same as `AddParameter`. Missing value fields after a type change are `422 E-RCP-005` |
| POST | `.../parameters/{key}/value` | `SetParameterValue` | `ParameterEdit` | `row_version`, the value fields (one per leaf) | `200` value fragment | `422 E-RCP-004`, `422 E-RCP-005`, `422 E-RCP-021`, `422 E-RCP-055` |
| GET | `.../parameters/{key}/limits` | `GetLimits` | `RecipeView` | - | `200` limits table fragment, one row per leaf path. `[*]` is one row for all elements of an array | `404 E-WEB-005` |
| POST | `.../parameters/{key}/limits` | `ReplaceLimits` | `ParameterEdit` | `row_version`, `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` | `200` limits fragment | `422 E-RCP-004`, `422 E-RCP-006`, `422 E-RCP-017`, `422 E-RCP-018`, `422 E-RCP-019`, `422 E-RCP-021`, `422 E-RCP-055` |
| GET | `.../parameters/{key}/translations` | `GetTranslations` | `RecipeView` | - | `200` translation table fragment | `404 E-WEB-005` |
| POST | `.../parameters/{key}/translations/{lang}` | `UpsertTranslation` | `ParameterEdit` | `row_version`, `name`, `description` | `200` translation fragment | `422 E-RCP-027` |
| POST | `.../parameters/{key}/translations/{lang}/delete` | `RemoveTranslation` | `ParameterEdit` | `row_version`, CSRF token | `200` row fragment of `{lang}` without a stored translation (ADR-0111) | `404 E-WEB-005` (malformed `{key}` or missing version), `404 E-RCP-051` (`{key}` not in the draft), `404 E-RCP-050` (`{lang}` not in the draft) |
| POST | `.../parameters/{key}/delete` | `RemoveParameter` | `ParameterEdit` | `row_version`, CSRF token | `200` parameter table fragment | `404 E-WEB-005` (malformed `{key}` or missing version), `404 E-RCP-051` (`{key}` not in the draft) |

Rules:

- A command touches only its own rows. `SetParameterValue` does not read or write `parameter_limit` or `parameter_translation`. `ReplaceLimits` replaces all limit rows of one parameter and nothing else. `UpsertTranslation` writes one language.
- The form identifies a parameter by `key`. A `tag` can change through `UpdateParameterDefinition`. The parameter page shows the current `tag`.
- The form sends one value field per leaf. The field name is `value` followed by the leaf path, for example `value[3]` or `value.zone.hold` (ADR-0039). The server builds the `ParameterValue` from all value fields. A missing leaf (a partial value) is `422 E-RCP-005`. A request with value fields has the larger body limit of section 7.6.
- A key outside the fields of the route, a malformed leaf path, a `[*]` segment, or a duplicate key in the form body is `422 E-WEB-008`. A well-formed leaf path that the type does not hold is `422 E-RCP-005`. A form answer lists every bad field and every fault of the value and the limits at once (ADR-0041).
- With `fields=1` or `limit_mode`, the route keeps the operation of its command. In read-only mode it answers `503 E-WEB-006` (ADR-0039).
- A `duration` or `instant` leaf in a value field, `limits[i].min`, `limits[i].max`, and `new_member_values` is an IEC literal, for example `T#1s` (`domain-model.md` 2.1). The server stores the count of the base unit.
- `AddParameter` always carries the value fields, and `limits[]` can be empty. `UpdateParameterDefinition` carries the value fields and `limits[]` when the type changes. Then both replace the old value and the old limits. Without a type change, a request with both also replaces them (ADR-0029). A request without them keeps the value and the limits unchanged. `SetParameterValue` and `ReplaceLimits` change one of them without a type change.
- The parameter table is paginated. A `Draft` with more than 100 parameters shows one unit per page by default.
- The parameter table of the version page has the column Value after the column Type (ADR-0108 (a), (b), ADR-0121). A parameter of a primitive type shows the text of its leaf. The cell holds at most `table.value_chars` characters (`reliability.md` section 7.11). A longer text ends with `…`. An array or a struct shows a fixed word. The table read loads the values of one page only, through the port method `parameter_values`. A stored part that breaks a current rule gets a field line, and the answer stays `200` (ADR-0118 (c)). A stored part that fails the earlier rule set gives `500 E-DB-008` (ADR-0118 (d)). The row fragment of `AddParameter` shows the canonical text of the new value.
- A save toast of a command after which the draft stays a live draft names the new `row_version`. The number equals `rowVersion` in `HX-Trigger` (section 4, ADR-0108 (f)).

### 2.7 Export

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/export/v1/recipes/{ref}/approved` | `ExportVersion` | `VersionExport` | - | `200 application/json`. The `Approved` version. `loadable` follows section 6.1 | `404 E-WEB-005` no such recipe, `404 E-EXP-001` no approved version (a deleted recipe has none, I14), `500 E-EXP-002` content hash mismatch, `500 E-EXP-004` serializer error |
| GET | `/export/v1/recipes/{ref}/versions/{n}` | `ExportVersion` | `VersionExportHistory` | - | `200 application/json`. An `Approved` or `Retired` version. `loadable` follows section 6.1 | `404 E-WEB-005` no such recipe or version, `409 E-EXP-003` version is a `Draft` or discarded, `409 E-RCP-012` recipe deleted, `500 E-EXP-002`, `500 E-EXP-004` |
| GET | `/export/v1/machines/{code}/recipes` | `ListExportableRecipes` | `VersionExport` | - | `200 application/json`. The machine and its recipes with an `Approved` version (below) | `404 E-WEB-005` no such machine, `503 E-WEB-009` the read passes `timeout.export` |
| GET | `/export/v1/machines/{code}/recipes/{ref}/approved` | `ExportVersion` | `VersionExport` | - | `200 application/json`. The v1 document of the `Approved` version (section 6.1). `loadable` follows section 6.1 | `404 E-WEB-005` no such machine, no such recipe, or a recipe of another machine, `404 E-EXP-001`, `500 E-EXP-002`, `500 E-EXP-004` |
| GET | `/export/v2/machines/{code}/recipes/{ref}/approved/units/{path}` | `ExportVersion` with a unit scope | `VersionExport` | - | `200 application/json`. The v2 document of one unit and its descendant units (section 6.4). `loadable` follows section 6.1 | `404 E-WEB-005` as the row above, or no such unit, `404 E-EXP-001`, `404 E-EXP-005` no parameter in the scope, `422 E-PLT-017`, `422 E-PLT-002`, `500 E-EXP-002`, `500 E-EXP-004` |

Two schemes. The first two routes use the session cookie (ADR-0014) and never read `Authorization`. The last three routes use HTTP Basic of an `ExportClient` account and never read the session cookie. A request with both credentials uses the scheme of its route (ADR-0109 (l)). Section 7.7 defines the Basic check. The last three routes also return the codes of section 7.7.

A `Draft` is never exportable. Each successful export writes one audit entry (A8). The list route writes none. Section 6 defines the documents. The second route exists for rollback and audit use. Operators do not have `VersionExportHistory` (section 3).

Machine and unit values of the three transfer routes:

- `{code}` names the live machine with that code. When no live machine has the code, it names the archived machine with that code that the system archived last. Recipes of an archived machine stay exportable (ADR-0109 (j)).
- A reused code hides the older archived machine from these routes. The session routes still export its recipes.
- `{path}` follows the rule of `UnitPath` in section 1. A path that names no unit of the machine gives `404 E-WEB-005`. On a live machine, an archived unit counts as no unit. On an archived machine, every unit counts (ADR-0109 (ja)).

List body. The object `machine` holds `code`, `plc_platform`, and `archived`. The array `recipes[]` holds one row for each recipe of the machine with an `Approved` version. A row holds `reference_number`, `name`, `version_number`, `approved_at`, and `content_sha256`. The rows follow the byte order of the reference number. The list has no pages, because A19 limits the count of recipes.

The query runs inside `timeout.export`. This limit is the time bound of the list (ADR-0109 (g)).

Polling rule. The client reads the list and compares `content_sha256` with its last value. It runs an export only after a change (ADR-0109 (ag)). An approval can occur between the list and the export. The client therefore trusts `version.number` and `content_sha256` of the document, not the values of the list row (ADR-0109 (ah)).

The export routes accept only `GET`. A `HEAD` request gets the answer of a method that the route table does not list. The use case does not run, and the audit log gets no entry (ADR-0054). The version page links to the export of its version (ADR-0052). A navigation guard refuses a cross-site or same-site request and an HTMX request before the use case (section 7.3, ADR-0055).

### 2.8 Users

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/users` | `ListUsers` | `UserManage` | `q`, `page` | `200` list | - |
| GET | `/users/new` | `GetUserForm` | `UserManage` | - | `200` form | - |
| POST | `/users` | `CreateUser` | `UserManage` | `login`, `role` | `200` page of the new user with a one-time password, shown once. `must_change_password` is set, except for the role `ExportClient`, which keeps the flag false (ADR-0109 (d)) | `409 E-IDN-001` |
| GET | `/users/{login}` | `GetUser` | `UserManage` | - | `200` user page with lock state | `404 E-WEB-005` |
| POST | `/users/{login}/role` | `SetUserRole` | `UserManage` | `role` | `200` fragment. All sessions of the user are deleted | `404 E-WEB-005`, `409 E-IDN-002`, `409 E-IDN-011` a change to or from `ExportClient` (ADR-0109 (f)) |
| POST | `/users/{login}/deactivate` | `DeactivateUser` | `UserManage` | CSRF token | `200` fragment. All sessions of the user are deleted | `404 E-WEB-005`, `409 E-IDN-002`, `409 E-IDN-010` (ADR-0035) |
| POST | `/users/{login}/activate` | `ActivateUser` | `UserManage` | CSRF token | `200` fragment | `404 E-WEB-005`, `409 E-IDN-009` |
| POST | `/users/{login}/reset-password` | `ResetPassword` | `UserManage` | CSRF token | `200` fragment with a one-time password, shown once. `must_change_password` is set, except for an `ExportClient` account, which keeps the flag false. All sessions of the user are deleted (ADR-0109 (e)) | `404 E-WEB-005` |
| POST | `/users/{login}/unlock` | `UnlockLogin` | `LoginUnlock` | CSRF token | `200` fragment. The login delay counters of the user are cleared | `404 E-WEB-005` |
| GET | `/me` | `GetOwnProfile` | authenticated | - | `200` own profile | - |
| GET | `/me/password` | `GetOwnPasswordForm` | authenticated | - | `200` password form. The only page a session with `must_change_password` can open | - |
| POST | `/me/password` | `ChangeOwnPassword` | authenticated | `current_password`, `password` | `200` fragment with the header `HX-Refresh: true`. Every session of the user is deleted, and a new session replaces the current one. `must_change_password` is cleared. The browser loads the page again and gets the new CSRF token (section 7.3). A request without `HX-Request` gets `303` to `/recipes` with the new session cookie (ADR-0035) | `422 E-IDN-006`, `422 E-IDN-008`, `429 E-IDN-005` (ADR-0035) |

The administrator never chooses a password for another user. `CreateUser` and `ResetPassword` generate the password (ADR-0014). The generated password of an `ExportClient` account is the credential of a transfer client, so the account has no forced change. Deactivate is a soft delete (ADR-0021). A deactivated user keeps the audit history and cannot log in.

### 2.9 Audit and security events

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/audit` | `ListAuditEntries` | `AuditView` or `AuditViewRecipes` | `aggregate`, `entity_type`, `entity_id`, `actor`, `action`, `from`, `to`, `before` (cursor) | `200` audit list, newest first. A user with only `AuditViewRecipes` sees entries with `aggregate_type = 'recipe'`: the entity types `recipe`, `recipe_version`, and `parameter`. A limit or translation change is a `parameter` entry (`data-model.md` 10) | `422 E-WEB-008` |
| GET | `/audit/{id}` | `GetAuditEntry` | same as above | - | `200` one entry with `before` and `after` as formatted JSON | `404 E-WEB-005` |
| GET | `/recipes/{ref}/history` | `GetRecipeHistory` | `RecipeView` | `before` (cursor) | `200` audit entries with `aggregate_id` of the recipe, without `before` and `after` bodies | `404 E-WEB-005`, `409 E-RCP-012`, `422 E-WEB-008` |
| GET | `/security-events` | `ListSecurityEvents` | `AuditView` | `kind`, `login`, `client_ip`, `from`, `to`, `before` | `200` list of security events | `422 E-WEB-008` |

The `/recipes/{ref}/history` route is the recipe timeline. Every role can open it (R24). It does not show user data or machine changes. The audit list uses cursor pagination on the entry id (`data-model.md` section 3.15).

Rules of the three lists (ADR-0051):

- Each list shows the newest row first, at most `page.audit`, `page.history`, or `page.security_events` rows. `before` is the id of a row, a positive integer.
- `aggregate` is the id of an aggregate (a UUID). `entity_id` is a UUID and needs `entity_type`. `entity_type`, `action`, and `kind` take a value of the fixed lists of `data-model.md` sections 3.8 and 10.
- `actor` is a `Login`. A malformed `actor` gives `422 E-WEB-008`. A well-formed `actor` that names no user gives the empty list. This is an exception to the rule of a query value of section 1, because a `404` would show which logins exist (A42).
- `login` is a `TypedLogin`. A malformed `login` gives `422 E-WEB-008`. `client_ip` is an IP address. `from` and `to` are dates `YYYY-MM-DD` in UTC, and both days are inside the range.
- A malformed value, `entity_id` without `entity_type`, or `to` before `from` gives `422 E-WEB-008`. An empty field means no filter.
- A user with only `AuditViewRecipes` gets an empty list for a filter value outside the recipe scope. `GET /audit/{id}` of an entry outside that scope gives `404 E-WEB-005`.
- `login` matches the user of the login and the `login_hmac` of the login under the current key of the key ring (`KeyRing::sign`). An event under an older key does not match. No page shows `login_hmac`.

### 2.10 Health

The health endpoints are not on the public listener. `recipe-server` opens a second listener on `http.internal_port` (9000) on the internal Docker network. It serves `GET /healthz` and `GET /readyz`. Both return `text/plain` with the body `ok` and the status `200`, or the body `not ready` and the status `503` (ADR-0059 (g)). The proxy answers `404` for `/healthz*` and `/readyz*` on the public site. `reliability.md` section 7.2 defines the checks.

`reliability.md` section 7.2 gives the health router, its layers, and the subcommand `healthcheck` (ADR-0059 (a), (k) to (m)).

## 3. Permission matrix

### 3.1 Permission enum

The enum lives in `recipe-domain`. `Role::permissions()` is a pure function in `recipe-domain`. The check runs in `recipe-app` as the first statement of every use case, for commands and for queries (ADR-0013, R28).

```rust
pub enum Permission {
    RecipeView, RecipeCreate, RecipeEdit, RecipeDelete, RecipeRestore,
    VersionCreate, VersionApprove, VersionRetire, VersionDiscard,
    VersionExport, VersionExportHistory,
    ParameterEdit,
    MachineManage, UnitManage, UdtManage,
    UserManage, LoginUnlock,
    AuditView, AuditViewRecipes,
}
```

### 3.2 Rules

- The HTTP layer only authenticates. It does not decide what a role can do.
- A form or preview route is a query use case with the permission of the command that it prepares (`domain-model.md` 5.5). The matrix row of that command names the query.
- The input of a form query names the command that it serves, the purpose. The query checks the permission of that command, and a `forbidden` event names that command. A write route of a form runs its form query before it reads its fields (ADR-0046). The five action routes of ADR-0049 have no form query and read their body first (ADR-0049 (l)).
- A role maps to a fixed set of permissions. There are no per-user overrides (ADR-0013).
- `ExportClient` is the role of a technical account. It holds only `VersionExport` (ADR-0109 (a), (b)). The three roles of R22 stay the roles of persons (A45). The three transfer routes accept only an `ExportClient` account, and `Login` refuses it.
- A generated integration test checks every (operation, role) pair of the matrix below, for commands and for queries (`reliability.md` section 6).
- Last administrator rule: `SetUserRole` and `DeactivateUser` return `Err(LastAdministrator)` when the target is the last active user with role `Administrator`. The use case takes the class-4 guard lock, then counts active administrators in a `READ COMMITTED` transaction with no retry (`data-model.md` section 6, ADR-0096). There is no database trigger for this rule.
- Separation of duties: `ApproveVersion` returns `Err(ApproverEditedDraft)` when the approver is `created_by` of the draft or has a row in `draft_editor` for it. The configuration value `approval.separation_of_duties` is `true` by default (A34). The server stores it in `app_setting`. A change of the value writes the audit entry `SettingChanged` at the next start (`reliability.md` 7.12).
- `EnsureDerivedType` is an internal operation of the Plant Model, not a use case. It has no permission and no matrix row. The calling use case (`AddParameter`, `UpdateParameterDefinition`, `DefineUdt`, `RebindTypeRevision`) checks its own permission first.
- The six supervised jobs (`cleanup`, `disk_check`, `audit_chain_check`, `heartbeat`, `clock_check`, `alerts`) and the startup settings comparison `CompareSettings` are internal operations too. They have no permission and no matrix row (ADR-0060 (t)).
- A `Forbidden` result writes one `security_event` row in its own transaction through the port `SecurityEventSink` (A33). The use case has no business transaction at that point. When that row cannot be written, the use case returns the storage error instead of `E-WEB-002` (ADR-0029).

### 3.3 Matrix

Legend: ✓ allowed, ✗ denied, (c) allowed with the condition in the last column.

| Operation | Permission | Administrator | ProcessEngineer | Operator | ExportClient | Condition |
|---|---|---|---|---|---|---|
| View recipes, versions, parameters, machines, units, UDTs, type usage. Queries `ListMachines`, `GetMachine`, `GetUnitTree`, `GetUnit`, `ListTypes`, `GetTypeTree`, `GetTypeUsage`, `ListRecipes`, `GetRecipe`, `ListVersions`, `GetVersion`, `DiffVersions`, `ListParameters`, `GetParameter`, `GetLimits`, `GetTranslations` | `RecipeView` | ✓ | ✓ | ✓ | ✗ | - |
| View recipe history (`/recipes/{ref}/history`). Query `GetRecipeHistory` | `RecipeView` | ✓ | ✓ | ✓ | ✗ | - |
| Export the `Approved` version (`ExportVersion`) | `VersionExport` | ✓ | ✓ | ✓ | ✓ | A4. ADR-0109 (b). Over HTTP, only an `ExportClient` account reaches the three transfer routes (section 7.7) |
| List the recipes of a machine with an `Approved` version (`ListExportableRecipes`) | `VersionExport` | ✓ | ✓ | ✓ | ✓ | R18, A4. ADR-0109 (g) |
| Export a `Retired` version by number (`ExportVersion`) | `VersionExportHistory` | ✓ | ✓ | ✗ | ✗ | ADR-0019 |
| Create recipe (`CreateRecipe`). Form query `GetRecipeForm` with the purpose `CreateRecipe` | `RecipeCreate` | ✓ | ✓ | ✗ | ✗ | R25 |
| Rename recipe (`RenameRecipe`). Form query `GetRecipeForm` with the purpose `RenameRecipe` | `RecipeEdit` | ✓ | ✓ | ✗ | ✗ | R25, ADR-0046 |
| Delete recipe, soft (`DeleteRecipe`) | `RecipeDelete` | (c) | (c) | ✗ | ✗ | ProcessEngineer: only when the recipe has no `Approved` and no `Retired` version (`E-RCP-026`). Administrator: only when the recipe has no `Approved` version (`E-RCP-010`, I14). R23, A13 |
| Restore recipe (`RestoreRecipe`). List deleted recipes (`ListRecipes` with `show_deleted`) | `RecipeRestore` | ✓ | ✗ | ✗ | ✗ | ADR-0021 |
| Create version, a new `Draft` (`CreateVersion`). Form query `GetVersionForm` with the purpose `CreateVersion` | `VersionCreate` | ✓ | ✓ | ✗ | ✗ | R25, A7 |
| Add, edit, delete parameters, limits, translations of a `Draft`. Rebind a type revision (`AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, `RemoveParameter`, `RebindTypeRevision`). Form query `GetParameterForm` with the purpose of each command of this row | `ParameterEdit` | ✓ | ✓ | ✗ | ✗ | R09, R25 |
| Approve version (`ApproveVersion`). Preview query `GetApprovalPreview` | `VersionApprove` | (c) | (c) | ✗ | ✗ | A5, A34. Not the creator or an editor of the draft (`E-RCP-015`). The approval retires the previous `Approved` version (R27) |
| Retire version without replacement (`RetireVersion`) | `VersionRetire` | ✓ | ✗ | ✗ | ✗ | A28 |
| Discard `Draft` (`DiscardDraft`) | `VersionDiscard` | ✓ | ✓ | ✗ | ✗ | - |
| Create, rename, archive machine (`CreateMachine`, `RenameMachine`, `ArchiveMachine`). Form query `GetMachineForm` with the purpose `CreateMachine` or `RenameMachine` | `MachineManage` | ✓ | ✓ | ✗ | ✗ | A9 |
| Add, rename, archive unit (`AddUnit`, `RenameUnit`, `ArchiveUnit`). Form query `GetUnitForm` with the purpose `AddUnit` or `RenameUnit` | `UnitManage` | ✓ | ✓ | ✗ | ✗ | A9 |
| Define UDT revision (`DefineUdt`). Form query `GetUdtForm` with the purpose `DefineUdt` | `UdtManage` | ✓ | ✓ | ✗ | ✗ | A9 |
| Create user, set role, activate, deactivate, reset password (`CreateUser`, `SetUserRole`, `ActivateUser`, `DeactivateUser`, `ResetPassword`). Form query `GetUserForm`. Queries `ListUsers`, `GetUser` | `UserManage` | ✓ | ✗ | ✗ | ✗ | A9. Last administrator rule |
| Unlock a login (`UnlockLogin`) | `LoginUnlock` | ✓ | ✗ | ✗ | ✗ | ADR-0014 |
| Log in (`Login`). Form query `GetLoginForm` | none | ✓ | ✓ | ✓ | ✗ | No session needed. `ExportClient` gets `E-IDN-003` after the password check (ADR-0109 (c)) |
| Log out (`Logout`) | authenticated | ✓ | ✓ | ✓ | ✗ | `ExportClient` has no session |
| Change own password (`ChangeOwnPassword`). Form query `GetOwnPasswordForm`. Own profile query `GetOwnProfile` | authenticated | ✓ | ✓ | ✓ | ✗ | `ExportClient` has no session |
| View full audit log and security events. Queries `ListAuditEntries`, `GetAuditEntry`, `ListSecurityEvents` | `AuditView` | ✓ | ✗ | ✗ | ✗ | A20 |
| View audit entries of recipes, versions, parameters. Queries `ListAuditEntries`, `GetAuditEntry` | `AuditViewRecipes` | ✓ | ✓ | ✗ | ✗ | A20 |

Role to permission mapping:

| Role | Permissions |
|---|---|
| Administrator | every permission |
| ProcessEngineer | `RecipeView`, `VersionExport`, `VersionExportHistory`, `RecipeCreate`, `RecipeEdit`, `RecipeDelete` (c), `VersionCreate`, `ParameterEdit`, `VersionApprove` (c), `VersionDiscard`, `MachineManage`, `UnitManage`, `UdtManage`, `AuditViewRecipes` |
| Operator | `RecipeView`, `VersionExport` |
| ExportClient | `VersionExport` |

## 4. Concurrency protocol

The protocol implements ADR-0008 and ADR-0015 for every `Draft`.

1. `GET /recipes/{ref}/versions/{n}` renders the current `row_version` of the version into a hidden field `row_version` of every form on the page.
2. Every mutating request on that version sends `row_version`.
3. The use case locks the recipe row and then the version row with `FOR UPDATE` and compares `row_version` with the sent value. A mismatch rolls back and returns `Err(ConcurrentEdit { reference, number, expected, actual })`. The application never sets `row_version`. The database trigger function `bump_row_version` increments it on every update of the version row (`data-model.md` section 4.4).
4. On success the response carries the new value in the header `HX-Trigger: {"rowVersion": <new>}`. The static script `row-version.js` updates every hidden `row_version` field on the page. The one exception is a discard success: it answers `303` without `HX-Trigger` (ADR-0049 (f)).
5. On conflict the server returns `409 E-WEB-001` with the fragment `conflict.html`. The fragment shows: the current server `row_version`, the login of the last editor, the time of the last change, and one link "Reload version". The link goes to the version page with `hx-boost="true"` (ADR-0043). The unsaved input stays in the form until the reload. The time is `updated_at` of the version, and the last editor is the actor of the audit entry of that time (ADR-0043).
6. The `409` response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML show:#error-panel:top` (ADR-0036). The original target of the request does not change. HTMX swaps the `409` fragment because of the `responseHandling` value of section 9.
7. Every approve and save form has `hx-sync="closest form:drop"`. A second click while a request is in flight is dropped in the browser. The retire, discard, delete, and restore forms have the same attribute (ADR-0049).
8. Idempotent approve: the version is already `Approved`, and the sent `row_version` equals the approved `row_version` minus one. Then `ApproveVersion` returns `Ok(ApprovalOutcome::AlreadyApproved)` and writes no audit entry. The server renders the version page with `200`.

A parameter edit changes the `row_version` of the version, not only of the parameter row. Two users who edit two different parameters of the same `Draft` get a conflict on the second save. This is intended. A `Draft` is one unit of work for approval.

Read requests do not use `row_version`. An `Approved` version is immutable, but `RetireVersion` still sends `row_version` and can return `409 E-WEB-001`.

## 5. Error responses

### 5.1 Status mapping

`reliability.md` section 1.2 is the only list of error codes. This table gives the status family and the fragment for each family. It does not repeat the codes.

| Family | HTTP status | Fragment |
|---|---|---|
| Malformed field, missing field, wrong content type (`E-WEB-008`) | `422` | `field-errors.html`: one line per field |
| Value or shape error from the domain (`E-PLT-*`, `E-RCP-*` with status 422 in `reliability.md` section 1.2) | `422` | `field-errors.html`, with `leaf_path` for value errors. The approve summary (`E-RCP-020`) lists every failing parameter |
| State conflict: wrong status, duplicate, last administrator, has approved version, deleted recipe, approver edited the draft, version not exportable (`E-PLT-*`, `E-RCP-*`, `E-IDN-*`, `E-EXP-003` with status 409) | `409` | `error.html` with the rule that blocks the operation |
| Database rule without a domain code (`StorageError` of `recipe-app`: `E-DB-003`, `E-DB-006`, `E-DB-007`) | `409` | `error.html`. The event log gets an `ERROR` line |
| Concurrent edit (`E-WEB-001`) | `409` | `conflict.html` (section 4) |
| Database trigger (`E-PLT-030..038`, `E-RCP-030..036`) | `409` for a state rule, `422` for a shape rule, as `reliability.md` section 1.2 gives | `error.html`. The event log gets an `ERROR` line, because the domain check should have fired first |
| No permission (`E-WEB-002`) | `403` | `error.html` with the operation name |
| CSRF token missing or wrong (`E-WEB-007`) | `403` | `error.html` |
| No session, expired session (`E-WEB-004`) | `401` | `HX-Redirect: /login` for HTMX, `303` to `/login` otherwise. `E-WEB-004` is the only code with `HX-Redirect` |
| Basic credentials missing or malformed (`E-WEB-012`) | `401` | JSON body of section 5.3 with the header `WWW-Authenticate` (section 7.7). Only the three transfer routes return it |
| Wrong credentials (`E-IDN-003`) | `401` | `field-errors.html` on the login form. No redirect. On a transfer route: the JSON body of section 5.3 with the header `WWW-Authenticate` |
| Wrong current password (`E-IDN-008`) | `422` | `field-errors.html` on the password form |
| Missing resource in the URL (`E-WEB-005`, `E-EXP-001`, `E-EXP-005`, `E-RCP-050`, `E-RCP-051`) or in a request field (`E-PLT-019`, `E-PLT-020`) | `404` | `error.html` |
| Request too large (`E-WEB-003`) | `413` | `error.html` |
| Login throttled (`E-IDN-005`) | `429` | `error.html` with `Retry-After` |
| Read-only mode (`E-WEB-006`). Every command except `Login`, `Logout`, and `ExportVersion` gets it. Queries, the five export routes, `POST /login`, `POST /logout`, and `POST /lang` stay available (`reliability.md` 7.8) | `503` | `error.html` with the banner text |
| Timeout, deadlock, pool exhausted, database unavailable (`E-WEB-009`, and the `StorageError` codes `E-DB-001`, `E-DB-002`, `E-DB-004`, `E-DB-005`) | `503` | `error.html` with `Retry-After` set to `http.retry_after` (`reliability.md` 7.11) |
| Export failure: content hash mismatch or serializer error (`E-EXP-002`, `E-EXP-004`) | `500` | JSON error body on the export routes. `ApproveVersion` returns `E-EXP-004` with `error.html` |
| Unexpected, caught panic, failed `spawn_blocking` task, unexpected database error (`E-WEB-010`, `E-WEB-011`, `E-AUD-001`, `E-DB-008`) | `500` | `error.html` with `request_id` and code only. Never the error chain, never SQL, never a path |

On a path under `/export/`, every family with a status of 400 or more gets the JSON body of section 5.3. That body replaces the fragment. On the two session routes, `E-WEB-004` and `E-IDN-007` keep their redirects (ADR-0053). A transfer route never answers `303` (ADR-0109 (r)).

A caught panic gives `500 E-WEB-010` with the answer of sections 5.2 and 5.3 (ADR-0057 (a), (b)). That answer is the page, the HTMX fragment, or the JSON body on `/export/`. The answer of the backstop is the one exception to sections 5.1 to 5.3 (ADR-0057 (c) to (e)):

- The backstop is the second `CatchPanicLayer` directly inside the layer `request_id`. It catches a panic of `TraceLayer`, the header layers, and `render_errors`.
- Its answer is a plain `500` with `Content-Type: text/plain`. The body holds the code `E-WEB-010` and the English text of its row in `reliability.md` section 1.2 without the reference.
- The layer `request_id` adds `X-Request-Id`. The answer has no `HX-*` header and no JSON body, also on `/export/`.

### 5.2 Error fragment content

Every error fragment shows:

- the error code `E-<CTX>-<nnn>`,
- one sentence in the UI language that tells the user what to do,
- the `request_id`.

The same code appears in the structured log of the request. A user can report the code and the `request_id`. The operator finds the log line with them (R36). The answer of the backstop of section 5.1 is the one exception.

### 5.3 Content negotiation

- An HTMX request gets the fragment. The response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML show:#error-panel:top` (ADR-0036) for every status `>= 400`, except `E-WEB-004`, which sets `HX-Redirect`. HTMX swaps a 4xx or 5xx fragment only because of the `responseHandling` value of section 9.
- The same HTMX error response also sets `HX-Push-Url: false`. A refused form with `hx-push-url="true"` then keeps the URL of its page (ADR-0037 (z)).
- A request without `HX-Request` gets a full error page with the same content and the page layout.
- The export endpoints return `application/json` errors: `{"code": "E-EXP-001", "message": "...", "request_id": "..."}`. `message` is the sentence of the code in the UI language, without the reference. The answer has no `HX-*` header, also for an HTMX request. On the two session routes, a request without a valid session gets `303` to `/login`, and a session with `must_change_password` gets `303` to `/me/password` (ADR-0053). On a transfer route, `message` uses the UI language of the request, and the default is `pl` (ADR-0109 (r)).
- The answer of the backstop of section 5.1 is the one exception to these rules (ADR-0057 (e)).

## 6. Export document (v1 and v2)

The export document is the only machine-readable output of the system (ADR-0019). It is the interface to the transfer client. The client pulls the document with HTTP Basic (section 2.7, ADR-0109). The transfer client is out of scope.

The JSON Schema file `crates/recipe-export/schema/export-v1.schema.json` is the source of truth. Golden-file tests compare the serializer output with the schema and with stored examples. This section describes the schema. When the two differ, the schema file wins.

The module `export` of `recipe-domain` builds the hashed body and computes the hash. `recipe-export` writes the HTTP response (ADR-0019). Sections 6.1 to 6.3 describe v1. Section 6.4 describes v2.

### 6.1 Fields

| Field | Type | In hash | Rule |
|---|---|---|---|
| `schema_version` | string | no | `"1"`. A breaking change creates `"2"` and `/export/v2/`. Section 6.4 describes `"2"` |
| `exported_at` | string, RFC 3339 UTC | no | Time of this export |
| `exported_by` | string | no | `Login` of the user |
| `loadable` | boolean | no | `true` only when `version.status` is `Approved` at export time and no value, limit, or address breaks a rule (ADR-0119 (c)) |
| `content_sha256` | string, 64 hex chars | no | SHA-256 of the hashed body (below). Computed and stored at approval |
| `machine.code` | string | yes | `MachineCode` |
| `machine.name` | string | no | Name at export time |
| `machine.plc_platform` | string | yes | one of `CodesysIec`, `TwinCat3`, `S7_1200`, `S7_1500`, `S7_300` |
| `recipe.reference_number` | string | yes | `ReferenceNumber` |
| `recipe.name` | string | no | Name at export time |
| `version.number` | integer | yes | |
| `version.status` | string | no | `Approved` or `Retired` |
| `version.approved_at` | string, RFC 3339 UTC | no | |
| `version.approved_by` | string | no | `Login` |
| `types[]` | array | yes | One entry per `PlcType` that a parameter uses directly, through a struct member, or as an array element, except primitives without a length. Ordered by `id` (section 6.3) |
| `types[].id` | string, UUID | yes | `PlcTypeId` of the revision |
| `types[].name` | string or null | yes | Struct name. `STRING` or `WSTRING` for a length-bearing primitive. Null for an array |
| `types[].revision` | integer | yes | Revision of a UDT. `1` for other types |
| `types[].kind` | string | yes | `Primitive`, `Array`, or `Struct` |
| `types[].string_length` | integer or null | yes | Present in every entry. An integer for `STRING` and `WSTRING`, null for every other type |
| `types[].element_type_id` | string | yes | Only for `Array`. A type reference |
| `types[].dimensions[]` | `{lower, upper}`, integers | yes | Only for `Array`. One to 3 entries (A38) |
| `types[].members[]` | `{name, type_id}` | yes | Only for `Struct`. Ordered by member `ordinal`. `type_id` is a type reference |
| `parameters[]` | array | yes | Ordered by `unit_path` and `tag` (section 6.3). `parameter.ordinal` is not exported and not hashed |
| `parameters[].unit_path` | string | yes | `UnitPath` |
| `parameters[].tag` | string | yes | `ParameterTag` |
| `parameters[].key` | string, UUID | yes | `ParameterKey`, stable across versions (A40) |
| `parameters[].name_en` | string | no | Informational. A name is not in the hash |
| `parameters[].type_id` | string | yes | A type reference |
| `parameters[].s7_address` | string or null | yes | The stored value when present. Always present on `S7_300`. Null on `CodesysIec` and `TwinCat3` (A31) |
| `parameters[].limits[]` | `{leaf_path, min, max, max_length}` | yes | `min`, `max`, and `max_length` are JSON strings with the canonical decimal text, or null. A limit of a duration or instant leaf is a count of the base unit (table below). `leaf_path` `[*]` applies to every element of an array. Ordered by `leaf_path` (section 6.3) |
| `parameters[].value` | JSON | yes | The `ParameterValue` tree. Every numeric, duration, and instant leaf is a JSON string with the canonical decimal text (`reliability.md` 4.4). A duration or instant leaf holds a count of the base unit of its type (table below). A `BOOL` is a JSON boolean. A string or char leaf is a JSON string |

A type reference (`type_id`, `element_type_id`) is the primitive name for a primitive without a length, for example `INT`. For every other type it is a `types[].id`.

Base units of time values. A consumer gets the unit from the type name of the leaf. The document carries no unit field. `domain-model.md` 2.1 is the definition (Assumption A43).

| Type | Value in the document |
|---|---|
| `TIME`, `S5TIME` | Milliseconds |
| `LTIME` | Nanoseconds |
| `DATE` | Days since 1970-01-01 |
| `LDATE`, `LDT`, `DTL` | Nanoseconds since 1970-01-01 |
| `TIME_OF_DAY` | Milliseconds since midnight |
| `LTOD` | Nanoseconds since midnight |
| `DATE_AND_TIME` | Seconds since 1970-01-01 |

Presence rule. A field with the rule "Only for `Array`" or "Only for `Struct`" is absent from every other `types[]` entry. It is never null. Every other field of the table is always present. It is null where the table allows null and the value does not apply. The example of section 6.2 follows this rule.

Descriptions and translations are not exported. The transfer system does not need them.

Hashed body: a JSON object with the fields marked "yes" and nothing else.

```json
{
  "machine": { "code": "M12", "plc_platform": "S7_300" },
  "recipe": { "reference_number": "R-100" },
  "version": { "number": 3 },
  "types": [ "... as in the document ..." ],
  "parameters": [ "... as in the document, without name_en ..." ]
}
```

`content_sha256` is the lower-case hex SHA-256 of the RFC 8785 (JCS) canonical form of the hashed body. Identifiers and values are in the hash.

Names are informational and are not in the hash: `machine.name`, `recipe.name`, and `parameters[].name_en`. Status, approval data, and export data are not in the hash either. A rename of the recipe or of the machine does not change the hash. The retirement of the version does not change it either. Every number in the hashed body is an integer below 2^53 or a JSON string, so the canonical form is exact (ADR-0019).

### 6.2 Example

The hash value in the example is a placeholder.

```json
{
  "schema_version": "1",
  "exported_at": "2026-10-12T09:30:00Z",
  "exported_by": "jkowalski",
  "loadable": true,
  "content_sha256": "3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855e",
  "machine": { "code": "M12", "name": "Filler 2", "plc_platform": "S7_300" },
  "recipe": { "reference_number": "R-100", "name": "Bottle 0.5 l" },
  "version": {
    "number": 3, "status": "Approved",
    "approved_at": "2026-10-11T14:02:10Z", "approved_by": "anowak"
  },
  "types": [
    {
      "id": "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f",
      "name": "STRING", "revision": 1, "kind": "Primitive", "string_length": 16
    },
    {
      "id": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "name": "ZoneTemp", "revision": 2, "kind": "Struct", "string_length": null,
      "members": [
        { "name": "setpoint", "type_id": "REAL" },
        { "name": "label", "type_id": "5d2c7b1e-3f4a-4b6c-8d9e-1a2b3c4d5e6f" }
      ]
    }
  ],
  "parameters": [
    {
      "unit_path": "U1", "tag": "SPEED",
      "key": "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
      "name_en": "Line speed", "type_id": "INT", "s7_address": "DB1.DBW0",
      "limits": [ { "leaf_path": "", "min": "10", "max": "250", "max_length": null } ],
      "value": "120"
    },
    {
      "unit_path": "U1.2", "tag": "ZONE1",
      "key": "9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a",
      "name_en": "Zone 1", "type_id": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "s7_address": "DB1.DBB2",
      "limits": [
        { "leaf_path": ".label", "min": null, "max": null, "max_length": "16" },
        { "leaf_path": ".setpoint", "min": "20", "max": "95", "max_length": null }
      ],
      "value": { "setpoint": "72.5", "label": "Z1" }
    }
  ]
}
```

`ZONE1` starts at byte 2 and occupies 22 bytes: `REAL` (4) and `STRING(16)` (18). It does not overlap `SPEED` at bytes 0 and 1.

### 6.3 Rules

- The hashed body of one `(reference_number, version.number)` never changes. Only fields outside it can differ between two exports: `exported_at`, `exported_by`, `loadable`, `version.status`, `machine.name`, and `recipe.name`.
- `ApproveVersion` stores the SHA-256 of the hashed body. `ExportVersion` rebuilds the hashed body for an `Approved` and for a `Retired` version and compares its hash with the stored value. A difference gives `500 E-EXP-002`.
- The response sets `Content-Disposition: attachment; filename="recipe_{ref}_v{n}.json"` for an `Approved` version and `filename="RETIRED_recipe_{ref}_v{n}.json"` for a `Retired` version.
- The response sets `Cache-Control: no-store`.
- Each export writes one `AuditEntry` with action `VersionExported` (A8). The entry holds the version id, the schema version, and `content_sha256`.
- Field order in the document is fixed. The serializer does not emit extra fields.
- Array order. The serializer writes each array in this order. A consumer that builds the hashed body again uses the same order:
  - `types[]`: by `id`, compared as text byte by byte.
  - `parameters[]`: by `unit_path`, then `tag`. `unit_path` is compared segment by segment as integers, the first segment without its `U` prefix. A path comes before a longer path with the same first segments. `tag` is compared byte by byte. `parameter.ordinal` sorts the UI only. It is not in the document.
  - `parameters[].limits[]`: by `leaf_path`, compared byte by byte. In the example of 6.2, `.label` comes before `.setpoint`.
  - `types[].members[]`: by the member `ordinal` of `plc_type_member`, which the document gives as the array order.
- Before the transfer system loads a v1 document into a PLC, it MUST check `loadable`. It MUST also build the hashed body from the document with the array order above, compute its SHA-256, and compare the result with `content_sha256`. Every sort key of the hashed arrays is in the document: `id`, `unit_path`, `tag`, `leaf_path`, and the order of `members[]`.

### 6.4 Unit scope document (v2)

The v2 document holds the parameters of one unit and of its descendant units. A subset of the parameters fails the v1 check of section 6.3, and the v1 schema file refuses an extra top-level field. The unit document therefore is a new version (ADR-0109 (z), (aa)).

The schema file is `crates/recipe-export/schema/export-v2.schema.json`. As for v1, the file wins when it differs from this section. The v1 routes, the v1 document, and the v1 schema file do not change.

The v2 document holds every field of section 6.1 and two more fields. These fields differ from v1:

| Field | Type | In scope hash | Rule |
|---|---|---|---|
| `schema_version` | string | no | `"2"` |
| `scope` | object | yes | `{"unit_path": <path>}`. The path is the `{path}` of the route |
| `scope_sha256` | string, 64 hex chars | no | Hash of the scope hashed body (below). The system computes it at each export and does not store it |
| `content_sha256` | string, 64 hex chars | no | The stored hash of the whole version. It does not describe the parameters of this document |
| `parameters[]` | array | yes | Each parameter of the version whose `unit_path` equals `{path}` or starts with `{path}` and a `.`. The comparison uses whole segments, so `U1` does not match `U12`. The array order of section 6.3 applies |
| `types[]` | array | yes | The types that these parameters use, by the rule of section 6.1. The array order of section 6.3 applies |

Every other field has the type and the rule of section 6.1.

The v2 document uses the field order of section 6.1 with two insertions. `scope_sha256` comes directly after `content_sha256`. `scope` comes directly after `version`. The top-level order therefore is `schema_version`, `exported_at`, `exported_by`, `loadable`, `content_sha256`, `scope_sha256`, `machine`, `recipe`, `version`, `scope`, `types`, `parameters`.

Scope hashed body: a JSON object with `machine.code`, `machine.plc_platform`, `recipe.reference_number`, `version.number`, `scope`, `types[]`, and `parameters[]` without `name_en`.

```json
{
  "machine": { "code": "M12", "plc_platform": "S7_300" },
  "recipe": { "reference_number": "R-100" },
  "version": { "number": 3 },
  "scope": { "unit_path": "U1.2" },
  "types": [ "... only the types of the scope parameters ..." ],
  "parameters": [ "... only the parameters of the scope, without name_en ..." ]
}
```

`scope_sha256` is the lower-case hex SHA-256 of the RFC 8785 (JCS) canonical form of the scope hashed body (ADR-0109 (u), (v)). The rules of section 6.1 about names, status, and export data apply. Two units of one version give the same `content_sha256` and two different values of `scope_sha256`.

Server rules:

- The system checks the whole version first. It rebuilds the hashed body of section 6.3 and compares its hash with the stored `content_sha256`. A difference gives `500 E-EXP-002`, and the system writes no audit entry (ADR-0109 (w)).
- Then the system filters the parameters, builds `types[]` of the filtered parameters, and computes `scope_sha256`.
- A unit without a parameter in the scope gives `404 E-EXP-005`. The client never gets an empty load (ADR-0109 (x)).
- The response sets `Content-Disposition: attachment; filename="recipe_{ref}_v{n}_{path}.json"` and `Cache-Control: no-store` (ADR-0109 (y)).
- Each export writes one `VersionExported` entry (A8). It holds the schema version `"2"`, `content_sha256`, `scope_unit_path`, and `scope_sha256` (ADR-0109 (ab)).

Consumer rule of v2. Before the transfer client loads a v2 document into a PLC, it MUST check `loadable`. It MUST also build the scope hashed body from the document with the array order of section 6.3. It MUST compute the SHA-256 of the canonical form and compare the result with `scope_sha256`. The client cannot rebuild `content_sha256` from a v2 document. It uses `content_sha256` only to find a change in any part of the version.

## 7. Session and CSRF

Sessions follow ADR-0014. The session store is the `session` table in PostgreSQL, owned by this application. No session library is used. `recipe-web` sets the session cookie from the result of the `Login` and `ChangeOwnPassword` use cases.

### 7.1 Cookie

| Attribute | Value |
|---|---|
| Name | `__Host-recipe_session` |
| `HttpOnly` | yes |
| `Secure` | yes. The server refuses to start when `session.secure` is `false`, except in a build with the `dev` feature |
| `SameSite` | `Strict` |
| `Path` | `/` |
| `Domain` | not set, as the `__Host-` prefix requires |
| Content | a random 128-bit token only. The database stores `SHA-256(token)` as `session.id`. No user data in the cookie |

The server sends `Strict-Transport-Security` with `max-age` set to `http.hsts_max_age` and `includeSubDomains` on every response (`reliability.md` 7.11).

The server reads `X-Forwarded-For` and `X-Forwarded-Proto` only from the trusted-proxy CIDR list in the configuration. A request from another address uses the socket address as `client_ip`. The server maps every address to its canonical form first, so `::ffff:a.b.c.d` and `a.b.c.d` are one address. The right-most entry of `X-Forwarded-For` that is not a trusted proxy is the client. An entry that is not an address gives `422 E-WEB-008`. A request from a trusted proxy without `X-Forwarded-For` gives `500 E-WEB-010` (ADR-0035).

The answer of the backstop of section 5.1 gets HSTS only from the defaults of Caddy (section 9, ADR-0057 (j)).

The default of the trusted-proxy list `http.trusted_proxies` is `172.28.0.2/32`, the address of `proxy` (ADR-0065 (j)). A request from another container of the Compose network therefore uses its socket address as `client_ip`. When a plant device stands in front of Caddy, its CIDR goes into the `trusted_proxies` of Caddy and into `http.trusted_proxies`. The runbook advises one `/32` for each device (ADR-0065 (m)).

### 7.2 Timeouts

| Setting | Default | Rule |
|---|---|---|
| Idle timeout | 30 min | The session ends when there is no request for this time |
| Absolute timeout | 12 h | The session ends this long after login, with or without activity |

The values are in `reliability.md` section 7.11 (A21). The idle and absolute limits are a security control (essential exception of WCAG 2.2.1 Timing Adjustable), so the UI shows no expiry warning or extension. A session that ends returns `401 E-WEB-004`. The supervised job `cleanup` deletes expired session rows every `session.cleanup_interval`.

A session row holds `user_id` and the `session_epoch` of the user at login. On every request the server reads `role`, `is_active`, `must_change_password`, and `session_epoch` from `app_user`. The session is invalid when the user is inactive or when the epoch differs. `SetUserRole`, `DeactivateUser`, `ResetPassword` delete the sessions of the user. `ChangeOwnPassword` deletes every session of the user and creates a new session for the current request. All deletions run in the transaction of the command.

### 7.3 CSRF

- `SameSite=Strict` is the first line of defense.
- Every mutating request from an authenticated session carries a token bound to the session. Forms send it in the hidden field `csrf_token`. HTMX requests send it in the header `X-CSRF-Token`. The `hx-headers` attribute on `<body>` sets the header. `Login` creates the token. `ChangeOwnPassword` creates a new session with a new token and returns `HX-Refresh: true`, so the browser loads the page again with the new token in `hx-headers`.
- `POST /login` and `POST /lang` without a session use a pre-session token: a signed double-submit cookie `__Host-recipe_csrf` and the field `csrf_pre`. `GET /login` sets the cookie. The key of the Docker secret `csrf_signing_key` signs it (`security.csrf_signing_key_file`, `reliability.md` 7.11).
- The pre-session cookie holds a 128-bit nonce and its HMAC-SHA256 under the first key. It has `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, and no `Max-Age`. The field `csrf_pre` holds the nonce. `POST /login` always accepts this token, also with a session (ADR-0035). `POST /lang` accepts the CSRF token of the session or a valid `csrf_pre`, with or without a session (ADR-0131).
- A mutating request is a request with any method except `GET`, `HEAD`, and `OPTIONS` (ADR-0035).
- Every mutating request must carry an `Origin` header that matches the site, or `Sec-Fetch-Site: same-origin`. An `Origin` matches the site when it equals `https://` followed by the `Host` header (ADR-0035). A request without either is rejected.
- A mutating request without a valid token or without a valid origin gets `403 E-WEB-007` and writes a `security_event` of kind `csrf_rejected` through the port `SecurityEventSink`. These events aggregate per client address and minute (`data-model.md` 3.8). When the event cannot be written, the request gets the storage error (ADR-0035).
- The export endpoints are `GET` and need no token.
- A guard runs before the use case on every export route. On the two session routes, it runs after the session layer and the gates. On the three transfer routes, it runs before the Basic check, so a rejected cross-site request runs no hash (ADR-0109 (p)). A request with a `Sec-Fetch-Site` header other than `same-origin` or `none` gets `403 E-WEB-007`. A request with `HX-Request: true` gets the same answer. The answer is the JSON body of section 5.3, and the guard writes one `csrf_rejected` event. The event kind aggregates per client address and minute. A request without `Sec-Fetch-Site` passes (ADR-0055).
- Session fixation: `Login` always creates a new session row and a new token. When the login request carries a session cookie, `Login` deletes that session row.

### 7.4 Login protection

The values of this section are in `reliability.md` section 7.11.

- Login delay: after 3 failed attempts for one `(login, client_ip)` pair in 15 minutes, an attempt within `2^(n-3)` seconds after the last failure gets `429 E-IDN-005` with `Retry-After`. The cap is 60 seconds. There is no permanent lockout (Assumption A22, ADR-0014).
- Per-IP budget: after 100 failed attempts from one `client_ip` in 15 minutes, every login from that address gets `429 E-IDN-005` with `Retry-After`. Parallel attempts with other logins can add at most `argon2.permits − 1` rows above the budget (ADR-0035).
- T1 records a throttled attempt as a failure, so the attempt starts a new delay. `Retry-After` of the login delay is the full delay of that attempt, counted from the attempt. `Retry-After` of the per-IP budget is the time until the oldest failure of the address leaves the window. Both values are whole seconds, rounded up, and at least 1 (ADR-0035).
- `ChangeOwnPassword` runs the same throttle for the own login and the client address. A wrong current password counts as a failed attempt (ADR-0035).
- Order: the use case takes an Argon2 permit and then runs T1 under the advisory lock of the login. T1 counts the failures in `login_attempt`, inserts the attempt row with `succeeded = false`, and commits.
- A throttled attempt releases the permit and gets 429. Otherwise the password check runs and the permit is released. T2 marks the attempt as a success and creates the session, or it writes a `security_event`.
- A failed login therefore always counts, also in read-only mode (`data-model.md` 6). After the per-IP budget is reached, T1 writes no attempt row, and the 429 comes from the count alone.
- `UnlockLogin` clears the counters of one login. It writes a `security_event` of kind `unlock`.
- A failed login gets one response for an unknown login, an inactive user, and a wrong password. The body, the status, and the response time are the same. The server hashes a dummy password when the user is not found or is inactive. The event log and the `security_event` row record the real reason.
- Every Argon2id call runs under a global semaphore with 4 permits and an acquire timeout of 2 seconds. A timeout returns `503 E-WEB-009` with `Retry-After`.
- Hash: Argon2id, `m = 19 MiB`, `t = 2`, `p = 1`. The hash is never logged.
- Password policy: 12 to 128 characters, no composition rule, and not present in the local breached-password list shipped with the image. Assumption A23.
- One-time password: `CreateUser` and `ResetPassword` generate a random password that meets the policy. The page shows it once. The command sets `must_change_password`, except for an `ExportClient` account (section 7.7). No request field carries a password for another user.
- `must_change_password`: a session of a user with the flag can open only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang`. Every other route returns `303` to `/me/password`. `GET /static/*` is outside this gate, because it needs no session (section 2.1).

### 7.5 Account state

A deactivated user gets the same `401 E-IDN-003` response as a wrong password (A42). The code `E-IDN-004` appears only in the event log and in `security_event`.

### 7.6 Body limits

| Route group | Limit |
|---|---|
| HTML forms | 256 KiB |
| The four routes that carry a parameter value: `POST .../parameters` (`AddParameter`, the value fields), `POST .../parameters/{key}/definition` (`UpdateParameterDefinition`, the value fields), `POST .../parameters/{key}/value` (`SetParameterValue`), and `POST .../rebind` (`RebindTypeRevision`, the fields `new_member_values`) | 4 MiB |
| `POST .../translations/{lang}` | 64 KiB |

The values are in `reliability.md` section 7.11. A request above the limit gets `413 E-WEB-003`. Each route has its own limit, also the buffer of the CSRF check (ADR-0039).

### 7.7 HTTP Basic of a technical account

The three transfer routes of section 2.7 use HTTP Basic (ADR-0109). The session cookie, the CSRF token, and the session store play no part. A `GET` request needs no token.

Parser of the header. The parser runs before any hash. It never logs the header. It gives `401 E-WEB-012` for each of these cases:

- More than one `Authorization` header, or a header longer than `export.basic_header_max`. The parser checks the length before it decodes.
- A scheme other than `Basic`. The parser ignores the case of the scheme.
- Text that is not standard Base64 with padding, or decoded bytes that are not UTF-8.
- Decoded text without `:`. The parser splits the text at the first `:`, so the password can hold `:`.
- A login outside `TypedLogin` (1 to 64 characters), a password outside 1 to 128 characters, or a control character in either part.

The Basic check is a use case of the identity module, not a command. It runs these steps:

1. It takes the Basic permit (`export.basic_permits`), then the Argon2 permit. Both use the acquire timeout of section 7.4.
2. It runs T1 of `Login` (section 7.4). A throttled check gives `429 E-IDN-005` with `Retry-After`. A permit timeout gives `503 E-WEB-009` with `Retry-After`.
3. It hashes the password in `spawn_blocking`. An unknown login, an inactive account, or a login outside `Login` gets the dummy hash (A42).
4. It refuses an account of a person after the hash check.
5. T2 locks the `app_user` row `FOR NO KEY UPDATE`. It checks again that the account is active, has the role `ExportClient`, and has the same password hash.
6. When T2 finds no change, it sets `succeeded = true`. T2 writes no audit entry and creates no session.

An unknown login, an inactive account, an account of a person, a changed state, and a wrong password give the same answer: `401 E-IDN-003`. The status, the body, and the hash work are the same (A42, ADR-0109 (m)). Each failure writes a `security_event` through `SecurityEventSink`. Its `detail` holds the reason and the scheme `basic`. T1 and T2 are an exception to the rule of one transaction per command, as T1 and T2 of `Login` are (ADR-0035 (b)).

The result is a `BasicPrincipal` with the `CurrentUser` of the account. It holds no session id, no session reference, no epoch, and no CSRF token (ADR-0109 (mb)). The audit entry of the export therefore has no session reference.

Answers:

- Every `401` of a transfer route has the header `WWW-Authenticate: RecipeExport realm="export"`. A browser does not know the scheme, so it shows no password dialog and keeps no credentials. The client sends `Authorization: Basic` on its first request (ADR-0109 (o)).
- `Login` refuses an `ExportClient` account after the password check with `401 E-IDN-003`. The `security_event` row has the reason `technical_account` (ADR-0109 (c)).
- In read-only mode, the three routes stay available. The Basic check keeps its `login_attempt` rows (ADR-0109 (ac)).
- Caddy handles TLS. The HTTP site of Caddy answers each path under `/export/` with `403` and no redirect (ADR-0109 (ad), (ae)).

## 8. Internationalization

- The plain cookie `recipe_lang` stores the UI language. The session does not store it. The default is `pl` (R10). `en` is the second supported language (A17).
- Cookie `recipe_lang`: value `pl` or `en`, `Max-Age` from `i18n.lang_cookie_max_age` (`reliability.md` 7.11), `Secure`, `SameSite=Strict`, `Path=/`. It holds no secret, so it needs no `HttpOnly`. The server checks the value on every request. Any other value counts as missing, and the UI uses `pl`.
- The server ignores `Accept-Language`. A plant workstation often has an English operating system and a Polish operator.
- `POST /lang` sets the cookie and returns `303` to the `next` field when it is a same-origin relative path. The path starts with `/`, and its second character is neither `/` nor `\`. The path holds no `\` and no control or white-space character (ADR-0035). Otherwise it returns `303` to `/recipes`. The server never redirects to the `Referer` header. `POST /lang` works before login and in read-only mode, because it writes no database row. The language change is not audited.
- Parameter names and descriptions show the translation for the UI language when one exists. Otherwise the English base text appears with a marker `(en)`. The rule applies to a parameter name or description without a label. A labeled field of the English base text shows the English base text in each UI language (ADR-0126 (f)). With the UI language `en`, the English base text appears without the marker (ADR-0126). In a page of another language, the English base text has `lang="en"`, and the marker stands outside it.
- Error messages, status words, and permission names come from `fluent` bundles. Error codes do not change with the language.

## 9. Security headers and HTMX hardening

Every response carries:

| Header | Value |
|---|---|
| `Content-Security-Policy` | `default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'` |
| `X-Content-Type-Options` | `nosniff` |
| `Referrer-Policy` | `same-origin` |
| `Cache-Control` | `no-store` on every authenticated page and fragment. `public, max-age` with the value of `static.cache_max_age` (`reliability.md` 7.11) on `GET /static/*` (section 2.1) |
| `Strict-Transport-Security` | `max-age` from `http.hsts_max_age` (`reliability.md` 7.11), `includeSubDomains` |
| `X-Request-Id` | the `request_id` of the request |

The application serves its fonts from `/static/fonts/`, so `default-src 'self'` covers them and the CSP has no `font-src`.

The health router of section 2.10 has the layer `request_id` and the security headers of this section (ADR-0059 (l)).

The answer of the backstop of section 5.1 is the one exception. It gets only `X-Request-Id` from `app`, and Caddy adds its defaults (ADR-0057 (j)).

Caddy sets HSTS, CSP, `nosniff`, and `Referrer-Policy` only as defaults. A default applies only when the answer has no value. The answers of `app` therefore keep their own values, and the answers of Caddy get the same headers. Caddy deletes the header `Server` (ADR-0065 (f), (g)).

HTMX configuration, set in a `<meta name="htmx-config">` tag: `allowEval: false`, `allowScriptTags: false`, `selfRequestsOnly: true`, `historyCacheSize: 0`, `methodsThatUseUrlParams: ["get"]`, `includeIndicatorStyles: false`, `responseHandling: [{"code":"204","swap":false},{"code":"[23]..","swap":true},{"code":"[45]..","swap":true,"error":true},{"code":"...","swap":false}]`. By default HTMX 2 does not swap a 4xx or 5xx response, and `HX-Retarget` does not change that. The value of `responseHandling` swaps it, so `conflict.html`, `field-errors.html`, and `error.html` reach the page (sections 4 and 5.3).

The value of `methodsThatUseUrlParams` keeps the parameters of every method other than `GET` in the body. No route uses `DELETE` (ADR-0040).

By default HTMX inserts an inline `<style>` element for its request indicator, and `style-src 'self'` blocks it. `includeIndicatorStyles: false` stops this. The rules for `.htmx-indicator` are in the static style sheet `static/app.css`.

The `<body>` tag has `hx-history="false"`. No page snapshot is stored in `localStorage` on a shared workstation.

Permitted client-side scripts. No other script exists in the application:

| File | Purpose | Limit |
|---|---|---|
| `static/htmx.min.js` | HTMX 2, vendored and pinned (ADR-0001) | unchanged upstream file |
| `static/row-version.js` | Updates hidden `row_version` fields from `HX-Trigger` (section 4) | 20 lines |
| `static/tree.js` | Expand and collapse of the unit tree, the type tree, and the groups of the leaf grid (ADR-0039), keyboard focus | 50 lines |

Rules for templates:

- User data never appears inside an `hx-*` attribute, a `<script>` tag, or an inline event handler.
- The askama `|safe` filter is not used on user data. A review rejects any `|safe` on a value that is not a literal.
- The HTTP test suite runs an XSS payload corpus through every text field and checks the rendered pages.
