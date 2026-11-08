# Design

## Context

See `proposal.md`, section "Why". ADR-0146 holds the rule. ADR-0018 holds the rest of the rebind. `domain-model.md` §5.2 lists the rebind rules.

The author read the code at `2fd0023`. These facts shape the design:

- `kept_members` (`crates/recipe-domain/src/recipe/rebind/level.rs:40-55`) keeps a target member when a source member has the same name and the same row. Line 51 holds the test `old == name && old_node.type_id() == node.type_id()`.
- Three callers use the result: the value mapping (`rebind/value.rs:45`), the limit mapping (`rebind/limit.rs:33`), and the new leaves of the rebind form (`rebind/leaves.rs:56`).
- `TypeNode` (`crates/recipe-domain/src/plant/type_tree.rs:116-150`) has three kinds. A leaf holds `type_id`, `primitive`, `leaf_class`, `size_bytes`, and `string_length`. `build_node` (`type_tree.rs:497-511`) copies the fields of the row.
- `check_primitive` (`plant/plc_type.rs:200-218`) requires `leaf_class` to equal `primitive.leaf_class()`. It also requires `size_bytes` to equal the size of the primitive and the length. A string row has `n + 2` or `2n + 4` bytes (`plant/string_length.rs:84-90`). Every other primitive has a fixed size (`plant/primitive.rs:233-254`).
- Every string row stores its length. The CHECK at `migrations/0001_schema.sql:120` requires it. `check_primitive` (`plant/plc_type.rs:203-218`) refuses a string row without a length.
- The seed rows store the default length of the platform (`migrations/0004_seed_plc_types.sql:22`, `:23`, `:36`, `:37`). `StringLength::default_length` (`plant/string_length.rs:70-78`) gives the same values. The test at `plant/string_length/tests.rs:101-121` pins them.
- `EnsureDerivedType` gives the seed row for a `STRING` without a length (`crates/recipe-app/src/plant/ensure_derived.rs:160-164`). For `STRING(n)` it finds or creates a row of the machine (`:165-185`, `:240-246`). Thus `STRING(80)` on `CodesysIec` is a row of the machine.
- `MemberName` derives `PartialEq` (`plant/type_name.rs:221-222`). The name test of line 51 compares the exact spelling.

Default lengths per platform, from `string_length.rs:70-78`:

| Platform | `STRING` | `WSTRING` |
|---|---|---|
| `CodesysIec`, `TwinCat3` | 80 | 80 |
| `S7_300` | 254 | not available |
| `S7_1200`, `S7_1500` | 254 | 254 |

## Goals / Non-Goals

Goals:

- One shape comparison for the three callers of `kept_members`.
- Tests at the domain, the use case, and the storage level.

Non-Goals:

- No change in `recipe-app`, `recipe-db`, or `recipe-web` outside tests.
- No change of the name match (ADR-0146 (l)). See "Open Questions".
- No shape rule over the members of a struct (ADR-0146 (e)).

## Decisions

### D1: A private function `same_type_shape` in `level.rs`

`kept_members` calls `same_type_shape(old_node, node)` in place of the row test. The name test stays.

`same_type_shape` walks the two nodes in one loop:

1. Two leaves: compare every field except `type_id`. The result ends the loop.
2. Two arrays: compare `dimensions`. If they differ, the result is `false`. Otherwise continue with the two elements.
3. Two structs: compare `type_id`. The result ends the loop (ADR-0146 (e)).
4. Two nodes of two kinds: the result is `false`.

The loop follows only array elements, so it needs no recursion and no stack (`reliability.md` §2.2).

Alternatives:

- A method of `TypeNode` in `plant/`. Rejected, because the rebind is the only caller.
- Compare only `primitive` and `string_length` of a leaf. The row check makes both forms equal today. The full compare also guards a later field of a leaf (ADR-0146, "Risks").

### D2: No resolution of the default length at the compare

The compare reads `string_length` of each leaf. Every string leaf holds it, see "Context". A call of `StringLength::default_length` would serve a leaf that cannot exist. Thus the compare does not call it. The rule "the default length counts as an explicit length" (ADR-0146 (c)) holds through the stored seed rows.

