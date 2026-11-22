# Tasks

Marks: `[opus]` is an ADR text (Opus). `[mechanical]` is a Haiku task. `[deploy]` is a Codex task. A task without a mark is a Sonnet task. Run every command from the root of the worktree.

## 1. Proof of V-15

- [x] 1.1 (e2e, `crates/recipe-web/examples/dev_server.rs`) V-15, D12. Run way A (open point 4). On a temporary branch, give `dev_server` an env switch for `secure=false`. Add a Playwright project with `--host-resolver-rules="MAP recipes.test 127.0.0.1"` and the `baseURL` `http://recipes.test:<port>`. The spec opens `GET /login` and sends the real login form. It records the status, the code, and the request headers `Origin`, `Sec-Fetch-Site`, and `Cookie`. It also records the reason of the `csrf_rejected` event. It asserts that the scheme stays `http`, because the HTTPS-Upgrades of Chromium can change it. A fault of the setup gives class I, and the task runs again after a fix. Open a draft pull request, so `playwright.yml` runs. Do not touch `~/Documents/recepty-edytor-test` or its containers. Red test: none, because this task is a proof and not a fix. Check: the log of the job `Playwright` of the draft pull request.
- [x] 1.2 (openspec) [mechanical] V-15. Fill the section "V-15 proof result" of `design.md` from the log of task 1.1. Close the draft pull request and delete its branch. Red test: none, because the task writes a record. Check: `openspec validate review-design-fixes-2 --strict`.

## 2. ADR texts

Each ADR task writes one file in the shape of ADR-0168, adds the "Amended by" notes, and runs ste-lint. An ADR has no red test, because it is a document. The check of each task is `python -I tools/ste-lint/ste-lint.py <the ADR file>` with 0 hard violations.

- [x] 2.1 (docs/adr) [opus] V-01, D1. Write `docs/adr/0210-no-lf-and-no-cr-at-command-input.md`. Name the pattern of ADR-0180 (j) as a rejected option. Add the item-scoped notes of D1 (c) to four Status lines. These are ADR-0137 (items (a), (b), (d), (g), (k)), ADR-0180 (items (a), (b)), ADR-0201 (item (a)), and ADR-0202 (item (f)). Check: ste-lint on the new file and on the four amended ADRs.
- [x] 2.2 (docs/adr) [opus] V-04, D2. Write `docs/adr/0211-leaf-path-length-and-value-body-limit.md` with the arithmetic of D2 and the key `budget.leaf_path_bytes`. The ADR also holds these points of D2:
  - Step 4: the bound covers the canonical texts that the server renders, and a typed longer form can get `413`.
  - (c): the rebind bound, the `413` above it, and the next step of the user.
  - (e): the permit decision with the key `http.value_body_permits` (4), the wait of `http.body_read_timeout`, and the answer `503 E-WEB-009`.
  - (f): `budget.value_bytes` limits the stored size (`pg_column_size`), next to the raw-size premise of the body limit.
  - (g) in Consequences: any authenticated role reaches the permit, and a client can hold every permit for one body deadline.
  - Add "Amended by ADR-0211 (item (g): 16 MiB from the budgets)" to the Status line of ADR-0039. Check: ste-lint on both files.
- [x] 2.3 (docs/adr) [opus] V-03, D3. Write `docs/adr/0212-struct-member-trigger-compares-the-transaction.md` with both tests of D3 (`xmin` and `created_at`), the `IF NOT FOUND` branch, and `IS DISTINCT FROM` (D3 (a0), (a2)). Add "Amended by ADR-0212 (the member trigger compares the transaction)" to the Status line of ADR-0006. Check: ste-lint on both files.
- [x] 2.4 (docs/adr) [opus] V-08, V-20, V-22, V-24, V-27, V-28, V-30, V-31, V-05, D4, D5. Write `docs/adr/0213-documents-follow-the-code-2.md` with the items (a) to (i) of D4. Item (h) states the empty case of D4. Item (i) states option 1 of D5. Add the notes to the Status lines of ADR-0028 (item (h)), ADR-0029 (items (c) and (i)), and ADR-0168 (item (a)). Each note of a restating item names its source, for example "item (c) restates ADR-0172 (y)". Check: ste-lint on the four files.
- [x] 2.5 (docs/adr) [opus] V-16, D6 (a). Write `docs/adr/0214-permitted-from-bytes-constructors-of-tokens.md`. It names `SessionToken::from_bytes` and `CsrfToken::from_bytes` as permitted constructors. Add "Amended by ADR-0214 (two more `from_bytes`)" to the Status line of ADR-0166. Check: ste-lint on both files.
- [x] 2.6 (docs/adr) [opus] w13 note, V-23, D7. Write `docs/adr/0216-known-limits-of-security-events-and-sessions.md` with the next step of the user for each limit. Check: ste-lint on the new file.
- [x] 2.7 (docs/adr) [opus] V-02, D8. First check the WAL facts of D8 against the PostgreSQL 16 source (`xlogrecord.h`, `xlog_internal.h`) and the table "Backup Control Functions". Record a fact that differs, and stop for the owner. Write `docs/adr/0217-end-of-a-recovery-at-the-last-archived-record.md` with steps 1 to 9 of D8. Its Consequences name the remaining risk of D8 (a1a). Add the note "Amended by ADR-0217 (items (g) step 2 and (p) step 4)" to the Status line of ADR-0106. The note names the tail rule after the end of the recovery. Check: ste-lint on both files.
- [x] 2.8 (docs/adr) [opus] V-06, D10. Write `docs/adr/0218-failed-stop-of-a-scratch-server.md`. Add the notes to the Status lines of ADR-0067 (item (g)) and ADR-0068 (cleanup of `pitr-check`). Check: ste-lint on the three files.
- [x] 2.9 (docs/adr) [opus] V-07, D11. Write `docs/adr/0219-clock-rule-of-backup-health.md`. Add "Amended by ADR-0219 (item (d): a manifest time ahead of the clock)" to the Status line of ADR-0066. Check: ste-lint on both files.
- [x] 2.10 (docs/adr, openspec) [opus] V-15, D12. Only for class R of task 1.1. Write `docs/adr/0220-origin-check-of-a-dev-build-on-plain-http.md` in the pattern of ADR-0171. Add "Amended by ADR-0220 (item (e): `http://` with `session.secure=false`)" to the Status line of ADR-0035. Add an `identity-access` delta of the requirement "Origin check for mutating requests" with the scenario "Origin with another scheme". Check: ste-lint on both ADR files and `openspec validate review-design-fixes-2 --strict`.
- [x] 2.11 (docs/adr) [mechanical] All ADRs. Run after tasks 2.1 to 2.10 and 2.13. Add the index rows of ADR-0210 to ADR-0219 (and ADR-0220 after task 2.10) to `docs/adr/README.md`. Copy each Status line of the amended ADRs into its index row. The amended ADRs are ADR-0006, ADR-0028, ADR-0029, ADR-0035 (class R only), ADR-0039, ADR-0066, ADR-0067, ADR-0068, ADR-0106, ADR-0109, ADR-0137, ADR-0166, ADR-0168, ADR-0180, ADR-0201, and ADR-0202. Update the decision map rows of the topics of each ADR. Red test: none, because the index is a document. Check: `grep -c '^| \[021' docs/adr/README.md` gives 10, `grep -c '^| \[0220\]' docs/adr/README.md` gives 1 for class R and 0 otherwise, and ste-lint on `docs/adr/README.md`.
- [x] 2.12 (docs/adr) [mechanical] ADR-0074 typo, D14. Change "the tag file is no set" to "the tag file is not a set". Edit the Status line of ADR-0074 and its row in `docs/adr/README.md`. Do not edit the body of ADR-0174. Red test: none, because the task fixes a word. Check: `grep -rn 'tag file is no set' docs/adr/` gives no line.
- [x] 2.13 (docs/adr) [opus] V-18, D6 (b) to (e). Run after task 2.5 and before task 2.6, so the ADR files come in number order. Write `docs/adr/0215-grammar-of-throttle-policy.md` with the grammar of D6 (c), the code `E-IDN-012`, and the API change of D6 (b1). The ADR amends no ADR. Check: ste-lint on the new file.

