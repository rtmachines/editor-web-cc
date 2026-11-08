# Proposal

## Why

The PLC data type review of 2026-11-06 found that no S7 layout test uses a Siemens source. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 10 (WP7, R1) holds it as #16, with Q09 and Q10.

The research record `docs/plan/review/2026-11-08-wp7-r1-layout-research.md` checked the 24 composite rows against TIA Portal V21. 19 rows agree. C04, A02, A03, A09, and A10 differ. The owner decided on 2026-11-08.

The change satisfies R12, R13, R14, R31, and R38. It uses Assumptions A31 and A38.

## What Changes

- The new ADR-0147 amends ADR-0017. A `SINT` or `USINT` member of a struct starts on an even byte. A `BOOL` array with two dimensions starts each line at the next byte. Row C04 becomes 6 bytes, and row A02 becomes 4 bytes.
- ADR-0147 keeps a `BOOL` array with three dimensions open. Row A03 keeps an unverified mark with the text "needs a TIA measurement".
- The new ADR-0148 amends ADR-0017 and ADR-0120. On `S7_1200` and `S7_1500`, a type text that a command writes cannot have an array as the element of an array. The error is `E-PLT-013`, or `E-PLT-006` for a member of `DefineUdt`.
- A stored type of that kind stays readable under the earlier rule set. A parameter keeps it while its type does not change.
- The new ADR-0149 amends ADR-0116 (e1) and ADR-0119 (b). A parameter command reports only an overlap that holds a parameter that it writes. `ApproveVersion` checks every pair. The export check and the version page check also run the overlap check of I20.
- The reference table gets the source mark `Documented` with the rule ids of the research record. Rows A09 and A10 leave the table.
- `domain-model.md` §2.1 names the new rules.
- An ops query lists the stored rows that the new rules affect. The runbook runs it before an upgrade.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: a type text on `S7_1200` and `S7_1500` cannot hold an array of an array. A stored type of that kind stays readable. The requirement "Type on the platform of the machine" names the rule of ADR-0148 and the stored exception.
- `recipe-management`: the S7 layout size of a struct with a `SINT` or `USINT` member and of a `BOOL` array with two dimensions. The overlap check of a command covers only the parameters that it writes.
- `export`: `loadable` is `false` for an `Approved` version with an overlap.
- `web-ui`: the notice of the version page names an overlap.

## Impact

- Code: `recipe-domain` (`plant/s7_layout.rs`, a new type rule in `plant/`, `recipe/draft/replace.rs`, `export/rule_check.rs`) and `recipe-app` (`AddParameter`, `UpdateParameterDefinition`, `DefineUdt`).
- Tests: `recipe-domain`, `recipe-app`, `recipe-web`, and `recipe-db`.
- Ops: the new query `deploy/db/check-s7-layout-rules.sql` and a runbook section.
- Documents: ADR-0147, ADR-0148, ADR-0149, `docs/adr/README.md`, the Status lines of ADR-0017, ADR-0116, ADR-0119, and ADR-0120, and `domain-model.md` §2.1, §3 (I6, I20), and §5.
- Stored data: no row changes. The computed layout of an affected stored type changes. No content hash changes.
- No migration, no new error code, no new text in the bundles, and no change of the export schema.

## Out of scope

- A TIA Portal measurement of A03, C01, C08, A11, and the header order of `WSTRING`.
- The start of a `SINT` or `USINT` element of an array and of a `SINT` or `USINT` parameter.
- The source marks of the primitive rows P01 to P30.
- The array of an array on `S7_300`.
- R2 to R4 of the fix plan. They need no code change. `design.md` records their results.
