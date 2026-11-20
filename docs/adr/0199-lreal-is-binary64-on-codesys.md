# ADR-0199: LREAL is binary64 on CODESYS

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ01 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- The reference file says that some CODESYS targets compile `LREAL` as `REAL` (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:109`).
- The type matrix of `data-model.md` §7 gives `LREAL` the value "yes" on `CodesysIec` without a condition.
- ADR-0117 (a) permits 15 significant digits for an `LREAL` number on every platform.
- The value 16777217 has 8 digits. It is exact in binary64. A conversion to binary32 gives 16777216.
- The editor never communicates with a PLC (R03). It cannot read the compiler settings of a target.

The project owner decided on 2026-11-19: add an Assumption and a check in the runbook, as A46 does for TwinCAT.

## Decision drivers

- An exported value equals the value in the PLC (R03, R11, R12).
- The plant has one production line and few machines (A25). A check by the operator is possible for each machine.
- No new machine setting without a need of the process.

## Considered options

### Option A: An Assumption and a check in the runbook

- Pros: no code change, no migration, and no new form.
- Cons: the editor does not stop a plant that skips the check.

### Option B: A machine attribute "LREAL is binary32"

- Pros: the domain can apply the digit rule of `REAL` to `LREAL` on such a machine.
- Cons: a new machine setting, a new form, a migration, and a new branch of the digit rule.

### Option C: 7 digits for `LREAL` on every `CodesysIec` machine

- Pros: no new setting.
- Cons: a plant with a binary64 target loses 8 digits without a reason.

## Decision

The project uses **Option A**.

- (a) Assumption A51: every `CodesysIec` machine compiles `LREAL` as an eight-byte binary64 number. `overview.md` §9 holds the row.
- (b) The runbook names the check of the `LREAL` size before a plant adds a `CodesysIec` machine (`docs/ops/runbook.md`).
- (c) ADR-0117 does not change. An `LREAL` number has at most 15 significant digits on every platform.
- (d) The type matrix of `data-model.md` §7 does not change. The editor stores no compiler setting and checks none.

## Consequences

### Positive

- No code change, no migration, and no new setting.
- A reader of `overview.md` §9 knows the `LREAL` size that the 15 digits need.

### Negative

- A plant that skips the check can approve an `LREAL` value that a binary32 target changes.

### Risks and mitigations

- Risk: a plant adds a `CodesysIec` machine whose target compiles `LREAL` as `REAL`. Mitigation: the runbook check. A new ADR can add the attribute of Option B.

## Related

- ADR-0117 (significant digits), ADR-0142 (the TwinCAT twin of this pattern)
- `data-model.md` §7 (type matrix), `docs/ops/runbook.md`
- `overview.md` §7: R03, R11, R12. §9: A25, A51
