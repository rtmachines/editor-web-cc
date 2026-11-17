# ADR-0185: Excluded Siemens type families and the standard program

Status: Accepted
Date: 2026-11-17
Deciders: project owner

## Context

The Siemens type coverage review of 2026-11-17 found R04 (`docs/plan/review/2026-11-17-siemens-type-coverage-review.md` §3). R04 continues the baseline item F13.

These facts shape the decision:

- A3 makes the matrix of `data-model.md` §7 the closed list of supported types. Every other type is out of scope, but only by implication.
- The note of `data-model.md` §7 names only IEC `ENUM`, subrange, alias, `POINTER`, `REFERENCE`, and `ANY`.
- The review lists 247 Siemens types outside the matrix. A reader cannot tell a decision from an omission.
- The Siemens reference file says that a type of the standard program is not always valid in the fail-safe program. The F-FB, the F-FC, and the F-DB have their own type set (`docs/plan/review/sources/Siemens_PLC_Data_Types_Detailed_EN.md:1701`).
- `PlcPlatform` has no mark of a fail-safe data block. The editor does not know the F rules.

The project owner decided on 2026-11-17: name the excluded families, and add an Assumption for the standard program.

## Decision drivers

- A document states each scope decision (R43, A3).
- The editor never writes data that the PLC treats in another way (R03, R31).

## Considered options

### Option A: One note in `data-model.md` §7 and one Assumption

- Pros: one place names every excluded family. No code change.
- Cons: the note can miss a family of a later TIA version.

### Option B: One Assumption for each family

- Pros: each family gets an A-number.
- Cons: many rows for one decision.

## Decision

The project uses **Option A**.

- (a) The note of `data-model.md` §7 names these excluded families:
  - parameter and block types: `VARIANT`, `REF_TO`, `TIMER`, `COUNTER`, `BLOCK_FC`, `BLOCK_FB`, `BLOCK_DB`, `BLOCK_SDB`, `VOID`, `PARAMETER`, `INSTANCE`, multi-instances, and formal block parameters
  - system data types of Siemens §8: IEC timer and counter types, system structures, `C_ALARM*`, and `C_NOTIFY*`
  - system identifiers of Siemens §8: `HW_*`, `EVENT_*`, `OB_*`, `CONN_*`, `DB_ANY`, `DB_WWW`, `DB_DYN`, `REMOTE`, `RTM`, and `PIP`
  - communication and technology types of Siemens §9: `TCON_*`, `TADDR_*`, `IF_CONF_*`, `Conf_*`, `TMAIL_*`, `OPC_UA_*`, the technology objects `TO_*`, and the FB instance types such as `PID_Compact`
  - the open array `ARRAY[*]` and the named value type (NVT) of S7-1500
  - every fail-safe construct: F data types, F-UDTs, F arrays, F open arrays, and F named value types
- (b) New Assumption A50: the editor models the standard program only. A recipe parameter never targets an F-DB.
- (c) ADR-0186 makes the names of item (a) reserved struct names on the S7 platforms.
- (d) No code changes. This ADR closes the baseline item F13.

## Consequences

### Positive

- A reader finds each excluded family in one note.
- The fail-safe program is out of scope by a decision.

### Negative

- The note can become incomplete after a new TIA version.

### Risks and mitigations

- Risk: a user enters the address of an F-DB. Mitigation: A50 names the rule, and the plant keeps F-DBs out of the recipe blocks.

## Related

- ADR-0006 (type catalog), ADR-0186 (reserved struct names on S7)
- `data-model.md` §7, `overview.md` §9: A3, A39, A50
- `overview.md` §7: R03, R31, R43
