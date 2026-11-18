# ADR-0194: Shared codes, SQL text, and stored-type codes of the draft rows

Status: Accepted. Amends `data-model.md` §6, `domain-model.md` §5, §5.1, and §5.2, and `api.md` §2.6. Amends ADR-0193 (g) and ADR-0029 (a new array type of a rebind)
Date: 2026-11-18
Deciders: project owner

## Context

ADR-0193 made the draft rows of `data-model.md` §6 agree with the code for the touch order and the rebind error sources. Its section "Gaps outside this ADR" lists five open gaps. The fix plan records them. This ADR takes all five. A follow-up ADR decides the codes of stored data that breaks the earlier rule set.

A check of the code for each gap gives the facts below. The paths of the use cases are in `crates/recipe-app/src/recipe/`. The paths of the adapter are in `crates/recipe-db/src/transaction/`.

### Codes of stored data

`Draft::load` trusts the stored values and limits (ADR-0028). A command checks a stored part that it reads and does not write with the earlier rule set (ADR-0120 (d)). It checks the part of the request with the current set (ADR-0120 (c)). ADR-0120 (j) says that no write path can store a part that fails the earlier set. Such a part is a storage defect.

ADR-0028 also decides one result: "`SetParameterValue` and `ReplaceLimits` on a parameter whose type does not resolve give `E-PLT-013`." The code does this:

- `OpenDraft::open` resolves the type of every parameter of the draft (`draft_session.rs` lines 164 to 177 and 205).
- `Draft::set_value` and `Draft::replace_limits` call `resolved_tree` (`crates/recipe-domain/src/recipe/draft/value.rs` lines 31 and 60).
- `resolved_tree` returns the `PlantError` of a failed resolution (`crates/recipe-domain/src/recipe/draft/add/step_definition.rs` line 101).
- A type row that is not on the platform gives `E-PLT-013` in that resolution (`crates/recipe-domain/src/plant/type_tree.rs` lines 272, 463, and 317 to 321).
- A type cycle gives `E-PLT-005`. A type above a size limit gives `E-PLT-018` (`type_tree.rs` lines 441, 444, and 591).
- The tests at lines 160 and 343 of `crates/recipe-domain/src/recipe/draft/value/tests.rs` cover the other `E-PLT-013` path, a type that the trees do not hold (`step_definition.rs` line 100).

The rows `SetParameterValue` and `ReplaceLimits` of `data-model.md` §6 and `domain-model.md` §5.2 do not name `E-PLT-013`.

The `E-PLT-013` path also needs a storage defect. Trigger 4.2 checks the platform of the type at each insert of a parameter and each change of its type or platform (`data-model.md` §4.2). A type in the tree passed the platform check of `DefineUdt` (`data-model.md` §3.3), and `recipe_app` cannot change a `plc_type` or `plc_type_member` row. `recipe_app` also cannot change a machine platform, or `parameter.plc_platform` (`crates/recipe-db/tests/grants.expected` lines 51 to 64 and 91 to 106).

Over HTTP, the routes of `SetParameterValue` and `ReplaceLimits` first run the form query (`crates/recipe-web/src/pages/parameters/value.rs` lines 159 and 208). The form query gives `500 E-DB-008` for a parameter type that the type rows of the platform do not hold. The code is in `queries/parameter_form.rs` line 280 and `queries/parameters.rs` lines 154 to 156. For a tree that does not resolve, the form query gives the resolution error (`queries/parameter_form.rs` line 169).

The other resolution errors and the other codes of stored data go to a follow-up ADR. The section "Gaps outside this ADR" lists their sites.

### Shared codes of the draft commands

The eight draft commands run the same steps in the same order:

