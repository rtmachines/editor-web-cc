# ADR-0200: CODESYS version of the long date types

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ04 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- The reference file says that CODESYS V3.5 SP16 introduced `LDATE`, `LDATE_AND_TIME`, and `LTIME_OF_DAY` (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:29`).
- The type matrix of `data-model.md` §7 gives `LDATE`, `LTOD`, and `LDT` the value "yes" on `CodesysIec`. It names no CODESYS version.
- A46 and ADR-0142 fix the TwinCAT version of the three types. No document fixes the CODESYS version.
- The editor never communicates with a PLC (R03). It cannot read the CODESYS version of a controller.
- A recipe with `LDT` for a controller older than SP16 passes the approval. The controller cannot load it.

The project owner decided on 2026-11-19: add an Assumption and a check in the runbook, as ADR-0142 does for TwinCAT.

## Decision drivers

- An exported recipe loads into the controller (R03, R11, R12).
- The plant has one production line and few machines (A25). A check by the operator is possible for each machine.
- No new machine setting without a need of the process.

## Considered options

### Option A: An Assumption and a check in the runbook

- Pros: no code change, no migration, and no new form.
- Cons: the editor does not stop a plant that skips the check.

### Option B: A machine attribute holds the CODESYS version

- Pros: the type catalog can hide the three types below SP16.
- Cons: a new machine setting, a new form, a migration, and a filter of the type catalog.

## Decision

The project uses **Option A**.

- (a) Assumption A52: every `CodesysIec` machine runs CODESYS V3.5 SP16 or later. `overview.md` §9 holds the row.
- (b) The runbook names the check of the CODESYS version before a plant adds a `CodesysIec` machine (`docs/ops/runbook.md`).
- (c) The type catalog of `CodesysIec` keeps `LDATE`, `LTOD`, and `LDT`. The matrix of `data-model.md` §7 does not change.
- (d) The editor stores no CODESYS version and checks none.

## Consequences

### Positive

- No code change, no migration, and no new setting.
- A reader of `data-model.md` §7 knows the CODESYS version that the three types need.

### Negative

- A plant that skips the check can approve a recipe that an older controller cannot load.

### Risks and mitigations

- Risk: a plant adds a `CodesysIec` machine with an older CODESYS version. Mitigation: the runbook check. A new ADR can add the attribute of Option B.

## Related

- ADR-0142 (the TwinCAT version of the same types)
- `data-model.md` §7 (type matrix), `docs/ops/runbook.md`
- `overview.md` §7: R03, R11, R12. §9: A25, A46, A52
