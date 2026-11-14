# Tasks

Each task names one finding of `docs/plan/review/2026-11-13-full-repo-review.md` §4 and one decision of `design.md`. A task without a red test states why, and it states the mutation that shows the test can fail.

## Order and shared files

Tasks that edit one file run in sequence. All other tasks can run in parallel.

- 1.1 → 1.2 → 1.3 → 1.4: all four edit `crates/recipe-server/tests/error_registry.rs`, its `#[path] mod` lines, and its scope list.
- 3.1 → 3.2: 3.2 calls the `pager_links` that 3.1 moves into `pages/render.rs`.
- 3.1 → 8.1: both edit `pages/render.rs` and `pages/render/tests.rs`.
- 3.1 → 5.2: both edit `pages/versions/parameter_table.rs`.
- 3.1 and 3.2 → 6.1: 3.1 and 6.1 edit `pages/recipes.rs` and `pages/recipes/list.rs`. 3.2 and 6.1 edit `pages/users.rs`.
- 5.1 → 5.2: 5.1 removes the two `expect(dead_code)` of `error/lists.rs`, and the check of 5.2 scans that file.
- 10.1 runs after the tasks 1 to 9. The tasks of section 11 run last.

## 1. Registry test of the error codes (F-03, design D1)

- [x] 1.1 (recipe-server) Add `tests/error_registry.rs` with the reader of the §1.2 rows and the facts 1 to 3 of D1. Set the scope to `AuditError`, `StorageError`, `ExportError`, and `IdentityError`, and add their modules. Give one sample for each pair of a row and a variant. — red test: none, because the four types agree with §1.2 at HEAD. Map one `IdentityError` variant to the code of another variant once, and see `error_registry_every_sample_code_belongs_to_a_row_that_names_its_variant` and `error_registry_every_pair_of_row_and_variant_has_a_sample` fail. Remove the sample of one trigger code of `StorageError::Constraint` once, and see `error_registry_every_pair_of_row_and_variant_has_a_sample` fail. Restore both. — Check: `cargo test -p recipe-server --test error_registry`
- [x] 1.2 (recipe-server) After 1.1, add `PlantError` to the scope and add its module with an exhaustive `match`. Give one sample for each pair of a row and a variant. — red test: after the change of the scope, `error_registry_every_pair_of_row_and_variant_has_a_sample` fails for each `PlantError` row without a sample. — Check: `cargo test -p recipe-server --test error_registry`
- [x] 1.3 (recipe-server) After 1.2, add `RecipeError` to the scope and add its module in the same way as 1.2. — red test: after the change of the scope, `error_registry_every_pair_of_row_and_variant_has_a_sample` fails for each `RecipeError` row without a sample. — Check: `cargo test -p recipe-server --test error_registry`
- [x] 1.4 (recipe-server) After 1.3, add fact 4 of D1, add `AppError` and `WebError` to the scope, and add their modules. Name each delegate variant of D1 in the `match` without a sample. — red test: `error_registry_scope_holds_every_type_of_an_owner_cell` fails until the scope holds `AppError` and `WebError`. Then `error_registry_every_pair_of_row_and_variant_has_a_sample` fails until each own variant of both types has its samples. — Check: `cargo test -p recipe-server --test error_registry` passes with the full scope of D1

## 2. Random identifiers in recipe-app (F-04, design D2)

- [x] 2.1 (recipe-app) Add `tests/random_ids.rs`. Then apply D2 to the 11 call sites in 7 files. — red test: `no_production_file_of_recipe_app_calls_uuid_new_v4` skips each line that starts with `//`. It lists the 11 call sites and fails at HEAD. The three doc comments of `identity/issue.rs` do not count. — Check: `cargo test -p recipe-app --test random_ids` and `cargo test -p recipe-app`

## 3. One pager function (F-05, design D3)

