# HTTP API

Status: Accepted (design stage, revised 2026-10-12 after review)
Source of requirements: `general-draft.txt`
Related: `domain-model.md`, `data-model.md`, `reliability.md`, ADR-0001, ADR-0008, ADR-0013, ADR-0014, ADR-0015, ADR-0016, ADR-0018, ADR-0019, ADR-0021, ADR-0022

## 1. Scope

This document defines the HTTP surface of `recipe-web`. It is a hypermedia API (ADR-0001).

- A full page request returns `text/html`.
- An HTMX request (`HX-Request: true`) returns an HTML fragment.
- The export endpoints return `application/json`. No other public endpoint returns JSON.
- The health endpoints live on a separate internal listener (section 2.10). They are not part of the public surface.

URL policy:

- HTML routes have no version prefix. Example: `/recipes/R-100/versions/3`.
- The JSON export lives under `/export/v1/`. A breaking change of the export document creates `/export/v2/` (ADR-0019).

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
| `{lang}` | `LangCode` | `422 E-RCP-027` for a malformed code. `404 E-WEB-005` for a code of a version that does not exist. `404 E-RCP-050` for a well-formed code that the existing draft does not hold |
| `{login}` | `Login` | `404 E-WEB-005` |
| `{id}` | audit entry id | `404 E-WEB-005` |

No path value holds an S7 address.

Exemption from the `E-WEB-005` rule. `E-WEB-005 NotFound` is for an unknown `{ref}`, `{n}`, `{code}`, or `{login}`, a malformed `{key}` (not a UUID), and a `{key}` or `{lang}` of a version that does not exist. `E-RCP-051 ParameterNotFound` and `E-RCP-050 TranslationNotFound` are for a well-formed `{key}` or `{lang}` that is absent from an existing draft. The draft commands of section 2.6 return them. The two DELETE rows of section 2.6 cite both codes with this rule.

Request fields. `E-WEB-005` is for URL values only. A request field that names a machine that does not exist returns `404 E-PLT-019 MachineNotFound`, for example `machine_code` of `CreateRecipe`. A request field that names a unit that does not exist returns `404 E-PLT-020 UnitNotFound`, for example `unit_path` of `AddParameter`. A missing parent in `AddUnit` returns `422 E-PLT-003 ParentNotFound`.

Mutating requests use `POST`, `PUT`, or `DELETE`. HTMX sends `PUT` and `DELETE` with `hx-put` and `hx-delete`. Every mutating request carries the CSRF token (section 7). Every mutating request on a `Draft` carries `row_version` (section 4).

The use case names in the tables are the commands from `domain-model.md` section 5 and ADR-0016, and the queries of `domain-model.md` 5.5. A query is also a use case. It checks its permission first, in the same way as a command (R28). Every `GET` row of the tables of section 2 names its query use case, except the two export rows, which run the command `ExportVersion`. A form or preview query has the permission of the command that it prepares.

## 2. Endpoints

Columns: Method, Path, Use case, Permission, Request, Success, Errors. The error column lists the HTTP status and the error code from `reliability.md` section 1.2. Every endpoint can also return these codes:

- `401 E-WEB-004` (no session), `403 E-WEB-002` (no permission), `403 E-WEB-007` (CSRF).
- `413 E-WEB-003` (request too large), `422 E-WEB-008` (malformed field).
- `503 E-WEB-006` (read-only mode), `503 E-WEB-009` (timeout), `500 E-WEB-010` (internal error), `500 E-WEB-011` (a `spawn_blocking` task failed).
- The storage codes `503 E-DB-001`, `503 E-DB-002`, `409 E-DB-003`, `503 E-DB-004`, `503 E-DB-005`, `409 E-DB-006`, `409 E-DB-007`, `500 E-DB-008`.
- The code of a bad path value (section 1).

The tables do not repeat these codes. In read-only mode every command route returns `E-WEB-006`, except `POST /login`, `POST /logout`, and the two export routes of section 2.7 (section 5.1).

