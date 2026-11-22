# Fix plan closure, 2026-11-22

- Scope: `main` at `316b02e`, the merge of PR #100, and the archive commit `7a78c2e` of PR #101. The findings of sections 3.1, 3.2, and 3.3 of `docs/plan/review/2026-11-21-fix-plan.md`. The item of section 5 is also in scope.
- The findings and their failure scenarios are in `docs/plan/review/2026-11-21-full-repo-review.md` section 4.
- Method: one author read the files of the branch `docs/archive-review-design-fixes-2`. The author ran no `cargo` command. The CI results come from the run numbers in "Evidence sources".
- This file is not normative. It records the closure verdicts.

## Result

The plan has 42 items. These are V-01 to V-36, the w13 note, A-05, A-09, A-10, A-11, and the typo of ADR-0074.

| Verdict | Count | Items |
|---|---|---|
| CLOSED | 39 | Every item except V-17, V-23, and the w13 note |
| KNOWN | 2 | V-23 and the w13 note. ADR-0216 records both |
| NO CHANGE | 1 | V-17. Plan section 5 keeps it as it is |

- V-02, V-07, and the heuristics of the XSS guard are CLOSED with a known limit. "Known limits" names each one.
- Change C moved V-06 and V-07 to change B. Change C closes only the proof of V-02.
- This record does not list each finding of the code reviews of changes A and C, except A-05, A-09, A-10, and A-11. Commit `9d007bc` (PR #95) holds the code-review fixes of change A. Commit `168c1fb` (PR #94) holds those of change C.

## Method

1. For each item, the author read the defect claim of the review and the row of the plan.
2. The author found the fix in a code, test, doc, or CI file of the branch. A checked box of `tasks.md` alone was not evidence.
3. The author checked that the test or the CI job exists under the name that the task gives. A shell `grep` did the text comparison.
4. The author named the test or the CI job that guards the fix. A doc-only fix names its file and "Doc".
5. The author applied the owner decisions of plan section 1 and of the 2026-11-22 code review. The author did not question them.
6. The author read the proof result of V-15 and the open points of `design.md` of change B.

Evidence sources:

| Source | Result |
|---|---|
| PR #95 (change A `review-code-fixes-2`), merge `b693609` | Merged. Archive `openspec/changes/archive/2026-11-21-review-code-fixes-2/` |
| PR #94 (change C `review-deploy-fixes-2`), merge `b8314c8` | Merged. Archive `openspec/changes/archive/2026-11-21-review-deploy-fixes-2/` |
| PR #97 (archive of A and C), merge `005b4f5` | Merged. At `005b4f5`, main CI passed 19 of 19 jobs and the job "Browser regression" passed |
| PR #98 (outcomes of A and C in the plan), merge `d1f6dbe` | Merged |
| PR #100 (change B `review-design-fixes-2`), merge `316b02e` | Merged. 60 of 60 tasks. ADR-0210 to ADR-0221. The commits are `6ca6d38`, `3c63c94`, `e02a476`, `0adbe3d`, `e58f47b`, `877a385`, `c0867e2`, and `ca93bd9` |
| PR #101 (archive of B) | Open when the author wrote this record. Archive `openspec/changes/archive/2026-11-22-review-design-fixes-2/` in the commit `7a78c2e` |
| CI run `45023073404` and Browser regression run `45023073440` on `c0867e2` | Both green |
| CI run `45025862204` and Browser regression run `45025862271` on `316b02e` | CI passed 19 of 19 jobs. Browser regression passed |
| Dispatch runs `44921173499`, `44933685608`, `44935433336` of `ci.yml` (change C) | Each run gave class S for V-02 |
| Dispatch run `45021178213` of `ci.yml` (change B, task 5.3) | The step "Check a late target, a WAL gap, and an early end (D8 refusals)" shows `class=R`. The job `pitr-recovery` passed, also the run to the end after `SIGKILL` |
| Run `45002627693` of "Browser regression" on commit `5d5c418` (draft PR #99) | The V-15 proof. `POST /login` gave `403 E-WEB-007` with `Origin: http://recipes.test:18082`. The author did not read the log. The result comes from `design.md` "V-15 proof result" |
| Runs `45021164294` and `45022465952` of "Browser regression" (task 4.14) | The run on `e58f47b` passed. The run with the mutation commit `877a385` failed both jump-link cases with "no scoped element has a delayed hx-trigger" |
| Run `44941064082` of "Browser regression" (task 7.1 of change A) | With the mutation of draft PR #96, both jump-link cases failed |
| Advisory RUSTSEC-2026-0592 (`url` below 2.5.9) | The job "Dependency policy" failed on 2026-11-22. Commit `c0867e2` holds `url` 2.5.9 and `form_urlencoded` 1.2.3 |
| ADR-0210 to ADR-0221 | Each file exists in `docs/adr/`. The author did not compare each index cell with its Status line |
| Index rows 0066, 0067, 0074, 0095, and 0103 of `docs/adr/README.md` | Each row names ADR-0174 (V-25) |

## Verdicts

Each row holds the verdict of this record. The column "Guard" names the test, the CI job, or "Doc".

### Change A and change C

| Id | Severity | Change | PR | Verdict | Evidence (file:line) | Guard |
|---|---|---|---|---|---|---|
| V-05 | LOW | A and B | #95, #100 | CLOSED | Change A: `crates/recipe-web/src/pages/parameters/edit.rs` keeps the stored value of an unchanged type with no value field and no limit field. Change B: ADR-0213 (i) gives the pair rule | `a_definition_save_without_value_fields_for_the_same_type_keeps_the_value` (`pages/parameters/edit_tests.rs:155`). `blank_limit_row_with_a_value_is_saved` (`pages/parameters/edit_pair_tests.rs:116`) |
| V-09 | LOW | A | #95 | CLOSED | `crates/recipe-app/src/recipe/queries/parameters.rs` ends the transaction before `PlatformTypes::collect` (ADR-0042 (f)) | `each_execute_of_the_parameter_queries_drops_the_transaction_before_the_resolve` (`recipe/queries/source_tests.rs:138`) |
| V-10 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/plant/machine.rs:439` lists the data block rule and then the encoding rule | Doc. Job Documentation |
| V-11 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/plant/type_tree.rs:276` and `:550` to `:553` use `checked_add` | `a_nesting_count_that_overflows_gives_e_plt_018_on_s7` (`plant/type_tree/tests.rs:295`) |
| V-12 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/plant/connection/shape.rs` runs the checks in the order of `domain-model.md` §2.1 | `a_request_with_faults_of_every_kind_lists_all_of_them_in_check_order` (`plant/connection/shape/tests/order.rs:88`) |
| V-13 | LOW | A | #95 | CLOSED | `crates/recipe-db/src/transaction/plc_connection.rs:226` to `:229` and `crates/recipe-app/src/memory/plc_connection.rs` build each stored row through `for_platform` | `a_stored_row_with_a_mode_that_the_platform_refuses_is_corrupt` (`recipe-app/src/memory/plc_connection/tests.rs:566`) |
| V-14 | LOW | A | #95 | CLOSED | `crates/recipe-web/src/settings/cidr.rs` stores the canonical form of an IPv4-mapped entry | `a_mapped_ipv6_entry_matches_the_canonical_ipv4_peer` (`settings/cidr/tests.rs:78`). `a_mapped_entry_below_prefix_96_stays_ipv6` (`:99`) |
| V-19 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/recipe/draft/s7.rs:189` to `:192` name `E-RCP-057`, `E-RCP-056`, and `E-RCP-058`. The docs of `add.rs` and `approval.rs` follow | Doc. Job Documentation |
| V-21 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/audit/security.rs:142` takes `client_ip: IpAddr` | `a_refusal_without_a_client_address_gives_e_db_008_and_writes_no_event` (`recipe-app/src/auth/authorize/tests.rs:54`) |
| V-25 | LOW | A | #95 | CLOSED | The five index rows of `docs/adr/README.md` name ADR-0174 | Doc |
| V-26 | LOW | A | #95 | CLOSED | `crates/recipe-domain/src/recipe/snapshot/mod.rs:18` has the public `distinct_tag_count`. `recipe-web` calls it | `the_distinct_tag_count_counts_each_tag_once_and_skips_version_items` (`recipe/snapshot/tests.rs:597`). `recipe_e_rcp_020_counts_the_parameters_with_a_problem_once_each` (`recipe-web/src/error/recipe_tests.rs:497`) |
| V-29 | LOW | A | #95 | CLOSED | The answer of a failed `POST /login` has no language switch. The spec requirement is "Language switch on full pages" (PR #97 sync) | `a_failed_login_page_has_no_switch_and_keeps_csrf_pre` (`recipe-web/src/pages/page_tests.rs:430`) |
| V-32 | LOW | A | #95 | CLOSED | The watchdog tests of `crates/recipe-server/src/watchdog/tests.rs` end on an event. The window tests inject the instants (`:3`) | The three mutations of task 5.1 of change A. Job Test |
| V-33 | LOW | A | #95 | CLOSED | `crates/recipe-server/src/serve/tests/signal.rs:17` defines `EVENT_WAIT`. `:118` waits with `recv_timeout` | `a_signal_after_the_start_ends_the_wait_of_the_running_server` (`serve/tests/signal.rs:107`) |
| V-34 | LOW | A | #95 | CLOSED | `crates/recipe-web/src/pages/xss_coverage_tests/compare.rs:103` finds a corpus test by its test attribute. `:129` refuses `#[ignore` | `xss_coverage_refuses_a_corpus_fn_without_a_test_attribute` (`xss_coverage_tests/mod.rs:641`) |
| V-35 | LOW | A | #95 | CLOSED | `e2e/tests/leaf-grid.spec.ts` proves "no request" with a sentinel request and no fixed wait | Job Browser regression. The mutation run `44941064082` failed both cases |
| V-36 | LOW | A | #95 | CLOSED | `crates/recipe-server/src/check_results.rs:190` opens the file with `NOFOLLOW` and `NONBLOCK` | `the_opener_refuses_a_link_without_following_it` (`check_results/tests.rs:649`). `the_opener_refuses_a_fifo_without_blocking` (`:664`) |

### Change B

| Id | Severity | Change | PR | Verdict | Evidence (file:line) | Guard |
|---|---|---|---|---|---|---|
| V-01 | HIGH | B | #100 | CLOSED | ADR-0210. A command refuses LF and CR in a text value. A stored value stays readable. `crates/recipe-domain/src/recipe/value/leaf.rs`. The ops query part `line_break` of `deploy/db/check-platform-ranges.sql` lists stored values | `line_break_in_a_string_value_is_refused` (`recipe/value/leaf/tests.rs:133`). `line_break_in_a_char_value_is_refused` (`:140`). `add_parameter_with_lf_gives_e_rcp_005` (`recipe-app/src/recipe/add_parameter/tests/line_break.rs:20`). `line_break_lists_a_stored_lf` (`recipe-db/tests/storage/platform_ranges.rs:680`) |
| V-02 | MEDIUM | C and B | #94, #100 | CLOSED | Change C proved the fault (class S, three runs). ADR-0217 and ADR-0221 add the tail rule. `deploy/pgtools/bin/pitr-volume:548` has `require_zero_tail`. `deploy/ci/pitr-recovery.sh:254` sets `EARLY_END_REFUSAL_REQUIRED=1`. Known limit: ADR-0221 (d) | Cases `case_pitr_check_damaged_record` (`deploy/pgtools/tests/lib/pitr-volume-check-cases.sh:121`) and `case_pitr_check_nonzero_tail_after_zero_length` (`:187`). Dispatch run `45021178213` shows `class=R`. The proof runs of change C show class S |
| V-03 | LOW | B | #100 | CLOSED | ADR-0212. `migrations/0019_struct_member_transaction.sql` compares `xmin` and `created_at` | `member_insert_with_forged_created_at_gives_e_plt_034` (`recipe-db/tests/storage/triggers/immutable_types.rs:190`) |
| V-04 | HIGH | B | #100 | CLOSED | ADR-0211. `reliability.md` §7.11 has `budget.leaf_path_bytes` (128 bytes), `http.body_limit.value` (16 MiB, `:1079`), and `http.value_body_permits` (4). `crates/recipe-web/src/state.rs:83` clamps the permits to `1..=Semaphore::MAX_PERMITS` | `leaf_path_above_128_bytes_gives_e_plt_018` (`recipe-app/src/plant/define_udt/tests.rs:750`). `largest_edit_form_fits_the_value_limit` (`recipe-web/src/pages/parameters/route_tests.rs:620`). `fifth_value_body_waits_and_gets_e_web_009` (`recipe-web/src/limits/route_tests.rs:467`). `stored_type_with_long_leaf_path_resolves` (`recipe-domain/src/plant/type_tree/tests.rs:319`) |
| V-06 | MEDIUM | B | #100 | CLOSED | ADR-0218. `deploy/pgtools/bin/restore-test:28` keeps the scratch directory after a failed stop. `pitr-check` follows | `case_restore_stale_stop_fails` (`deploy/pgtools/tests/lib/restore-stale-cases.sh:54`). `case_restore_current_stop_fails` (`:72`). `case_wal_pitr_stop_fails` (`wal-pitr-cases.sh:288`) |
| V-07 | LOW | B | #100 | CLOSED | ADR-0219. `backup-health` refuses a manifest time ahead of the clock by more than `alerts.interval`. Known limit: a set stays untrusted until the clock passes it (D11 (i)) | `case_health_future_boundary` (`deploy/pgtools/tests/lib/health-cases.sh:56`). A manifest 60 s ahead passes and 61 s ahead fails |
| V-08 | LOW | B | #100 | CLOSED | ADR-0213 (a). `data-model.md` §6 intro, `CLAUDE.md`, and `.claude/rules/sql.md` name the `READ ONLY` pre-read. Each of the two rule files has one `ADR-0213 (a)` | Doc |
| V-15 | LOW | B | #100 | CLOSED | Class R. ADR-0220. `crates/recipe-web/src/csrf.rs:236` accepts `http://` when `session.secure=false` | `csrf_http_origin_passes_when_secure_is_false` (`recipe-web/src/csrf/tests.rs:419`). The proof run `45002627693` |
| V-16 | LOW | B | #100 | CLOSED | ADR-0214. The doc comments of `from_bytes` in `crates/recipe-domain/src/identity/secret.rs:219` and `:294` cite it | Doc |
| V-18 | LOW | B | #100 | CLOSED | ADR-0215. `ThrottlePolicy::parse` (`crates/recipe-domain/src/identity/throttle.rs:128`) has private fields. The row `E-IDN-012` is at `reliability.md:169` | `throttle_policy_parse_refuses_a_bad_field` (`identity/throttle/tests.rs:367`) |
| V-20 | LOW | B | #100 | CLOSED | ADR-0213 (h) amends ADR-0028. No code change | `empty_snapshot_with_foreign_units_gives_no_item` (`recipe-domain/src/recipe/snapshot/tests.rs:282`) |
| V-22 | LOW | B | #100 | CLOSED | ADR-0213 (b). `domain-model.md` §2.4 names four actions | Doc |
| V-24 | LOW | B | #100 | CLOSED | ADR-0213 (c). `overview.md` §5 says "always (ADR-0172 (y))" | Doc |
| V-27 | LOW | B | #100 | CLOSED | ADR-0213 (d). `reliability.md` §8 lists five more places of interior mutability | Doc |
| V-28 | LOW | B | #100 | CLOSED | ADR-0213 (e). `reliability.md` §8 adds "or a banned dependency edge". `CONTRIBUTING.md` has no fault | Doc |
| V-30 | LOW | B | #100 | CLOSED | ADR-0213 (f). `api.md` §4 step 1 says "every form that mutates the version" | Doc |
| V-31 | LOW | B | #100 | CLOSED | ADR-0213 (g). `reliability.md` §7.7 names `pitr-volume --drill --target-time <time>` | Doc |
| A-05 | LOW | B | #100 | CLOSED | `crates/recipe-web/src/pages/xss_coverage_tests/compare.rs`. Each `Parsed` check has its own parse window | `parsed_entry_needs_its_own_parse_window` (`xss_coverage_tests/kind_tests.rs:35`) |
| A-09 | LOW | B | #100 | CLOSED | The same file. A read of the name of an `Unread` field fails | `unread_field_with_a_read_of_its_name_fails` (`kind_tests.rs:255`) |
| A-10 | LOW | B | #100 | CLOSED | The same file. A corpus test must reach its field | `corpus_test_must_name_the_field` (`kind_tests.rs:351`) |
| A-11 | LOW | B | #100 | CLOSED | `e2e/tests/leaf-grid.spec.ts:183` asserts that no scoped element has a delayed `hx-trigger` | Job Browser regression. Run `45021164294` passed. Mutation run `45022465952` failed both cases |
| ADR-0074 typo | Not rated | B | #100 | CLOSED | `grep -rn 'tag file is no set' docs/adr/` finds no line | Doc |

### Known limits and a decision of no change

| Id | Severity | Change | PR | Verdict | Evidence | Guard |
|---|---|---|---|---|---|---|
| V-23 | LOW | B | #100 | KNOWN | ADR-0216 records the lost cookie after `ChangeOwnPassword` and the next step of the user. No code changes | Doc |
| w13 note | LOW | B | #100 | KNOWN | ADR-0216 records the lost event `forbidden` or `csrf_rejected` before its commit. No code changes | Doc |
| V-17 | LOW | None | None | NO CHANGE | Plan section 5. ADR-0166 stays accepted. The wrong line number has no effect | None |

## Code review of change B

Two reviewers read change B. Opus read the Rust part and the database and deploy part. Codex `gpt-6.2-astra` at the effort `high` read the whole change.

| Finding | Severity | Verdict | Detail |
|---|---|---|---|
| Opus B-01, Codex F03 | HIGH | Fixed in `0adbe3d` | `wrap_arrays` in `crates/recipe-app/src/plant/ensure_derived.rs` checks the leaf path only when the call creates a row (ADR-0211 (c)). Test: `stored_type_with_long_leaf_path_resolves` |
| Opus F1 of the database and deploy part, Codex F02 | MEDIUM, HIGH | Fixed. Owner decision 1 | A length of 0 reads the rest of the newest file (ADR-0221 (a)). A short file is refused (ADR-0221 (b)). `0adbe3d` refuses a short file. `e58f47b` adds the zero tail |
| Codex F01 | HIGH | KNOWN. Owner decision 2 | A torn last record into an existing next segment is a known limit (ADR-0221 (d)). The gap check refuses it, and the next step is a run with `--target-time` |
| Codex F04 | HIGH | Fixed. Owner decision 3 | `crates/recipe-web/src/state.rs:83` clamps `http.value_body_permits`. Test: `crates/recipe-web/src/state/tests.rs` |
| Codex F05 | HIGH | No change | The design D5 holds. Opus judged task 4.7 sound |
| Codex F06 | HIGH | Later change | A memory measurement of 4 x 16 MiB value bodies goes to a separate change |
| Codex F07 to F14 | MEDIUM, HIGH | Fixed | F07 by task 4.14 and the commits up to `e58f47b`. F08 to F14 by `0adbe3d` and `e58f47b`. The fixes cover the XSS guard, the continuation flag case, the trigger test, the pair test, and the `line_break` query test |
| Opus B-02 | LOW | Not taken | `preview_tree` skips the leaf path check |
| Opus B-04 | LOW | Not taken | No test pins the order of the permit layer and the body deadline |
| Opus F4 of the database and deploy part | LOW | Not taken | The tail check reads the file of the base timeline after a later timeline |
| The upper-case `HTTP://` case and two LOW CSRF tests | LOW | Not taken | The finding ids are not recorded |
| Opus B-03, Codex F15, Opus F2, Opus F3 of the database and deploy part | LOW | Fixed in `e58f47b` (PR #100) | `docs/architecture/reliability.md` (row `E-IDN-012`: no route returns the code, and no setting builds the policy yet). `crates/recipe-web/i18n/en.ftl` and `pl.ftl` line 24 (one sentence with a period). `CHANGELOG.md` (the V-18 line and the tail rule lines of V-02) |

Owner decision 4 of 2026-11-22: push after the fixes.

## Known limits

- ADR-0221 (d): a torn last record into an existing next segment gives a refusal that looks like a gap.
- ADR-0217 (a1a): an extent that ends inside the last written page stays the one remaining risk of the tail rule.
- ADR-0216: the lost cookie after `ChangeOwnPassword` and the lost event before its commit.
- D11 (i) of change B: a set with a manifest time ahead of the clock stays untrusted. It stays so until the clock passes that time minus `alerts.interval`.
- XSS coverage heuristics: a child module in another file, a route binding, and an inferred type.
- ADR-0212: a future `SAVEPOINT` breaks the `xmin` test. A test inserts a struct and its members through the real adapter.

## Notes

- Task 5.1 of change B says `xlp_rem_len = 0`. The fix F11 uses 48 for that record. The archive keeps the task text as history.
- The V-15 proof used a temporary branch. The author closed draft PR #99 and deleted its branch (`design.md` "V-15 proof result").

## Open items

- PR #101 (archive of B) is open. It adds the archive of `review-design-fixes-2` and the spec sync.
- Codex F06: the memory measurement of 4 x 16 MiB value bodies. A separate change owns it.
- `range_under`: a separate issue owns it.
- LOW findings that the owner did not take: Opus B-02, Opus B-04, Opus F4, the upper-case `HTTP://` case, and two LOW CSRF tests.
