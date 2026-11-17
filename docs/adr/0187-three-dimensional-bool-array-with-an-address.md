# ADR-0187: Three-dimensional BOOL array with an address

Status: Accepted. Amends ADR-0147 (g) and (g1) (the interim size of a `BOOL` array with three dimensions). Amends ADR-0116 (e) and (f) (the rules of one parameter with an address). Amends ADR-0119 (b) (the address rules of the export check)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ12 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- ADR-0147 (g) gives a `BOOL` array with three dimensions an interim size. Each line of the last index starts at the next byte. The size is the larger of two readings of rule S-AR4.
- ADR-0147 (g1) records one gap. When such an array is a struct member, the offsets of the later members are a guess.
- The reference row A03 `ARRAY[1..3,1..3,1..3] OF BOOL` has the mark `Unverified("needs a TIA measurement")` (`crates/recipe-domain/src/plant/s7_layout/reference.rs`).
- A wrong offset of a member can make the transfer client write a value into the wrong byte of the PLC. The overlap check cannot find this fault.
- ADR-0178 adds the standard-access rule and the code `E-RCP-058 S7TypeNotAddressable`.

The project owner decided on 2026-11-17: refuse such a type with an address until a TIA measurement gives the layout.

## Decision drivers

- A value in the PLC equals the approved value (R03, R12).
- Data safety has priority over function (R31, A1).
- A rule change does not lock a draft (ADR-0028, ADR-0116 (e1)).

## Considered options

### Option A: Keep the interim size

- Pros: no change.
- Cons: the offsets of later struct members can differ from the PLC.

### Option B: Refuse such a type with an address

- Pros: no export carries an offset without a Siemens source.
- Cons: a plant cannot address a `BOOL` array with three dimensions on S7.

## Decision

The project uses **Option B**.

- (a) A parameter with an address on `S7_300`, `S7_1200`, or `S7_1500` has no `BOOL` array with three dimensions in its type tree. The rule reads the type of the parameter, struct members, and array elements at each level.
- (b) A break of item (a) gives `E-RCP-058 S7TypeNotAddressable` with the reason "a BOOL array with three dimensions has no verified S7 layout" (ADR-0178 (b)).
- (c) The rule runs with the standard-access rule of ADR-0178 (c) and (d). A command runs it for each parameter that it writes or rebinds. `ApproveVersion` runs it for every parameter. This item amends ADR-0116 (e) and (f).
- (d) The export check and the version page check of an `Approved` version run item (a). A break makes `loadable` `false`. This item amends ADR-0119 (b).
- (e) Stored rows. A stored address of another parameter does not refuse a command (ADR-0116 (e1)). `CreateVersion` copies a base that breaks item (a) (ADR-0120 (h)). The rule has no branch of `RuleSet` (ADR-0166 (t)).
- (f) A parameter without an address on `S7_1200` or `S7_1500` keeps such a type. A struct with such a member stays definable.
- (g) The interim size of ADR-0147 (g) stays in `S7Layout`. The type page and a stored type still need a size. This item amends ADR-0147 (g) and (g1).
- (h) A new ADR removes item (a) when a TIA Portal measurement gives the layout of row A03.
- (i) The ops query `deploy/db/check-s7-layout-rules.sql` lists each stored parameter with an address whose type tree holds such an array, in a new part `bool_array_3d`.

## Consequences

### Positive

- No export carries the guessed offsets of ADR-0147 (g1).
- No new error code. ADR-0178 adds `E-RCP-058`.

### Negative

- A plant cannot address a `BOOL` array with three dimensions on S7 until the measurement.
- An `Approved` version with such a parameter becomes not loadable.

### Risks and mitigations

- Risk: the stored data holds such a parameter. Mitigation: the ops query of item (i) lists it before an upgrade.

## Related

- ADR-0116 (rules of one parameter), ADR-0119 (export check), ADR-0120 (earlier rule set), ADR-0147 (S7 layout), ADR-0166 (rules without a branch), ADR-0178 (standard-access rule, `E-RCP-058`)
- `domain-model.md` §2.1 (`S7Layout`), `reliability.md` §1.2 (`E-RCP-058`)
- `overview.md` §7: R03, R12, R31. §9: A1, A38
