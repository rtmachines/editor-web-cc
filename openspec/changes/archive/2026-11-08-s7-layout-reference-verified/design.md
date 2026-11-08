# Design

## Context

See `proposal.md`, section "Why". ADR-0147, ADR-0148, and ADR-0149 hold the rules. ADR-0017, ADR-0116, ADR-0119, ADR-0120, and ADR-0143 hold the context. This design links to them and does not repeat them.

The research record `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` gives the rule ids S-ST1 to S-BOOL and the verdict of each row. Its section "Rows" is the source of each new mark.

The state of the code on the branch `fix/s7-layout-reference-verified`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Start class of a leaf | `leaf_start` gives `Byte` to each leaf of 1 byte, also to `SINT` and `USINT` | `crates/recipe-domain/src/plant/s7_layout.rs` lines 232 to 239 |
| Struct layout | `Frame::finish` gives the child layouts to `struct_layout`. `place` aligns each member by its start class | `s7_layout.rs` lines 212 to 228, 242 to 254, 268 to 283 |
| Member offsets | `members_of_root` calls `layout_of_node` and `place` for each member of the type page | `s7_layout.rs` lines 130 to 167 |
| Array layout | `array_layout` gets the element count and no dimensions. `BOOL` elements pack 8 per byte over all dimensions | `s7_layout.rs` lines 222 to 225, 286 to 299 |
| Dimensions | `ArrayDimensions` has `iter`, `len`, and `element_count`. `ArrayDimension` has `count` | `crates/recipe-domain/src/plant/array_dimension.rs` lines 83, 112, 118, 133 |
| Source marks | `Source` has `Siemens` and `Unverified`. The doc comment of `Siemens` names a manual and a page | `crates/recipe-domain/src/plant/s7_layout/reference.rs` lines 14 to 23 |
| Composite rows | `composite` sets both marks to `Unverified("Q10")`. C04 has size 4. A02 has size 2. A03 has size 4 | `reference.rs` lines 63, 243 to 258, 287 to 292, 346, 347 |
| Rows of an array of an array | A09 and A10, with the catalog rows 212, 213, 409, and 410 | `reference.rs` lines 132, 133, 237, 238, 357, 360 |
| Pin of the marks | `reference_table_marks_every_composite_row_unverified` | `reference.rs` lines 397 to 411 |
| Random struct test | `first_fit` and `reference_width` use the start class of the row. P06 and P07 are `Byte` | `crates/recipe-domain/src/plant/s7_layout/prop.rs` lines 89 to 109, 129 to 160 |
| Address rules | The alignment check, the end rule, and the overlap check use `S7Layout` | `crates/recipe-domain/src/recipe/draft/s7.rs` lines 81 to 106, 130 to 154, 224, 235 |
| Commit path of the parameter commands | `replace_parameters` runs `check_s7_layout` on every pair of the changed draft. `SetParameterValue`, `ReplaceLimits`, `AddParameter`, `UpdateParameterDefinition`, and the rebind use it | `crates/recipe-domain/src/recipe/draft/replace.rs` line 28, `draft/value.rs` line 81, `draft/add.rs` line 71, `draft/definition.rs` line 94, `recipe/rebind/apply.rs` line 137 |
| Overlap search | `occupy` gives the occupied bits of one parameter. `find_overlap` returns the first overlap of the list | `draft/s7.rs` lines 235, 270 |
| Export check | `check_version_parameters` calls `check_stored_parameter` for each parameter. `address_faults` runs only the data block rule and the end rule | `crates/recipe-domain/src/export/rule_check.rs` lines 82 to 103, `crates/recipe-domain/src/recipe/read_check/mod.rs` lines 273 to 295 |
| Callers of the export check | The export and the version page query | `crates/recipe-app/src/export/document.rs` line 234, `crates/recipe-app/src/recipe/queries/versions.rs` line 325 |
| Array levels of a type text | `TypeExpr::arrays` is a list of array prefixes. Two prefixes are an array of an array | `crates/recipe-domain/src/plant/type_expr.rs` lines 127 to 152 |
| Reason `NotOnPlatform` | `DerivedTypeReason::NotOnPlatform` with the text "the type does not exist on the platform of the machine" | `crates/recipe-domain/src/plant/error.rs` lines 40 to 75, 215 to 220 |
| Current rule of a written type | `require_current_type` parses the text again. It knows no platform | `crates/recipe-app/src/plant/current_type.rs` line 16 |
| `AddParameter` | `require_current_type` runs before the draft opens. `resolve_type` runs after it. `open.draft.platform()` gives the platform | `crates/recipe-app/src/recipe/add_parameter.rs` lines 109, 115, 117, `definition_steps.rs` line 74 |
| `UpdateParameterDefinition` | `require_current_type` runs only when the resolved type id differs from the stored type id | `crates/recipe-app/src/recipe/update_definition.rs` lines 165 to 168 |
| `DefineUdt` | `require_current_type` runs before the command reads the platform. `ensure_members` maps `NotOnPlatform` to `E-PLT-006` through `member_error` | `crates/recipe-app/src/plant/define_udt.rs` lines 100 to 102, 141 to 159, 281 to 295 |
| Derived rows | `ensure_expr`, `ensure_shape`, and `wrap_arrays` build the array rows. Rebind calls `ensure_shape` | `crates/recipe-app/src/plant/ensure_derived.rs` lines 98 to 148, `crates/recipe-app/src/recipe/rebind/new_types.rs` line 48 |
| Preview | The preview copies the rules of `ensure_derived.rs` and checks the platform of each base | `crates/recipe-app/src/recipe/queries/preview.rs` lines 1 to 8, 61 to 90 |
| Callers of the layout | `draft/s7.rs` and the type page only | `crates/recipe-web/src/pages/types/tree.rs` lines 210 to 211 |
| Stored nested array on the type page | A test seeds `ARRAY[0..1,1..3] OF ARRAY[1..8] OF INT` and reads the tree | `crates/recipe-web/src/pages/types/tree_tests.rs` lines 245 to 262 |

