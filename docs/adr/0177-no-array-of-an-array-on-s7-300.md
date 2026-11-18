# ADR-0177: No array of an array on S7-300

Status: Accepted. Amends ADR-0148 (i) and (j) (the platforms of the rule) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ01 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4). ADR-0148 refuses an array of an array on `S7_1200` and `S7_1500` only.

These facts shape the decision:

- ADR-0148 (i) keeps `domain-model.md` §2.1 on `S7_300`: "The element of an array is any `TypeExpr`, also another array."
- The Siemens reference file says: "Direct `ARRAY OF ARRAY` declarations are not supported by this construct" (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:515`).
- An `S7_300` CPU has standard access only. ADR-0148 quotes TIA Portal V21 for standard access: "All data types except ARRAY".
- The layout rows A09 and A10 of ADR-0147 have no Siemens source. ADR-0148 (j) keeps them on `S7_300` only.
- Every parameter on `S7_300` has an address (I5). Such a type thus always reaches a standard-access block.

The project owner decided on 2026-11-17.

## Decision drivers

- An exported type exists in the PLC (R03, R12).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- One rule lives in one function (ADR-0120).

## Considered options

### Option A: Refuse the type on `S7_300` with the rule of ADR-0148

- Pros: one rule for the three S7 platforms. No new code and no new text.
- Cons: a stored type of this kind on `S7_300` keeps a layout without a Siemens source.

### Option B: Keep the type on `S7_300`

- Pros: no change.
- Cons: the editor accepts a shape that the PLC cannot declare.

## Decision

The project uses **Option A**.

- (a) On `S7_300`, `S7_1200`, and `S7_1500`, the element of an array cannot be an array. The rule and the codes of ADR-0148 (a) to (d) apply on `S7_300` too.
- (b) `CodesysIec` and `TwinCat3` keep the rule of `domain-model.md` §2.1. This item replaces ADR-0148 (i).
- (c) Stored rows. The earlier rule set accepts a stored array row on `S7_300` whose element is an array. ADR-0148 (e) to (h) apply to such a row. This item amends ADR-0120 (a).
- (d) A command checks only a type text that it writes (ADR-0143 (i2)). The rule has no branch of `RuleSet`, as ADR-0166 (t) states for ADR-0148 (e).
- (e) The snapshot check, `ApproveVersion`, and the export do not check item (a) (ADR-0148 (g)).
- (f) The layout rows A09 and A10 stay in the golden tests on `S7_300`. A stored type still needs a layout for the end rule and the overlap rule. This item replaces ADR-0148 (j).
- (g) The ops query `deploy/db/check-s7-layout-rules.sql` lists the stored array rows of item (c) on `S7_300` in its part `nested_array`.

## Consequences

### Positive

- A new type on an S7 machine exists in a block with standard access.
- No new error code and no migration.

### Negative

- A stored version on `S7_300` with such a type stays approvable and exportable.
- The message of `E-PLT-013` does not name the array element (ADR-0148).

### Risks and mitigations

- Risk: the stored data of a production server holds such a type on `S7_300`. Mitigation: the ops query of item (g) lists the rows before an upgrade.

## Related

- ADR-0017 (derived types), ADR-0120 (earlier rule set), ADR-0143 (type texts that a command writes)
- ADR-0147 (S7 layout), ADR-0148 (the rule on `S7_1200` and `S7_1500`), ADR-0166 (constructors for stored data)
- `domain-model.md` §2.1 (`TypeExpr`), `reliability.md` §1.2 (`E-PLT-013`, `E-PLT-006`)
- `overview.md` §7: R03, R12, R13, R19, R38. §9: A38
