# Tasks

Each task names one finding of `docs/plan/review/2026-11-21-full-repo-review.md` §4 and one decision of `design.md`. A task without a red test states why, and it states the mutation that shows the test can fail.

## Order and shared files

Tasks that edit one file run in sequence. All other tasks can run in parallel.

- 2.2 → 2.3: both edit `crates/recipe-domain/src/plant/connection/shape.rs`. 2.2 also changes `shape/tests.rs`, and 2.3 adds no test there.
- 6.1 → 6.2: both edit `crates/recipe-web/src/pages/xss_coverage_tests/compare.rs`, `registry.rs`, and `scan.rs`.
- 2.6 → 3.2: 2.6 adds the public function that 3.2 calls in `recipe-web`.
- 1.2 runs alone and first. It changes the public signature of `SecurityEvent::forbidden` in `recipe-domain`, and four crates use it. The other tasks start after 1.2 passes its Check.
- 1.2 edits test files of four crates: `recipe-domain/src/audit/security/tests.rs`, five test files of `recipe-app`, two files of `recipe-db/tests/storage/`, and `recipe-web/src/pages/approval_sweep_tests.rs` and `approval_xss_tests.rs`. No other task edits these files.
- No other task pair edits one file. The task of V-19 (2.5) is alone in `recipe/draft/approval.rs`, because V-20 left this change.
- 9.1 runs after the tasks 1 to 8. The tasks of section 10 run last.

## 1. recipe-app (V-09, V-21)

- [x] 1.1 (recipe-app) Apply D2 in `src/recipe/queries/parameters.rs`. — red test: `each_execute_of_the_parameter_queries_drops_the_transaction_before_the_resolve` in `src/recipe/queries/source_tests.rs` finds each `execute` body of `parameters.rs` that calls `PlatformTypes::collect(`. The fourth `execute` has no such call, and the test skips it. The test asserts that `drop(tx)` comes before `PlatformTypes::collect(` in each found body. It fails at HEAD for `ListParameters`. — Check: `cargo test -p recipe-app queries`
- [x] 1.2 (recipe-domain, recipe-app) Apply D8 in `recipe-domain/src/audit/security.rs` and `recipe-app/src/auth/authorize.rs`. Change the 11 test callers of `SecurityEvent::forbidden` in `recipe-domain`, `recipe-app`, `recipe-db/tests`, and `recipe-web` to a plain `IpAddr`. — red test: `a_refusal_without_a_client_address_gives_e_db_008_and_writes_no_event` in `recipe-app/src/auth/authorize/tests.rs` builds a `RequestMeta` without an address and a denied permission. It asserts `E-DB-008` and an empty memory sink. It fails at HEAD, because `refuse` gives the event to the sink. — Check: `cargo test -p recipe-domain audit::security` and `cargo test -p recipe-app auth`

## 2. recipe-domain (V-10, V-11, V-12, V-13, V-19, V-26)

