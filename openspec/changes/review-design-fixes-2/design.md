# Design

## Context

The review of 2026-11-21 (`docs/plan/review/2026-11-21-full-repo-review.md` §4, §5.2) reports the findings of this change. The fix plan (`docs/plan/review/2026-11-21-fix-plan.md` §1, §1.1, §3.2) fixes the owner decisions. Changes A (`2026-11-21-review-code-fixes-2`) and C (`2026-11-21-review-deploy-fixes-2`) left their open points to this change.

This design checks each finding at HEAD `d1f6dbe` (`origin/main`). The facts below come from a read of the files at that commit.

- V-01: `grid.html:9` renders a text leaf as `<input type="text">`. A browser drops LF and CR from the value of such an input. So the next save of a form drops a stored LF or CR without a message.
- V-01: `leaf.rs:219` refuses only `U+0000` (`has_nul`). A form text reaches the same function through `request_text` (`texts.rs:291`, `request_leaf.rs:14`). ADR-0137 (d) and (g) put the rule into every value path and into both rule sets.
- V-01: `check_for_approval` passes `RuleSet::Current` to the snapshot check (`snapshot/check.rs:8`). So the rule set does not separate a command input from a stored value. The mode does: `Mode::Request` (`parse.rs:25`, `request_leaf.rs:52`) and `Mode::Stored` (`check.rs:97`, `stored.rs:58`).
- V-04: `walk.rs` names the value field `value<path>` (line 374). The limit fields are `limits[<i>].min`, `.max`, and `.max_length` (line 436). The value of the hidden field `limits[<i>].leaf_path` is the leaf path (lines 453 to 456). So the leaf path occurs twice for each leaf with a limit. No document limits the length of a leaf path.
- V-04: `settings.rs` sets `DEFAULT_BODY_LIMIT_VALUE` to 4 MiB. ADR-0039 (g) chose that value without the leaf path in the calculation.
- V-03: `migrations/0002_triggers.sql:155,160` compares `created_at` of the struct row with `now()`. `data-model.md` §4.6 (line 1316) says that this test is true only inside the creating transaction. Two transactions can share one start time.
- V-02: change C proved class S (runs 44921173499, 44933685608, 44935433336). A record in the newest segment had one inverted byte at offset 859124. The replay ended at offset 859104, before marker 2 (`chain_seq` 24). `pitr-volume` exited with 0.
- V-02: `check_recovery_end` (`pitr-volume` lines 545 to 557) checks only the segment name of the end. The damaged record and marker 2 lie in that same segment.
- V-15: `csrf.rs:46` sets `ORIGIN_PREFIX` to `https://`. `origin_matches` (line 95) does not read `settings().secure`.
- V-05: `edit.rs:39-43` (`keeps_value`) keeps the value only without value fields and without limit fields. A request with value fields and no `limits[i]` field replaces the limits with an empty set.
- V-06: `restore-test` line 186 runs `stop_server "$dir/data" || true` and then deletes the directory. `pitr-check` lines 90 to 93 do the same in `cleanup`.
- V-07: `backup-health` lines 64 and 65 compute `age=$((now - mtime))` with no lower bound.
- V-18: `ThrottlePolicy` has four public fields (`throttle.rs:90` to 99). `LoginThrottle::new` (`throttle.rs:175`) is a public `const fn`. Its only production caller is `recipe-app/src/identity/throttle.rs:61`. It passes `ctx.settings.throttle`, which `recipe-app/src/identity/settings.rs:77` sets to `ThrottlePolicy::default()`.
- V-20: `check.rs:37` compares only `trees.machine()` with the machine of the recipe. Foreign `LiveUnits` give one `UnknownUnit` item per parameter.

## Goals / Non-Goals

**Goals:**

- Close each finding of fix plan §3.2 with the smallest set of ADRs that the rule "one ADR for one decision" permits.
- Keep every stored row readable. A new input rule applies only to the input of a command (`RuleSet::Current` of ADR-0120 and ADR-0166 stays as it is).
- Get the V-15 proof before any V-15 design text.
- Record the decision of the owner on each open point (2026-11-22).

**Non-Goals:**

- No rewrite of stored data. The ops query lists stored text leaves with LF or CR.
- No index form of the field names of ADR-0039 (D2 shows that the path form fits).
- No edit of `CLAUDE.md` or `.claude/rules/` other than the clause of task 3.4 (disagreement 3, option B).

## ADR plan

The next free number is 0210. The conditional ADR of V-15 is the last number, so a dropped V-15 leaves no gap.

| ADR | Title | Findings | Amends |
|---|---|---|---|
| 0210 | No LF and no CR in a text value at command input | V-01 | ADR-0137 (a), (b), (d), (g), (k), ADR-0180 (a), (b), ADR-0201 (a), ADR-0202 (f) |
| 0211 | Length of a leaf path, the body limit, and the permits of the value routes | V-04 | ADR-0039 (g) |
| 0212 | The struct-member trigger compares the transaction | V-03 | ADR-0006 (the trigger of immutable revisions) |
| 0213 | Documents follow the code, and the pair of value and limit fields | V-08, V-20, V-22, V-24, V-27, V-28, V-30, V-31, V-05 | ADR-0028, ADR-0029, ADR-0168 (a), (e) |
| 0214 | Permitted `from_bytes` constructors of `SessionToken` and `CsrfToken` | V-16 | ADR-0166 (u) |
| 0215 | Grammar of `ThrottlePolicy` | V-18 | None |
| 0216 | Known limits of security events, sessions, and backup health | w13 note, V-23, V-07 clock limit | ADR-0109 (ae) |
| 0217 | End of a recovery at the last archived record | V-02 | ADR-0106 (g) step 2, (p) step 4 |
| 0218 | Failed stop of a scratch server | V-06 | ADR-0067 (g), ADR-0068 |
| 0219 | Clock rule of `backup-health` | V-07 | ADR-0066 (d) |
| 0220 | Origin check of a dev build on plain HTTP (conditional) | V-15 | ADR-0035 (e) |

Each ADR copies the shape of ADR-0168 or ADR-0171: Status, Date, Deciders, Context, Decision drivers, Considered options, Decision, Consequences, Related. Each amended ADR gets one "Amended by ADR-NNNN (scope)" note on its Status line. The ADR-0074 typo needs no ADR (D14).

Why the groups are as they are:

- ADR-0213 is the amendments ADR of fix plan §3.2. The plan also puts the pair rule of V-05 into "the amendments ADR". V-05 is the only item of ADR-0213 that changes behavior.
- ADR-0214 and ADR-0215 hold V-16 and V-18 apart (open point 8). V-16 only permits two constructors. V-18 adds a grammar, the error code `E-IDN-012`, and an API change.
- V-06 and V-07 stay apart. One is a cleanup rule, the other is a clock rule.

