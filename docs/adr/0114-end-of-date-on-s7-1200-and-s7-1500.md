# ADR-0114: End of DATE on S7-1200 and S7-1500

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

ADR-0026 gives `DATE` on `S7_1200` and `S7_1500` the range 1990-01-01 to 2169-06-06. It gives `DATE` on `S7_300` the range 1990-01-01 to 2168-12-31. The base-unit table of `domain-model.md` section 2.1 repeats these rows. The code holds the count range `7_305..=72_840` for `S7_1200` and `S7_1500`.

The Siemens reference manual "SIMATIC S7-300/S7-1200/S7-1200 G2/S7-1500 Comparison list for programming languages" (edition 02/2025, entry ID 109986777) gives one `DATE` range for every S7 family. The section "Overview of data types" gives "01.01.1990 ... 31.12.2168".

With the current range, the domain accepts the days 2169-01-01 to 2169-06-06 on `S7_1200` and `S7_1500`. Such a value reaches the export outside the range of the PLC.

## Decision drivers

- The range must be the range of the PLC, so that an exported value loads into the controller (R03, R12).
- A limit and a value check use the same range in every place (R09, R38).
- Every range in code has a row in a normative document (ADR-0026).

## Considered options

### Option A: End `DATE` at 2168-12-31 on every S7 platform

- Pros: the range equals the current Siemens manual. All three S7 platforms use one range.
- Cons: a stored value or limit from 2169-01-01 to 2169-06-06 no longer fits its type.

### Option B: Keep 2169-06-06 on S7-1200 and S7-1500

- Pros: no stored value changes its state.
- Cons: the domain accepts a date that the Siemens manual does not permit.

## Decision

The project uses **Option A**.

- (a) `DATE` on `S7_1200` and `S7_1500` has the range 1990-01-01 to 2168-12-31. This replaces the row of these platforms in the table of ADR-0026.
- (b) The count of days since 1970-01-01 is `7_305..=72_683`. The day 2168-12-31 is the count 72 683. The day 2169-06-06 was the count 72 840.
- (c) All three S7 platforms now share one `DATE` range. The rows of `CodesysIec` and `TwinCat3` do not change.
- (d) A value outside the range gives `E-RCP-005`. A limit outside the range gives `E-RCP-006` (`domain-model.md` section 2.1, `IecLiteral`).
- (e) A stored value or limit from 72 684 to 72 840 follows the rules of ADR-0028 for a range change. `Draft::load` trusts it, and the snapshot check of the approval reports it. `CreateVersion` copies it (ADR-0120 (g)).

## Consequences

### Positive

- The `DATE` range of every S7 platform equals the Siemens manual.
- The range table has one `DATE` row for S7 in place of two.

### Negative

- A draft with a date from 2169-01-01 to 2169-06-06 cannot be approved until the user changes the value or the limit.
- A new draft from a base version with such a date shows a field line. The line stays until the user changes the date (ADR-0118, ADR-0120 (g)).

### Risks and mitigations

- Risk: an approved version holds such a date. Approved versions are immutable (ADR-0009). Mitigation: ADR-0118 shows the date with a field line, and ADR-0119 sets `loadable` to `false` at export.

## Related

- ADR-0026 (ranges of `DATE` and `DATE_AND_TIME`), ADR-0028 (range change of a stored value), ADR-0009 (immutable snapshot)
- `domain-model.md` section 2.1 (base-unit table, `IecLiteral`)
- `overview.md` section 7: R03, R09, R12, R38. Section 9: A43
