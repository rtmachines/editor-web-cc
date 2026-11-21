# Design

## Context

See proposal.md, section Why. The review is `docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md`. Its source file is `docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md`. The review and the source file are not normative.

The change uses four existing mechanisms:

- Type rules without a branch of `RuleSet`. A command checks only the type text that it writes (ADR-0143 (i2), ADR-0166 (t)).
- Value rules with the earlier rule set for stored values (ADR-0120, ADR-0166).
- A machine field with an expand migration, a `NOT VALID` check, and a second migration that runs `VALIDATE CONSTRAINT` (ADR-0156, ADR-0157).
- The export check, which sets `loadable` and keeps the hash (ADR-0119).

## Goals / Non-Goals

**Goals:**

- Put each new rule into the mechanism that matches its kind. The only new mechanism is the encoding of a machine.
- Keep every stored row readable and every draft editable (ADR-0028, ADR-0120).
- Give each stored-data rule an ops query part, so an operator can list affected rows before an upgrade.

**Non-Goals:**

- No new trigger. The trigger `plc_type_struct_has_member` does not change (ADR-0204 (g)).
- No change of a stored hash, of the v1 hashed body, or of the v1 schema file.

## Decisions

Each decision has its ADR. This section gives the placement of each rule and one decision of this change (transitive nesting).

### Map of the items

The table maps each owner question and each review item to its ADR, its requirement, and its tasks. A requirement name is a heading in `specs/<capability>/spec.md`.

| Item | ADR | Requirement (capability) | Tasks |
|---|---|---|---|
| OQ01 | ADR-0199, A51 | "Checks before a plant adds a CODESYS machine" (operations) | 1.7, 1.9 |
| OQ02 | ADR-0201 | "WSTRING characters on CODESYS and TwinCAT" (recipe-management). "WSTRING characters of CODESYS and TwinCAT in the export" (export). "STRING encoding of a stored machine after the upgrade" (operations) | 2.3, 3.1 to 3.4, 10.1, 10.2 |
| OQ03 | ADR-0202 | The five requirements on the `STRING` encoding (plant-model). "STRING values on a Utf8 machine" (recipe-management). "STRING encoding in the export document v2" and "Loadable flag of a v1 document of a Utf8 machine" (export). The four requirements on the encoding (web-ui). "Rollback on the newer schema after the upgrade" (operations) | 2.1 to 2.5, 9.2, 10.1, 10.3, 11.1 to 11.20 |
| OQ04 | ADR-0200, A52 | "Checks before a plant adds a CODESYS machine" (operations) | 1.9 |
| OQ05 | ADR-0203 | "Reserved names on CODESYS and TwinCAT" (plant-model), "Ops query parts of the CODESYS and TwinCAT rules" (operations) | 4.1 to 4.3 |
| OQ06 | ADR-0204 | "Two members of a struct on CODESYS" (plant-model), "Ops query parts of the CODESYS and TwinCAT rules" (operations) | 5.1 to 5.3 |
| OQ07 | ADR-0205 | "Long prefixes of IEC literals" (recipe-management) | 6.1, 6.2 |
| OQ08 | ADR-0206 | "Vendor aliases of type names" and "Reserved struct reference in the preview of the parameter form" (plant-model) | 7.1 to 7.3 |
| OQ09 | ADR-0207, A53 | "No layout in the export of CODESYS and TwinCAT" (export) | 10.3, 11.18 |
| review item R01 | ADR-0208 | "Type positions of the schema v2" and "Type and value rules of the schema v2" (export) | 8.1, 10.2 |
| review item R02 | ADR-0209 (a) | "Type texts outside the type grammar" (plant-model) | 1.1 |
| review item R03 | ADR-0117 | "No NaN and no infinity in a REAL or LREAL leaf" (recipe-management) | 1.3 |
| review item R04 | ADR-0137 (b) | "Last Latin-1 character of a STRING value" (recipe-management) | 1.4 |
| review item R05 | ADR-0141 | "WSTRING length of zero" (plant-model) | 1.2 |
| review item R06 | ADR-0209 (c) | none, docs only (`data-model.md` §7) | 1.9 |
| review item R07 | ADR-0209 (a) | none, docs only (`data-model.md` §7) | 1.9 |
| review item R08 | ADR-0137 (i) | none, comments only | 1.8 |
| review item R09 | ADR-0115, ADR-0139 | the range requirements of the base specs. The new golden files pin them | 1.6, 1.7 |
| review item R10 | ADR-0209 (d), ADR-0181, ADR-0202 (i) | "UTF-16 code units of WSTRING and WCHAR values", "Latin-1 bytes of STRING and CHAR values" (export) | 1.9 |
| review item R11 | ADR-0209 (e), (f) | "Order of array dimensions in a JSON value" (recipe-management) | 1.5 |

