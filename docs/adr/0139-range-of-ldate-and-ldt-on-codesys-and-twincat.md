# ADR-0139: Range of LDATE and LDT on CodesysIec and TwinCat3

Status: Accepted. Amends ADR-0026 (base-unit table), ADR-0115 (a fourth range narrower than the vendor range), and ADR-0120 (a) and (g) (earlier rule set, copy of a base)
Date: 2026-11-07
Deciders: project owner

## Context

The PLC data type review of 2026-11-06 found a range fault and two table faults. `docs/plan/review/2026-11-06-plc-type-review.md` holds them as #20, #18, and #19.

These facts shape the decision:

- The base-unit table of `domain-model.md` §2.1 gives `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` the count `0..=2^64-1` ns. The code holds the same range (`crates/recipe-domain/src/plant/iec_literal/range.rs`, constants `LDATE` and `LDT_UNSIGNED`).
- The count `2^64-1` ns ends on 2554-07-21. The editor therefore accepts `LDT#2300-01-01-00:00:00` on `TwinCat3`, and an approved export can hold it.
- CODESYS and Beckhoff give `LDT#1677-9-21-0:12:43.145224192` to `LDT#2262-4-11-23:47:16.854775807` for `LDT`. They give `LD#1677-9-22` to `LD#2262-4-11` for `LDATE`. Both ranges are a signed 64-bit count of nanoseconds.
- A43 keeps a count non-negative. The editor therefore refuses the dates 1677 to 1969, and no ADR records this narrowing.
- `LDT` on `S7_1500` and `DTL` already count `0..=2^63-1` ns (ADR-0115 (b)).
- The `LTOD` row of the table gives the range `0..=2^64-1` and also "below one day" (#18). The code uses `0..=86_399_999_999_999` on every platform with `LTOD`. CODESYS, Beckhoff, and Siemens end `LTOD` at `23:59:59.999999999`.
- `data-model.md` §7 says that the width follows the S7 size, with `D` for 4 bytes (#19). Thus `STRING(2)` with 4 bytes would get `D`. The width table of `domain-model.md` §2.1, the type matrix of §7, and the code give `B` to every `STRING` and `WSTRING`.

The project owner decided on 2026-11-06 (fix plan, question Q-A, option (1)).

## Decision drivers

- An exported value loads into the controller (R03, R11, R12).
- A count is never negative (A43). The export document, the content hash, and the UI literal do not change.
- A rule change does not lock a draft, and a stored value stays readable (ADR-0028, ADR-0118, ADR-0120).
- Every range in code has a row in a normative document (ADR-0026).

## Considered options

### Option A: The count range 0 to 2^63-1

- Pros: the range of `LDT` on `S7_1500` and of `DTL`. A43, the export schema, and the UI literal stay as they are.
- Cons: the editor refuses the dates 1677-09-21 to 1969-12-31, which the platform accepts.

### Option B: The full vendor range with negative counts

- Pros: every value of the platform is valid.
- Cons: A43, the export schema, the canonical text, and the UI literal change. A transfer client must read a negative count.

## Decision

The project uses **Option A**.

- (a) On `CodesysIec` and `TwinCat3`, an `LDT` count is `0..=9_223_372_036_854_775_807` ns. That is 1970-01-01-00:00:00 to 2262-04-11-23:47:16.854775807.
- (b) On `CodesysIec` and `TwinCat3`, an `LDATE` count is in the same range and is a whole number of days. The last `LDATE` is 2262-04-11.
- (c) A count is never negative (A43). The refusal of 1677-09-21 to 1969-12-31 is a deliberate narrowing, in the pattern of ADR-0115. This item adds it as a fourth range to the three ranges of ADR-0115.
- (d) A count outside the range gives `E-RCP-005` for a value and `E-RCP-006` for a limit (`reliability.md` §4.3).
- (e) The earlier rule set of ADR-0120 keeps the range `0..=2^64-1` for `LDATE` and `LDT` on `CodesysIec` and `TwinCat3`. This item amends ADR-0120 (a).
- (f) The command paths follow ADR-0120 (c) to (h). A command checks a written value or limit with the current set. It checks a stored part that it only reads with the earlier set. `ApproveVersion` uses the current set and refuses a stored count above the range with an item of `E-RCP-020`. `CreateVersion` copies such a base. This item amends ADR-0120 (g).
- (g) A read shows such a stored part with a field line (ADR-0118). The export sets `loadable` to `false` (ADR-0119).
- (h) The `LTOD` row of the base-unit table gives `0..=86_399_999_999_999` ns on every platform with `LTOD`. The code already uses this range.
- (i) `data-model.md` §7 says that every `STRING` and `WSTRING` uses the width `B`. The code already uses this width.
- (j) Items (h) and (i) change the text of two tables. They add no rule.
- (k) No migration and no new error code. No stored value, limit, or content hash changes.

## Consequences

### Positive

- An approved `LDATE` or `LDT` value on `CodesysIec` and `TwinCat3` loads into the controller.
- `LDT` and `DTL` share one count range on every platform.
- The tables of `domain-model.md` and `data-model.md` agree with the code.

### Negative

- A recipe cannot hold an `LDATE` or `LDT` value before 1970-01-01.
- The old end `2^64-1` of both types stays in the code as a named constant of the earlier set.

### Risks and mitigations

- Risk: a stored value or limit is above the new end. Mitigation: the ops query of the change lists such rows before the upgrade. The read shows a field line, and `ApproveVersion` refuses the value.
- Risk: a process needs a date before 1970. Mitigation: a new ADR adds negative counts with Option B.

## Related

- ADR-0026 (ranges of `DATE` and `DATE_AND_TIME`), ADR-0115 (ranges narrower than the vendor range), ADR-0118 (reads), ADR-0119 (export), ADR-0120 (earlier rule set)
- `domain-model.md` §2.1 (base-unit table, width table), `data-model.md` §7
- `overview.md` §7: R03, R11, R12. §9: A43
