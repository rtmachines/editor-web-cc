# Proposal

## Why

The Siemens type coverage review of 2026-11-17 found types and values that the editor accepts and an S7 CPU refuses. The owner decided each open question on 2026-11-17. This change records the decisions in ADR-0177 to ADR-0189 and adds the rules. The ADRs have the status "Proposed" until the owner accepts them (task 1.0). It satisfies R03, R11, R12, R13, R19, R31, R38, and R43. It uses A3, A31, A38, A49, and A50.

## What Changes

- `S7_300` also refuses an array of an array (ADR-0177).
- On S7, a parameter with an address has each array bound in `-32768..=32767` (ADR-0178). Its type holds no `BOOL` array with three dimensions (ADR-0187). A break gives the new code `E-RCP-058`. Approval and the export check run both rules.
- A type on S7 has at most 8 levels in the Siemens count (ADR-0179). `RebindTypeRevision` also counts each moved type. A break gives `E-PLT-018`.
- On `S7_1200` and `S7_1500`, a `WCHAR` is in `U+0001..=U+D7FF`. A `WSTRING` code unit is in `0x0001..=0xCFFF` (ADR-0180). Stored values keep ADR-0137.
- 269 Siemens names and the telegram family `PD_TEL<n>` are reserved struct names on S7 (ADR-0186).
- The export schema v2 gets a pattern of `s7_address` (ADR-0189).
- ADR-0181 to ADR-0185 and ADR-0188 change no code. They record decisions and two Assumptions.
- Three ops queries get new parts for stored rows that break a new rule.
- A matrix of tests shows that each stored row stays readable and exports with its stored hash, also after a restore.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `plant-model`: array of an array on `S7_300`, nesting limit, Siemens reserved struct names, `STRING(n)` notation.
- `recipe-management`: rules of a parameter with an address, `WCHAR` and `WSTRING` on S7, decimal text.
- `export`: the v2 pattern of `s7_address`, `loadable`, Latin-1 bytes.
- `web-ui`: the expected form of a refused wide text value on S7, the S7 nesting limit in `E-PLT-018`, and the line of `E-RCP-058`.

## Impact

- `recipe-domain`, `recipe-app`, `recipe-web`, and `recipe-export` change. The error registry test of `recipe-server` gets one sample.
- `deploy/db/` gets new query parts. The `recipe-db` tests check them and the stored data matrix.
- No migration. No change of the export hash or of the v1 schema file.

## Out of scope

- A setting of the digits of `REAL` (OQ08).
- The S7-1200 G2, a firmware setting, and the fail-safe program.
- A code page per machine.
- Based literals, typed literals, and `STRING[n]`.
- A TIA Portal measurement of a `BOOL` array with three dimensions.