## 3. Edits of docs/architecture and docs/ops

Each task edits the text that its ADR names. A document edit has no red test. The check of each task is ste-lint on the edited file with 0 hard violations.

- [x] 3.1 (docs/architecture) V-01, ADR-0210. Edit the character ranges of `domain-model.md` §2.1 and `reliability.md` §4.3 (line 521). Check: ste-lint on both files.
- [x] 3.2 (docs/architecture, recipe-server) V-04, ADR-0211. Add the row `budget.leaf_path_bytes` with the value `128 bytes` and the sections `4.4, domain-model.md 2.1` to `reliability.md` §7.11. Change `http.body_limit.value` to 16 MiB in §7.11 and in `api.md` §7.6 (line 823). Add the leaf path length to the `LeafPath` grammar of `domain-model.md` §2.1 and to the size budgets of `reliability.md` §4.4. Add "or shorter member names" to the hint of the row `E-PLT-018` in §1.2. Add `budget.leaf_path_bytes` to both copies of A26 in one edit: `overview.md` §9 (line 271) and `domain-model.md` §8 (line 931). Add `("budget.leaf_path_bytes", "bytes")` to `EXPLAINED` in `crates/recipe-server/tests/deploy_values/units.rs`. Red test: `deploy_values` fails after the new row and before the `EXPLAINED` entry, because the unit word `bytes` of a count needs that entry. Check: `cargo test -p recipe-server --test deploy_values && python -I tools/ste-lint/ste-lint.py docs/architecture/reliability.md docs/architecture/api.md docs/architecture/domain-model.md docs/architecture/overview.md`.
- [x] 3.3 (docs/architecture) [mechanical] V-03, ADR-0212. Run after task 4.5, because the task copies the function text of migration `0019`. Replace the `created_at` sentence of `data-model.md` §4.6 (line 1316) with the two tests of D3: `xmin` and `created_at`. Copy the function text of migration `0019` into the SQL block of §4.6 (lines 1319 to 1341). Add the row of `0019_struct_member_transaction.sql` to the migration list of §8 (after line 2389). Check: ste-lint on `data-model.md`.
- [x] 3.4 (docs/architecture, CLAUDE.md, .claude/rules) V-08, ADR-0213 (a), disagreement 3 (option B). Edit three files:
  - `data-model.md` §6 intro (line 2227): replace "Every command runs in exactly one transaction, except" with "Every command writes in exactly one transaction, except". Keep the rest of that sentence and the Basic sentences.
  - After that sentence, add: "Before that transaction, a command can read identifiers in one `READ ONLY` transaction that takes no lock. The command transaction then locks the rows and checks them again (ADR-0168 (e), ADR-0213 (a))."
  - `CLAUDE.md`, section "Hard rules", line 59: replace "A command runs in one transaction." with the new sentence below.
    "A command runs in one transaction, after `READ ONLY` pre-reads that take no lock (ADR-0168, ADR-0213 (a))."
  - `.claude/rules/sql.md`, section "Locks and transactions", line 44: replace "One transaction per command, except" with the new text below.
    "One transaction per command, after `READ ONLY` pre-reads that take no lock (ADR-0168, ADR-0213 (a)), except".
  - Check: ste-lint on `data-model.md`, and `grep -c 'ADR-0213 (a)' CLAUDE.md .claude/rules/sql.md` gives 1 for each file.