1. `authorize` is the first statement (`crates/recipe-app/src/auth/authorize.rs` lines 20 to 30). It writes a `forbidden` security event and then gives `Forbidden` (`E-WEB-002`). A failed event write gives the storage error in place of `E-WEB-002` (lines 55 and 56, ADR-0029).
2. `resolve_version` reads the recipe and the version before the transaction. A missing recipe or version gives `NotFound` (`E-WEB-005`) (`resolve.rs` lines 38 and 58).
3. `OpenDraft::open` takes the locks (`draft_session.rs` lines 195 to 205). Five commands take no machine lock (lines 102 to 104). The table below shows the three commands with the machine lock.
4. A locked row that is missing gives `NotFound` (`draft_session.rs` lines 109 to 111, 124 to 126, and 146 to 148). `recipe_app` has no `DELETE` grant on `machine`, `recipe`, or `recipe_version` (`crates/recipe-db/tests/grants.expected`). The soft delete keeps these rows. Only a storage defect reaches these arms.
5. `TouchedDraft::commit` calls `UnitOfWork::commit` (`draft_session.rs` line 258). A failed audit insert gives `AuditWriteFailed` (`E-AUD-001`) (`crates/recipe-app/src/ports/unit_of_work.rs` lines 64 and 65).

The lines of `authorize`, `OpenDraft::open`, and `commit` in the eight use cases are:

| Command | `authorize` | `OpenDraft::open` | `commit` |
|---|---|---|---|
| `AddParameter` | `add_parameter.rs` 108 | 114, machine lock | 140 |
| `UpdateParameterDefinition` | `update_definition.rs` 118 | 124, machine lock | 139 |
| `SetParameterValue` | `set_value.rs` 71 | 77 | 94 |
| `ReplaceLimits` | `replace_limits.rs` 71 | 77 | 96 |
| `UpsertTranslation` | `translation.rs` 87 | 93 | 120 |
| `RemoveTranslation` | `translation.rs` 164 | 170 | 188 |
| `RemoveParameter` | `remove_parameter.rs` 65 | 71 | 89 |
| `RebindTypeRevision` | `rebind.rs` 85 | 132, machine lock | 116 |

`recipe-web` parses the path values before the use case. A malformed `{ref}`, `{n}`, or `{key}` gives `404 E-WEB-005` (`crates/recipe-web/src/pages/path_values.rs` lines 130 and 131, 140 to 143, and 171 to 176). `api.md` §1 gives the same codes.

The rule of §6 that starts "Every draft command can return ..." names `VersionNotEditable`, `RecipeDeleted`, and `ConcurrentEdit` of these steps. The rule names no `E-WEB-002`, `E-WEB-005`, or `E-AUD-001`. No draft row names them.

§6 shows a shared code in two forms. A rule of the §6 intro names a code that every draft command shares. A row names a shared code only when the row has its own source of the code. Examples are the `seal` of `SetPlcConnection` (`E-WEB-011`) and the audit insert of `CompareSettings` (`E-AUD-001`). `domain-model.md` §5 uses one sentence for a code that many commands share: the sentence about `E-WEB-006`.

### Shared codes of every command

The same codes come from the commands outside the draft rows. The paths in this section are in `crates/recipe-app/src/`.

- Every command use case except `Login` calls `authorize`. `identity/login.rs` has no call.
- `Logout` and `ChangeOwnPassword` deny only `ExportClient` (`auth/matrix.rs` lines 90 and 97 to 98). `Login` refuses that role (`identity/login.rs` line 297).
- The doc comment of `SetUserRole` says that a role change ends every session of the target (`identity/set_user_role.rs` lines 23 and 43).
- `ExportApprovedVersion` and `ExportPlcConnection` deny no role (`auth/matrix.rs` lines 102 to 103 and 110 to 111).
- `Login` and `Logout` commit their audit entries through `UnitOfWork::commit` (`identity/login.rs` line 222, `identity/logout.rs` line 58). So both can give `E-AUD-001`.
- The paths that write no audit entry are in `domain-model.md` §5. The supervised jobs and `CompareSettings` are not commands.

