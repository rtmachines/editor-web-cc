# HTTP API

Status: Accepted (design stage, 2026-10-12)
Source of requirements: `general-draft.txt`
Related: `domain-model.md`, `data-model.md`, `reliability.md`, ADR-0001, ADR-0002, ADR-0004, ADR-0008, ADR-0013

## 1. Scope

This document defines the HTTP surface of `recipe-web`. It is a hypermedia API (ADR-0001).

- A full page request returns `text/html`.
- An HTMX request (`HX-Request: true`) returns an HTML fragment.
- The export endpoints return `application/json`. No other endpoint returns JSON.

URL policy:

- HTML routes have no version prefix. Example: `/recipes/R-100/versions/3`.
- The JSON export lives under `/export/v1/`. A change that breaks the export document creates `/export/v2/` (ADR-0002).
- Path values are the human identifiers from `domain-model.md`: `{code}` is a `MachineCode`, `{path}` is a `UnitPath`, `{ref}` is a `ReferenceNumber`, `{n}` is a version number, `{tag}` is a `ParameterTag`, `{login}` is a `Login`.

Every mutating request uses `POST`. The browser form model does not use `PUT` or `DELETE`. Every `POST` carries the CSRF token (section 7). Every `POST` on a `Draft` carries `row_version` (section 4).

The use case names in the tables are the commands from `domain-model.md` section 5. The permission names are from section 3 of this document.

## 2. Endpoints

Columns: Method, Path, Use case, Permission, Request, Success, Errors. The error column lists HTTP status and the error code family from `reliability.md`. Every endpoint can also return `401` (no session), `403` (no permission), and `500` (unexpected). The tables do not repeat these three.

### 2.1 Auth

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/login` | - | none | - | `200` login page | - |
| POST | `/login` | `Login` | none | `login`, `password` | `303` to `/recipes`, sets session cookie | `422 E-IDN-*` invalid form, `401 E-IDN-*` bad credentials, `429 E-IDN-*` rate limit |
| POST | `/logout` | `Logout` | authenticated | CSRF token | `303` to `/login`, clears session | - |

### 2.2 Machines and units

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines` | query | `RecipeView` | `q`, `page` | `200` list | - |
| GET | `/machines/new` | - | `MachineManage` | - | `200` form | - |
| POST | `/machines` | `CreateMachine` | `MachineManage` | `code`, `name`, `plc_platform` | `303` to `/machines/{code}` | `422 E-PLT-*` validation, `409 E-PLT-*` `DuplicateMachineCode` |
| GET | `/machines/{code}` | query | `RecipeView` | - | `200` machine page with unit tree | `404` |
| POST | `/machines/{code}` | `RenameMachine` | `MachineManage` | `name` | `200` fragment | `422`, `404` |
| POST | `/machines/{code}/archive` | `ArchiveMachine` | `MachineManage` | CSRF token | `303` to `/machines` | `404`, `409 E-PLT-*` machine has recipes that are not deleted |
| GET | `/machines/{code}/units` | query | `RecipeView` | - | `200` unit tree fragment | `404` |
| POST | `/machines/{code}/units` | `AddUnit` | `UnitManage` | `parent_path` (empty for a main unit), `segment`, `name` | `200` tree fragment | `422 E-PLT-*` `UnitDepthExceeded`, `404` `ParentNotFound`, `409` `DuplicateUnitPath` |
| GET | `/machines/{code}/units/{path}` | query | `RecipeView` | - | `200` unit detail | `404` |
| POST | `/machines/{code}/units/{path}` | `RenameUnit` | `UnitManage` | `name` | `200` fragment | `422`, `404` |
| POST | `/machines/{code}/units/{path}/archive` | `ArchiveUnit` | `UnitManage` | CSRF token | `200` tree fragment | `404`, `409 E-PLT-*` unit has sub-units or is used by a parameter |