- [x] 3.5 (docs/architecture) [mechanical] V-22, ADR-0213 (b). Change "Three actions" to "Four actions" in `domain-model.md` §2.4 (line 564) and name `PlcConnectionExported`. Check: ste-lint on `domain-model.md`.
- [x] 3.6 (docs/architecture) [mechanical] V-24, ADR-0213 (c). Change the `parking_lot` edge of `recipe-app` in `overview.md` §5 (line 133) to "always (ADR-0172 (y))". Check: ste-lint on `overview.md`.
- [x] 3.7 (docs/architecture) V-27, ADR-0213 (d). Add the five places to the list of interior mutability in `reliability.md` §8 (line 1248), each with its source. Check: ste-lint on `reliability.md`.
- [x] 3.8 (docs/architecture) [mechanical] V-28, ADR-0213 (e). Add "or a banned dependency edge" to the `cargo deny check` line of `reliability.md` §8 (line 1244). Check: ste-lint on `reliability.md`.
- [x] 3.9 (docs/architecture) [mechanical] V-30, ADR-0213 (f). Change step 1 of `api.md` §4 (line 410) to "every form that mutates the version". Check: ste-lint on `api.md`.
- [x] 3.10 (docs/architecture) [mechanical] V-31, ADR-0213 (g). Change the drill command of `reliability.md` §7.7 (line 871) to `pitr-volume --drill --target-time <time>`. Check: ste-lint on `reliability.md`.
- [x] 3.11 (docs/architecture) V-05, ADR-0213 (i). Replace "Otherwise both are optional (ADR-0029)" in the row `.../definition` of `api.md` §2.6 (line 215) with the pair rule of the owner. Check: ste-lint on `api.md`.
- [x] 3.12 (docs/architecture) V-15, ADR-0220. Only for class R of task 1.1. Add the dev-build rule to the Origin bullet of `api.md` §7.3 (line 789). Check: ste-lint on `api.md`.
- [x] 3.13 (docs/architecture) V-18, ADR-0215. Add the grammar of `ThrottlePolicy` to `domain-model.md` §2.3 (line 523), and name `ThrottlePolicy` as the value object with the one `parse` (open point 5). Add the row `E-IDN-012` `IdentityError::ThrottlePolicyInvalid { field }` with the status `-` to `reliability.md` §1.2. Check: ste-lint on both files.
- [x] 3.14 (docs/ops, docs/architecture) [mechanical] V-02, ADR-0217. Add the tail rule of D8 and the next step after a refusal (a run with `--target-time` before the damaged record) to `docs/ops/runbook.md` (line 1040). Change "shows a gap in the archive" (line 1043) to "shows a gap or a damaged record in the archive". Add the tail rule to check 2 in `reliability.md` §7.7 (line 862). Check: ste-lint on both files.
- [x] 3.15 (docs/architecture) [mechanical] V-06, ADR-0218. Replace the `trap` sentence of `reliability.md` §7.7 (line 800) with the rule of D10. Check: ste-lint on `reliability.md`.
- [x] 3.16 (docs/architecture) [mechanical] V-07, ADR-0219. Add the clock rule to the `backup-health` bullet of `reliability.md` §7.7 (line 724). Add §7.7 to the sections of the §7.11 row `alerts.interval`. Check: ste-lint on `reliability.md`.
- [x] 3.17 (docs/architecture) [mechanical] w13 note, V-23, ADR-0216. Add a link to ADR-0216 to the row `POST /me/password` of `api.md` §2 and to the alert `security_events` of `reliability.md` §3.7. Check: ste-lint on both files.
- [x] 3.18 (docs/architecture, recipe-server) V-04 permit, ADR-0211, D2 (e). Edit these places of `reliability.md`:
  - §7.11: add the row `http.value_body_permits` with the value `4 requests on the four value routes that wait for, read, or hold a body at the same time (ADR-0211)` and the sections `7.4, 7.8, api.md 7.6`.
  - §7.4: add the row "Value body permit" with the key `http.body_read_timeout` and the mechanism of D2 (e).
  - §1.1 (line 34): add the value body permit to the users of `E-WEB-009`, with the stage `ValueBodyPermit`.
  - §7.8 (line 916): add one sentence on the memory of the value body permits.
  - §8: add the semaphore of the value body permits to the list of interior mutability.
  - §6: add a load case with parallel 16 MiB bodies on the value routes. The container stays below `soak.memory_ceiling`.
  - `api.md` §7.6: add one sentence on the permit and its answer `503 E-WEB-009`.
  - Add `("http.value_body_permits", "requests")` to `EXPLAINED` in `crates/recipe-server/tests/deploy_values/units.rs`.
  - Red test: `deploy_values` fails after the new row and before the `EXPLAINED` entry. Check: `cargo test -p recipe-server --test deploy_values && python -I tools/ste-lint/ste-lint.py docs/architecture/reliability.md docs/architecture/api.md`.

## 4. Code

- [x] 4.1 (recipe-domain) V-01, D1 (b1), (d). Files: `src/recipe/value/leaf.rs` (`string`, `character`, keyed on `Mode::Request`), `src/recipe/value/expected.rs` (lines 130 to 153), and the pinned texts in `src/recipe/value/expected_tests.rs` and `src/recipe/value/leaf/utf8_tests.rs`. The generators of `src/recipe/value/prop/generate.rs` (lines 347 to 355 and 413 to 429) can emit LF and CR. They map both to another character, as they map `U+0000`. Red tests in `src/recipe/value/leaf/tests.rs`:
  - `line_break_in_a_string_value_is_refused` asserts the `ValueFault` kind of a refused character for `"A\nB"` in `STRING(10)` through `ParameterValue::parse`.
  - `line_break_in_a_char_value_is_refused` asserts the `LeafFault` kind of a refused character for `"\r"` in `CHAR` through `request_text`. `LeafFault` is `pub(in crate::recipe)` (`leaf.rs:41`), so the test stays inside `crate::recipe`.
  - These fault types have no `fn code`. Task 4.17 asserts `E-RCP-005` at the use cases.
  - HEAD accepts both values, because `leaf.rs:219` refuses only `U+0000`.
  - Guard tests: `stored_line_break_passes_the_approval_check` asserts that `check_for_approval` accepts a stored LF. `stored_line_break_stays_loadable` builds an `Approved` version with the stored value `"A\nB"` through `Mode::Stored`. It asserts that the export check passes and that the export body holds `"A\nB"`. Both pass at HEAD and guard D1 (b2).
  - Check: `cargo test -p recipe-domain line_break`.