`random_uuid` gives `TaskFailed` with the stage `Token` (`E-WEB-011`) when the read of the random bytes fails (`identity/issue.rs` lines 26 to 31 and 71 to 74). These commands call it:

- `CreateMachine` (`plant/create_machine.rs` lines 79 and 80) and `AddUnit` (`plant/add_unit.rs` line 96).
- `DefineUdt` (`plant/define_udt.rs` line 186) and `CreateUser` (`identity/create_user.rs` line 72).
- `CreateRecipe` (`recipe/create_recipe.rs` lines 130 and 131) and `CreateVersion` (`recipe/create_version.rs` lines 150 and 371).
- `AddParameter` (`recipe/add_parameter.rs` lines 122 and 123).
- The callers of `EnsureDerivedType`: `AddParameter` and `UpdateParameterDefinition` (`recipe/definition_steps.rs` line 76), `DefineUdt` (`plant/define_udt.rs` line 115), and `RebindTypeRevision` (`recipe/rebind/new_types.rs` line 45).

`create-admin` (`identity/bootstrap.rs` line 69) and `new_request_id` (`identity/issue.rs` line 59) also call it. They are not in the command rows.

### `E-WEB-011` and the statements of `EnsureDerivedType`

`ensure_row` creates the identifier of a `STRING(n)`, `WSTRING(n)`, or array row with `random_uuid` (`crates/recipe-app/src/plant/ensure_derived.rs` line 234). Then it looks for an existing row with the same shape (lines 240 to 246). The read of the random bytes therefore runs also when the row exists.

The statement cell of the row `EnsureDerivedType` gives a `SELECT` by `name` for a primitive and a `SELECT` by `(machine_id, name, revision)` for a struct. The code runs neither statement. It finds a primitive or a struct in the type rows that the caller read. The code is in `ensure_derived.rs` lines 156 to 159, 186 to 188, and 193 to 222. When no row of the machine matches a `STRING(n)`, `WSTRING(n)`, or array, the adapter runs `INSERT ... ON CONFLICT DO NOTHING RETURNING`. It runs the `SELECT` only when the insert writes no row (`type_rows.rs` lines 54 to 58 and 411 to 429).

The row `EnsureDerivedType` of §6 names no `E-WEB-011`. `domain-model.md` §5.1 says "Its errors are `E-PLT-013 DerivedTypeInvalid` and `E-PLT-018 TypeTooLarge`." It also says that a new array type of `RebindTypeRevision` "gives only `E-PLT-018`". ADR-0029 says that a new array type of a rebind "can fail only with `E-PLT-018`".

### SQL text of the draft rows

The statement column of §6 gives short SQL. The adapter runs this SQL (`draft_writes.rs`, `draft_rows.rs`, `unit_rows.rs`):

