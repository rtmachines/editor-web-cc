# ADR-0018: UDT revisions and the rebind of recipe drafts

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "A change to a parameter type, process limit, or UDT structure creates a new recipe version". ADR-0006 made UDT revisions immutable and ADR-0009 made versions snapshots. But the architecture review (F4) and the traceability review (Pass A, item 15) showed that no mechanism connected the two. `DefineUdt` inserted a new revision and nothing found the recipes that used the old one. A parameter that was re-bound by hand could lose a limit on a renamed member without an error. A UDT that embedded another UDT stayed on the old revision.

The owner decided that the system marks affected parameters and that the engineer rebinds them in a draft. The system does not create drafts on its own.

## Decision drivers

- R19: a UDT change must lead to a new recipe version, with a clear path for the engineer.
- A limit must never disappear without a decision (R09, R31).
- At most one live draft per recipe (A7). Automatic drafts would break this for hundreds of recipes.
- The export must say which revision each parameter uses (ADR-0019).

## Considered options

### Option A: Automatic draft for every affected recipe

- Pros: nothing is forgotten.
- Cons: hundreds of unattended drafts, conflicts with A7, no owner for each draft, the values of new members are unknown.

### Option B: Outdated marker plus an explicit rebind command in a draft

- Pros: the engineer decides per recipe, the rebind is audited, the limits are mapped with a rule.
- Cons: an old revision can stay in use until someone acts. The UI must make this visible.

### Option C: Mutable UDTs

- Pros: no revisions.
- Cons: an approved version would change under the engineer. Rejected in ADR-0006.

## Decision

The project uses **Option B**.

- `DefineUdt` with `supersedes` creates revision `n + 1` of the same name. The old revision stays immutable.
- A "where used" query lists every parameter, and every struct member and array element type, that references a given revision. The query is recursive over `plc_type_member` and `element_type_id`.
- A parameter is outdated when any `plc_type` in its resolved type tree has a successor revision. The tree holds the type itself, every member, and every element type, at any depth. The recipe list and the version page show this state. It is computed, not stored.
- `RebindTypeRevision { draft, from_type_id, to_type_id }` runs in a live draft only. `to_type_id` is a newer revision of the struct of `from_type_id` on the same machine. Otherwise the command returns `E-RCP-052 RebindTargetInvalid`.
- The command moves every parameter of the draft whose type uses `from_type_id`. A parameter uses it as the parameter type, or as the element type of an array type at any depth. `EnsureDerivedType` gives each new array type.
- The command maps values and limits by member name and by array index:
  - A member with the same name and a compatible primitive keeps its value and its limits.
  - A new member needs an explicit value in the command. The command fails with `E-RCP-028 RebindNeedsValue` otherwise.
  - A removed member that had a limit makes the command fail with `E-RCP-029 RebindDropsLimit`. The engineer removes the limit first, on purpose.
  - A member whose type changed is treated as new.
- The command checks every affected parameter first. It returns one `E-RCP-028` or `E-RCP-029` entry per failing parameter in one list, and it changes nothing.
- When a UDT `ST_Line` embeds `ST_Zone`, a new revision of `ST_Zone` does not change `ST_Line`. The engineer creates a new revision of `ST_Line` that references the new `ST_Zone`. A parameter of type `ST_Line` is outdated by the rule above, because its type tree holds the old `ST_Zone`. The "where used" query shows `ST_Line` as a user of the old `ST_Zone`. This is a cascade by explicit action, not by the system (Assumption A29).
- `ApproveVersion` returns `E-RCP-024 TypeRevisionOutdated` when a parameter of the draft is outdated, unless the request carries `allow_outdated = true`. The plant may run an old layout on purpose. The approve page lists the outdated parameters and offers the flag. The audit entry `VersionApproved` records the flag value (Assumption A29).
- The export lists every revision in `types[]` with its `revision` number (ADR-0019).

## Consequences

### Positive

- R19 has a mechanism, and every rebind is one audited draft command.
- A limit cannot vanish during a rebind.
- The engineer sees the scope of a UDT change before the change is used.

### Negative

- An outdated parameter can stay outdated for a long time. The UI must keep it visible.
- The "where used" query is recursive. On 10 000 recipes it needs the indexes on `parameter.plc_type_id` and `plc_type_member.member_type_id`.

### Risks and mitigations

- Risk: a plant expects automatic propagation. Mitigation: Assumption A29 states the rule. A later ADR can add a "create drafts for all" bulk command without a change to the data model.

## Related

- Amends ADR-0007 (limits at rebind). ADR-0006, ADR-0009, ADR-0016, ADR-0017, ADR-0019.
- `domain-model.md` sections 2.1 and 5.
- `data-model.md` section 6.
- Review findings: architecture F4. Traceability Pass A, item 15.