## Decisions

### D1: LF and CR at command input (V-01, ADR-0210)

ADR-0210 uses the pattern of ADR-0115: the domain range is narrower than the vendor range. The vendors permit `$N`, `$R`, and `$L` in a string. The domain refuses LF (`U+000A`) and CR (`U+000D`) in a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value.

- (a) The rule applies to each value that a command receives. These are the JSON value, the form text of `AddParameter` and `UpdateParameterDefinition`, and a new member of `RebindTypeRevision`. A break gives `E-RCP-005`.
- (b) The rule is not part of `RuleSet::Current` or `RuleSet::Earlier`. `check_for_approval`, the snapshot check, and the export check `loadable` do not apply it. So a stored value with LF or CR stays readable, approvable, and loadable.
- (b1) The seam is `Mode::Request` of `leaf.rs:75`. `string` and `character` refuse LF and CR only when the mode is `Mode::Request`. `ParameterValue::parse` (`parse.rs:25`) and `request_leaf` (`request_leaf.rs:52`) use that mode. `request_leaf` serves the form texts (`texts.rs:291`) and the new members of a rebind (`rebind/value.rs:159`, 160).
- (b2) The approval check and every stored read use `Mode::Stored` (`check.rs:97`, `stored.rs:58`). These paths keep the rule of HEAD. `texts.rs:187` and 207 also use `Mode::Stored`, but only after `request_text` checked each form text.
- (c) ADR-0210 narrows or changes these items. Each amended ADR gets one note with its item scope:
  - ADR-0137: "Amended by ADR-0210 (items (a), (b), (d), (g): LF and CR at command input only, item (k): new expected texts)".
  - ADR-0180: "Amended by ADR-0210 (items (a), (b): LF and CR at command input only)".
  - ADR-0201: "Amended by ADR-0210 (item (a): LF and CR at command input only)".
  - ADR-0202: "Amended by ADR-0210 (item (f): LF and CR at command input only)".
  - The decision map row of the value rules names ADR-0210 next to these ADRs.
- (d) The six expected texts of `E-RCP-005` (`expected.rs` lines 130 to 153) name LF and CR. The `ftl` keys of `en.ftl` and `pl.ftl` (lines 544 to 549) do the same. Tests pin these texts: `expected_tests.rs`, `leaf/utf8_tests.rs`, and `recipe-web` `error/recipe_expected_tests.rs`.
- (e) The ops query `deploy/db/check-platform-ranges.sql` gets the part `line_break`. It lists each stored text leaf with LF or CR, on every platform and every version status.
- (e1) The query header says "rows that the current rule set rejects" (line 3). A `line_break` row is not such a row, so the header gets a second sentence for this part. The part lists rows that every rule set accepts, but that the next form save changes.
- (e2) `docs/ops/runbook.md` explains each part of the query (lines 1185 to 1208). Its paragraph for `line_break` says this: an approved row stays loadable. Before the next form save of a draft row, the owner changes the value or approves the loss of the line break.
- (f) Docs: `domain-model.md` §2.1 (character ranges), `reliability.md` §4.3 (line 521).

The fix plan decided "the read of a stored row does not change". So D1 does not follow the pattern of ADR-0180 (j), which puts its rule into the export check. The section "Disagreements" names this point.

### D2: Leaf path length and the value body limit (V-04, ADR-0211)

The new §7.11 key `budget.leaf_path_bytes` limits the canonical text of a `LeafPath` (`domain-model.md` §2.1, line 196). Its value is 128 bytes (open point 2). `DefineUdt` and `EnsureDerivedType` refuse a longer path with `E-PLT-018 TypeTooLarge`. The §1.2 row of `E-PLT-018` adds "or shorter member names" to its hint. A stored type is read under `RuleSet::Earlier` and keeps a longer path.

No new code is needed. `E-PLT-018` already covers four size limits. A new code would have to come from the range `E-PLT-050..069` (`reliability.md` §1.1), because `E-PLT-030..049` belong to the triggers.

- The enum `TypeLimit` (`plant/error.rs:20`) gets a fifth variant for the leaf path. The match of `recipe-web` `error/plant.rs:35` to 38 then needs a new `ftl` key.
- The §7.11 value is `128 bytes`. `deploy_values/units.rs` reads each §7.11 row. So the key needs the row `("budget.leaf_path_bytes", "bytes")` in `EXPLAINED`, as `check_result.max_bytes` has.
- The check runs only where a command creates a type row: `recipe-app` `plant/define_udt.rs` and `plant/ensure_derived.rs`. The array types of a rebind reuse that path through `DerivedTypes`.
- The check does not go into `recipe-domain` `TypeTree::resolve`. That function applies the size limits on every read (`type_tree.rs:291` to 294), so a check there would make stored types unreadable. `TypeTree` has no `collect`. `PlatformTypes::collect` (`type_tree.rs:37`) reads the stored rows.

The arithmetic. N is `budget.leaves_per_parameter` (16 384). B is `budget.value_bytes` (1 MiB). P is the encoded length of one leaf path. A form body percent-encodes `[`, `]`, and `,` to 3 bytes each. The grammar of `LeafPath` (`domain-model.md` §2.1) holds only ASCII, so no other character of a path grows.

1. The value pair is `value` + P + `=` + text + `&`: 7 + P bytes and the text.
2. The hidden pair `limits%5B16383%5D.leaf_path=` + P + `&` is at most 29 + P bytes.
3. The pairs of `.min` and `.max` are 23 bytes each, without their texts.
4. The texts of a non-string leaf (value, min, max) are at most 40 encoded bytes each, so 120 bytes. The bound covers the canonical texts that the server renders. The longest is an `LDT#` or `DTL#` text with 39 bytes. The test of task 4.4 asserts that each canonical `Display` text encodes to at most 40 bytes.
   - A typed text can be longer. `LDATE_AND_TIME#2026-10-12-12:30:00.000000001` has 44 raw and 50 encoded bytes. With 50 bytes for each text, the bound of L = 128 is 16 449 536 bytes, still below 16 MiB.
   - Leading zeros make a typed text unbounded. A typed form above the limit gets `413 E-WEB-003`. ADR-0211 states both points.
5. The texts of the string leaves are at most 3·B encoded bytes. This counts a value whose JSON text is at most B. The `CHECK` of `data-model.md` §1.3 limits `pg_column_size`, the size after compression, so it does not bound the raw text (risk below).
6. The fixed fields (`csrf_token`, `row_version`, `type`, `name_en`, `description_en`, and the short fields) get F = 64 KiB. The two texts have at most 200 and 4 000 characters. At 4 UTF-8 bytes and 3 encoded bytes for each byte, they need at most 50 400 bytes.