| Row | Text of §6 | SQL of the adapter |
|---|---|---|
| `AddParameter` | `INSERT parameter_limit` per entry of `limits[]` | One `INSERT ... SELECT ... FROM unnest(...)` for all entries (`draft_writes.rs` lines 36 to 41). No statement for an empty list (lines 141 to 143) |
| `UpdateParameterDefinition` | `UPDATE parameter ... WHERE id = $p AND recipe_version_id = $v` | `WHERE recipe_version_id = $1 AND parameter_key = $2` (lines 106 to 109) |
| `UpdateParameterDefinition` | `DELETE parameter_limit`, `INSERT parameter_limit`, then `UPDATE parameter SET value` | `UPDATE parameter SET value`, then the limit statements of `ReplaceLimits` (`update_definition.rs` lines 213 to 220) |
| `SetParameterValue` | `UPDATE parameter SET value WHERE id = $p AND recipe_version_id = $v` | `WHERE recipe_version_id = $1 AND parameter_key = $2` (lines 82 and 83) |
| `ReplaceLimits` | `DELETE parameter_limit WHERE parameter_id = $p`, `INSERT parameter_limit` per entry | `SELECT id FROM parameter` by `parameter_key`, the `DELETE`, then one `unnest` insert (lines 245 to 264, 85 and 86, 88) |
| `RemoveTranslation` | `DELETE parameter_translation WHERE parameter_id = $p AND lang = $l` | `DELETE ... USING parameter p` by `recipe_version_id`, `parameter_key`, and `lang`. A CTE returns the deleted row (lines 98 to 101) |
| `RemoveParameter` | `DELETE parameter WHERE id = $p AND recipe_version_id = $v` | `WHERE recipe_version_id = $1 AND parameter_key = $2`. A CTE returns the row with its limit and translation rows (lines 75 to 80) |
| `RebindTypeRevision` | One `UPDATE parameter SET plc_type_id = $new, value = $mapped` | Per moved parameter: `update_definition`, `set_value`, and `replace_limits` (`rebind.rs` lines 101 to 107) |
| Rule of the unit lock | `SELECT archived_at FROM unit WHERE id = $u FOR SHARE` | `WHERE machine_id = $1 AND path = $2 FOR SHARE` (`unit_rows.rs` lines 36 to 38). No row gives `UnitNotFound` (`definition_steps.rs` lines 40 to 54) |

These differences only shorten the SQL. The text says nothing false about them:

- The lock statement joins `recipe` and locks with `FOR UPDATE OF v` (`draft_rows.rs` lines 28 to 31). The domain checks the state of the version (`crates/recipe-domain/src/recipe/draft/load.rs` lines 20 to 26).
- The touch adds `RETURNING row_version`. `INSERT draft_editor` adds `ON CONFLICT DO NOTHING` (`draft_rows.rs` lines 37 to 41).
- The unit lock is followed by a read of the live units of the machine (`definition_steps.rs` lines 46 to 54).
- The load also reads the `draft_editor` rows and the parameter rows (`draft_rows.rs` lines 33, 34, and 65 to 70).
- `INSERT parameter` is an `INSERT ... SELECT` that copies `plc_platform` from the version row. It also writes `id` and `recipe_version_id` (`draft_writes.rs` lines 29 to 33).
- `AddParameter` reads the image of the new row in one more statement (lines 67 to 71 and 189 to 193).
- `UpsertTranslation` is an `INSERT ... SELECT` from `parameter` by `parameter_key` (lines 90 to 95).
- The domain checks that the parameter exists in the loaded draft (`load.rs` lines 60 to 67). No SQL statement makes this check.

A statement that finds no row also gives a code that the rows do not name:

- The touch gives `E-DB-008` (`draft_rows.rs` line 133). `missing_row` builds this error (`crates/recipe-db/src/error.rs` lines 120 to 124).
- `UPDATE parameter`, the `SELECT` of `$p`, the upsert, and the two `DELETE` statements with a row image also give `E-DB-008`.
- Their checks are in `draft_writes.rs` lines 218 to 220, 238 to 240, 257, 287 to 289, 309, and 326.
- `INSERT parameter` without a version row gives `MissingReference`, which is `E-DB-007` (lines 183 to 187).
- `INSERT draft_editor` uses `ON CONFLICT DO NOTHING`. The `DELETE` of the limit rows and the limit insert check no row count (lines 136 to 156 and 258 to 263).
- A conflict on `parameter_tag_unique` gives `E-RCP-014` (`definition_steps.rs` lines 131 to 145). Another unique conflict gives `E-DB-006`.

The locks and the domain check make the paths without a row need a storage defect.

Other text of these rows and rules disagrees with the code:

- The guard cell of `AddParameter` names no `S7TypeNotAddressable` (`E-RCP-058`). The domain runs that check (`crates/recipe-domain/src/recipe/draft/add/step_definition.rs` line 83, `draft/s7.rs` lines 169 and 242).
- ADR-0178 (c), `domain-model.md` §5.2, and `api.md` §2.6 name `E-RCP-058` for `AddParameter`.
- Before the unit lock, `AddParameter` checks the type: current revision, no nested array, and the S7 nesting count (`add_parameter.rs` lines 115 to 120).
- `UpdateParameterDefinition` runs these checks only for a changed type (`update_definition.rs` lines 168 to 175, ADR-0179 (e)).
- The guard cell of `AddParameter` names `parameter_tag_unique` as the only source of `DuplicateTag`. The domain checks the tag first (`step_definition.rs` lines 69 to 75).
- `api.md` §2.6 says that `SetParameterValue` does not read `parameter_limit`. The load reads the limit and translation rows (`draft_rows.rs` line 70, `parameter_rows.rs` lines 27 to 33).
- The domain checks the stored limits of `SetParameterValue` (`crates/recipe-domain/src/recipe/draft/value.rs` line 33, ADR-0120 (d)).

### `InvalidLangCode` of `RemoveTranslation`

`recipe-web` parses the path value `{lang}` with `LangCode::parse` before the use case. A malformed code gives `422 E-RCP-027` (`crates/recipe-web/src/pages/path_values.rs` lines 178 to 185). The routes of `UpsertTranslation` and `RemoveTranslation` both read `{lang}` this way (`crates/recipe-web/src/pages/parameters/translation_routes.rs` lines 79 to 94, 342, and 418). The use cases get a parsed `LangCode` (`translation.rs` lines 3 and 4). `api.md` §1 gives `422 E-RCP-027` for a malformed `{lang}`.

The row `UpsertTranslation` of §6 and of `domain-model.md` §5.2 names `InvalidLangCode`. The row `RemoveTranslation` names it in neither document. The route of `RemoveTranslation` in `api.md` §2.6 names no `422 E-RCP-027`.

The form parser of `AddParameter` and `UpdateParameterDefinition` parses the field `unit_path` with `UnitPath::parse` (`crates/recipe-web/src/pages/parameters/definition.rs` lines 87 to 98 and 124). A malformed path gives `E-PLT-017`. More than five segments give `E-PLT-002` (`crates/recipe-domain/src/plant/unit_path.rs` lines 42 to 55). No row names these codes.

### Gaps outside this ADR

A follow-up ADR decides the codes of a stored part that fails the earlier rule set. A read gives `500 E-DB-008` for a storage defect (ADR-0118 (d)). A command gives the code of the broken rule for the same defect, with the status of its row in `reliability.md` §1.2. The follow-up ADR starts from these sites. The paths are in `crates/recipe-domain/src/recipe/` when no crate is named:

