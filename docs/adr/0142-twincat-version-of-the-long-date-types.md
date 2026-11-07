# ADR-0142: TwinCAT version of the long date types

Status: Accepted
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a platform fact that no document records. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #21.

These facts shape the decision:

- The type matrix of `data-model.md` §7 gives `LDATE`, `LTOD`, and `LDT` the value "yes" in the column `TwinCat3`. It names no TwinCAT version.
- Beckhoff says about `LDATE`, `LDATE_AND_TIME`, and `LTIME_OF_DAY` that TwinCAT version 3.1.4026.0 or later is required.
- The editor never communicates with a PLC (R03). It cannot read the TwinCAT version of a controller.
- A recipe with `LDT` for a controller with TwinCAT 3.1.4024 passes the approval. The controller cannot load it.

The project owner decided on 2026-11-06 (fix plan, question Q-G, option (1)).

## Decision drivers

- An exported recipe loads into the controller (R03, R11, R12).
- The plant has one production line and few machines (A25). A check by the operator is possible for each machine.
- No new machine setting without a need of the process.

## Considered options

### Option A: An Assumption and a check in the runbook

- Pros: no code change, no migration, and no new form.
- Cons: the editor does not stop a plant that skips the check.

### Option B: A machine attribute holds the runtime version

- Pros: the type catalog can hide the three types below 3.1.4026.0.
- Cons: a new machine setting, a new form, a migration, and a filter of the type catalog. A runtime update of the controller also needs an update of the attribute.

## Decision

The project uses **Option A**.

- (a) Assumption A46: every `TwinCat3` machine runs TwinCAT 3.1.4026.0 or later. `overview.md` §9 holds the row.
- (b) The runbook names the check of the TwinCAT version before a plant adds a `TwinCat3` machine (`docs/ops/runbook.md`).
- (c) The type catalog of `TwinCat3` keeps `LDATE`, `LTOD`, and `LDT`. The matrix of `data-model.md` §7 does not change.
- (d) The editor stores no runtime version and checks none.

## Consequences

### Positive

- No code change, no migration, and no new setting.
- A reader of `data-model.md` §7 knows the version that the three types need.

### Negative

- A plant that skips the check can approve a recipe that an older controller cannot load.

### Risks and mitigations

- Risk: a plant adds a `TwinCat3` machine with an older runtime. Mitigation: the runbook check. A new ADR can add the attribute of Option B.

## Related

- `data-model.md` §7 (type matrix), `docs/ops/runbook.md`
- `overview.md` §7: R03, R11, R12. §9: A25, A46
