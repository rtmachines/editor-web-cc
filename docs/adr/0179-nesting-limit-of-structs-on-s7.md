# ADR-0179: Nesting limit of structs on S7

Status: Accepted. Amends ADR-0017 (cases of `E-PLT-018`) and ADR-0120 (a) (earlier rule set)
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 asked OQ03 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §4).

These facts shape the decision:

- The Siemens reference file gives "Up to 8 levels" for S7-300, classic S7-1200, and S7-1500 before FW 4.0. It gives 26 levels for S7-1500 with FW 4.0 or later (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:569` and `:570`).
- The same file says: "The top-level structure counts as a level. Under the 26-level rule, an `ARRAY OF STRUCT/UDT` consumes three hierarchy levels" (`:573`). The limits also apply to UDT hierarchies (`:598`).
- The only depth limit of the domain is `json.max_depth` of 32 (`reliability.md` §7.11). It counts one level for each struct and one for each array dimension.
- The project has no firmware setting of a machine. It cannot apply the 26-level rule.
- `E-PLT-018 TypeTooLarge` names a type and a limit. Its text already says "fewer levels".

The project owner decided on 2026-11-17: 8 levels on every S7 platform, with the Siemens count.

## Decision drivers

- An exported type exists in the PLC (R03, R12, R13).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- A limit has one row in `reliability.md` §7.11.

## Considered options

### Option A: 8 levels with the Siemens count on every S7 platform

- Pros: the limit holds on every S7 CPU and firmware.
- Cons: an S7-1500 with FW 4.0 or later accepts more levels than the editor.

### Option B: 8 levels and one level for each struct only

- Pros: a simple count.
- Cons: an array of a struct can pass the editor and fail in TIA Portal.

### Option C: No platform limit

- Pros: no change.
- Cons: the editor accepts a type that the PLC refuses.

## Decision

The project uses **Option A**.

- (a) On `S7_300`, `S7_1200`, and `S7_1500`, a type has at most `type.s7_max_nesting` levels. The value is 8 (`reliability.md` §7.11).
- (b) The S7 count. A struct counts 1 level, also the struct at the top. An array whose element is a struct counts 3 levels in place of the 1 level of that struct. An array of a primitive or a string counts no level. A primitive or a string counts no level.
- (c) The count of a type is the largest sum of the levels on one path from the root to a leaf.
- (d) `DefineUdt` counts the new struct revision as the top level. A struct over the limit gives `E-PLT-018` with the name of the struct and the limit 8.
- (e) `AddParameter` and a changed type of `UpdateParameterDefinition` count the type of the parameter. A type over the limit gives `E-PLT-018` with the type text and the limit 8. This item adds a case to the list of `E-PLT-018` in ADR-0017.
- (f) On the S7 platforms, `RebindTypeRevision` counts the resulting type of each parameter that it moves. An example of a resulting type is `ARRAY[1..2] OF ST@2` in place of `ARRAY[1..2] OF ST@1`. A resulting type over the limit gives `E-PLT-018` with the type text and the limit 8. Then the command changes nothing. A parameter that the command does not move keeps its stored type and its read path (item (g)).
- (g) Stored rows. The earlier rule set accepts a stored struct row and a stored parameter type over the limit. The type page and a stored parameter type stay readable. This item amends ADR-0120 (a).
- (h) The rule has no branch of `RuleSet`. A command checks only the type that it writes (ADR-0166 (t)).
- (i) The snapshot check, `ApproveVersion`, and the export do not run the rule, as for ADR-0148 (g). Thus a stored type over the limit stays approvable and exportable. Only a command that writes or moves a type runs the rule (items (d), (e), and (f)).
- (j) `json.max_depth` stays 32 on every platform. `CodesysIec` and `TwinCat3` get no new limit.
- (k) The ops query `deploy/db/check-s7-layout-rules.sql` lists each stored struct row and each stored parameter type over the limit, in a new part `nesting`.
- (l) The limit of `E-PLT-018` has a kind. The S7 count gets a new kind with the value of `type.s7_max_nesting`. Its message names 8 levels of the S7 count in the bundles `pl` and `en`. The kind of `json.max_depth` keeps its text of 32 levels.

## Consequences

### Positive

- A new type on an S7 machine fits the nesting limit of every S7 CPU.
- No new error code and no migration.

### Negative

- An S7-1500 with FW 4.0 or later gets a lower limit than its CPU allows.
- A stored version with a deeper type stays approvable and exportable.
- A rebind on S7 to a deeper revision fails. The engineer first defines a revision with fewer levels.

### Risks and mitigations

- Risk: the data block of the parameter counts as one more level in TIA Portal. Then a type with 8 levels fails in the PLC. Mitigation: `design.md` of the change `siemens-type-rules` holds the open question. A TIA measurement can lower the value of `type.s7_max_nesting` with a new ADR.
- Risk: the stored data holds a type over the limit. Mitigation: the ops query of item (k) lists it before an upgrade.

## Related

- ADR-0017 (derived types, `E-PLT-018`), ADR-0120 (earlier rule set), ADR-0148 (rule of a written type), ADR-0166 (rules without a branch)
- `domain-model.md` §2.1 (`TypeTree`), `reliability.md` §1.2 (`E-PLT-018`), §4.4, §7.11
- `overview.md` §7: R03, R12, R13, R19, R38