- `SetParameterValue`: a stored limit that the tree rejects gives `E-RCP-006`, `E-RCP-017`, `E-RCP-018`, or `E-RCP-019` (`draft/value.rs` line 33).
- `ReplaceLimits`: a stored value of the wrong form gives `E-RCP-005` (`draft/value.rs` line 63).
- `RebindTypeRevision`: a stored value of the wrong form gives `E-RCP-005` (`rebind/value.rs` lines 40 and 53). Line 43 is a case of the tree shape.
- `RebindTypeRevision`: a kept limit that the planned tree rejects gives a limit code (`rebind/limit.rs` lines 49 to 55).
- `RebindTypeRevision`: a stored value outside a stored limit gives `E-RCP-004` (`rebind/apply.rs` lines 201 to 209).
- `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision` check the platform, width, and alignment rules of every parameter (`replace_parameters`, `draft/s7.rs` lines 131 and 138).
- Another stored parameter can therefore give `E-RCP-003`, `E-RCP-053`, `E-RCP-054`, or `E-RCP-023`. The callers are `draft/add.rs` line 71, `draft/definition.rs` line 94, `draft/value.rs` line 81, and `rebind/apply.rs` line 136.
- The rebind checks the address rules of each moved parameter in `check_definition` (`rebind/apply.rs` line 212). A stored address can give the same four codes (`draft/s7.rs` lines 165 to 167).
- Every draft command: two parameters with one tag, key, or identifier give `E-RCP-014` (`draft/load.rs` lines 27 to 39).
- Every draft command: two type rows with one identifier that both pass the machine and platform filter give `E-PLT-013` (`crates/recipe-domain/src/plant/type_tree.rs` lines 43 to 54).
- `SetParameterValue` and `ReplaceLimits`: a type cycle gives `E-PLT-005`, and a type above a size limit gives `E-PLT-018` (`type_tree.rs` lines 441, 444, and 591).
- The `NotFound` arms of `OpenDraft::open` (`crates/recipe-app/src/recipe/draft_session.rs` lines 109 to 111, 124 to 126, and 146 to 148).
- The `NotFound` arm of a missing machine in the unit lock (`crates/recipe-app/src/recipe/definition_steps.rs` lines 50 to 53).
- `UpdateParameterDefinition` cannot replace a type that does not resolve, against ADR-0028. The audit diff reads the tree of the old type.
- The diff is in `crates/recipe-app/src/recipe/update_definition.rs` line 191 and `crates/recipe-app/src/audit/diff.rs` lines 103, 153, 226, and 231 to 238.
- The follow-up ADR decides between a code change of the diff and an amendment of the sentence of ADR-0028 that `UpdateParameterDefinition` "still works".
- `random_bytes` also makes the session token, the CSRF token, and the one-time password (`crates/recipe-app/src/identity/issue.rs` lines 90 and 91, `crates/recipe-app/src/identity/one_time.rs` line 78).
- A failed read gives `E-WEB-011`. The rows of `Login` and `ChangeOwnPassword` name `E-WEB-011` only for the password hash.
- `api.md` §2 and `reliability.md` §1.1 name only a failed `spawn_blocking` task and a closed sender as causes of `E-WEB-011`. They omit a failed read of random bytes.
- The shared sentence of `domain-model.md` §5 about the error set of a command holds apart from the codes of this section.

## Decision drivers

- The normative text states the behavior of the shipped code when the code meets the requirements (R34).
- Each command lists its complete error set (R35).
- A shared code uses the form that the document already uses for shared codes.
- The statement column gives no predicate, statement order, or count of write statements that the adapter does not run.

## Considered options

### Option A: Change the text to the code

- Pros: no code change. The rows then state the SQL and the error set of the code.
- Cons: the rows get longer.

### Option B: Change the code to the text

- Pros: the rows stay.
- Cons: a predicate by `id` needs the parameter identifier in every adapter call. The bulk insert of the limit rows becomes one statement per row. The shared codes come from the permission check, the lookups, and the commit, so the code cannot drop them.

## Decision

The project uses **Option A**. No code, no migration, and no spec change. Each code that this ADR adds has its row in `reliability.md` §1.2.

Rules of `data-model.md` §6:

- (a) The rule "Every draft command can return ..." gets this text:

  "Every draft command can return `NotFound` (`E-WEB-005`) for a malformed `{ref}`, `{n}`, or `{key}` (`api.md` 1). It can also return `NotFound` when the recipe or the version of the URL does not exist. It can return `VersionNotEditable`, `RecipeDeleted`, and `ConcurrentEdit` from these steps. The touch, the `SELECT` of the parameter identifier, and a write of a `parameter` or `parameter_translation` row give `E-DB-008` when they find no row. `INSERT parameter` without a version row gives `E-DB-007` instead. The guard column of a draft command lists its other errors."

Rows of `data-model.md` §6:

