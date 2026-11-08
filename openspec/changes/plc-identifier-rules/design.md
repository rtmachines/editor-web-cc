# Design

## Context

See `proposal.md`, section "Why". ADR-0143, ADR-0144, and ADR-0145 hold the rules. ADR-0017, ADR-0024, and ADR-0120 hold the context of the derived types, the member names, and the earlier rule set. This design links to them and does not repeat them.

The state of the code at HEAD `68e2def`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Struct name rule | `is_name` checks the grammar. `TypeName::parse` refuses a primitive name in exact case. The doc comment says that `Int` is valid | `crates/recipe-domain/src/plant/type_name.rs` lines 13 to 14, 68 to 80, 89 to 97 |
| Member name rule | `MemberName::parse` checks the grammar only | `type_name.rs` lines 118 to 123 |
| Pin of the old rule | `reserved_check_is_case_sensitive` asserts that `Int` and `int` pass | `crates/recipe-domain/src/plant/type_name/tests.rs` line 73 |
| Member list check | `check_struct` puts each `MemberName` into a `BTreeSet`, so the compare is exact. It runs inside `PlcType::try_from` | `crates/recipe-domain/src/plant/plc_type.rs` lines 122, 212, 247 |
| Reasons of `E-PLT-013` | `DerivedTypeReason` has `Grammar`, `UnknownStruct`, `NotOnPlatform`, `OutOfRange`, `RowShape`. The web maps each reason to a key | `crates/recipe-domain/src/plant/error.rs` lines 42 to 75, `crates/recipe-web/src/error/plant.rs` line 18 |
| Struct reference | `parse_udt` calls `TypeName::parse` and maps `Reserved` to `UnknownStruct` | `crates/recipe-domain/src/plant/type_expr.rs` lines 211 to 225 |
| Rule sets | `RuleSet::{Current, Earlier}` of ADR-0120, exported from `plant` | `crates/recipe-domain/src/plant/iec_literal/range.rs` line 63, `plant/mod.rs` line 33 |
| Reads of stored names | `shape_of` parses the stored name with `TypeName::parse`. `build_type` calls `PlcType::try_from`. `rows` fails as a whole when one row fails | `crates/recipe-db/src/transaction/type_rows.rs` lines 137, 168, 178, 191, 208 |
| Audit label | The label of a `plc_type` entity parses the stored name with `TypeName::parse` | `crates/recipe-db/src/transaction/audit_reads.rs` line 174 |
| Path value | `type_name` parses the path with `TypeName::parse` and gives `404 E-WEB-005` | `crates/recipe-web/src/pages/path_values.rs` lines 81 to 82 |
| New struct form | `new_name` refuses `new` and calls `TypeName::parse`. A failure is `E-WEB-008` with `FieldReason::TypeName` | `crates/recipe-web/src/pages/types/define.rs` lines 67 to 74 |
| Member types of a struct form | `TypeExpr::parse` of each member type | `define.rs` lines 100, 210 |
| Parameter type texts | `definition::check` parses the type of the add form and of the edit form. `fields` parses the sent type for the grid. `finish` parses `value_type`. The table parses the type text to find `BOOL` | `crates/recipe-web/src/pages/parameters/definition.rs` line 140, `prepare.rs` line 83, `fields.rs` line 407, `crates/recipe-web/src/pages/versions/parameter_table.rs` line 183 |
| Type resolution | `resolve_type` takes the class-2 locks and resolves the type for `AddParameter` and `UpdateParameterDefinition` | `crates/recipe-app/src/recipe/definition_steps.rs` line 65, `add_parameter.rs` line 115, `update_definition.rs` line 161 |
| Type change | `set_definition` compares the new type id with the stored type id | `crates/recipe-domain/src/recipe/draft/definition.rs` line 83 |
| Texts | `reason-type-name` names `new` and the primitive names. `type-reason-*` holds the reasons | `crates/recipe-web/i18n/en.ftl` and `pl.ftl` lines 296 and 396 to 400 |
| Revision lookup | `next_revision` compares `*name == input.name` for both modes | `crates/recipe-app/src/plant/define_udt.rs` lines 225 to 260 |
| Duplicate mapping | `insert_struct_row` maps `PLC_TYPE_NAMED_UNIQUE` and `PLC_TYPE_MEMBER_NAME_UNIQUE` | `define_udt.rs` lines 178 to 200, `crates/recipe-app/src/error/mapping.rs` lines 21, 23 |
| Memory adapter | `insert_struct` compares the name in exact case and returns `PLC_TYPE_NAMED_UNIQUE`. The fault injection knows the two old names | `crates/recipe-app/src/memory/types/mod.rs` lines 150 to 172, `crates/recipe-app/src/memory/fault.rs` lines 68, 74 |
| Lock keys | `derived_type_locks` sorts by the `Ord` of `DerivedTypeLock` and removes duplicates | `crates/recipe-app/src/plant/derived_lock.rs` lines 18 to 33 |
| Lock key type | `DerivedTypeLock {Struct(TypeName), String, Wstring, Array}`. The doc comment names the fixed order | `crates/recipe-app/src/ports/types.rs` lines 34 to 58 |
| Pins of the old order | `struct_definition_adds_struct_key_before_string_and_array`, `keys_are_sorted_not_in_input_order` | `crates/recipe-app/src/plant/derived_lock/tests.rs` lines 45, 59 |
| Memory locks | `lock_derived_types` records one entry per key in the given order | `memory/types/mod.rs` line 103, `memory/types/tests.rs` line 386 |
| Postgres locks | One statement per key in the given order | `type_rows.rs` lines 25, 41, 233 to 247 |
| Pin of the given order | `postgres_only_the_keys_are_taken_in_the_given_order` | `crates/recipe-db/tests/storage/contract/types/advisory.rs` line 129 |
| Statements for tests | The module `statements` exports SQL texts under the feature `test-support` | `crates/recipe-db/src/lib.rs` lines 38 to 43 |
| `dims_valid` | No check of the start subscript | `migrations/0001_schema.sql` line 68, `data-model.md` §3.3 |
| `dims_valid` tests | Positive and negative cases with arrays at the subscript 1 | `crates/recipe-db/tests/storage/constraints/types.rs` line 416. The file has 701 lines |
| Document tests | `as_shipped_in_0001` undoes the pieces of ADR-0109 before it compares `data-model.md` §3 with `0001`. `every_name_of_section_3_exists_in_the_catalog` finds each name of §3 in the catalog. `every_named_constraint_and_unique_index_of_section_3_is_covered_or_exempt` needs each unique index of §3 in a `COVERED` list | `crates/recipe-db/tests/storage/support/document.rs` lines 87 to 133, `schema.rs` lines 17, 52, `constraints/mod.rs` lines 85 to 144 |
| Restore code | The line of `dims_valid` holds the hash of its definition | `deploy/pgtools/restore-code.expected` line 79 |
| Migration count | `EXPECTED_MIGRATIONS` is 10 | `crates/recipe-server/tests/migrate.rs` lines 27 to 28 |
| Rewrite check | `migration_rewrite.py` rejects a `DO` block, an index without `CONCURRENTLY`, and a `LOCK TABLE` statement. It reads function bodies. `EXEMPT` keys on the pair of file and form. Migration `0011` has two entries, `index without concurrently` and `lock table` | `deploy/ci/migration_rewrite.py` lines 1 to 35, 464 to 472, 588 to 598 |
| Rewrite tests | `test_shipped_migrations_pass` runs the check on the whole directory `migrations/` | `deploy/ci/test_migration_rewrite.py` lines 671 to 677 |
| Temporary objects | `01-roles.sh` revokes `TEMPORARY` from `PUBLIC`. `recipe_migrator` owns the database | `deploy/db/initdb/01-roles.sh` lines 92 to 93 |

