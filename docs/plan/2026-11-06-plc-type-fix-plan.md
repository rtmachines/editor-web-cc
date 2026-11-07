# Fix plan for the PLC data type findings of 2026-11-06

- Source: the PLC data type review of 2026-11-06 on main at `47731e8`, with the vendor check of the same day. WP0 records both in `docs/plan/review/2026-11-06-plc-type-review.md`.
- Scope: 21 findings. 1 CRITICAL, 9 HIGH, 8 MEDIUM, 3 LOW. Each finding keeps its number of the review (`#n`).
- This file is not normative. It is an operator plan. The ADRs and the OpenSpec changes below become the record.
- No code starts before the owner approves `tasks.md` of each change (`CLAUDE.md` "Workflow").
- Status: approved by the owner on 2026-11-06. Each question of section 1 takes option (1).

## 1. Decisions for the owner

The recommended option is first in each row. A package does not start before the owner answers each question that the package needs.

| Id | Finding | Question | Options | Decided | Package |
|---|---|---|---|---|---|
| Q-A | #20 | Which count range do `LDATE` and `LDT` get on `CodesysIec` and `TwinCat3`? Both vendors end at 2262-04-11-23:47:16.854775807 (2^63-1 ns) and start at 1677-09-21. | (1) `0..=2^63-1`, the range of `LDT` on `S7_1500` and of `DTL`. A43 stays: a count is never negative. (2) The full vendor range with negative counts. A43, the export schema, and the UI literal change. | option (1), 2026-11-06 | WP4 |
| Q-B | #13 | How long can `STRING(n)` and `WSTRING(n)` be on `CodesysIec` and `TwinCat3`? Both vendors set no limit. Their standard string functions stop at 255. | (1) Keep `1..=255`. A new ADR records it as a deliberate narrowing (amends ADR-0115). No code change. (2) Allow `1..=16382`, the limit of `plc_type_string_length_range`. | option (1), 2026-11-06 | WP4 |
| Q-C | #8 | Which data block numbers are valid on `S7_1200` and `S7_1500`? The CPU 1517 manual of 11/2023 gives 1..59999 for the user and 60000..60999 for SFC 86. The S7-1200 manual gives 1..59999. | (1) `1..=59999` on both. The earlier rule set of ADR-0120 keeps `1..=65535` for stored addresses. (2) `1..=60999` on `S7_1500`. (3) A firmware attribute of the machine selects the range. | option (1), 2026-11-06 | WP4 |
| Q-D | #9, #6 | Which identifier rules do struct names and member names follow? IEC 61131-3 compares names without case and reserves its keywords. | (1) Compare without case, and reserve the IEC 61131-3 keywords and every primitive name. (2) Reserve only `ARRAY`, `OF`, `STRUCT`, and `END_STRUCT`. Names stay case-sensitive. (3) No change. An ADR records the case-sensitive rule. | option (1), 2026-11-06 | WP3 |
| Q-E | #1 | What happens to U+0000 in a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value? PostgreSQL `jsonb` cannot store it. | (1) The domain rejects it with `E-RCP-005`. A PLC string ends at its first zero, so U+0000 carries no data. (2) The domain stores an escaped form. | option (1), 2026-11-06, confirmed 2026-11-07 with the storage reason | WP2 |
| Q-F | #11 | Does a form answer list an unknown key together with the other bad fields? | (1) No. An ADR amends `api.md` §2.6: an unknown key means a changed request and gives one line. (2) Yes. The reader collects every bad key and then runs the definition check. | option (1), 2026-11-06 | WP6 |
| Q-G | #21 | `LDATE`, `LDT`, and `LTOD` need TwinCAT 3.1.4026.0 or later. How does the editor handle an older TwinCAT? | (1) A new Assumption: every `TwinCat3` machine runs 3.1.4026.0 or later. The runbook names the check. (2) A machine attribute holds the runtime version, and the type catalog hides the three types below 4026. | option (1), 2026-11-06 | WP4 |
| Q-H | #10 | `STRING` and `STRING(80)` on `CodesysIec` are two rows with one shape. Which rule fixes the rebind? | (1) `kept_members` compares the shape, with the default length of the platform resolved. Type identity stays the row (ADR-0017). (2) `EnsureDerivedType` maps `STRING(n)` with the default `n` to the seed row. Existing machine rows stay. | option (1), 2026-11-06 | WP5 |
| Q-I | #6 | How do the class-2 locks keep one order when two key texts collide? | (1) A command sorts its keys by the final key value (`hashtext`) and removes duplicates. `data-model.md` §1.4 changes. (2) Only the keyword rule of Q-D prevents the collision. | option (1), 2026-11-06 | WP3 |