- (b) `AddParameter`: one `INSERT parameter_limit ... SELECT ... FROM unnest(...)` writes all entries of `limits[]`. An empty `limits[]` runs no insert.
- (c) `UpdateParameterDefinition`: the `UPDATE` selects the row by `recipe_version_id` and `parameter_key`. With `value` and `limits[]`, `UPDATE parameter SET value` runs first. The limit statements of `ReplaceLimits` follow: the `SELECT` of `$p`, the `DELETE`, and, for a non-empty `limits[]`, the `unnest` insert.
- (d) `SetParameterValue`: the `UPDATE` selects the row by `recipe_version_id` and `parameter_key`. The error cell adds `DerivedTypeInvalid` (`E-PLT-013`, ADR-0028) for a parameter type, or a type in its tree, that is not on the platform.
- (e) `ReplaceLimits`: the statement column gives the `SELECT` of the parameter identifier by `parameter_key`, the `DELETE`, and the one `unnest` insert. An empty `limits[]` runs no insert. The error cell adds the same `E-PLT-013` sentence as item (d).
- (f) `RemoveTranslation`: the `DELETE` joins `parameter` and selects by `recipe_version_id`, `parameter_key`, and `lang`. It returns the deleted row for the audit entry. The error cell becomes "`InvalidLangCode` (`E-RCP-027`) from the path value `{lang}` before the use case. `TranslationNotFound` from the domain check. 0 rows → `E-DB-008`". This replaces the cell of ADR-0193 (g).
- (g) `RemoveParameter`: the `DELETE` selects the row by `recipe_version_id` and `parameter_key`. It returns the row with its limit and translation rows for the audit entry.
- (h) `RebindTypeRevision`: for each moved parameter, the statement column gives the statements of three adapter calls. The `UPDATE` of the definition columns writes `plc_type_id = $new` and writes the other columns with their current values. `UPDATE parameter SET value = $mapped` follows. Then the limit statements of `ReplaceLimits` write the mapped limits.
- (i) `EnsureDerivedType`: the guard column adds two sentences. The first is "A failed read of the random bytes for the identifier of a `STRING(n)`, `WSTRING(n)`, or array row → `TaskFailed` (`E-WEB-011`)". The second is "The read runs also when the row exists".

`domain-model.md`:

- (j) §5 adds three sentences after the sentence about `draft_editor` and `row_version`. They name `E-WEB-005 NotFound` with the causes of item (a), and the `E-DB-008` and `E-DB-007` sentences of item (a). The next sentence becomes "The error column and the shared sentences of this section give the complete error set of each command."
- (k) §5.1: the errors of `EnsureDerivedType` are `E-PLT-013 DerivedTypeInvalid`, `E-PLT-018 TypeTooLarge`, and `E-WEB-011 TaskFailed`. The cause of `E-WEB-011` is the random bytes of the identifier of a `STRING(n)`, `WSTRING(n)`, or array row. The sentence about a new array type of `RebindTypeRevision` says "its only domain error is `E-PLT-018`". This narrows ADR-0029 (a new array type of a rebind) to domain errors.
- (l) §5.2: the rows `SetParameterValue` and `ReplaceLimits` add `E-PLT-013 DerivedTypeInvalid` with the cause of item (d). The row `RemoveTranslation` adds `E-RCP-027 InvalidLangCode` (the path value `{lang}`).

Shared codes of every command:

- (m) `data-model.md` §6 adds a rule after the rule of item (a). The rule has this text:

  "Every command except `Login` returns `Forbidden` (`E-WEB-002`) when the matrix of `api.md` 3.3 denies the role of the user. Every command can return `AuditWriteFailed` (`E-AUD-001`) from the audit insert at the commit, except on a path that writes no audit entry (`domain-model.md` 5). A command that reads random bytes for an identifier, directly or through `EnsureDerivedType`, can return `TaskFailed` (`E-WEB-011`) when the read fails. A path value or a request field that fails its parse gives its error before the use case (`reliability.md` 4.3). `api.md` 1 gives the code of each path value. The field `unit_path` gives `E-PLT-017` for a malformed text and `E-PLT-002` for more than five segments."
