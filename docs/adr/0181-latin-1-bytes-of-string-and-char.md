# ADR-0181: Latin-1 bytes of STRING and CHAR in the transfer

Status: Accepted. Amended by ADR-0202 (item (b): a `Utf8` machine gets the UTF-8 bytes of each `STRING` character)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ05 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0137 (h) says that each `STRING` character is one Latin-1 byte. `domain-model.md` §2.1 gives a `CHAR` value in `U+0001..=U+00FF`.
- The Siemens reference file says that the codes above `16#7F` depend on the code page and the interface (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:419` and `:450`).
- WinCC Unified uses Windows-1252 for `STRING` on an S7-1200 (review OQ05, Codex XQ08). This is a fact of one interface, not of the PLC.
- `api.md` §6 does not tell the transfer client how to write a `STRING` or `CHAR` character into the PLC.

The project owner decided on 2026-11-17: keep Latin-1, and state the encoding in `api.md` §6.

## Decision drivers

- The export document is the interface to the transfer client (R03, ADR-0019).
- A value in the PLC equals the approved value (R12).
- No change of the stored data and no change of the hash.

## Considered options

### Option A: Keep Latin-1 and state it in `api.md` §6

- Pros: no code change. One byte for each character, as the length rule counts it.
- Cons: a plant with a Windows-1252 HMI shows other glyphs for `0x80..=0x9F`.

### Option B: A code page for each machine

- Pros: each plant can select its code page.
- Cons: a new setting, a new rule set for the value check, and a new field of the export document.

## Decision

The project uses **Option A**.

- (a) A `STRING` or `CHAR` value is text of Unicode scalars in `U+0001..=U+00FF` (ADR-0137). This rule does not change.
- (b) The transfer client writes each `STRING` or `CHAR` character as one byte. The byte value is the scalar value of the character (ISO 8859-1).
- (c) `api.md` §6.1 states item (b) in the row of `parameters[].value`.
- (d) The schema files, the hashed body, and `content_sha256` do not change.

## Consequences

### Positive

- A transfer client knows the bytes of each `STRING` and `CHAR` value.
- No code change and no migration.

### Negative

- A user cannot enter a Windows-1252 character such as "€" (byte `0x80`).

### Risks and mitigations

- Risk: an HMI of the plant shows a byte in `0x80..=0x9F` with another glyph. Mitigation: a new ADR can add Option B when a plant needs it.

## Related

- ADR-0019 (export contract), ADR-0137 (text rules)
- `api.md` §6.1, `domain-model.md` §2.1
- `overview.md` §7: R03, R11, R12