### D3: A struct compares by row

ADR-0146 (e) gives the reason. A member of type `ST_Zone@2` and a member of type `ST_Zone@3` have two shapes, also with equal members.

### D4: Doc comments

- `level.rs:36-39`: the doc comment of `kept_members` names the type shape of ADR-0146.
- `limit.rs:16-22`: "a member with a changed type" becomes "a member of another type shape".

### D5: What the rebind writes

The use case does not change (`crates/recipe-app/src/recipe/rebind.rs:77-115`):

- The parameter gets the target revision or a new array row, as before.
- The kept value stays unchanged. The rebind replaces the limit rows by `DELETE` and `INSERT` (`crates/recipe-db/src/transaction/draft_writes.rs:245-263`). Thus a kept limit row has the same content and a new `created_at`.
- A kept member uses the row that the target revision names for it.
- `rebind_diff` (`crates/recipe-app/src/audit/diff.rs:70-118`) adds a section only for a changed part (`diff.rs:135-143`). Thus a rebind that keeps every member gives a diff with `plc_type_id` only.
- The export of the new version lists the rows of the target revision (`api.md` §6.1). No stored version changes, so no stored content hash changes.

### D6: Mutations of the red steps

| Id | Mutation in `level.rs` | Test that fails |
|---|---|---|
| M1 | `kept_members` compares `type_id` again | Every test of a kept member with two rows of one shape |
| M2 | A leaf compare reads only `primitive` and `leaf_class` | `STRING(80)` to `STRING(81)` |
| M3 | A leaf compare ignores `primitive` | `INT` to `UINT` |
| M4 | An array compare ignores `dimensions` | `ARRAY[1..2]` to `ARRAY[1..3]` of one element |
| M5 | A struct compare reads `name` and not `type_id` | `ST_Zone@2` to `ST_Zone@3` |
| M6 | The array compare ignores the element | `kept_members_drops_an_array_member_whose_element_changes_from_int_to_dint` in `level/tests.rs`, task 1.2 |

`check_primitive` gives each leaf a size from its primitive and its length (see "Context"). Thus a mutation that ignores only `string_length` still sees another `size_bytes`. M2 therefore drops the length and the size. M3 needs a pair with one leaf class and one size. `INT` and `UINT` are both `Numeric` with 2 bytes (`plant/primitive.rs:200-237`).

A task with a mutation copies `level.rs` to the scratch directory first. After the run it restores the file and runs `cmp` on the copy. `cmp` gives no output. The report of the task gives this result.

### D7: Fixtures

- The fixture `string_row` (`plant/type_tree/fixtures.rs:65-78`) builds a `STRING(n)` row of the machine on `TwinCat3`. `on_platform` (`:176-188`) moves rows to another platform.
- No fixture builds a seed string row. Each new test file builds it from `PlcTypeRecord` with `machine: None`. Its platforms and its length follow `0004_seed_plc_types.sql`.
- The new test files do not change `rebind/test_support.rs`. Thus the tasks of section 1 of `tasks.md` touch no shared fixture.

## Risks / Trade-offs

- [The rebind keeps more members than before] → A kept value and its limits still pass the check against the planned tree. The check uses the earlier rule set (`value.rs:60`, `limit.rs:49`, ADR-0120 (d)).
- [A later field of a leaf changes the PLC layout] → D1 compares every field of a leaf except `type_id`.
- [A case-only rename stays a new member] → The engineer gives the values again. The follow-up F-8 of the fix plan records the point (see "Open Questions").

## Migration Plan

No migration and no data step. A rollback reverts the commit of the change.

## Open Questions

None. The owner closed the two earlier questions on 2026-11-08. Both stay out of scope. The fix plan `docs/plan/2026-11-06-plc-type-fix-plan.md` records each one as a follow-up.

- Q1, closed (F-8): a member that changes only the case of its name stays a new member. ADR-0143 (b) compares member names without case only inside one struct. The rebind match keeps the exact spelling (line 51 of `level.rs`, the key read at `value.rs:114`, ADR-0146 (l)).
- Q2, closed (F-9): a member of a newer struct revision stays a new member, also when the two revisions have equal members. ADR-0146 (e) gives the reason.
