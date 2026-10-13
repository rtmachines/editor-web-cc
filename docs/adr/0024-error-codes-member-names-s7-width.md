# ADR-0024: Error codes for duplicate member names and the S7 address width

Status: Accepted
Date: 2026-10-13
Deciders: project owner

## Context

The plan review of step P2.1 found two domain rules without an error code in `reliability.md` section 1.2:

- `domain-model.md` section 2.1 says that the width of an S7 address MUST match the type. `data-model.md` section 3.12 says that the domain checks the width letter against the type size. No code reports a mismatch.
- `data-model.md` section 3.4 holds the constraint `plc_type_member_name_unique`. No domain error reports two members with one name. The duplicate mapping of `reliability.md` section 1.1 does not name this constraint, so the database result is `E-DB-006`.

A value object must be valid by construction (`reliability.md` section 4.3). The constructor of a struct type therefore must reject a duplicate member name, and it needs a code for it.

## Decision drivers

- Each rule of the domain has one stable code (R36).
- The message names the value that the user must change (R36, R39).
- The S7 checks of a parameter are already variants of `RecipeError` with the field `tag`.

## Considered options

### Option A: Two new codes

- Pros: each rule gets a precise message. The codes follow the owner of the existing rules.
- Cons: two new rows in the registry and in the error lists.

### Option B: Reuse existing codes

`E-RCP-023 S7AddressMisaligned` for the width, and `E-DB-006` for the duplicate member name.

- Pros: no new code.
- Cons: the message of `E-RCP-023` speaks about an even byte, not about the width. `E-DB-006` does not name the member. A user cannot see which field to change.

### Option C: Report both as a malformed field (`E-WEB-008`)

- Pros: no new code.
- Cons: the HTTP layer checks only the shape of a request (`reliability.md` section 4.1). The width rule needs the resolved type tree, which only the domain has.

## Decision

The project uses **Option A**.

- `E-PLT-021 PlantError::DuplicateMemberName { type_name, member }`, status 422. `DefineUdt` returns it. The domain checks the rule before the insert. `plc_type_member_name_unique` stays the second line, and `recipe-app` maps it to the same variant.
- `E-RCP-054 RecipeError::S7AddressWidthMismatch { tag, address, width }`, status 422. `AddParameter` and `UpdateParameterDefinition` return it. `width` is the width letter that the type needs.
- `E-PLT-021` is the next free number of `E-PLT-001..029`. `E-RCP-054` is the next free number of `E-RCP-050..069`.

## Consequences

### Positive

- A user sees the member name or the expected width in the message.
- The constructor of a struct type can reject every invalid member list.

### Negative

- `reliability.md`, `domain-model.md`, `data-model.md`, and `api.md` get new entries in their error lists.

### Risks and mitigations

- Risk: a document lists one code and another document misses it. Mitigation: the test of the registry compares the rows of `reliability.md` section 1.2 with the Rust variants.

## Related

- `reliability.md` sections 1.1, 1.2
- `domain-model.md` sections 2.1, 5.1, 5.2
- `data-model.md` sections 3.4, 3.12, 6
- `api.md` sections 2.3, 2.6
- ADR-0017 (S7 layout), ADR-0025 (parse reasons)
