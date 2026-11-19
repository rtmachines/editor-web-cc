# ADR-0195: Codes of a storage defect in a command

Status: Accepted. Amends the sections §6, §9.2, and §10 of `data-model.md`. Amends the sections §2.4, §5, §5.2, and §5.3 of `domain-model.md`. Amends `reliability.md` §1.1, `api.md` §2 and §5.1, and `design-system.md` §10.4. Amends ADR-0018 (the one list of the rebind). Amends ADR-0027 (the variants and the conversion of `DraftError`, the list of the rebind plan). Amends ADR-0041 (b) (the one list of the faults of a value and its limits). Amends ADR-0028 (the code of a type that does not resolve, its replacement by `UpdateParameterDefinition`). Amends ADR-0120 (d) (the code of a stored part in a command). Amends ADR-0005 (the diff of a type without a tree). Amends ADR-0039 (i), ADR-0043 (k), ADR-0118 (a) and (d), and ADR-0121 (c) (the page and the table of a type that does not resolve)
Date: 2026-11-18
Deciders: project owner

## Context

ADR-0194 lists the sites of a stored part that fails the earlier rule set in its section "Gaps outside this ADR". This ADR takes these sites. The paths are in `crates/recipe-domain/src/recipe/` unless the line names another crate.

### Reads and commands give different codes

A part that fails the earlier rule set is a storage defect (ADR-0120 (j)). A read gives `500 E-DB-008` for it (ADR-0118 (d)). A command gives the code of the broken rule, with the status of its row in `reliability.md` §1.2. A command checks the part of the request with the current set (ADR-0120 (c)). It checks a stored part that it reads and does not write with the earlier set (ADR-0120 (d)).

A read gives `E-DB-008` through one mechanism:

- `check_stored_parameter` returns a `StructureFault` for a stored part that fails the earlier set (`read_check/mod.rs` lines 107 to 115 and 163 to 176).
- `StructureFault` holds the first `DraftError` of the part. Its `code()` returns the code of that error (lines 124 to 128).
- `StorageError::broken_stored_parameter` takes a `ParameterKey` and a `StructureFault`. It turns them into `StorageError::Other` (`crates/recipe-app/src/error/storage.rs` lines 99 to 110 and 145 to 156).
- `StorageError::Other` gives `E-DB-008` (`storage.rs` line 169). `Display` hides the cause.
- The cause names the key and the code of the broken rule (`storage.rs` line 104).

A draft command reaches `AppError` through `From<DraftError> for AppError`. That conversion unwraps the inner error and adds no variant (`crates/recipe-app/src/error/app.rs` lines 214 to 222). So no error that passes through `From<DraftError>` gives `E-DB-008`. `AddParameter` gives it on two other paths (`crates/recipe-app/src/recipe/add_parameter.rs` lines 160 to 164). They are a new parameter without a type tree and a fault of the value that it has just checked. `ExportVersion` already gives it through `broken_stored_parameter` (`crates/recipe-app/src/export/document.rs` lines 233 to 237).

### The sites in the domain

The domain checks the stored parts and the request parts in the same functions. Each site below reads a stored part only:

- `Draft::set_value` reads the stored limits with the earlier set (`draft/value.rs` line 33). The new value is the request part (line 34). A conflict of two stored limit rows on one leaf is a stored part, not a request part.
- `Draft::replace_limits` reads the stored value with the earlier set (`draft/value.rs` line 63). The new limit rows and the process-limit check I7 are the request part (lines 61 and 64, ADR-0120 (e)).
- Both methods call `resolved_tree` (`draft/value.rs` lines 31 and 60). It returns the stored error of a failed resolution (`draft/add/step_definition.rs` lines 93 to 103).
- `TypeTree::resolve` gives `E-PLT-005` for a cycle through a struct and `E-PLT-018` above a size limit (`crates/recipe-domain/src/plant/type_tree.rs` lines 271 to 292 and 473 to 485).
- It gives `E-PLT-013` with the reason `NotOnPlatform` for a root or a member that the platform does not hold (`type_tree.rs` lines 317 to 322 and 463).
- It gives `E-PLT-013` with the reason `RowShape` for a cycle of array rows without a struct (`type_tree.rs` lines 324 to 329 and 482 to 483). The doc comment of `RowShape` says that only a corrupt stored row or a bug gives this reason (`crates/recipe-domain/src/plant/error.rs` lines 57 to 61).
- `Draft::load` gives `E-RCP-014` when two stored parameters share a tag, a key, or an identifier (`draft/load.rs` lines 27 to 39).
- `PlatformTypes::collect` gives `E-PLT-013` when two kept type rows share one identifier (`plant/type_tree.rs` lines 52 to 53). `OpenDraft::open` calls it for every draft command (`crates/recipe-app/src/recipe/draft_session.rs` line 169).

