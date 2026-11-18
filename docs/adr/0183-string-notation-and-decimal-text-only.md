# ADR-0183: STRING(n) notation and decimal text only

Status: Accepted
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ07 and OQ09 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4). OQ09 continues the baseline item F12.

These facts shape the decision:

- TIA Portal writes a string length as `STRING[n]` (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:410`). The grammar of `TypeExpr` accepts only `STRING(n)` and `WSTRING(n)` (`domain-model.md` §2.1).
- TIA Portal and IEC 61131-3 accept based and typed literals such as `16#FF`, `2#1010`, and `REAL#1.0` (`:104`, `:216`).
- `domain-model.md` §2.1 says: "The text of a `Numeric` leaf is decimal text with a dot." `reliability.md` §4.4 defines the canonical decimal text.
- Each bit pattern of a numeric type has exactly one decimal text. A based literal thus adds no value that the decimal text cannot hold.
- No test names the refusal of a based or typed literal (review R03).

The project owner decided on 2026-11-17: no change.

## Decision drivers

- One text form for each value in a request, a page, and the export (R11, R38).
- The export is not affected by the notation of a type (`api.md` §6.1).

## Considered options

### Option A: Keep `STRING(n)` and the decimal text

- Pros: no change of the grammar, the form, or the export.
- Cons: a user who types the TIA form `STRING[20]` or `16#FF` gets an error.

### Option B: Accept `STRING[n]` and based literals as input

- Pros: the TIA forms work.
- Cons: two input forms for one value or type. ADR-0039 and the grammar change, and a page needs a rule for the shown form.

## Decision

The project uses **Option A**.

- (a) A `TypeExpr` accepts only `STRING(n)` and `WSTRING(n)`. `STRING[n]` gives `E-PLT-013`.
- (b) The text of a `Numeric` leaf is decimal text only. A based literal (`16#`, `8#`, `2#`), a typed literal (`BYTE#`, `REAL#`), and a digit separator `_` give `E-RCP-005`.
- (c) A test of `recipe-domain` pins item (b) for the `BYTE`, `WORD`, `DWORD`, `LWORD`, and `REAL` leaves (review R03). A test pins item (a).
- (d) This ADR closes the baseline item F12.

## Consequences

### Positive

- One text form for each value and each type.
- A test fails when a later change accepts a based literal for one class only.

### Negative

- A bit mask shows as a decimal number, which is hard to read.

### Risks and mitigations

- Risk: users need the hexadecimal form of a mask. Mitigation: a new ADR can add a second shown form of a bit-string leaf.

## Related

- ADR-0039 (one field for each leaf), ADR-0137 (text rules)
- `domain-model.md` §2.1 (`TypeExpr`, leaf texts), `reliability.md` §4.4
- `overview.md` §7: R11, R38