The fix plan names `type_name.rs:89`, `plc_type.rs:246`, and `type_rows.rs:41`. The author found `TypeName::parse` at line 89, `seen.insert` at line 247, and `LOCK_DERIVED_TYPE` at line 41.

## Goals / Non-Goals

Goals:

- New struct names and member lists follow ADR-0143. The database enforces the compare without case.
- Every stored struct stays readable. A draft that uses it stays editable while the type of the parameter does not change.
- A type text that a command writes follows the current rule (ADR-0143 (i2)).
- The class-2 locks follow ADR-0144 in the PostgreSQL adapter and in the memory adapter.
- `dims_valid` follows ADR-0145.
- The upgrade stops before a schema change when the stored rows break a new rule.
- Each pin shows a red step on a mutation of D11.
- `cargo test --workspace` passes after each task.

Non-Goals:

- A keyword rule for member names, a rename of a stored struct, and the keywords of Siemens SCL.
- A new error code, a new setting, or a new grant.
- A change of the flag `loadable` for a version whose type names a struct with a reserved name (see "Open Questions").

## Decisions

### D1: Two constructors of `TypeName`

`TypeName::parse(text)` applies the current rule of ADR-0143 (c). `TypeName::from_stored(text, rules: RuleSet)` applies the current rule for `RuleSet::Current` and the earlier rule for `RuleSet::Earlier`. The earlier rule is the grammar and no primitive name in exact case. This form follows `ParameterValue::from_stored` of ADR-0120.