### Kind of each rule

| Decision | ADR | Kind | Code | Stored rows |
|---|---|---|---|---|
| OQ01 | ADR-0199 | Assumption A51, docs only | none | no change |
| OQ02 | ADR-0201 | value rule with a branch of `RuleSet` | `E-RCP-005`, item of `E-RCP-020` | earlier set keeps ADR-0137, approval refused, export `loadable` false |
| OQ03 | ADR-0202 | machine field, new command, value rule without a branch | `E-PLT-053`, `E-PLT-054`, `E-RCP-005` | every stored machine gets `Latin1`, no value changes |
| OQ04 | ADR-0200 | Assumption A52, docs only | none | no change |
| OQ05 | ADR-0203 | type rule in the use case | `E-PLT-013` "reserved name" and "reserved member name" | readable, kept by rebind, left out of proposals |
| OQ06 | ADR-0204 | type rule in the use case | `E-PLT-013` "too few members" | as OQ05 |
| OQ07 | ADR-0205 | literal parser | no new code | no change |
| OQ08 | ADR-0206 | type parser | no new code | no change |
| OQ09 | ADR-0207 | Assumption A53, docs only | none | no change |

### Placement in the code

- ADR-0201: `crates/recipe-domain/src/recipe/value/leaf.rs` gets a third `WSTRING` branch for `CodesysIec` and `TwinCat3` under `RuleSet::Current`. The leaf type already holds the platform and the rule set. A new variant `Expected::WstringIec { length }` carries the form of ADR-0201 (c). `recipe-web` maps it to the key `expected-wstring-iec`.
- ADR-0202, value rule: `LeafType` gets the field `string_encoding`. `string()` and `string_length_units` take it. On `Utf8`, a `STRING` length is `text.len()`, and a character is any scalar except `U+0000`. The limit check calls the same function, so the value rule and the `max_length` rule cannot drift. A new variant `Expected::StringUtf8 { length }` maps to the key `expected-string-utf8`.
- ADR-0202, `CHAR`: a `Utf8` machine has no `CHAR` leaf, because `CHAR` does not exist on `CodesysIec` and `TwinCat3`. ADR-0202 (h) gives the rule for a later `CHAR`. This change adds no code for it.
- ADR-0203 and ADR-0204, lists and checks: a new module `crates/recipe-domain/src/plant/iec_names.rs` holds the one copy of the 15 struct names and of the 2 extra keywords. The 182 keywords stay in `type_name.rs`.
  - The module has the predicates `is_iec_reserved_struct_name` and `is_iec_reserved_member_name`.
  - It has two checks of one struct. `check_iec_names(platform, name, members)` runs ADR-0203 (a) and (d). `check_codesys_members(platform, name, members)` runs ADR-0204 (a). The argument `name` gives the type text of the error.
  - Two checks let task 4.2 enforce the names before task 5.2 enforces the count. Thus the tests of task 5.2 fail before that task.
  - `DerivedTypeReason` gets `ReservedMemberName` and `TooFewMembers`. The module follows `check_siemens_struct_name` of `plant/siemens_names.rs`.