- [x] 3.1 (recipe-web) Move `pager_links` to `pages/render.rs`. Call it from the machine list, the recipe list, and the usage list. Change the import of the two version tables. — red test: `pager_links_encode_the_search_text_and_omit_the_previous_link_after_the_end` in `pages/render/tests.rs` does not compile before the move. After the move, it asserts `/machines?q=a%26b&page=3` for the next link and an empty previous link after the end. — Check: `cargo test -p recipe-web render` and `cargo test -p recipe-web -- machines recipes types versions`
- [x] 3.2 (recipe-web) After 3.1, make the user list call `pager_links` and `page_url` of `pages/render.rs`. Remove its local `page_url`. Change `previous` and `next` of `UsersPager` to `String`. Change `ListRequest` of `pages/users/query.rs` to hold a `Page` from `path_values::page` in place of its fields `page` and `index` (D3). — red test: `users_list_refuses_a_page_number_that_is_not_canonical` in `pages/users/tests.rs` sends `GET /users?page=01` and `GET /users?page=%2B2`. It asserts `422 E-WEB-008` for each. It fails at HEAD with `200`. The test sends `%2B`, because a `+` in a query string decodes to a space. Add `users_pager_links_keep_q_and_the_page_number` in `pages/users/tests.rs`. The fixture has 201 users with the login `user<n>`. For `GET /users?q=user&page=2` with a full page, it asserts the previous link `/users?q=user&page=1` and the next link `/users?q=user&page=3`. For `GET /users?q=user&page=9` after the end, it asserts the first link `/users?q=user&page=1`. Each assertion reads the `href` text, where askama writes `&` as `&#38;`. Leave out the pair `("q", q)` once and see the test fail. Restore it. The encoding of `q` stays unobservable (D3), so this test has no red state of its own. — Check: `cargo test -p recipe-web users`

## 4. Both inputs of a UDT member row (F-08, design D4)

- [x] 4.1 (recipe-web) Apply D4 in `pages/types/define.rs`. — red test: `a_member_row_with_a_bad_name_and_no_type_gives_two_lines` in `pages/types/define_error_tests.rs` sends the scenario of the `web-ui` delta. It asserts `422 E-WEB-008`, two lines, and an error line under `members-0-name` and `members-0-type`. A second test `a_member_row_with_no_name_and_a_bad_type_gives_two_lines` asserts the lines `MemberRows` and `E-PLT-013`. Both fail at HEAD with one line. — Check: `cargo test -p recipe-web types::define`

## 5. Dead code in recipe-web (F-10, design D5)

- [x] 5.1 (recipe-web) [mechanical] Remove `DraftFault`, `draft_lines`, their two `expect(dead_code)` in `error/lists.rs`, their re-export in `error.rs`, and `error/draft_lines_tests.rs`. — red test: none, because the change removes code that no production path calls. — Check: `rg -n 'draft_lines|DraftFault' crates/recipe-web/src` prints nothing, and `cargo clippy -p recipe-web --all-targets --all-features -- -D warnings` passes
- [x] 5.2 (recipe-web) After 3.1 and 5.1, remove the inner `expect(dead_code)` of `pages/parameters/grid.rs` and the one of `ParameterRowFragment` in `pages/versions/parameter_table.rs`. Treat each dead item that `cargo build -p recipe-web` names by D5. — red test: none, because only dead code changes. The build without the attributes shows the dead items. — Check: `rg -n 'expect\(\s*dead_code' crates/recipe-web/src` prints nothing, clippy as in 5.1 passes, and `cargo test -p recipe-web -- parameters versions` passes

## 6. Shared form module (F-11, design D6)

- [x] 6.1 (recipe-web) [mechanical] After 3.1 and 3.2, move `FieldLines`, `Choice`, and `TokenFields` into `pages/form_fields.rs` and change each import. Move only the `pub(crate)` `Choice` of `pages/recipes.rs` (D6). In `pages/xss_coverage_tests/registry.rs`, change the path of the `TokenFields` entry from `pages/users.rs` to `pages/form_fields.rs`. — red test: `shared_form_items_live_in_the_form_fields_module` in `tests/architecture.rs` fails at HEAD. The three `pub(crate)` definitions are in `pages/recipes.rs`, `pages/recipes/fields.rs`, and `pages/users.rs`. — Check: `cargo test -p recipe-web --test architecture` and `cargo test -p recipe-web`, which runs the XSS registry test

## 7. recipe-server fixes (F-20, F-21, design D7, D8)

- [x] 7.1 (recipe-server) Apply D7 in `listener.rs`. — red test: `a_panic_of_a_connection_task_writes_no_error_line_in_the_loop` in `listener/tests.rs` gets the `JoinError` of a panicked task. Inside `tracing::subscriber::with_default`, it calls `collect` with that error on the test thread. It then writes one control event at `ERROR` in the same scope. It asserts that the control event is the only captured event at `ERROR`. It fails at HEAD. — Check: `cargo test -p recipe-server listener`
- [x] 7.2 (recipe-server) [mechanical] Apply D8 in `create_admin.rs`. — red test: `a_long_input_cut_inside_a_character_is_too_long` in `create_admin/tests.rs` sends `STDIN_LIMIT_BYTES` bytes of `a` and then `é`. It asserts the message "too long". It fails at HEAD with "cannot read". — Check: `cargo test -p recipe-server create_admin`