### 2.3 UDTs

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/machines/{code}/types` | query | `RecipeView` | - | `200` list of UDTs, latest revision first | `404` |
| GET | `/machines/{code}/types/new` | - | `UdtManage` | - | `200` form | - |
| POST | `/machines/{code}/types` | `DefineUdt` | `UdtManage` | `name`, `members[i].name`, `members[i].type_id`, optional `supersedes` | `303` to the new revision | `422 E-PLT-*` validation, `409 E-PLT-*` `TypeCycle`, `MemberTypeNotOnPlatform` |
| GET | `/machines/{code}/types/{name}` | query | `RecipeView` | optional `rev` | `200` type tree of the latest revision, or of `rev` | `404` |
| POST | `/machines/{code}/types/{name}/revisions` | `DefineUdt` | `UdtManage` | same as `POST /types`, `supersedes` is set by the server | `303` to the new revision | same as `POST /types` |

A UDT is immutable. The second `POST` creates a new revision. It never edits a row (ADR-0006).

### 2.4 Recipes

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes` | query | `RecipeView` | `q`, `machine`, `status`, `page` | `200` list. Deleted recipes are hidden unless `show_deleted=1` | - |
| GET | `/recipes/new` | - | `RecipeCreate` | - | `200` form | - |
| POST | `/recipes` | `CreateRecipe` | `RecipeCreate` | `machine_code`, `reference_number`, `name`, `change_note` | `303` to `/recipes/{ref}/versions/1` | `422 E-RCP-*` validation, `409 E-RCP-*` `DuplicateReference`, `409 E-RCP-*` `MachineArchived` |
| GET | `/recipes/{ref}` | query | `RecipeView` | - | `200` recipe page with version list | `404` |
| POST | `/recipes/{ref}` | `RenameRecipe` | `RecipeEdit` | `name` | `200` fragment | `422`, `404` |
| POST | `/recipes/{ref}/delete` | `DeleteRecipe` | `RecipeDelete` (conditional, section 3) | CSRF token | `303` to `/recipes` | `404`, `409 E-RCP-*` `RecipeHasApprovedVersion` |