- ADR-0203 and ADR-0204, enforcement: the rule has no branch of `RuleSet` (ADR-0203 (l), ADR-0204 (f)). The struct row constructor of `plant/plc_type.rs` and `PlcType::from_stored` do not change. A use case of `recipe-app` calls both checks on the data that it writes:
  - `DefineUdt` checks the new struct in `require_current_names` of `crates/recipe-app/src/plant/define_udt.rs`, beside `check_siemens_struct_name`.
  - `DefineUdt` checks the stored struct row of a member type in `ensure_members` of the same file. There each member type resolves to its row, also as the element of an array.
  - `AddParameter`, `UpdateParameterDefinition` with a changed type, and `DefineUdt` check the stored struct revision that a written type text names directly. The name can be the base, the element of an array, or a member type (ADR-0203 (i1), ADR-0204 (c1)).
  - The new function `require_iec_struct(open, expr)` sits beside `require_s7_nesting` in `crates/recipe-app/src/recipe/definition_steps.rs`. It reads the type rows of the open draft, as `require_s7_nesting` does.
  - `require_iec_struct` calls the pure function `check_iec_struct(types, expr)` of `crates/recipe-app/src/plant/current_type.rs`. It takes `PlatformTypes` and finds the stored revision that `expr` names directly. The preview of the parameter form calls the same function (ADR-0203 (j1), ADR-0204 (d1)).
  - `is_refused_struct_name` in `crates/recipe-app/src/plant/current_type.rs` becomes `is_refused_struct(platform, row)`. It takes the stored row and calls both checks. Both proposal lists call it: `crates/recipe-app/src/recipe/queries/parameter_form.rs` and `crates/recipe-app/src/plant/queries/types.rs` (ADR-0203 (j), ADR-0204 (d)). Both lists take the latest revision of each name first, and they leave out the name when that revision is refused.
  - `RebindTypeRevision`, the snapshot check, `ApproveVersion`, and the export do not call it (ADR-0203 (k), ADR-0204 (e)).
- Decision of this change, transitive nesting: the check covers only the stored revision that the written text names directly. It does not follow the members of that revision into a deeper stored struct.
  - Reason: a deeper prohibited struct is legacy data. The earlier rule set accepts it, and `RebindTypeRevision` keeps it (ADR-0203 (k), ADR-0204 (e)). The ops query parts of tasks 4.3 and 5.3 list it before the upgrade.
- ADR-0205: `literal_prefix` in `crates/recipe-domain/src/plant/iec_literal/mod.rs` is the one place of the literal aliases. It maps each long prefix to its short prefix before the existing match. `Display` does not change.
- ADR-0206: `parse_base` in `crates/recipe-domain/src/plant/type_expr.rs` is the one place of the type aliases. It maps the alias before `Primitive::parse`, in the primitive form only, and only under `RuleSet::Current`. Thus `TypeExpr::parse` maps it (ADR-0206 (b)).
  - Reason: a stored type text holds a catalog name. A stored alias is a corrupt row, and the earlier set must refuse it.
  - Request text: the parameter forms of `recipe-web` parse the field `type` under `RuleSet::Earlier`, because an edit form sends a stored reference such as `Int@1` again. The three places are `definition.rs:141`, `prepare.rs:83` (the preview `fields=1`), and `fields.rs:407` (`value_type`) in `crates/recipe-web/src/pages/parameters/`.
  - One helper `request_type_expr(text)` in `crates/recipe-web/src/pages/parameters/` replaces the three calls. It returns the result of `TypeExpr::parse(text)` when that result is `Ok`. Otherwise it returns `TypeExpr::from_stored(text, RuleSet::Earlier)`, so the line of a refused text does not change.
  - Thus `TOD` and `ARRAY[1..2] OF DT` give the catalog names, and `Int@1` stays an unchanged reference. `DT@1` parses as a struct reference under the earlier rule set.
  - The helper reads no row, so it cannot tell an unchanged stored reference from a new text. The form query reads the rows.
  - The save path and the preview `fields=1` run the preview of the form query before the command.
  - `GetParameterForm::execute` of `crates/recipe-app/src/recipe/queries/parameter_form.rs` checks the type text before it calls `preview_tree`. It calls `require_current_type(expr, platform)` of `plant/current_type.rs` (ADR-0206 (d3)).
  - The query skips the check only for an unchanged type text. The purpose is `UpdateParameterDefinition`, the form edits a stored parameter, and `expr_names_row(&types, stored.plc_type, expr)` holds.
  - After `require_current_type`, the same branch calls `check_iec_struct(&types, expr)` of task 4.2. Thus a new or changed reference to a stored `VERSION@1` on `TwinCat3` or to a stored one-member struct on `CodesysIec` gives `E-PLT-013` at once (ADR-0203 (j1), ADR-0204 (d1)).
  - `expr_names_row` (`plant/queries/type_text.rs:60`) compares the stored type text of the parameter with `expr.to_string()`, the canonical text of the parsed request. It does not see the sent spelling.
  - Thus `ARRAY[-0..007] OF DT@1` is unchanged for a stored `ARRAY[0..7] OF DT@1`. `UpdateParameterDefinition` uses the same call (`update_definition.rs:167`), so the form query and the command agree.
  - A new reference to a stored legacy revision is not unchanged. Thus the add form and a changed type text give "reserved name" for `DT@1`, also when the machine holds a stored `DT@1`. An unchanged `DT@1` keeps its grid.
  - `base_id` of `crates/recipe-app/src/recipe/queries/preview.rs` does not change. A missing struct revision that passes the check still gives "unknown struct".
  - The check covers each struct reference that the parser of the request accepts. It refuses the reserved words of ADR-0143 (c), and on S7 the Siemens reserved names of ADR-0186 (a). A missing `Int@9` thus gives "reserved name". A missing `ST_Zone@9` still gives "unknown struct".
  - A primitive name in exact case, such as `INT@1`, fails `request_type_expr` with "unknown struct" and never reaches the form query.
  - The fallback of `load_shown` (`value_type`) is the type of the shown grid. On an edit form, the grid of the stored type is unchanged. On the add form, a shown grid already passed the check.
  - The struct form already calls `TypeExpr::parse` (`crates/recipe-web/src/pages/types/define.rs`). The stored reads keep `from_stored(..., RuleSet::Earlier)`, so a stored alias stays a corrupt row.