## Goals / Non-Goals

**Goals:**

- The S7 layout of a struct with a `SINT` or `USINT` member and of a `BOOL` array with two dimensions follows the cited Siemens rules (ADR-0147).
- A command that writes a type text on `S7_1200` or `S7_1500` refuses an array of an array (ADR-0148).
- Each composite row of the reference table names its Siemens rules.
- The owner can list the stored rows that the change affects before an upgrade.
- A stored overlap after the layout change does not lock a draft. An `Approved` version with an overlap is not loadable (ADR-0149).

**Non-Goals:**

- A TIA Portal measurement.
- A new error code, a new reason of `E-PLT-013`, or a new Fluent text.
- A migration, a trigger, or a `CHECK` constraint.
- The marks of the primitive rows P01 to P30.
- A change of the start of a `SINT` or `USINT` array element or parameter.

## Decisions

### D1: The struct member rule raises `SINT` and `USINT` to `Even`

`leaf_start` stays as it is. A leaf of `SINT` or `USINT` keeps the start class `Byte` as a value, as an array element, and as a parameter (ADR-0147 (c)).

A new free function in `s7_layout.rs` gives the start class of a struct member. It gives `Even` to a leaf node of `SINT` or `USINT`. It gives the start class of the layout to every other node.

Two places use the function:

1. `Frame::finish` for a struct node. It pairs each member node with its layout in `done` and places the pair.
2. `members_of_root` before `place`. The offset of the type page then equals the size rule.

`S7Member::layout()` keeps the layout of the type, so a `SINT` member keeps the start class `Byte`. Only `place` uses the start class of the member.

Alternative: change `leaf_start` to `Even` for `SINT` and `USINT`. The author rejected it. It also moves an array element and a parameter, and no page gives a rule for them.