### 2.5 Versions

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/recipes/{ref}/versions` | query | `RecipeView` | - | `200` version list fragment | `404` |
| POST | `/recipes/{ref}/versions` | `CreateVersion` | `VersionCreate` | `based_on` (version number, optional), `change_note` | `303` to the new `Draft` | `422 E-RCP-*`, `404`, `409 E-RCP-*` `RecipeDeleted`, `DraftAlreadyExists` |
| GET | `/recipes/{ref}/versions/{n}` | query | `RecipeView` | - | `200` version page. Editable form only for a `Draft` and a user with `ParameterEdit` | `404` |
| POST | `/recipes/{ref}/versions/{n}/approve` | `ApproveVersion` | `VersionApprove` | `row_version`, CSRF token | `200` version page. Shows the retired version number when one exists | `404`, `409 E-RCP-*` `VersionNotDraft`, `DraftHasNoParameters`, `ConcurrentEdit`, `422 E-RCP-*` `ValidationFailed` with the list of failing parameters |
| POST | `/recipes/{ref}/versions/{n}/retire` | `RetireVersion` | `VersionRetire` | `row_version`, CSRF token | `200` version page | `404`, `409 E-RCP-*` `VersionNotApproved`, `ConcurrentEdit` |
| POST | `/recipes/{ref}/versions/{n}/discard` | `DiscardDraft` | `VersionDiscard` | `row_version`, CSRF token | `303` to `/recipes/{ref}` | `404`, `409 E-RCP-*` `VersionNotDraft`, `ConcurrentEdit` |
| GET | `/recipes/{ref}/versions/{n}/diff/{m}` | query | `RecipeView` | - | `200` parameter diff between version `n` and `m` | `404` |

### 2.6 Parameters

All routes in this group are under `/recipes/{ref}/versions/{n}`. Mutations are allowed only when version `n` is a `Draft` (I3). The server returns `409 VersionNotEditable` otherwise.

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `.../parameters` | query | `RecipeView` | `unit` (path filter) | `200` parameter table fragment, grouped by unit | `404` |
| GET | `.../parameters/new` | - | `ParameterEdit` | `unit` | `200` empty parameter form | `404` |
| GET | `.../parameters/{tag}` | query | `RecipeView` | - | `200` parameter detail or edit form | `404` |
| POST | `.../parameters/{tag}` | `UpsertParameter` | `ParameterEdit` | `row_version`, `unit_path`, `name_en`, `description_en`, `plc_type_id`, `s7_address`, `value` (JSON text), `ordinal` | `200` updated parameter row fragment and new `row_version` in `HX-Trigger` | `422 E-RCP-*` `S7AddressRequired`, `ValueOutOfLimit`, `ValueShapeMismatch`, `TypeNotOnPlatform`, `409 E-RCP-*` `VersionNotEditable`, `ConcurrentEdit` |
| POST | `.../parameters/{tag}/delete` | `RemoveParameter` | `ParameterEdit` | `row_version`, CSRF token | `200` parameter table fragment | `404`, `409 E-RCP-*` `VersionNotEditable`, `ConcurrentEdit` |
| GET | `.../parameters/{tag}/limits` | query | `RecipeView` | - | `200` limits table fragment, one row per leaf path | `404` |
| POST | `.../parameters/{tag}/limits` | `UpsertParameter` (limits part) | `ParameterEdit` | `row_version`, `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`, `limits[i].max_length` | `200` limits fragment | `422 E-RCP-*` `LimitOutsideTypeRange`, `LeafPathNotFound`, `MinGreaterThanMax`, `409` `VersionNotEditable`, `ConcurrentEdit` |
| GET | `.../parameters/{tag}/translations` | query | `RecipeView` | - | `200` translation table fragment | `404` |
| POST | `.../parameters/{tag}/translations` | `UpsertParameter` (translation part) | `ParameterEdit` | `row_version`, `lang`, `name`, `description` | `200` translation fragment | `422 E-RCP-*` invalid language code, `409` `VersionNotEditable`, `ConcurrentEdit` |
| POST | `.../parameters/{tag}/translations/{lang}/delete` | `UpsertParameter` (translation part) | `ParameterEdit` | `row_version`, CSRF token | `200` translation fragment | `404`, `409` |

The `value` field is the JSON text of a `ParameterValue`. The form builds it from one input per leaf. The server parses it as a whole. A partial value is a `422 ValueShapeMismatch`.

### 2.7 Export

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/export/v1/recipes/{ref}/approved` | `ExportVersion` | `VersionExport` | - | `200 application/json` export document of the `Approved` version | `404 E-EXP-*` no approved version or recipe deleted |
| GET | `/export/v1/recipes/{ref}/versions/{n}` | `ExportVersion` | `VersionExport` | - | `200 application/json` export document | `404 E-EXP-*` version is not `Approved` and not `Retired`, or recipe deleted |

The export endpoints use the same session cookie as the HTML routes (ADR-0004). A `Draft` is never exportable. Each successful export writes an audit entry (A8). Section 6 defines the document.

### 2.8 Users

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/users` | query | `UserManage` | `q`, `page` | `200` list | - |
| GET | `/users/new` | - | `UserManage` | - | `200` form | - |
| POST | `/users` | `CreateUser` | `UserManage` | `login`, `password`, `role` | `303` to `/users/{login}` | `422 E-IDN-*` password policy, `409 E-IDN-*` `DuplicateLogin` |
| GET | `/users/{login}` | query | `UserManage` | - | `200` user page | `404` |
| POST | `/users/{login}/role` | `SetUserRole` | `UserManage` | `role` | `200` fragment | `404`, `409 E-IDN-*` `LastAdministrator` |
| POST | `/users/{login}/deactivate` | `DeactivateUser` | `UserManage` | CSRF token | `200` fragment | `404`, `409 E-IDN-*` `LastAdministrator` |
| POST | `/users/{login}/activate` | `ActivateUser` | `UserManage` | CSRF token | `200` fragment | `404` |
| POST | `/users/{login}/reset-password` | `ResetPassword` | `UserManage` | `password` | `200` fragment. All sessions of that user are deleted | `404`, `422 E-IDN-*` password policy |
| GET | `/me` | query | authenticated | - | `200` own profile | - |
| POST | `/me/password` | `ChangeOwnPassword` | authenticated | `current_password`, `password` | `200` fragment. Other sessions of the user are deleted | `401 E-IDN-*` wrong current password, `422 E-IDN-*` password policy |

Deactivate is a soft delete (ADR-0011). A deactivated user keeps the audit history. The user cannot log in. Existing sessions of the user are deleted in the same transaction.

### 2.9 Audit

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/audit` | query | `AuditView` or `AuditViewRecipes` | `entity_type`, `entity_id`, `actor`, `action`, `from`, `to`, `page` | `200` audit list. A user with `AuditViewRecipes` only sees entries with `entity_type` in `recipe`, `recipe_version`, `parameter` | `422` invalid filter |
| GET | `/audit/{id}` | query | same as above | - | `200` one entry with `before` and `after` as formatted JSON | `404` |
| GET | `/recipes/{ref}/history` | query | `RecipeView` | `page` | `200` audit entries of the recipe and its versions, without `before`/`after` bodies | `404` |

