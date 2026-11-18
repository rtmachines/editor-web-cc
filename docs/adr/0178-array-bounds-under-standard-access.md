# ADR-0178: Array bounds under standard access

Status: Accepted. Amends ADR-0116 (e) and (f) (the rules of one parameter with an address) and ADR-0119 (b) (the address rules of the export check)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ02 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- The Siemens reference file gives the index range `−32,768…32,767` for standard access and the 32-bit range for optimized access (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:512` and `:513`).
- `domain-model.md` §2.1 accepts each bound in the INTEGER range. No platform rule limits a bound.
- An `S7_300` CPU has standard access only. Every parameter on `S7_300` has an address (I5).
- An address gives a byte offset, and an optimized block has no offset (ADR-0116). A parameter with an address thus lives in a block with standard access.
- ADR-0116 adds rules that need the address and the type tree of one parameter. A command runs them for the parameter that it writes, and `ApproveVersion` runs them for every parameter.
- ADR-0119 (b) runs the end rule and the data block rule of ADR-0116 in the export check.

The project owner decided on 2026-11-17. The rule applies to every S7 parameter with an address.

## Decision drivers

- An exported type exists in the PLC (R03, R12).
- The domain checks all external input (R38).
- A rule change does not lock a draft (ADR-0028, ADR-0116 (e1)).
- A new error code only when no existing code has the same meaning (`reliability.md` §1.2).

## Considered options

### Option A: Limit the bounds on `S7_300` only

- Pros: no rule for an optional address.
- Cons: a parameter with an address on `S7_1200` or `S7_1500` also uses a standard-access block. TIA Portal refuses the bound there too.

### Option B: Limit the bounds for every S7 parameter with an address

- Pros: one rule for each block with standard access.
- Cons: the rule needs the address and the type tree. It cannot run on a type text alone.

### Option C: Limit the bounds of every type on the S7 platforms

- Pros: the rule runs on a type text.
- Cons: a parameter without an address on `S7_1200` or `S7_1500` uses an optimized block. The rule then refuses a valid type.

## Decision

The project uses **Option B**.

- (a) Standard-access rule. A parameter with an address on `S7_300`, `S7_1200`, or `S7_1500` has every array bound in `-32768..=32767`. The rule reads every array of the type tree: the type of the parameter, struct members, and array elements at each level.
- (b) A break of item (a) gives the new code `E-RCP-058 S7TypeNotAddressable`. The variant names the tag, the address, and a reason. The reason of this item is "an array bound is outside -32768 to 32767".
- (c) The rule runs with the rules of ADR-0116. A command runs it for the parameter that `AddParameter` or `UpdateParameterDefinition` writes and for each parameter that `RebindTypeRevision` rebinds. `ApproveVersion` runs it for every parameter. This item amends ADR-0116 (e).
- (d) The checks of one parameter run in this order: platform rule, data block rule, width, alignment, end rule, standard-access rule. This item amends ADR-0116 (f).
- (e) Stored rows. A stored address of another parameter does not refuse a command (ADR-0116 (e1)). `CreateVersion` copies a base that breaks item (a), because the snapshot check does not run the S7 rules (ADR-0120 (h)).
- (f) The export check and the version page check of an `Approved` version run item (a) too. A break makes `loadable` `false` (ADR-0119 (c)). This item amends ADR-0119 (b).
- (g) The rule has no branch of `RuleSet`. The stored type rows, the type page, and the stored parameter types stay readable (ADR-0166 (t)).
- (h) A struct on `S7_300` with such an array member stays definable. A parameter with that struct then gets `E-RCP-058`.
- (i) The values `-32768` and `32767` are facts of the platform. They are not settings (ADR-0116 (h)).
- (j) The ops query `deploy/db/check-s7-layout-rules.sql` lists each stored parameter with an address whose type tree breaks item (a), in a new part `array_bound`.
- (k) A form shows the line of `E-RCP-058` under the input `s7_address`, as for `E-RCP-056` and `E-RCP-057` (ADR-0041). The approval preview links the parameter of the tag (ADR-0049 (c)).

## Consequences

### Positive

- A parameter with an address has a type that a block with standard access can declare.
- The user can fix the parameter: change the type, or delete the address on `S7_1200` and `S7_1500`.

### Negative

- One new error code and its texts in the bundles `pl` and `en`.
- An `Approved` version with such a parameter becomes not loadable.

### Risks and mitigations

- Risk: the stored data holds such a parameter. Mitigation: the ops query of item (j) lists it before an upgrade.

## Related

- ADR-0116 (rules of one parameter, no offset in an optimized block), ADR-0119 (export check), ADR-0120 (earlier rule set)
- ADR-0166 (rules without a branch), ADR-0187 (second reason of `E-RCP-058`)
- `domain-model.md` §2.1 (`S7Address`, `ArrayDimension`), `reliability.md` §1.2 (`E-RCP-058`)
- `overview.md` §7: R03, R12, R38. §9: A31, A38