- [x] 4.2 (recipe-web) V-01, D1 (d). Files: `i18n/en.ftl`, `i18n/pl.ftl` (lines 544 to 549), and the pinned texts in `src/error/recipe_expected_tests.rs`. Red tests: `expected_texts_name_the_line_break` in `src/pages/parameters/adr_0202_tests.rs` asserts that the `en` and `pl` texts of the six `expected-*` keys of D1 (d) name LF and CR. HEAD names only `U+0000`. `form_text_with_a_line_break_gives_e_rcp_005` in `src/pages/parameters/add_tests.rs` posts `value=A%0AB` for a `STRING(20)` leaf. It asserts `422 E-RCP-005` and no stored parameter. HEAD answers `200` and stores the value. Check: `cargo test -p recipe-web line_break`.
- [x] 4.3 (recipe-domain, recipe-app, recipe-web) V-04, D2. Files:
  - `recipe-domain`: `src/plant/limits.rs` (new constant from §7.11) and `src/plant/error.rs` (a fifth `TypeLimit` variant). Do not add the check to `TypeTree::resolve` (`type_tree.rs:291` to 294), because it runs on every read.
  - `recipe-app`: `src/plant/define_udt.rs` and `src/plant/ensure_derived.rs` run the check. The array types of a rebind reuse that path through `DerivedTypes`.
  - `recipe-web`: `src/error/plant.rs` with a new key in `i18n/en.ftl` and `i18n/pl.ftl`.
  - Red tests: `leaf_path_above_128_bytes_gives_e_plt_018` in `recipe-app` `src/plant/define_udt/tests.rs` runs `DefineUdt` with a 129-byte leaf path and asserts `E-PLT-018` and no stored type. A second red test in `src/plant/ensure_derived/tests.rs` does the same for `EnsureDerivedType`. HEAD accepts both, because no length limit exists.
  - Guard test: `stored_type_with_long_leaf_path_resolves` in `recipe-domain` `src/plant/type_tree/tests.rs` reads stored rows with a 200-byte leaf path through `PlatformTypes::collect`. Then it runs `TypeTree::resolve` and expects success.
  - A third test compares the constant with the §7.11 row, as `identity/throttle/tests.rs` does.
  - Check: `cargo test -p recipe-domain -p recipe-app -p recipe-web leaf_path`.
- [x] 4.4 (recipe-web) V-04, D2. Files: `src/settings.rs` (`DEFAULT_BODY_LIMIT_VALUE`), `src/limits.rs`, `src/limits/route_tests.rs` (lines 204, 276, 284 to 299, 354 to 360), and `src/csrf/limit_tests.rs` (line 61).
  - Move the pinned boundaries of the two test files from 4 MiB to 16 MiB on the four value routes. Keep a declared body and a chunked body for each route.
  - Red test: `largest_edit_form_fits_the_value_limit` in `src/pages/parameters/route_tests.rs` builds the edit form of D2 with 16 384 leaves, 128-byte paths, and the longest texts. It asserts that the size is at most `http.body_limit.value` and that the save succeeds with the stored value. HEAD answers `413`, because the limit is 4 MiB.
  - The test also asserts that each canonical `Display` text of a non-string leaf encodes to at most 40 bytes.
  - Check: `cargo test -p recipe-web largest_edit_form_fits_the_value_limit && cargo test -p recipe-web limit`.
