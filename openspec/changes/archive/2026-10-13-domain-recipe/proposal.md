# Proposal: domain-recipe

## Why

The use cases of P2.3 need the Recipe Management rules as pure domain code. The rules check versions, drafts, values, limits, and S7 addresses without I/O.

The change satisfies these requirements of `overview.md` section 7: R02, R08, R09, R10, R11, R12, R14, R16, R17, R18, R19, R23, R26, R27, R32, R36, R38.

It uses these Assumptions of `overview.md` section 9: A5, A7, A11, A12, A13, A14, A17, A26, A28, A29, A31, A34, A36, A38, A40, A43.

## What Changes

- Add the identifiers `RecipeId`, `RecipeVersionId`, `ParameterId`, and `ParameterKey` (A40).
- Add ADR-0027 (error lists) and ADR-0028 (version header, approval token, clarifications).
- Add `UserId` and `Login` to a new module `identity` (design D2).
- Add `VersionStatus` and the state machine of `domain-model.md` section 4 as one transition function. Each forbidden edge returns `Err`.
- Add the root `Recipe` with `VersionHeader`, `approve` (I1, I2, I16) with an approval token, `retire` (A28), `discard`, `delete`, `restore`, `rename`, and `create_version` (A7, ADR-0021).
- Add `Draft` with `Parameter`, `ParameterTag`, `LangCode`, `Translation`, and the description texts.
- Add `ProcessLimit` per leaf path and `ParameterValue` with exact shape, type ranges, and canonical decimal text (I7, ADR-0007).
- Add the S7 checks I5, I20, and the width check of ADR-0024. Reuse `S7Address`, `S7Width`, and `S7Layout` of the module `plant`.
- Add the version budgets (I18), the snapshot check (I19), the approval check (I17, A29), and the outdated rule.
- Add the pure mapping of `RebindTypeRevision` (ADR-0018).
- Add `TypeTrees`, `LiveUnits`, the leaf walk, and the platform of a `TypeTree` to the module `plant`.
- Add `RecipeError` with `code()` for `E-RCP-001` to `E-RCP-029` and `E-RCP-050` to `E-RCP-054`, and the carriers `RecipeErrors` and `DraftError`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: add eight requirements of `domain-model.md` sections 2.2 and 4. Extend the version transitions and the separation of duties.

## Impact

- Code: `crates/recipe-domain` only. New dependencies: `serde_json` with `arbitrary_precision` and `rust_decimal` (`overview.md` section 5).
- Docs: ADR-0027, ADR-0028, and their changes of `reliability.md` and `domain-model.md`.
- No migration, no port, and no I/O.

## Out of scope

- The codes `E-RCP-030` to `E-RCP-036`. They are `StorageError::Constraint` rows (P2.3, P2.4).
- The use cases, the ports, the row version compare of I12, and `EnsureDerivedType` (P2.3).
- The module `export`, `ExportBody`, and the computation of `content_sha256` (ADR-0019, P2.3 or P3.5).
- The read models `ApprovedVersion` and `RetiredVersion` (P2.3).
- The audit entries and `draft_editor` rows. The use case writes them (P2.3).
- The `User` and `Session` entities (P2.5).
