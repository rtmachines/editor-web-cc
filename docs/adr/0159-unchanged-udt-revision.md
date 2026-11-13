# ADR-0159: Refusal of an unchanged UDT revision

Status: Accepted
Date: 2026-11-12
Deciders: project owner

## Context

The UI and UX audit of 2026-11-12 (finding F4) reports that Enter in the UDT form has two meanings. The change `ui-form-fixes` makes the button "Create type" the first submit button (ADR-0037 (h)). Enter then sends `DefineUdt` at each count of rows.

- The revision form of `GET /machines/{code}/types/new?from=` holds the members of the latest revision.
- Enter in that form, before any edit, sends those members back.
- `DefineUdt` in the revision mode does not compare the members with the latest revision. It writes a new revision with the same members.
- A new revision marks each parameter that uses the old revision as outdated (ADR-0018). The user must then rebind each draft, and no member changed.

## Decision drivers

- One key press never writes a revision that changes nothing (R39).
- A revision of a UDT records a change of its members (R06, R07, ADR-0006).
- The check runs in the domain or the use case, not only in the browser (ADR-0013).
- The check sees the same rows as the insert, so two commands at the same time cannot both pass it.

## Considered options

### Option A: `DefineUdt` refuses members that equal the latest revision

The use case compares the resolved members with the members of the latest revision. Equal members give a new error code.

### Option B: The form disables the button until the user edits a member

This option needs a script and does not stop a request from another client. `api.md` section 9 permits no such script.

### Option C: Accept the unchanged revision

The revision stays harmless for the data, but each draft that uses the type needs a rebind with no reason.

## Decision

Option A.

- (a) `DefineUdt` in the mode `Revision` refuses a request when its members equal the members of the latest revision of the struct. The command writes nothing.
- (b) Two member lists are equal when two conditions hold. They have the same count. At each position they have the same member name in the exact case and the same type row. A change of the letter case of a member name is a change.
- (c) The use case compares the members after `EnsureDerivedType` resolves each `TypeExpr`, under the class-2 advisory locks of `data-model.md` section 1.4. A parallel `DefineUdt` of the same name therefore waits, and the comparison reads the revision that it wrote.
- (d) When the members are equal, every member type already exists. `EnsureDerivedType` then creates no row, and the refusal leaves no derived type.
- (e) The new variant `PlantError::RevisionUnchanged { type_name, revision }` returns `E-PLT-052` with HTTP `409`. `revision` is the latest revision. The English text: "Revision {revision} of type {type_name} has the same members. Change a member before you create the revision."
- (f) The web layer shows the error in the summary of `#error-panel` with no field line, like `E-PLT-015`. The form keeps the sent rows.
- (g) The mode `New` does not change.

This decision amends `domain-model.md` section 5.1 (the errors of `DefineUdt`), `data-model.md` section 6 (the row `DefineUdt`), `reliability.md` section 1.2, and `api.md` section 2.3 (the errors of the two `DefineUdt` routes).

## Consequences

### Positive

- Enter in an unedited revision form gives a clear message and writes nothing.
- No draft gets an outdated parameter from a revision without a change.

### Negative

- A user who wants a revision only as a marker cannot write it. No requirement asks for such a marker.
- The command compares up to 1024 members. This cost is small beside the insert of 1024 member rows.

### Risks and mitigations

- [An existing test writes a revision with the same members] → The implementation task runs the full suite. It changes each such test to change one member.
- [The comparison misses a change of the type of a member] → The comparison uses the resolved type row, so `ST_Zone@2` and `ST_Zone@3` differ.

## Related

- ADR-0006 (relational type catalog with immutable revisions)
- ADR-0018 (UDT revisions and the rebind of drafts)
- ADR-0037 (h) (the second submit button "Add a member")
- ADR-0143 (struct and member names compare without case for uniqueness)
- Change `ui-form-fixes` (audit finding F4)
