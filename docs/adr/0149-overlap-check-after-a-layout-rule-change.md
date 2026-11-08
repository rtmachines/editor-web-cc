# ADR-0149: Overlap check after a change of the layout rules

Status: Accepted. Amends ADR-0116 (e1) (overlap check of a command) and ADR-0119 (b) (address check of the export and the version page)
Date: 2026-11-08
Deciders: project owner

## Context

ADR-0147 changes the S7 layout size of a struct with a `SINT` or `USINT` member and of a `BOOL` array with two or three dimensions. The layout is a computed value, so a stored parameter gets the new size at its next check (ADR-0147 (j)).

These facts shape the decision:

- A stored address can overlap another stored address under the new size. An example is the struct `ST_Bytes@1` (now 6 bytes) at `DB1.DBB0` and a `BYTE` at `DB1.DBB4`.
- ADR-0116 (e1) says that a stored address of another parameter does not refuse a command. Its last sentence keeps the overlap check of I20 on the whole draft.
- Every parameter command commits through `Draft::replace_parameters`. That method runs the overlap check on every pair of parameters of the changed draft (`crates/recipe-domain/src/recipe/draft/replace.rs` line 28).
- `SetParameterValue`, `ReplaceLimits`, `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` therefore refuse a draft with a stored overlap of two other parameters. Only `RemoveParameter` has no S7 check.
- ADR-0119 (b) checks every S7 address of a version with the end rule and the data block rule only. No path checks an overlap of an `Approved` version.
- An `Approved` version with a stored overlap therefore exports with `loadable` `true`. Two parameters then share bytes in the PLC.

The project owner decided on 2026-11-08.

## Decision drivers

- Two parameters never share bytes in the PLC (R14, R31).
- A rule change does not lock a draft (ADR-0116 (e1), ADR-0120).
- A consumer gets one clear signal that it must not load a document (ADR-0119).
- Inside a transaction, no use case waits for I/O or for a lock that it does not hold (`reliability.md` section 7.4).

## Considered options

### Option A: A command checks the pairs of the parameters that it writes, and the export checks every pair

- Pros: a stored overlap of two other parameters does not lock a draft. `ApproveVersion` and the export still find every overlap.
- Cons: a draft can hold a stored overlap until its approval.

### Option B: Every command checks every pair of the draft

- Pros: no change of the code of the commands.
- Cons: one stored overlap refuses every edit of the draft. The user must remove a parameter and loses its value, limits, and translations.

### Option C: No overlap check at export, and an ops query before the upgrade

- Pros: no change of the export.
- Cons: SQL cannot compute the layout, so the query lists only candidates. An overlapping `Approved` version stays loadable.

## Decision

The project uses **Option A**. Both parts decide where the overlap check of I20 runs after a change of the layout rules, so one ADR holds them.

- (a) A command that writes parameters reports only an overlap of a pair that holds at least one parameter that it writes. This amends the last sentence of ADR-0116 (e1).
- (b) The written parameters are the new parameter of `AddParameter` and the changed parameter of `UpdateParameterDefinition`. They are also the parameter of `SetParameterValue` and of `ReplaceLimits`, and each parameter that `RebindTypeRevision` rebinds.
- (c) `ApproveVersion` checks every pair of parameters of the draft. A stored overlap refuses the approval with `E-RCP-022`.
- (d) The user removes a stored overlap with `UpdateParameterDefinition` on one parameter of the pair. A second stored pair of two other parameters does not refuse that command.
- (e) The export check and the version page check of ADR-0119 (b) and (l) also run the overlap check of I20. The check covers every parameter of the version. This amends ADR-0119 (b).
- (f) The check uses only the rows and the type trees that the export already read (ADR-0119 (j)). It runs in the same blocking task, and `timeout.export` bounds it (ADR-0122).
- (g) Each parameter of an overlapping pair gets a rule fault `E-RCP-022` on its address. Then `loadable` is `false` (ADR-0119 (c)), and the notice of the version page names `E-RCP-022` (ADR-0119 (l)).
- (h) An overlap is a rule fault and not a structure fault. It never gives `500 E-DB-008`.
- (i) A v2 document checks the pairs of every parameter of the version, not only of its scope (ADR-0119 (f)).
- (j) The hashed body and `content_sha256` do not change (ADR-0119 (d)).
- (k) The decision adds no error code, no text in the bundles, and no setting.

## Consequences

### Positive

- A stored overlap after a change of the layout rules does not lock a draft.
- An `Approved` version with an overlap is not loadable, and its version page shows the code.

### Negative

- A draft can hold a stored overlap until `ApproveVersion` refuses it.
- `SetParameterValue` and `ReplaceLimits` refuse an edit of a parameter of a stored overlap pair. The user first moves one parameter of the pair.
- Each export and each version page check of an `Approved` version computes the occupied bytes of every parameter with an S7 address.

### Risks and mitigations

- Risk: a stored overlap pair stays in a draft for a long time. Mitigation: `ApproveVersion` refuses it, and the approval preview of ADR-0049 shows it.

## Related

- ADR-0017 (overlap rule), ADR-0028 (overlap check at rebind), ADR-0049 (approval preview), ADR-0116 (end rule, check paths)
- ADR-0119 (export check), ADR-0122 (bounds of the version page check), ADR-0147 (S7 layout from the Siemens rules)
- `domain-model.md` §2.1 (overlap rule), §3 (I20), §5 (commands)
- `overview.md` §7: R14, R31. §9: A31