The bound is F + 3·B + N·(82 + 120 + 2·P) = 6 520 832 + N·2·P bytes. A path of L raw bytes with E characters `[`, `]`, and `,` encodes to L + 2·E bytes. The limits of 32 levels and 3 indices per array bound E. For L = 128, E is at most 80 (32 segments `[0]` and 8 segments that grow to `[0,0,0]`). For L = 64, E is at most 42 (21 segments `[0]`).

The index form has the names `v<n>` and `l<n>.min` and no hidden field. Its pairs need 8 + 12 + 12 = 32 bytes for each leaf without the texts. So its bound is F + 3·B + N·(32 + 120) = 5 701 632 bytes.

| L | P | Bound | Limit |
|---|---|---|---|
| 64 | 148 | 11 370 496 (10.8 MiB) | 11 MiB |
| 128 | 288 | 15 958 016 (15.2 MiB) | 16 MiB |
| any (index form) | none | 5 701 632 (5.4 MiB) | 6 MiB |

Every row is above the current 4 MiB. The path form with L = 128 needs 16 MiB. This value is not "too large" in the sense of fix plan §1 V-04. One request buffers the body about three times (bytes, pairs, parsed texts). That is about 48 MiB of the 512 MiB of `container.app_memory`. So the result does not need the index form. The index form would also lose the leaf path of the limit mode of ADR-0039, which is a redesign.

- (a) `http.body_limit.value` changes from 4 MiB to 16 MiB in §7.11, `api.md` §7.6, and `settings.rs`. It stays a fixed value and not an operator setting.
- (b) ADR-0039 (g) gets the note "Amended by ADR-0211 (16 MiB from the budgets and the leaf path length)".
- (c) The rebind form sends one field `new_member_values[<key>]<leaf path>` for each new member leaf (`api.md` §2, row `.../rebind`). ADR-0044 (b) permits at most `form.rebind_leaves` (16 384) new leaves on one page, across all moved parameters.
  - A `ParameterKey` is a UUID of 36 bytes. So one pair needs 61 + P bytes and its text: `new_member_values%5B` (20), the key (36), `%5D` (3), `=`, and `&`.
  - With P = 288 and 40 bytes for each text, the pairs need at most 16 384 · 389 = 6 373 376 bytes. With F, that is 6 438 912 bytes.
  - Each moved parameter can carry string texts up to `budget.value_bytes`, so 3·B encoded bytes. The string texts thus grow with the number M of moved parameters.
  - The rest of 16 MiB is 10 338 304 bytes. That holds the full string texts of 3 moved parameters (3 · 3 145 728 bytes), but not of 4.
  - A rebind with more encoded string text gets `413 E-WEB-003`. The next step of the user: send the rebind with shorter texts for the new members. Then set the full texts with `SetParameterValue` on each parameter. That form has the bound of the table.
- (d) A test builds the largest form of the table and sends it. The save succeeds. The tests that pin 4 MiB move to 16 MiB (task 4.4).
- (e) Permits on the value routes (open point 3). The new §7.11 key `http.value_body_permits` is 4. It counts the requests on the four value routes that wait for, read, or hold a body at the same time.
  - A new layer `value_body_permit` in `recipe-web` `limits.rs` takes an owned permit for each `POST` on the four patterns of `limits.rs`. It runs between the request timeout and the body deadline (`routes.rs:366` to 380).
  - So it runs before `csrf_layer` buffers the body (`csrf.rs:220` to 222). It also runs outside `timeout::body_deadline`, so the body deadline wraps the body only after the wait. A waiting request thus keeps its full body deadline. The request holds the permit until the handler returns its answer.
  - The wait for a permit lasts at most `http.body_read_timeout` (10 s). Then the layer returns `AppError::Timeout { stage: ValueBodyPermit }`. `render_errors` gives `503 E-WEB-009` with `Retry-After`. The layer reads no body, and nothing writes.
  - No new error code. `E-WEB-009` owns every timeout (`reliability.md` §1.1). The §1.1 bullet that lists the users of `E-WEB-009` names the new stage before the variant exists.
  - Memory: 4 permits of about 48 MiB each use about 192 MiB. With the 76 MiB of the Argon2 semaphore, that is about 268 MiB. `soak.memory_ceiling` is 384 MiB, and `container.app_memory` is 512 MiB.
  - `reliability.md` gets the row of the key in §7.11, the row of the wait in §7.4, and the semaphore in the list of §8. The memory sentence of §7.8 (line 916) names the permits. The load test of §6 gets a case with parallel 16 MiB bodies.
- (f) `budget.value_bytes` limits the stored size (`pg_column_size`), as the `CHECK` of `data-model.md` §1.3 does (disagreement 6, option A). The body limit assumes a value whose raw JSON text has at most `budget.value_bytes`. ADR-0211 states both readings next to each other. No code changes.
- (g) ADR-0211 Consequences name the remaining risk. The layer runs before `resolve_access` and `session_layer`, so each client takes a permit, also a client without a session. `session_layer` then refuses a missing session before the body read (`session.rs:82` to 84), and the permit returns at once.
  - `csrf_layer` buffers the body before the token check and the permission check, so any authenticated role can hold a permit for the body read. A client with a session can hold the 4 permits for one body deadline. Other value requests then wait, and some get `503 E-WEB-009`. The memory stays inside the bound above.

### D3: The struct-member trigger (V-03, ADR-0212)

Migration `0019` runs `CREATE OR REPLACE FUNCTION plc_type_immutable()`. The member branch compares the `xmin` of the struct row with the current transaction id (`pg_current_xact_id()`, PostgreSQL 16). It keeps the test of `created_at`. A member insert passes only when both tests are true: `xmin = pg_current_xact_id()::xid` and `created_at = now()`.

- (a0) `xmin` has 32 bits, and a frozen row keeps its raw `xmin`. After a wraparound, an old struct can match the current id. The `created_at` test closes that case. The `xmin` test closes the case of two transactions with one start time.
- (a2) With no visible struct row, `v_kind <> 'struct'` and `v_created <> now()` are NULL, so HEAD passes the insert (`0002_triggers.sql:155` to 163). The new function adds `IF NOT FOUND THEN RAISE 'E-PLT-035 ...'` after the `SELECT`. Its compares use `IS DISTINCT FROM`, so a NULL value refuses.
  - The foreign key needs a visible parent at its later check. So only a commit between the two checks opens this window, and no route knows the identifier of an uncommitted struct. Task 4.5 tests the branch with a deferred foreign key in one test transaction, or it records why no test can reach the branch.