The random struct test of `prop.rs` places the reference rows as struct members. `first_fit` and `reference_width` get the same rule for the rows P06 and P07.

### D2: A `BOOL` array places its lines at the next byte

`array_layout` gets the dimensions in place of the element count. For a `BOOL` element, the size has these steps:

1. Let `last` be the count of the last dimension. Let `lines` be the product of the counts of the other dimensions. A one-dimensional array has one line.
2. Each line takes `last.div_ceil(8)` bytes.
3. The size is `lines` times that value, rounded up to an even byte count.

A one-dimensional array gets the same size as before. A02 `ARRAY[1..3,1..3] OF BOOL` gets 4 bytes.

The `Byte` and `Even` branches keep their formulas. A line of `BYTE` elements ends on a byte, and a line of even elements ends on an even byte. The arithmetic uses `checked_*` and `try_from` (`reliability.md` §2).

ADR-0147 (g) applies the same steps to three dimensions. A03 `ARRAY[1..3,1..3,1..3] OF BOOL` then takes 10 bytes. This is the larger size of the two readings, so no address of either reading overlaps. ADR-0147 (g) does not decide the reading. The row A03 keeps `Unverified("needs a TIA measurement")`.

One ADR covers the two rules of D1 and D2. Both rules change the S7 layout of ADR-0017 from one research record and one decision of the owner. ADR-0148 is a separate ADR, because it refuses a type and does not change a layout.

### D3: The new mark `Source::Documented`

The doc comment of `Source::Siemens` says that the text names a manual and a page. A derivation from several rules of the research record does not fit that text. A new variant `Documented(&'static str)` therefore holds the rule ids.

- Each of the 21 rows C01 to C13, A01, A02, and A04 to A08, A11 gets `Documented` for its size and its layout.
- The text names the rule ids of the column "Cited rules" in the section "Rows" of the research record, for example `"WP7-R1 S-ST1 S-ST2"`.
- The texts of C01, C08, A11, and C04 end with `"medium confidence"` (ADR-0147 (i)).
- A03 gets `Unverified("needs a TIA measurement")`.
- A09, A10, and the catalog rows 212, 213, 409, and 410 leave the table (ADR-0148 (j)).
- `s7_layout/tests.rs` gets A09 and A10 as golden cases on `S7_300` with the sizes 8 and 6. A comment names `domain-model.md` §2.1 and ADR-0148 (i) as their rule. The cases pin the `Even` branch of an array element when task 1.2 rewrites `array_layout`.
- The test of the marks asserts each row by its id. No composite row keeps `Unverified("Q10")`.

The doc comment of `S7Layout::of` and of `composite` names the new marks. The primitive rows keep their marks.

### D4: The nested array rule is a pure function of `recipe-domain`

A new method of `TypeExpr` in `type_expr.rs` takes a `PlcPlatform`. It returns `PlantError::DerivedTypeInvalid` with the reason `NotOnPlatform` when the platform is `S7_1200` or `S7_1500` and `arrays()` holds more than one prefix. The field `type_expr` holds the text of the expression.

The method checks only the written text. A struct reference in the text can name a stored struct with an array of an array. The method does not open it (ADR-0148 (c)).

### D5: Three commands call the rule

| Command | Place of the call | Error |
|---|---|---|
| `AddParameter` | After `OpenDraft::open` and before `resolve_type`, with `open.draft.platform()` | `E-PLT-013` |
| `UpdateParameterDefinition` | In the branch that calls `require_current_type`, when the type changes | `E-PLT-013` |
| `DefineUdt` | In `ensure_members`, before `ensure_expr` of each member | `E-PLT-006` through `member_error` |

The command writes nothing on a refusal. `UpdateParameterDefinition` calls `resolve_type` before the check, but the transaction rolls back.

