# Design

## Context

See `proposal.md`, section "Why". ADR-0139 to ADR-0142 hold the rules. ADR-0116, ADR-0118, ADR-0119, and ADR-0120 hold the context of the address rules, the reads, the export, and the earlier rule set. This design links to them and does not repeat them.

The state of the code at HEAD `8c5802b`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| `LDATE` range | `0..=18_446_744_073_709_551_615` on `CodesysIec` and `TwinCat3`, in both rule sets | `crates/recipe-domain/src/plant/iec_literal/range.rs` line 30 |
| `LDT` range | `LDT_UNSIGNED` `0..=18_446_744_073_709_551_615` on `CodesysIec` and `TwinCat3`, in both rule sets. `LDT_S7` and `DTL` end at `9_223_372_036_854_775_807` | `range.rs` lines 40, 42, 44 |
| `LTOD` range | `0..=86_399_999_999_999` on every platform with `LTOD`. The code already follows ADR-0139 (h) | `range.rs` lines 34, 75, 76 |
| Rule sets | `range` differs between `Current` and `Earlier` only for `DATE` on `S7_1200` and `S7_1500` | `range.rs` lines 58 to 85, arm at line 71 |
| Pins of the old ranges | `ldate_range_per_platform` and `ldt_range_per_platform` assert the end `2^64-1`. `adr_0120_the_sets_differ_only_for_date_on_s7_1200_and_s7_1500` asserts that no other type differs | `crates/recipe-domain/src/plant/iec_literal/range/tests.rs` lines 188, 244, 306 |
| Data block rule | `check_data_block_rule(platform, tag, address)` refuses only `S7_300` above `S7_300_MAX_DATA_BLOCK` `16_000`. It takes no `RuleSet` | `crates/recipe-domain/src/recipe/draft/s7.rs` line 154 |
| Callers of the data block rule | `check_definition` (written parameter), `check_layout` with `AddressRules::Run` (approval), the read check (field line), and the rebind | `s7.rs` lines 82 and 116, `crates/recipe-domain/src/recipe/read_check/mod.rs` line 280, `crates/recipe-domain/src/recipe/rebind/apply.rs` line 211 |
| Variant `E-RCP-057` | `S7DataBlockOutOfRange { tag, address }` with the fixed text "above 16000" and "An S7-300 controller" | `crates/recipe-domain/src/recipe/error.rs` lines 233 to 243 |
| Constructors of the variant in tests | four struct literals. The registry test also lists the arguments of the variant | `recipe/error/registry_tests.rs` lines 359 and 363, `recipe-web/src/error/recipe_tests.rs` line 367, `recipe-web/src/pages/approval/blockers_tests.rs` line 190, `recipe-web/src/pages/parameters/reply/tests.rs` line 41 |
| Asserts of the old text | two tests of `check_definition` and of the approval check assert "An S7-300 controller" | `crates/recipe-domain/src/recipe/draft/s7/tests.rs` lines 466 to 471 and 506 to 511 |
| Tests that read the registry | `registry_matches_reliability` and the test at `recipe_answer_tests.rs` line 107 compare the code with `reliability.md` §1.2. Both fail after the propose step | `crates/recipe-domain/src/recipe/error/registry_tests.rs`, `crates/recipe-web/src/error/recipe_answer_tests.rs` |
| Web message | `recipe.rs` puts `tag` and `address` into the arguments of one arm with other variants. Both bundles hold the fixed number 16000 | `crates/recipe-web/src/error/recipe.rs` line 200, `crates/recipe-web/i18n/en.ftl` and `pl.ftl` line 479 |
| Approval | `check_for_approval` runs `check_snapshot` with `RuleSet::Current` and then `check_s7_layout_for_approval` | `crates/recipe-domain/src/recipe/draft/approval.rs` lines 157, 164, 173 |
| String length | `max_length` gives 255 on `CodesysIec` and `TwinCat3`. The tests pin 255 and 256 for both types on both platforms | `crates/recipe-domain/src/plant/string_length.rs` lines 96 to 106, `string_length/tests.rs` lines 11, 21, 66 |
| Width of a string | `STRING` and `WSTRING` give `B`. The code already follows ADR-0139 (i) | `crates/recipe-domain/src/plant/s7_width.rs` line 28 |
| Ops query pattern | a test builds the path of a SQL file of `deploy/` with `concat!(env!("CARGO_MANIFEST_DIR"), ...)`, reads it with `std::fs::read_to_string`, and runs it on the harness database | `crates/recipe-db/tests/storage/restore_code.rs` lines 18 to 21 and 46 to 50 |
| Doc comments with the old rule | `range_under` says that only `DATE` differs between the sets. `S7DataBlockOutOfRange` names only the S7-300 controller | `crates/recipe-domain/src/plant/iec_literal/mod.rs` line 64, `crates/recipe-domain/src/recipe/error.rs` line 233 |
| Parse of a literal | `IecLiteral::parse` checks the range with `RuleSet::Current`. `IecLiteral::from_count` takes the rule set | `crates/recipe-domain/src/plant/iec_literal/mod.rs` line 120, `iec_literal/canonical.rs` line 79 |

