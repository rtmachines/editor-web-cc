# Proposal

## Why

The CODESYS and TwinCAT review of 2026-11-19 found types and values that the editor accepts and a `CodesysIec` or `TwinCat3` controller refuses. ADR-0199 to ADR-0209 record the owner decisions of 2026-11-19 and the answers of the plan review of 2026-11-20. They stay "Proposed" until task 1.0. The change satisfies R03, R09, R11, R12, R13, R15, R19, R20, R28, R31, R36, R38, R39, and R43. It uses A2, A3, A9, A24, A25, A31, A43, A46, A51, A52, and A53.

## What Changes

- A machine gets a `STRING` encoding, `Latin1` or `Utf8` (ADR-0202). A `Utf8` machine counts a `STRING(n)` length in UTF-8 bytes. `SetMachineStringEncoding` changes it only while the machine has no version. New codes: `E-PLT-053`, `E-PLT-054`.
- Only the v2 document of a `Utf8` machine holds `machine.string_encoding`. Its v1 document has `loadable` false. ADR-0202 adds no byte to a document of a `Latin1` machine.
- On `CodesysIec` and `TwinCat3`, a new `WSTRING` value holds only UCS-2 characters (ADR-0201). A command refuses 15 struct names and 184 member keywords in the data that it writes (ADR-0203).
- A struct on `CodesysIec` has at least 2 members (ADR-0204).
- The parsers accept 8 long literal prefixes and 4 type aliases (ADR-0205, ADR-0206).
- The export schema v2 gets type, length, member, and value kind rules (ADR-0208).
- ADR-0199, ADR-0200, ADR-0207, and ADR-0209 change no code.
- Pin tests close review items R02 to R05 and R09. Four new ops query parts.
- The runbook permits a rollback to the previous image on the newer schema only when two counts are 0 (ADR-0202 (q3)).
  - The machines with `machine.string_encoding = 'Utf8'`.
  - The audit entries with `audit_log.action = 'MachineStringEncodingSet'`.
- A test matrix shows that stored rows stay readable and keep their hashes.

## Capabilities

### New Capabilities

- None.

### Modified Capabilities

- `plant-model`: `STRING` encoding, reserved names, two members on `CodesysIec`, type aliases.
- `recipe-management`: `WSTRING` characters, `STRING` values on a `Utf8` machine, literal prefixes, array dimensions.
- `export`: the v2 encoding field, `loadable` of v1 on a `Utf8` machine, schema v2 rules, `STRING` bytes, no layout on CODESYS and TwinCAT.
- `web-ui`: encoding on the machine form and page, the notice of a v1 document, new reasons and expected forms.
- `operations`: encoding after the upgrade, ops query parts, the rollback check, the checks before a plant adds a CODESYS machine.

## Impact

- All six crates change.
- Two migrations, `0017` and `0018`, add the column. `crates/recipe-db/tests/grants.expected` and `deploy/pgtools/restore-roles.expected` get one row each. `deploy/pgtools/restore-code.expected` gets two rows.
- `deploy/db/` gets new query parts. `docs/ops/runbook.md` names them.
- No stored hash and no v1 schema change.
- New golden files of the export v2 for a `Latin1` and a `Utf8` machine.

## Out of scope

- A machine setting for the `LREAL` size or the CODESYS version (ADR-0199, ADR-0200).
- A layout model, `pack_mode`, or `EXTENDS` on `CodesysIec` and `TwinCat3` (ADR-0207).
- The type families of ADR-0209 (a).
- `CHAR` and `WCHAR` on `CodesysIec` and `TwinCat3`.
- A range check in the schema v2.
