# ADR-0141: String length on CodesysIec and TwinCat3

Status: Accepted. Amends ADR-0115 (a fifth range narrower than the vendor range)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a range that no document records as deliberate. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #13.

These facts shape the decision:

- The `StringLength` table of `domain-model.md` §2.1 gives `STRING(n)` and `WSTRING(n)` the range `1..=255` on `CodesysIec` and `TwinCat3`. The code holds the same range (`crates/recipe-domain/src/plant/string_length.rs`, function `max_length`).
- A longer length gives `E-PLT-013`. The tests of `string_length/tests.rs` pin 255 and 256 on both platforms for both types.
- CODESYS says that the length of a string is not limited in principle. The 255 limit applies to the functions of the `Standard` library.
- Beckhoff says that TwinCAT does not limit the string length in principle. Some string functions process only the lengths 1 to 255.
- ADR-0115 records three ranges that are narrower than the Siemens range. ADR-0139 (c) adds a fourth range. Neither ADR names this range.
- The column `string_length` of `plc_type` permits up to 16 382 (`plc_type_string_length_range`).

The project owner decided on 2026-11-06 (fix plan, question Q-B, option (1)).

## Decision drivers

- A string value of a recipe works with the standard string functions of the PLC program (R11, R12).
- A rule of the domain that is narrower than the vendor range has a record (ADR-0115).
- The decision keeps every stored row, every export document, and every content hash.

## Considered options

### Option A: Keep 1 to 255 and record it as deliberate

- Pros: no code change. Every value fits the standard string functions of both vendors.
- Cons: the domain refuses a length that the PLC accepts.

### Option B: Allow 1 to 16 382

- Pros: the domain accepts more lengths that the PLC accepts.
- Cons: a value above 255 characters can fail in a standard string function of the PLC program. The code, the tests, and the `StringLength` table change.

## Decision

The project uses **Option A**.

- (a) On `CodesysIec` and `TwinCat3`, the length of a `STRING(n)` and of a `WSTRING(n)` stays `1..=255`.
- (b) The reason is the 255 limit of the standard string functions of CODESYS and TwinCAT 3. The vendors set no limit for the type itself.
- (c) The range is a deliberate narrowing of the vendor range, in the pattern of ADR-0115. This ADR adds it as a fifth range, after the three ranges of ADR-0115 and the fourth range of ADR-0139 (c).
- (d) A longer length still gives `E-PLT-013`. The code and the tests do not change.
- (e) The unit of the length stays the unit of ADR-0137 (h) and (i).

## Consequences

### Positive

- A reader of `domain-model.md` §2.1 knows that the range is deliberate.
- No stored row, export document, or content hash changes.

### Negative

- A recipe cannot hold a `STRING` or `WSTRING` value longer than 255 on `CodesysIec` and `TwinCat3`.

### Risks and mitigations

- Risk: a process needs a longer string. Mitigation: a new ADR widens the range with Option B. The column `string_length` already permits up to 16 382.

## Related

- ADR-0115 (ranges narrower than the vendor range), ADR-0137 (unit of a string length)
- `domain-model.md` §2.1 (`StringLength`), `data-model.md` §3.3 (`plc_type_string_length_range`)
- `overview.md` §7: R11, R12, R13