## 2. Work packages and order

| WP | Name | Findings | Kind | ADR | Spec delta | Can run in parallel with |
|---|---|---|---|---|---|---|
| WP0 | Review record | all | docs commit | none | none | all |
| WP1 | OpenSpec change `value-checks-round-1` | #2, #3, #7, #4, #17, #14, #15 | change | none | `recipe-management` | WP2, WP6 |
| WP2 | OpenSpec change `text-value-rules` | #1, #12 | change | 1 new | `recipe-management` | WP1, WP6 |
| WP3 | OpenSpec change `plc-identifier-rules` | #9, #6, #5 | change | 2 or 3 new | `plant-model` | WP6 |
| WP4 | OpenSpec change `platform-ranges-round-1` | #20, #8, #13, #21, #18, #19 | change | 3 or 4 new, 1 Assumption | `plant-model`, `recipe-management` | WP6 |
| WP5 | OpenSpec change `rebind-shape-identity` | #10 | change | 1 new | `recipe-management` | WP6 |
| WP6 | OpenSpec change `form-error-listing` | #11 | change | 1 new | `web-ui` | all |
| WP7 | Vendor research by the owner | #16, open points | owner task | per result | per result | all |

Order rules:

1. WP0 has no dependency. A Haiku agent does it first.
2. WP1 and WP2 touch different files. They run in parallel on two branches.
3. WP3 starts after the answers to Q-D and Q-I. It owns the migrations `0011` to `0013`.
4. WP4 starts after WP1, because both change the time and limit checks. It needs Q-A, Q-B, Q-C, and Q-G.
5. WP5 starts after WP3, because both change the type rows of a machine.
6. WP6 has no code dependency. It runs at any time after Q-F.
7. WP7 runs in parallel. Its results open a follow-up change, see section 10.
8. Each ADR takes the next free number at its propose step. The next free number today is 0137. The next free migration is `0011`.

Each change follows the cycle of `CLAUDE.md`:

1. `openspec-propose` (Opus).
2. `plan-eng-review` (Opus), then `openspec-update-change`.
3. Approval of `tasks.md` by the owner.
4. `openspec-apply-change` with `superpowers:test-driven-development`. Each task starts with a failing test.
5. `code-review` (Opus) and a Codex second review (gpt-6-astra, effort high).
6. `superpowers:verification-before-completion` with the clippy, test, and fmt output.
7. `openspec-archive-change` after `cargo test` and `openspec validate` pass.

Agent rules of this plan:

- One task per agent.
- Sonnet (effort high) writes and fixes Rust code, SQL migrations, and tests.
- Codex (gpt-6.1-sol) writes shell scripts and CI changes, if a package needs one.
- Haiku writes simple doc edits. Opus designs, reviews, and verifies.
- Error codes: each fix reuses an existing code of `reliability.md` §1.2. If a fix needs a new code, the row goes into §1.2 first.

## 3. WP0: review record

- Task 1 [mechanical, Haiku]: write `docs/plan/review/2026-11-06-plc-type-review.md`. It holds the 21 findings, the coverage matrix, the "Not checked" list, the vendor sources, and the proving tests of the four review worktrees.
- Task 2 [mechanical, Haiku]: copy the proving tests out of the worktrees `.claude/worktrees/agent-*` into the review record. Then remove the four worktrees with `git worktree remove`.
- Check: the record names every finding `#1` to `#21` once.

## 4. WP1: change `value-checks-round-1`

No document changes. The rules exist in `reliability.md` §4.4, ADR-0117, and `domain-model.md` §2.1. The code does not follow them.

### 4.1 #2, #3, #7: exact decimal parse (HIGH)