- [x] 4.5 (recipe-db) V-03, D3. Files: `migrations/0019_struct_member_transaction.sql`, `crates/recipe-db/tests/storage/triggers/immutable_types.rs`. Red test: `member_insert_with_forged_created_at_gives_e_plt_034` opens transaction B and reads its `now()`. A superuser connection with `session_replication_role = replica` sets `created_at` of a committed struct to that value. B then inserts a member, and the test asserts SQLSTATE and `E-PLT-034`. HEAD accepts the row, because the trigger compares `created_at` with `now()`. A positive test inserts a struct and its members through the adapter. The function of `0019` requires both tests of D3 and adds the `IF NOT FOUND` branch with `IS DISTINCT FROM` (D3 (a2)). A second red test, `member_insert_without_a_visible_struct_gives_e_plt_035`, reaches that branch as D3 (a2) states. It asserts SQLSTATE and `E-PLT-035`. Check: `cargo test -p recipe-db --test storage immutable_types`. The FK test `plc_type_member_struct_type_id_fkey_rejects_an_unknown_struct` in `tests/storage/constraints/types.rs` disables the trigger in its transaction, because the trigger now refuses an unknown struct first.
- [x] 4.6 (deploy/pgtools) [deploy] V-03, D3 (b). File: `deploy/pgtools/restore-code.expected`. Replace the `FUNCTION` line of `plc_type_immutable` with the new hash. Take the hash from the output of the test `restore_code` on the migrated database, because the hash covers `pg_get_functiondef` and not the migration file. Red test: the test `restore_code` of `recipe-db` fails after task 4.5 and before this task. Check: `cargo test -p recipe-db --test storage restore_code`.
- [x] 4.7 (recipe-web) V-05, D5, option 1. Files: `src/pages/parameters/edit.rs` (`keeps_value`, lines 39 to 43) and `src/pages/parameters/route_tests.rs`. `DEFINITION_BODY` (line 15) sends numeric value fields and no `limits[i]`, and its matrix row (lines 88 to 94) expects `saved`. Add the limit fields to `DEFINITION_BODY`, so the row stays `saved` under the pair rule. The check reads `Ready.form.preview` (`prepare.rs:24`, 101) to see whether the type has a leaf with a limit. A leaf takes a limit unless its class is `Bool` or `Char` (`grid/walk.rs:409`). Red tests in `src/pages/parameters/edit_tests.rs`: `value_fields_without_limit_fields_give_e_web_008` asserts `422 E-WEB-008` and unchanged limits. HEAD answers `200` and deletes every limit. `limit_fields_without_value_fields_give_e_web_008` asserts `422 E-WEB-008`. HEAD passes the request to the use case with `retype: Some`. Guard tests: `bool_value_without_limit_fields_is_saved` covers the BOOL scenario of the delta. `blank_limit_row_with_a_value_is_saved` sends a limit row with blank `min` and `max` next to a new value. Check: `cargo test -p recipe-web fields_give_e_web_008 && cargo test -p recipe-web limit_fields_is_saved && cargo test -p recipe-web blank_limit_row`.
- [x] 4.8 (recipe-domain, recipe-app, recipe-db, recipe-web, recipe-server) V-18, ADR-0215, D6 (b) to (e). Files:
  - `recipe-domain`: `src/identity/throttle.rs` (private fields, accessors, `parse`, `From<ThrottlePolicy> for LoginThrottle`), `src/error_codes.rs`, `src/identity/error.rs`.
  - Callers: `recipe-app` `src/identity/throttle.rs:61`, `src/memory/identity/pool.rs:57`, `src/identity/queries/users.rs:132`, and `recipe-db` `src/login_attempts.rs:126`.
  - Test readers: `recipe-app` `src/identity/hasher/tests.rs:279` to 284 and `recipe-db` `tests/storage/contract/attempts/mod.rs:118`.
  - Test literals: `recipe-db` `tests/storage/contract/attempts/mod.rs:132` (budget 2) and `recipe-app` `src/memory/identity/tests/begin_attempt.rs:20` to 24 (`budget(3)` at lines 85 and 99). Each sets `free_failures` inside the grammar (D6 (b3)).
  - The new variant: `recipe-server` `tests/error_registry/identity.rs` (exhaustive match) and the `IdentityError` arms of `recipe-web` `src/error/lists.rs`.
  - Red test: `throttle_policy_parse_refuses_a_bad_field` in `recipe-domain` `src/identity/throttle/tests.rs` asserts `E-IDN-012` for `free_failures = 0` and for `delay_cap` above `window`. It also asserts that the defaults of §7.11 pass. HEAD has no `ThrottlePolicy::parse`, so the test does not compile. Check: `cargo test -p recipe-domain throttle_policy && cargo test -p recipe-app -p recipe-db -p recipe-web -p recipe-server --no-run`.
- [x] 4.9 (recipe-domain) V-16, D6 (a). File: `src/identity/secret.rs`. Change the doc comments of `SessionToken::from_bytes` and `CsrfToken::from_bytes` to cite ADR-0214. Red test: none, because the code does not change. Check: `cargo doc -p recipe-domain --no-deps` with `RUSTDOCFLAGS="-D warnings"`.
- [x] 4.10 (recipe-web) V-15, D12. Only for class R of task 1.1. File: `src/csrf.rs` (`origin_matches`). Red test: `csrf_http_origin_passes_when_secure_is_false` in `src/csrf/tests.rs` sends `Origin: http://<host>` without `Sec-Fetch-Site` with `secure=false` and expects no `403`. HEAD answers `403 E-WEB-007`, because line 108 needs `https://`. A second test keeps the refusal with `secure=true`. Check: `cargo test -p recipe-web csrf_http_origin`.
- [x] 4.11 (recipe-web) A-05, D13. File: `src/pages/xss_coverage_tests/compare.rs`. Red test: `parsed_entry_needs_its_own_parse_window` in `src/pages/xss_coverage_tests/kind_tests.rs` feeds a module with `A.role` and `B.role`, and two parse windows of `A.role` only. It expects a failure for `B.role`. HEAD accepts both entries, because it counts the windows by the field name. The fix keys each window by the struct and the field. Check: `cargo test -p recipe-web parsed_entry_needs_its_own_parse_window`.
- [x] 4.12 (recipe-web) A-09, D13. File: `src/pages/xss_coverage_tests/compare.rs`. Red test: `unread_field_with_a_read_of_its_name_fails` in `kind_tests.rs` feeds `_note` with the kind `Unread` and a handler that reads `form._note`. It expects a failure. HEAD accepts it. A second red test, `unread_field_in_a_destructure_fails`, feeds `let Form { _note: n, .. } = form;` and expects a failure. Check: `cargo test -p recipe-web unread_field_with_a_read_of_its_name_fails`.
- [x] 4.13 (recipe-web) A-10, D13. File: `src/pages/xss_coverage_tests/compare.rs`. Red test: `corpus_test_must_name_the_field` in `kind_tests.rs` gives the field `note` the entry `Corpus("xss_covers_text")`, a test whose request does not submit the field `note`. The test holds the literal `"note"` only in an assertion message. It expects a failure, because the coverage binds to a submitted field name in the request. HEAD accepts it. Check: `cargo test -p recipe-web corpus_test_must_name_the_field`.
- [x] 4.14 (e2e) A-11, D13. File: `e2e/tests/leaf-grid.spec.ts`. Add an assertion that the jump link and the grid have no `hx-trigger` with `delay:`, `throttle:`, or `every`. Scope it to these two elements, not to the whole page. After the sentinel, advance the browser clock and assert that no new request starts. Red test: none at HEAD, because no such trigger exists. A local mutation of `grid.html` with `hx-trigger="change delay:1s"` must make the spec fail. Check: `cd e2e && npm ci && E2E_PASSWORD=<12 to 128 characters> npx playwright test tests/leaf-grid.spec.ts`.
- [x] 4.15 (recipe-app, recipe-web, recipe-server) V-04 permit, ADR-0211, D2 (e). Run after task 3.18. Files:
  - `recipe-app` `src/error/app.rs`: the variant `TimeoutStage::ValueBodyPermit` with the `Display` text `value body permit`. Add it to each exhaustive match of `TimeoutStage`, also in `recipe-server` `tests/error_registry/app.rs`.
  - `recipe-web` `src/settings.rs`: the constant `DEFAULT_VALUE_BODY_PERMITS = 4` and its field of `WebSettings`.
  - `recipe-web` `src/limits.rs`: the layer `value_body_permit` with a `tokio::sync::Semaphore` in the state. It takes an owned permit for a `POST` on the four value patterns and holds it until the handler returns its answer.
  - `recipe-web` `src/routes.rs`: add the layer between `timeout::request_timeout` and `timeout::body_deadline` (lines 366 to 380). Update the order comment.
  - Red test: `fifth_value_body_waits_and_gets_e_web_009` in `src/limits/route_tests.rs` runs with paused time. The test takes the 4 permits from the semaphore of the state, or it holds 4 requests in a gated handler. A body that does not end cannot hold a permit, because the body deadline ends it at the same instant.
  - A fifth request then waits `http.body_read_timeout`, gets `503 E-WEB-009` with `Retry-After`, and its handler does not run. HEAD reads the fifth body at once.
  - Guard tests: a permit returns after an answer, so a later request passes. A `POST` on a form route takes no permit. A third test compares the constant with the §7.11 row, as `identity/throttle/tests.rs` does.
  - Check: `cargo test -p recipe-web value_body && cargo test -p recipe-server --test error_registry`.