- (a) Under a `SAVEPOINT`, `xmin` is the id of the subtransaction. `recipe-db` opens no savepoint at HEAD. ADR-0212 states the rule "the struct insert and the member insert run in the top-level transaction" and names the risk.
- (a1) `recipe_app` has the table grant `plc_type|SELECT` (`grants.expected` line 107), so the trigger reads `xmin` without a new grant. No savepoint exists in `crates/recipe-db/src`, `crates/recipe-db/tests`, or `crates/recipe-web/src/test_support`. No `EXCEPTION` block exists in `migrations/`. Task 4.5 chooses the exact `xid` and `xid8` expression.
- (b) `deploy/pgtools/restore-code.expected` gets the new `FUNCTION` hash line of `plc_type_immutable`.
- (c) `data-model.md` §4.6 states the new test in its text (line 1316) and in its SQL copy of the function (lines 1319 to 1341). The migration list of `data-model.md` §8 (line 2389) gets the row of `0019`. ADR-0006 gets the note "Amended by ADR-0212".
- (c1) `CREATE OR REPLACE FUNCTION` keeps the owner and the grant `PUBLIC:plc_type_immutable()|EXECUTE` of `grants.expected`. The function keeps `SET search_path = pg_catalog, public, pg_temp`.
- (d) The red test runs two connections. Transaction B reads its `now()`. A second connection, as a superuser with `session_replication_role = replica`, sets `created_at` of a committed struct to that value and commits. Then B inserts a member. HEAD accepts the row. After `0019`, the `xmin` test refuses it.
- (e) Task 4.6 takes the new `FUNCTION` hash from the output of the test `restore_code` on the migrated database. The hash covers `pg_get_functiondef` (`restore-code.sql:9`, 31, 79), not the migration file.

### D4: The amendments (ADR-0213)

Fix plan §3.2 asks for each amendment whether it restates a current decision. The column "Restates" gives the answer.

| Item | Finding | Target | Restates |
|---|---|---|---|
| (a) | V-08 | `data-model.md` §6 intro (line 2227): a command can read identifiers in one `READ ONLY` transaction before its command transaction. The command transaction locks and checks again | Extends ADR-0168 (a) to (e) from three rows to every command. New general rule |
| (b) | V-22 | `domain-model.md` §2.4 (line 564): "Four actions" | Yes, `data-model.md` §10 and ADR-0154 (j) |
| (c) | V-24 | `overview.md` §5 (line 133): `recipe-app` always links `parking_lot`. ADR-0029 gets a note | Yes, ADR-0172 (y) |
| (d) | V-27 | `reliability.md` §8 (line 1248): five more places | Two places restate ADR-0172 (x), (y). The `Gate` permits of `blocking.rs` restate ADR-0050 and ADR-0056. `BasicGate` restates ADR-0109 (n). The request-id `AtomicU64` (`pipeline.rs:70`) is new |
| (e) | V-28 | `reliability.md` §8 (line 1244): "or a banned dependency edge" | Yes, `deny.toml` `[bans]` and `CONTRIBUTING.md` line 77 |
| (f) | V-30 | `api.md` §4 step 1 (line 410): every form that mutates the version | Yes, ADR-0008 ("every edit form and every HTMX fragment that mutates the draft"). The compare form of ADR-0108 (d), `POST /lang`, and `POST /logout` keep no field |
| (g) | V-31 | `reliability.md` §7.7 (line 871): `pitr-volume --drill --target-time <time>` | Yes, ADR-0106 (o) and the argument rule of (f) step 1 ("`--drill` requires `--target-time`") |
| (h) | V-20 | ADR-0028 (line 63): foreign `LiveUnits` give one `E-PLT-020` item per parameter inside `E-RCP-020`. An empty snapshot gives no item, and its approval gives `E-RCP-008` | No. It aligns ADR-0028 with the code (open point 6) |
| (i) | V-05 | `api.md` §2.6 (line 215): the pair rule of D5. ADR-0029 gets a note | No. New behavior (open point 1) |

The items (b), (c), (e), (f), and (g) only align a text with a current decision. ADR-0213 lists them for the Status notes and the decision map. It makes no new decision for them.

- Each Status note of a restating item names its source. For example, the note on ADR-0029 says "Amended by ADR-0213 (item (c) restates ADR-0172 (y), item (i) adds the pair rule)". So the note does not hide the ADR that made the decision.
- Item (a) rewrites the sentence "Every command runs in exactly one transaction" of `data-model.md` line 2227. The general rule keeps the conditions of ADR-0168 (e). The read transaction is `READ ONLY` and holds no lock. The command transaction locks the rows and checks them again.
- Item (a) also adds one clause to `CLAUDE.md` and `.claude/rules/sql.md` (disagreement 3, option B). Task 3.4 names the exact sentences.
- Item (h), the empty case: `snapshot/check.rs:36` to 38 compares the machine only when a first parameter exists. So an empty snapshot with foreign `LiveUnits` gives no item. The approval of an empty draft gives `E-RCP-008` on another path. ADR-0213 (h) states this case, and task 4.16 tests it.

### D5: The pair of value and limit fields (V-05, ADR-0213 (i))

The question of the owner (fix plan §1.1 V-05): are the value fields and the limit fields optional only as a pair (`api.md` §2.6)?

| Option | Rule | Cost | Risk |
|---|---|---|---|
| 1 (decided) | A pair. Without a type change, a request with only one part gets `422 E-WEB-008`. A type with no leaf that accepts a limit has no limit fields | One check in `edit.rs` before the use case. It reads `Ready.form.preview` (`prepare.rs:24`, 101) to see whether the type has a leaf with a limit. A leaf takes a limit unless its class is `Bool` or `Char` (`grid/walk.rs:409`) | None. A browser form always sends both parts, because `walk.rs` renders `limits[i].leaf_path` for each limit leaf |
| 2 | Each part is optional alone. A missing part keeps its stored state | A new input shape in `recipe-app` and `Draft::set_definition` | Larger change. A save can then keep old limits next to a new value without a check |
| 3 | Keep HEAD and document it | One sentence in `api.md` §2.6 | A changed request silently deletes every limit of the parameter |

Option 1 fails closed and keeps the stored data. The `recipe-management` delta "Type change needs a new value" follows option 1. The owner chose option 1 on 2026-11-22.

### D6: Constructors of identity value objects (V-16 in ADR-0214, V-18 in ADR-0215)

