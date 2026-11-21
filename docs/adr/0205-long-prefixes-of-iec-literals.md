# ADR-0205: Long prefixes of IEC literals

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 asked OQ07 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §4).

These facts shape the decision:

- The grammar of `IecLiteral` in `domain-model.md` §2.1 accepts one short prefix for each type, for example `LT#` for `LTIME`.
- The reference file gives the literal `LTIME#1us` (`docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md:135`). It gives no other long prefix. The review cites vendor pages for the other long forms.
- A user who pastes `LTIME#1us` gets `E-RCP-005`. The same value is valid as `LT#1us`.
- The export holds the count of the base unit, not the literal (A43).

The project owner decided on 2026-11-19: accept the long prefixes as input aliases. The canonical form, the counts, and the hashes do not change.

## Decision drivers

- A user can enter a value in the spelling of the PLC project (R13).
- The stored data, the export, and the hashes do not change (`reliability.md` §4.4).
- One parser holds the grammar (`domain-model.md` §2.1).

## Considered options

### Option A: Input aliases, canonical output unchanged

- Pros: the vendor spelling works. Pages, counts, and hashes stay the same.
- Cons: the parser accepts two spellings for each prefix.

### Option B: Keep the grammar

- Pros: no change.
- Cons: the vendor spelling stays an error.

## Decision

The project uses **Option A**.

- (a) `IecLiteral::parse` accepts 8 long prefixes as aliases of the short prefixes:

  | Long prefix | Short prefix | Type |
  |---|---|---|
  | `TIME#` | `T#` | `TIME` |
  | `LTIME#` | `LT#` | `LTIME` |
  | `DATE#` | `D#` | `DATE` |
  | `LDATE#` | `LD#` | `LDATE` |
  | `TIME_OF_DAY#` | `TOD#` | `TIME_OF_DAY` |
  | `LTIME_OF_DAY#` | `LTOD#` | `LTOD` |
  | `DATE_AND_TIME#` | `DT#` | `DATE_AND_TIME` |
  | `LDATE_AND_TIME#` | `LDT#` | `LDT` |

- (b) `S5T#` and `DTL#` get no long prefix.
- (c) An alias gives the same count as its short prefix. Every other rule of the literal grammar applies unchanged.
- (d) A prefix is upper case, as before. `ltime#1us` stays a grammar error.
- (e) `Display` writes the short prefix. Thus `Display` of a literal with a long prefix gives the canonical literal with the short prefix. A canonical literal still round-trips through `parse` and `Display`.
- (f) `IecLiteral::parse` is the one place of the aliases. It does not read the platform, so the aliases apply on every platform. The range of the base-unit table still depends on the platform.
- (g) The stored text, the export, `content_sha256`, `scope_sha256`, and the audit entries do not change.
- (h) A limit accepts the aliases too, because it uses the same parser. A refused limit still gives `E-RCP-006`.

## Consequences

### Positive

- A user can paste a literal from a CODESYS or TwinCAT project.
- No stored value and no hash changes.

### Negative

- A page shows the short prefix after a save of a long prefix.

### Risks and mitigations

- Risk: a vendor uses a long prefix that the table does not hold. Mitigation: a new ADR adds it.

## Related

- ADR-0183 (decimal text only for `Numeric` leaves)
- `domain-model.md` §2.1 (`IecLiteral`), `reliability.md` §1.2 (`E-RCP-005`, `E-RCP-006`), §4.4
- `overview.md` §7: R13. §9: A43
