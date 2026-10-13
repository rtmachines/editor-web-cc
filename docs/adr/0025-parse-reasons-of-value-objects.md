# ADR-0025: Parse reasons of value objects without an own error code

Status: Accepted
Date: 2026-10-13
Deciders: project owner

## Context

`reliability.md` section 4.3 gives every value object one constructor `parse(input) -> Result<Self, Error>`. Some value objects have their own row in section 1.2, for example `UnitPath` (`E-PLT-017`, `E-PLT-002`) and `TypeExpr` (`E-PLT-013`). Other value objects have no row:

- `MachineCode`, `PlcPlatform`, `Text`, `TypeName`, and `MemberName`,
- `S7Address`, `LeafPath`, and `IecLiteral`.

`api.md` section 1 gives `E-WEB-008` to a malformed request field and `E-WEB-005` to a path value that names no resource. `domain-model.md` section 2.1 gives `E-RCP-005` or `E-RCP-006` to a bad `IecLiteral`. These codes carry fields that only the caller knows: the field name, the tag, or the leaf. The value object therefore cannot return them.

`reliability.md` section 1.1 says that every error type has `code()`, and its table names four error types of `recipe-domain`. A fifth error type with a code would duplicate the codes of the callers.

## Decision drivers

- Every code has one owner variant (R36).
- A value object is valid by construction (R38).
- The HTTP layer shows a reason in the UI language (`api.md` section 8).

## Considered options

### Option A: A reason enum as the error of `parse`

- Pros: the caller gets a typed reason and keeps the code of its context. No code gets a second owner. The web layer can map each reason to a localized text.
- Cons: the rule of `reliability.md` section 1.1 needs one more sentence.

### Option B: A new code for each value object

- Pros: every `parse` returns a coded error.
- Cons: the new codes duplicate `E-WEB-008` and `E-WEB-005`. A bad `IecLiteral` would get a code that differs from `E-RCP-005`.

### Option C: `Option<Self>` as the result of `parse`

- Pros: no new type.
- Cons: the reason is lost for the event log. The signature differs from `reliability.md` section 4.3.

## Decision

The project uses **Option A**.

- A value object with its own row in `reliability.md` section 1.2 returns the error type of that row.
- Every other value object returns a reason enum from `parse`. A reason is a field type, like the `reason` of `E-PLT-013` and of `E-WEB-008`. It is not an error type and has no `code()`. It never holds a secret.
- The caller wraps the reason into its coded error:
  - `recipe-web` uses `E-WEB-008` for a request field and `E-WEB-005` for a path value (`api.md` section 1).
  - `TypeExpr` uses `E-PLT-013` for a bad part of a type, for example a bad `TypeName`.
  - The value check of a parameter uses `E-RCP-005` for a bad value and `E-RCP-006` for a bad limit.

## Consequences

### Positive

- Each code keeps one owner. The registry test of `reliability.md` section 1.2 stays simple.
- The domain stays free of HTTP field names.

### Negative

- Each caller writes its own mapping of the reason.

### Risks and mitigations

- Risk: a caller drops the reason and shows a general message. Mitigation: the code review checks that each mapping keeps the reason in the message or in the event log.

## Related

- `reliability.md` sections 1.1, 1.2, 4.1, 4.3
- `api.md` sections 1, 8
- `domain-model.md` section 2.1 (`IecLiteral`)
- ADR-0024 (codes `E-PLT-021`, `E-RCP-054`)