- (a) V-16: ADR-0214 names `SessionToken::from_bytes` and `CsrfToken::from_bytes` as permitted constructors only for random bytes that the application supplies. Stored and cookie values go through `parse`. This uses the pattern of ADR-0166 (u), which names `OneTimePassword::from_bytes` only. ADR-0166 gets the note "Amended by ADR-0214 (two more `from_bytes`)". No code changes.
- (b) V-18 (ADR-0215, open point 5): `domain-model.md` §2.3 (line 523) gets the grammar of `ThrottlePolicy`. The fields become private and get accessors. `ThrottlePolicy::parse(free_failures, delay_cap, window, address_budget)` checks the grammar.
- (b1) `LoginThrottle::new` goes. `LoginThrottle` implements `From<ThrottlePolicy>`, because the policy is already a parsed value. So `LoginThrottle` has no public constructor that skips the grammar.
- (b2) The field reads outside the module change to the accessors:
  - `recipe-db/src/login_attempts.rs:126`
  - `recipe-app/src/memory/identity/pool.rs:57`
  - `recipe-app/src/identity/queries/users.rs:132`
  - `recipe-app/src/identity/hasher/tests.rs:279` to 284
  - `recipe-db/tests/storage/contract/attempts/mod.rs:118`
- (b3) The test literals change to `parse`: `recipe-db/tests/storage/contract/attempts/mod.rs:132` and `recipe-app/src/memory/identity/tests/begin_attempt.rs:20` to 24.
  - These fixtures use `address_budget` 2 (`attempts/mod.rs:132`) and 3 (`budget(3)`, `begin_attempt.rs:85`, 99) with the default `free_failures = 3`. The grammar of (c) refuses them.
  - So each of these fixtures sets `free_failures` inside the grammar: 1 for the budget 2, and at most 2 for the budget 3.
- (b4) The exhaustive match of `recipe-server/tests/error_registry/identity.rs` does not compile with the new variant. It gets the variant. The status map of `recipe-web` `src/error/lists.rs` (the `IdentityError` arms) gets it too.
- (c) The grammar: `free_failures` from 1 to `address_budget` − 1. `delay_cap` from 1 s to `window`. `window` from 1 s to 24 h. The defaults of §7.11 (3, 60 s, 900 s, 100) pass.
- (d) A break gives the new code `E-IDN-012` `IdentityError::ThrottlePolicyInvalid { field }`. The status is `-`. No route reaches it, because `settings.rs:77` uses the constant default. A later setting of the policy reaches it only at startup. Its row in `reliability.md` §1.2 comes before the variant.
- (e) `ThrottlePolicy::default()` stays a constant inside the module and needs no `parse`. A unit test runs `parse` on the default values.

### D7: Known limits (w13 note, V-23, V-07 clock limit, ADR-0216)

ADR-0216 records three limits and the next step of each. No code changes.

- (a) w13: `refuse` (`recipe-app/src/auth/authorize.rs:43`, the await on line 59) and `csrf::reject` (`csrf.rs:198`) await the insert of the event. A client that closes the connection before the commit can suppress one `forbidden` or `csrf_rejected` event. A dropped request leaves no line, because `deploy/Caddyfile` has no `log` directive and `on_failure` in `pipeline.rs` is `()` (owner decision of 2026-11-22). A lost event that was the only one in `alerts.security_window` keeps the alert `security_events` inactive (`reliability.md` §3.7, ADR-0216 (b)). A lost `forbidden` on a permission other than `UserManage` never reaches that alert (ADR-0216 (c)). The next step: none for the user.
- (b) V-23: a render or header fault after the commit of `ChangeOwnPassword` drops the new session cookie. The user sees `500 E-WEB-010` and then `E-WEB-004`. The new password is stored. The next step: the user logs in again with the new password.
- (c) V-07 clock limit (D11 (i)): a complete set that `backup-health` reads with a manifest time ahead of the clock stays untrusted (ADR-0219 (c)). It stays untrusted until the clock passes its manifest time minus `alerts.interval` (ADR-0219 Risks). A fix of the clock does not clear the set sooner, because the retention of ADR-0066 (k) compares the same manifest time. The next step: the operator fixes the clock (ADR-0219 (i)). Then the healthcheck fails until that border.
- (d) ADR-0216 amends ADR-0109 (ae): Caddy writes no access log. `api.md` §2 (row `POST /me/password`) and `reliability.md` §3.7 link to it.

### D8: End of a recovery at the last archived record (V-02, ADR-0217)

The new sub-check is part of `check_recovery_end`, in step 9 of ADR-0106 (f). It runs only without a target time, after `check_promoted` and after the hash line of step 7.

The WAL facts of these steps come from recall of the PostgreSQL 16 source and docs. Task 2.7 checks them against the PostgreSQL 16 source (`xlogrecord.h`, `xlog_internal.h`) before it writes ADR-0217.

1. The check reads the end LSN from the history file (`switch_lsn`), as now.
2. At a segment boundary (`lsn mod wal_segment_size = 0`), `pg_walfile_name` names the preceding segment (`pitr-volume:549`). The check then uses the next segment. When `/wal` does not hold that segment, the check passes.
3. When the segment of the end is the newest segment of `/wal` on the base timeline, it opens that file or its `.partial` file.
4. It reads the length `xl_tot_len` (4 bytes) of the record at the offset of the end. At a page boundary, the record starts after the page header: 24 bytes, or 40 bytes at a segment start. A record header that crosses a page boundary gives the strict form: the check reads the bytes from the offset of the end.
5. A length of 0 is a clean end. The check passes and writes no `WARN` line.
6. Otherwise the check computes the physical extent of the record. `xl_tot_len` excludes the page headers, so the check adds each page header that the record crosses. A record that crosses the end of the segment continues in the next segment file.
7. Each page inside the extent after its first page is a continuation page. An all-zero page counts as "not written", which agrees with a torn record. Any other page must have the flag `XLP_FIRST_IS_CONTRECORD` and the `xlp_rem_len` that the length gives, else the check refuses the end.
8. The check reads the bytes from the end of the extent to the end of the newest file. When one of these bytes is not zero, the check calls `fail "$RECOVERY_END_ERROR"`.
9. Otherwise the last record is torn or damaged, and only zero bytes follow it. The check passes and writes one `WARN` line. The line says that the newest record may be lost, because it is torn or damaged.

The `ERROR` line is the line of the gap: `the recovery ended before the end of the WAL archive`. So `pitr-recovery.sh` sets `EARLY_END_REFUSAL_REQUIRED=1` and `EARLY_END_ERROR="the recovery ended before the end of the WAL archive"`. `test_early_end_requirement_matches_the_proof_result` then holds `proof_class = "R"` and that message.