The whole-draft check of a command runs the platform rule, the width rule, and the alignment rule on every parameter (`draft/s7.rs` lines 122 to 150). It does not run the data block rule, the end rule, or the standard-access rule there (ADR-0116 (e1)). For a parameter whose type resolution failed, it runs only the platform rule (`draft/s7.rs` line 131). It skips the rules that need the tree (`draft/guard.rs` lines 50 to 61).

Five commands run this check through `replace_parameters`: `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision` (`draft/add.rs` line 71, `draft/definition.rs` line 94, `draft/value.rs` line 81, `rebind/apply.rs` line 136). A command that writes a definition first checks it with `check_definition` (`draft/add/step_definition.rs` line 83). `SetParameterValue` and `ReplaceLimits` write no address and no type. So a fault of these three rules in the whole-draft check always comes from a stored address and a stored type.

`check_definition` also runs in the rebind for each moved parameter (`rebind/apply.rs` line 212). The address of a moved parameter is stored. A moved type is a struct or an array of that struct. The expected width of both is `B` (`crates/recipe-domain/src/plant/s7_width.rs` line 19). Both start at an even byte (`plant/s7_layout.rs` lines 270 and 316). So the platform rule, the width rule, and the alignment rule fail for the new type only when they fail for the stored type.

The rebind has four more sites:

- `map_value` reads the stored value under the old tree (`rebind/value.rs` line 40). A value without the shape of the array chain also fails (line 53). Line 43 is a fault of the tree shape, not of the stored value.
- `map_limits` checks a kept limit against the planned tree (`rebind/limit.rs` lines 49 to 55). It treats every member name outside the kept members as a dropped limit (lines 35 to 41). So a stored limit on a path that the old struct does not have becomes `E-RCP-029`, an entry of the request.
- `map_parameters` puts the faults of `map_value` and `map_limits` in one list with the request entries (`rebind/map.rs` lines 26 to 73).
- `rebound_parameter` checks the planned value against the kept limits with I7 (`rebind/apply.rs` lines 203 to 209). A new member has no stored limit, so a fault comes from two stored parts. `plan_rebind` stops at the list of `map_parameters` before this check runs (`rebind/target.rs` line 84).

### The sites in the application

- `OpenDraft::open` gives `NotFound` when a lock finds no row. The locks are on the machine, the recipe, and the version (`draft_session.rs` lines 105 to 111, 120 to 126, and 143 to 148).
- `lock_live_unit` gives `NotFound` when the live units of the machine have no machine row (`crates/recipe-app/src/recipe/definition_steps.rs` lines 46 to 53).
- `recipe_app` has no `DELETE` grant on `machine`, `recipe`, or `recipe_version` (ADR-0194, `crates/recipe-db/tests/grants.expected`). `resolve_version` found the rows before the transaction. So only a storage defect reaches these arms.
- `RebindTypeRevision` reads the type rows a second time and calls `PlatformTypes::collect` again (`crates/recipe-app/src/recipe/rebind.rs` lines 133 to 135).

### Why no write path stores these parts

- A stored value or limit that fails the earlier set is a storage defect by ADR-0120 (j). The Status line of ADR-0120 lists its amendments, ADR-0139 to ADR-0186. Each one widens the earlier set or names its parsers. None narrows it.
- `CHECK parameter_s7_address_platform` holds the platform rule (`data-model.md` §3.12, R14). Only ADR-0017, ADR-0024, ADR-0025, ADR-0049, and ADR-0194 name the width code `E-RCP-054` or the alignment code `E-RCP-023`. No ADR changed these two rules after ADR-0024.
- `parameter_tag_unique`, `parameter_key_unique`, and the primary keys of `parameter` and `plc_type` hold the duplicate rules (ADR-0194, `migrations/0001_schema.sql` lines 413 to 414).
- The size limits of a type are constants of `reliability.md` §7.11. `DefineUdt` and `EnsureDerivedType` reject a type above them (`reliability.md` §4.4).
- `DefineUdt` returns `TypeCycle` for a struct that contains itself (I8 in `domain-model.md` §3).
- No trigger checks the leaf path of a limit row. So a limit on a path that the type does not have is possible only as a storage defect.

