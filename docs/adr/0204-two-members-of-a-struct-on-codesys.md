# ADR-0204: Two members of a struct on CODESYS

Status: Accepted. Amends ADR-0120 (a) (earlier rule set)
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ06 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- `domain-model.md` §2.1 gives a struct 1 to 1024 members on every platform. A struct with no member gives `E-PLT-016`.
- The CODESYS page "Structure" gives "at least 2" declarations in its syntax table. The reference file links that page (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:238`). The reference file gives no minimum itself.
- The review found no source for a minimum of 2 on TwinCAT.
- The message of `E-PLT-016` says that a type has no members. It does not fit a struct with one member.

The project owner decided on 2026-11-19: a struct on `CodesysIec` has at least 2 members. `TwinCat3` keeps 1. The earlier rule set reads stored structs with one member.

## Decision drivers

- A struct of the editor can match a DUT of the PLC (R03, R13).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- Each refusal uses a code of `reliability.md` §1.2 (R36).

## Considered options

### Option A: At least 2 members on `CodesysIec`

- Pros: the struct follows the CODESYS syntax table.
- Cons: a CODESYS plant cannot define a struct with one member.

### Option B: No change

- Pros: no work.
- Cons: the editor can export a struct that the CODESYS compiler refuses.

## Decision

The project uses **Option A**.

- (a) On `CodesysIec`, a struct has 2 to 1024 members. Every other platform keeps 1 to 1024.
- (b) `DefineUdt` on a `CodesysIec` machine with one member gives `E-PLT-013` with the reason "too few members". The row of `E-PLT-013` covers this refusal, so no code is new (ADR-0025). A struct with no member still gives `E-PLT-016` on every platform.
- (c) A command on a `CodesysIec` machine checks the type text that it writes. The texts are those of ADR-0143 (i2).
  - (c1) The check also covers the stored struct revision that the text names, also as an array element or as a member type.
  - (c2) The commands are `AddParameter`, `UpdateParameterDefinition` with a changed type, and `DefineUdt`.
  - (c3) A revision with one member gives `E-PLT-013` with the reason "too few members".
- (d) The parameter form and the struct form of a `CodesysIec` machine propose types. The proposals leave out a stored struct with one member. The query use case of each list runs the check of item (c).
  - (d1) The preview of the parameter form runs the check of item (c) before it resolves a type text. Thus a new or changed reference to a stored struct with one member gives `E-PLT-013` at once.
    The preview skips the check for an unchanged type text of the edit form, as ADR-0206 (d3) states. An unchanged reference keeps its grid.
- (e) Stored rows. A stored struct with one member stays readable. A stored parameter type keeps it while a command does not change the type. `RebindTypeRevision` keeps it, as in ADR-0186 (i). The earlier rule set accepts it. This item amends ADR-0120 (a).
- (f) The rule has no branch of `RuleSet`. A command use case of `recipe-app` runs it on the data that the command writes, as in items (b) and (c) (ADR-0166 (t)). `PlcType::from_stored` runs no rule of a platform on a struct row. The snapshot check, `ApproveVersion`, and the export do not run the rule, as for ADR-0179 (i).
- (g) The trigger `plc_type_struct_has_member` of `data-model.md` §4.6 does not change. It still requires at least one member on every platform.
- (h) The ops query `deploy/db/check-identifier-rules.sql` lists each stored struct with one member on a `CodesysIec` machine, in a new part `codesys_one_member_struct`.

## Consequences

### Positive

- A new struct on a `CodesysIec` machine follows the CODESYS syntax table.

### Negative

- A CODESYS plant adds a second member to a struct with one member.
- The bundles `pl` and `en` get the reason "too few members".

### Risks and mitigations

- Risk: the CODESYS compiler of the plant accepts a struct with one member. Mitigation: a new ADR can remove the rule.
- Risk: the stored data holds a struct with one member. Mitigation: the ops query of item (h) lists it before an upgrade.

## Related

- ADR-0025 (reasons without a code), ADR-0120 (earlier rule set), ADR-0143 (i2) (type texts that a command writes)
- ADR-0166 (rules without a branch), ADR-0179 (rules that the export does not run)
- `domain-model.md` §2.1 (`PlcType`), `data-model.md` §4.6, `reliability.md` §1.2 (`E-PLT-013`, `E-PLT-016`)
- `overview.md` §7: R03, R13, R19, R36