`ensure_expr`, `ensure_shape`, the preview, the snapshot check, `ApproveVersion`, and the export do not call the rule (ADR-0148 (f), (g)). The stored type of a parameter then stays readable, and a rebind keeps its array levels.

The preview of a new array of an array on `S7_1500` shows the leaf rows, and the save refuses the type. The author accepts this gap. The preview also shows the stored type of the edit form, and that type must stay visible.

### D6: No new error code

`E-PLT-013` with the reason `NotOnPlatform` fits exactly. The type `ARRAY[..] OF ARRAY[..] OF ..` does not exist on the platform (S-AR5). The reason has a Fluent text in `pl` and `en`. `DefineUdt` maps the reason to `E-PLT-006` with the member name (`domain-model.md` §2.1). `reliability.md` §1.2 gets no row.

### D7: Stored data and ADR-0120

The S7 layout is a computed value:

- `data-model.md` line 393 says that the domain computes the layout and that no column stores it.
- `migrations/0001_schema.sql` lines 142 to 151 require `size_bytes IS NULL` for an array row and a struct row.
- No code of `recipe-db`, `recipe-export`, or `migrations/` calls `s7_members` or `S7Layout::of`.

The layout rules of D1 and D2 therefore need no migration and no earlier rule set (ADR-0147 (j)). A stored draft or version gets the new layout at its next check. ADR-0116 (e) names the paths of the end rule: the written parameters and `ApproveVersion`. D9 and D10 give the paths of the overlap check. The export check of ADR-0119 (b) can set `loadable` to `false`. `loadable` is outside the hashed body (`api.md` §6, line 576).

The nested array rule of D4 follows the pattern of ADR-0143 (i). ADR-0148 (e) amends ADR-0120 (a): the earlier rule set accepts a stored array row whose element is an array. No stored row changes, so no migration runs.

### D8: The ops query `deploy/db/check-s7-layout-rules.sql`

The query copies the form of `deploy/db/check-identifier-rules.sql`. It runs in `BEGIN READ ONLY` and writes nothing. It has two parts:

- `nested_array`: each array row on a machine with `S7_1200` or `S7_1500` whose element row is an array row. It also lists each parameter of a live draft or a version whose type tree holds such a row.
- `layout_changed`: each parameter with an S7 address whose type tree holds an affected row. An affected row is a struct with a direct `SINT` or `USINT` member, or a `BOOL` array with more than one dimension.

The columns are `machine_code`, `reference_number`, `version_number`, `tag`, `part`, and `type_text`. A recursive query walks the type tree over `element_type_id` and `plc_type_member`.

The runbook gets a section and a sub-step of step 2 of "Upgrade". The owner runs the query on production. On 2026-11-08 no production exists, so that task stays open.

### D9: A command checks the overlaps of the parameters that it writes

ADR-0149 (a) to (d) hold the rule. `replace_parameters` knows the keys of its replacements, so it passes them to the overlap check.

- `check_s7_layout` gets the set of written keys. `find_overlap` then skips a pair that holds no written key.
- `check_s7_layout_for_approval` checks every pair, as before.
- The per-parameter rules of `check_layout` do not change.
- `RemoveParameter` keeps no S7 check. `CreateVersion` keeps the copy without an S7 check (ADR-0120 (h)).

A draft with two stored overlap pairs is the test case. An edit of a parameter outside both pairs passes. An edit of a parameter of one pair gives `E-RCP-022`. A move of one parameter of a pair passes, although the other pair stays. `ApproveVersion` gives `E-RCP-022`.

### D10: The export check and the version page check run the overlap check

ADR-0149 (e) to (j) hold the rule. `check_version_parameters` gets one more step after its loop over the parameters.

1. The step collects the occupied bits of each parameter with an S7 address and a tree. It uses `occupy` of `draft/s7.rs`.
2. The step finds every overlapping pair, not only the first.
3. Each parameter of a pair gets a `RuleFault` with `RuleField::Address` and `E-RCP-022`. The step adds the fault to the list of the parameter in parameter order.