- ADR-0208: the schema file gets one `if` and `then` pair for each platform. A schema test compares the enum lists of the file with the platform columns of the domain catalog. Thus the two lists cannot drift.
- ADR-0209 (f): one pin test in `crates/recipe-domain/src/recipe/value/parse/tests.rs`.

### STRING encoding (OQ03)

The encoding is a field of the machine (`domain-model.md` §2.1, `data-model.md` §3.1, ADR-0202).

- Migration. `0017_machine_string_encoding.sql` runs in one transaction. `0018_machine_string_encoding_validate.sql` runs `VALIDATE CONSTRAINT` on both checks, as `0013` and `0015` do. The column, the checks, and the grant equal `data-model.md` §3.1 and the fourth SQL block of §5.2.

  ```sql
  -- 0017_machine_string_encoding.sql
  ALTER TABLE machine ADD COLUMN string_encoding TEXT NOT NULL DEFAULT 'Latin1';
  ALTER TABLE machine ADD CONSTRAINT machine_string_encoding_value
      CHECK (string_encoding IN ('Latin1', 'Utf8')) NOT VALID;
  ALTER TABLE machine ADD CONSTRAINT machine_string_encoding_platform
      CHECK (string_encoding = 'Latin1' OR plc_platform IN ('CodesysIec', 'TwinCat3')) NOT VALID;
  GRANT UPDATE (string_encoding) ON machine TO recipe_app;
  -- 0018_machine_string_encoding_validate.sql
  ALTER TABLE machine VALIDATE CONSTRAINT machine_string_encoding_value;
  ALTER TABLE machine VALIDATE CONSTRAINT machine_string_encoding_platform;
  ```

- The column default is a literal, so `ADD COLUMN` does not rewrite the table (ADR-0084 (d)). `VALIDATE CONSTRAINT` scans under a lock that permits writes. `deploy/ci/migration_rewrite.py` accepts both files.
- Lock order of `SetMachineStringEncoding`: `machine FOR NO KEY UPDATE`, then the read of the version rows without a lock, then `UPDATE machine`, then the audit chain head (ADR-0015, `data-model.md` §6).
- Freeze rule. The command reads the version rows under its machine lock. The lock conflicts with the `FOR SHARE` lock of `CreateRecipe`, `RestoreRecipe`, and `CreateVersion`. No trigger repeats the check (ADR-0202 (d3)). Thus the value rule needs no branch of `RuleSet`.
- Archived machine. The lookup before the transaction gives `404 E-WEB-005` for a missing or archived machine. A machine that is archived after the lookup gives `409 E-PLT-010` under the lock (ADR-0202 (d2)), as `SetMachineDataBlock` does.

