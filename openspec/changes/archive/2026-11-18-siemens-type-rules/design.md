# Design

## Context

See proposal.md, section Why. The review is `docs/plan/review/2026-11-17-siemens-type-coverage-review.md`. Its source file is `docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md`. The review and the source file are not normative.

The change uses three existing mechanisms:

- Type rules without a branch of `RuleSet`. A command checks only the type text that it writes (ADR-0143 (i2), ADR-0148, ADR-0166 (t)).
- Address rules of one parameter. They run in `check_definition`, in the rebind apply, at approval, and in the export check (ADR-0116, ADR-0119 (b)).
- Value rules with the earlier rule set for stored values (ADR-0120, ADR-0166).

## Goals / Non-Goals

**Goals:**

- Put each new rule into the mechanism that matches its kind. Add no new mechanism.
- Keep every stored row readable. Keep every draft editable (ADR-0028, ADR-0120).
- Give each new rule an ops query part, so an operator can list affected rows before an upgrade.

**Non-Goals:**

- No migration and no new trigger. The rules live in `recipe-domain`.
- No change of the export hash, the writer, or the v1 schema file.

## Decisions

Each decision has its ADR. This section gives only the placement of each rule.

| Decision | ADR | Kind | Code | Stored rows |
|---|---|---|---|---|
| OQ01 | ADR-0177 | type rule | `E-PLT-013`, `E-PLT-006` | readable, approvable, exportable |
| OQ02 | ADR-0178 | address rule | `E-RCP-058` | draft editable, approval refused, export `loadable` false |
| OQ03 | ADR-0179 | type rule, also on a rebind | `E-PLT-018` with the kind `S7Nesting` | readable, approvable, exportable |
| OQ04 | ADR-0180 | value rule with a branch | `E-RCP-005`, item of `E-RCP-020` | earlier set keeps ADR-0137, approval refused, export `loadable` false |
| OQ11, R04 | ADR-0186, ADR-0185 | type rule | `E-PLT-013` "reserved name" | readable, kept by rebind, left out of proposals |
| OQ12 | ADR-0187 | address rule | `E-RCP-058` | as OQ02 |
| R01 | ADR-0189 | schema | none | v1 unchanged |

Placement in the code:

- OQ01 widens the platform test of the existing array-of-array check in `TypeExpr` from `S7_1200` and `S7_1500` to the three S7 platforms. The callers do not change.
- OQ02 and OQ12 are one new function `check_standard_access_rule(tag, address, tree)` in `draft/s7.rs`. It walks the type tree once and does nothing for a parameter without an address. Four places call `check_end_rule` today, and each place calls the new function directly after it:
  - `check_definition` in `draft/s7.rs` (`AddParameter`, `UpdateParameterDefinition`)
  - `check_layout` with `AddressRules::Run` in `draft/s7.rs` (`ApproveVersion`)
  - the rebind apply in `rebind/apply.rs` (`RebindTypeRevision`)
  - `address_faults` in `read_check/mod.rs` (export check and version page check)
- `E-RCP-058` gets one variant `RecipeError::S7TypeNotAddressable { tag, address, reason }`. `reason` is a closed enum with two values, so the bundles hold one text for each value. A free text would break the rule of `web.md` that UI text comes from the bundles. The form line stands under `s7_address`, and the approval preview and the blocker list link the tag (ADR-0178 (k)).
- OQ03 adds `MAX_S7_NESTING = 8` to `plant/limits.rs` and a count of the S7 levels to the type tree metrics. `DefineUdt`, `AddParameter`, and `UpdateParameterDefinition` call the check where they call the array-of-array check. `RebindTypeRevision` calls it on the resulting tree of each moved parameter (ADR-0179 (f)). `TypeLimit` gets the kind `S7Nesting`, and `recipe-web` maps it to the key `type-limit-s7-nesting` (ADR-0179 (l)).
- OQ04 changes the leaf check of `WCHAR` and `WSTRING` in `recipe/value/leaf.rs`. The leaf type already holds the platform and the rule set. A new `Expected` variant carries the length and the highest code unit of an S7 `WSTRING`.
- OQ11 adds one module with the 269 names and the family `PD_TEL<n>` of ADR-0186. The struct name check and the type proposals call one function `is_siemens_reserved_name`. The ops query holds the second copy, and a `recipe-db` test compares both copies.
- R01 adds a `pattern` to `export-v2.schema.json`. The schema tests also run each form against `S7Address::parse`, so the two grammars cannot drift.

The pattern of R01, in one line of the schema file:

```text
^DB(?:[1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])\.(?:DBX(?:[0-9]{1,4}|[0-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])\.[0-7]|DB[BWD](?:[0-9]{1,4}|[0-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))$
```