The `/recipes/{ref}/history` route is the recipe timeline. It is visible to every role (R24: an operator can view recipes and their versions). It does not show user data or machine changes.

### 2.10 Ops

| Method | Path | Use case | Permission | Request | Success | Errors |
|---|---|---|---|---|---|---|
| GET | `/healthz` | liveness | none | - | `200 text/plain ok`. No database access | - |
| GET | `/readyz` | readiness | none | - | `200 text/plain ready` after `SELECT 1` with a 2 s timeout | `503` when the database does not answer |

Both routes bind only to the internal Docker network. The reverse proxy does not forward them (ADR-0003). They have no session and no CSRF check.

## 3. Permission matrix

### 3.1 Permission enum

```rust
pub enum Permission {
    RecipeView, RecipeCreate, RecipeEdit, RecipeDelete,
    VersionCreate, VersionApprove, VersionRetire, VersionDiscard, VersionExport,
    ParameterEdit,
    MachineManage, UnitManage, UdtManage,
    UserManage,
    AuditView, AuditViewRecipes,
}
```

### 3.2 Rules

- The permission check is the first statement of every use case. The HTTP layer only authenticates. It does not decide what a role can do (ADR-0013, R28).
- A role maps to a fixed set of permissions. There are no per-user overrides (ADR-0013).
- `Role::permissions()` is a pure function in `recipe-domain`. An integration test checks every (operation, role) pair of the matrix below (`reliability.md` section 6).
- Last administrator rule: `SetUserRole` and `DeactivateUser` return `Err(LastAdministrator)` when the target is the last active user with role `Administrator`. The database enforces the same rule with a constraint trigger (`data-model.md`).

### 3.3 Matrix

Legend: ✓ allowed, ✗ denied, (c) allowed with the condition in the last column.

| Operation | Permission | Administrator | ProcessEngineer | Operator | Condition |
|---|---|---|---|---|---|
| View recipes, versions, parameters, machines, units, UDTs | `RecipeView` | ✓ | ✓ | ✓ | - |
| View recipe history (`/recipes/{ref}/history`) | `RecipeView` | ✓ | ✓ | ✓ | - |
| Export `Approved` or `Retired` version | `VersionExport` | ✓ | ✓ | ✓ | A4 |
| Create recipe | `RecipeCreate` | ✓ | ✓ | ✗ | R25 |
| Rename recipe | `RecipeEdit` | ✓ | ✓ | ✗ | R25 |
| Delete recipe (soft) | `RecipeDelete` | ✓ | (c) | ✗ | ProcessEngineer only when the recipe has no `Approved` version (R23, A13). Administrator: never when an `Approved` version exists (I14) |
| Create version (new `Draft`) | `VersionCreate` | ✓ | ✓ | ✗ | R25, A7 |
| Edit parameters, limits, translations of a `Draft` | `ParameterEdit` | ✓ | ✓ | ✗ | R9, R25 |
| Approve version | `VersionApprove` | ✓ | ✓ | ✗ | A5. The approval retires the previous `Approved` version (R27) |
| Retire version without replacement | `VersionRetire` | ✓ | ✗ | ✗ | N11 |
| Discard `Draft` | `VersionDiscard` | ✓ | ✓ | ✗ | - |
| Create, rename, archive machine | `MachineManage` | ✓ | ✓ | ✗ | A9 |
| Add, rename, archive unit | `UnitManage` | ✓ | ✓ | ✗ | A9 |
| Define UDT revision | `UdtManage` | ✓ | ✓ | ✗ | A9 |
| Create user, set role, activate, deactivate, reset password | `UserManage` | ✓ | ✗ | ✗ | A9. Last administrator rule |
| Change own password | authenticated | ✓ | ✓ | ✓ | - |
| View full audit log (`/audit`) | `AuditView` | ✓ | ✗ | ✗ | A20 |
| View audit entries of recipes, versions, parameters (`/audit`) | `AuditViewRecipes` | ✓ | ✓ | ✗ | A20 |
| Health and readiness | none | - | - | - | internal network only |

