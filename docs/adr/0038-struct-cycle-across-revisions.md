# ADR-0038: Struct cycle across revisions

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

Invariant I8 of `domain-model.md` section 3 says that a struct has no cycle. `TypeTree::resolve()` and the check of a struct row compare type identifiers.

- A new revision of a struct is a new row with a new identifier. A member of that revision can name an earlier revision of the same struct, for example `ST_A@2` with a member of the type `ST_A@1`. The check by identifier accepts it.
- A PLC holds one data type for each name. On the PLC, `ST_A@1` and `ST_A@2` are both the type `ST_A`. So `ST_A` contains itself, and no PLC program can declare it.
- The test in a browser of step P3.2 found this case. ADR-0037 (u) says that no request can produce `E-PLT-005`.

## Decision drivers

- A version holds only types that a PLC can declare (R13).
- A stored type row never changes (I22). Each stored row stays readable, also for the export of an approved version.
- The project adds no error code. `E-PLT-005 TypeCycle` already names the rule.

## Considered options

### Option A: `DefineUdt` compares the struct names

- Pros: one check at the start of each new row. A read of a stored row does not change.
- Cons: a stored row from before this ADR is not checked. No production database exists before this ADR.

### Option B: `TypeTree::resolve()` compares the struct names

- Pros: each read checks the rule.
- Cons: a stored row that breaks the rule makes each read of its parameters fail, also the export of an approved version.

### Option C: Keep the check by identifier

- Pros: no change.
- Cons: the editor accepts a type that no PLC can declare.

## Decision

The project uses **Option A**.

- (a) Two revisions of one struct are the same type for invariant I8. A struct contains itself when its type tree holds a struct with the same name, in any revision.
- (b) The type tree of a struct holds its members, the element of each array, and the members of each struct in the tree.
- (c) `DefineUdt` checks the type tree of the new struct by name. A struct name that occurs twice on one path from the root gives `E-PLT-005 TypeCycle` with that name. The command creates no type.
- (d) `TypeTree::resolve()` keeps the check by identifier. A read of a stored row does not run the check by name.
- (e) The `datalist` of the form of a new revision (ADR-0037 (i)) leaves out each revision of the struct that the form revises.
- (f) The test in a browser can now produce `E-PLT-005` with a new revision of `ST_A` and a member of the type `ST_A@1`. The test of ADR-0037 (u) with two seeded structs stays.

## Consequences

### Positive

- The editor refuses a struct that no PLC can declare.
- A user gets `E-PLT-005` for this case, and no new code is necessary.
- A read and an export of a stored row do not change.

### Negative

- `DefineUdt` walks the type tree of the new struct one more time. The size limits of `reliability.md` section 7.11 bound this walk.
- A stored row from before this ADR can break the rule. No production database holds such a row.

### Risks and mitigations

- Risk: the check by name and the check by identifier give different answers for one tree. Mitigation: a test of `recipe-domain` covers a cycle by identifier, a cycle by name, and a struct that holds one struct twice on two paths.

## Related

- ADR-0006, ADR-0017 (PLC types and UDT revisions)
- ADR-0037 (i) and (u), amended by this ADR
- `domain-model.md` sections 2 and 3 (I8, I22)
- `api.md` section 2.3
