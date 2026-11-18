# ADR-0188: DTL range equals the Siemens range

Status: Accepted. Amends ADR-0115 (the `DTL` row and its widening mitigation)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 found R06 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §3). R06 follows the obsolete baseline item F03.

These facts shape the decision:

- ADR-0115 lists `DTL` as a range narrower than the vendor range. It gives the vendor end 2554-12-31-23:59:59.999999999.
- The mitigation of ADR-0115 says that a new ADR widens the range of such a type.
- The S7-1200 manual update (SIOS 89851659) corrects the `DTL` end to 2262-04-11-23:47:16.854775807. The Siemens reference file agrees (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:398`).
- The domain range of `DTL` is 1970-01-01 to 2262-04-11-23:47:16.854775807 (`domain-model.md` §2.1). It equals the corrected Siemens range.

The project owner decided on 2026-11-17: correct the record with a short ADR.

## Decision drivers

- A record of a decision matches the vendor facts (R43).
- No later ADR widens a range past the PLC range (R03, R12).

## Considered options

### Option A: An "Amended by" note on ADR-0115 through this ADR

- Pros: ADR-0115 stays valid for its other rows.
- Cons: none.

### Option B: Supersede ADR-0115

- Pros: one current record.
- Cons: the other rows of ADR-0115 do not change. A full replacement copies them.

## Decision

The project uses **Option A**.

- (a) The `DTL` range of `domain-model.md` §2.1 equals the current Siemens range. It is not a restriction below the vendor range.
- (b) The `DTL` row leaves the list of narrower ranges of ADR-0115. The widening mitigation of ADR-0115 does not apply to `DTL`.
- (c) No code, no test, and no document of `docs/architecture/` changes.

## Consequences

### Positive

- A reader of ADR-0115 does not widen `DTL` past the PLC range.

### Negative

- None.

### Risks and mitigations

- Risk: a later Siemens manual widens `DTL` again. Mitigation: a new ADR records the new source.

## Related

- ADR-0115 (ranges narrower than the vendor range), ADR-0026 (time ranges)
- `domain-model.md` §2.1
- `overview.md` §7: R03, R12, R43