- [x] 2.1 (recipe-domain) [mechanical] Apply D3 V-10 to the doc comment of `TryFrom<StoredMachine>` in `src/plant/machine.rs`. — red test: none, because the task changes a doc comment. — Check: `rg -n 'The checks run in this order: the data block rule' crates/recipe-domain/src/plant/machine.rs` prints one line, and `RUSTDOCFLAGS='-D warnings' cargo doc -p recipe-domain --no-deps`
- [x] 2.2 (recipe-domain) Apply D5 in `src/plant/connection/shape.rs`. — red test: change the expected list of `a_request_with_faults_of_every_kind_lists_all_of_them_in_check_order` in `src/plant/connection/shape/tests.rs` to `PLT_022 rack`, `PLT_023 host`, `PLT_023 login`, `PLT_025 comm_mode`, `PLT_026 address`. Add `a_missing_login_of_the_mode_comes_before_the_mode_fault`: `TwinCat3` with the mode `CodesysV3` and no login and no password gives `[PLT_023 login, PLT_023 password, PLT_025 comm_mode]`. Both fail at HEAD with `E-PLT-023` after `E-PLT-025`. — Check: `cargo test -p recipe-domain plant::connection` and `cargo test -p recipe-web connection`
- [x] 2.3 (recipe-db, recipe-app) After 2.2, apply D6 in `recipe-db/src/transaction/plc_connection.rs` and `recipe-app/src/memory/plc_connection.rs`. Name `for_platform` as the constructor of a stored row in the doc comment of `PlcConnection` in `recipe-domain/src/plant/connection/shape.rs`. — red test: `a_stored_row_with_a_mode_that_the_platform_refuses_is_corrupt` in a new `#[cfg(test)]` module of `plc_connection.rs` of `recipe-db` builds a `ConnectionRecord` of `TwinCat3` with `CodesysV3`. It calls `build_connection` and asserts `E-DB-008`. The memory adapter gets the same test for `ConnectionRow::stored`. Both fail at HEAD, because the builders accept the row. — Check: `cargo test -p recipe-db --lib plc_connection`, `cargo test -p recipe-db --test storage plc_connection` (needs Docker), and `cargo test -p recipe-app memory::plc_connection`
- [x] 2.4 (recipe-domain) [mechanical] Apply D4 in `src/plant/type_tree.rs`. — red test: none, because HEAD and the fix agree on each input. Add `a_nesting_count_that_overflows_gives_e_plt_018_on_s7` and `a_nesting_count_that_overflows_passes_on_codesys` in `src/plant/type_tree/tests.rs`. Each builds a tree with `s7_nesting: u32::MAX` over a struct and an array expression. Return `Ok(())` on `None` once and see the S7 test fail. Restore it. — Check: `rg -n 'saturating_' crates/recipe-domain/src/plant --glob '!*tests*'` prints nothing, and `cargo test -p recipe-domain type_tree`
- [x] 2.5 (recipe-domain) [mechanical] Apply D3 V-19 to the `# Errors` docs of `Draft::add_parameter` in `src/recipe/draft/add.rs`, `check_new_definition` in `src/recipe/draft/add/step_definition.rs`, `check_for_approval` in `src/recipe/draft/approval.rs`, and `check_definition` in `src/recipe/draft/s7.rs`. — red test: none, because the task changes doc comments. — Check: read the `# Errors` block of each of the four functions. Each block names the codes of D3 V-19 in that order. A count of `E-RCP-058` lines proves nothing, because `s7.rs` already has two (lines 120 and 239). Then run `RUSTDOCFLAGS='-D warnings' cargo doc -p recipe-domain --no-deps`
- [x] 2.6 (recipe-domain) Apply the domain part of D9 in `src/recipe/snapshot/mod.rs` and `src/recipe/error.rs`. The private helper `distinct_tags` of `error.rs` (line 451) becomes the public `distinct_tag_count` in `snapshot/mod.rs`. — red test: `the_distinct_tag_count_counts_each_tag_once_and_skips_version_items` in `src/recipe/snapshot/tests.rs` calls `distinct_tag_count` with two items of one tag, one `UnknownUnit` item, and one version item. It asserts 2. It does not compile at HEAD, because `distinct_tag_count` does not exist. — Check: `cargo test -p recipe-domain snapshot`

## 3. recipe-web, recipe forms (V-05, V-26)

- [x] 3.1 (recipe-web) Apply D1 in `src/pages/parameters/edit.rs`. — red test: `a_definition_save_without_value_fields_for_the_same_type_keeps_the_value` in `src/pages/parameters/edit_tests.rs` sends an HTMX `POST .../parameters/{key}/definition` of `SPEED` with a new `name_en`, `type` and `value_type` equal to the stored type, and no value or limit field. It asserts `200`, the new name, and the unchanged value and limits. It fails at HEAD with `422 E-RCP-005`. — Check: `cargo test -p recipe-web parameters`
- [x] 3.2 (recipe-web) [mechanical] After 2.6, apply the web part of D9 in `src/error/recipe.rs`. The task deletes `distinct_tags` and calls `distinct_tag_count`, so `[mechanical]` fits. — red test: none, because the count does not change. The existing test `recipe_e_rcp_020_counts_the_parameters_with_a_problem_once_each` in `src/error/recipe_tests.rs` (line 497) asserts "2 parameters". Make `distinct_tag_count` of `recipe-domain` return the item count once and see that test fail. Restore it. — Check: `rg -n 'fn distinct_tags' crates/recipe-web/src` prints nothing, and `cargo test -p recipe-web error`

## 4. recipe-web, identity pages (V-14, V-29)

