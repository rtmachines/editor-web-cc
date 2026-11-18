# ADR-0182: S7-1200 is the classic family from FW 4.1

Status: Accepted
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ06 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4). OQ06 continues the baseline item F14.

These facts shape the decision:

- The Siemens reference file gives separate columns for the classic S7-1200 and the S7-1200 G2. It says that the two families "must be treated as separate families" (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:75`).
- The S7-1200 G2 supports `LWORD`, `LINT`, `ULINT`, `LTIME`, `LTOD`, and `LDT`. The matrix of `data-model.md` §7 gives "no" for these types on `S7_1200`.
- The same file names FW 4.1 with TIA V13 SP1 as a supported combination for `WCHAR` and `WSTRING` on the classic S7-1200 (`:413`). The status of FW 4.0 is not known.
- No document says which family `S7_1200` means.

The project owner decided on 2026-11-17.

## Decision drivers

- The type matrix holds only types that the PLC has (R03, A3).
- A rule of the platform needs a known CPU family.

## Considered options

### Option A: An Assumption for the classic family from FW 4.1

- Pros: no code change. The matrix stays valid.
- Cons: a plant with an S7-1200 G2 cannot use the 64-bit types.

### Option B: A new platform for the S7-1200 G2

- Pros: the G2 gets its own type set.
- Cons: a new enum variant, a seed migration, and new rules. The plant does not use a G2 now.

## Decision

The project uses **Option A**.

- (a) New Assumption A49: `S7_1200` is the classic S7-1200 family with firmware 4.1 or later. The editor does not check the firmware.
- (b) The S7-1200 G2 is not in scope. A later change adds a platform when the plant uses a G2.
- (c) The matrix of `data-model.md` §7 does not change. A note there names A49.
- (d) This ADR closes the baseline item F14.

## Consequences

### Positive

- The type set of `S7_1200` has a defined CPU family and firmware.
- No code change and no migration.

### Negative

- A machine with an S7-1200 G2 cannot use the 64-bit types of the G2.

### Risks and mitigations

- Risk: a plant adds a machine with a classic S7-1200 and firmware before 4.1. Then `WCHAR` and `WSTRING` can fail in the PLC. Mitigation: A49 names the firmware, and the plant checks it before it adds the machine.
- Risk: the classic S7-1200 with FW 2.2 has no `LREAL` with an address (review OQ06, part NQ03). The later firmware boundary is not known. Mitigation: A49 starts at FW 4.1. A TIA measurement can add an address rule with a new ADR.

## Related

- ADR-0006 (type catalog), ADR-0140 (data block number on `S7_1200`)
- `data-model.md` §7, `domain-model.md` §2.1 (`PlcPlatform`)
- `overview.md` §7: R03, R15. §9: A3, A49
