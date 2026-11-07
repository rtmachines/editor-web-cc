# Design

## Context

See `proposal.md`, section "Why". ADR-0137 holds the rules. `domain-model.md` §2.1, `reliability.md` §4.4, ADR-0028, ADR-0041, and ADR-0120 hold the context. This design links to them and does not repeat them.

The state of the code at HEAD `f2df662`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| One conversion of a leaf | Every path that builds or checks a value calls `convert`. These are the parse of a JSON value, the stored check, the text of a form field, and a new leaf of a rebind | `crates/recipe-domain/src/recipe/value/leaf.rs` line 82, `walk.rs` line 65, `check.rs` line 99, `request_leaf.rs` line 44, `recipe/rebind/value.rs` lines 152 and 153 |
| #1, `STRING` and `WSTRING` | `string` accepts `U+0000`. A `STRING` character must be at most `U+00FF` | `leaf.rs` lines 95 to 116 |
| #1, `CHAR` and `WCHAR` | `character` accepts one scalar up to `U+00FF` or `U+FFFF`, also `U+0000` | `leaf.rs` lines 120 to 141 |
| #1, form text | The form reader decodes `%00` to `U+0000`. Then `request_text` converts the text | `crates/recipe-web/src/pages/parameters/fields.rs` line 163, `request_leaf.rs` line 14 |
| #1, storage | The adapter binds the JSON of the value. PostgreSQL refuses `U+0000` in `jsonb` with SQLSTATE `22P05`. `map_error` maps it to `StorageError::Other` | `crates/recipe-db/src/transaction/draft_writes.rs` lines 178 and 234, `crates/recipe-db/src/error.rs` line 227 |
| #1, limits | A limit holds no text. `max_length` is a number | `domain-model.md` §2.2 (`ProcessLimit`) |
| #1, expected forms | The phrase of `Expected::Char` starts at `U+0000`. The phrases of `Expected::String` and `Expected::Wstring` name no rule for `U+0000` | `crates/recipe-domain/src/recipe/value/expected.rs` lines 110 to 116, `crates/recipe-web/i18n/en.ftl` and `pl.ftl` lines 498 to 500 |
| #1, property tests | The generator draws `CHAR`, `WCHAR`, and `STRING` characters from `'\0'`, and `WSTRING` characters from `any::<char>()` | `crates/recipe-domain/src/recipe/value/prop/generate.rs` lines 353, 396, and 398 |
| #1, XSS tests | Two tests send the NUL payload as a `STRING` value and expect a success. The module comment says that the value check accepts the NUL payload | `crates/recipe-web/src/pages/approval_text_xss_tests.rs` lines 14 to 20, 228, and 259 |
| #12, value | `string` counts `chars()` for `STRING` and `encode_utf16()` for `WSTRING` | `leaf.rs` lines 105 to 109 |
| #12, limit | `check_leaf` counts `max_length` in the same units | `crates/recipe-domain/src/recipe/value/check.rs` lines 199 to 207 |
| #1, order in a command | `SetParameterValue` opens the transaction and locks the draft before `Draft::set_value`. `AddParameter`, `UpdateParameterDefinition`, and `RebindTypeRevision` can insert a new derived type before the value check | `crates/recipe-app/src/recipe/set_value.rs` lines 77 to 79, `draft_session.rs` line 192, `add_parameter.rs` lines 113 to 121, `update_definition.rs` lines 164 and 178, `rebind.rs` lines 86 and 90 |
| #1, rule sets | `from_stored` passes its `RuleSet` to the leaf conversion. The read check of a stored part uses `RuleSet::Earlier` | `recipe/value/stored.rs` lines 53 to 58, `walk.rs` line 63, `recipe/read_check/mod.rs` line 145 |

The code already follows the unit of #12. Thus #12 needs only the documents and pinning tests.

## Goals / Non-Goals

Goals:

- Close #1 with one check in `recipe-domain`. The tests of `recipe-web` and `recipe-db` pin the answer at the HTTP layer and on PostgreSQL.
- Close #12 with the documents of ADR-0137 and pinning tests.
- Keep one function for each rule (ADR-0120 (i)).

Non-Goals:

- A new error code, a migration, or a new setting.
- A check of `U+0000` in the adapter of `recipe-db` or in the form reader of `recipe-web`.
- A change of the error lists of `domain-model.md` §5 and `api.md` §2.6. A later change adds the missing codes.
- A change of `reliability.md` §7.4.

## Decisions

### D1. One check in the leaf conversion (#1)

`string` and `character` in `recipe/value/leaf.rs` refuse `U+0000`. `string` refuses a text that holds `'\0'`. `character` accepts a scalar only in `U+0001..=highest`. The fault is the same `Expected` variant as for a bad length or a bad range. Thus every path of the "Context" table gets the rule, and the answer is `E-RCP-005` with the field line of ADR-0041.