- [x] 4.1 (recipe-web) Apply D7 in `src/settings/cidr.rs`. — red test: `a_mapped_ipv6_entry_matches_the_canonical_ipv4_peer` in `src/settings/cidr/tests.rs` parses `::ffff:172.28.0.2/128` and asserts that it contains `172.28.0.2`. `a_mapped_entry_below_prefix_96_stays_ipv6` asserts that `::ffff:0:0/80` contains no IPv4 address. `a_loopback_ipv6_entry_stays_ipv6` asserts that `::1/128` does not contain `0.0.0.1`. The first test fails at HEAD. Add the scenario of the `identity-access` delta as `pipeline_trusted_proxy_entry_in_mapped_form_forwards_the_client` in `src/pipeline/tests.rs` with the helper `client_from`. — Check: `cargo test -p recipe-web cidr` and `cargo test -p recipe-web pipeline`
- [x] 4.2 (recipe-web) Apply D10 in `src/pages/own/templates.rs` and `src/layout.rs`. — red test: change `page_switch_after_a_failed_login_keeps_the_nonce_of_the_new_cookie` in `src/pages/page_tests.rs` to `a_failed_login_page_has_no_switch_and_keeps_csrf_pre`. It asserts no form with `action="/lang"` and one field `csrf_pre` in the login form with the nonce of the new cookie. It fails at HEAD, because the page holds the switch. Change the spec name "Language switch" in the comment of line 212 of `page_tests.rs` to "Language switch on full pages". — Check: `cargo test -p recipe-web page_tests` and `cargo test -p recipe-web layout`

## 5. recipe-server (V-32, V-33, V-36)

- [x] 5.1 (recipe-server) Apply D11 V-32 to `a_watchdog_that_is_not_armed_never_probes`, `disarm_stops_the_probes`, `arm_after_disarm_probes_again`, and `a_dropped_handle_ends_the_thread` in `src/watchdog/tests.rs`. — red test: none, because the tests are flaky and not wrong. Run three mutations, one at a time, and restore each. First, make the `Disarm` arm of `settle_commands` (`src/watchdog.rs` line 271) keep `*armed = true` and see `disarm_stops_the_probes` fail. Second, make both `Disconnected` arms continue the loop: the arm in `settle_commands` and the arm of `watch` at line 222. See `a_dropped_handle_ends_the_thread` fail. Third, make the `Arm` arms of `watch` and `settle_commands` keep `armed` false and see `arm_after_disarm_probes_again` fail. — Check: `rg -n 'thread::sleep' crates/recipe-server/src/watchdog/tests.rs` prints only the line of `wait_for`, and `cargo test -p recipe-server watchdog`
- [x] 5.2 (recipe-server) [mechanical] Apply D11 V-33 to `a_signal_after_the_start_ends_the_wait_of_the_running_server` in `src/serve/tests/signal.rs`. Define `const EVENT_WAIT: Duration` in that file with the value of `src/watchdog/tests.rs` (10 s). If `Server` is not `Send`, stop and report it. — red test: none, because the behavior is right. Make `race_stop` skip the arm of the signal once and see the test fail within `EVENT_WAIT` with a message. Restore it. — Check: `cargo test -p recipe-server serve::tests::signal`
- [x] 5.3 (recipe-server) Apply D13 in `src/check_results.rs`. Change the comment of `rustix` in `Cargo.toml` to name `statvfs` and the open of a result file. — red test: `the_opener_refuses_a_link_without_following_it` and `the_opener_refuses_a_fifo_without_blocking` in `src/check_results/tests.rs`, both `#[cfg(unix)]`. Define `const EVENT_WAIT: Duration` (10 s) in that file. The tests call the new opener on a link to a valid file and on a FIFO. `rustix::fs::mknodat` with `FileType::Fifo` makes the FIFO. A helper thread runs each open and sends the result on a channel. The test waits with `recv_timeout(EVENT_WAIT)` and asserts `Reading::Rejected`, so a blocked open fails and does not hang. They do not compile at HEAD, because the opener does not exist. Drop `NOFOLLOW` once and see the link test fail. Restore it. — Check: `cargo test -p recipe-server check_results` and `cargo deny --locked check`

## 6. recipe-web, XSS guard (V-34)

