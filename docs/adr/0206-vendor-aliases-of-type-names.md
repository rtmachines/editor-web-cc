# ADR-0206: Vendor aliases of type names

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ08 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- The note of `data-model.md` §7 gives one name to each type in every document. `TIME_OF_DAY` and `DATE_AND_TIME` are never written as `TOD` or `DT`.
- The reference file says that `DT`, `TOD`, `LDT`, and `LTOD` are also type names (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:122` to `:126`, `:143`).
- `TypeExpr::parse` compares a primitive name in exact spelling. Thus `TOD`, `DT`, `LTIME_OF_DAY`, and `LDATE_AND_TIME` give `E-PLT-013`.
- The four aliases are keywords of ADR-0143 (d). Thus no struct can have such a name.

The project owner decided on 2026-11-19: the request boundary normalizes the four aliases to the catalog names. The seed and the export do not change.

## Decision drivers

- A user can enter a type in the spelling of the PLC project (R13).
- One name for each type in the stored data and in every document (`data-model.md` §7).
- One parser holds the grammar (`domain-model.md` §2.1).

## Considered options

### Option A: Normalize at the request boundary

- Pros: the vendor spelling works. The stored data and the export keep one name.
- Cons: the parser accepts two spellings for four types.

### Option B: Keep the grammar

- Pros: no change.
- Cons: the vendor spelling stays an error.

## Decision

The project uses **Option A**.

- (a) `TypeExpr::parse` maps four aliases to the catalog names:

  | Alias | Catalog name |
  |---|---|
  | `TOD` | `TIME_OF_DAY` |
  | `DT` | `DATE_AND_TIME` |
  | `LTIME_OF_DAY` | `LTOD` |
  | `LDATE_AND_TIME` | `LDT` |

- (b) `TypeExpr::parse` is the one place of the mapping. It applies to each request field that carries a `TypeExpr`: `type` of `AddParameter` and `UpdateParameterDefinition`, and `members[i].type` of `DefineUdt`. It also applies to the element of an array.
- (c) The mapping does not read the platform, so it applies on every platform. The check that the type exists on the platform of the machine runs after the mapping.
- (d) The mapping applies only to the `primitive` form. Thus a bare `DT` maps to `DATE_AND_TIME`, and `DT@1` stays a `udt` text.
  - (d1) Each alias is a keyword of ADR-0143 (d) and not a primitive name. A type text `DT@1` that a command writes therefore gives `E-PLT-013` with the reason "reserved name" (ADR-0143 (f), (i2)).
  - (d2) The rule of item (d1) also applies to `TOD@1`, `LTIME_OF_DAY@1`, and `LDATE_AND_TIME@1`.
  - (d3) The preview of the parameter form shows the reason of item (d1) too. The form query checks the type text with the current rule set before it resolves the text. The query skips the check only for an unchanged type text of an edit form.
    A type text is unchanged when the canonical text of the parsed request equals the stored type text of the edited parameter. This rule follows ADR-0143 (i2) and ADR-0186 (i).
    The canonical text is the text that the parsed `TypeExpr` writes. Thus `ARRAY[-0..007] OF DT@1` is unchanged for a stored `ARRAY[0..7] OF DT@1`. `UpdateParameterDefinition` uses the same comparison.
    A new reference to a stored legacy revision is not unchanged.
    Thus the add form and a changed type text of the edit form give "reserved name" for `DT@1`. This result also applies when the machine holds a stored struct `DT@1`. An unchanged `DT@1` keeps its stored revision.
    The check covers each struct reference that the parser of the request accepts. The reserved words come from ADR-0143 (c). On an S7 platform, the check also covers the Siemens reserved names of ADR-0186 (a).
    Thus a missing `Int@9` also gives "reserved name". A missing `BEGIN@9` gives "reserved name" on an S7 platform and "unknown struct" on `TwinCat3`.
    A primitive name in exact case, such as `INT@1`, fails the parser of the request with "unknown struct". It never reaches the form query.
- (e) An alias is upper case, as every primitive name.
- (f) The stored type, the type page, the audit entries, the export, and the seed keep the catalog name. A test pins that a request with `TOD` stores and exports `TIME_OF_DAY`.
- (g) `T`, `D`, and `LD` are literal prefixes and not type names. They stay grammar errors in a `TypeExpr`.

## Consequences

### Positive

- A user can paste a type name from a CODESYS, TwinCAT, or TIA project.
- The stored data and the export keep one name for each type.

### Negative

- A form shows the catalog name after a save of an alias.

### Risks and mitigations

- Risk: a vendor uses an alias that the table does not hold. Mitigation: a new ADR adds it.

## Related

- ADR-0143 (d) (the keyword list), ADR-0205 (long prefixes of literals)
- `domain-model.md` §2.1 (`TypeExpr`), `data-model.md` §7, `reliability.md` §1.2 (`E-PLT-013`)
- `overview.md` §7: R13