Alternative: refuse `%00` in the form reader of `recipe-web`. Rejected, because a JSON value and a rebind do not pass that reader. The value rules belong to the domain.

Alternative: map SQLSTATE `22P05` in `recipe-db`. Rejected, because the write statement then fails and aborts the transaction. The answer has no field line (ADR-0137, option C).

### D2. No rule-set gate (#1)

The check has no condition on `RuleSet`. ADR-0137 (g) puts the rule in both rule sets. Thus the stored check and the request check stay one function. Task 1.5 pins the rule under both rule sets.

Alternative: run the check only under `RuleSet::Current`. Rejected, because no stored row can hold `U+0000`, so the condition adds a branch without an effect.

### D3. Phrases of the expected forms (#1)

The `Expected` variants stay. The phrases of three keys change in `Display` and in the bundles:

| Key | `en` | `pl` |
|---|---|---|
| `expected-wstring` | a string of at most { $length } UTF-16 units without U+0000 | tekst o najwyżej { $length } jednostkach UTF-16 bez znaku U+0000 |
| `expected-string` | a string of at most { $length } characters from Latin-1 without U+0000 | tekst o najwyżej { $length } znakach z Latin-1 bez znaku U+0000 |
| `expected-char` | one character from U+0001 to { $max } | jeden znak od U+0001 do { $max } |

`Display` writes the `en` phrase with the values, as now.

Alternative: a new variant `Expected::NoNul`. Rejected, because the user then sees a second phrase for one type. The enum is exhaustive, so a new variant also needs a new key in both bundles.

### D4. Property generator (#1)

The ranges of `CHAR`, `WCHAR`, and `STRING` characters in `prop/generate.rs` start at `'\u{1}'`. The `WSTRING` strategy maps `'\0'` to `'\u{1}'` or filters it out. A generated valid value then passes, as `reliability.md` §6 requires.

### D5. XSS tests of a STRING value (#1)

The two tests of `approval_text_xss_tests.rs` that send `payloads()` as a `STRING` value send `plain_payloads()`. The first test gets the name `approval_text_xss_a_string_value_accepts_every_plain_payload_and_shows_it_escaped`. A separate test sends the NUL payload and expects the field line of `E-RCP-005`. The module comment names the new rule and ADR-0137. `recipe_xss_tests.rs` line 773 already skips a payload that the value check refuses.

### D6. Pinning tests of the HTTP layer and PostgreSQL (#1)

The fix of D1 lands in task 1.4. The later tests of `recipe-domain`, `recipe-web`, and `recipe-db` then pass at once. They are pinning tests with the red step of `tasks.md`.

- `recipe-web` runs the add form `POST .../parameters` and the edit form `POST .../parameters/{key}/definition` with `value=%00`. Each answer is `422` with the field line of `E-RCP-005`.
- `recipe-db` runs the use cases `SetParameterValue` and `AddParameter` on PostgreSQL (`tests/storage/atomic/`). Each returns `E-RCP-005` before the first write of the command, except the insert of a new derived type (ADR-0137 (c)). The command commits no change, and the draft keeps its row version.

Under the mutation of the red step, the `recipe-db` test gets the storage error of SQLSTATE `22P05`. That proves the test reaches PostgreSQL.

### D7. Unit of StringLength (#12)

No code changes. Existing tests already pin three scenarios of the spec delta:

| Scenario | Test |
|---|---|
| STRING length in characters and not in UTF-8 bytes | `string_length_counts_characters_and_not_bytes`, `recipe/value/leaf/tests.rs` line 46 |
| WSTRING length in UTF-16 units | `wstring_counts_utf16_units`, `recipe/value/leaf/tests.rs` line 56 |
| max_length of a WSTRING counts UTF-16 units | `wstring_length_counts_utf16_units`, `recipe/value/check/tests.rs` line 143 |

No test pins the scenario "max_length of a STRING counts characters". Task 2.1 adds it with "éé", which has four UTF-8 bytes.

## Risks / Trade-offs

- [The phrase change breaks a test that compares the text] → Task 1.1 changes `Display`, `expected_tests.rs`, both bundles, and `crates/recipe-web/src/error/recipe_tests.rs` together. No test of `e2e/` holds the phrases.
- [A red step leaves a mutation in the tree] → Each task copies the file before the mutation. After the revert, `cmp` compares the file with the copy.
- [The fix of D1 breaks the generator of D4 and the XSS tests of D5] → Tasks 1.2 and 1.3 come before task 1.4. Thus `cargo test --workspace` passes after each task.
- [A `CHAR` cannot hold the value 0, and an S7 `STRING` cannot hold a zero byte] → ADR-0137, "Risks and mitigations".

## Migration Plan

No migration. No stored row holds `U+0000`, so a deploy changes no data. A rollback is the deploy of the previous image. The rollback also changes no data.

## Open Questions

None.