- [x] 4.16 (recipe-domain) V-20, ADR-0213 (h). File: `src/recipe/snapshot/tests.rs`. Guard test: `empty_snapshot_with_foreign_units_gives_no_item` checks a snapshot with no parameter against `LiveUnits` of another machine. It asserts no item. A second guard test asserts that the approval check of an empty draft gives `E-RCP-008`. Red test: none, because both pass at HEAD and pin the empty case of D4. Check: `cargo test -p recipe-domain empty_snapshot`.
- [x] 4.17 (recipe-app) V-01, D1 (a). Files: `src/recipe/add_parameter/` and `src/recipe/rebind/` test modules. Red tests: `add_parameter_with_lf_gives_e_rcp_005` runs `AddParameter` with the value text `A` LF `B` for a `STRING(10)` leaf. It asserts `E-RCP-005` and no stored parameter. `rebind_with_cr_in_a_new_member_gives_e_rcp_005` runs `RebindTypeRevision` with CR in the new member `.label`. It asserts `E-RCP-005` and the old type. HEAD accepts both values. Run after task 4.1. Check: `cargo test -p recipe-app gives_e_rcp_005`.

## 5. Deploy

The pgtools check. The suites test the scripts of the image (`PGTOOLS_BIN=/usr/local/bin`, `deploy/ci/pgtools.sh:12`), not the edited `bin/`. So each run builds the image of the revision first. `deploy/ci/common.sh` (lines 6 to 8) needs three variables, else it exits. The command:

```bash
docker build --build-context db=deploy/db -f deploy/pgtools/Dockerfile -t recipe-pgtools:local deploy/pgtools \
  && GITHUB_ACTIONS=true GITHUB_WORKSPACE=$PWD RUNNER_TEMP=$(mktemp -d) COMPOSE_PROJECT_NAME=recipe-ci-1-1 \
     RECIPE_VERSION=local bash deploy/ci/pgtools.sh
```

Without Docker on the host, the job `compose` ("Compose stack") of a dispatched CI run of the branch is the check. That job does not run on a pull request (ADR-0094). Each suite calls its cases by name, so each new case gets a call in the entry point of its suite.

- [x] 5.1 (deploy/pgtools) [deploy] V-02, D8. Files: `deploy/pgtools/bin/pitr-volume` (`check_recovery_end`), `deploy/pgtools/tests/lib/pitr-volume-check-cases.sh`, `deploy/pgtools/tests/lib/pitr-volume-stubs.sh`, and the entry point `deploy/pgtools/tests/pitr-volume-test`. Implement steps 1 to 9 of D8.
  - Red tests: the case "damaged record in the newest segment" writes a record length after the end offset in the stub segment. It also writes a later non-zero byte. It expects the gap `ERROR` line and an exit code other than 0. HEAD exits with 0. A second red case writes a valid continuation page whose `xlp_rem_len` differs from the claimed length. It expects the same refusal. A third red case writes a crossed page with a valid header, `xlp_rem_len = 0`, and no flag `XLP_FIRST_IS_CONTRECORD`. It expects the gap `ERROR` line.
  - Guard cases, each with its expected result:
    - clean end (`xl_tot_len = 0`): passes with no `WARN` line
    - torn last record with only zero bytes after it: passes with one `WARN` line that names a possible loss of the newest record
    - torn record that crosses a page: the extent adds the page header, and the run passes with one `WARN` line
    - torn record that crosses the end of the segment, with no next segment file: passes with one `WARN` line
    - all-zero continuation page inside the extent: passes with one `WARN` line
    - switched segment after `pg_switch_wal()`: passes
    - end at a segment boundary (offset 0), with no next segment in `/wal`: passes
    - end at a segment boundary, with a next segment that holds records: refuses
  - Check: the pgtools check of this group.
