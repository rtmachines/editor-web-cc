# ADR-0184: End of DATE on S7 after TIA V21

Status: Accepted. Amends ADR-0114 (a note on the TIA V21 end of `DATE`)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ10 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0114 (a) ends `DATE` on `S7_1200` and `S7_1500` at 2168-12-31. Its source is the Siemens comparison list 02/2025.
- The Siemens reference file says that `FF FF` means 2169-06-06 in the current TIA V21 help. It also says that older S7-300 and classic S7-1200 manuals give 2168-12-31 (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:324` and `:326`).
- The domain range lies inside both Siemens ranges. The domain never accepts a date that a target refuses.

The project owner decided on 2026-11-17: keep the end of ADR-0114.

## Decision drivers

- An exported value fits every target of the platform (R03, R12).
- No change of a range without a need.

## Considered options

### Option A: Keep 2168-12-31

- Pros: the end fits every Siemens source.
- Cons: the domain refuses 157 days that a TIA V21 target accepts.

### Option B: Widen the end to 2169-06-06

- Pros: the TIA V21 range.
- Cons: an older target can refuse a date after 2168-12-31.

## Decision

The project uses **Option A**.

- (a) `DATE` on `S7_300`, `S7_1200`, and `S7_1500` keeps the end of ADR-0114 (a) and `domain-model.md` §2.1.
- (b) ADR-0114 gets an "Amended by" note. The note records that TIA V21 gives 2169-06-06 and that the domain keeps the safe end.
- (c) No code, no test, and no document of `docs/architecture/` changes.

## Consequences

### Positive

- The range stays safe for every S7 target.

### Negative

- A user cannot enter a date from 2169-01-01 to 2169-06-06.

### Risks and mitigations

- Risk: a later ADR widens the end with only the TIA V21 source. Mitigation: item (b) records why the end is lower.

## Related

- ADR-0114 (end of `DATE` on S7), ADR-0115 (ranges narrower than the vendor range)
- `domain-model.md` §2.1
- `overview.md` §7: R03, R12
