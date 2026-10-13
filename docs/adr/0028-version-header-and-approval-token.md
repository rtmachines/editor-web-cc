# ADR-0028: Version header, approval token, and draft clarifications

Status: Accepted
Date: 2026-10-13
Deciders: project owner

## Context

The design of the Recipe Management domain types showed gaps between the documents and a public API that cannot build an invalid state:

- `VersionHeader { id, number, status, row_version }` in `domain-model.md` section 2.2 has no deletion time. The root cannot tell a live draft from a discarded draft (A7, section 4).
- `Recipe::approve(.., by: UserId, .., editors: &DraftEditors)` in section 6 takes no proof that the draft passed its approval checks. An empty draft or a draft without limits can become `Approved` (I17).
- `E-RCP-015` names the login of the approver. `DraftEditors` holds user identifiers only.
- I12 gives the row version compare to the use case. `api.md` section 4 gives the order: the idempotent approve first, then the compare. A domain method that takes the expected row version needs the same order.
- I19 names `Draft::check_snapshot(&LiveUnits)`. The snapshot items need the type trees and the recipe (section 5.2).
- `domain-model.md` I7 names `ParameterValue::check`. `reliability.md` section 6 names `ParameterValue::validate`.
- A draft command cannot load a draft whose stored value no longer fits after a catalog or range change. The user then cannot remove the parameter.
- A browser form sends a line end as CRLF. `reliability.md` section 4.3 permits only "newline".
- Three value rules follow from `reliability.md` section 4.4, but no document states them. They are a fraction for an integer type, a `STRING` character outside Latin-1, and the IEEE range of `REAL` and `LREAL`.
- `domain-model.md` section 4 says "Any version can be copied into a new Draft" only in the note of `Retired`.

## Decision drivers

- The public API of `recipe-domain` cannot build an invalid version state, `Draft`, or `ParameterValue` (R32, R38).
- The code follows the documents (CLAUDE.md, source of truth).
- Validation runs before the transaction (`reliability.md` section 7.4).

## Considered options

### Option A: One ADR that changes the listings

- Pros: the documents and the code agree. One record holds every clarification of the domain change.
- Cons: one ADR covers several related points.

### Option B: Record the changes only in the design of the change

- Pros: no change of the documents.
- Cons: `domain-model.md` sections 2.2 and 3 then contradict the code.

## Decision

The project uses **Option A**.

Version header and commands:

- `VersionHeader` stores a `VersionState`: `LiveDraft`, `Discarded`, `Approved`, or `Retired`. The SQL status and `deleted_at` give the state. A header therefore cannot be approved and discarded at once.
- `VersionState::apply` is the one transition function of ADR-0016. A forbidden transition gives `E-RCP-007` for `ApproveVersion` and `DiscardDraft`, `E-RCP-009` for `RetireVersion`, and `E-RCP-002` for a draft command.
- `Recipe::approve`, `Recipe::retire`, and `Recipe::discard` take the expected row version. A mismatch returns `Optimistic::Conflict`, and the use case returns `E-WEB-001`. The order is the idempotent approve, then the row version, then the transition (`api.md` section 4).
- A draft command compares the row version in the use case after `Draft::load`, as I12 says. `Draft::load` gives `E-RCP-002` first (`data-model.md` section 6).
- `Recipe::delete` takes `RecipeDeleter` with the values `Administrator` and `ProcessEngineer` (ADR-0021).
- `CreateVersion` accepts any version of the recipe as its base, also a discarded draft. Without a base, the draft has no parameters (`api.md` section 2.5).
- A version number fits the SQL type `INTEGER` (`data-model.md` section 1.2). A next number above this range gives `E-RCP-021` with the budget "version numbers".
- The root holds no timestamp of a version. `approve`, `retire`, and `discard` take no time, because the SQL statements write `now()`.

Approval:

- `Draft::check_for_approval(&LiveUnits, &TypeTrees, allow_outdated)` returns the token `ApprovalReady` or `DraftError` (ADR-0027). Only this function creates the token.
- The token holds the version, its row version, the creator, the editors of `draft_editor`, and `allow_outdated`. The `Draft` loads the creator and the editors with the version, so `DraftEditors` is part of the `Draft`.
- `Recipe::approve(ApprovalReady, expected_row_version, &Approver, &ApprovalPolicy)` consumes the token. `Approver` holds the identifier and the login of the approving user. `E-RCP-015` names that login.
- `Recipe::approval_replay(number, expected_row_version)` gives `AlreadyApproved` before the transaction, because an approved version has no draft for a token.
- `Recipe::check_approvable(number)` gives `E-RCP-007` for a version that is not a live draft. The use case calls it before `Draft::load`, so `ApproveVersion` on a retired version gives `E-RCP-007`, not `E-RCP-002`.
- The token borrows the `Draft` that created it. While the token lives, no draft command can change that `Draft`.
- `check_for_approval` and `VersionSnapshot::check` reject `TypeTrees` and `LiveUnits` of another machine or platform with `E-PLT-013` or `E-PLT-020`.

Draft and checks:

- `Draft::load` trusts the stored values and limits. A command parses only the parameter that it changes. A parameter whose type does not resolve stays in the draft, so the user can redefine or remove it.
- `VersionSnapshot::check(&Recipe, &LiveUnits, &TypeTrees)` parses every stored value and limit before the transaction. It is the snapshot check of `CreateVersion`. `Draft::check_for_approval` runs the same check.
- A snapshot item can also be `E-RCP-005` for a stored value that no longer fits its type. A stored limit that no longer fits gives its limit code (`E-RCP-006`, `E-RCP-017`, `E-RCP-018`, or `E-RCP-019`). A failed type resolution other than "not on the platform" gives its `PlantError`.
- A snapshot item `E-PLT-020` names a unit identifier that the machine does not know. The constraints of `data-model.md` section 3.12 and section 4.3 prevent this case, so the item only reports corrupt storage.
- A whole-draft check needs the tree of every parameter type. A type that `TypeTrees` does not hold gives `E-PLT-013`. A type whose resolution failed is skipped, and the snapshot check reports it.
- `SetParameterValue` and `ReplaceLimits` on a parameter whose type does not resolve give `E-PLT-013`. `RemoveParameter` and `UpdateParameterDefinition` still work.
- `RebindTypeRevision` also checks the S7 overlap, because a newer struct revision can be larger. An overlap gives `E-RCP-022`.
- I5 and I15 run in `Draft::add_parameter` and `Draft::set_definition`. A new parameter takes a `LiveUnit`, which only `LiveUnits::require` creates. A unit or a type tree of another machine gives `E-PLT-020` or `E-PLT-013` through `DraftError` (ADR-0027).
- I20 runs in `Draft::check_s7_layout(&TypeTrees)`.

Value and text rules:

- `ParameterValue::parse` builds a value from a request. `ParameterValue::check(&TypeTree, &[ProcessLimit])` checks it against its limits (I7). `reliability.md` section 6 uses these names.
- A value records the root type of its tree. A check against another tree fails.
- A number with a fraction is outside the range of an integer type and gives `E-RCP-005`.
- A `STRING` character outside Latin-1 has no byte in the target encoding and gives `E-RCP-005`.
- The range of `rust_decimal::Decimal` lies inside the IEEE 754 ranges of `REAL` and `LREAL`. A unit test proves this, and the code has no other range check for these types.
- A line end in a multi-line text is LF or CRLF. A lone CR is a control character.

## Consequences

### Positive

- Only a checked draft can become `Approved`, and the type system enforces it.
- A catalog or range change does not lock a draft. The snapshot check reports the problem.
- `domain-model.md`, `reliability.md`, and the code use the same names.

### Negative

- `Recipe::approve` has a different signature than the earlier reference in `domain-model.md` section 6.
- The use case calls two functions for an approve: `check_for_approval` and `approve`.

### Risks and mitigations

- Risk: `Draft::load` accepts a corrupt stored row. Mitigation: the constraints of `data-model.md` sections 3.12 and 3.13 hold the shape. The snapshot check parses every row before an approval.

## Related

- `domain-model.md` sections 2.2, 3, 4, 5.2, 6
- `reliability.md` sections 4.3, 4.4, 6, 7.4
- `api.md` sections 2.5, 4
- `data-model.md` section 6
- ADR-0008 (row version), ADR-0016 (aggregates), ADR-0021 (rules of `DeleteRecipe`), ADR-0022 (approval), ADR-0027 (error lists)