- [x] 5.2 (deploy/ci) [deploy] V-02, D8. Files: `deploy/ci/test_pitr_recovery.py`, `deploy/ci/pitr-recovery.sh` (lines 254 to 256).
  - Red test: change `test_early_end_requirement_matches_the_proof_result` to `proof_class = "R"` and the message `the recovery ended before the end of the WAL archive`. It fails at HEAD, because the script holds `EARLY_END_REFUSAL_REQUIRED=0`.
  - Then set `EARLY_END_REFUSAL_REQUIRED=1` and `EARLY_END_ERROR="the recovery ended before the end of the WAL archive"`, and delete the V-02 comment line.
  - The default stub output is class S (`test_pitr_recovery.py:521`, `EARLY_END_LINE`), and about ten `run_refusals()` calls use it. Case `S:1` then falls to the `*)` arm of `pitr-recovery.sh:504`. So the default stub becomes a class R run with the gap line and a status other than 0.
  - Change `EARLY_END_ERROR_LINE` to the gap message. Keep class S as one explicit test that expects the refusal.
  - Check: `python3 -I -B deploy/ci/test_pitr_recovery.py`.
- [x] 5.3 (.github) [deploy] V-02, D8 (c). Dispatch the workflow `CI` on the branch of this change and read the log of the step `Check a late target, a WAL gap, and an early end (D8 refusals)`. The step must show class R. Step 6 of ADR-0106 (p), the run to the end after `SIGKILL`, must still pass. The task edits no file. Red test: none, because the task is a CI run. Check: the log of the job `pitr-recovery`.
- [x] 5.4 (deploy/pgtools, docs/ops) [deploy] V-06, D10. Files: `deploy/pgtools/bin/restore-test` (`delete_stale_runs`, line 186, and `cleanup`), `deploy/pgtools/tests/lib/restore-stale-cases.sh`, the entry point `deploy/pgtools/tests/restore-test-test`, and `docs/ops/runbook.md` (the step of D10 (h)).
  - Red test: a stale directory with `STUB_PGCTL_STOP_FAIL=1` stays, and the stub log holds no `initdb`. HEAD deletes the directory and runs `initdb`.
  - The test also asserts one cleanup `ERROR` line and the result `failed`.
  - Check: the pgtools check of this group, and ste-lint on `docs/ops/runbook.md`.
- [x] 5.5 (deploy/pgtools) [deploy] V-06, D10. Files: `deploy/pgtools/bin/pitr-check` (`cleanup`, lines 90 to 93), `deploy/pgtools/tests/lib/wal-stubs.sh`, `deploy/pgtools/tests/lib/wal-pitr-cases.sh`, and the entry point `deploy/pgtools/tests/walarchive-test`.
  - Red test: with the new stop-fail switch of `wal-stubs.sh`, the scratch directory stays and the exit code is not 0. HEAD deletes the directory and exits with 0 after `pitr check passed`.
  - The test also asserts one cleanup `ERROR` line after `pitr check passed` (D10 (f)).
  - Check: the pgtools check of this group.
- [x] 5.6 (deploy/pgtools, recipe-server, docs/ops) [deploy] V-07, D11. Files: `deploy/pgtools/bin/backup-health` (lines 64 and 65), `deploy/pgtools/tests/lib/health-cases.sh`, `deploy/pgtools/tests/lib/stubs.sh` (fixed epoch), the entry point `deploy/pgtools/tests/backup-test`, `crates/recipe-server/tests/deploy_values/files/scripts.rs` (`SCRIPT_ASSIGNMENTS`), `crates/recipe-server/tests/deploy_values/file_mutations/scripts.rs` (`SCRIPT_CONSTANTS`), and `docs/ops/runbook.md` (the step of D11 (h)).
  - The new constant of `backup-health` takes its value from `alerts.interval`, so it gets one row in each table (25 and 16 rows).
  - Red test: a manifest 61 s ahead of the fixed epoch fails the healthcheck, and 60 s ahead passes. HEAD passes both.
  - A second red test covers the mixed case of D11 (g): an older complete set 61 s ahead and a healthy newest set. The healthcheck fails. HEAD passes it.
  - The runbook states the known limit of D11 (i). A set with a time ahead of the clock stays untrusted until the clock passes its manifest time minus `alerts.interval`.
  - Check: `cargo test -p recipe-server --test deploy_values`, the pgtools check of this group, and ste-lint on `docs/ops/runbook.md`.
- [x] 5.7 (deploy/db, recipe-db, docs/ops) [deploy] V-01, D1 (e), (e1), (e2). Files: `deploy/db/check-platform-ranges.sql` (new part `line_break` and the second header sentence of D1 (e1)), `crates/recipe-db/tests/storage/platform_ranges.rs`, and `docs/ops/runbook.md` (the paragraph of D1 (e2), next to lines 1185 to 1208).
  - Red test: `line_break_lists_a_stored_lf` stores a `STRING` leaf with LF through SQL and expects one row with the part `line_break`. HEAD has no such part.
  - Check: `cargo test -p recipe-db --test storage platform_ranges`, and ste-lint on `docs/ops/runbook.md`.
- [x] 5.8 (deploy/pgtools, docs/ops) [deploy] V-02, ADR-0221. Files: `deploy/pgtools/bin/pitr-volume` (the helper `wal_tail_state`), `deploy/pgtools/tests/lib/pitr-volume-check-cases.sh`, the entry point `deploy/pgtools/tests/pitr-volume-test`, and `docs/ops/runbook.md` (section "Recover from the WAL archive").
  - Order: after 5.1, and after 5.7 for the runbook. The dispatch of 5.3 runs after this task.
  - At a length of 0, the helper reads the bytes from the record start to the end of the newest file (ADR-0221 (a)). A non-zero byte refuses the end.
  - Red test: a case writes a record length of 0 at the end offset and a later non-zero byte in the newest file. It expects the gap `ERROR` line and an exit code other than 0. HEAD exits with 0.
  - Guard cases: the clean end (`xl_tot_len = 0`, only zero bytes after it) still passes with no `WARN` line. The cases of a short file of ADR-0221 (b) still refuse the end.
  - The runbook names the known limit of ADR-0221 (d) and its next step: a run with `--target-time` before the newest commit.
  - Check: the pgtools check of this group, and ste-lint on `docs/ops/runbook.md`.