- (a) `pg_receivewal` runs without compression (`walarchive-entry:154`). It fills each new file with zero bytes and writes only received WAL. So a complete archive has only zero bytes after its last record.
- (a1) The tail rule tells two cases apart. A damaged record has more records after it, so a non-zero byte follows its length. A torn last record after a crash has only zero bytes after it. In the proof run, the inverted byte (offset 859124) lies in the CRC field of the record at 859104, so its length stays readable.
- (a1a) One case remains. A damaged length can end its extent inside the last written page, after the later records of that page. No continuation page is then in the extent, and only zero bytes follow it. The check then passes a damaged archive with one `WARN` line. ADR-0217 records this risk.
- (a2) Rejected option (open point 9): the strict form for every case. It refuses a torn last record too. A run with `--target-time` then cannot reach the newest commit, because a target after the newest commit gives `TARGET_ERROR`. The owner kept the tail rule on 2026-11-22.
- (b) ADR-0217 adds the tail rule to check 2 of ADR-0106 (g). It adds the fourth run of change C to the list of ADR-0106 (p) step 4. That run now expects the `ERROR` line. The body of ADR-0106 does not change. Its Status line gets the note "Amended by ADR-0217".
- (c) The CI job `pitr-recovery` does not run on a pull request (`ci.yml` line 542). The owner or the orchestrator dispatches it on the branch. The step `refusals` must show class R.
- (d) `docs/ops/runbook.md` names the new check and the next step after a refusal (risk below). Line 1040, which change C wrote, and line 1043 ("shows a gap") change.
- (e) `reliability.md` §7.7 (line 862) states check 2. It gets the tail rule in the same change.

### D9: The scenario "Recovery after the loss of the pgdata disk" (V-02 open point)

The GIVEN "an intact WAL archive disk" does not cover a damaged archive. An archive with an inverted byte is not intact, also when the disk works. The THEN "`db` runs on the new volume with every commit that the archive received" is then right for each intact archive.

The tail rule of D8 accepts a torn last record after a crash, which is the case of this scenario. So the THEN stays right, and that scenario stays unchanged. With the strict form of D8 (a2), the THEN fails for a torn last record, and the scenario needs a MODIFIED delta. The `operations` delta of "Checks of a recovered cluster" adds the scenario "Damaged record in the newest segment". It states the refusal for the case that is not intact.

### D10: Failed stop of a scratch server (V-06, ADR-0218)

ADR-0067 (g) and `reliability.md` §7.7 (line 800) say: "A `trap` deletes the scratch cluster on each exit of the script." ADR-0068 states no cleanup rule for `pitr-check`.

The new rule of ADR-0218:

- (a) The trap of `restore-test` and of `pitr-check` deletes the directory only after a successful stop of the scratch server.
- (b) A failed stop keeps the directory, writes one `ERROR` line, and ends the script with a code other than 0. The current-run path of `restore-test` (lines 127 to 136) already has this form.
- (c) A failed stop of a stale scratch server (`delete_stale_runs`, line 186) ends the run before `initdb`. The result file then holds `failed`, because line 288 arms it first.
- (d) ADR-0067 (g) gets the note "Amended by ADR-0218". ADR-0068 gets the note "Amended by ADR-0218 (cleanup of `pitr-check`)". §7.7 line 800 changes.
- (e) `wal-stubs.sh` gets a switch for a failed stop, like `STUB_PGCTL_STOP_FAIL` of `restore-stubs.sh`.
- (f) `pitr-check` writes `pitr check passed` (line 191) before the trap runs. So the test of a failed stop asserts one cleanup `ERROR` line after that line and an exit code other than 0. The test of `restore-test` also asserts the result `failed`.
- (g) Each suite calls its cases by name, for example `restore-test-test:84` and after. So each new case gets a call in the entry point of its suite: `restore-test-test` for `restore-test` and `walarchive-test` for `pitr-check`.
- (h) `docs/ops/runbook.md` gets one step for a scratch server that does not stop. The operator stops the server by hand, then deletes the directory, and then runs the job again.

### D11: Clock rule of `backup-health` (V-07, ADR-0219)

No normative document states a clock rule for `backup-health`. ADR-0093 (a) gives the rule of the job `alerts`: a stored time later than `database_now` by more than `alerts.interval` is untrusted.

- (a) ADR-0219 reuses `alerts.interval` (60 s) as the tolerance. No new §7.11 key.
- (b) The rule applies to each complete set that the loop of `backup-health` reads, not only to the newest. A manifest time later than the clock of `backup` by more than 60 s is untrusted. The healthcheck then fails, because a future time hides the age.
- (c) A manifest time later by 60 s or less counts as age 0.
- (d) The §7.11 row of `alerts.interval` adds §7.7 to its sections. `reliability.md` §7.7 (line 724) states the rule. ADR-0066 (d) gets the note "Amended by ADR-0219".
- (e) The new constant of `backup-health` needs a row in `SCRIPT_ASSIGNMENTS` (24 rows) of `crates/recipe-server/tests/deploy_values/files/scripts.rs`. It also needs a row in `SCRIPT_CONSTANTS` (15 rows) of `deploy_values/file_mutations/scripts.rs`.
- (f) The date stub of `tests/lib/stubs.sh` gets a fixed epoch, so a test can set 60 s and 61 s.
- (g) A mixed case gets a test: an older complete set with a manifest time 61 s ahead and a healthy newest set. The healthcheck fails, because rule (b) applies to each set. The entry point `backup-test` calls each new case by name.
- (h) `docs/ops/runbook.md` gets one step for a manifest time in the future. The operator compares the clock of the host with a time server and fixes the clock. Then the operator waits for the next healthcheck.
- (i) Known limit (owner decision of 2026-11-22). A set with a manifest time ahead of the clock stays untrusted until the clock passes that time minus `alerts.interval`. No step deletes the set sooner. The runbook states this limit.

### D12: V-15 proof and the conditional ADR (ADR-0220)

Static reading at HEAD:

1. `csrf_layer` calls `origin_matches` for each mutating request before it buffers the body (`csrf.rs:217`).
2. `origin_matches` (line 95) passes a request with `Sec-Fetch-Site: same-origin` (lines 96 to 101).
3. Otherwise it needs `Origin` equal to `ORIGIN_PREFIX` + `Host` (line 108). `ORIGIN_PREFIX` is `https://` (line 46).
4. The function reads no setting. So `session.secure=false` does not change the result.
5. A browser sends the `Sec-Fetch-*` headers only to a potentially trustworthy URL. `http://192.0.2.5:8080` is not one. So the browser sends `Origin: http://192.0.2.5:8080` and no `Sec-Fetch-Site`.
6. The check then fails. `reject` writes `csrf_rejected` and the answer is `403 E-WEB-007` (lines 217, 218).

The e2e suite does not show this. Its `baseURL` is `http://127.0.0.1:<port>`, and a loopback URL is potentially trustworthy. `dev_server.rs` binds `Ipv4Addr::LOCALHOST` and uses `WebSettings::default()` (`secure=true`).

