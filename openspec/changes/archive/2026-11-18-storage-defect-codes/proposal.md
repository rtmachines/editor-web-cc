# Proposal

## Why

A read gives `500 E-DB-008` for a stored part that fails the earlier rule set (ADR-0118 (d)). A draft command gives the `422` or `409` code of the broken rule for the same storage defect. ADR-0195 makes the command give `E-DB-008` too. This satisfies R35 and R36, and R14 for the address rules. ADR-0028 also says that `UpdateParameterDefinition` repairs a parameter whose type does not resolve, but no user reaches that repair today.

## What Changes

- A draft command gives `500 E-DB-008` when a stored part that it reads and does not write fails the earlier rule set. The part of the request keeps the code of its rule.
- ADR-0195 items (f) to (n) list the sites in the domain. Items (w) to (y) define the type terms.
- A stored type that the platform does not hold keeps `422 E-PLT-013` (ADR-0195 (h), (x)).
- A lock of `OpenDraft::open` or `lock_live_unit` that finds no row gives `E-DB-008` in place of `E-WEB-005`.
- `ApproveVersion` and `GetApprovalPreview` give `E-DB-008` for duplicate stored parameters (ADR-0195 (j)).
- `UpdateParameterDefinition` replaces a parameter type that does not resolve. The audit entry then holds the whole old value under the key `""` (ADR-0195 (q)).
- The parameter form query gives `E-DB-008` for three faults. They are a failed resolution of the stored type, a missing type text, and two kept type rows with one identifier. For the purpose `UpdateParameterDefinition`, a stored type that does not resolve opens the form. Two kept type rows with one identifier still give `E-DB-008` (ADR-0195 (z) to (ac)).
- `ListParameters` and `GetParameter` return a view without a tree for a type that does not resolve. The parameter page shows the definition without the value grid (ADR-0195 (ad), (ae)).
- `UpdateParameterDefinition` writes both value sections of its audit entry for a replaced type without a tree, also for equal sides (ADR-0195 (q)).
- Text only: `E-WEB-011` of the random bytes of a session and of a one-time password in `data-model.md`, `domain-model.md`, `reliability.md`, and `api.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: draft commands and the approval on a storage defect, and the parameter form query. Also the repair of a parameter whose type does not resolve.
- `web-ui`: the parameter page and the parameter table of a parameter whose type does not resolve. The edit form of such a parameter has no leaf grid.
- `audit`: the diff of `ParameterDefinitionUpdated` for a replaced type without a tree.

## Impact

- `recipe-domain`: `DraftError` gets the variant `Stored`. `Draft::load` returns `DraftError`. The draft and rebind methods mark their stored sites.
- `recipe-app`: one conversion maps `Stored` to `StorageError::Other`. The lock arms, the type rows, the audit diff, the form query, and the queries `ListParameters` and `GetParameter` change. The memory store gets two fault hooks.
- `recipe-web`: the parameter page and its call sites handle a view without a tree. Route tests check the `500` answer and the repair.
- Docs: ADR-0195, `docs/adr/README.md`, `data-model.md` §6, §9.2, and §10, `domain-model.md` §2.4, §5, §5.2, and §5.3, `reliability.md` §1.1, `api.md` §2 and §5.1, `design-system.md` §10.4.
- The Status lines of ADR-0005, ADR-0018, ADR-0027, ADR-0028, ADR-0039, ADR-0041, ADR-0043, ADR-0118, ADR-0120, and ADR-0121 get the note "Amended by ADR-0195".
- No migration, no new error code, no new setting.

## Out of scope

- `DefineUdt`, `EnsureDerivedType`, `ApproveVersion`, `CreateVersion`, `ExportVersion`, and the queries other than the parameter form query keep `E-PLT-013` for two kept type rows with one identifier.
- `GetLimits` and the queries other than `GetParameterForm`, `GetParameter`, and `ListParameters` keep the codes of a failed resolution.
- `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` keep `E-PLT-005`, `E-PLT-018`, `E-PLT-013` with the reason `RowShape`, and `E-RCP-052` for a type that the request names.
- A draft command keeps `E-RCP-021` for a stored draft above a version budget.
- `ApproveVersion` keeps `422 E-RCP-020` and the codes of the S7 rules for a stored part that fails the earlier set (ADR-0120 (f), (h)).
- `CreateVersion` keeps `E-RCP-020` for a base part that fails the earlier set (ADR-0120 (g)).
- No code change for `E-WEB-011`. The code already gives it.
