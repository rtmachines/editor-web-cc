# ADR-0010: A recipe belongs to exactly one machine

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft describes "recipes for an industrial production line" and a line made of machines with units. It says: "Each machine has one PLC controller" and "Machines can use different PLC types". It does not say whether a recipe spans the line or one machine.

A parameter has a PLC type that must exist on a platform, and for S7-300 an address in a data block. Both are properties of one controller.

## Decision drivers

- Type availability and address rules depend on one platform (I5, I6).
- The export document goes to one PLC (ADR-0002).
- The database must be able to enforce "S7-300 parameter has an address" without application code.
- Keep the first release simple (R34).

## Considered options

### Option A: Recipe per machine

- Pros: one platform per recipe, constraints are local, export is one document per PLC, approval is per machine.
- Cons: a line-wide product change means one approval per machine.

### Option B: Recipe per line, parameters on units of many machines

- Pros: one approval for the whole line.
- Cons: mixed platforms inside one version. Address rules per parameter must look up the machine. Export must be split per PLC. A conflict on one machine blocks the whole line.

## Decision

We use Option A.

- `recipe.machine_id` is `NOT NULL`. A recipe cannot move to another machine.
- `machine.plc_platform` is propagated down with composite foreign keys: `recipe(machine_id, plc_platform)` references `machine(id, plc_platform)`. `recipe_version(recipe_id, plc_platform)` references `recipe(id, plc_platform)`. `parameter(recipe_version_id, plc_platform)` references `recipe_version(id, plc_platform)`. This lets a `CHECK` on `parameter` enforce the S7-300 address rule (I5).
- A `Parameter.unit` must belong to the machine of the recipe. The trigger function `enforce_unit_machine` on `parameter` enforces it (`data-model.md` section 4.3, invariant I15).
- `machine.plc_platform` cannot change after the first recipe exists. A trigger rejects the update.
- A line-level "recipe set" that groups one recipe per machine is deferred. It is a new aggregate on top of this model and does not change it.

## Consequences

### Positive

- Every platform rule is a local constraint.
- Export is one document per machine with no split logic.
- The Recipe Management context needs only `MachineId`, `UnitId`, and `PlcPlatform` from the Plant Model.

### Negative

- A product that runs on five machines has five recipes and five approvals.
- The `plc_platform` column is repeated in three tables.

### Risks and mitigations

- Risk: users want one "approve the line" action. Mitigation: the deferred recipe set aggregate can run five `ApproveVersion` commands in one transaction. Record it in a new ADR when needed.
- Risk: the repeated `plc_platform` column drifts. Mitigation: the composite foreign keys make drift impossible. The column has no independent write path.

## Related

- ADR-0006 (platform availability of types)
- ADR-0009 (snapshot per version)
- `domain-model.md` section 1, invariants I5, I15
- `data-model.md` (composite foreign keys)