- Tasks:
  1. Failing tests (Sonnet, `recipe-domain`, `recipe/value/decimal/tests.rs`): `150e-29`, `10e-29`, and `100e-30` give 1.5e-27, 1e-28, and 1e-28. `1.` followed by 29 zeros gives 1. `0e99999999999` gives 0. `1e-29` still gives `None`.
  2. Failing tests (Sonnet, `recipe-domain`, `recipe/limit/parse/tests.rs`): the same texts as a `REAL` bound give a limit, not `E-RCP-006`.
  3. Fix (Sonnet, `decimal.rs`): parse the mantissa digits and the exponent into one integer and one signed scale. Remove trailing zeros before the range check of `Decimal`. Check `base.is_zero()` before the exponent parse. Every step uses `checked_*`.
- Note: the fix accepts more input. It cannot change a stored value, because the canonical text stays `normalize()`.

### 4.2 #4: stored time limits get the step and day rules (HIGH)

- Tasks:
  1. Failing test (Sonnet, `recipe-domain`, `recipe/limit/parse/tests.rs`): `ProcessLimit::from_stored` refuses `S5TIME max = 15` on `S7_1500` and `LDATE max = 1` with the same fault as the value path.
  2. Failing test (Sonnet, `recipe-domain`, `recipe/read_check/tests.rs`): the read check of an approved version refuses that limit with `500 E-DB-008` (ADR-0118 (d), ADR-0119 (a), ADR-0120 (b)).
  3. Fix (Sonnet, `limit/parse.rs:233`): `is_inside_type` builds each time bound with `IecLiteral::from_count`, as `ParameterValue::from_stored` does.

### 4.3 #17: messages show the literal under the earlier rule set (LOW)

- Tasks:
  1. Failing test (Sonnet, `recipe-domain`): an `S7_1500` `DATE` value 72_700 against a stored max 72_690 under `Earlier` gives `D#2169-01-17` and `D#2169-01-07` in the fault.
  2. Fix (Sonnet, `value/check.rs:230`, `limit/parse.rs:163`): call `bound_text_under(.., rules)`.

### 4.4 #14, #15: boundary tests (MEDIUM)

- Task 1 (Sonnet, `recipe-domain`): one table test per integer type through `ParameterValue::parse` and `ProcessLimit::parse`. It covers min-1, min, max, and max+1 of all 12 integer types.
- Task 2 (Sonnet, `recipe-domain`, `iec_literal/canonical/tests.rs`): `S5TIME` refuses 10_010, 100_100, and 1_001_000. It accepts 9_990, 10_000, 100_000, and 1_000_000.
- Check: each new test fails when a tier edge of `check_s5time_step` moves one step down or two steps up. Record each mutant run in `tasks.md`.

## 5. WP2: change `text-value-rules`

### 5.1 #1: no U+0000 in a text value (HIGH)

- ADR (new, amends ADR-0028), per Q-E: a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value holds no U+0000.
- Documents: `domain-model.md` §2.1 gives `CHAR` the range `U+0001..=U+00FF` and `WCHAR` the range `U+0001..=U+FFFF`. The `String` row names the same rule.
- Tasks:
  1. Failing test (Sonnet, `recipe-domain`): `ParameterValue::parse` refuses U+0000 in each of the four types with `E-RCP-005`.
  2. Failing test (Sonnet, `recipe-web`): `value=%00` on the add and edit forms gives the field line of `E-RCP-005`, not a storage error.
  3. Failing test (Sonnet, `recipe-db`): a guard test writes a value with `\u0000` through the adapter and expects the domain error before SQL.
  4. Fix (Sonnet, `recipe/value/leaf.rs:109`, `:132`).
- Note: no stored row can hold U+0000, because PostgreSQL refuses it. The earlier rule set needs no change.

### 5.2 #12: one unit for `StringLength` (MEDIUM)

- The same ADR records the unit: a `STRING` length counts characters, which are bytes in Latin-1. A `WSTRING` length counts UTF-16 code units.
- Task [mechanical, Haiku]: align `domain-model.md:93` and `reliability.md:473` with the ADR. The code already follows it.

## 6. WP3: change `plc-identifier-rules`

### 6.1 #9: IEC identifier rules for struct and member names (MEDIUM)