### `E-WEB-011` of the random bytes

`random_bytes` gives `TaskFailed` with the stage `Token` (`E-WEB-011`) when the read fails (`crates/recipe-app/src/identity/issue.rs` lines 26 to 31). These calls use it:

- `IssuedSession::generate` reads the session token and the CSRF token (`issue.rs` lines 89 to 91). `Login` and `ChangeOwnPassword` call it (`identity/login.rs` line 195, `identity/change_own_password.rs` line 125).
- `hashed_one_time_password` reads the one-time password (`identity/one_time.rs` line 78). `CreateUser` and `ResetPassword` call it (`identity/create_user.rs` line 70, `identity/reset_password.rs` line 58).
- `seal_password` maps every failure of the seal of a PLC password to `TaskFailed { stage: Token }` (`crates/recipe-app/src/ports/credential_cipher.rs` lines 104 to 112). The seal fails on a failed read of random bytes and on a failed cipher (`crates/recipe-server/src/credential_cipher.rs` lines 82 and 91).

So the code already gives `E-WEB-011`. The text omits these causes:

- The rows of the four identity commands in `domain-model.md` §5.3 name `E-WEB-011` only for the password hash and the detached task.
- The rule about `TaskFailed` in `reliability.md` §1.1 names a failed `spawn_blocking` task and a closed sender. The table of §1.1 already names the stage `Token` and the seal.
- `api.md` §2 and §5.1 name only a failed `spawn_blocking` task as the cause of `500 E-WEB-011`.
- A failed Argon2 hash gives `TaskFailed` with the stage `PasswordHash` (`crates/recipe-app/src/identity/hasher.rs` lines 50 to 56). This includes a failed read of the random bytes of the salt (`hasher.rs` line 144).
- A closed Argon2 permit gives the same stage (`identity/gate.rs` lines 14 to 16 and 58, `identity/basic_permit.rs` lines 44 to 46). The rule of §1.1 omits both causes.

### The repair of a type that does not resolve

ADR-0028 says that `UpdateParameterDefinition` "still works" on a parameter whose type does not resolve. The code refuses it:

- `UpdateParameterDefinition` builds the audit diff after the domain method (`crates/recipe-app/src/recipe/update_definition.rs` line 191).
- The diff reads the value leaves of both sides with the tree of each type (`crates/recipe-app/src/audit/diff.rs` lines 96 to 106 and 145 to 156).
- `tree_of` returns the error of a failed resolution, or `E-PLT-013` for a type without a tree (`diff.rs` lines 222 to 240).
- The same `value_leaves` serves `value_diff_of` and `rebind_diff_of` (`diff.rs` lines 74 to 118). A test requires the error for a type without a tree (`crates/recipe-app/src/audit/diff/tests.rs` lines 345 to 353).
- So the old type of the parameter blocks the command, also when the request changes only the name.

The browser cannot reach the repair:

- Over HTTP, the route `POST .../parameters/{key}/definition` first runs the parameter form query with the purpose `UpdateParameterDefinition` (`crates/recipe-web/src/pages/parameters/edit.rs` lines 122 to 128).
- The form query resolves the stored type with `?` (`crates/recipe-app/src/recipe/queries/parameter_form.rs` line 169). So the route stops before the command.
- The form query writes the type text before it resolves the tree (`parameter_form.rs` lines 277 to 282). `type_text` returns `None` for a missing row on the array chain of the root and for a cycle of array rows (`crates/recipe-app/src/plant/queries/type_text.rs` lines 15 to 24). The missing row is the root row or an element row. `text_of` turns `None` into `E-DB-008` (`crates/recipe-app/src/recipe/queries/parameters.rs` lines 154 to 157).
- The form query also calls `PlatformTypes::collect` with `?` (`parameter_form.rs` lines 274 to 275). Two kept type rows with one identifier give `422 E-PLT-013` there.
- The page `GET .../parameters/{key}` runs `GetParameter` first (`crates/recipe-web/src/pages/parameters/page.rs` lines 52 to 65 and 177 to 178). `GetParameter` resolves the stored type with `?` (`queries/parameters.rs` line 530). So the page fails before it loads the form.
- `ListParameters` resolves every used type with `?` and writes every type text (`queries/parameters.rs` lines 210 to 218, 369, and 372). So the parameter table fails too.

