# Proposal

## Why

The review of 2026-11-21 (§4) reports faults that need a design decision. The fix plan §3.2 gives them to this change, with the rest of changes A and C.

The change satisfies R08, R11, R12, R13, R19, R28, R29, R30, R31, R32, R34, R35, R36, R38, and R39. It uses A22, A26, A33, A35, and A38.

## What Changes

- V-01: a command refuses LF and CR in a text value. A stored value stays readable.
- V-04: a new key limits the leaf path length. The value body limit follows from the budgets. A permit limits the value bodies that the server reads at the same time.
- V-03: the trigger `plc_type_immutable` compares the transaction and `created_at`.
- V-02: `pitr-volume` refuses a recovery that ends before the newest archived record.
- V-15: a browser proof comes first. A conditional ADR follows.
- V-05: without a type change, the value fields and the limit fields follow the pair rule of the owner.
- V-06: `restore-test` and `pitr-check` delete the scratch cluster only after a successful stop.
- V-07: `backup-health` refuses a manifest time in the far future.
- V-16: one ADR permits the `from_bytes` constructors of two tokens.
- V-18: a second ADR gives `ThrottlePolicy` a grammar and changes its API.
- V-08, V-20, V-22, V-24, V-27, V-28, V-30, V-31: one ADR of amendments aligns the documents with the code. V-08 also adds one clause to `CLAUDE.md` and `.claude/rules/sql.md`.
- The w13 note, V-23: one ADR records two known limits.
- A-05, A-09, A-10, A-11: two test guards get stricter.
- ADR-0074: a Status line typo gets its fix.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: three requirements on text values and type changes.
- `plant-model`: the leaf path length and the trigger of struct members.
- `web-ui`: "Body limit per route" gets the computed value limit and the permit of the value routes.
- `identity-access`: "Request body limits" follows the value limit.
- `operations`: an early end of a recovery, a failed scratch stop, and a manifest time ahead of the clock.

## Impact

- ADR-0210 to ADR-0220 (ADR-0220 only for class R of the V-15 proof), `docs/adr/README.md`, and five files of `docs/architecture/`.
- `CLAUDE.md` and `.claude/rules/sql.md`: one clause each on the read before the command transaction.
- Code in five crates and `e2e/`.
- Migration `0019`, `deploy/pgtools/`, `deploy/ci/`, `deploy/db/`, `docs/ops/runbook.md`, and `CHANGELOG.md`.
- New error code `E-IDN-012`. V-01 reuses `E-RCP-005`, and V-04 reuses `E-PLT-018` and `E-WEB-009` (new stage `ValueBodyPermit`). New §7.11 keys `budget.leaf_path_bytes` and `http.value_body_permits`.

## Out of scope

- The findings of changes A and C.
- A rewrite of stored values with LF or CR. An ops query lists them.
- The index form of the field names of ADR-0039.
- Any other edit of `CLAUDE.md` or `.claude/rules/`.
