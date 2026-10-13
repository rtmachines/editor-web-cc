# Tasks: domain-plant-model

Each code task starts with a failing test. Run it and see it fail. Then write the code. After each code task, run the clippy, test, and fmt commands of `openspec/config.yaml`, section `operations.apply.guidance`. A `[mechanical]` task goes to a Haiku subagent. Sonnet does every other code task.

The ste-lint command is the last command of the section "Local checks" of `CONTRIBUTING.md`.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0024 for the codes `E-PLT-021` and `E-RCP-054`. Add both rows to `reliability.md` section 1.2. Add the codes to the error lists of `domain-model.md` sections 2.1, 5.1, 5.2, `data-model.md` section 6, and `api.md` sections 2.3, 2.6. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.2 (docs/adr) Write ADR-0025 for the parse reasons of value objects. Add the rule to `reliability.md` sections 1.1 and 4.3. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.3 (docs/adr) Write ADR-0026 for the ranges of `DATE` and `DATE_AND_TIME`. Change the rows of the base-unit table of `domain-model.md` section 2.1. Add the index row and the decision map entry. Check with the ste-lint command.
- [x] 1.4 (CHANGELOG.md) Add the three ADRs and the plant model types under `[Unreleased]`. Check with the ste-lint command.

## 2. Error code type

- [x] 2.1 (recipe-domain) Add the type `ErrorCode` (D4). First write failing tests for the text form `E-PLT-001` and the order. Also test that `REGISTRY` holds sorted, unique codes inside the ranges of `reliability.md` section 1.2. Add `thiserror`, `uuid`, `time`, the dev-dependency `proptest`, and `#![deny(missing_docs)]`. Implement `error_codes.rs` and an empty `PlantError` with `code()`. Check with `cargo test -p recipe-domain error_codes`.

## 3. Identifiers and shared value objects

- [x] 3.1 (recipe-domain) [mechanical] Add `MachineId`, `UnitId`, and `PlcTypeId`. First write failing tests for the conversion from `Uuid` and the `Display` form. Check with `cargo test -p recipe-domain plant::ids`.
- [x] 3.2 (recipe-domain) [mechanical] Add `Timestamp` and `Text` (D11). First write failing tests from the row "Text (names)" of `reliability.md` section 4.3. Cover 200 characters, an empty text after trim, 201 characters, and a control character. Check with `cargo test -p recipe-domain text`.
- [x] 3.3 (recipe-domain) [mechanical] Add `PlcPlatform`. First write failing tests for the five values of `domain-model.md` section 2.1 and one bad value. Check with `cargo test -p recipe-domain plant::platform`.
- [x] 3.4 (recipe-domain) [mechanical] Add `MachineCode`. First write failing tests from its row of `reliability.md` section 4.3. `M1` and `M9999999` pass. `M0`, `M01`, `M10000000`, and `m1` fail. Add a proptest for the round trip of `parse` and `Display`. Check with `cargo test -p recipe-domain machine_code`.
- [x] 3.5 (recipe-domain) [mechanical] Add `ReferenceNumber` to the module `recipe` (D10). First write failing tests from A27: the upper-case folding, 50 characters pass, 51 characters fail. Add a proptest for the round trip (`reliability.md` section 6). Check with `cargo test -p recipe-domain reference_number`.

## 4. Unit path and machine

- [x] 4.1 (recipe-domain) [mechanical] Add `UnitPath` and the variants `E-PLT-002` and `E-PLT-017`. First write failing tests from I9. `U1.2.4.5.6` passes. `U65`, `U01`, and `U1..2` give `E-PLT-017`. Six segments give `E-PLT-002`. Also test `parent` and the numeric order. Add a proptest for the round trip. Check with `cargo test -p recipe-domain unit_path`.
- [x] 4.2 (recipe-domain) Add `Machine`, `Unit`, `StoredMachine`, and `TryFrom<StoredMachine>` (D5). First write failing tests for a duplicate path and a unit without its parent (I10). Also test a live unit under an archived parent and no live main unit. Add the variants `E-PLT-003`, `E-PLT-004`, `E-PLT-008`, and `E-PLT-009`. Check with `cargo test -p recipe-domain plant::machine`.
- [x] 4.3 (recipe-domain) Add `Machine::create`, `add_unit`, and the variant `E-PLT-010`. First write a failing test that a new machine has the main unit `U1`. Then write failing tests for the errors of `AddUnit` in `domain-model.md` section 5.1. Also test the segment `1.2`. Check with `cargo test -p recipe-domain add_unit`.
- [x] 4.4 (recipe-domain) Add `Machine::unit`, `rename`, `rename_unit`, `archive`, and the variant `E-PLT-011`. First write failing tests from the rows of `domain-model.md` section 5.1. Add a `compile_fail` doctest for I21. Also add `E-PLT-001`, `E-PLT-019`, and `E-PLT-020` with a test of `code()` and `Display`. Check with `cargo test -p recipe-domain plant::machine`.
- [x] 4.5 (recipe-domain) Add `archive_unit`, `UnitUsage`, and the variants `E-PLT-012` and `E-PLT-014`. First write failing tests from ADR-0021. Cover the cascade, the `id` order of the result, and the errors `E-PLT-008`, `E-PLT-012`, `E-PLT-014`. Check with `cargo test -p recipe-domain archive_unit`.