- ADR (new, amends ADR-0024), per Q-D option (1): a struct name compares without case on one machine. A member name compares without case in one struct. A struct name is not an IEC 61131-3 keyword and not a primitive name, in any case.
- Documents: `domain-model.md` §2.1 grammar of `name` and `member`. `data-model.md` §3.3 and §3.4 name the new indexes.
- Tasks:
  1. Failing tests (Sonnet, `recipe-domain`): `TypeName::parse` refuses `Int`, `real`, `ARRAY`, `OF`, `STRUCT`, `END_STRUCT`. `PlcType::try_from` refuses the members `speed` and `Speed`. Each refusal uses the existing code of `reliability.md` §1.2.
  2. Failing test (Sonnet, `recipe-db`): a second struct `st_zone` next to `ST_Zone` on one machine fails on the new unique index.
  3. Migration check (Sonnet, `recipe-db`): a test query finds existing rows that break the new rule. The migration stops with a clear message when it finds one.
  4. Migrations (Sonnet): `0012_type_name_ci_unique.sql` and `0013_member_name_ci_unique.sql`. Each file starts with `-- no-transaction` and holds one `CREATE UNIQUE INDEX CONCURRENTLY` on `lower(name)`. `grants.expected` does not change.
  5. Fix (Sonnet, `type_name.rs:89`, `plc_type.rs:246`).
- Note: existing struct revisions stay valid. A new revision follows the new rule.

### 6.2 #6: the class-2 lock order cannot invert (HIGH)

- ADR (new, amends ADR-0017), per Q-I option (1): a command sorts its class-2 keys by the final key value and removes duplicates. Two texts with one hash then take one lock.
- Documents: `data-model.md` §1.4 and `.claude/rules/sql.md` ("Locks and transactions") replace the order "struct name, `STRING`, `WSTRING`, `ARRAY`".
- Tasks:
  1. Failing test (Sonnet, `recipe-app`, `derived_lock/tests.rs`): the keys of `DefineUdt ARRAY` with `STRING(5)` and of `AddParameter ARRAY[1..2] OF STRING(5)` come in one order.
  2. Failing test (Sonnet, `recipe-db`): two transactions that lock the two key sets in parallel finish without `40P01`.
  3. Fix (Sonnet, `recipe-db/src/transaction/type_rows.rs:41`): read the `hashtext` values in one query, sort them, and lock each one.
- Note: Q-D option (1) already refuses the name `ARRAY`. This fix also covers a hash collision of two other texts.

### 6.3 #5: `dims_valid` requires lower bound 1 (HIGH)

- ADR (new, amends ADR-0017) or a section of the ADR of 6.2: the two arrays of a dimension start at subscript 1.
- Documents: the DDL of `dims_valid` in `data-model.md` §3.3.
- Tasks:
  1. Failing test (Sonnet, `recipe-db`): `dims_valid('[0:0]={5}', '{0}')` and `dims_valid('[0:0]={0}', '{65536}')` give `false`. An insert with these arrays fails on `plc_type_array_shape`.
  2. Migration (Sonnet): `0011_dims_valid_lower_bound.sql` runs `CREATE OR REPLACE FUNCTION dims_valid` with `array_lower(lower_b, 1) = 1 AND array_lower(upper_b, 1) = 1`. Before the replace, it checks that no row breaks the new rule.
  3. Check: the job `migration-rewrite` accepts the file.

## 7. WP4: change `platform-ranges-round-1`

This package aligns the type tables with the vendor manuals. Each range that gets narrower uses the earlier rule set of ADR-0120. A stored value that the new range refuses stays readable, and a draft does not lock. `ApproveVersion` checks with `Current`, so a new approval refuses the value.

### 7.1 #20: `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` (CRITICAL)