`type_name.rs` gets `pub const RESERVED_KEYWORDS: [&str; 182]` in the order of ADR-0143 (d) and `pub fn is_reserved_name(text: &str) -> bool`. The function compares with `eq_ignore_ascii_case` against the keywords and against `Primitive::ALL`. `TypeNameReason::Reserved` gets the text "a type name cannot be an IEC keyword or a primitive type name". `MemberName` does not change. Alternative: a second type `StoredTypeName`. The change rejects it, because every consumer of a struct name would need two forms.

### D2: Two constructors of `PlcType`

`PlcType::try_from(record)` applies the current rules. `PlcType::from_stored(record, rules: RuleSet)` applies the earlier rule for `RuleSet::Earlier`. Only the check of the struct name differs between the sets:

- Both sets: after the member count, the member names compare with `to_ascii_lowercase`. The second name of a pair is the `member` of `E-PLT-021`. The indexes of ADR-0143 (j) hold for every stored row, so the earlier set needs no exact compare.
- Current: a reserved struct name gives `E-PLT-013` with `type_expr` `{name}@{revision}` and the new reason `DerivedTypeReason::ReservedName`.
- Earlier: no check of the struct name against the reserved words.

The rule of the name is for a struct row only. A primitive row keeps its name `INT` or `STRING`. `ReservedName` gets the `Display` text "the struct name is a reserved IEC 61131-3 word or a primitive name" and the web key `type-reason-reserved-name`. The `en` text of the key is the same text. The `pl` text is "nazwa struktury jest zastrzeżonym słowem IEC 61131-3 albo nazwą typu prostego".

### D3: The paths that use the earlier rule

These paths read a stored name or a stored type, so they use `RuleSet::Earlier` (ADR-0143 (i1)):

- `recipe-db`: `shape_of` and `build_type` of `type_rows.rs`, and the label of `audit_reads.rs` line 174. `ensure_derived` (line 382) checks a new row and keeps `PlcType::try_from`.
- `recipe-web`: `path_values::type_name`. It serves the type page, the usage page, the form of a new revision, and the POST of a revision. The domain then refuses a new revision of a reserved name (D2).
- `recipe-web`: the four parse sites of a parameter type text in the table "State of the code". They use `TypeExpr::from_stored(_, RuleSet::Earlier)`, because the edit form sends the stored type text again.
- Test fixtures that build a stored struct with a reserved name use `PlcType::from_stored`.

The form of a new struct keeps `TypeName::parse`, so it gives `E-WEB-008`. `reason-type-name` gets the `en` text "a type name has 1 to 64 letters, digits, and underscores and starts with a letter or an underscore. It is not new, an IEC keyword, or a primitive name, in any case". The `pl` text is "nazwa typu ma od 1 do 64 liter, cyfr i podkreśleń i zaczyna się literą lub podkreśleniem. Nie może to być new, słowo kluczowe IEC ani nazwa typu prostego, bez względu na wielkość liter".

### D3a: The rule of a type text that a command writes

`parse_udt` of `TypeExpr` takes a `RuleSet` (ADR-0143 (i2)). `TypeExpr::parse(text)` applies `RuleSet::Current`. `TypeExpr::from_stored(text, rules)` passes the rule set on.

- Current: a reserved struct name in any case gives `E-PLT-013` with `ReservedName`. A primitive name in exact case keeps `UnknownStruct`, as today.
- Earlier: only a primitive name in exact case fails, with `UnknownStruct`. Thus `Int@1` names the stored struct `Int@1`.

The member types of the struct form keep `TypeExpr::parse`. Thus `DefineUdt` gets no member of type `Int@1`.