## 5. Type catalog and type tree

- [x] 5.1 (recipe-domain) [mechanical] Add `Primitive` and `LeafClass`. First write failing tests from the matrix of `data-model.md` section 7. Cover the 29 names, the class of each name, and the SQL names of `LeafClass`. Check with `cargo test -p recipe-domain plant::primitive`.
- [x] 5.2 (recipe-domain) [mechanical] Add `TypeName` and `MemberName`. First write failing tests from `reliability.md` section 4.3 and `data-model.md` section 3.4. A name of 64 characters passes and 65 fail. A primitive name fails for `TypeName`. Check with `cargo test -p recipe-domain type_name`.
- [x] 5.3 (recipe-domain) [mechanical] Add `StringLength`. First write failing tests from its table in `domain-model.md` section 2.1. Cover each range end per platform, `WSTRING` on `S7_300`, and the default length per platform. Check with `cargo test -p recipe-domain string_length`.
- [x] 5.4 (recipe-domain) [mechanical] Add `ArrayDimension` and the list of 1 to 3 dimensions (A38). First write failing tests. `5..1` and `0..65536` fail. `0..65535` passes. `ARRAY[1..2,1..3]` has 6 elements. Check with `cargo test -p recipe-domain array_dimension`.
- [x] 5.5 (recipe-domain) Add `PlcType` for primitive and array rows, `DerivedTypeReason`, and the variant `E-PLT-013` (D6). First write failing tests for the shape rules of these rows in `data-model.md` section 3.3. Also test a wrong class, a string length per platform, and the size of `STRING(n)`. Check with `cargo test -p recipe-domain plc_type`.
- [x] 5.6 (recipe-domain) Add struct rows to `PlcType` with `TypeMember` and the variants `E-PLT-015`, `E-PLT-016`, `E-PLT-018`, `E-PLT-021`. First write failing tests for no member, 1024 members, 1025 members, and a duplicate member name. Add a `compile_fail` doctest for I22. Also add `E-PLT-006` with a test of `code()` and `Display`. Check with `cargo test -p recipe-domain plc_type`.
- [x] 5.7 (recipe-domain) [mechanical] Add `TypeExpr` with the loop of D2. First write failing tests from the grammar, the examples, and the reason table of `domain-model.md` section 2.1. Also test `STRING(99999)`, `ST_Zone@4294967296`, and `ARRAY[2147483648..1] OF INT`. Add a proptest for the round trip. Check with `cargo test -p recipe-domain type_expr`.
- [x] 5.8 (recipe-domain) Add `TypeTree`, `PlatformTypes`, `resolve`, and the variant `E-PLT-005` (D7). First write failing tests for a struct cycle (I8) and a missing identifier (`E-PLT-013`). Also test the tree of `ST_Zone` of `data-model.md` section 9.1. Check with `cargo test -p recipe-domain type_tree`.
- [x] 5.9 (recipe-domain) Add the size limits to `resolve` (D7). First write failing tests from `reliability.md` section 7.11. 16 384 leaves and 32 levels pass. 16 385 leaves and 33 levels give `E-PLT-018`. Check with `cargo test -p recipe-domain type_tree::limits`.
- [x] 5.10 (recipe-domain) Add a proptest strategy for random type trees up to depth 6 (`reliability.md` section 6). Assert that `resolve` returns a tree or an error. Assert that the leaf count equals the product of the element counts. Make the strategy available to P2.2. Check with `cargo test -p recipe-domain type_tree::prop`.

## 6. Leaf path