OQ05, OQ06, OQ07, OQ09, OQ10, R04, and R06 change no code. Their ADRs and the notes of `api.md` §6.1, `data-model.md` §7, and `overview.md` §9 record them.

OQ08 needs no ADR. The owner keeps the 7 significant digits of `REAL` (`domain-model.md` §2.1).

R01 result: the policy permits a change of v2. ADR-0019 forbids only a breaking change. ADR-0119 (g) changed both schema files without a new version. The pattern refuses no document that the writer can emit.

## Stored data matrix

Each rule narrows what a command accepts. A stored row of the earlier release stays readable (ADR-0120). Tasks 9.1 and 9.2 test each row of this table on PostgreSQL. Task 9.1 writes the rows with the code before the rules, so the stored hash comes from the earlier release. Task 9.1 keeps a full dump of that database, with `_sqlx_migrations`. Task 9.2 loads the dump into an empty database and then runs the migrator of this release. Thus the test also covers the upgrade path.

| Decision | Stored row of an approved version | Read | `loadable` | `content_sha256` | `CreateVersion` |
|---|---|---|---|---|---|
| OQ01, ADR-0177 | `ARRAY[1..2] OF ARRAY[1..3] OF BYTE` at `DB1.DBB0` on `S7_300` | readable | true | stored hash | copies |
| OQ02, ADR-0178 | `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0` on `S7_300` | readable | false | stored hash | copies |
| OQ03, ADR-0179 | `ST_Top@1` with nine levels on `S7_1500` | readable | true | stored hash | copies |
| OQ04, ADR-0180 | `WCHAR` with the value `"\uE000"` on `S7_1500` | readable | false | stored hash | copies |
| OQ04, ADR-0180 | `WSTRING(10)` with the value `"A\uD000"` on `S7_1200` | readable | false | stored hash | copies |
| OQ11, ADR-0186 | `PID_Compact@1` on `S7_1500` | readable | true | stored hash | copies |
| OQ12, ADR-0187 | `ARRAY[1..2,1..2,1..2] OF BOOL` at `DB1.DBB0` on `S7_300` | readable | false | stored hash | copies |

Each row also passes a backup and a restore: `pg_dump --format=custom`, then `pg_restore` into an empty database. The export after the restore gives the same document.

## Risks / Trade-offs

- [The count of ADR-0179 can differ from TIA Portal] → Open questions 1 and 2. A later ADR can change the count. The ops query lists the affected rows.
- [The SCL keyword list of ADR-0186 (d2) can miss a word] → Open question 3. The list lives in one module, so an addition is one line and one test.
- [An `Approved` version on a production server becomes not loadable] → The ops queries of ADR-0178, ADR-0180, and ADR-0187 list the rows before the upgrade. No production server exists on 2026-11-17.
- [The two copies of the reserved names drift] → A `recipe-db` test compares them.
- [The triggers of the schema refuse a row of an `Approved` version] → The fixture of task 9.1 is a full plain dump. It creates the triggers after it loads the data, so no trigger runs on a stored row.
- [A later migration changes the schema of the fixture] → Task 9.2 runs the migrator after the load. Thus a later migration runs on the fixture as on a production upgrade. A migration that fails on the fixture fails the test.

## Migration Plan

1. Before the upgrade, run `check-identifier-rules.sql`, `check-s7-layout-rules.sql`, and `check-platform-ranges.sql` against the production database (`docs/ops/runbook.md`).
2. Record each listed row in the upgrade ticket. The new parts of this change give two kinds of result:
   - Refused only for new writes: `nested_array` on `S7_300` (OQ01, ADR-0177), `nesting` (OQ03, ADR-0179), and `s7_reserved_struct_name` (OQ11, ADR-0186). A listed `Approved` version stays loadable after the upgrade.
   - Violations that change `loadable`: `array_bound` (OQ02, ADR-0178), `wide_text` (OQ04, ADR-0180), and `bool_array_3d` (OQ12, ADR-0187). A listed `Approved` version becomes not loadable after the upgrade.
3. Deploy the release. No migration runs for this change.
4. Rollback: deploy the previous image. The change writes no new data form, so the old release reads every row.

## Open Questions

1. Does TIA Portal V21 count the data block as one of the 8 nesting levels (ADR-0179)? The answer changes only the limit value or the start of the count.
2. Does an array of a primitive count a level in TIA Portal V21 (ADR-0179 (b))?
3. Does the list of ADR-0186 (d2) equal the S7-SCL keyword list of TIA Portal V21?
4. When does a TIA Portal measurement give the layout of row A03 (ADR-0187 (h))?
5. Which firmware of the classic S7-1200 first accepts `LREAL` with an address (ADR-0182)?