Role to permission mapping:

| Role | Permissions |
|---|---|
| Administrator | every permission |
| ProcessEngineer | `RecipeView`, `VersionExport`, `RecipeCreate`, `RecipeEdit`, `RecipeDelete` (c), `VersionCreate`, `ParameterEdit`, `VersionApprove`, `VersionDiscard`, `MachineManage`, `UnitManage`, `UdtManage`, `AuditViewRecipes` |
| Operator | `RecipeView`, `VersionExport` |

## 4. Concurrency protocol

The protocol implements ADR-0008 (optimistic locking) for every `Draft`.

1. `GET /recipes/{ref}/versions/{n}` renders the current `row_version` of the version into a hidden field `row_version` of every form on the page.
2. Every `POST` on that version sends `row_version`.
3. The use case runs `UPDATE recipe_version SET ..., row_version = row_version + 1 WHERE id = $1 AND row_version = $2`. Zero affected rows means a conflict. The transaction rolls back. The use case returns `Err(ConcurrentEdit { expected, actual })`.
4. On success the response carries the new value in the header `HX-Trigger: {"rowVersion": <new>}`. A small inline script updates every hidden `row_version` field on the page. This is the only client-side script in the application.
5. On conflict the server returns `409 Conflict` with the fragment `conflict.html`. The fragment shows: the user's `row_version`, the current server `row_version`, the login of the last editor, the time of the last change, and one button "Reload version". The button does `hx-get` on the version page. The user's unsaved input stays in the form until the reload.
6. The `409` response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML`. The original target of the request does not change. The user does not lose the form.

A parameter edit changes the `row_version` of the version, not only of the parameter row. Two users who edit two different parameters of the same `Draft` get a conflict on the second save. This is intended. A `Draft` is one unit of work for approval.

Read requests do not use `row_version`. An `Approved` or `Retired` version is immutable and has no conflict.

## 5. Error responses

### 5.1 Status mapping

| Error family (from `recipe-app` and `recipe-domain`) | HTTP status | Body |
|---|---|---|
| Input validation (`ParseError` of a value object, missing field, wrong format) | `422 Unprocessable Content` | `field-errors.html`: one line per field, field name and message |
| `Forbidden` (permission denied) | `403 Forbidden` | `error.html` with the operation name |
| No session or expired session | `401 Unauthorized` | `HX-Redirect: /login` for HTMX, `303` to `/login` otherwise |
| CSRF token missing or wrong | `403 Forbidden` | `error.html` |
| `NotFound` (recipe, version, parameter, machine, unit, user) | `404 Not Found` | `error.html` |
| `ConcurrentEdit` | `409 Conflict` | `conflict.html` (section 4) |
| Invariant violation (`VersionNotEditable`, `VersionNotDraft`, `VersionNotApproved`, `RecipeHasApprovedVersion`, `DraftAlreadyExists`, `DraftHasNoParameters`, `LastAdministrator`, `MachineArchived`, `RecipeDeleted`) | `409 Conflict` | `error.html` with the rule that blocks the operation |
| Duplicate (`DuplicateReference`, `DuplicateMachineCode`, `DuplicateUnitPath`, `DuplicateLogin`) | `409 Conflict` | `field-errors.html` on the duplicate field |
| Domain validation of a value (`ValueOutOfLimit`, `ValueShapeMismatch`, `S7AddressRequired`, `TypeNotOnPlatform`, `LimitOutsideTypeRange`, `TypeCycle`, `ValidationFailed`) | `422 Unprocessable Content` | `field-errors.html`, with `leaf_path` for value errors |
| Rate limit on login | `429 Too Many Requests` | `error.html` with `Retry-After` header |
| Database unavailable, timeout | `503 Service Unavailable` | `error.html` with `Retry-After: 5` |
| Unexpected (`Internal`) | `500 Internal Server Error` | `error.html` with `request_id` and error code only. Never the error chain, never SQL, never a path |

### 5.2 Error fragment content

Every error fragment shows:

- the error code `E-<context>-<nnn>` (`reliability.md` section 1 defines the codes),
- one sentence in the UI language that tells the user what to do,
- the `request_id`.

The same code appears in the structured log of the request. A user can report the code and the `request_id`. The operator of the system can find the log line with them (R36).

### 5.3 Content negotiation

- An HTMX request gets the fragment. The response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML` for every status `>= 400`, except `401`, which sets `HX-Redirect`.
- A request without `HX-Request` gets a full error page with the same content and the page layout.
- The export endpoints return `application/json` errors: `{"code": "E-EXP-001", "message": "...", "request_id": "..."}`.