`definition_steps.rs` gets `require_current_type(expr: &TypeExpr) -> Result<(), PlantError>`. It runs `TypeExpr::parse` on the text of `expr`. This follows ADR-0120 (c): a command checks the part that it writes with the current set.

- `AddParameter` calls it before `OpenDraft::open`, so the check runs before the first lock (`reliability.md` §7.4).
- `UpdateParameterDefinition` calls it after `resolve_type`, when the resolved type id differs from the stored type id of the parameter. An unchanged type keeps `Int@1`.

Alternative: the web parses each form with the current rule. The edit form then refuses its own stored type, so the change rejects it.

### D4: Lookup, new struct, and duplicate mapping

The lookup of a struct stays exact (ADR-0143 (g)). `next_revision` compares without case for `DefineUdtMode::New`, so `st_zone` next to `ST_Zone` gives `E-PLT-015`. The error names the stored spelling `ST_Zone`. `DefineUdtMode::Revision` keeps the exact compare, so `st_zone` finds no revision of `ST_Zone` and gives the existing `NotFound`.

`mapping.rs` gets `PLC_TYPE_STRUCT_NAME_CI_UNIQUE` and `PLC_TYPE_MEMBER_NAME_CI_UNIQUE`. `insert_struct_row` maps them as the two old names. The mapped error names the spelling of the input, because the adapter does not read the stored row. The memory adapter compares the struct name without case in `insert_struct` and returns `PLC_TYPE_STRUCT_NAME_CI_UNIQUE` for a match in another case. `memory/fault.rs` can inject both new names.

### D5: Order of the class-2 locks

`derived_type_locks` and `rebind_locks` sort the keys by `key_text` and remove duplicate texts. Thus a struct `ARRAY` and the key `ARRAY` give one text. The order of the texts is not the order of the locks. The doc comment of `DerivedTypeLock` names ADR-0144.

`lock_derived` of `type_rows.rs` runs the statement `DERIVED_LOCK_KEYS` first:

```sql
SELECT DISTINCT hashtext($1::text || ':' || k) AS key
FROM unnest($2::text[]) AS k
ORDER BY key
```

Then it runs `SELECT pg_advisory_xact_lock($1, $2)` with the class 2 and each key value, in that order. `recipe_db::statements` exports `DERIVED_LOCK_KEYS` under the feature `test-support`, so a storage test can run it. The memory adapter records the texts in their sorted order. Alternative: compute `hashtext` in Rust. ADR-0144 rejects it.

### D6: Migration `0011_plc_type_rules.sql`

The file runs in one transaction, in this order:

1. `CREATE FUNCTION plc_type_rule_check() RETURNS void LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp`. The body counts three kinds of rows over every machine, also an archived one, and every revision:
   - array rows where `array_lower(dim_lower, 1) <> 1` or `array_lower(dim_upper, 1) <> 1`
   - groups of struct rows with one `(machine_id, lower(name), revision)` and more than one `name`
   - groups of member rows with one `(struct_type_id, lower(member_name))` and more than one row
2. When a count is not 0, the body raises one exception with this text: `migration 0011 stopped: <a> array rows start at another subscript than 1, <s> groups of struct names and <m> groups of member names differ only in case. Run deploy/db/check-identifier-rules.sql (docs/ops/runbook.md).`
3. `SELECT plc_type_rule_check();`
4. `DROP FUNCTION plc_type_rule_check();`
5. `CREATE OR REPLACE FUNCTION dims_valid` with the text of `data-model.md` §3.3.
6. `CREATE UNIQUE INDEX plc_type_struct_name_ci_unique` with the text of `data-model.md` §3.3.
7. `CREATE UNIQUE INDEX plc_type_member_name_ci_unique` with the text of `data-model.md` §3.4.

An exception or a lock timeout rolls back the whole file, so `dims_valid`, the indexes, and the history table do not change. The old `app` keeps the old schema (`reliability.md` §7.10 step 3). On `55P03` the retry loop of ADR-0031 runs the file again.

The function lives in `public` and not in `pg_temp`. The function exists only inside the transaction of the file, so `restore-code.expected` and `grants.expected` do not see it. Alternative: a `DO` block. `migration_rewrite.py` rejects it. Alternative: a function in `pg_temp`. It depends on the implicit `TEMPORARY` right of the database owner, which `01-roles.sh` does not state.

### D7: The indexes and the rewrite check