- [x] 6.1 (recipe-web) Apply the corpus part of D12 in `src/pages/xss_coverage_tests/compare.rs`. — red test: `xss_coverage_refuses_a_corpus_fn_without_a_test_attribute` and `xss_coverage_refuses_a_corpus_fn_under_a_cfg_that_is_not_test` in `src/pages/xss_coverage_tests/mod.rs` feed a sample source to `coverage_faults`. Each asserts one fault. Both fail at HEAD, because the text `fn name(` passes. — Check: `cargo test -p recipe-web xss_coverage`
- [x] 6.2 (recipe-web) After 6.1, apply the `NoText` rule of D12 in `compare.rs`, `registry.rs`, and `scan.rs`. Give each current `NoText` entry the kind of the D12 list. — red test, all in `mod.rs`: `xss_coverage_accepts_the_parsed_kind_on_the_list_query_page` feeds the real `pages/users/query.rs` with the entry `ListQuery.page` and `Parsed("path_values::page")`. It asserts no fault. This `String` field must not give a fault. `xss_coverage_refuses_the_typed_kind_on_a_string_field` gives a sample `String` field the kind `Typed` and asserts one fault. `xss_coverage_refuses_a_parsed_kind_that_the_module_does_not_call_for_the_field` gives a sample field a path that the sample calls only for another field. It asserts one fault. `xss_coverage_refuses_the_unread_kind_on_a_named_field` gives `Unread` to a field without `_` and asserts one fault. They do not compile at HEAD, because `Cover::NoText` holds only a text. — Check: `cargo test -p recipe-web xss_coverage`

## 7. e2e (V-35)

- [ ] 7.1 (e2e) Apply D11 V-35 in `e2e/tests/leaf-grid.spec.ts`. — red test: none, because the behavior is right. Add `hx-get="/recipes"` to the jump link in its template once. The template is part of the server image, so rebuild the dev image with the `up -d --build` command of `CLAUDE.md` "Commands". Then see both cases fail. Restore the template and rebuild again. — Check: `cd e2e && npm ci && E2E_PASSWORD=<12 to 128 characters> npx playwright test tests/leaf-grid.spec.ts` on the dev stack

## 8. Documents (V-25)

- [x] 8.1 (docs/adr) [mechanical] Apply D14 V-25 to the rows 0066, 0067, 0074, 0095, and 0103 of `docs/adr/README.md`. — red test: none, because the task changes a document. — Check: `for n in 0066 0067 0074 0095 0103; do grep "^| \[$n\]" docs/adr/README.md | grep -q 'ADR-0174' || echo "$n"; done` prints nothing

## 9. Changelog

- [x] 9.1 (CHANGELOG.md) [mechanical] Add one line for each changed behavior to `[Unreleased]` under `Fixed` (`reliability.md` §8). The lines name V-05, V-09, V-12, V-13, V-14, V-21, V-29, and V-36. Change C `review-deploy-fixes-2` also adds lines to `[Unreleased]`. The second change to merge rebases this file. — red test: none, because the task changes a document. — Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` reports 0 hard violations

## 10. Integration checks

- [x] 10.1 (.) Run the lint gate. — Check: `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 10.2 (.) Run every test. — Check: `cargo test --workspace --all-features --locked`
- [x] 10.3 (.) Check the format and the docs. — Check: `cargo fmt --check` and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features --locked`
- [x] 10.4 (.) Check the dependencies. — Check: `cargo deny --locked check`
- [x] 10.5 (.) Check the coverage gate of 80 % lines per crate. — Check: the three commands of `CONTRIBUTING.md` "Local checks": `coverage-gate.py --write-inventory`, `cargo llvm-cov`, and `coverage-gate.py --report`
- [x] 10.6 (openspec) Run the validator on the change. — Check: `openspec validate review-code-fixes-2 --strict` and `openspec validate --all`
- [x] 10.7 (openspec) Check the STE rules. — Check: `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/review-code-fixes-2/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'` reports 0 hard violations

## Workflow follow-up

- Review the code with `code-review` (Opus) and Codex. Fix each CRITICAL, HIGH, and MEDIUM finding.
- Run `superpowers:verification-before-completion`.
- Archive the change with `openspec-archive-change`.
- Change B `review-design-fixes-2` carries V-18, V-20, and V-28.