### Carrier of the encoding (ADR-0202 (m))

`PlatformTypes::collect` takes `string_encoding` as a new argument, and `TypeTree::string_encoding()` returns it. The argument `machine` and its filter of the owner stay. Each call site takes the encoding from the same value that gives it the platform.

- Alternative: a setter `with_string_encoding` with the default `Latin1`. Rejected, because a missed call gives the `Latin1` rule on a `Utf8` machine with no compile error.
- The old function with the default `Latin1` stays only from task 11.3 to task 11.9. It lets each path group compile alone.

The count: `rg -o 'PlatformTypes::collect' crates | wc -l` prints 56. Of these, 20 call sites in 15 files of `crates/recipe-app/src` are production code. The other matches are tests, test support, and one doc comment (`ports/types.rs:83`).

Each path gets the encoding from one read:

| Path | Read that gives the encoding | Call sites of `PlatformTypes::collect` in `crates/recipe-app/src` |
|---|---|---|
| Machine read | The six statements of `crates/recipe-db/src/transaction/machines.rs` (`SELECT_LIVE_BY_CODE`, `SELECT_ARCHIVED_BY_CODE`, `SELECT_EXPORT_BY_CODE`, `SELECT_BY_ID`, `LOCK_SHARED`, `LOCK_NO_KEY_UPDATE`) read `string_encoding` into `Machine` | `plant/queries/types.rs` (3), `plant/queries/usage.rs` (2), `plant/define_udt.rs`, `recipe/queries/parameters.rs` (3), `recipe/create_version.rs`, `recipe/queries/preview.rs` |
| Draft read | `SELECT_HEADER` and `LOCK_HEADER` of `crates/recipe-db/src/transaction/draft_rows.rs` join `machine m ON m.id = r.machine_id` and read `m.string_encoding` into `StoredDraft`. `Draft::load` copies it into `Draft` | `recipe/draft_session.rs`, `recipe/approve_version/write.rs`, `plant/ensure_derived.rs` (`DerivedTypes::new`) |
| Version read | The query use cases read the `Machine` of the version through the machine read | `recipe/queries/versions.rs`, `recipe/queries/diff.rs`, `recipe/queries/outdated.rs`, `recipe/queries/approval_preview.rs`, `recipe/approve_version/prepare.rs` |
| Export read | `read_rows` of `crates/recipe-app/src/export/document.rs` gets the `Machine` of `SELECT_EXPORT_BY_CODE` or `SELECT_BY_ID` | `export/document.rs` |

- `LOCK_HEADER` keeps `FOR UPDATE OF v`. The join locks no machine row. The encoding cannot change while the draft exists (ADR-0202 (d)).
- Some call sites get the platform as a bare argument. Each such site gets the encoding from its caller as a second argument: `create_version.rs:194`, `draft_session.rs:168`, `preview.rs:187`, and `check_tree` of `define_udt.rs:399`.
- `DerivedTypes::new` of `ensure_derived.rs` gets the encoding from its three production callers. `definition_steps.rs:75` and `rebind/new_types.rs:45` pass the encoding of `Draft`. `define_udt.rs:115` passes the encoding of its `Machine`.
  - The two draft callers have an `OpenDraft` and no `Machine`. Thus `DerivedTypes::new` moves in the draft group (task 11.6), after `Draft::string_encoding()` exists.
- `preview_tree` of `preview.rs` gets its platform from the `Machine` of the form in `GetParameterForm::execute`. That caller also passes the encoding of the same `Machine`.
- `collect_stored_types` of `draft_session.rs` takes the encoding. `draft_session.rs:181` and `rebind.rs:144` pass the encoding of `Draft`. The call in `GetParameterForm::execute` of `parameter_form.rs` passes the encoding of `Machine`.
- `approve_version/prepare.rs:150` reads the platform from `ExportHeader`. The encoding does not travel through `ExportHeader` into `HashedBody`, so the v1 hashed body cannot get it (ADR-0202 (k)).
- `check_and_hash` of `prepare.rs` gets a `Snapshot` with a `MachineId` only. `Snapshot` therefore gets the field `string_encoding`. `read_snapshot` sets it from its `Machine` before the blocking closure.
- Machine write. `INSERT_MACHINE` of `machines.rs` writes `string_encoding`, and `insert` binds it (task 11.11). Without the column in the statement, the default stores `Latin1`.
- The machine read (task 11.1) and the draft read (task 11.2) are additive changes, and each compiles alone.
- Task 11.3 adds `PlatformTypes::collect_encoded(machine, platform, string_encoding, rows)` beside the old `collect`. Tasks 11.5 to 11.8 move one path group each, so each task compiles.
- Task 11.9 deletes the old `collect` and gives the new function its name. Then the compiler finds each call site that a path task missed.