The use case `RemoveParameter` writes no diff and works on such a parameter. Its route first runs the form query with the purpose `RemoveParameter` (`crates/recipe-web/src/pages/parameters/remove.rs` line 139).

### `CreateVersion` and its base

`CreateVersion` checks its base with `VersionSnapshot::check` (`crates/recipe-app/src/recipe/create_version.rs` lines 196 to 203). ADR-0120 (g) decides that every fault of the earlier set there gives `E-RCP-020`, and the system creates no draft. A test pins it: a base `DATE` count `72900` gives `422 E-RCP-020` with the item `E-RCP-005` (`crates/recipe-app/src/recipe/create_version/tests.rs` lines 754 to 772). The check shares `check_snapshot` with the approval check. It collects every problem into one list, also problems that are no storage defect, such as an archived unit.

## Decision drivers

- A command and a read give one code for one storage defect (R35, R36).
- The part of the request keeps the code of the broken rule (ADR-0120 (c)).
- A rule change does not lock a draft (ADR-0028).
- A user can repair a parameter whose type does not resolve in the browser (ADR-0028).
- One mechanism serves every site. A site does not get its own error path.
- No new error code.

## Considered options

### The mechanism

#### Option A: A stored-defect variant of `DraftError`

- Pros: the domain knows which part is stored at each site. `From<DraftError> for AppError` maps one variant in one place. The read path keeps its own function.
- Cons: `Draft::load` returns `DraftError` in place of `RecipeError`. The rebind mapping must keep a defect apart from its list of entries.

#### Option B: Classify by code in the application

- Pros: no change of the domain.
- Cons: one code, for example `E-RCP-006`, comes from a stored limit in `SetParameterValue` and from a request limit in `ReplaceLimits`. The code cannot tell the two parts apart.

#### Option C: Run `check_stored_parameter` in each command

- Pros: the read and the command share one check.
- Cons: that check also reads the part that the command writes. A stored value that fails the earlier set then blocks the `SetParameterValue` that repairs it.

### The repair of a type that does not resolve

- Repair A (chosen): the audit diff writes the whole stored value of the old side under the key `""`. The form query with the purpose `UpdateParameterDefinition` returns the form. The user keeps the key and the translations.
- Repair B: amend ADR-0028 so that `UpdateParameterDefinition` gives `E-DB-008`. The only repair is `RemoveParameter` and then `AddParameter`, or a restore. The user loses the key and the translations.

### The path of the repair in the browser

- Path A (chosen): `ListParameters` and `GetParameter` return a view without a tree. The page shows the definition form without the value grid. Cost: one more render path of the page.
- Path B: the two GETs give `500 E-DB-008`. The repair works only at the level of the use case, and no user reaches it.

### The resolution errors in the parameter form query

- Form A: keep the resolution errors of the form query. The HTTP answer then differs from the use case.
- Form B (chosen): the form query gives `E-DB-008` for a failed resolution of the stored type, except for the purpose `UpdateParameterDefinition`.
- Form C: also turn `E-PLT-013` of a type that the platform does not hold into `E-DB-008`. This needs an amendment of ADR-0194 (l).

### Duplicate type rows and a missing type text in the form query

- Rows A (chosen): the form query maps the error of `PlatformTypes::collect` to `E-DB-008`. A missing row on the array chain of the root keeps the `E-DB-008` of `text_of`. A cycle of array rows is a cycle.
- Rows B: text only. The form query keeps `422 E-PLT-013` for two kept type rows with one identifier. The HTTP answer then differs from the use case.
- Rows C: as Rows A, but the form query resolves the tree before it writes the type text. A missing root row or element row then gives `422 E-PLT-013`. This changes the answer of a read that works today.

### The callers of `Draft::load`

- Load A (chosen): `ApproveVersion` and `GetApprovalPreview` give `500 E-DB-008` for duplicate stored parameters. A duplicate is a storage defect in every caller, and the preview is a read of ADR-0118 (d).
- Load B: keep `409 E-RCP-014` in these two callers. This needs a second error path.

### The base of `CreateVersion`