## 6. Export document (v1)

The export document is the only machine-readable output of the system (ADR-0002). It is the contract with the recipe transfer system, which is out of scope.

### 6.1 Fields

| Field | Type | Rule |
|---|---|---|
| `schema_version` | string | `"1"`. A breaking change creates `"2"` and `/export/v2/` |
| `exported_at` | string, RFC 3339 UTC | Time of this export |
| `exported_by` | string | `Login` of the user |
| `machine.code` | string | `MachineCode` |
| `machine.plc_platform` | string | one of `CodesysIec`, `TwinCat3`, `S7_1200`, `S7_1500`, `S7_300` |
| `recipe.reference_number` | string | `ReferenceNumber` |
| `recipe.name` | string | |
| `version.number` | integer | |
| `version.status` | string | `Approved` or `Retired` |
| `version.approved_at` | string, RFC 3339 UTC | |
| `version.approved_by` | string | `Login` |
| `types[]` | array | One entry per `PlcType` that a parameter uses directly or through a member, except primitives |
| `types[].id` | string, UUID | `PlcTypeId` of the revision |
| `types[].name` | string | |
| `types[].kind` | string | `Array` or `Struct` |
| `types[].element_type` | string | Type name or `types[].id` of the element. Only for `Array` |
| `types[].dimensions[]` | `{lower, upper}` | Only for `Array` |
| `types[].members[]` | `{name, type}` | Ordered. `type` is a primitive name or a `types[].id`. Only for `Struct` |
| `parameters[]` | array | Ordered by `unit_path`, then `ordinal` |
| `parameters[].unit_path` | string | `UnitPath` |
| `parameters[].tag` | string | `ParameterTag` |
| `parameters[].key` | string, UUID | `ParameterKey`, stable across versions |
| `parameters[].name_en` | string | |
| `parameters[].type` | string | Primitive name, or `types[].id` for an array or struct. `STRING(n)` carries the length |
| `parameters[].s7_address` | string or null | Present and not null for `S7_300`. Null otherwise |
| `parameters[].limits[]` | `{leaf_path, min, max, max_length}` | Absent fields are null |
| `parameters[].value` | JSON | The `ParameterValue` tree. Numbers are JSON numbers. 64-bit integers are JSON strings to avoid precision loss |

Descriptions and translations are not exported. The transfer system does not need them.

### 6.2 Example