- [x] 6.1 (recipe-domain) [mechanical] Add `LeafPath`. First write failing tests from its grammar and examples in `domain-model.md` section 2.1. Four indexes, `[*,1]`, and an index outside the INTEGER range fail. Add a proptest for the round trip. Check with `cargo test -p recipe-domain leaf_path`.
- [x] 6.2 (recipe-domain) Add the resolution of a `LeafPath` against a `TypeTree`. First write failing tests. `.ramps[*]` names 3 leaves, and the empty path names a primitive parameter. No leaf exists for `.ramps[4]`, `.ramps`, `[1]` on a 2-dimensional array, and the empty path on a struct. Check with `cargo test -p recipe-domain leaf_path::resolve`.

## 7. S7 address and layout

- [x] 7.1 (recipe-domain) [mechanical] Add `S7Address`, the enum `S7Width`, and `width` (D8). First write failing tests from the grammar of `domain-model.md` section 2.1. `DB1.DBX0.7` and `DB65535.DBD65535` pass. `DB0.DBW0`, `DB1.DBX0`, `DB1.DBW0.1`, and `DB1.DBX0.8` fail. Add a proptest for the round trip. Check with `cargo test -p recipe-domain s7_address`.
- [x] 7.2 (recipe-domain) [mechanical] Add the expected `S7Width` of a type tree. First write failing tests from the width table of `domain-model.md` section 2.1. Use the column "S7 width" of `data-model.md` section 7. Check with `cargo test -p recipe-domain s7_width`.
- [x] 7.3 (recipe-domain) Add `S7Layout` (D8). First write a failing golden test. Cover `BOOL` packing, the padding before a wide member, and the even end of a struct and an array. Also cover an array of `STRING(3)` and the structs of the delta specs. Check with `cargo test -p recipe-domain s7_layout`.
- [x] 7.4 (recipe-domain) Add a proptest for `S7Layout` with the strategy of 5.10. Assert that a struct and an array have an even size. Assert that the size is at least the storage of the leaves. Check with `cargo test -p recipe-domain s7_layout::prop`.

## 8. IEC literal

- [x] 8.1 (recipe-domain) [mechanical] Add the calendar functions of D9. First write failing tests. `2026-10-12` is day 20738. `2024-02-29` exists, and `2026-02-29` does not exist. Check with `cargo test -p recipe-domain iec_literal::calendar`.
- [x] 8.2 (recipe-domain) [mechanical] Add `TimeType` and the parser of the duration literals. First write failing tests from `domain-model.md` section 2.1. `T#1d2h3m4s5ms` is 93784005. Cover the unit order, the units per prefix, and the sign. Check with `cargo test -p recipe-domain iec_literal::duration`.
- [x] 8.3 (recipe-domain) [mechanical] Add the canonical `Display` of a duration and the `S5TIME` steps. First write failing tests. `T#90m` shows `T#1h30m`, zero shows `T#0ms`, and `S5T#12345ms` fails. Check with `cargo test -p recipe-domain iec_literal::canonical`.
- [x] 8.4 (recipe-domain) [mechanical] Add the date, time-of-day, and date-and-time literals. First write failing tests from the examples of `domain-model.md` section 2.1. Cover the fraction digits per type and the hour `24`. Check with `cargo test -p recipe-domain iec_literal::instant`.
- [x] 8.5 (recipe-domain) Add the range of each `TimeType` (ADR-0026). First write failing tests from the base-unit table of `domain-model.md` section 2.1. Cover both ends and one value outside. Check with `cargo test -p recipe-domain iec_literal::range`.
- [x] 8.6 (recipe-domain) [mechanical] Add a proptest for each `TimeType`. A count inside the range survives `Display` and `parse`. A canonical literal survives `parse` and `Display`. Check with `cargo test -p recipe-domain iec_literal::prop`.

## 9. Integration checks

- [x] 9.1 (recipe-domain) [mechanical] Add the test of D4. It compares the 20 `E-PLT` domain rows of `reliability.md` section 1.2 with the variants and `REGISTRY`. Check with `cargo test -p recipe-domain registry_matches_reliability`.
- [x] 9.2 (openspec/changes/domain-plant-model) Check the change with `openspec validate domain-plant-model --strict`. Run `ste-lint.py` on the proposal, the design, the tasks, and the delta specs.

## Workflow follow-up

- Run every command of the section "Local checks" of `CONTRIBUTING.md`, including the coverage gate.
- Archive the change with the skill `openspec-archive-change` after the code review.
- P2.2 adds the variant `E-RCP-054` and the width check of a draft.
- The owner supplies the Siemens reference table of ADR-0017. A later change adds the comparison test.