## 6. CHANGELOG

- [x] 6.1 (CHANGELOG.md) [mechanical] All behavior changes. Add one line under `[Unreleased]` for V-01, V-02, V-03, V-04, V-05, V-06, V-07, V-18, and V-15 (class R only). The line of V-04 names the leaf path length, the body limit, and the value body permits. Red test: none, because the changelog is a document. Check: ste-lint on `CHANGELOG.md`.

## 7. Final gates

- [x] 7.1 (workspace) Run the gates and show their output. Red test: none, because this task runs checks. Check: each command below passes.
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test --workspace --all-features --locked`
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
  - `cargo deny --locked check`
  - the coverage gate of `CONTRIBUTING.md` (`cargo llvm-cov --workspace` and `tools/` coverage gate)
  - `cargo test -p recipe-server --test deploy_values`
  - `bash deploy/ci/check.sh`
  - `openspec validate --all --strict`
  - `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`

## Order and shared files

The tasks of one file run one after the other, in the order below. All other tasks can run in parallel inside their group, with the crate rule below.

Group order exceptions:

- Task 3.3 runs after task 4.5, because it copies the function text of migration `0019`.
- Task 2.13 runs after task 2.5 and before task 2.6, so the ADR files come in number order. Task 2.11 runs after every other ADR task.
- Task 3.18 runs before task 4.15, and task 3.2 runs before task 4.3, so each §7.11 row comes before its code.

Documents:

- `docs/adr/README.md`: 2.11, then 2.12.
- ADR-0029: 2.4 adds two notes in one edit.
- `docs/architecture/reliability.md`: 3.1, 3.2, 3.7, 3.8, 3.10, 3.13, 3.14, 3.15, 3.16, 3.17, 3.18.
- `docs/architecture/domain-model.md`: 3.1, 3.2, 3.5, 3.13.
- `docs/architecture/api.md`: 3.2, 3.9, 3.11, 3.12, 3.17, 3.18.
- `docs/architecture/overview.md`: 3.2, then 3.6.
- `docs/architecture/data-model.md`: 3.4, then 3.3 (after 4.5).
- `docs/ops/runbook.md`: 3.14, then 5.4, then 5.6, then 5.7, then 5.8.
- `CLAUDE.md` and `.claude/rules/sql.md`: 3.4 only.
- `design.md` of this change: 1.2 only. The `identity-access` delta of this change: 2.10 only.
- `CHANGELOG.md`: 6.1 only.

Crates. A half-done edit of one crate stops the build of that crate and of each crate that depends on it. So the tasks of one crate run one after the other. A parallel agent in another crate uses its own worktree.

- `recipe-domain`: 4.1, 4.3, 4.8, 4.9, 4.16.
- `recipe-app`: 4.3, 4.8, 4.15, 4.17. Task 4.17 runs after 4.1.
- `recipe-web`: 4.2, 4.3, 4.4, 4.15, 4.7, 4.8, 4.10, 4.11, 4.12, 4.13. Task 4.2 runs after 4.1, because both change the expected texts of `E-RCP-005`.
  - `i18n/en.ftl` and `pl.ftl`: 4.2, then 4.3.
  - `src/settings.rs`, `src/limits.rs`, and `src/limits/route_tests.rs`: 4.4, then 4.15. Task 4.15 also edits `src/routes.rs`.
  - `src/pages/parameters/route_tests.rs`: 4.4, then 4.7.
  - `src/csrf/`: 4.4 (`limit_tests.rs`), then 4.10 (`csrf.rs`, `tests.rs`).
  - `src/error/lists.rs`: 4.8 only.
  - `src/pages/xss_coverage_tests/compare.rs` and `kind_tests.rs`: 4.11, then 4.12, then 4.13.
- `recipe-db` and `migrations/`: 4.5, then 4.6, then 4.8, then 5.7. They build one test binary `storage`.
- `recipe-server` tests: 3.2 (`deploy_values/units.rs`), 3.18 (`deploy_values/units.rs`), 4.8 (`error_registry/identity.rs`), 4.15 (`error_registry/app.rs`), and 5.6 (`deploy_values/files/scripts.rs`, `deploy_values/file_mutations/scripts.rs`).
- Tasks 4.3, 4.8, and 4.15 span several crates. Each of them runs while no other task edits one of its crates.

Other files:

- `crates/recipe-web/examples/dev_server.rs`: 1.1 (temporary branch only). Task 4.10 does not edit it.
- `migrations/0019` and `deploy/pgtools/restore-code.expected`: 4.5, then 4.6.
- `deploy/pgtools/tests/lib/stubs.sh`: 5.6 only. `restore-stubs.sh` and `wal-stubs.sh` are separate files.
- The pgtools check builds one image and runs every suite: 5.1, 5.4, 5.5, 5.6, and 5.8 run it one after the other.
- `deploy/pgtools/bin/pitr-volume`, `deploy/pgtools/tests/lib/pitr-volume-check-cases.sh`, and `deploy/pgtools/tests/pitr-volume-test`: 5.1, then 5.8. Task 5.8 runs after 5.7 for the runbook.
- `deploy/ci/test_pitr_recovery.py` and `deploy/ci/pitr-recovery.sh`: 5.2 only. Task 5.3 runs after 5.1, 5.2, and 5.8.

## Workflow follow-up

- The owner decided the open points 1 to 10 of `design.md` on 2026-11-22. The deltas and the tasks follow these decisions.
- Task 3.4 edits `CLAUDE.md` and `.claude/rules/sql.md`. The owner approves that edit with the approval of this file.
- A task that edits `.github/workflows/ci.yml` reads the whole file first. No task of this change edits it. Task 5.3 only dispatches the workflow.
- Archive the change only after task 7.1 passes and the CI run of task 5.3 shows class R.
