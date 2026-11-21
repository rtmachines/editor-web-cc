# ADR-0203: Reserved names on CODESYS and TwinCAT

Status: Accepted. Amends ADR-0143 (c) and (d) (reserved struct names on `CodesysIec` and `TwinCat3`). Amends ADR-0143 (l) (reserved member names on these platforms). Amends ADR-0120 (a) (earlier rule set)
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ05 and found R07 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §3 and §4).

These facts shape the decision:

- ADR-0143 (c) to (e) reserve 182 keywords and the 29 primitive names as struct names on every platform. ADR-0186 adds the Siemens names on the S7 platforms only.
- The list of ADR-0143 (d) does not hold `UNION` and `END_UNION`. `UNION` is a DUT form of both vendors (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:247`).
- The reference file names native and library types that no list reserves. These are `BIT` (`:51`), the native integers and `PVOID` (`:259` to `:264`), and the library types of `:305`, `:319`, `:320`, `:323`, and `:325`.
- ADR-0143 (l) lets a member name be a keyword. The Beckhoff page that ADR-0143 (d) cites says that a keyword cannot be a variable name. A struct member is declared as a variable.
- A form of a new struct parses the names before the use case reads the platform of the machine.

The project owner decided on 2026-11-19: on `CodesysIec` and `TwinCat3`, reserve these names as struct names, and reserve the keywords as member names. The earlier rule set reads stored names, as ADR-0186 does on S7.

## Decision drivers

- A struct of the editor can match a DUT of the PLC (R03, R13).
- The domain checks all external input (R38).
- A rule change does not lock a draft, and a stored type stays readable (ADR-0120, R19).
- One list lives in one place, and a test compares each copy.

## Considered options

### Option A: Reserve the names on `CodesysIec` and `TwinCat3` only

- Pros: the S7 platforms keep their names. The rule follows the PLC of the machine.
- Cons: the rule needs the platform, so it runs in the use case and not in the form parser.

### Option B: Reserve the names on every platform

- Pros: the form parser can refuse the name.
- Cons: an S7 plant loses names such as `VERSION` and member names such as `R` without a reason.

### Option C: No change

- Pros: no work.
- Cons: the editor keeps exporting structs that the PLC project refuses.

## Decision

The project uses **Option A**.

- (a) On `CodesysIec` and `TwinCat3`, a struct name is not one of the 15 names of item (c), in any case. This item amends ADR-0143 (c).
- (b) The 211 reserved words of ADR-0143 (c) to (e) stay reserved as struct names on every platform. The list of ADR-0143 (d) does not change.
- (c) 14 names come from the reference file, and each of them occurs on the line that the list gives. `END_UNION` comes from the decision of the project owner on 2026-11-19 (OQ05). None of the 15 names is in the 211 words of ADR-0143 or in the 269 names of ADR-0186 in any case.
  - (c1) The 2 keywords `UNION` (`:247`) and `END_UNION`. Line 247 names `UNION` only. The reference file does not hold `END_UNION`.
  - (c2) The 13 type names `BIT` (`:51`), `__XINT`, `__UXINT`, `__XWORD`, `PVOID` (`:259` to `:262`), `XINT`, `UXINT`, `XWORD` (`:264`), `VERSION` (`:305`), `RTS_SYSTIMEDATE` (`:319`), `T_FILETIME` (`:320`), `TIMESTRUCT` (`:323`), and `ST_AmsAddr` (`:325`).
- (d) On `CodesysIec` and `TwinCat3`, a member name is not one of 184 keywords, in any case. These are the 182 words of ADR-0143 (d) and the 2 words of item (c1). A member name can still be a primitive name or a type name of item (c2). This item amends ADR-0143 (l).
- (e) On the S7 platforms, ADR-0143 (l) does not change. A member name can be a keyword or a primitive name there.
- (f) `recipe-domain` holds the one copy of the lists of items (c) and (d). The ops query of item (m) holds a second copy, and a test compares both copies.
- (g) Codes. A new struct or a new revision of `DefineUdt` on these platforms can have a name of item (a). Then `DefineUdt` gives `E-PLT-013` with the reason "reserved name". A member name of item (d) gives `E-PLT-013` with the reason "reserved member name". The row of `E-PLT-013` covers both refusals, so no code is new (ADR-0025).
- (h) The form of a new struct does not refuse a name of item (a) or (d), because it does not know the platform. The use case refuses it with item (g).
- (i) A command on these platforms checks the type text that it writes. The texts are those of ADR-0143 (i2).
  - (i1) The check also covers the stored struct revision that the text names, also as an array element or as a member type.
  - (i2) The commands are `AddParameter`, `UpdateParameterDefinition` with a changed type, and `DefineUdt`.
  - (i3) A revision with a name of item (a) or a member of item (d) gives `E-PLT-013` with the reason of item (g).
- (j) The parameter form and the struct form of these platforms propose types. The proposals leave out a stored struct of item (i3). The query use case of each list runs the check of item (i).
  - (j1) The preview of the parameter form runs the check of item (i) before it resolves a type text. Thus a new or changed reference to a revision of item (i3) gives `E-PLT-013` at once.
    The preview skips the check for an unchanged type text of the edit form, as ADR-0206 (d3) states. An unchanged reference keeps its grid.
- (k) Stored rows. A stored struct with a name of item (a) or a member of item (d) stays readable. A stored parameter type keeps it while a command does not change the type. `RebindTypeRevision` keeps it, as in ADR-0186 (i). The earlier rule set accepts it. This item amends ADR-0120 (a).
- (l) The rule has no branch of `RuleSet`. A command use case of `recipe-app` runs it on the data that the command writes, as in items (g) and (i) (ADR-0166 (t)). `TypeName::from_stored` takes no platform. `PlcType::from_stored` reads the platform list of a struct row only to require one platform. It runs no rule of a platform on a struct row. The snapshot check, `ApproveVersion`, and the export do not run the rule, as for ADR-0179 (i).
- (m) The ops query `deploy/db/check-identifier-rules.sql` lists each stored struct on these platforms with a name of item (a), in a new part `iec_reserved_struct_name`. It lists each stored member with a name of item (d), in a new part `iec_keyword_member_name`.

## Consequences

### Positive

- A new struct on `CodesysIec` or `TwinCat3` never has the name of a keyword or of a native or library type of the reference file.
- A new member on these platforms never has the name of a keyword.

### Negative

- A plant on these platforms loses member names such as `R`, `S`, `EN`, `ON`, and `AT`, because the list of ADR-0143 (d) holds them.
- The rule runs in the use case and not in the form parser. Such a name gets `E-PLT-013` and not a field line.
- The bundles `pl` and `en` get the reason "reserved member name".

### Risks and mitigations

- Risk: a vendor library holds a type name that item (c) does not hold. Mitigation: a new ADR adds a name. The list lives in one place.
- Risk: the stored data holds an affected struct or member. Mitigation: the ops query of item (m) lists it before an upgrade.

## Related

- ADR-0025 (reasons without a code), ADR-0120 (earlier rule set), ADR-0143 (identifier rules), ADR-0166 (rules without a branch)
- ADR-0179 (rules that the export does not run), ADR-0186 (Siemens reserved names), ADR-0209 (excluded IEC type families)
- `domain-model.md` §2.1 (`TypeName`, `TypeExpr`), `reliability.md` §1.2 (`E-PLT-013`)
- `overview.md` §7: R03, R13, R19, R38
