# ADR-0209: Excluded IEC type families and three text fixes

Status: Accepted
Date: 2026-11-19
Deciders: project owner

## Context

The CODESYS and TwinCAT type coverage review of 2026-11-19 found R07, R06, R10, and R11 (`docs/plan/review/2026-11-19-codesys-twincat-type-coverage-review.md` §3). R07 is the IEC twin of the Siemens finding that ADR-0185 closed.

These facts shape the decision:

- A3 makes the matrix of `data-model.md` §7 the closed list of supported types. Every other type is out of scope, but only by implication.
- The note of `data-model.md` §7 names IEC `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY`, and the Siemens families of ADR-0185. It names no CODESYS or TwinCAT family.
- The reference file names native, system, library, and safety types of both vendors. The lines are `:51`, `:187`, `:204`, `:247`, `:259` to `:276`, `:284` to `:289`, `:305` to `:325`, and `:356` to `:366` of `docs/plan/review/sources/CODESYS_TwinCAT3_PLC_Data_Types_EN.md`.
- `data-model.md` §7 says that the domain holds the IEC sizes in code. No code holds them. `Primitive::fixed_size_bytes` gives the S7 size only.
- `api.md` §6.1 states the bytes of a `STRING` and `CHAR` character, but not of a `WSTRING` or `WCHAR` character.
- No document says which dimension of a multi-dimensional array is the outer JSON level. The two vendors use different orders for flat initializers (`:189` to `:196`).
- The walk of a value in `recipe-domain` matches the first dimension with the outer JSON array and the last dimension with the inner arrays.

The project owner decided on 2026-11-19: name the excluded families in the pattern of ADR-0185, and fix the three texts.

## Decision drivers

- A document states each scope decision (R43, A3).
- A document states only what the code does (R43).
- The export document tells the transfer client how to write each value (R03, ADR-0019).

## Considered options

### Option A: One note in `data-model.md` §7 and three text fixes

- Pros: one place names every excluded family. No code change.
- Cons: the note can miss a family of a later vendor release.

### Option B: One Assumption for each family

- Pros: each family gets an A-number.
- Cons: many rows for one decision.

## Decision

The project uses **Option A**.

- (a) The note of `data-model.md` §7 names these families that are not in scope on `CodesysIec` and `TwinCat3`:
  - `BIT`, the native integers `__XINT`, `__UXINT`, `__XWORD`, `XINT`, `UXINT`, `XWORD`, and `PVOID`
  - the vector type `__VECTOR`, the compiler types `VERSION` and `__SYSTEM.*`, and library types such as `SysTime`, `RTS_SYSTIMEDATE`, `T_FILETIME`, `TIMESTRUCT`, `T_AmsNetID`, and `ST_AmsAddr`
  - the DUT forms `UNION` and struct `EXTENDS`, `INTERFACE` variables, and function block instances
  - the generic families `ANY_*`, and the open array `ARRAY[*]`
  - the types of the CODESYS Safety Extension, such as `SAFEBOOL` and `SAFEINT`, and every safety program of either vendor
- (b) ADR-0203 makes some names of item (a) reserved struct names on these platforms. ADR-0207 records why the editor models no layout there.
- (c) The note of `data-model.md` §7 on sizes changes. `size_bytes` holds the S7 size on every platform. The editor holds no CODESYS or TwinCAT size and models no layout there (A31, A53). The note names the vendor sizes that differ:
  - `DATE` and `DATE_AND_TIME` have 4 bytes.
  - `STRING(n)` has `n + 1` bytes, and `WSTRING(n)` has `2(n + 1)` bytes.
- (d) The row `parameters[].value` of `api.md` §6.1 gets one sentence. The transfer client writes each `WSTRING` or `WCHAR` character as its UTF-16 code units. A `WSTRING(n)` length counts these code units (ADR-0137 (i)). ADR-0201 and ADR-0180 limit the characters. The sentence states no byte order.
- (e) `domain-model.md` §2.2, `data-model.md`, and `api.md` §6.1 state the order of the dimensions. The first dimension is the outer JSON level, and the last index changes fastest. For example, `ARRAY[1..2,1..3] OF INT` is a JSON array of 2 arrays of 3 elements. The leaf `[2,1]` is at the JSON position `[1][0]`.
- (f) A test of `recipe-domain` pins item (e) with `ARRAY[1..2,1..3] OF INT`. It accepts `[[1,2,3],[4,5,6]]` and refuses `[[1,2],[3,4],[5,6]]`.
- (g) No other code changes.

## Consequences

### Positive

- A reader finds each excluded CODESYS and TwinCAT family in one note.
- A reader of `data-model.md` §7 does not expect an IEC size in code.
- A transfer client knows the code units of a `WSTRING` value and the nesting order of an array value.

### Negative

- The note can become incomplete after a new vendor release.

### Risks and mitigations

- Risk: a later code change reverses the nesting order of item (e). Mitigation: the test of item (f) fails.

## Related

- ADR-0006 (type catalog), ADR-0137 (text rules), ADR-0185 (excluded Siemens families), ADR-0201 (`WSTRING` characters), ADR-0203 (reserved names), ADR-0207 (symbolic transfer)
- `data-model.md` §7, `domain-model.md` §2.2 (`ParameterValue`), `api.md` §6.1
- `overview.md` §7: R03, R43. §9: A3, A31, A39, A53