Three proof ways. None touches the dev stack of the owner in `~/Documents/recepty-edytor-test`.

| Way | Method | Cost | Evidence |
|---|---|---|---|
| A (decided) | A temporary draft pull request, as task 7.1 of change A. A Playwright project starts Chromium with `--host-resolver-rules="MAP recipes.test 127.0.0.1"` and the `baseURL` `http://recipes.test:<port>`. `dev_server` gets an env switch for `secure=false` on that branch only | One CI run of `playwright.yml` (about 15 minutes) and one small branch | High. A real browser decides the headers |
| B | The same spec, run on the host in a separate worktree and a free port | Needs a local Chromium and Node 22. No CI run | High, but no CI log for the record |
| C | An HTTP test in `recipe-web` with `Origin: http://<host>` and no `Sec-Fetch-Site` | Small | Low. It shows only the server half, not what the browser sends. Task 4.10 adds it anyway as the red test of the fix |

Plan, as change C planned V-02:

1. Task 1.1 runs way A and records the class: R (refusal, `403 E-WEB-007`), P (the login passes), or I (inconclusive).
   - A bad or missing pre-session token also gives `403 E-WEB-007`. So the status alone does not prove the refusal of the Origin check.
   - The spec opens `GET /login` and sends the real login form. It records the `Cookie` header of the request and the reason of the `csrf_rejected` event.
   - The spec asserts that the scheme stays `http`, because the HTTPS-Upgrades of Chromium can change it.
   - A fault of the setup gives class I. Task 1.1 then runs again after a fix of the setup.
2. Class R: task 2.10 writes ADR-0220 in the pattern of ADR-0171. With `session.secure=false`, the check also accepts `http://` + `Host`. It amends ADR-0035 (e) and `api.md` §7.3 (line 789). Task 2.10 also writes the `identity-access` delta of the scenario "Origin with another scheme".
3. Class P: tasks 2.10, 3.12, and 4.10 drop. Nobody writes ADR-0220. The section "V-15 proof result" records the class.

### D13: The test guards (A-05, A-09, A-10, A-11)

No normative document states the rules D11 and D12 of change A. They live in its archived design. So each fix is a test fix and needs no ADR. This design replaces D12 and D11 of change A for these points.

The line numbers of `verified-A.md` come from the branch of change A. The numbers below are those of HEAD.

- A-05: a `Parsed` entry matches by module and field name (`parses_field`, `compare.rs:205` to 221, called on line 239). So two structs `A.role` and `B.role` share one parse. New rule: each `Parsed` entry needs its own parse call in a distinct window. The guard keys each window by the struct and the field, not by the field name alone. So two windows of `A.role` do not cover `B.role`.
- A-09: the `Unread` check reads only the prefix `_` of the Rust name (`compare.rs:226`). Rust permits a read of `form._note`. New rule: an `Unread` field has no read in its module. The guard searches the module text for `._<name>` (`reads_field`, line 199) and for a destructure pattern, for example `let Form { _note: n, .. } = form;`.
- A-10: a `Corpus` entry can name the test of another field (`cover_fault`, `compare.rs:273`, with `has_test_fn` of line 146). New rule: the named test submits the form name of the field in its request. A quoted name elsewhere in the test does not count, because it does not prove that the corpus reached that field.
- A-11 (PLAUSIBLE): a request with a delayed trigger can start after the sentinel of `leaf-grid.spec.ts` (lines 152 to 165). New check: the jump link and the grid have no `hx-trigger` with `delay:`, `throttle:`, or `every`. The spec asserts this on these two elements, not on the whole page. No template has such a trigger at HEAD.
  - After the sentinel, the spec advances the browser clock and asserts that no new request starts.

### D14: ADR-0074 typo

The Status line of ADR-0074 and its row in `docs/adr/README.md` (line 86) hold "the tag file is no set". Both become "the tag file is not a set". The Status line may change (fix plan §1.1). ADR-0174 (ae) holds "it is no set" in its body. An accepted body cannot change, so that text stays (see "Disagreements").

### D15: Order of the work

1. The V-15 proof (group 1).
2. The ADR texts in number order, then the index and the decision map (group 2).
3. The `docs/architecture/` edits of each ADR (group 3). Task 3.2 adds the key `budget.leaf_path_bytes`, task 3.18 adds the key `http.value_body_permits`, and task 3.13 adds the row of `E-IDN-012`. Each comes before its code.
   - Exception: task 3.3 runs after task 4.5, because it copies the function text of migration `0019`.
4. The code tasks of the crates and of `e2e/` (group 4). The tasks of one crate run one after the other, because a half-done edit stops the build of the crate. "Order and shared files" of `tasks.md` gives the order.
5. The deploy tasks (group 5).
6. `CHANGELOG.md` (group 6) and the final gates (group 7).

## V-15 proof result

A Haiku agent fills this section from the log of task 1.1. Write each value as the log shows it. Write `none` for a row that does not apply.

- Proof way (A, B, or C): A.
- Commit and run URL: Commit 5d5c418. Run 45002627693 of the workflow "Browser regression", on draft pull request #99. The pull request is closed and its branch is deleted. The other 461 tests of the run passed.
- Class (R, P, or I): R. Tasks 2.x, 3.x, and 4.x of V-15 stay in the change, as D12 states.
- `Cookie` header of the request and reason of the `csrf_rejected` event: The log shows `cookie=present`, which is the pre-session cookie. The `csrf_rejected` event has no reason field, so the log gives `reason=unknown`.
- Answer of `POST /login` (status and code): Status 403, code `E-WEB-007`. The server log line gives `request failed error_code=E-WEB-007`.
- `Origin` and `Sec-Fetch-Site` of the request, as the log shows them: The browser sent `Origin: http://recipes.test:18082` and no `Sec-Fetch-Site` header. The scheme stayed `http`. The log shows `sec_fetch_site=none`. The line `V15-HINT` gives `origin_check_would_pass=false`.

## Open points for the owner

The owner decided each point on 2026-11-22. Points 11 to 13 are the decisions on the disagreements.