No test of `recipe-export`, `recipe-db`, or `e2e` holds an `LDATE` or `LDT` count above `2^63-1`. The counts `18446744073709551615` in those tests belong to `ULINT` and `LWORD`.

## Goals / Non-Goals

Goals:

- End `LDATE` and `LDT` on `CodesysIec` and `TwinCat3` at `2^63-1` ns in the current rule set (ADR-0139).
- End the data block number at 59 999 on `S7_1200` and `S7_1500` (ADR-0140).
- Keep every stored row readable and every draft unlocked (ADR-0120).
- Give the owner a query that lists the stored rows that the new ranges refuse, before the merge.
- Pin each new boundary with a test that shows a red step.

Non-Goals:

- Negative counts, a firmware attribute, or a string length above 255. `proposal.md`, section "Out of scope", lists the rest.
- A migration, a new error code, or a new setting.

## Decisions

### D1: The range constants per rule set

`range.rs` keeps the old end of `LDATE` and `LDT` as named constants of the earlier set, for example `LDATE_EARLIER` and `LDT_UNSIGNED_EARLIER`. The current constants `LDATE` and `LDT_UNSIGNED` end at `9_223_372_036_854_775_807`. Two new arms of `range` return the earlier constants for `rules == RuleSet::Earlier` on `CodesysIec` and `TwinCat3`. The doc comments of `range` and of `RuleSet::Earlier` name ADR-0139. The whole-day rule of `LDATE` does not change.

The test `adr_0120_the_sets_differ_only_for_date_on_s7_1200_and_s7_1500` gets the new name `adr_0120_and_adr_0139_the_sets_differ_only_for_date_ldate_and_ldt` and lists the three types. Alternative: one current constant for `LDT`, `LDT_S7`, and `DTL`. The change keeps one constant per type, because each doc comment names its own ADR.

### D2: The data block rule takes no rule set

`check_data_block_rule` gets the highest number per platform: 16 000 on `S7_300` and 59 999 on `S7_1200` and `S7_1500`. It keeps its signature. The earlier set has no data block rule (ADR-0140 (d)). The grammar of `S7Address` still allows `1..=65535`.

The callers do not change. A stored address of another parameter never reaches the rule, because `check_s7_layout` runs with `AddressRules::Skip` (ADR-0116 (e1)). The read check gives a field line, because `E-RCP-057` is a rule fault (ADR-0118 (e)). Thus "passes under Earlier" in the fix plan means four behaviors. The read gives a field line, and `loadable` is `false`. `CreateVersion` copies the base, and a command on another parameter passes.

The rebind keeps the current set and takes no `RuleSet` parameter. ADR-0116 (e) runs the address rules on each rebound parameter, as it does for a data block above 16 000 on `S7_300`. The refusal does not lock the draft. `UpdateParameterDefinition` moves the address, and `DiscardDraft` does not run the rule. The scenario "Approval after the fix of a stored data block above 59999" pins the fix path. `plan-eng-review` accepted this decision.

Alternative: a `RuleSet` parameter, so that `RebindTypeRevision` checks a rebound address with the earlier set. The change rejects it, because ADR-0116 (e) and ADR-0140 (c) name each parameter of `RebindTypeRevision` as a path of the rule.

### D3: The variant names its highest number

`S7DataBlockOutOfRange` gets the field `max: u16`. `reliability.md` §1.2 holds the new text. `check_data_block_rule` fills `max` from the platform. `recipe-web/src/error/recipe.rs` gives the variant its own arm and passes `max` as a string, so that Fluent adds no group separator. The bundles get these texts:

- `en`: `Address { $address } of { $tag } uses a data block above { $max }. The controller of the machine has no such data block.`
- `pl`: `Adres { $address } parametru { $tag } używa bloku danych powyżej { $max }. Sterownik maszyny nie ma takiego bloku danych.`

The four test constructors of the context table get `max: 16_000`, and the registry test gets the argument `max`. The two asserts of the old text get the new text. Task 1.1 lists each place.

