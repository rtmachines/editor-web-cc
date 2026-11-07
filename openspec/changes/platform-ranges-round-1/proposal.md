# Proposal

## Why

The PLC data type review of 2026-11-06 found six faults in the platform ranges. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 7 (WP4) groups them:

- #20 (CRITICAL): `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` count up to 2^64-1 ns. The vendors end both types at 2262-04-11-23:47:16.854775807, which is 2^63-1 ns.
- #8 (HIGH): the editor accepts the data blocks 60000 to 65535 on `S7_1200` and `S7_1500`. The user range ends at 59999.
- #13 (HIGH): no document records the string length 255 on `CodesysIec` and `TwinCat3` as deliberate.
- #21 (MEDIUM): `LDATE`, `LTOD`, and `LDT` need TwinCAT 3.1.4026.0 or later. No document names this version.
- #18, #19 (LOW): the `LTOD` row of `domain-model.md` §2.1 and the width text of `data-model.md` §7 disagree with the code.

The owner chose option (1) of the questions Q-A, Q-B, Q-C, and Q-G on 2026-11-06. The change satisfies R03, R11, R12, R13, R14, and R38. It uses A31 and A43, and it adds A46.

## What Changes

- ADR-0139 ends `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` at 2^63-1 ns. It also changes the `LTOD` row and the width text. It amends ADR-0026, ADR-0115, and ADR-0120 (a) and (g).
- ADR-0140 ends the data block number at 59999 on `S7_1200` and `S7_1500`. The message of `E-RCP-057` names the limit of the platform. It amends ADR-0116 (c) and ADR-0120 (a).
- ADR-0141 records `STRING(n)` and `WSTRING(n)` `1..=255` on `CodesysIec` and `TwinCat3` as a deliberate narrowing. It amends ADR-0115 with a fifth range. The code does not change.
- ADR-0142 adds Assumption A46: every `TwinCat3` machine runs TwinCAT 3.1.4026.0 or later. The runbook names the check.
- The earlier rule set keeps the old ranges. A stored value or address stays readable, and a draft does not lock. `ApproveVersion` refuses it.
- An ops query lists the stored rows that the new ranges refuse. The merge needs 0 rows or the sign-off of the owner on each row.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: the range of `LDATE` and `LDT`, the data block number on S7, and the copy of an affected base.
- `plant-model`: the string length on `CodesysIec` and `TwinCat3`.
- `export`: `loadable` for a stored part that breaks a new range.

## Impact

- Code: `recipe-domain` (`plant/iec_literal/range.rs`, `recipe/draft/s7.rs`, `recipe/error.rs`) and `recipe-web` (`error/recipe.rs`, `i18n/pl.ftl`, `i18n/en.ftl`).
- Tests: `recipe-domain`, `recipe-web`, and `recipe-db`.
- Ops: `deploy/db/check-platform-ranges.sql` and two sections of `docs/ops/runbook.md`.
- Documents: ADR-0139 to ADR-0142, `docs/adr/README.md`, the Status lines of four ADRs, `domain-model.md`, `data-model.md`, `overview.md`, and `reliability.md` §1.2.
- No migration, no new error code, and no new setting. No stored row or content hash changes.

## Out of scope

- Negative counts for `LDATE` and `LDT` (option (2) of Q-A).
- A firmware or runtime attribute of a machine (Q-C (3), Q-G (2)).
- A string length above 255 (option (2) of Q-B).
- Vendor point R4 of the fix plan and the other WP7 points.
- The follow-ups F-1 to F-7 of the fix plan, and the findings of the other work packages.
