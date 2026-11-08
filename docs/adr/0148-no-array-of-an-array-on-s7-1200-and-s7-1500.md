# ADR-0148: No array of an array on S7-1200 and S7-1500

Status: Accepted. Amends ADR-0017 (cases of `E-PLT-013`) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-08
Deciders: project owner

## Context

The S7 type coverage review asked as Q09 whether TIA Portal accepts `ARRAY[..] OF ARRAY[..]` in a data block (`docs/plan/review/plc-s7-type-coverage-review.md`). The PLC data type review of 2026-11-06 links Q09 to its finding #16.

These facts shape the decision:

- `domain-model.md` §2.1 says "The element of an array is any `TypeExpr`, also another array."
- The research record `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` cites rule S-AR5 of TIA Portal V21: "All data types except ARRAY are permitted for the components."
- The table of the same page gives for standard access: "All data types except ARRAY Multi-instances". The research gives the rule high confidence.
- The reference rows A09 and A10 pin a layout of 8 and 6 bytes for two such types. No Siemens layout exists for them.
- ADR-0045 decides how a page shows an array inside a struct element of an array. It does not decide which types are valid.
- `E-PLT-013 DerivedTypeInvalid` has the reason "the type does not exist on the platform of the machine". `DefineUdt` gives `E-PLT-006` with the member name for that reason (ADR-0017).
- ADR-0143 (i2) names the type texts that a command writes. ADR-0143 (i1) keeps a stored type readable under the earlier rule set.

The project owner decided on 2026-11-08. The decision names `S7_1200` and `S7_1500`.

## Decision drivers

- An exported type exists in the PLC (R03, R12).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- A new error code only when no existing code has the same meaning (`reliability.md` §1.2).

## Considered options

### Option A: Refuse the type with the reason "not on the platform"

- Pros: no new error code and no new text. `DefineUdt` already maps the reason to `E-PLT-006`.
- Cons: the message does not name the array element as the cause.

### Option B: Refuse the type with a new reason of `E-PLT-013`

- Pros: the message names the cause.
- Cons: a new reason needs new texts in the bundles `pl` and `en` and a second mapping to `E-PLT-006`.

### Option C: Keep the type and record that its layout has no Siemens source

- Pros: no change of the type rules.
- Cons: TIA Portal refuses the type in a block with standard access. A recipe with it cannot go into such a block.

## Decision

The project uses **Option A**.

- (a) On `S7_1200` and `S7_1500`, the element of an array cannot be an array. A type text with two `ARRAY[...] OF` prefixes in a row breaks the rule, for example `ARRAY[1..2] OF ARRAY[1..3] OF BYTE`.
- (b) One prefix with two or three dimensions keeps the rule of A38, for example `ARRAY[1..2,1..3] OF BYTE`. An array of a struct with an array member also stays valid.
- (c) A command checks a type text that it writes with the rule of item (a) (ADR-0120 (c)). The texts are the ones of ADR-0143 (i2): the type of `AddParameter`, a changed type of `UpdateParameterDefinition`, and the member types of `DefineUdt`.
- (d) A break of item (a) gives `E-PLT-013` with the reason "the type does not exist on the platform of the machine". A member of `DefineUdt` gets `E-PLT-006` with the member name. This item adds a case to the list of `E-PLT-013` in ADR-0017.
- (e) The earlier rule set accepts a stored array row whose element is an array. This item amends ADR-0120 (a). The type rows, the type page, and a stored parameter type stay readable.
- (f) A parameter keeps such a stored type while a command does not change its type. `RebindTypeRevision` keeps the array levels of such a type, because it writes no type text.
- (g) The snapshot check and `ApproveVersion` do not check item (a) (ADR-0120 (h)). The export does not change `loadable` for such a type.
- (h) The database gets no new CHECK. The domain holds the rule, as it holds the other platform rules of a type (ADR-0006).
- (i) `S7_300`, `CodesysIec`, and `TwinCat3` keep the rule of `domain-model.md` §2.1.
- (j) The reference rows A09 and A10 leave the layout table of ADR-0147, because no Siemens rule gives their layout. Item (i) keeps the type on `S7_300`, where the address is required. Golden tests of the layout keep their sizes of 8 and 6 bytes on `S7_300`.

## Consequences

### Positive

- A new type on `S7_1200` and `S7_1500` exists in a block with standard access.
- No new error code, no new text, and no migration.

### Negative

- The message of `E-PLT-013` says "not on the platform" and does not name the array element.
- The preview of the parameter form keeps the rules of `EnsureDerivedType`. It shows the leaves of such a type, and the command then refuses it.
- A stored version with such a type stays approvable and exportable.

### Risks and mitigations

- Risk: the stored data holds such a type on a production server. Mitigation: an ops query lists the stored array rows of this kind and the parameters that use them before an upgrade.
- Risk: an optimized block accepts an array of an array, and a user needs it. Mitigation: a new ADR can add a setting of the machine with Option C.

## Related

- ADR-0017 (derived types), ADR-0045 (nested arrays on a page), ADR-0120 (earlier rule set), ADR-0143 (type texts that a command writes), ADR-0147 (S7 layout)
- `domain-model.md` §2.1 (`TypeExpr`), `reliability.md` §1.2 (`E-PLT-013`, `E-PLT-006`)
- `overview.md` §7: R03, R12, R13, R19, R38. §9: A38