### Export of the encoding

- Export v2. `ScopeBody` and the v2 writer get a machine struct of their own with an optional `string_encoding`. The struct skips the field for `Latin1`.
  - `ScopeBody::build` takes the encoding as a parameter, because `HashedBody` never holds it. `write_scope` passes the encoding of the `Machine` of `read_rows`. `MachineBody` of v1 does not change, so the v1 hashed body cannot get the field.
- `loadable` of v1. `DocumentFacts` gets the field `string_encoding` and the method `v1_loadable()`. The method gives `loadable() && string_encoding == Latin1`.
  - The v1 writer `write_document` (`crates/recipe-export/src/document.rs:136`) writes `facts.v1_loadable()`. The v2 writer `write_scoped` (`:181`) keeps `document.loadable()`.
  - `write_scope` of `crates/recipe-app/src/export/document.rs` selects the writer by `ExportScope`. It selects the flag with the same rule: `v1_loadable()` for `ExportScope::Version` and `loadable()` for `ExportScope::Unit`. `Written` and `ExportedVersion` carry that one flag. Thus the bytes and `ExportedVersion::loadable()` cannot differ.
  - `has_rule_fault` keeps its meaning: a stored part breaks a current rule (ADR-0119). The encoding never sets it, so the rule notice of the version page does not change.
  - `recipe-web` is the only crate with the production writer (`crates/recipe-web/src/pages/export.rs`). Thus the HTTP tests of task 11.17 read `loadable` in the response body of each export path. The writers emit pretty JSON (`serde_json::to_vec_pretty`), so the body holds `"loadable": false` with a space after the colon.
- The version page gets the notice of ADR-0202 (l) in `crates/recipe-web/src/pages/versions/actions.rs`. An Approved version with a rule fault on a `Utf8` machine shows both notices. The export link names both in `aria-describedby`.

## Stored data matrix

Each rule of ADR-0201, ADR-0203, and ADR-0204 narrows what a command accepts. A stored row of the earlier release stays readable (ADR-0120). ADR-0202 adds a column to every stored machine. Tasks 2.3 and 10.1 test each row of this table on PostgreSQL.

Task 2.3 writes the rows with the code before this change, before the migration `0017`. It keeps a full plain dump of that database, with `_sqlx_migrations`. It also records both hashes and two captures of each case. A capture holds the canonical bytes of the hashed body or of the scope body, its hash, `status`, and `has_rule_fault`. It holds no `exported_at`. Task 10.1 loads the dump into an empty database and runs the migrator of this release. Thus the test also covers the upgrade path through `0017` and `0018`.

| Decision | Stored row of an approved version | Read | `loadable` after the upgrade | Hashes | Capture bytes | `CreateVersion` |
|---|---|---|---|---|---|---|
| OQ02, ADR-0201 | `WSTRING(10)` with the value `"A😀"` on `TwinCat3` | readable | false, by ADR-0201 (i) | stored `content_sha256` and `scope_sha256` | equal body bytes, `has_rule_fault` becomes true | copies |
| OQ02, ADR-0201 | `WSTRING(10)` with the value `"Z𝄞"` on `CodesysIec` | readable | false, by ADR-0201 (i) | stored `content_sha256` and `scope_sha256` | equal body bytes, `has_rule_fault` becomes true | copies |
| OQ05, ADR-0203 | a parameter of the struct `VERSION@1` on `TwinCat3` | readable | true | stored | equal | copies |
| OQ05, ADR-0203 | a parameter of `ST_Coil@1` with the members `END_IF` and `S` on `CodesysIec` | readable | true | stored | equal | copies |
| OQ06, ADR-0204 | a parameter of `ST_One@1` with one member on `CodesysIec` | readable | true | stored | equal | copies |
| OQ03, ADR-0202 | `STRING(4)` with the value `"é"` on `CodesysIec` | readable, encoding `Latin1` | true | stored | equal, no field `machine.string_encoding` | copies |

