# ADR-0146: Shape of a member type at a rebind

Status: Accepted. Amends ADR-0018 (a member of one shape keeps its value and its limits)
Date: 2026-11-08
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a fault in the rebind. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #10.

These facts shape the decision:

- The seed rows of `STRING` and `WSTRING` store the default length of the platform. On `CodesysIec` and `TwinCat3` the length is 80 (`data-model.md` §7).
- `EnsureDerivedType` gives the seed row for `STRING` without a length. It gives a row of the machine for `STRING(80)` (ADR-0017). Thus one shape has two rows.
- ADR-0018 keeps the value and the limits of a member "with the same name and a compatible primitive". It treats "a member whose type changed" as new. It does not define either phrase.
- The domain compares the row identifiers of the two member types. A revision that only rewrites `STRING(80)` as `STRING` therefore needs a new value (`E-RCP-028`) and refuses the limit (`E-RCP-029`).
- The owner chose option (1) of question Q-H on 2026-11-06.

## Decision drivers

- A limit must never disappear without a decision (R09, R31).
- A UDT change leads to a new version with a clear path for the engineer (R19, A29).
- Type identity stays the row (ADR-0017).
- A value that fits the old member type also fits a member type of one shape.

## Considered options

### Option A: The rebind compares the shape of the member types

- Pros: one function of the domain changes. The stored rows and the indexes stay.
- Cons: the domain holds a second notion next to the row identity.

### Option B: `EnsureDerivedType` maps `STRING(n)` with the default `n` to the seed row

- Pros: one shape gets one row for new types.
- Cons: the stored rows of the machines stay. A rebind between stored revisions still fails. The identity rule of ADR-0017 changes.

### Option C: The rebind keeps the row comparison

- Pros: no change.
- Cons: the fault stays. The engineer must remove each limit and enter each value again.

## Decision

The project uses **Option A**.

- (a) The rebind keeps a member under two conditions. The source and the target revision both have a member of that name. The two member types have one shape. Every other target member is new. A source member without such a match is dropped.
- (b) Item (a) replaces the words "a compatible primitive" and "a member whose type changed" of ADR-0018. The other rules of ADR-0018 stay.
- (c) Two leaf types have one shape when they have the same primitive and the same string length. Every string row stores its length, and a seed row stores the default length of the platform. Thus the default length counts as an explicit length.
- (d) Two array types have one shape when they have the same dimensions, bound by bound, and their element types have one shape.
- (e) Two struct types have one shape only when they are the same row. Two revisions of one struct have two shapes, also with equal members. Two structs of two names have two shapes.
- (f) A leaf type, an array type, and a struct type never have one shape with a type of another kind.
- (g) Type identity stays the row. `EnsureDerivedType`, the unique indexes, and the stored rows do not change.
- (h) A kept member keeps its value. Its limit rows keep the same content. After the rebind, the member uses the row that the target revision names for it.
- (i) The audit entry `TypeRevisionRebound` of a parameter shows the change of `plc_type_id`. A kept member adds no change of a value leaf or a limit row.
- (j) The export of the new version lists the rows of the target revision in `types[]`. The export schema and the rule of the content hash do not change.
- (k) The text of `E-RCP-029` stays. Its words "another type" mean a type of another shape.
- (l) The match of the member names does not change. It compares the exact spelling. A rename that changes only the case gives a new member. ADR-0143 (b) compares member names without case only inside one struct.

Item (e) has a reason. A struct is identified by its name and its revision (ADR-0006, ADR-0017). ADR-0018 moves an embedded struct only through an explicit new revision of the outer struct. A shape rule over the struct members would keep more members than question Q-H asks for.

## Consequences

### Positive

- A revision that only rewrites `STRING(80)` as `STRING` on `CodesysIec` keeps every value and every limit.
- The rebind form asks only for the leaves of the members that are new.

### Negative

- A member of a newer struct revision is still new, also when the two revisions have equal members.
- A member that changes only the case of its name is still new.

### Risks and mitigations

- Risk: a later field of a leaf type changes the layout, but the comparison does not read it. Mitigation: The leaf pattern of the shape compare names every field of a leaf, so a new leaf field fails the build.

## Related

- Amends ADR-0018 (rebind of recipe drafts). ADR-0006, ADR-0017 (identity of derived types), ADR-0143 (b) (member names compare without case in one struct)
- `domain-model.md` §5.2 and §7, `data-model.md` §6 and §7
- `overview.md` §7: R09, R19, R31. §9: A29