Steps 6 and 7 do not use `CONCURRENTLY` (ADR-0143 (k)). The file first takes a `SHARE` lock on `plc_type` and on `plc_type_member`, and it holds the lock to the end of the file. Thus the old `app` cannot store a case duplicate between the precheck and the indexes. The lock is short, because both tables are small.

`migration_rewrite.py` reports the form `index without concurrently` for steps 6 and 7, and the form `lock table` for the `LOCK TABLE` statement. `EXEMPT` gets two entries: `("0011_plc_type_rules.sql", "index without concurrently"): "ADR-0143"` and `("0011_plc_type_rules.sql", "lock table"): "ADR-0143"`. One entry per form is enough, because `EXEMPT` keys on the pair of file and form (ADR-0136, Risks). Each entry comes in the same task as the statement it permits, because the check reports a dead entry for a file without the form.

`CREATE FUNCTION`, `CREATE OR REPLACE FUNCTION`, `SELECT`, `IF`, and `RAISE` are no rejected form. The owner of `plc_type` and `plc_type_member` is `recipe_migrator`, so the migrator can build both indexes. The two indexes give `recipe_app` no right, so `grants.expected` does not change. `restore-code.expected` holds no index lines.

### D8: No recovery of an INVALID index

The change has no `-- no-transaction` file. Thus no run leaves an INVALID index or an index without a history row. The runbook gets no recovery section for this change.

### D9: The ops query

`deploy/db/check-identifier-rules.sql` runs in one `READ ONLY` transaction and writes nothing. It has the header of `deploy/db/check-platform-ranges.sql`. It lists one row for each affected stored row with the columns `machine_code`, `type_name`, `revision`, `part`, and `stored_text`. Thus a case pair gives two rows, one for each spelling. `part` is one of these values:

- `struct_name_case`: a struct row whose `(machine_id, lower(name), revision)` has another spelling
- `member_name_case`: a member row whose `(struct_type_id, lower(member_name))` has another row
- `array_subscript`: an array row with a bound array that starts at another subscript than 1
- `reserved_struct_name`: a struct row whose name is a reserved word of ADR-0143 (c), in any case

The list of reserved words sits between the lines `-- reserved words: begin` and `-- reserved words: end`, one word on each line. A test of `recipe-db` compares that list with `RESERVED_KEYWORDS` and `Primitive::ALL`. The first three parts stop migration `0011`. The part `reserved_struct_name` is for information: such a struct gets no new revision.

### D10: The document tests

Each task keeps the SQL blocks of `data-model.md` §3.3 and §3.4 equal to the migrations that exist at its end. Thus the three document tests of the table "State of the code" pass after each task. A task adds a line to §3.3 or §3.4 together with its migration piece:

| Task | Line in `data-model.md` |
|---|---|
| 1.1 | In `dims_valid` of §3.3, after `AND cardinality(lower_b) BETWEEN 1 AND 3`: the comment `-- Both arrays start at the subscript 1 (ADR-0145).` and `AND array_lower(lower_b, 1) = 1 AND array_lower(upper_b, 1) = 1` |
| 1.3 | In §3.3, after `plc_type_array_unique`: the comment `-- A struct name compares without case on one machine (ADR-0143). Migration 0011 builds it.` and `CREATE UNIQUE INDEX plc_type_struct_name_ci_unique ON plc_type (machine_id, lower(name), revision) WHERE kind = 'struct';` |
| 1.4 | In §3.4, after `plc_type_member_member_type_id_idx`: the comment `-- A member name compares without case in one struct (ADR-0143). Migration 0011 builds it.` and `CREATE UNIQUE INDEX plc_type_member_name_ci_unique ON plc_type_member (struct_type_id, lower(member_name));` |

`support/document.rs` renames `Adr0109Piece` to `ShippedPiece` and `ADR_0109_PIECES` to `PIECES_AFTER_0001` in task 1.1. Each task adds its line to that list, with a name and an empty shipped text. Each task also adds a test to `tests/storage/migration_0011.rs` that compares its piece of `0011` with the document. The statements of `0011` equal the document.

The index tests go into a new module `constraints/type_rules.rs` with its own `COVERED` list, because `constraints/types.rs` has 701 lines. `constraints/mod.rs` adds the list to both `concat` lists.

### D11: The red steps

Each pinning test shows its red step on one temporary mutation:

- M1a: `dims_valid` in `0011` without `array_lower(lower_b, 1) = 1`. M1b: without `array_lower(upper_b, 1) = 1`.
- M2: `0011` builds the struct index on `name` instead of `lower(name)`. M3: the same for `member_name`.
- M4: `is_reserved_name` compares with `==` instead of `eq_ignore_ascii_case`.
- M5: `RESERVED_KEYWORDS` holds `END_STRUCTX` in place of `END_STRUCT`. The array keeps 182 words, so the crate compiles.
- M6: `check_struct` compares the member names in exact case.
- M7: `RuleSet::Earlier` runs the current checks in one of these places: `TypeName::from_stored`, `PlcType::from_stored`, `TypeExpr::from_stored`, or a caller of D3.
- M8: `derived_type_locks` sorts by the `Ord` of `DerivedTypeLock` again.
- M9: `lock_derived` takes the keys in the given order, without the statement of D5.
- M10a: the precheck of `0011` returns before the `RAISE`. M10b: the precheck counts the array rows without the condition on `dim_upper`.
- M11: `next_revision` compares in exact case for `DefineUdtMode::New`.
- M12: `insert_struct_row` does not map the two new constraint names.
- M13: `DERIVED_LOCK_KEYS` without `DISTINCT`.
- M14: `TypeExpr::parse` passes `RuleSet::Earlier` to `parse_udt`.
- M15: `require_current_type` returns `Ok(())` at once.
- M16: the reserved list of `check-identifier-rules.sql` without the line `END_STRUCT`.
- M17: `EXEMPT` without the entry `index without concurrently` of `0011`. `migration_rewrite.py migrations` then exits with 1 and prints `0011_plc_type_rules.sql:<line>: index without concurrently`.
- M17b: `EXEMPT` without the entry `lock table` of `0011`. `migration_rewrite.py migrations` then exits with 1 and prints `0011_plc_type_rules.sql:9: lock table`.

Before a mutation, the agent copies the file to the scratch directory. After the red run, it reverts the mutation and runs `cmp <file> <copy>`. A wrong expected value in a test is not a red step.

### D12: The tests stay green

The plan review found that the propose step made three `recipe-db` tests fail. The update of 2026-11-07 removed the SQL lines of §3.3 and §3.4 again. Thus the tree passes `cargo test --workspace` before task 1.1. D10 gives each line back to its task.

The prose note under §3.4 and the lines of `data-model.md` §6 and §8 name the indexes already. No test reads them. No test reads `domain-model.md`, `reliability.md` §4.3, §5, or the ADRs. `reliability.md` §1.2 does not change.

## Risks / Trade-offs

- Risk: production holds a case duplicate or an array row with another subscript. Then `0011` stops the upgrade. Mitigation: task 5.3 runs the query of D9 on production before the merge. A finding in the first three parts stops the merge and needs a new decision of the owner.
- Risk: a transaction of the old `app` holds a lock on `plc_type` past `migrate.lock_timeout`. Mitigation: the file rolls back, and the retry of ADR-0031 runs it again, up to `migrate.retries` times.
- Risk: the IEC list comes from a public reproduction of the standard. Mitigation: ADR-0143 (d) names the source, and a new ADR can add words.
- Trade-off: a stored parameter of type `Int@1` keeps it, and its export still holds the struct `Int`. The change accepts this, because the draft of an existing parameter must stay editable.
- Trade-off: a stored struct with a reserved name gets no new revision (ADR-0143, Negative).

## Migration Plan

1. The owner runs the query of D9 on production before the merge (task 5.3). The merge needs 0 rows in the parts `struct_name_case`, `member_name_case`, and `array_subscript`. Each row of `reserved_struct_name` needs the sign-off of the owner.
2. The upgrade runs the same query in a sub-step of step 2 of "Upgrade" (task 5.2).
3. `migrate` applies `0011`. A failure of `0011` changes nothing.
4. Rollback: the indexes and the new `dims_valid` accept every row of the old `app`, except a case duplicate. Thus the old image runs on the new schema (`reliability.md` §7.10 step 6). The old `app` does not know the names of the new indexes, so it reports such a refusal without the mapping of D4.

## Open Questions

- The owner decides whether `loadable` becomes `false` for a version whose type names a struct with a reserved name (ADR-0119). This change keeps `loadable` as it is.
- The owner decides whether to check the IEC list against the licensed text of IEC 61131-3 Ed. 3. The list of ADR-0143 (d) uses a public reproduction.