- ADR (new, amends ADR-0026 and the base-unit table), per Q-A option (1): `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` count `0..=9_223_372_036_854_775_807` ns. That is 1970-01-01 to 2262-04-11-23:47:16.854775807.
- Documents: `domain-model.md:229` and `:233`.
- Tasks:
  1. Ops query first (Sonnet, `recipe-db` test plus a runbook step): find every stored `LDATE` or `LDT` value and limit above 2^63-1 on the two platforms. The owner reads the result before WP4 merges.
  2. Failing tests (Sonnet, `recipe-domain`, `iec_literal/range` tests): `LDT#2262-04-11-23:47:16.854775807` passes on both platforms. `LDT#2262-04-11-23:47:16.854775808` and `LD#2262-04-12` fail with `Current`. They pass with `Earlier`.
  3. Failing test (Sonnet, `recipe-domain`): `ApproveVersion` refuses a draft with `LDT#2300-01-01-00:00:00` on `TwinCat3`.
  4. Fix (Sonnet, `range.rs:29`, `:39`): split the constants by rule set.

### 7.2 #8: data block numbers on `S7_1200` and `S7_1500` (HIGH)

- ADR (new, amends ADR-0116 (c)), per Q-C option (1): the data block number is `1..=59999` on `S7_1200` and `S7_1500`.
- Documents: `domain-model.md:189`.
- Tasks:
  1. Failing tests (Sonnet, `recipe-domain`, `recipe/draft/s7/tests.rs`): `DB59999.DBW0` passes. `DB60000.DBW0` and `DB65535.DBW0` fail with `E-RCP-057` under `Current` and pass under `Earlier`.
  2. Fix (Sonnet, `recipe/draft/s7.rs:154`).
  3. Ops query (Sonnet): find stored addresses with a data block above 59999.

### 7.3 #13: string length on `CodesysIec` and `TwinCat3` (HIGH)

- Per Q-B option (1): an ADR (amends ADR-0115) records `1..=255` as a deliberate narrowing. The reason is the 255 limit of the standard string functions. No code change.
- Per Q-B option (2): the ADR sets `1..=16382`. Then a failing test for `STRING(300)` and `WSTRING(1000)` and a fix of `string_length.rs:96` follow.

### 7.4 #21: TwinCAT version for the long date types (MEDIUM)

- Per Q-G option (1): a new Assumption in `overview.md` §9, and its copy in `domain-model.md` §8: a `TwinCat3` machine runs TwinCAT 3.1.4026.0 or later.
- Task [mechanical, Haiku]: a runbook step in `docs/ops/` names the check of the TwinCAT version before a plant adds a `TwinCat3` machine.

### 7.5 #18, #19: table text (LOW)

- The ADR of 7.1 also corrects these rows. They follow the code and the vendor manuals.
- Task [mechanical, Haiku]: `domain-model.md:231` gives `LTOD` the range below one day on every platform. `data-model.md:2048` says that `STRING` and `WSTRING` always use the width `B`.

## 8. WP5: change `rebind-shape-identity`

### 8.1 #10: one shape keeps its member (MEDIUM)

- ADR (new, amends ADR-0018), per Q-H option (1): the rebind keeps a member when the old and new member types have one shape. The default length of the platform counts as an explicit length.
- Tasks:
  1. Failing test (Sonnet, `recipe-domain`, `rebind/level` tests): a revision that rewrites `STRING(80)` as `STRING` on `CodesysIec` keeps the value and the limit. No `E-RCP-028` and no `E-RCP-029`.
  2. Failing test (Sonnet, `recipe-domain`): the same rewrite of `STRING(80)` as `STRING(81)` still gives `E-RCP-028`.
  3. Fix (Sonnet, `rebind/level.rs:40`): compare the canonical shape, not `type_id`.

## 9. WP6: change `form-error-listing`

### 9.1 #11: unknown keys in a form answer (MEDIUM)

- Per Q-F option (1): an ADR amends `api.md:214`. A request with an unknown key gives one line `E-WEB-008`. The HTML of the editor never sends such a key.
- Per Q-F option (2): failing tests in `recipe-web` for `tag=bad tag&bogus=1&value[*]=2` expect three lines. Then `fields.rs:256` collects the faults and `new.rs:170` runs the definition check.

## 10. WP7: vendor research by the owner

These points need TIA Portal or a vendor answer. An agent cannot close them.

