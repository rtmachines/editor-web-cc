# ADR-0117: Significant digits of REAL and LREAL

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

`reliability.md` section 4.4 checks the range of a `REAL` or `LREAL` number. It does not check the precision. A `REAL` leaf accepts up to 28 significant digits, the limit of `rust_decimal::Decimal` (ADR-0028).

The Siemens "Programming Guideline for S7-1200/1500" (V1.6, 12/2018, entry ID 81318674) has Table 2-10 in section 2.8.1. The table gives `REAL` a precision of 7 places and `LREAL` 15 places.

The export keeps the decimal text unchanged. The transfer client stores the nearest `REAL`. For example, `16777217` becomes `16777216`. A value at a limit can then be outside the limit by one unit of the last place.

## Decision drivers

- A value that the user sees is the value that the PLC gets (R12, R31).
- The domain checks all external input and never rounds a value (R38, `reliability.md` section 4.4).
- A rule of the domain gives one error code and a clear message (`reliability.md` section 1.2).

## Considered options

### Option A: Reject a number with too many significant digits

- Pros: the user sees the problem before the export. No value changes without the user.
- Cons: a new error code. The domain rejects a number that the user typed.

### Option B: Show a warning and keep the number

- Pros: the domain does not reject a number for its digits.
- Cons: the export still holds a number that the PLC changes. A warning needs a new UI state.

### Option C: Round to the precision of the type

- Pros: no rejected input.
- Cons: the stored value differs from the typed value. This breaks the rule that the domain never rounds.

## Decision

The project uses **Option A**.

- (a) On every platform, a `REAL` number has at most 7 significant digits, and an `LREAL` number at most 15.
- (b) The count of significant digits runs from the first to the last digit that is not zero. The 28-digit rule of `reliability.md` section 4.4 uses the same count. `1000000000` has 1 significant digit. `0.00012345` has 5. Zero has none.
- (c) The rule applies to a value leaf and to the `min` and `max` of a limit on such a leaf.
- (d) A number with more digits gives the new code `E-RCP-055`. The variant names the tag, the leaf path, the type, and the largest count of digits. The domain does not round.
- (e) The rule applies to a request and to the stored check of the snapshot (ADR-0028). At approval, a stored number with more digits gives the item `E-RCP-055` in `E-RCP-020`. `CreateVersion` copies such a number (ADR-0120 (g)).
- (f) The range rule of ADR-0028 and ADR-0115 does not change. A number outside the range of `Decimal` still gives `E-RCP-005`.

## Consequences

### Positive

- A `REAL` or `LREAL` value that the domain accepts has a precision that the type can hold.
- A limit and a value use the same precision.

### Negative

- A user cannot store a `REAL` number such as `16777217`.
- A stored draft with such a number cannot be approved until the user changes it.

### Risks and mitigations

- Risk: a decimal number of 7 digits does not always give back the same 7 digits after a conversion to IEEE 754 single precision. Only 6 digits always do. Mitigation: the owner chose the Siemens value of 7. A later ADR can change the count.
- Risk: an approved version holds such a number. Approved versions are immutable (ADR-0009). Mitigation: ADR-0118 shows the number with a field line, and ADR-0119 sets `loadable` to `false` at export.

## Related

- ADR-0028 (range of `REAL` and `LREAL`), ADR-0115 (ranges narrower than the vendor range), ADR-0009 (immutable snapshot)
- `reliability.md` section 4.4, `domain-model.md` section 2.1 (`LeafClass`)
- `overview.md` section 7: R12, R31, R38
