# Design

## Context

See `proposal.md`, section "Why". `docs/plan/2026-11-06-plc-type-fix-plan.md` section 4 holds the fix direction. The rules exist in `reliability.md` §4.4, `domain-model.md` §2.1, ADR-0117, ADR-0118, ADR-0119, and ADR-0120. This design links to them and does not repeat them.

The state of the code at HEAD `47731e8`. The author read each place in the source:

| Finding | State | Place |
|---|---|---|
| #2 | `apply_exponent` adds the exponent to `base.normalize().scale()`. `normalize` keeps the trailing zeros of the whole part, so `150e-29` gets the scale 29 and gives `None`. The exact value `1.5e-27` has the scale 28 | `crates/recipe-domain/src/recipe/value/decimal.rs` lines 73 to 78 |
| #3 | `Decimal::from_str_exact` refuses a text with more than 28 fraction digits, also when the digits are zeros. Both the text without an exponent and the mantissa use it | `decimal.rs` lines 20 and 22 |
| #7 | The exponent text parses into `u32` before the zero check. `0e99999999999` gives `None` | `decimal.rs` lines 69 and 70 |
| #4 | `is_inside_type` checks a time bound only against `range_under(rules)`. It skips the `S5TIME` step and the whole-day rule of `LDATE`. A stored value runs `IecLiteral::from_count`, which applies both rules | `crates/recipe-domain/src/recipe/limit/parse.rs` lines 233 to 252, `recipe/value/leaf.rs` line 160 |
| #17 | `check_leaf` and `finish` write a bound with `bound_text`, which uses the current rule set. A count that only the current set refuses shows as a raw number. `SetParameterValue` checks its value under the current set, so a stored bound of the earlier set also shows as a raw number there | `crates/recipe-domain/src/recipe/value/check.rs` line 230, `limit/parse.rs` line 163, `limit/render.rs` line 17, `recipe/draft/add/step_content.rs` line 75 |
| #14 | No table test runs the ends of the 12 integer types through `ParameterValue::parse` and `ProcessLimit::parse` | `recipe/value/leaf/tests.rs`, `recipe/limit/parse/tests.rs` |
| #15 | The tests of `check_s5time_step` cover 9 990, 12 300, 12 345, and the maximum. No test covers a count next to the tier edges 9 990, 99 900, and 999 000 | `crates/recipe-domain/src/plant/iec_literal/canonical.rs` line 34, `canonical/tests.rs` |

## Goals / Non-Goals

Goals:

- Close the seven findings of WP1 with a change inside `recipe-domain`. The pinning tests of D2 also go into `recipe-app`.
- Keep one function for each rule (ADR-0120 (i)).

Non-Goals:

- A change of `docs/architecture/`, a new ADR, a new error code, or a migration.
- A change of the canonical text of a number or of the content hash.

## Decisions

### D1. Exact decimal parse (#2, #3, #7)

`parse_decimal` keeps its signature and its contract: it returns the exact value or `None`, and it never rounds. `canonical` and `significant_digits` do not change. The new body has these steps:

1. `is_json_number` checks the grammar first, as now.
2. The function splits the text into the sign, the digits of the whole part and of the fraction, and the exponent text.
3. It removes the leading zeros of the digits. When no digit remains, it returns `Decimal::ZERO` before it reads the exponent. Thus an exponent above `i64::MAX`, such as `0e99999999999999999999`, also gives zero. This closes #7.
4. It removes the trailing zeros of the digits. Each removed zero lowers the scale by one. This closes #2 and #3.
5. It reads the exponent text into `i64`. An overflow gives `None`. The count of fraction digits comes from the fraction text as written. The scale is that count, minus the removed zeros, minus the exponent.
6. More than 29 digits give `None`. Otherwise the digits become one `i128` with the sign.
7. A scale below -28 gives `None`, because the value is then at least 10^29 and outside the range of `Decimal`. A scale above 28 gives `None`. A negative scale then multiplies the integer by ten at most 28 times. `Decimal::try_from_i128_with_scale` builds the result. Its error gives `None`.

Every step uses `checked_*`, `strip_prefix`, and iterators. No step uses `as` or an index (`reliability.md` §2).

Alternative: keep `from_str_exact` and strip the zeros of the text first. Rejected, because a text edit must still move the decimal point across the exponent. That is the same work without one integer and one scale.

### D2. Count rules of a stored time limit (#4)

The time branch of `is_inside_type` calls `IecLiteral::from_count(time_type, count, rules)` and accepts the bound only when it returns `Ok`. `from_count` runs `check_count` (the `S5TIME` step, the whole day of `LDATE`, and the year window) and then the range check under `rules`. Thus the value path and the limit path share one function.

A bound of a request already passed `IecLiteral::parse`, so the result of `ProcessLimit::parse` does not change.

Alternative: make `check_count` public and call it next to `range_under`. Rejected, because two calls copy the order of the rules, and ADR-0120 (i) asks for one function.