- Each case is a recipe of its own with one parameter. Thus an affected row cannot change the document of an unaffected case.
- "Equal" means the exact bytes of the captures of task 2.3. ADR-0202 adds no byte to a document of a `Latin1` machine (ADR-0202 (j2)).
- Only ADR-0201 (i) changes `loadable` of a stored version, as ADR-0180 (j) does on S7.

Each row also passes a backup and a restore: `pg_dump --format=custom`, then `pg_restore` into an empty database. The export after the restore gives the same captures.

`recipe-db` has no JSON Schema library and no production writer. Thus task 10.2 checks ADR-0208 (f) in `recipe-export`. It builds the stored rows of ADR-0201, ADR-0203, and ADR-0204 through `from_stored` under `RuleSet::Earlier`. Each v2 document of these rows passes the v2 schema file. Task 10.3 pins the bytes of the production writer for a `Latin1` machine before the writer changes in wave 4.

## Risks / Trade-offs

- [An `Approved` version on `CodesysIec` or `TwinCat3` becomes not loadable] → The part `iec_wide_text` lists the rows before the upgrade. No production server exists on 2026-11-19.
- [An old v2 consumer refuses the document of a `Utf8` machine] → The refusal fails closed. ADR-0202 adds no byte to a `Latin1` document (ADR-0202 (j2)). Only ADR-0201 (i) can change its `loadable`. The runbook tells the plant to update the consumer before it sets `Utf8` (ADR-0202).
- [A plant sets an encoding that differs from the PLC project] → The encoding freezes at the first version. The runbook names the check before a plant adds a machine.
- [The carrier change touches 56 matches of `PlatformTypes::collect`, 20 of them in production code] → After task 11.9, the new argument is not optional. Thus the compiler finds each site.
- [The two copies of the reserved names drift] → A `recipe-db` test compares the domain copy with the ops query copy.
- [A schema rule of ADR-0208 refuses a document that the writer emits] → A test checks every golden v2 document against the schema. Another test checks every v2 document of the matrix.
- [The previous release runs after a rollback] → It inserts a machine without `string_encoding`, so the column default gives `Latin1`. Both checks accept the row.
- [An image-only rollback with a `Utf8` machine or an entry `MachineStringEncodingSet`] → The previous release exports a `Utf8` machine with `loadable` `true` and Latin-1 bytes (ADR-0202 (q1)). It also reads the unknown audit action as corrupt (ADR-0202 (q2)). The query of the runbook blocks this path (ADR-0202 (q3)).
  - A `Utf8` value inside Latin-1, such as `é`, is enough. The transfer bytes differ: `16#E9` under `Latin1` and `16#C3 16#A9` under `Utf8`.
  - A change back to `Latin1` keeps the audit entry. Thus the count of the entries stays above 0.

## Migration Plan

1. Before the upgrade, run `check-identifier-rules.sql` and `check-platform-ranges.sql` against the production database (`docs/ops/runbook.md`).
2. Record each listed row in the upgrade ticket. The new parts give two kinds of result:
   - Refused only for new writes: `iec_reserved_struct_name`, `iec_keyword_member_name`, and `codesys_one_member_struct`. A listed `Approved` version stays loadable.
   - Violations that change `loadable`: `iec_wide_text`. A listed `Approved` version becomes not loadable after the upgrade.
3. Deploy the release. The migrator runs `0017` and `0018`. Each stored machine gets `Latin1`.
4. Rollback. Follow the runbook section "Roll back on the newer schema" (ADR-0202 (q)):
   1. Stop `app`, so no user can change an encoding.
   2. Run the query of the runbook. The runbook holds the one executable copy. It counts the machines with `machine.string_encoding = 'Utf8'` and the entries with `audit_log.action = 'MachineStringEncodingSet'`.
   3. If both counts are 0, deploy the previous image on the newer schema.
   4. If a count is not 0, roll forward with the new image, or restore the `_premigrate` set. The restore loses every write since that dump.

## Open Questions

None. Each owner question has its ADR. The owner answered the questions of the plan review on 2026-11-20.
