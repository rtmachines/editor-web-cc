# Proposal

## Why

The S7 type review of 2026-11-03 (`docs/plan/review/plc-s7-type-coverage-review.md` §3) found four gaps in the tests, F08 to F11.
No test compares `S7Layout` with a reference table, as ADR-0017 requires (F08).
Some layout cases have no golden test (F09).
The export golden tests use few types (F10).
No storage test reads a 64-bit extreme back from JSONB (F11).

This change closes F10 and F11. It closes F08 and F09 only in part.
ADR-0017 requires a reference table from the Siemens manual. The composite rows and all start classes stay "unverified, Q10".
Q10 stays an open item until the owner supplies a TIA Portal export.

A wrong layout or a changed value can reach a PLC through the export (R03, R14, R31).
The tests cover the S7 types of R12, the arrays and UDTs of R13, and the limits of R08 and R09.
They use the type matrix of A3, the address rule of A31, the dimensions of A38, and the base units of A43.

## What Changes

- `recipe-domain`: a reference table of `S7Layout` in test code. Each primitive size cites the Siemens source S1. Each alignment rule and each composite row is marked "unverified, Q10".
- `recipe-domain`: a table test and the property test of ADR-0017 against the reference table.
- `recipe-domain`: the layout rows of F09. These are `WSTRING(n)`, multi-dimensional `ARRAY OF BOOL`, an array of an array, and an odd `STRING(n)` before a `BYTE`.
- `recipe-export`: one golden document for each S7 platform with one parameter for each available primitive and its limit.
- `recipe-db`: contract cases that write the 64-bit extremes and read them back through both value decoders and the limit columns.

- `recipe-domain`: the doc comment of `S7Layout::of` names the reference table and says that its composite rows are not verified (design D7).

The tests check the current behavior. A test that fails against the current code stops its task, and the task reports a finding. No task changes production code, except the doc comment of task 1.5.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

None. The change adds tests only, so `.openspec.yaml` sets `skip_specs: true`.

## Impact

Test code, test data, and one doc comment:

- `crates/recipe-domain/src/plant/s7_layout/` (new test modules, one `mod` line in `s7_layout.rs`).
- `crates/recipe-domain/src/plant/s7_layout.rs` (the doc comment of `S7Layout::of`, task 1.5).
- `crates/recipe-export/tests/` (one new test file and three golden files).
- `crates/recipe-db/tests/storage/contract/` (new contract cases).

No production code changes, except the doc comment of task 1.5. No migration, ADR, or `docs/architecture/` file changes.

## Out of scope

- The rules of the parallel change `plc-type-ranges`: the `DATE` end 2168-12-31, the address end rule, the `S7_300` DB limit 16000, and the `REAL` and `LREAL` digit limits. No test value hits these rules.
- The open questions Q01 to Q09 of the review. The tests pin the current behavior of Q06, Q07, and Q09 and name the question.
- A TIA Portal export for Q10. The reference rows stay "unverified, Q10" until the owner supplies it.
- The other findings F01 to F07 and F12 to F15.