| Point | Finding | What to check | Result goes to |
|---|---|---|---|
| R1 | #16 | Offsets and sizes of the 24 composite rows of `s7_layout/reference.rs` (Q09, Q10), in a non-optimized DB in TIA Portal | Follow-up change `s7-layout-reference-verified`: rows become `Verified`, A02 and A10 get one size |
| R2 | open | Can a non-optimized DB on S7-1500 hold `WSTRING`, `LTIME`, `LTOD`, `LDT`, and `DTL`? Siemens entry 67655611 | New ADR if a type is refused |
| R3 | open | Does S7-1200 support `WCHAR` and `WSTRING`, and from which firmware? The guideline of 12/2018 marks them "only S7-1500" | `data-model.md` §7 by ADR if needed |
| R4 | #8 | The data block range of S7-1200 by firmware, from the current system manual | ADR of 7.2 |

### 10.1 Follow-ups from the WP2 plan review

The plan review of WP2 on 2026-11-07 found three faults outside the scope of WP2. The owner moved them to later changes.

| Point | What to do | Result goes to |
|---|---|---|
| F-1 | `reliability.md` §7.4 says that validation runs before the transaction. Every draft command runs its validation inside the transaction after its row locks. A new ADR states "before the first write of the command". | New ADR and `reliability.md` §7.4, `CLAUDE.md` "Hard rules" |
| F-2 | The `SetParameterValue` row of `domain-model.md` §5 and `api.md` lacks `E-RCP-006`, `E-RCP-017`, `E-RCP-018`, `E-RCP-019`, and `E-PLT-013`. The `ReplaceLimits` row lacks `E-RCP-005` and `E-PLT-013`. | Small docs change with an ADR |
| F-3 | The phrases at `en.ftl` and `pl.ftl` lines 498 and 499 have no plural form, for example "at most 1 characters". | Later `web-ui` change with a Fluent plural selector |

### 10.2 Follow-ups from the WP6 plan review

The plan review of WP6 on 2026-11-07 found two faults outside the scope of WP6. The owner moved them to later changes.

| Point | What to do | Result goes to |
|---|---|---|
| F-4 | `reliability.md` §4.2 (line 423) says that a missing required field of a `serde` form gives `E-WEB-008` with the field name. The code gives the field `form` (`crates/recipe-web/src/error.rs:242-249`). Decide which one changes. | Later `web-ui` change: fix the code or the document |
| F-5 | On the rebind route, a bad hidden field gives a list without an out-of-band form (`crates/recipe-web/src/pages/versions/rebind.rs:301-307`, `form: None`). The `web-ui` requirement "Every field error at once" says the answer swaps the form out of band. | Later `web-ui` change: fix the code or the requirement |

### 10.3 Follow-ups from the WP6 code review

The code review of WP6 on 2026-11-07 found two gaps between the code and the wish of ADR-0138. The owner kept the code and moved them to later changes.

| Point | What to do | Result goes to |
|---|---|---|
| F-6 | On the value and limits routes, a bad `row_version` gives one line before the other fields are read (`crates/recipe-web/src/pages/parameters/value.rs:44`, `:126`, `:175`). The add and edit forms put it into the list (`definition.rs:123`). | Later `web-ui` change: put `row_version` into the list of stage 1 |
| F-7 | The one line of a changed body has the label `Form`. Its text depends on the class of the fault (ADR-0138 (a)). `reason-form` (`crates/recipe-web/i18n/en.ftl:176`) names a missing field, an unknown field, or a wrong content type. The reader also gives it for a plain key twice and too many pairs (`crates/recipe-web/src/pages/parameters/fields.rs:258`, `:316`, `:323`). A value key twice gives `reason-value-key`, and a limit key fault gives `reason-limit-row` (`en.ftl:540`, `:541`). No text names the fault itself. | Later `web-ui` change: a text for each fault of ADR-0138 (b) |

## 11. Done check

The plan is done when all of these pass on main:

1. `cargo fmt --all --check`.
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`.
3. `cargo test --workspace --all-features --locked`, including the `recipe-db` container tests.
4. `cargo deny --locked check`.
5. `openspec validate --all --strict`.
6. `ste-lint.py` on the scope of ADR-0023 and ADR-0135 with 0 hard violations.
7. `cargo llvm-cov` with `tools/coverage-gate.py`, 80 % lines per crate.
8. The CI run of the last merge is green.
9. A closure review (Opus and Codex) gives each finding `#1` to `#21` a verdict in `docs/plan/review/`.