Effect on a stored limit: the step rule and the day rule are in both rule sets (ADR-0120 (b)). A stored bound that breaks one fails both sets. Each caller of `stored_limits_under` (`recipe/draft/add/step_content.rs` line 120) then refuses it. ADR-0118 (d), ADR-0119 (a), and ADR-0120 (f), (g), (j) give these answers:

- A query that runs the read check on the stored limits, such as `GetParameter`, returns `500 E-DB-008`. `ListParameters` reads no limit, so its result does not change. The diff of two versions also reads the stored limits (`recipe/diff/pairs.rs` line 110).
- `ExportVersion` returns `500 E-DB-008`. The `export` spec already states this for every part that fails the earlier set.
- `CreateVersion` refuses the base with `E-RCP-020` and the item `E-RCP-006`. `VersionSnapshot::check` passes `RuleSet::Earlier`.
- `ApproveVersion` refuses the draft with `E-RCP-020` and the item `E-RCP-006`. `Draft::check_for_approval` passes `RuleSet::Current` (`recipe/snapshot/check.rs` line 8).
- `SetParameterValue` reads the stored limits under `RuleSet::Earlier` (`recipe/draft/value.rs` line 33). It returns `E-RCP-006`.
- `RebindTypeRevision` reads the kept limits under `RuleSet::Earlier` (`recipe/rebind/limit.rs` line 49, `recipe/rebind/apply.rs` line 203). Its error list holds `E-RCP-006` for the limit.

Each answer exists today for a stored bound outside the range of its type. The change adds no code and no path.

### D3. Literal of a bound under the earlier rule set (#17)

`bound_text` writes each time bound under `RuleSet::Earlier`. It keeps its three parameters and has no rule-set parameter. The fix removes `bound_text_under` and its re-export in `recipe/limit/mod.rs`. `recipe/diff/limits.rs` line 47 then calls `bound_text`.

The earlier set accepts every count that the current set accepts. `IecLiteral::from_count` runs `check_count`, which does not read the rule set. Then it runs the range check of `plant/iec_literal/range.rs`. Only the `DATE` range on `S7_1200` and `S7_1500` differs, and the earlier end 72 840 is above the current end 72 683. Thus a count that the current set accepts gets the same literal. Only a count that only the earlier set accepts changes from a raw number to a literal. The change affects message text only. No check result changes.

The `recipe-web` function `bound_text` of `pages/parameters/grid/texts.rs` line 88 already uses the earlier set (ADR-0118 (g)).

Alternative: pass the rule set of the caller to `bound_text`. Rejected, because `SetParameterValue` checks under the current set (`recipe/draft/add/step_content.rs` line 75), and its stored limits can hold a count of the earlier set.

### D4. Boundary tests (#14, #15)

1. One table test per path covers the 12 integer types: `SINT`, `USINT`, `BYTE`, `INT`, `UINT`, `WORD`, `DINT`, `UDINT`, `DWORD`, `LINT`, `ULINT`, and `LWORD`. Each row holds min-1, min, max, and max+1. The value test runs `ParameterValue::parse`. The limit test runs `ProcessLimit::parse`.
2. The `S5TIME` test runs `IecLiteral::from_count` and `IecLiteral::parse` on the counts of the scenarios of the spec delta. It accepts 9 990, 10 000, 99 900, 100 000, 999 000, and 1 000 000. It refuses 10 010, 100 100, and 1 001 000.
3. The mutation check moves one tier edge of `check_s5time_step` at a time. For each edge E with the lower step s, it runs two mutants: E-s and E+2s. The accepted edge E kills E-s. The refused count E+s of the upper tier kills E+2s. The mutant E+s is equivalent, because E+s is a multiple of the upper step. The task records the six runs in `tasks.md`.

### D5. Test placement

Each test goes into the `tests.rs` file next to the code under test. A file that would pass 800 lines gets a sibling test file. `crates/recipe-app/src/recipe/create_version/tests.rs` has 787 lines, so its new tests go into `create_version/stored_limit_tests.rs`. The proving tests of the review are the source of the cases. They do not enter the tree as files: their `println!` probes break the lint `print_stdout`.

## Risks / Trade-offs

- [The parser accepts input that it refused before] → Each new input has an exact value inside the range of `Decimal`. The canonical text stays the text of `normalize`, so no stored value and no content hash changes. A differential property test supports this claim. It compares the new parser with a copy of the parser at HEAD on each text that the copy accepts.
- [A stored time limit that passed before now fails] → No write path can store such a limit. `ProcessLimit::parse` runs `IecLiteral::parse`, and a rebind keeps a member only with its type. The tests of D2 use a hand-built `StoredLimit`.
- [The new parser has an off-by-one error in the scale] → A property test takes a mantissa that is not zero. It appends zeros and shifts the exponent. It compares the parse with the exact `Decimal`. A unit test covers zero.

## Migration Plan

No migration. The change ships in the next release of the application. A rollback is the release before it.