## 8. Control characters in a site path (F-23, design D9)

- [x] 8.1 (recipe-web) [mechanical] After 3.1, apply D9 in `pages/render.rs`. — red test: `see_other_path_refuses_a_control_character` in `pages/render/tests.rs` asserts `E-WEB-010` for `/\t/evil.com` and for a path with `\u{7f}`. It fails at HEAD for the tab. — Check: `cargo test -p recipe-web render`

## 9. Test quality (F-30 to F-33, design D10)

- [x] 9.1 (recipe-server) Apply the three F-30 items of D10 in `tests/healthcheck.rs`. Apply the twin item of D10 in `src/probe/tests.rs`. Add no unit test, because `defaults_equal_the_settings_table` in `watchdog/tests.rs` already ties `PROBE_BUDGET` to §7.11. — red test: none, because the two tests are flaky and not wrong. Mutation 1: in `probe_local`, pass `Duration::from_secs(2)` in place of `PROBE_BUDGET` once. In `--test healthcheck`, only `a_listener_that_stays_silent_gives_exit_code_1_at_the_budget_of_3_s` fails. Mutation 2: in `answer_once_within`, pass `ACCEPT_WAIT` to `accept_within` in place of `accept_wait` once. In `--test healthcheck`, only `a_fake_server_without_a_connection_ends_at_its_accept_bound` fails. Mutation 3: in `serve_once_within`, pass `ACCEPT_WAIT` to `accept_within` in place of `accept_wait` once. Only `a_fake_server_without_a_connection_ends_at_its_accept_bound` in `src/probe/tests.rs` fails. Restore all three. — Check: `cargo test -p recipe-server --test healthcheck` and `cargo test -p recipe-server probe` and `cargo test -p recipe-server watchdog`
- [x] 9.2 (recipe-app) [mechanical] Apply D10 F-31 in `blocking/tests.rs` to `a_diff_after_its_deadline_does_not_start_the_work`. — red test: none, because the behavior is right. Make `diff` run the closure before it returns `E-WEB-009` once and see the test fail. Restore it. — Check: `cargo test -p recipe-app blocking`
- [x] 9.3 (recipe-domain) [mechanical] Apply D10 F-32 in `recipe/limit/parse/tests.rs` to the two tests `adr_0117_a_request_real_bound_with_7_digits_passes` and `adr_0117_a_request_lreal_bound_with_15_digits_passes`. — red test: none, because the behavior is right. Change one expected bound by one digit once and see the test fail. Restore it. — Check: `cargo test -p recipe-domain limit::parse`
- [x] 9.4 (recipe-domain) [mechanical] Apply D10 F-33 in `export/hash/tests.rs` to `a_changed_limit_gives_another_hash`. — red test: none, because the behavior is right. Leave `limits` out of the hashed body once and see the test fail. Restore it. — Check: `cargo test -p recipe-domain export::hash`

## 10. Changelog

- [x] 10.1 (CHANGELOG.md) [mechanical] Add one line for each changed behavior to `[Unreleased]` under `Fixed` (`reliability.md` §8). The lines name F-04, F-05, F-08, F-20, F-21, and F-23. Change C `review-deploy-fixes` also adds lines to `[Unreleased]`. The second change to merge rebases this file. — red test: none, because the task changes a document. — Check: `python -I tools/ste-lint/ste-lint.py CHANGELOG.md` reports 0 hard violations

## 11. Integration checks

- [x] 11.1 (.) Run the lint gate. — Check: `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 11.2 (.) Run every test. — Check: `cargo test --workspace --all-features --locked`
- [x] 11.3 (.) Check the format and the docs. — Check: `cargo fmt --check` and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features --locked`
- [x] 11.4 (.) Check the dependencies. — Check: `cargo deny --locked check`
- [x] 11.5 (.) Check the coverage gate of 80 % lines per crate. — Check: the three commands of `CONTRIBUTING.md` lines 106 to 108: `coverage-gate.py --write-inventory`, `cargo llvm-cov`, and `coverage-gate.py --report`
- [x] 11.6 (openspec) Run the validator on the change. — Check: `openspec validate review-code-fixes --strict` and `openspec validate --all`
- [x] 11.7 (openspec) Check the STE rules. — Check: `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/review-code-fixes/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'` reports 0 hard violations

## Workflow follow-up

- Review the code with `code-review` (Opus) and Codex. Fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion`.
- Archive the change with `openspec-archive-change`.
- Change B `review-design-fixes` carries F-09, F-22, and the §1.2 and §1.1 text of `E-PLT-020`.