### D4: The ops query

`deploy/db/check-platform-ranges.sql` runs in one `READ ONLY` transaction and writes nothing. It lists these rows, with the machine code, the reference number, the version number and status, the tag, the part, and the stored text:

- a parameter on `CodesysIec` or `TwinCat3` whose type tree holds `LDATE` or `LDT`, with a value leaf above `9223372036854775807`
- a limit row of such a parameter with `min_value` or `max_value` above `9223372036854775807`
- a parameter on `S7_1200` or `S7_1500` whose `s7_address` has a data block number above 59999

The query covers every version status and also soft-deleted rows: a deleted draft, a deleted recipe, and an archived machine. It finds a leaf at each depth of the value: a struct member, a member of a nested struct, and an array element. The domain stores a `Numeric`, `Duration`, or `Instant` leaf as a JSON number (`domain-model.md` §2.2). The query keeps only the leaves with `@.type() == "number"` before the numeric compare, so a `STRING` or `BOOL` leaf gives no error under `ON_ERROR_STOP=1`. A `ULINT`, `LWORD`, `LTIME`, `REAL`, or `LREAL` leaf in the same type tree can give a false hit. The false hit comes from a value row or from a limit row of that leaf. The owner reads each row.

The runbook runs each `docker compose` command in the directory `deploy/`. Thus it gives the command `docker compose exec -T -u postgres db psql -d recipe -v ON_ERROR_STOP=1 < db/check-platform-ranges.sql`.

A test of `recipe-db` reads the file with `std::fs::read_to_string` and a path from `CARGO_MANIFEST_DIR`, as `restore_code.rs` does. It seeds hits and non-hits and compares the exact rows of the query. Task 2.1 lists the seeded rows.

### D5: No test task for the string length

`string_length/tests.rs` lines 11, 21, and 66 already pin 255 and 256 on `CodesysIec` and `TwinCat3`. The two new scenarios of the `plant-model` delta follow from these tests. Thus ADR-0141 needs no code task.

### D6: The export delta

The brief of WP4 does not name the `export` capability. The rule check of the export uses the current set (ADR-0119). Thus `loadable` becomes `false` for a stored `LDT` above the new end and for a data block above 59 999. The `export` delta records both scenarios. Tasks 3.4 and 4.2 pin them.

### D7: The red steps

Each pinning test shows its red step on one temporary mutation:

- M1: the current `LDATE` and `LDT_UNSIGNED` end again at `18_446_744_073_709_551_615` (`range.rs` lines 30 and 40).
- M2: the highest data block number of `S7_1200` and `S7_1500` is again 65 535 (`s7.rs`, function `check_data_block_rule`).
- M3: the new earlier arms of `range` return the current constants. A pin of the earlier set fails on M3.
- M4: the earlier end of `LDT` is `18_446_744_073_709_551_616`. A pin of a structure fault above the earlier end fails on M4.
- M5: the highest data block number of `S7_1200` and `S7_1500` is 59 998. A pin of `DB59999` that passes fails on M5.

Before a mutation, the agent copies the file to the scratch directory. After the red run, it reverts the mutation and runs `cmp <file> <copy>`. A wrong expected value in a test is not a red step.

## Risks / Trade-offs

- Risk: a stored value or address on production breaks a new range. Mitigation: task 2.3 runs the query before the merge, and its box is a hard gate of the merge. The read shows a field line, and `ApproveVersion` refuses the value.
- Risk: a rebind on `S7_1200` or `S7_1500` refuses a parameter at a data block above 59 999 (D2). Mitigation: the user changes the address with `UpdateParameterDefinition` first.
- Trade-off: the query can give a false hit for a `ULINT` leaf. The owner reads each row, so a false hit costs only a look.

## Migration Plan

No migration and no new step of the deploy files. The owner runs the query of D4 before the merge (task 2.3). The merge needs 0 rows or the sign-off of the owner on each row. The pull request holds the number of rows and the identity of each row. The stored text stays in the change log of the site. The upgrade of production runs the query again, in the step of `docs/ops/runbook.md` that task 2.2 adds.

## Open Questions

None. `plan-eng-review` closed both questions of the propose step:

- D2: the rebind keeps the current set and takes no `RuleSet` parameter (ADR-0116 (e)).
- Task 2.1 writes a SQL file in `deploy/` and its test in `recipe-db`. A Sonnet agent does the task. `AGENTS.md` ("Role of Codex") lets Codex only review, and plan section 2 gives SQL and tests to Sonnet.
