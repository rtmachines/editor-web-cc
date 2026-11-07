# ADR-0115: Ranges narrower than the vendor range

Status: Accepted. Amended by ADR-0139 (a fourth range: `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` start at 1970-01-01). Amended by ADR-0141 (a fifth range: `STRING(n)` and `WSTRING(n)` on `CodesysIec` and `TwinCat3` stay `1..=255`)
Date: 2026-11-03
Deciders: project owner

## Context

Three ranges of the domain are narrower than the ranges of the Siemens manuals. The source is the reference manual "SIMATIC S7-300/S7-1200/S7-1200 G2/S7-1500 Comparison list for programming languages" (edition 02/2025, entry ID 109986777), section "Overview of data types".

| Type | Domain range | Siemens range |
|---|---|---|
| `DATE_AND_TIME` | 1990-01-01-00:00:00 to 2089-12-31-23:59:59, whole seconds (A43, ADR-0026) | to 2089-12-31-23:59:59.999, with milliseconds |
| `DTL` | 1970-01-01 to 2262-04-11, a signed 64-bit count of nanoseconds (`domain-model.md` section 2.1) | to 2554-12-31-23:59:59.999999999 |
| `REAL`, `LREAL` | The range of `rust_decimal::Decimal`, about ±7.9e28, at most 28 significant digits (ADR-0028, `reliability.md` section 4.4) | The IEEE 754 ranges, to about ±3.4e38 and ±1.8e308 |

No normative document says if these restrictions are deliberate. A reader can take them for errors.

## Decision drivers

- A recipe holds process values. It does not need the extreme values of a type (R12).
- A stored and hashed value is exact decimal text with no `f64` round trip (`reliability.md` section 4.4, ADR-0019).
- A change of a base unit changes the export document and the content hash (`api.md` section 6.1).

## Considered options

### Option A: Keep the three ranges and record them as deliberate

- Pros: no change of the base units, the export, or the stored values.
- Cons: the domain rejects some values that the PLC accepts.

### Option B: Widen each range to the Siemens range

- Pros: every value that the PLC accepts is valid.
- Cons: `DATE_AND_TIME` needs a new base unit and a new export unit. `DTL` needs a count above the signed 64-bit range. `REAL` and `LREAL` need a number type other than `Decimal`.

## Decision

The project uses **Option A**. Each range stays as it is, and this ADR records it as a deliberate restriction.

- (a) `DATE_AND_TIME` counts whole seconds since 1970-01-01 (A43). A literal with a fraction of a second gives `E-RCP-005`. The S7 range ends at 2089-12-31-23:59:59.
- (b) `DTL` ends at 2262-04-11-23:47:16.854775807, the largest signed 64-bit count of nanoseconds. Its count is a signed 64-bit count of nanoseconds since 1970-01-01, the same range as `LDT` on `S7_1500`. A later value gives `E-RCP-005`.
- (c) `REAL` and `LREAL` keep the range of `rust_decimal::Decimal`. A number outside this range, or with more than 28 significant digits, gives `E-RCP-005`. ADR-0117 adds a limit of significant digits for each type.

## Consequences

### Positive

- The base units of A43, the export document, and the content hash do not change.
- `DTL` and `LDT` share one count range on `S7_1500`.
- Exact decimal arithmetic stays the rule for every numeric leaf.

### Negative

- A recipe cannot hold the milliseconds of a `DATE_AND_TIME` value. A user who needs them uses `LDT` or `DTL`.
- A recipe cannot hold a `DTL` value after 2262-04-11.
- A recipe cannot hold a `REAL` or `LREAL` value above about 7.9e28 or below 1e-28 in magnitude, except zero.

### Risks and mitigations

- Risk: a process needs one of these values. Mitigation: a new ADR widens the range of that type. The change of a base unit then also changes `api.md` section 6.1.

## Related

- ADR-0026 (ranges of `DATE` and `DATE_AND_TIME`), ADR-0028 (range of `REAL` and `LREAL`), ADR-0117 (significant digits of `REAL` and `LREAL`)
- `domain-model.md` section 2.1 (base-unit table), `reliability.md` section 4.4, `api.md` section 6.1
- `overview.md` section 7: R12, R38. Section 9: A43