The export and the version page query share the function, so both callers get the faults. The callers need no change, because `loadable` and the notice already follow the list of faults. A parameter without a tree stays a structure fault, and the step runs only after the loop has no structure fault.

### D11: One ADR for D9 and D10

D9 and D10 answer one question: where does the overlap check of I20 run after a change of the layout rules. ADR-0149 therefore amends ADR-0116 (e1) and ADR-0119 (b) together. Two ADRs would split one trade-off: a draft that stays editable needs an export that still finds the overlap.

## Risks / Trade-offs

- [A TIA measurement contradicts D1 or D2] → The marks name the rules and the confidence. A new ADR amends ADR-0147.
- [A stored address overlaps under the new layout] → `ApproveVersion` refuses the draft (`E-RCP-022`, `E-RCP-056`). A parameter command refuses only an edit of a parameter of the pair (D9). The ops query lists the parameters before the upgrade.
- [An approved version gets `loadable` `false`] → The end rule or the overlap check of D10 sets it. The hashed body does not change. The owner reads the part `layout_changed` and makes a new version.
- [The preview shows a type that the save refuses] → The save gives `E-PLT-013` with its text. D5 gives the reason.
- [A stored struct with an array of an array gets a new revision] → `DefineUdt` checks each member text again. The user changes the member type.
- [A rebind creates a new nested array row on `S7_1200` or `S7_1500` after the upgrade] → ADR-0148 (f) permits it. The rebind writes no type text. The ops query before the upgrade does not see such a row. A later run of the query lists it.
- [`UpdateParameterDefinition` checks the nested array rule after `resolve_type`] → `resolve_type` can take class-2 locks and insert derived rows before the refusal. The transaction rolls back, so no row stays. ADR-0143 (i2) uses the same order. A later change can split `resolve_type`.

## Migration Plan

1. No schema migration runs.
2. Before the upgrade, the owner runs `deploy/db/check-s7-layout-rules.sql` with the command of the runbook.
3. The owner reads each row. A row of `layout_changed` can need a new address in a draft.
4. A rollback is the previous image. No stored row changes, so a rollback needs no restore.

## Results of R2 to R4

The record `docs/plan/review/2026-11-08-wp7-r2-r4-platform-research.md` holds the evidence. No result needs a code change or an ADR.

- R2: no source refuses `WSTRING`, `LTIME`, `LTOD`, `LDT`, or `DTL` in a data block with standard access on `S7_1500`.
- R3: `S7_1200` supports `WCHAR` and `WSTRING`. The first firmware is unknown. Sources give two length limits, 65534 words or 16382 characters. The current limit of 16382 stays.
- R4: data block numbers 1 to 59999 on firmware V4.7 and G2 agree with ADR-0140. Firmware V3.0 permitted 65535.

## Open Questions

- O-1: decided on 2026-11-08. The owner accepts the size 10 of A03 as an interim bound. ADR-0147 (g1) records the gap of the member offsets on the type page.
- O-2: the start of a `SINT` or `USINT` element of an array, and of its lines. No page decides. The code keeps one byte.
- O-3: the start of a `SINT` or `USINT` parameter (`domain-model.md` line 190). No page decides. The code keeps any byte.
- O-4: must `ApproveVersion` or the export refuse a stored array of an array on `S7_1200` or `S7_1500`? ADR-0148 (g) says no. A yes adds a requirement.
- O-5: S-AR5 applies to each block with standard access, so also to `S7_300`. The owner decision names only `S7_1200` and `S7_1500`. `S7_300` keeps the current rule.
- O-6: decided on 2026-11-08. ADR-0148 amends ADR-0017 and ADR-0120. ADR-0045 stays under "Related", because it decides only the page groups of nested arrays.
- O-7: decided on 2026-11-08. Task 5.1 uses Sonnet. The owner does not use Codex for this change.
