# Proposal: domain-plant-model

## Why

Every later change needs the Plant Model types. Recipe Management checks values, limits, and S7 addresses against a resolved type tree.

The change satisfies these requirements of `overview.md` section 7: R05, R06, R07, R09, R11, R12, R13, R14, R15, R16, R32, R33, R36, R38.

The change uses these Assumptions of `overview.md` section 9: A3, A6, A26, A27, A30, A31, A32, A38, A39, A43.

## What Changes

- Add ADR-0024 (codes `E-PLT-021`, `E-RCP-054`), ADR-0025 (parse reasons), and ADR-0026 (ranges of `DATE` and `DATE_AND_TIME`).
- Add the module `plant` to `recipe-domain` with the value objects of `domain-model.md` section 2.1.
- Each value object gets one `parse` constructor: `MachineCode`, `PlcPlatform`, `UnitPath`, `TypeName`, `StringLength`, `ArrayDimension`, `TypeExpr`, `LeafPath`, `S7Address`, `IecLiteral`.
- Add `ReferenceNumber` to the module `recipe`. Three `PlantError` variants carry it.
- Add the entity `Machine` with its unit tree. The tree enforces I9, I10, the archive of descendants, and `LastMainUnit`.
- Add the entity `PlcType` with the kinds `Primitive`, `Array`, `Struct`, and the enum `LeafClass`.
- Add `TypeTree::resolve` with `E-PLT-005` for a cycle and `E-PLT-018` above a size limit.
- Add `S7Layout`. It computes the layout size and the alignment of a type tree.
- Add `PlantError` with `code()` for `E-PLT-001` to `E-PLT-006` and `E-PLT-008` to `E-PLT-021`.
- Add the type `ErrorCode` and the registry `error_codes.rs` with these rows.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: add the string length per platform and the unique member names of a struct.
- `recipe-management`: add the IEC literal, the leaf path, the S7 layout size, the S7 address grammar, and the S7 address width.

## Impact

- Code: `crates/recipe-domain` only. New dependencies: `thiserror`, `uuid`, `time`. New dev-dependency: `proptest`.
- Docs: three new ADRs and their rows in `reliability.md`, `domain-model.md`, `data-model.md`, and `api.md`.
- No migration, no port, no I/O, and no change of another crate.

## Out of scope

- The variants of `RecipeError`, including `E-RCP-054`. P2.2 adds them with the checks of a draft.
- `EnsureDerivedType`, `DefineUdt`, and the lookup of a `TypeExpr` in the type catalog (P2.3).
- The trigger codes `E-PLT-030` to `E-PLT-038` of `StorageError::Constraint` (P2.3, P2.4).
- The registry rows of the other contexts. Each change adds the rows of its own variants.