- Base A: a base fault of the earlier set gives `500 E-DB-008`. The other items of the snapshot check keep `E-RCP-020`. This changes a check that `ApproveVersion` shares.
- Base B (chosen): `CreateVersion` keeps `E-RCP-020` (ADR-0120 (g)). The section "Gaps outside this ADR" names it.

## Decision

The project uses **Option A** of the mechanism. `E-DB-008` keeps its one row in `reliability.md` §1.2 and its owner `StorageError::Other`.

The mechanism:

- (a) `recipe-domain` gets the type `StoredDefect` next to `StructureFault`. It holds the key of the stored parameter when the site has one, and the first `DraftError` of the stored part.
- (b) `DraftError` gets the variant `Stored(Box<StoredDefect>)`. Its `code()` returns the code of the inner error, as for every carrier of `reliability.md` §1.1.
- (c) `From<DraftError> for AppError` turns `Stored` into `AppError::Storage` with `StorageError::Other` and `E-DB-008`. The cause names the key when the defect has one, and the code of the broken rule. A new constructor of `StorageError` takes only the `StoredDefect` and reads the key from it. `broken_stored_parameter` keeps its argument types.
- (d) An arm of `recipe-app` that only a storage defect reaches gives `StorageError::invariant` with `E-DB-008`. A cause text names the missing row.
- (e) The `WebError` mapping writes the one `ERROR` line, as for every `500` (`reliability.md` §1.1). The line holds `error_code`, `request_id`, and `chain`. It holds no password, hash, token, or cookie value.

The sites that give `Stored`:

- (f) `SetParameterValue`: a stored limit that fails the earlier set. A conflict of two stored limit rows on one leaf (`E-RCP-019`) is a stored part in `SetParameterValue`.
- (g) `ReplaceLimits`: a stored value that fails the earlier set.
- (h) `SetParameterValue` and `ReplaceLimits`: a failed resolution of the stored parameter type, as item (w) defines it. A type that the platform does not hold keeps `E-PLT-013` (ADR-0028, ADR-0194 (l)).
- (i) `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, and `RebindTypeRevision`: a fault of the platform rule, the width rule, or the alignment rule in the whole-draft check. An overflow of the layout in `occupy` also gives `Stored`. An overlap keeps `E-RCP-022` (ADR-0149 (a), (h)). The check for approval keeps the code of the rule.
- (j) `Draft::load`: two stored parameters with one tag, key, or identifier. `ApproveVersion` and `GetApprovalPreview` also load a draft, so they give `500 E-DB-008` too.
- (k) Every draft command: two kept type rows with one identifier in `OpenDraft::open` and in the second type read of `RebindTypeRevision`.
- (l) `RebindTypeRevision`: a stored value that fails under the old tree, or a kept limit that the planned tree rejects. A stored limit on a path that the old tree does not have also gives `Stored`. An I7 fault between the planned value and a kept limit also gives `Stored`.
- (m) `RebindTypeRevision`: a fault of the platform rule, the width rule, or the alignment rule of a moved parameter. The data block rule, the end rule, and the standard-access rule keep their codes (ADR-0116 (e), ADR-0178).
- (n) `RebindTypeRevision`: a stored defect of a value, of a limit, or of the I7 check stops the plan before the list of entries. The command gives `E-DB-008` and no list. `map_parameters` finds the I7 defect also when the same parameter has request entries. The address faults of items (i) and (m) come after a plan without entries.

Items (f) to (h) are an exception to the one list of ADR-0041 (b). A stored defect gives one `E-DB-008` and no list. Item (n) is the same exception to the list of ADR-0018 and ADR-0027.

The sites that give `StorageError::invariant`:

- (o) The `NotFound` arms of the machine lock, the recipe lock, and the version lock of `OpenDraft::open`.
- (p) The `NotFound` arm of the live units in `lock_live_unit`.

The repair of a type that does not resolve:

- (q) `UpdateParameterDefinition` replaces a type that does not resolve. When the type changes and the old type has no tree, `before_state.value` holds the whole stored value under the key `""`. `after_state.value` then holds the value leaves of the new type. The audit entry always writes both sections in that case, also for equal sides. This is an exception to the rule of ADR-0005 that a diff holds only the changed fields. The new side, `value_diff_of`, and `rebind_diff_of` keep the error of a type without a tree.

`E-WEB-011` of the random bytes:

- (r) `data-model.md` §6 and `domain-model.md` §5: the shared sentence about random bytes names "an identifier, a session token, a CSRF token, or a one-time password". A second sentence keeps the identifier of `EnsureDerivedType`.
- (s) `domain-model.md` §5.3: the rows `Login` and `ChangeOwnPassword` name "random bytes of the session" in the cause list of `E-WEB-011`. The rows `CreateUser` and `ResetPassword` name "random bytes of the one-time password" there.
- (t) `data-model.md` §6: the rows `Login` and `ChangeOwnPassword` add "A failed read of the random bytes of the session → `TaskFailed` (`E-WEB-011`)". The rows `CreateUser` and `ResetPassword` add the same text for the one-time password.
- (u) `reliability.md` §1.1 keeps the one list of the causes of `E-WEB-011`. `api.md` §2 and §5.1 refer to that list. No code changes for items (r) to (u). The causes are these:
  - a failed `spawn_blocking` task or a detached task without a result
  - a failed read of the random bytes of an identifier, a session token, a CSRF token, or a one-time password
  - a failed seal of a PLC password
  - a failed Argon2 hash, also a failed read of the random bytes of its salt
  - a closed Argon2 permit

`data-model.md` §6 adds this text in a new rule after the rule that starts "Every draft command can return ...":

- (v) "A draft command can read a stored part that it does not write. When that part fails the earlier rule set, it is a storage defect (ADR-0120 (j)). The command then gives `E-DB-008` and writes nothing (ADR-0195). The part of the request keeps the code of its rule."

`domain-model.md` §5 adds the same text after the paragraph of the shared codes.

The terms of the type sites:

- (w) A failed resolution of a stored type is an `E-PLT-005` or an `E-PLT-018` of `TypeTree::resolve`, or an `E-PLT-013` with the reason `RowShape`. The last one covers a cycle of array rows and a stored row of the wrong shape. Only a corrupt stored row or a bug gives the reason `RowShape` (`plant/error.rs` lines 57 to 61). A stored row that fails `PlcType::from_stored` fails earlier, in the adapter of `recipe-db` (`crates/recipe-db/src/transaction/type_rows.rs` line 202). Every read and command that reads the type rows of that machine then gives `500 E-DB-008`. Items (ab), (ad), and (ae) do not cover such a row.
- (x) A type that the platform does not hold gives `E-PLT-013` with the reason `NotOnPlatform`. It is a root type, a member type, or an array element type without a kept row.
- (y) A type that does not resolve is one of three cases. It has a failed resolution, the platform does not hold it, or it has no type text.

The parameter form query (`GetParameterForm`):

- (z) The form query gives `500 E-DB-008` for a failed resolution of the stored type. A member type that the platform does not hold keeps `422 E-PLT-013`. The preview of a type of the request keeps its codes.
- (aa) The form query maps the error of `PlatformTypes::collect` to `E-DB-008`. A missing type text keeps the `E-DB-008` of `text_of`. A missing row on the array chain of the root (the root row or an element row) therefore gives `E-DB-008`. These are the exceptions to the `422` of item (x).
- (ab) For the purpose `UpdateParameterDefinition`, the form query returns the form for a stored type that does not resolve. The form then holds no rule faults of the stored parameter, and it holds no type text when none exists. Two kept type rows with one identifier still give `E-DB-008`.
- (ac) The routes of `SetParameterValue`, `ReplaceLimits`, `RemoveParameter`, `UpsertTranslation`, and `RemoveTranslation` run the form query first. So they give `500 E-DB-008` for a failed resolution of the stored type. The use case `RemoveParameter` still works on such a parameter.

The parameter pages:

- (ad) `ListParameters` and `GetParameter` return a view without a tree for a parameter whose type does not resolve. That view has no value text and no rule faults. It has no type text when none exists. Its row skips the read check of ADR-0121 (c), because that check needs the tree. The parameter table of a version that is not approved shows such a parameter without a value text. The table of an approved version keeps the `500 E-DB-008` of `GetVersion` (ADR-0118 (d), ADR-0119 (l)).
  - The value cell of a row without a type text is empty. A row with a type text and without a value text shows the word of `parameters-value-composite` (`design-system.md` §10.4).
- (ae) The page of such a parameter shows its definition without the value grid. A reader who gets the edit form today gets the definition form without the value grid. That reader has a live draft of a live recipe, `ParameterEdit`, and a mode that permits `UpdateParameterDefinition`. Every other reader gets the read-only definition without the value grid.

The approval:

- (af) `domain-model.md` §5 names the `E-DB-008` of item (j) for `ApproveVersion` and `GetApprovalPreview`. The row `ApproveVersion` of `data-model.md` §6 names it for the command and for `GetApprovalPreview`.
- (ag) The sentence of `domain-model.md` §5 that names the steps that give `E-DB-008` when they find no row adds the locks of items (o) and (p). The exception "The codes of a storage defect" of that section narrows to the sites of the section "Gaps outside this ADR".

## Consequences

### Positive

- A command and a read give `E-DB-008` for one storage defect.
- The part of the request keeps its code. The user can still repair a stored value and its limits in either order (ADR-0120).
- One variant and one conversion carry every site of the domain.
- A user with `ParameterEdit` can replace a type that does not resolve in the browser. The parameter keeps its key and its translations.

### Negative

- `Draft::load` returns `DraftError`. Three callers change.
- The rebind mapping keeps a defect apart from its list of entries.
- The parameter page gets a second render path without a value grid.
- A reader of the audit entry sees the key `""` once.

### Risks and mitigations

- Risk: a later ADR narrows the earlier set or an address rule. Then a stored part breaks it, and the command locks the draft with `500`. Mitigation: such an ADR must name the earlier set of ADR-0120 for the stored part.
- Risk: a reader of the log sees a stored value in the chain. Mitigation: a value is not a secret by `reliability.md` §1.1. The chain holds no password, hash, token, or cookie value.
- Risk: the page hides the value of a parameter whose type does not resolve. Mitigation: the repair by `UpdateParameterDefinition` writes the old value into the audit entry under the key `""`.

## Gaps outside this ADR

- `DefineUdt`, `ApproveVersion`, `CreateVersion`, `ExportVersion`, and the queries other than the parameter form query call `PlatformTypes::collect` with `?`. They give `E-PLT-013` for two kept type rows with one identifier. A later ADR can take these sites.
- `EnsureDerivedType` calls `PlatformTypes::collect` too (`crates/recipe-app/src/plant/ensure_derived.rs` line 77). A draft command reaches it only after `OpenDraft::open` read the same type rows.
- `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` keep the code of a type of the request whose stored rows fail. That code is `E-PLT-005`, `E-PLT-018`, or `E-PLT-013` with the reason `RowShape`, and `E-RCP-052` for the source or target of the rebind. The last one covers a requested struct whose member enters a cycle of array rows (`crates/recipe-app/src/recipe/definition_steps.rs` line 127). The request names that type, so this ADR keeps the code.
- `replace_parameters` checks the version budgets before the S7 rules (`draft/replace.rs` lines 32 to 33). A stored draft above a budget gives `422 E-RCP-021` in a draft command. The budgets are constants, so only a storage defect reaches this case. This ADR keeps the code.
- `GetLimits` resolves the stored type with `?` (`crates/recipe-app/src/recipe/queries/parameters.rs` line 608). It keeps the codes of a failed resolution.
- `CreateVersion` keeps `422 E-RCP-020` for a base part that fails the earlier set (Base B). ADR-0120 (g) decides it, and the base is immutable. The one item list names every problem of the base for the operator. A later ADR can take this site with the other gaps.
- `ApproveVersion` keeps `422 E-RCP-020` and the codes of the S7 rules for a stored part that fails the earlier set. ADR-0120 (f) checks every parameter with the current set, and the item list names each problem for the approver. ADR-0116 (e) refuses an address. A later ADR can take this site.

## Related

- ADR-0005, ADR-0018, ADR-0024, ADR-0027, ADR-0028, ADR-0039 (i), ADR-0041 (b), ADR-0043 (k), ADR-0116 (e), ADR-0118 (a) and (d), ADR-0119 (l), ADR-0121 (c)
- ADR-0120 (c), (d), (e), (f), (g), and (j), ADR-0149 (a) and (h), ADR-0178, ADR-0194
- `data-model.md` §6, §9.2, and §10, `domain-model.md` §2.4, §5, §5.2, and §5.3, `reliability.md` §1.1 and §1.2, `api.md` §2 and §5.1, `design-system.md` §10.4
- `overview.md` §7: R14, R29, R35, R36