1. V-05 pair rule (D5). Decided: option 1.
2. V-04 value of `budget.leaf_path_bytes` (D2). Decided: 128 bytes with a body limit of 16 MiB. The rebind bound of D2 (c) stays below 16 MiB for the pairs, so the decision holds.
3. V-04 request concurrency on the value routes. Decided: a permit now. D2 (e) gives the key `http.value_body_permits` (4), the wait, and the answer `503 E-WEB-009`. ADR-0211 records the decision.
4. V-15 proof way (D12). Decided: way A, with the checks of D12 step 1.
5. V-18 grammar (D6). Decided: the grammar of D6 (c) with `E-IDN-012`, and `From<ThrottlePolicy>` for `LoginThrottle` (D6 (b1)). The fixtures of D6 (b3) change to fit the grammar.
6. V-20 (D4 (h)). Decided: the ADR-0028 amendment and no code change. ADR-0213 (h) also states the empty case.
7. V-28. Decided: add "or a banned dependency edge" to `reliability.md` §8.
8. ADR-0214 as one ADR or two (V-16, V-18). Decided: two ADRs. ADR-0214 holds V-16, and ADR-0215 holds V-18. The later ADRs move up by one number, and the conditional ADR of V-15 is ADR-0220.
9. V-02 tail rule (D8 (a1), (a2)). Decided: the tail rule, with the page header evidence of D8 step 7. ADR-0217 records the remaining risk of D8 (a1a).
10. V-04 premise (D2 step 5). Decided: `http.body_limit.value` holds a value whose JSON text has at most `budget.value_bytes`. The `web-ui` delta states this premise.
11. Disagreement 3, the read before the command transaction. Decided: option B. Task 3.4 rewrites `data-model.md` line 2227 and adds one clause to `CLAUDE.md` and `.claude/rules/sql.md`.
12. Disagreement 6, the reading of `budget.value_bytes`. Decided: option A. The key limits the stored size (`pg_column_size`), and ADR-0211 records this (D2 (f)). No code changes.
13. Disagreements 1, 2, 4, and 5. Decided: the plan handles them as the section "Disagreements" states. The `identity-access` delta writes `POST .../translations/{lang}`.
14. V-02 page that starts a new record inside the extent (D8 step 7). Decided on 2026-11-22: option 1. Step 7 requires the flag `XLP_FIRST_IS_CONTRECORD` and the expected `xlp_rem_len` on each written continuation page. D8 (a1a) stays the only remaining risk.
15. V-07 set with a manifest time ahead of the clock (D11 (i)). Decided on 2026-11-22: a known limit and a runbook statement. The set stays untrusted until the clock passes its manifest time minus `alerts.interval`, and no step deletes it sooner.

## Disagreements with normative documents

Each point below is a place where this plan and a normative document disagree. The owner decided points 3 and 6 on 2026-11-22. The plan handles the other points.

1. `identity-access` "Request body limits" says 1 MiB for the value routes. `reliability.md` §7.11 says 4 MiB. The delta of this change sets both to `http.body_limit.value`. Task 4.4 moves the tests that pin 4 MiB.
2. The same requirement names the route `PUT .../translations/{lang}`. ADR-0040 permits no `PUT`, and `api.md` §2 names `POST`. The delta writes `POST .../translations/{lang}`.
3. V-08: ADR-0213 (a) permits a read transaction before every command transaction. `CLAUDE.md` ("A command runs in one transaction") and `.claude/rules/sql.md` ("One transaction per command") state no such exception. ADR-0168 already has three such rows. Decided: option B. Task 3.4 rewrites `data-model.md` line 2227 and adds one clause to each of the two files.
4. V-01: ADR-0210 keeps the LF and CR rule out of the export check. ADR-0180 (j) put its new rule into the export check. Fix plan §1 V-01 asks for the first form. ADR-0210 names the pattern of ADR-0180 (j) as a rejected option.
5. ADR-0174 (ae) holds "it is no set" in its body. The amendment rule forbids an edit there. The body stays, and D14 fixes the Status line of ADR-0074 and its index row.
6. V-04: `budget.value_bytes` (§7.11) says "1 MiB per parameter value". The `CHECK` of `data-model.md` §1.3 limits `pg_column_size`, the size after compression. D2 counts the raw size. Decided: option A. The key limits the stored size, and ADR-0211 records this next to the raw-size premise of the body limit (D2 (f)). No code changes.

Two points are not disagreements: ADR-0068 has no cleanup item for `pitr-check`, so ADR-0218 adds one. Change C wrote that the message of class R can differ from the gap line. D8 reuses the gap line, and change C is not normative.

## Risks / Trade-offs

- [A stored LF or CR still drops at the next form save] → The part `line_break` lists such values before the upgrade. The runbook paragraph of D1 (e2) names the next step of the owner. No route takes a JSON value: `.../value` takes the value fields of a form (`api.md` §2).
- [16 MiB bodies use memory] → One request uses about 48 MiB. The permits of D2 (e) keep the buffered bodies at about 192 MiB.
- [A client with a session holds every permit] → Other value requests wait for at most `http.body_read_timeout` and then get `503 E-WEB-009`. Nothing writes. ADR-0211 Consequences name this risk.
- [A stored type has a leaf path above 128 bytes] → It stays readable. Its edit form stays inside 16 MiB while N·P is at most 5 128 192, for example 16 384 leaves with P = 313. Task 4.3 adds a test of the stored case.
- [A compressible value with more than 1 MiB of raw text passes the `CHECK`] → Its form can exceed 16 MiB. It then gets `413`. The route of `SetParameterValue` takes the same form fields and has the same limit, so the value cannot grow further through a form.
- [16 MiB in `http.body_read_timeout` (10 s)] → A client needs about 1.7 MB/s. The LAN of the plant gives more. A slow client gets the timeout of HEAD.
- [V-02: a damaged length field can point past every later record] → The `xlp_rem_len` of each crossed page refuses most such lengths (D8 step 7). The case of D8 (a1a) stays: an extent that ends inside the last written page. The fault needs one damaged byte inside 4 bytes.
- [V-02: the WAL facts of D8 come from recall] → Task 2.7 checks them against the PostgreSQL 16 source before it writes ADR-0217. The stub cases of task 5.1 pin each fact that the check uses.
- [V-03: a future `SAVEPOINT` breaks the `xmin` test] → ADR-0212 states the rule. A test inserts a struct and its members through the real adapter.
- [V-15 way A needs a temporary branch] → The orchestrator deletes the branch after the run. The design keeps the run URL.
- [V-02: a segment after `pg_switch_wal()`] → PostgreSQL 16 zero-fills the rest of the segment after the switch record (`xlog.c` 1250 to 1292). The reader moves the end to the segment boundary (`xlogreader.c` line 880). So step 2 of D8 covers this case. The marker runs of ADR-0106 (p) steps 4 and 6 must still pass. Task 5.1 adds a stub case for a switched segment.

## Migration Plan

- Migration `0019_struct_member_transaction.sql` replaces the function `plc_type_immutable`. It runs no table scan and takes no exclusive lock on a table.
- The ops query part `line_break` runs before the upgrade, as `docs/ops/runbook.md` names for `check-platform-ranges.sql`.
- No data migration. A rollback is a new forward migration or a restore (`reliability.md` §5).
