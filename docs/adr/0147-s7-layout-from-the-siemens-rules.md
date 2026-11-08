# ADR-0147: S7 layout from the Siemens rules

Status: Accepted. Amends ADR-0017 (S7 layout of a `SINT` or `USINT` struct member and of a `BOOL` array with two dimensions, source marks of the reference table)
Date: 2026-11-08
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found that no test of the S7 layout uses a Siemens source. `docs/plan/review/2026-11-06-plc-type-review.md` holds it as #16, with the open questions Q09 and Q10.

These facts shape the decision:

- The reference table `crates/recipe-domain/src/plant/s7_layout/reference.rs` has 24 composite rows, C01 to C13 and A01 to A11. Each row has the mark `Unverified("Q10")`.
- The research record `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` derives each row from the TIA Portal V21 documentation. It names its rules S-ST1 to S-BOOL. No TIA Portal measurement exists.
- 19 rows agree with the documentation. C04, A02, A03, A09, and A10 differ.
- C04: rule S-ST2 names only `BOOL`, `BYTE`, and `CHAR` as packed. "Components with different data type start at a word limit." ADR-0017 lets a `SINT` or `USINT` member start on any byte.
- A02: `domain-model.md` §2.1 packs `BOOL` elements of an array 8 per byte over all dimensions. Rule S-AR4 says: "In bit and byte components a new dimension always starts at the next byte."
- A03: rule S-AR4 has two readings for three dimensions. Reading 1 gives 10 bytes, and reading 2 gives 6 bytes. The text does not decide.
- No page in reach gives the start of a `SINT` or `USINT` element of an array. No page gives the start of a `SINT` or `USINT` parameter in a data block (rule S-ST2b).
- A09 and A10 hold an array of an array. ADR-0148 refuses that type on `S7_1200` and `S7_1500`.

The project owner decided on 2026-11-08. The owner accepts a mark "documented" in place of `Verified` for a row that follows from cited Siemens rules.

## Decision drivers

- A wrong S7 size lets two parameters share bytes in the PLC (R14, R31).
- Each layout rule cites a Siemens source (ADR-0017, "Risks and mitigations").
- A rule without a source stays an open point. It does not become a fact.
- The domain computes the layout and does not store it (ADR-0017, `data-model.md` §3.3).

## Considered options

### Option A: The layout follows the literal text of TIA Portal V21

- Pros: each rule cites a Siemens page. The two rows that differ get the size of the documentation.
- Cons: the reading of S-ST2 for `SINT` and `USINT` has medium confidence. A later measurement can change it.

### Option B: The layout keeps the rules of ADR-0017 until a TIA Portal measurement

- Pros: no change of a computed size.
- Cons: two rows stay against the documentation. A `BOOL` array with two dimensions can overlap the next parameter in the PLC.

### Option C: The domain refuses the affected types on S7 until a measurement

- Pros: no size without a source.
- Cons: a `SINT` member and a `BOOL` array with two dimensions are common. The refusal blocks valid recipes.

## Decision

The project uses **Option A**.

- (a) In a struct, a `SINT` or `USINT` member starts at the next even byte (S-ST2, S-ST2b). Only `BOOL`, `BYTE`, and `CHAR` members keep the bit or byte packing.
- (b) For a struct member, item (a) replaces two rules of the section "S7-300 layout" of ADR-0017. These are "The next non-`BOOL` member starts at the next byte" and the words "`SINT`, `USINT`" in the list of 1-byte elements. The struct `BYTE`, `CHAR`, `SINT`, `USINT` takes 6 bytes. Its members start at 0.0, 1.0, 2.0, and 4.0.
- (c) Item (a) applies only to a member of a struct. A `SINT` or `USINT` element of an array keeps one byte. A `SINT` or `USINT` parameter keeps the alignment rule of `domain-model.md` §2.1. No page in reach gives a rule for these two cases.
- (d) In a `BOOL` array with two dimensions, each line of the last index starts at the next byte (S-AR4). The array ends on an even byte (S-AR1). Thus `ARRAY[1..3,1..3] OF BOOL` takes 4 bytes.
- (e) Item (d) replaces the words "`BOOL` elements pack 8 per byte" of `domain-model.md` §2.1 for an array with more than one dimension. A one-dimensional `BOOL` array keeps the packing (S-AR2, S-AR3).
- (f) An array of `BYTE` or `CHAR` keeps one byte per element in each dimension. A new line starts at the next byte, so no gap occurs. An element wider than 1 byte keeps the even stride, so each line starts on an even byte (S-AR4).
- (g) A `BOOL` array with three dimensions has no decided reading. Until a TIA Portal measurement decides, each line of the last index starts at the next byte. This size is the larger of the two readings, so it can refuse an address but never lets two parameters share a byte.
- (g1) The owner accepts this interim size on 2026-11-08. The plan review notes one gap. When such an array is a member of a struct, the type page shows a guess for the offsets of the later members.
- (h) A row of the reference table gets the mark `Documented` when its values follow from cited Siemens rules. The mark names the rule ids of the research record. The owner accepts it in place of `Verified`.
- (i) A mark whose rule rests on an example or on a literal reading says "medium confidence". This applies to C01, C08, and A11 (S-ST2a) and to C04 (S-ST2b). The row A03 keeps the mark `Unverified` with the text "needs a TIA measurement".
- (j) The layout stays a computed value (ADR-0017). No earlier rule set of ADR-0120 applies to it. A stored struct or array gets the new layout at its next computation.
- (k) Every rule that uses the layout follows item (j). These are the end rule, the overlap rule, and the S7 layout of the type page. The rules run on the paths of ADR-0116 (e) and ADR-0149.
- (k1) The export check of ADR-0119 (b) runs the end rule. ADR-0149 adds the overlap rule to that check and to the version page check.

## Consequences

### Positive

- Each composite row of the reference table cites the Siemens rules that give its values.
- A `BOOL` array with two dimensions and a struct with `SINT` or `USINT` members get the size of the documentation.

### Negative

- A stored parameter can now overlap the next parameter or pass the end of its data block. `ApproveVersion` then refuses the draft with `E-RCP-022` or `E-RCP-056`.
- A parameter command reports such a stored overlap only when it writes one parameter of the pair (ADR-0149 (a)). The user fixes it with `UpdateParameterDefinition` on one of the two parameters.
- The export of an approved version can change `loadable` to `false` through the end rule (ADR-0119 (i)) or the overlap rule (ADR-0149 (e)). The hashed body does not change.

### Risks and mitigations

- Risk: a TIA Portal measurement contradicts item (a), (d), or (g). Mitigation: the marks name the rules and their confidence. A new ADR then amends this one.
- Risk: a stored version holds an address that the new layout makes overlap. Mitigation: an ops query lists the stored parameters with an address whose type holds an affected struct or array. The owner reads it before an upgrade. The export of such a version gives `loadable` `false` (ADR-0149 (e)).

## Related

- ADR-0017 (S7 layout), ADR-0116 (end rule, check paths), ADR-0119 (export check), ADR-0120 (earlier rule set)
- ADR-0148 (array of an array), ADR-0149 (overlap check after a change of the layout rules)
- `domain-model.md` §2.1 (`S7Layout`, `S7Address`)
- `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` (rules S-ST1 to S-BOOL)
- `overview.md` §7: R12, R13, R14, R31. §9: A31
