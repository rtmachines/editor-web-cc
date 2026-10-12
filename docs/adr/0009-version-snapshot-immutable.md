# ADR-0009: A recipe version is a full immutable snapshot

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "A change to a parameter type, process limit, or UDT structure creates a new recipe version." It also says: "A recipe can have no more than one approved version at a time." The draft does not say where parameter definitions live: with the machine, or with the version.

This is the central structural decision. It determines what an `Approved` version means and what the export contains.

## Decision drivers

- An approved version must be reproducible later: same types, same limits, same values.
- The draft text ties type, limit, and UDT changes to a new version. That is only possible when the version owns them.
- Audit `before`/`after` must be small and local (ADR-0005).
- The export must not depend on current master data (ADR-0002).

## Considered options

### Option A: Version is a full snapshot

Each version owns its `parameter` rows: definition (unit, tag, name, type revision, address), limits, translations, and value. A new version is a copy of a base version plus edits.

- Pros: an approved version is self-contained and immutable. The draft rule maps one to one. Export reads one version. History is complete without joins to master data.
- Cons: parameter rows are duplicated per version. A definition change for all recipes of a machine must be applied to each recipe.

### Option B: Definitions as master data of the machine, versions hold only values

- Pros: no duplication, one place to change a definition.
- Cons: a definition change must create new versions of every recipe of the machine at once, or it silently changes approved versions. Either way the draft rule is hard to enforce. Export must join to the master data as it was at approval time, which needs master data versioning anyway.

## Decision

We use Option A.

- `parameter`, `parameter_limit`, and `parameter_translation` rows belong to one `recipe_version`.
- `Parameter.key` is a stable identity across versions (N4). `CreateVersion` copies every row and keeps `key`.
- A `plc_type` row is immutable. A parameter references one revision. A UDT change is a new revision (ADR-0006). To use the new revision, the engineer edits the parameter in a `Draft`.
- A `Draft` is the only mutable version. The domain returns `Err(VersionNotEditable)` for any edit to an `Approved` or `Retired` version (I3, I4).
- A trigger on `parameter`, `parameter_limit`, and `parameter_translation` rejects `INSERT`, `UPDATE`, and `DELETE` when the version status is not `draft`. The trigger is the second line of defence.
- At most one `Draft` per recipe (Assumption A7).
- Values are part of the snapshot (Assumption A11).

## Consequences

### Positive

- The draft rule "change creates a new version" is the only way to change anything.
- Export, audit, and history all read one version and nothing else.
- Immutability is enforced in two layers: domain and database.

### Negative

- Row count grows with versions times parameters. At the expected scale (Assumption A19) this is small.
- A machine-wide definition change (for example a renamed unit) must be repeated per recipe. A future bulk action can do this. It would still create one new draft per recipe.

### Risks and mitigations

- Risk: a copy of a large version is slow. Mitigation: `INSERT ... SELECT` in one statement per table, inside one transaction.
- Risk: a developer bypasses the domain and updates an approved row with SQL. Mitigation: the trigger rejects it. Only the migration role can disable the trigger.

## Related

- ADR-0005 (audit per version row)
- ADR-0006 (immutable type revisions)
- ADR-0008 (the version is the lock unit)
- `domain-model.md` section 2.2, invariants I3, I4
- `data-model.md` (snapshot tables, trigger)