```json
{
  "schema_version": "1",
  "exported_at": "2026-10-12T09:30:00Z",
  "exported_by": "jkowalski",
  "machine": { "code": "M12", "plc_platform": "S7_300" },
  "recipe": { "reference_number": "R-100", "name": "Bottle 0.5 l" },
  "version": {
    "number": 3, "status": "Approved",
    "approved_at": "2026-10-11T14:02:10Z", "approved_by": "anowak"
  },
  "types": [
    {
      "id": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "name": "ZoneTemp", "kind": "Struct",
      "members": [ { "name": "setpoint", "type": "REAL" }, { "name": "enabled", "type": "BOOL" } ]
    }
  ],
  "parameters": [
    {
      "unit_path": "U1", "tag": "SPEED",
      "key": "1f3a6b2c-8d9e-4f00-a1b2-c3d4e5f60718",
      "name_en": "Line speed", "type": "INT", "s7_address": "DB1.DBW0",
      "limits": [ { "leaf_path": "", "min": 10, "max": 250, "max_length": null } ],
      "value": 120
    },
    {
      "unit_path": "U1.2", "tag": "ZONE1",
      "key": "9a8b7c6d-5e4f-4321-b0a9-8f7e6d5c4b3a",
      "name_en": "Zone 1", "type": "7c1e9d0a-2b3f-4c4d-9e8f-0a1b2c3d4e5f",
      "s7_address": "DB1.DBD2",
      "limits": [ { "leaf_path": ".setpoint", "min": 20.0, "max": 95.0, "max_length": null } ],
      "value": { "setpoint": 72.5, "enabled": true }
    }
  ]
}
```

### 6.3 Rules

- For a given `(reference_number, version.number)` the document is immutable. Only `exported_at` and `exported_by` change between two exports.
- The response sets `Content-Disposition: attachment; filename="recipe_{ref}_v{n}.json"`.
- The response sets `Cache-Control: no-store`.
- Each export writes one `AuditEntry` with action `VersionExported` (A8).
- Field order in the document is fixed. The serializer does not emit extra fields.

## 7. Session and CSRF

Sessions follow ADR-0004. The session store is the `session` table in PostgreSQL.

### 7.1 Cookie

| Attribute | Value |
|---|---|
| Name | `recipe_session` |
| `HttpOnly` | yes |
| `Secure` | yes. The reverse proxy terminates TLS (A18). Config flag `session.secure = false` only for local development |
| `SameSite` | `Strict` |
| `Path` | `/` |
| Content | random 256-bit session id only. No user data in the cookie |

### 7.2 Timeouts

| Setting | Config key | Default | Rule |
|---|---|---|---|
| Idle timeout | `session.idle_timeout` | 30 min | The session ends when there is no request for this time |
| Absolute timeout | `session.absolute_timeout` | 12 h | The session ends this long after login, with or without activity |

Both values are Assumption A21. A session that ends returns `401` and a redirect to `/login`. An expired session row is deleted by a background task every 5 minutes.

### 7.3 CSRF

- `SameSite=Strict` is the first line of defense.
- Every `POST` form carries a hidden field `csrf_token`. The value is a random 256-bit token bound to the session. It is created at login.
- HTMX requests send the same token in the header `X-CSRF-Token`. The `hx-headers` attribute on `<body>` sets it.
- A `POST` without a valid token gets `403 E-IDN-*`. The server logs the event with the `request_id` and the session id.
- The export endpoints are `GET` and need no token.

### 7.4 Login protection

- Rate limit: 5 failed logins per `login` value in 15 minutes. The sixth attempt gets `429` with `Retry-After`. The counter lives in PostgreSQL (`login_attempt` table), not in memory, so a restart does not reset it. Assumption A22.
- The response time of a failed login does not depend on whether the login exists. The server hashes a dummy password when the user is not found.
- Password policy: minimum 12 characters, maximum 128 characters, no other composition rule. Assumption A23.
- Hash: Argon2id with the parameters in `reliability.md`. The hash is never logged.
- A password reset by an administrator and a password change by the user delete every other session of that user in the same transaction.

## 8. Internationalization

- The UI language is a session value. The default is `pl` (R10). `en` is the second supported language (A17).
- The server ignores `Accept-Language`. A plant workstation often has an English operating system and a Polish operator.
- `POST /lang` with field `lang` in `pl` or `en` sets the session value and returns `303` to the `Referer`, or to `/recipes` when there is no `Referer`. The language change is not audited.
- Parameter names and descriptions show the translation for the UI language when one exists. Otherwise the English base text appears with a marker `(en)`.
- Error messages, status words, and permission names come from `fluent` bundles. Error codes do not change with the language.