### 2.1 Auth and language

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/login` | `GetLoginForm` | none | - | `200` login page. Sets the pre-session CSRF cookie (section 7.3) | - |
| POST | `/login` | `Login` | none | `login`, `password`, `csrf_pre` | `303` to `/recipes`. `303` to `/me/password` when `must_change_password` is set. Sets the session cookie | `401 E-IDN-003`, `429 E-IDN-005`, `303 E-IDN-007` (the redirect to `/me/password`), `E-IDN-004` (only in the event log and `security_event`. The client gets `401 E-IDN-003`) |
| POST | `/logout` | `Logout` | authenticated | CSRF token | `303` to `/login`. Deletes the session row and clears the cookie | - |
| POST | `/lang` | - (sets the cookie `recipe_lang`, section 8) | none | `lang` in `pl` or `en`, `csrf_pre` or CSRF token, `next` | `303` to `next` when `next` is a same-origin relative path. Otherwise `303` to `/recipes`. Sets the cookie `recipe_lang`. Works without a session and in read-only mode | - |

The response body and the response time of `POST /login` are the same for an unknown login, an inactive user, and a wrong password (section 7.4, A42).

Static files. `GET /static/*` serves the files of section 9 and `static/app.css`. The route needs no session and has no use case. It is outside the `must_change_password` gate of section 7.4. The response sets `Cache-Control: public, max-age` with the value of `static.cache_max_age` (`reliability.md` 7.11).

### 2.2 Machines and units

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines` | `ListMachines` | `RecipeView` | `q`, `page`, `show_archived` | `200` list of live machines. `show_archived=1` adds the archived machines, each with a link to `/machines/by-id/{uuid}` | - |
| GET | `/machines/new` | `GetMachineForm` | `MachineManage` | - | `200` form | - |
| POST | `/machines` | `CreateMachine` | `MachineManage` | `code`, `name`, `plc_platform`, `main_unit_name` | `303` to `/machines/{code}` | `409 E-PLT-001` |
| GET | `/machines/{code}` | `GetMachine` | `RecipeView` | - | `200` page of the live machine with this code, with the unit tree | `404 E-WEB-005` |
| GET | `/machines/by-id/{uuid}` | `GetMachine` | `RecipeView` | - | `200` machine page of a live or an archived machine, with the unit tree. Read-only for an archived machine | `404 E-WEB-005` |
| POST | `/machines/{code}/rename` | `RenameMachine` | `MachineManage` | `name` | `200` fragment | `404 E-WEB-005`, `409 E-PLT-010` |
| POST | `/machines/{code}/archive` | `ArchiveMachine` | `MachineManage` | CSRF token | `303` to `/machines` | `404 E-WEB-005`, `409 E-PLT-011` |
| GET | `/machines/{code}/units` | `GetUnitTree` | `RecipeView` | - | `200` unit tree fragment | `404 E-WEB-005` |
| GET | `/machines/{code}/units/new` | `GetUnitForm` | `UnitManage` | optional `parent_path` | `200` form | `404 E-WEB-005` |
| POST | `/machines/{code}/units` | `AddUnit` | `UnitManage` | `parent_path` (empty for a main unit), `segment`, `name` | `200` tree fragment | `404 E-WEB-005`, `422 E-PLT-002`, `422 E-PLT-003`, `409 E-PLT-004`, `409 E-PLT-009`, `409 E-PLT-010`, `422 E-PLT-017` |
| GET | `/machines/{code}/units/{path}` | `GetUnit` | `RecipeView` | - | `200` unit detail | `404 E-WEB-005` |
| POST | `/machines/{code}/units/{path}/rename` | `RenameUnit` | `UnitManage` | `name` | `200` fragment | `404 E-WEB-005`, `409 E-PLT-009` |
| POST | `/machines/{code}/units/{path}/archive` | `ArchiveUnit` | `UnitManage` | CSRF token | `200` tree fragment. The unit and its descendants are archived | `404 E-WEB-005`, `409 E-PLT-012`, `409 E-PLT-014`, `409 E-PLT-008` |

`/machines/{code}` and every route under it resolve to the live machine with that code. An archived machine keeps its history at `/machines/by-id/{uuid}`, because a new machine can use its code (A32, ADR-0021).

`ArchiveMachine` is blocked while a recipe of the machine has a live `Draft` (`E-PLT-011`). The recipes stay readable. `ArchiveUnit` is blocked while the live `Draft` of a recipe references the unit or a descendant (`E-PLT-012`). It is also blocked while the `Approved` version of a recipe references them (`E-PLT-014`), and when the unit is the last live main unit (`E-PLT-008`).

### 2.3 Types

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines/{code}/types` | `ListTypes` | `RecipeView` | - | `200` list of UDTs, latest revision first | `404 E-WEB-005` |
| GET | `/machines/{code}/types/new` | `GetUdtForm` | `UdtManage` | - | `200` form | `404 E-WEB-005` |
| POST | `/machines/{code}/types` | `DefineUdt` | `UdtManage` | `name`, `members[i].name`, `members[i].type` (a `TypeExpr`) | `303` to the new type | `422 E-PLT-005`, `422 E-PLT-006`, `422 E-PLT-013`, `409 E-PLT-015`, `422 E-PLT-016`, `422 E-PLT-018`, `422 E-PLT-021` |
| GET | `/machines/{code}/types/{name}` | `GetTypeTree` | `RecipeView` | optional `rev` | `200` type tree of the latest revision, or of `rev` | `404 E-WEB-005` |
| POST | `/machines/{code}/types/{name}/revisions` | `DefineUdt` | `UdtManage` | same as `POST /types`. The server sets `supersedes` | `303` to the new revision | same as `POST /types` |
| GET | `/machines/{code}/types/{name}/usage` | `GetTypeUsage` | `RecipeView` | optional `rev` | `200` list of recipes and parameters that use the revision, directly or through another type | `404 E-WEB-005` |

A UDT is immutable. A second `POST` creates a new revision (ADR-0006). The usage query is the "where used" view that the rebind workflow needs (ADR-0018).

The fields `members[i].type` of `DefineUdt` and `type` of section 2.6 carry a `TypeExpr` (`domain-model.md` 2.1). Examples: `INT`, `STRING(20)`, `ST_Zone@3`, `ARRAY[1..8] OF REAL`, `ARRAY[0..1,1..2] OF ST_Zone@3`. The internal operation `EnsureDerivedType` resolves the expression and creates a missing `STRING(n)`, `WSTRING(n)`, or array type (`domain-model.md` 5.1). A bad expression returns `422 E-PLT-013`. The exception is a member of `DefineUdt` whose type is not on the platform, which returns `422 E-PLT-006`. A type above a size limit returns `422 E-PLT-018` (`domain-model.md` 2.1).

### 2.4 Recipes

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes` | `ListRecipes` | `RecipeView` | `q`, `machine`, `status`, `page`, `show_deleted` | `200` list. Deleted recipes are hidden. `show_deleted=1` works only with `RecipeRestore` | - |
| GET | `/recipes/new` | `GetRecipeForm` | `RecipeCreate` | - | `200` form | - |
| POST | `/recipes` | `CreateRecipe` | `RecipeCreate` | `machine_code`, `reference_number`, `name`, `change_note` | `303` to `/recipes/{ref}/versions/1` | `409 E-RCP-001`, `409 E-PLT-010`, `404 E-PLT-019` |
| GET | `/recipes/{ref}` | `GetRecipe` | `RecipeView` | - | `200` recipe page with version list | `404 E-WEB-005` |
| POST | `/recipes/{ref}/rename` | `RenameRecipe` | `RecipeEdit` | `name` | `200` fragment | `404 E-WEB-005`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/delete` | `DeleteRecipe` | `RecipeDelete` (conditional, section 3) | CSRF token | `303` to `/recipes` | `404 E-WEB-005`, `409 E-RCP-010`, `409 E-RCP-026`, `409 E-RCP-012` |
| POST | `/recipes/{ref}/restore` | `RestoreRecipe` | `RecipeRestore` | CSRF token | `303` to `/recipes/{ref}` | `404 E-WEB-005`, `409 E-RCP-025`, `409 E-PLT-010` |

`DeleteRecipe` discards the live `Draft` in the same transaction.

### 2.5 Versions

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes/{ref}/versions` | `ListVersions` | `RecipeView` | - | `200` version list fragment | `404 E-WEB-005` |
| GET | `/recipes/{ref}/versions/new` | `GetVersionForm` | `VersionCreate` | - | `200` form with `based_on` and `change_note` | `404 E-WEB-005` |
| POST | `/recipes/{ref}/versions` | `CreateVersion` | `VersionCreate` | `based_on` (version number, optional), `change_note` | `303` to the new `Draft`. The draft page lists only the outdated parameters, which need a rebind. A base version can reference an archived unit, an archived machine, or a type that is not on the platform. Then the server returns `422 E-RCP-020` with the items and creates no draft | `404 E-WEB-005`, `409 E-RCP-011`, `409 E-RCP-012`, `409 E-PLT-010`, `422 E-RCP-020`, `422 E-RCP-021` |
| GET | `/recipes/{ref}/versions/{n}` | `GetVersion` | `RecipeView` | `page` for the parameter table | `200` version page. Editable forms only for a `Draft` and a user with `ParameterEdit` | `404 E-WEB-005` |
| GET | `/recipes/{ref}/versions/{n}/approve` | `GetApprovalPreview` | `VersionApprove` | - | `200` approve page. Shows the diff against the current `Approved` version. Limit changes are highlighted. Lists the outdated parameters and offers the flag `allow_outdated` | `404 E-WEB-005`, `409 E-RCP-007` |
| POST | `/recipes/{ref}/versions/{n}/approve` | `ApproveVersion` | `VersionApprove` | `row_version`, `allow_outdated` (default `false`), CSRF token | `200` version page. Shows the retired version number when one exists. An already approved version with `row_version == expected + 1` returns the same page without a change | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-RCP-015`, `409 E-WEB-001`, `422 E-RCP-008`, `422 E-RCP-016`, `422 E-RCP-020`, `422 E-RCP-022`, `422 E-RCP-023`, `422 E-RCP-024`, `500 E-EXP-004` |
| POST | `/recipes/{ref}/versions/{n}/retire` | `RetireVersion` | `VersionRetire` | `row_version`, CSRF token | `200` version page | `404 E-WEB-005`, `409 E-RCP-009`, `409 E-RCP-012`, `409 E-WEB-001` |
| POST | `/recipes/{ref}/versions/{n}/discard` | `DiscardDraft` | `VersionDiscard` | `row_version`, CSRF token | `303` to `/recipes/{ref}` | `404 E-WEB-005`, `409 E-RCP-007`, `409 E-RCP-012`, `409 E-WEB-001` |
| POST | `/recipes/{ref}/versions/{n}/rebind` | `RebindTypeRevision` | `ParameterEdit` | `row_version`, `from_type_id`, `to_type_id`, `new_member_values` (JSON text) | `200` parameter table fragment | `404 E-WEB-005`, `409 E-RCP-002`, `409 E-RCP-012`, `409 E-WEB-001`, `422 E-RCP-005`, `422 E-RCP-021`, `422 E-RCP-028`, `422 E-RCP-029`, `422 E-RCP-052`, `422 E-PLT-013`, `422 E-PLT-018` |
| GET | `/recipes/{ref}/versions/{n}/diff/{m}` | `DiffVersions` | `RecipeView` | - | `200` parameter diff between versions `n` and `m` | `404 E-WEB-005` |

A version with `based_on` empty starts with no parameters. The approve `POST` is idempotent (section 4). It returns `422 E-RCP-024` for an outdated parameter unless `allow_outdated` is `true`. The audit entry `VersionApproved` records the flag (A29, ADR-0018).

`RebindTypeRevision` moves every parameter of the draft whose type uses `from_type_id` to `to_type_id` (`domain-model.md` 5.2). A failed rebind returns one `E-RCP-028` or `E-RCP-029` line per failing parameter and changes nothing. The field `new_member_values` is the JSON text of a map from `(parameter_key, leaf_path)` to a value: a decimal text, a string, or a bool. The JSON object has one member per parameter key, and each member maps a leaf path to the value. A new member leaf without an entry gives `E-RCP-028`.

### 2.6 Parameters

All routes in this group are under `/recipes/{ref}/versions/{n}`. A mutating route works only when version `n` is a live `Draft` (I3). Otherwise the server returns `409 E-RCP-002`. Every mutating route in this group can also return `409 E-WEB-001`, and `409 E-RCP-012` when the recipe is deleted. Every mutating route with `{key}` can also return `404 E-RCP-051` when the draft does not hold the key (section 1). The tables do not repeat these codes, except in the two DELETE rows.

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `.../parameters` | `ListParameters` | `RecipeView` | `unit` (path filter), `page` | `200` parameter table fragment, grouped by unit, 100 rows per page | `404 E-WEB-005` |
| GET | `.../parameters/new` | `GetParameterForm` | `ParameterEdit` | `unit` | `200` empty parameter form. The server generates `key` at `AddParameter` (A40) | `404 E-WEB-005` |
| POST | `.../parameters` | `AddParameter` | `ParameterEdit` | `row_version`, `unit_path`, `tag`, `name_en`, `description_en`, `type` (a `TypeExpr`), `s7_address`, `ordinal`, `value` (JSON text, required), `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` (`limits[]` can be empty) | `200` parameter row fragment with the new `key` | `409 E-RCP-014`, `422 E-RCP-003`, `422 E-RCP-004`, `422 E-RCP-005`, `422 E-RCP-006`, `422 E-RCP-017`, `422 E-RCP-018`, `422 E-RCP-019`, `422 E-RCP-021`, `422 E-RCP-022`, `422 E-RCP-023`, `422 E-RCP-053`, `422 E-RCP-054`, `409 E-PLT-009`, `422 E-PLT-013`, `422 E-PLT-018`, `404 E-PLT-020` |
| GET | `.../parameters/{key}` | `GetParameter` | `RecipeView` | - | `200` parameter detail or edit form | `404 E-WEB-005` |
| POST | `.../parameters/{key}/definition` | `UpdateParameterDefinition` | `ParameterEdit` | `row_version`, `unit_path`, `tag`, `name_en`, `description_en`, `type` (a `TypeExpr`), `s7_address`, `ordinal`. When `type` names a different type than the current one: also `value` and `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length`, all required. Otherwise neither `value` nor `limits[]` | `200` parameter row fragment | same as `AddParameter`. A missing `value` after a type change is `422 E-RCP-005` |
| POST | `.../parameters/{key}/value` | `SetParameterValue` | `ParameterEdit` | `row_version`, `value` (JSON text) | `200` value fragment | `422 E-RCP-004`, `422 E-RCP-005`, `422 E-RCP-021` |
| GET | `.../parameters/{key}/limits` | `GetLimits` | `RecipeView` | - | `200` limits table fragment, one row per leaf path. `[*]` is one row for all elements of an array | `404 E-WEB-005` |
| POST | `.../parameters/{key}/limits` | `ReplaceLimits` | `ParameterEdit` | `row_version`, `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` | `200` limits fragment | `422 E-RCP-004`, `422 E-RCP-006`, `422 E-RCP-017`, `422 E-RCP-018`, `422 E-RCP-019`, `422 E-RCP-021` |
| GET | `.../parameters/{key}/translations` | `GetTranslations` | `RecipeView` | - | `200` translation table fragment | `404 E-WEB-005` |
| PUT | `.../parameters/{key}/translations/{lang}` | `UpsertTranslation` | `ParameterEdit` | `row_version`, `name`, `description` | `200` translation fragment | `422 E-RCP-027` |
| DELETE | `.../parameters/{key}/translations/{lang}` | `RemoveTranslation` | `ParameterEdit` | `row_version`, CSRF token | `200` translation fragment | `404 E-WEB-005` (malformed `{key}` or missing version), `404 E-RCP-051` (`{key}` not in the draft), `404 E-RCP-050` (`{lang}` not in the draft) |
| DELETE | `.../parameters/{key}` | `RemoveParameter` | `ParameterEdit` | `row_version`, CSRF token | `200` parameter table fragment | `404 E-WEB-005` (malformed `{key}` or missing version), `404 E-RCP-051` (`{key}` not in the draft) |

Rules:

- A command touches only its own rows. `SetParameterValue` does not read or write `parameter_limit` or `parameter_translation`. `ReplaceLimits` replaces all limit rows of one parameter and nothing else. `UpsertTranslation` writes one language.
- The form identifies a parameter by `key`. A `tag` can change through `UpdateParameterDefinition`. The parameter page shows the current `tag`.
- The `value` field is the JSON text of a `ParameterValue`. The form builds it from one input per leaf. The server parses it as a whole. A partial value is `422 E-RCP-005`. A request with a `value` field has the larger body limit of section 7.6.
- A `duration` or `instant` leaf in `value`, `limits[i].min`, `limits[i].max`, and `new_member_values` is an IEC literal, for example `T#1s` (`domain-model.md` 2.1). The server stores the count of the base unit.
- `AddParameter` always carries `value`, and `limits[]` can be empty. `UpdateParameterDefinition` carries `value` and `limits[]` only when the type changes. Then both replace the old value and the old limits. Otherwise the value and the limits stay unchanged. `SetParameterValue` and `ReplaceLimits` change one of them without a type change.
- The parameter table is paginated. A `Draft` with more than 100 parameters shows one unit per page by default.

### 2.7 Export

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/export/v1/recipes/{ref}/approved` | `ExportVersion` | `VersionExport` | - | `200 application/json`. The `Approved` version. `loadable` is `true` | `404 E-WEB-005` no such recipe, `404 E-EXP-001` no approved version (a deleted recipe has none, I14), `500 E-EXP-002` content hash mismatch, `500 E-EXP-004` serializer error |
| GET | `/export/v1/recipes/{ref}/versions/{n}` | `ExportVersion` | `VersionExportHistory` | - | `200 application/json`. An `Approved` or `Retired` version. `loadable` is `true` only for `Approved` | `404 E-WEB-005` no such recipe or version, `409 E-EXP-003` version is a `Draft` or discarded, `409 E-RCP-012` recipe deleted, `500 E-EXP-002`, `500 E-EXP-004` |

The export endpoints use the session cookie (ADR-0014). A `Draft` is never exportable. Each successful export writes one audit entry (A8). Section 6 defines the document. The second route exists for rollback and audit use. Operators do not have `VersionExportHistory` (section 3).

### 2.8 Users

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/users` | `ListUsers` | `UserManage` | `q`, `page` | `200` list | - |
| GET | `/users/new` | `GetUserForm` | `UserManage` | - | `200` form | - |
| POST | `/users` | `CreateUser` | `UserManage` | `login`, `role` | `200` page of the new user with a one-time password, shown once. `must_change_password` is set | `409 E-IDN-001` |
| GET | `/users/{login}` | `GetUser` | `UserManage` | - | `200` user page with lock state | `404 E-WEB-005` |
| POST | `/users/{login}/role` | `SetUserRole` | `UserManage` | `role` | `200` fragment. All sessions of the user are deleted | `404 E-WEB-005`, `409 E-IDN-002` |
| POST | `/users/{login}/deactivate` | `DeactivateUser` | `UserManage` | CSRF token | `200` fragment. All sessions of the user are deleted | `404 E-WEB-005`, `409 E-IDN-002` |
| POST | `/users/{login}/activate` | `ActivateUser` | `UserManage` | CSRF token | `200` fragment | `404 E-WEB-005`, `409 E-IDN-009` |
| POST | `/users/{login}/reset-password` | `ResetPassword` | `UserManage` | CSRF token | `200` fragment with a one-time password, shown once. `must_change_password` is set. All sessions of the user are deleted | `404 E-WEB-005` |
| POST | `/users/{login}/unlock` | `UnlockLogin` | `LoginUnlock` | CSRF token | `200` fragment. The login delay counters of the user are cleared | `404 E-WEB-005` |
| GET | `/me` | `GetOwnProfile` | authenticated | - | `200` own profile | - |
| GET | `/me/password` | `GetOwnPasswordForm` | authenticated | - | `200` password form. The only page a session with `must_change_password` can open | - |
| POST | `/me/password` | `ChangeOwnPassword` | authenticated | `current_password`, `password` | `200` fragment with the header `HX-Refresh: true`. Every session of the user is deleted, and a new session replaces the current one. `must_change_password` is cleared. The browser loads the page again and gets the new CSRF token (section 7.3) | `422 E-IDN-006`, `422 E-IDN-008` |

The administrator never chooses a password for another user. `CreateUser` and `ResetPassword` generate the password (ADR-0014). Deactivate is a soft delete (ADR-0021). A deactivated user keeps the audit history and cannot log in.

### 2.9 Audit and security events

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/audit` | `ListAuditEntries` | `AuditView` or `AuditViewRecipes` | `aggregate`, `entity_type`, `entity_id`, `actor`, `action`, `from`, `to`, `before` (cursor) | `200` audit list, newest first. A user with only `AuditViewRecipes` sees entries with `aggregate_type = 'recipe'`: the entity types `recipe`, `recipe_version`, and `parameter`. A limit or translation change is a `parameter` entry (`data-model.md` 10) | - |
| GET | `/audit/{id}` | `GetAuditEntry` | same as above | - | `200` one entry with `before` and `after` as formatted JSON | `404 E-WEB-005` |
| GET | `/recipes/{ref}/history` | `GetRecipeHistory` | `RecipeView` | `before` (cursor) | `200` audit entries with `aggregate_id` of the recipe, without `before` and `after` bodies | `404 E-WEB-005` |
| GET | `/security-events` | `ListSecurityEvents` | `AuditView` | `kind`, `login`, `client_ip`, `from`, `to`, `before` | `200` list of security events | - |

The `/recipes/{ref}/history` route is the recipe timeline. Every role can open it (R24). It does not show user data or machine changes. The audit list uses cursor pagination on the entry id (`data-model.md` section 3.15).

### 2.10 Health

The health endpoints are not on the public listener. `recipe-server` opens a second listener on port 9000 on the internal Docker network. It serves `GET /healthz` and `GET /readyz`. Both return `text/plain` with the body `ok` or `not ready`. The proxy answers `404` for `/healthz*` and `/readyz*` on the public site. `reliability.md` section 7.2 defines the checks.

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
- A role maps to a fixed set of permissions. There are no per-user overrides (ADR-0013).
- A generated integration test checks every (operation, role) pair of the matrix below, for commands and for queries (`reliability.md` section 6).
- Last administrator rule: `SetUserRole` and `DeactivateUser` return `Err(LastAdministrator)` when the target is the last active user with role `Administrator`. The use case counts active administrators in a `SERIALIZABLE` transaction (`data-model.md` section 6). There is no database trigger for this rule.
- Separation of duties: `ApproveVersion` returns `Err(ApproverEditedDraft)` when the approver is `created_by` of the draft or has a row in `draft_editor` for it. The configuration value `approval.separation_of_duties` is `true` by default (A34). The server stores it in `app_setting`. A change of the value writes the audit entry `SettingChanged` at the next start (`reliability.md` 7.12).
- `EnsureDerivedType` is an internal operation of the Plant Model, not a use case. It has no permission and no matrix row. The calling use case (`AddParameter`, `UpdateParameterDefinition`, `DefineUdt`, `RebindTypeRevision`) checks its own permission first.
- A `Forbidden` result writes one `security_event` row in its own transaction through the port `SecurityEventSink` (A33). The use case has no business transaction at that point.

### 3.3 Matrix

Legend: ✓ allowed, ✗ denied, (c) allowed with the condition in the last column.

| Operation | Permission | Administrator | ProcessEngineer | Operator | Condition |
|---|---|---|---|---|---|
| View recipes, versions, parameters, machines, units, UDTs, type usage. Queries `ListMachines`, `GetMachine`, `GetUnitTree`, `GetUnit`, `ListTypes`, `GetTypeTree`, `GetTypeUsage`, `ListRecipes`, `GetRecipe`, `ListVersions`, `GetVersion`, `DiffVersions`, `ListParameters`, `GetParameter`, `GetLimits`, `GetTranslations` | `RecipeView` | ✓ | ✓ | ✓ | - |
| View recipe history (`/recipes/{ref}/history`). Query `GetRecipeHistory` | `RecipeView` | ✓ | ✓ | ✓ | - |
| Export the `Approved` version (`ExportVersion`) | `VersionExport` | ✓ | ✓ | ✓ | A4 |
| Export a `Retired` version by number (`ExportVersion`) | `VersionExportHistory` | ✓ | ✓ | ✗ | ADR-0019 |
| Create recipe (`CreateRecipe`). Form query `GetRecipeForm` | `RecipeCreate` | ✓ | ✓ | ✗ | R25 |
| Rename recipe (`RenameRecipe`) | `RecipeEdit` | ✓ | ✓ | ✗ | R25 |
| Delete recipe, soft (`DeleteRecipe`) | `RecipeDelete` | (c) | (c) | ✗ | ProcessEngineer: only when the recipe has no `Approved` and no `Retired` version (`E-RCP-026`). Administrator: only when the recipe has no `Approved` version (`E-RCP-010`, I14). R23, A13 |
| Restore recipe (`RestoreRecipe`). List deleted recipes (`ListRecipes` with `show_deleted`) | `RecipeRestore` | ✓ | ✗ | ✗ | ADR-0021 |
| Create version, a new `Draft` (`CreateVersion`). Form query `GetVersionForm` | `VersionCreate` | ✓ | ✓ | ✗ | R25, A7 |
| Add, edit, delete parameters, limits, translations of a `Draft`. Rebind a type revision (`AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, `RemoveParameter`, `RebindTypeRevision`). Form query `GetParameterForm` | `ParameterEdit` | ✓ | ✓ | ✗ | R09, R25 |
| Approve version (`ApproveVersion`). Preview query `GetApprovalPreview` | `VersionApprove` | (c) | (c) | ✗ | A5, A34. Not the creator or an editor of the draft (`E-RCP-015`). The approval retires the previous `Approved` version (R27) |
| Retire version without replacement (`RetireVersion`) | `VersionRetire` | ✓ | ✗ | ✗ | A28 |
| Discard `Draft` (`DiscardDraft`) | `VersionDiscard` | ✓ | ✓ | ✗ | - |
| Create, rename, archive machine (`CreateMachine`, `RenameMachine`, `ArchiveMachine`). Form query `GetMachineForm` | `MachineManage` | ✓ | ✓ | ✗ | A9 |
| Add, rename, archive unit (`AddUnit`, `RenameUnit`, `ArchiveUnit`). Form query `GetUnitForm` | `UnitManage` | ✓ | ✓ | ✗ | A9 |
| Define UDT revision (`DefineUdt`). Form query `GetUdtForm` | `UdtManage` | ✓ | ✓ | ✗ | A9 |
| Create user, set role, activate, deactivate, reset password (`CreateUser`, `SetUserRole`, `ActivateUser`, `DeactivateUser`, `ResetPassword`). Form query `GetUserForm`. Queries `ListUsers`, `GetUser` | `UserManage` | ✓ | ✗ | ✗ | A9. Last administrator rule |
| Unlock a login (`UnlockLogin`) | `LoginUnlock` | ✓ | ✗ | ✗ | ADR-0014 |
| Log in (`Login`). Form query `GetLoginForm` | none | ✓ | ✓ | ✓ | No session needed |
| Log out (`Logout`) | authenticated | ✓ | ✓ | ✓ | - |
| Change own password (`ChangeOwnPassword`). Form query `GetOwnPasswordForm`. Own profile query `GetOwnProfile` | authenticated | ✓ | ✓ | ✓ | - |
| View full audit log and security events. Queries `ListAuditEntries`, `GetAuditEntry`, `ListSecurityEvents` | `AuditView` | ✓ | ✗ | ✗ | A20 |
| View audit entries of recipes, versions, parameters. Queries `ListAuditEntries`, `GetAuditEntry` | `AuditViewRecipes` | ✓ | ✓ | ✗ | A20 |

Role to permission mapping:

| Role | Permissions |
|---|---|
| Administrator | every permission |
| ProcessEngineer | `RecipeView`, `VersionExport`, `VersionExportHistory`, `RecipeCreate`, `RecipeEdit`, `RecipeDelete` (c), `VersionCreate`, `ParameterEdit`, `VersionApprove` (c), `VersionDiscard`, `MachineManage`, `UnitManage`, `UdtManage`, `AuditViewRecipes` |
| Operator | `RecipeView`, `VersionExport` |

## 4. Concurrency protocol

The protocol implements ADR-0008 and ADR-0015 for every `Draft`.

1. `GET /recipes/{ref}/versions/{n}` renders the current `row_version` of the version into a hidden field `row_version` of every form on the page.
2. Every mutating request on that version sends `row_version`.
3. The use case locks the recipe row and then the version row with `FOR UPDATE` and compares `row_version` with the sent value. A mismatch rolls back and returns `Err(ConcurrentEdit { reference, number, expected, actual })`. The application never sets `row_version`. The database trigger function `bump_row_version` increments it on every update of the version row (`data-model.md` section 4.4).
4. On success the response carries the new value in the header `HX-Trigger: {"rowVersion": <new>}`. The static script `row-version.js` updates every hidden `row_version` field on the page.
5. On conflict the server returns `409 E-WEB-001` with the fragment `conflict.html`. The fragment shows: the current server `row_version`, the login of the last editor, the time of the last change, and one button "Reload version". The button does `hx-get` on the version page. The unsaved input stays in the form until the reload.
6. The `409` response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML`. The original target of the request does not change. HTMX swaps the `409` fragment because of the `responseHandling` value of section 9.
7. Every approve and save form has `hx-sync="closest form:drop"`. A second click while a request is in flight is dropped in the browser.
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
| Wrong credentials (`E-IDN-003`) | `401` | `field-errors.html` on the login form. No redirect |
| Wrong current password (`E-IDN-008`) | `422` | `field-errors.html` on the password form |
| Missing resource in the URL (`E-WEB-005`, `E-EXP-001`, `E-RCP-050`, `E-RCP-051`) or in a request field (`E-PLT-019`, `E-PLT-020`) | `404` | `error.html` |
| Request too large (`E-WEB-003`) | `413` | `error.html` |
| Login throttled (`E-IDN-005`) | `429` | `error.html` with `Retry-After` |
| Read-only mode (`E-WEB-006`). Every command except `Login`, `Logout`, and `ExportVersion` gets it. Queries, the two export routes, `POST /login`, `POST /logout`, and `POST /lang` stay available (`reliability.md` 7.8) | `503` | `error.html` with the banner text |
| Timeout, deadlock, pool exhausted, database unavailable (`E-WEB-009`, and the `StorageError` codes `E-DB-001`, `E-DB-002`, `E-DB-004`, `E-DB-005`) | `503` | `error.html` with `Retry-After` set to `http.retry_after` (`reliability.md` 7.11) |
| Export failure: content hash mismatch or serializer error (`E-EXP-002`, `E-EXP-004`) | `500` | JSON error body on the export routes. `ApproveVersion` returns `E-EXP-004` with `error.html` |
| Unexpected, caught panic, failed `spawn_blocking` task, unexpected database error (`E-WEB-010`, `E-WEB-011`, `E-AUD-001`, `E-DB-008`) | `500` | `error.html` with `request_id` and code only. Never the error chain, never SQL, never a path |

### 5.2 Error fragment content

Every error fragment shows:

- the error code `E-<CTX>-<nnn>`,
- one sentence in the UI language that tells the user what to do,
- the `request_id`.

The same code appears in the structured log of the request. A user can report the code and the `request_id`. The operator finds the log line with them (R36).

### 5.3 Content negotiation

- An HTMX request gets the fragment. The response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML` for every status `>= 400`, except `E-WEB-004`, which sets `HX-Redirect`. HTMX swaps a 4xx or 5xx fragment only because of the `responseHandling` value of section 9.
- A request without `HX-Request` gets a full error page with the same content and the page layout.
- The export endpoints return `application/json` errors: `{"code": "E-EXP-001", "message": "...", "request_id": "..."}`.

## 6. Export document (v1)

The export document is the only machine-readable output of the system (ADR-0019). It is the interface to the recipe transfer system, which is out of scope.

The JSON Schema file `crates/recipe-export/schema/export-v1.schema.json` is the source of truth. Golden-file tests compare the serializer output with the schema and with stored examples. This section describes the schema. When the two differ, the schema file wins. The module `export` of `recipe-domain` builds the hashed body and computes the hash. `recipe-export` writes the HTTP response (ADR-0019).

### 6.1 Fields

| Field | Type | In hash | Rule |
|---|---|---|---|
| `schema_version` | string | no | `"1"`. A breaking change creates `"2"` and `/export/v2/` |
| `exported_at` | string, RFC 3339 UTC | no | Time of this export |
| `exported_by` | string | no | `Login` of the user |
| `loadable` | boolean | no | `true` only when `version.status` is `Approved` at export time |
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

`content_sha256` is the lower-case hex SHA-256 of the RFC 8785 (JCS) canonical form of the hashed body. Identifiers and values are in the hash. Names are informational and are not in the hash: `machine.name`, `recipe.name`, and `parameters[].name_en`. Status, approval data, and export data are not in the hash either. A rename of the recipe or of the machine does not change the hash. The retirement of the version does not change it either. Every number in the hashed body is an integer below 2^53 or a JSON string, so the canonical form is exact (ADR-0019).

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
- Before the transfer system loads a recipe into a PLC, it MUST check `loadable`. It MUST also build the hashed body from the document with the array order above, compute its SHA-256, and compare the result with `content_sha256`. Every sort key of the hashed arrays is in the document: `id`, `unit_path`, `tag`, `leaf_path`, and the order of `members[]`.

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

The server sends `Strict-Transport-Security` with `max-age` set to `http.hsts_max_age` and `includeSubDomains` on every response (`reliability.md` 7.11). The server reads `X-Forwarded-For` and `X-Forwarded-Proto` only from the trusted-proxy CIDR list in the configuration. A request from another address uses the socket address as `client_ip`.

### 7.2 Timeouts

| Setting | Default | Rule |
|---|---|---|
| Idle timeout | 30 min | The session ends when there is no request for this time |
| Absolute timeout | 12 h | The session ends this long after login, with or without activity |

The values are in `reliability.md` section 7.11 (A21). A session that ends returns `401 E-WEB-004`. The supervised job `cleanup` deletes expired session rows every `session.cleanup_interval`.

A session row holds `user_id` and the `session_epoch` of the user at login. On every request the server reads `role`, `is_active`, `must_change_password`, and `session_epoch` from `app_user`. The session is invalid when the user is inactive or when the epoch differs. `SetUserRole`, `DeactivateUser`, `ResetPassword` delete the sessions of the user. `ChangeOwnPassword` deletes every session of the user and creates a new session for the current request. All deletions run in the transaction of the command.

### 7.3 CSRF

- `SameSite=Strict` is the first line of defense.
- Every mutating request from an authenticated session carries a token bound to the session. Forms send it in the hidden field `csrf_token`. HTMX requests send it in the header `X-CSRF-Token`. The `hx-headers` attribute on `<body>` sets the header. `Login` creates the token. `ChangeOwnPassword` creates a new session with a new token and returns `HX-Refresh: true`, so the browser loads the page again with the new token in `hx-headers`.
- `POST /login` and `POST /lang` without a session use a pre-session token: a signed double-submit cookie `__Host-recipe_csrf` and the field `csrf_pre`. `GET /login` sets the cookie. The key of the Docker secret `csrf_signing_key` signs it (`security.csrf_signing_key_file`, `reliability.md` 7.11).
- Every mutating request must carry an `Origin` header that matches the site, or `Sec-Fetch-Site: same-origin`. A request without either is rejected.
- A mutating request without a valid token gets `403 E-WEB-007` and writes a `security_event` of kind `csrf_rejected` through the port `SecurityEventSink`. These events aggregate per client address and minute (`data-model.md` 3.8).
- The export endpoints are `GET` and need no token.
- Session fixation: `Login` always creates a new session row and a new token. When the login request carries a session cookie, `Login` deletes that session row.

### 7.4 Login protection

The values of this section are in `reliability.md` section 7.11.

- Login delay: after 3 failed attempts for one `(login, client_ip)` pair in 15 minutes, an attempt within `2^(n-3)` seconds after the last failure gets `429 E-IDN-005` with `Retry-After`. The cap is 60 seconds. There is no permanent lockout (Assumption A22, ADR-0014).
- Per-IP budget: after 100 failed attempts from one `client_ip` in 15 minutes, every login from that address gets `429 E-IDN-005` with `Retry-After`.
- Order: the use case takes an Argon2 permit and then runs T1 under the advisory lock of the login. T1 counts the failures in `login_attempt`, inserts the attempt row with `succeeded = false`, and commits.
- A throttled attempt releases the permit and gets 429. Otherwise the password check runs and the permit is released. T2 marks the attempt as a success and creates the session, or it writes a `security_event`.
- A failed login therefore always counts, also in read-only mode (`data-model.md` 6). After the per-IP budget is reached, T1 writes no attempt row, and the 429 comes from the count alone.
- `UnlockLogin` clears the counters of one login. It writes a `security_event` of kind `unlock`.
- A failed login gets one response for an unknown login, an inactive user, and a wrong password. The body, the status, and the response time are the same. The server hashes a dummy password when the user is not found or is inactive. The event log and the `security_event` row record the real reason.
- Every Argon2id call runs under a global semaphore with 4 permits and an acquire timeout of 2 seconds. A timeout returns `503 E-WEB-009` with `Retry-After`.
- Hash: Argon2id, `m = 19 MiB`, `t = 2`, `p = 1`. The hash is never logged.
- Password policy: 12 to 128 characters, no composition rule, and not present in the local breached-password list shipped with the image. Assumption A23.
- One-time password: `CreateUser` and `ResetPassword` generate a random password that meets the policy. The page shows it once. The command sets `must_change_password`. No request field carries a password for another user.
- `must_change_password`: a session of a user with the flag can open only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang`. Every other route returns `303` to `/me/password`. `GET /static/*` is outside this gate, because it needs no session (section 2.1).

### 7.5 Account state

A deactivated user gets the same `401 E-IDN-003` response as a wrong password (A42). The code `E-IDN-004` appears only in the event log and in `security_event`.

### 7.6 Body limits

| Route group | Limit |
|---|---|
| HTML forms | 256 KiB |
| The four routes that carry a parameter value: `POST .../parameters` (`AddParameter`, field `value`), `POST .../parameters/{key}/definition` (`UpdateParameterDefinition`, field `value`), `POST .../parameters/{key}/value` (`SetParameterValue`), and `POST .../rebind` (`RebindTypeRevision`, field `new_member_values`) | 1 MiB |
| `PUT .../translations/{lang}` | 64 KiB |

The values are in `reliability.md` section 7.11. A request above the limit gets `413 E-WEB-003`.

## 8. Internationalization

- The plain cookie `recipe_lang` stores the UI language. The session does not store it. The default is `pl` (R10). `en` is the second supported language (A17).
- Cookie `recipe_lang`: value `pl` or `en`, `Max-Age` from `i18n.lang_cookie_max_age` (`reliability.md` 7.11), `Secure`, `SameSite=Strict`, `Path=/`. It holds no secret, so it needs no `HttpOnly`. The server checks the value on every request. Any other value counts as missing, and the UI uses `pl`.
- The server ignores `Accept-Language`. A plant workstation often has an English operating system and a Polish operator.
- `POST /lang` sets the cookie and returns `303` to the `next` field when it is a same-origin relative path that starts with `/` and not with `//`. Otherwise it returns `303` to `/recipes`. The server never redirects to the `Referer` header. `POST /lang` works before login and in read-only mode, because it writes no database row. The language change is not audited.
- Parameter names and descriptions show the translation for the UI language when one exists. Otherwise the English base text appears with a marker `(en)`.
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

HTMX configuration, set in a `<meta name="htmx-config">` tag: `allowEval: false`, `allowScriptTags: false`, `selfRequestsOnly: true`, `historyCacheSize: 0`, `methodsThatUseUrlParams: ["get"]`, `includeIndicatorStyles: false`, `responseHandling: [{"code":"204","swap":false},{"code":"[23]..","swap":true},{"code":"[45]..","swap":true,"error":true},{"code":"...","swap":false}]`. By default HTMX 2 does not swap a 4xx or 5xx response, and `HX-Retarget` does not change that. The value of `responseHandling` swaps it, so `conflict.html`, `field-errors.html`, and `error.html` reach the page (sections 4 and 5.3). The value of `methodsThatUseUrlParams` makes `hx-delete` send a form body, which the form-only routes of section 2.6 require (`reliability.md` 4.2). By default HTMX inserts an inline `<style>` element for its request indicator, and `style-src 'self'` blocks it. `includeIndicatorStyles: false` stops this. The rules for `.htmx-indicator` are in the static style sheet `static/app.css`. The `<body>` tag has `hx-history="false"`. No page snapshot is stored in `localStorage` on a shared workstation.

Permitted client-side scripts. No other script exists in the application:

| File | Purpose | Limit |
|---|---|---|
| `static/htmx.min.js` | HTMX 2, vendored and pinned (ADR-0001) | unchanged upstream file |
| `static/row-version.js` | Updates hidden `row_version` fields from `HX-Trigger` (section 4) | 20 lines |
| `static/tree.js` | Expand and collapse of the unit tree and the type tree, keyboard focus | 50 lines |

Rules for templates:

- User data never appears inside an `hx-*` attribute, a `<script>` tag, or an inline event handler.
- The askama `|safe` filter is not used on user data. A review rejects any `|safe` on a value that is not a literal.
- The HTTP test suite runs an XSS payload corpus through every text field and checks the rendered pages.
