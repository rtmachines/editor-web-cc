# Proposal

## Why

The PLC data type review of 2026-11-06 found that the value and limit checks of `recipe-domain` do not follow their rules. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 4 (WP1) groups seven findings:

- #2, #3, #7 (HIGH): `parse_decimal` refuses exact numbers inside the range of `Decimal`. Examples are `150e-29`, `1.` with 29 zeros, and `0e99999999999` (`reliability.md` §4.4).
- #4 (HIGH): a stored time limit skips the `S5TIME` step and the whole-day rule of `LDATE`. A stored value does not skip them (`domain-model.md` §2.1, ADR-0120 (b)).
- #17 (LOW): under the earlier rule set, a message shows a raw count instead of the IEC literal (`domain-model.md` §2.1, ADR-0120 (d)).
- #14, #15 (MEDIUM): no test pins the ends of the 12 integer types or the tier edges of the `S5TIME` steps.

The change satisfies R09, R11, R12, R36, and R38. It uses A43.

## What Changes

- `parse_decimal` reads the digits and the exponent of a JSON number into one integer and one scale. It removes trailing zeros before the range check of `Decimal`. A zero gives `0` before the exponent parse.
- The check of a stored time limit applies the count rules of `IecLiteral::from_count`, as the check of a stored value does.
- A message writes each time bound under the earlier rule set.
- New table tests pin the integer ends and the `S5TIME` tier edges.
- No document, ADR, error code, migration, or setting changes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: exact parse of a number, count rules of a stored time limit, literal of a bound in a message, integer ends, and `S5TIME` steps.

## Impact

- Code: `recipe-domain` only. The files are `recipe/value/decimal.rs`, `recipe/limit/parse.rs`, `recipe/limit/render.rs`, `recipe/limit/mod.rs`, and `recipe/diff/limits.rs`, with their tests.
- Tests: also in `recipe-app`.
- Stored data: the canonical text stays the text of `Decimal::normalize`. Thus no stored value, limit, or content hash changes.
- A stored time limit that breaks a count rule becomes a storage defect (ADR-0118 (d)). No write path can store such a limit.
- Message text: a count that only the earlier rule set accepts shows as a literal. No check result changes.

## Out of scope

- The findings of WP2 to WP7 of the fix plan.
- A change of `docs/architecture/` or a new ADR.
- A new error code.
- The vendor ranges of `LDATE` and `LDT` (#20, WP4).