- (n) `domain-model.md` §5 adds a paragraph with the same six rules after the paragraph about the `AuditEntry` rows. Its `E-AUD-001` sentence ends "except on a path without an `AuditEntry`".

Other text of the draft rows and rules:

- (o) The rule of the unit lock in §6 becomes: "`AddParameter` and `UpdateParameterDefinition` then run `SELECT ... FROM unit WHERE machine_id = $m AND path = $unit_path FOR SHARE`. No row gives `UnitNotFound` (`E-PLT-020`). They require a live unit (`UnitArchived`)."
- (p) `EnsureDerivedType`: the statement column resolves the `TypeExpr` against the type rows that the caller read. A primitive or a struct must be in these rows. A `STRING(n)`, `WSTRING(n)`, or array can match no row of the machine in these rows. Then `INSERT plc_type ... ON CONFLICT DO NOTHING RETURNING` writes it with the platform of the machine in `platforms`. When the insert writes no row, a `SELECT` reads the existing row.
- (q) `AddParameter`: the statement column adds this sentence after the parse:

  "Before the unit lock, the command checks the type that it writes: current revision, no nested array, and the S7 nesting count of ADR-0179 (e)."

  `UpdateParameterDefinition` adds the same sentence for a changed type.
- (r) `AddParameter`: the guard column gives "`DuplicateTag` from the domain check. `parameter_tag_unique` → `DuplicateTag` as the second line". It adds `S7TypeNotAddressable` (`E-RCP-058`, ADR-0178) after `S7DataBlockOutOfRange` (`E-RCP-057`).
- (s) `RebindTypeRevision`: the sentence about the three adapter calls becomes two sentences. "The command makes three adapter calls for each moved parameter. A moved parameter has the type `from_type_id`, or an array type with that element type."

`api.md` §2.6:

- (t) The route `POST .../translations/{lang}/delete` adds `422 E-RCP-027`, as the route of `UpsertTranslation` does.
- (u) The rule about `SetParameterValue` becomes "`SetParameterValue` reads the stored limits and writes no `parameter_limit` or `parameter_translation` row."

## Consequences

### Positive

- The draft rows of §6 give the predicates, the order, and the count of the write statements of the adapter.
- The error sets of the draft rows name the shared codes and `E-RCP-027` of `RemoveTranslation`. They name `E-PLT-013` for a type that is not on the platform.
- `data-model.md` §6 and `domain-model.md` §5 give the shared codes of every command in one rule each.
- The row `EnsureDerivedType` and `domain-model.md` §5.1 give its domain errors and `E-WEB-011`. Its storage codes are the shared codes of `api.md` §2.

### Negative

- The codes of stored data that breaks the earlier rule set stay out of the rows until the follow-up ADR. A command and a read give different answers for one storage defect.
- The rows name `E-PLT-013` only for a type that is not on the platform. The follow-up ADR decides the other resolution errors.
- The shared rule gives the parse code of one request field only, `unit_path`. The other fields keep the shared codes of `api.md` §2.

### Risks and mitigations

- Risk: a later change of the adapter changes a predicate or a statement order. Then the rows no longer state the code.
- Risk: no test runs `SetParameterValue` or `ReplaceLimits` on a type whose resolution fails. The domain tests cover only a type that the trees do not hold.
- Mitigation: the atomic tests of `crates/recipe-db/tests/storage/atomic/draft.rs` make a later statement fail and check that no partial write stays (lines 1 to 5). No test compares the text of §6 with the SQL.

## Related

- ADR-0017, ADR-0028, ADR-0029, ADR-0118 (d), ADR-0120 (c), (d), and (j), ADR-0178 (c), ADR-0179 (e), ADR-0191, ADR-0193
- `data-model.md` §4.2 and §6, `domain-model.md` §5, §5.1, and §5.2, `reliability.md` §1.2 and §4.3, `api.md` §1, §2.6, and §3.3
- `overview.md` §7: R28, R29, R34, R35
