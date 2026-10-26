# Why versions are immutable

## Purpose and reader

This page explains why you cannot edit an approved version of a recipe. It has no procedure.

The reader is a process engineer, an operator, or an auditor. You ask why you cannot "just fix" a value in a version that already ran on a machine.

The editor shows Polish by default. The buttons **Polski** and **English** in the top bar change the language. This page uses the English texts.

## What a version is

A version is a full snapshot of one recipe. It holds every parameter with its type, its value, and its process limits ([ADR-0009](../adr/0009-version-snapshot-immutable.md)).

A version has one of three states, as [`domain-model.md`](../architecture/domain-model.md) section 4 shows:

- **Draft** is the only version that you can edit. A recipe has at most one draft (A7).
- **Approved** is the active version. A recipe has at most one approved version (R18).
- **Retired** is a former approved version. It is final.

A draft becomes Approved. An Approved version becomes Retired. No other transition exists. A Retired version never becomes Approved again, and an Approved version never becomes a draft.

## Why a change makes a new version

The requirements say that a change of a type, a limit, or a UDT creates a new version (R19). The snapshot design makes this the only way to change anything. Four reasons follow.

### You can prove what ran on the machine

An approved version keeps the same types, limits, and values for its whole life. A later question about a past production run has one answer. If someone could edit the version, the answer would depend on the date of the question.

### The audit log shows who changed what

Every command writes an audit entry in the same transaction as the change (R20, [ADR-0005](../adr/0005-audit-log-all-mutations.md)). The application cannot edit or delete an entry. Each entry also holds a hash of the previous entry. A changed entry therefore breaks the chain, and a nightly job reports the break ([ADR-0022](../adr/0022-approval-policy.md)).

### A second person approves the change

By default, the person who created or edited a draft cannot approve it ([ADR-0022](../adr/0022-approval-policy.md), A34). A process limit protects the process only when one person cannot widen the limit, move the value, and approve alone.

This rule needs a stable object to approve. The approver reviews a draft that nobody can change after the approval.

### The export carries a hash

The approval stores a content hash of the version. An export document holds this hash. A transfer system can recompute it and find a file that someone edited after the download ([ADR-0019](../adr/0019-export-integrity-and-contract.md)). [Check an export document before a PLC load](check-export-document.md) shows how.

The hash only has meaning when the version behind it never changes.

## Why the type stays with the version

A UDT revision never changes. An edit of a UDT creates a new revision, and the old revision stays ([ADR-0006](../adr/0006-plc-type-catalog-relational.md)). A parameter points to one revision.

A new revision does not change any existing version. The editor marks the affected parameters as outdated. You then rebind them in a draft ([ADR-0018](../adr/0018-udt-revision-rebind.md)). The rebind stops when the new revision lacks a member that had a limit. No limit disappears without a decision.

The approval of a draft with outdated parameters needs an explicit flag. The audit entry records the flag.

## What you do instead of an edit

1. You create a draft from the version that you want to change. The draft is a copy.
2. You change the parameters in the draft.
3. A second person approves the draft.

The approval makes the draft Approved and retires the previous Approved version in one transaction (R27, `domain-model.md` section 5.2). The recipe never has two approved versions.

An Administrator can also retire an Approved version without a replacement (A28). You can use any version of the recipe as the base of a new draft. A Retired version is therefore a way to return to an older layout.

The procedures are in [Edit a parameter in a draft](how-to-edit-a-parameter-in-a-draft.md), [Approve a version](how-to-approve-a-version.md), and [Compare two versions](how-to-compare-two-versions.md).

## Delete without loss

The editor does not delete history. A deleted recipe gets a deletion mark and an Administrator can restore it ([ADR-0021](../adr/0021-soft-delete-and-delete-rules.md)). The database role of the application cannot delete a version row.

Two rules protect the Approved and Retired versions. A ProcessEngineer cannot delete a recipe that has an Approved or Retired version. An Administrator cannot delete a recipe that has an Approved version. A deleted recipe keeps its reference number.

## Trade-offs

The design has costs:

- The database holds a copy of all parameters for each version. The documents estimate that this is small at the expected size (ADR-0009).
- Each approval needs a second person. A plant with one engineer needs an Administrator for the approval, or an audited setting that turns the rule off (ADR-0022).
- A small fix, such as a wrong value, needs a new draft and a new approval. The fix is slower than an edit in place.
- A change of a UDT does not reach recipes by itself. An outdated parameter can stay outdated until someone rebinds it (ADR-0018).

The project accepts these costs. The editor holds recipes of a production line, and a recipe error can damage product or equipment. Data safety has priority over speed (R31).

## Related

- [`overview.md`](../architecture/overview.md) section 7 lists R18, R19, R20, R27, and R31.
- [`domain-model.md`](../architecture/domain-model.md) section 3 lists the invariants I3, I4, and I16.
