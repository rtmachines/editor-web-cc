# Proposal: plc-type-ranges

## Why

The review `docs/plan/review/plc-s7-type-coverage-review.md` (F01 to F07) shows S7 ranges and address rules that differ from Siemens. The owner decided each point. This change records the decisions in ADRs and applies them.

The change satisfies R12, R14, and R38. It keeps R03 and R31. It uses A2, A31, and A43.

## What Changes

- **BREAKING** `DATE` on `S7_1200` and `S7_1500` ends at 2168-12-31, the count 72 683 (F01, Q01). ADR-0114 amends ADR-0026.
- ADR-0115 keeps three restrictions: `DATE_AND_TIME` in whole seconds (F02), `DTL` to 2262-04-11-23:47:16.854775807 (F03), and `REAL` and `LREAL` in the `Decimal` range (F06).
- **BREAKING** ADR-0116: an S7 parameter ends at byte 65 535 of its data block (`E-RCP-056`, F04). On `S7_300`, the data block number is at most 16 000 (`E-RCP-057`, F05).
- **BREAKING** A `REAL` number has at most 7 significant digits, an `LREAL` number at most 15 (`E-RCP-055`, F07). ADR-0117 amends ADR-0028.
- A command checks the data that it writes with the current rules. A stored part that it only reads uses the earlier rule set. `CreateVersion` copies an affected base (ADR-0120).
- A page shows an affected stored part with a field line (ADR-0118).
- The parameter table reads the value of every row of one page for the read check (ADR-0121).
- The check of the parameter table runs inside `timeout.validation`. The check of the version page of an `Approved` version takes a permit of `export.permits` and runs inside `timeout.export` (ADR-0122).
- **BREAKING** `loadable` is `false` when a value, a limit, or an address breaks a current rule. The hash does not change. The version page shows a notice (ADR-0119).
- Tasks update `reliability.md`, `domain-model.md`, `data-model.md`, A43, `api.md`, and `CHANGELOG.md`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: seven new requirements for the ranges, digits, and address rules, and for commands on stored data that breaks a newer rule.
- `export`: the requirement "Loadable flag" follows the current rules.
- `web-ui`: field lines in the parameter table and the other reads, and the notice of the version page.

## Out of scope

- The known gaps Q06 to Q10 of the review.
- The tests of F08 to F11. The change `plc-type-test-coverage` owns them.
- Findings F12 to F15.
- A data block limit per CPU model, and a database constraint for the new rules.
- A rewrite of stored versions. Approved versions stay immutable (ADR-0009).

## Impact

- `recipe-domain`: rule sets, ranges, value and limit checks, S7 rules, error codes, read check, `loadable`.
- `recipe-app` and `recipe-web`: blockers, HTTP status, field lines, the notice, and the `pl` and `en` texts.
- `recipe-export`: one rule of the v1 and v2 schema files.
