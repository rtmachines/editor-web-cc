# Tasks

Each task names its finding, its decision of `design.md`, its files, its implementer, its red test, and its Check command. The tasks of one section touch disjoint files. One agent does one task, and the agents of one section run in parallel. A section starts after the section before it ends.

The ste-lint Check of a doc task is `python -I tools/ste-lint/ste-lint.py <files of the task>`. It must report 0 hard violations.

## 1. Group G1: ADRs

- [x] 1.1 (docs/adr) [opus] F-01, D1. Write `docs/adr/0163-success-answer-after-a-failed-read.md`. Add the "Amended by ADR-0163" note to the Status line of `0041-error-lists-of-a-form.md`. Red test: none. Check: ste-lint on `docs/adr/0163-*.md docs/adr/0041-*.md`.
- [x] 1.2 (docs/adr) [opus] F-02, D2. Write `docs/adr/0164-second-pick-of-the-approved-version.md`. Add the note to the Status lines of `0019-export-integrity-and-contract.md` and `0109-export-api-for-transfer-clients.md`. Red test: none. Check: ste-lint on `docs/adr/0164-*.md docs/adr/0019-*.md docs/adr/0109-*.md`.
- [x] 1.3 (docs/adr) [opus] F-12 and the F-03 note, D3. Write `docs/adr/0165-code-of-every-error-type.md` with `E-WEB-016` and `E-WEB-017`. Add the note to the Status lines of ADR-0027, ADR-0151, and ADR-0154. Red test: none. Check: ste-lint on `docs/adr/0165-*.md docs/adr/0027-*.md docs/adr/0151-*.md docs/adr/0154-*.md`.
- [x] 1.4 (docs/adr) [opus] F-13, D4. Write `docs/adr/0166-constructor-for-stored-data.md` with the stored form, the input form, and the skipped checks of `RuleSet::Earlier`. Add the note to the Status lines of ADR-0120 and ADR-0143. Red test: none. Check: ste-lint on `docs/adr/0166-*.md docs/adr/0120-*.md docs/adr/0143-*.md`.
- [x] 1.5 (docs/adr) [opus] F-16, D7. Write `docs/adr/0167-search-path-of-the-audit-guard.md`. Add the note to the Status line of ADR-0098. Red test: none. Check: ste-lint on `docs/adr/0167-*.md docs/adr/0098-*.md`.
- [x] 1.6 (docs/adr) [opus] F-15, F-17, F-18, D6. Write `docs/adr/0168-data-model-text-follows-the-schema.md`. Red test: none. Check: ste-lint on `docs/adr/0168-*.md`.
- [x] 1.7 (docs/adr) [opus] F-19, D8. Write `docs/adr/0169-new-version-row-is-a-draft.md`. Name the `FUNCTION` line and the `TRIGGER` line of `restore-code.expected` (ADR-0076). Red test: none. Check: ste-lint on `docs/adr/0169-*.md`.
- [x] 1.8 (docs/adr) [opus] F-25, D9. Write `docs/adr/0170-secret-mounts-of-the-db-service.md`. Red test: none. Check: ste-lint on `docs/adr/0170-*.md`.
- [x] 1.9 (docs/adr) [opus] F-09, D10. Write `docs/adr/0171-cookies-without-secure-in-a-dev-build.md`. Add the note to the Status line of ADR-0062. Red test: none. Check: ste-lint on `docs/adr/0171-*.md docs/adr/0062-*.md`.
- [x] 1.10 (docs/adr) [opus] F-06, F-07, F-22, D11 to D13. Write `docs/adr/0172-dropped-future-after-a-committed-step.md` with the owner decisions of 2026-11-14. Add the note to the Status lines of ADR-0125, ADR-0130, and ADR-0133. Red test: none. Check: ste-lint on `docs/adr/0172-*.md docs/adr/0125-*.md docs/adr/0130-*.md docs/adr/0133-*.md`.
- [x] 1.11 (docs/adr) [mechanical] [haiku] F-14, D5. Add "Amended by ADR-0046" to the Status line of `0013-rbac-fixed-permissions.md`. Red test: none. Check: ste-lint on `docs/adr/0013-*.md`.
- [x] 1.12 (docs/adr) [mechanical] [haiku] F-24, D5. Add the NUL note to the Status line of `0031-migrator-configuration-and-secret-files.md`. Red test: none. Check: ste-lint on `docs/adr/0031-*.md`.

## 2. Group G2: index, F-01 text, F-15 text

- [x] 2.1 (docs/adr) [mechanical] [haiku] All findings, D14. Add the rows 0163 to 0172 to the index of `docs/adr/README.md`. Copy the new Status lines of the 15 ADRs of tasks 1.1 to 1.12 into their index cells. Update the decision map. Red test: none. Check: ste-lint on `docs/adr/README.md`.
- [x] 2.2 (docs/architecture) [opus] F-01, D1. Write the notice answer with the empty `#error-panel` in `api.md` §4 and §5. Add the new log place to `reliability.md` §1.1, `CLAUDE.md`, and `.claude/rules/rust.md`. Red test: none. Check: ste-lint on `docs/architecture/api.md docs/architecture/reliability.md`.
- [x] 2.3 (docs/architecture) [mechanical] [haiku] F-15, D6. Add the read before the transaction to the rows `CreateUser`, `ResetPassword`, and `CreateRecipe` of `data-model.md` §6. Red test: none. Check: ste-lint on `docs/architecture/data-model.md`.

## 3. Group G3: F-02 text, F-12 text

- [x] 3.1 (docs/architecture) [opus] F-02, D2. Write the second pick, the second check under the lock, and the `404` without a third pick in `api.md` §2.7 and the row `ExportVersion` of `data-model.md` §6. Red test: none. Check: ste-lint on `docs/architecture/api.md docs/architecture/data-model.md`.
- [x] 3.2 (docs/architecture) [opus] F-12 and the F-03 note, D3. Change `reliability.md` §1.1 for `PlantErrors`, `CheckResultFault`, and `SnapshotItem`. Add the rows `E-WEB-016` and `E-WEB-017` to §1.2. Change the row `E-PLT-020`. Red test: none. Check: ste-lint on `docs/architecture/reliability.md`.

## 4. Group G4: F-19 text, F-06 text

- [x] 4.1 (docs/architecture) [opus] F-19, D8. Add the row `E-RCP-037` to `reliability.md` §1.2 first. Add `data-model.md` §4.14, its row of §4.11, the sentence of §5.2, and migration 0016 to §8. Extend I4 in `domain-model.md` §3. Red test: none. Check: ste-lint on `docs/architecture/reliability.md docs/architecture/data-model.md docs/architecture/domain-model.md`.
- [x] 4.2 (docs/architecture) [mechanical] [haiku] F-06, D11. Add the create form to the `hx-sync` list of `api.md` §4 and `.claude/rules/web.md`. Add the step of D11 (a) to `docs/ops/runbook.md`. Red test: none. Check: ste-lint on `docs/architecture/api.md docs/ops/runbook.md`.

## 5. Group G5: F-09 text, F-16 text

- [x] 5.1 (docs/architecture) [opus] F-09, D10. Write the cookie names and attributes for `session.secure=false` in `api.md` §7.1, §7.3, and §8. Change the rows `session.cookie_name` and `session.secure` of `reliability.md` §7.11. Red test: none. Check: ste-lint on `docs/architecture/api.md docs/architecture/reliability.md`.
- [x] 5.2 (docs/architecture) [mechanical] [haiku] F-16, D7. Add the `search_path` exception of the audit guard function to `data-model.md` §1.1 and `.claude/rules/sql.md`. Red test: none. Check: ste-lint on `docs/architecture/data-model.md`.

## 6. Group G6: F-13 text, F-17 text

- [x] 6.1 (docs/architecture) [mechanical] [haiku] F-13, D4. Copy the `from_stored` rule of ADR-0166 into `reliability.md` §4.3, `CLAUDE.md`, and `.claude/rules/rust.md`. Name the stored form and the input form apart. `VersionState::from_stored` takes no rule set and is out of the scope of ADR-0166 (aa). Red test: none. Check: ste-lint on `docs/architecture/reliability.md`.
- [x] 6.2 (docs/architecture) [mechanical] [haiku] F-17, D6. Add the rows `plc_type_member` and `draft_editor` to the exception table of `data-model.md` §1.1. Red test: none. Check: ste-lint on `docs/architecture/data-model.md`.

## 7. Group G7: F-07 text, F-18 text

- [x] 7.1 (docs/architecture) [opus] F-07, D12. Write the detached tasks in `reliability.md`. In §1.1, add the `WARN` line of a lost event, and remove the handler line of ADR-0130. Change §2.2 (row `JoinHandle`) and §2.4 rules 6 and 7. Change §7.3 steps 3 to 5, and add the row `identity.detached_tasks` with the value 8 to §7.11. Add the new `TaskStage` and `TimeoutStage`. Name the log place and the catch place in `CLAUDE.md` and `.claude/rules/rust.md`. Change the commit paths of the four use cases in `domain-model.md` §5. Red test: none. Check: ste-lint on `docs/architecture/reliability.md docs/architecture/domain-model.md`.
- [x] 7.2 (docs/architecture) [mechanical] [haiku] F-18, D6. State in `data-model.md` §1.3 that `TypedLogin` refuses a control character and the database checks the length only. Red test: none. Check: ste-lint on `docs/architecture/data-model.md`.

## 8. Group G8: code foundations

- [x] 8.1 (recipe-app) [sonnet] F-01, D1. Add `MemoryStore::fail_begin_after_commit(fault)` in `src/memory/core/store.rs`. Red test: `fail_begin_after_commit_fails_the_first_begin_after_a_commit` does not compile, because the method is absent. Check: `cargo test -p recipe-app fail_begin_after_commit`.
- [x] 8.2 (recipe-web) [sonnet] F-01, D1. Add the shared notice answer in `src/pages/saved_notice.rs` and its text in `i18n/en.ftl` and `i18n/pl.ftl`. Red test: `saved_notice_htmx_answer_targets_toast_and_empties_error_panel` does not compile, because the module is absent. Check: `cargo test -p recipe-web saved_notice`.
- [x] 8.3 (recipe-domain) [sonnet] F-12, D3. Add `PlantErrors::code()` in `src/plant/errors.rs`. Red test: `plant_errors_code_is_code_of_first_entry` does not compile, because `code()` is absent. Check: `cargo test -p recipe-domain plant_errors_code`.
- [x] 8.4 (recipe-web) [sonnet] F-09, D10. Add `secure` to `WebSettings` in `src/settings.rs` with the default `true`. Make the writers in `src/cookies.rs` (also `CLEAR_SESSION_FALLBACK` and `lang_cookie`) and `src/csrf/pre_session.rs` use it. Make the readers in `src/session.rs`, `src/pages/request.rs`, and `src/csrf.rs` use the same names. Red tests: `login_then_get_with_recipe_session_without_secure` and `get_login_then_post_with_recipe_csrf_without_secure` fail, because the names stay `__Host-`. Add the same round trips for `secure = true`. Check: `cargo test -p recipe-web cookie`.
- [x] 8.5 (recipe-server) [mechanical] [haiku] F-22, D13. Add the known limit of ADR-0172 to the doc comment of `read_with_limit` in `src/limited_read.rs`. Red test: none, because no code changes. Check: `cargo doc -p recipe-server --no-deps`.
- [x] 8.6 (recipe-server) [sonnet] F-24, D5. Make `secret_line` in `src/config/mod.rs` refuse a NUL byte. Red test: `secret_line_refuses_a_nul_byte` fails, because the function accepts the byte. Check: `cargo test -p recipe-server secret_line`.
- [x] 8.7 (recipe-db) [sonnet] F-19, D8. Write `migrations/0016_recipe_version_insert.sql` with the trigger `recipe_version_insert_draft`. Add the positive and the negative trigger tests. Change the eight tests that D8 names in `tests/storage/constraints/recipe.rs`. Add the `EXECUTE` line to `tests/grants.expected`. Red test: `recipe_version_insert_of_approved_row_is_rejected` fails, because the database accepts the row. Check: `cargo test -p recipe-db recipe_version`.

## 9. Group G9: F-01 handlers, F-02, F-19 code, F-09 wiring

Each F-01 test asserts `HX-Trigger` with `rowVersion` when the output holds the value, and no `HX-Trigger` otherwise (D1).

- [x] 9.1 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `src/pages/parameters/remove.rs`. Red test in `remove_tests.rs`: `remove_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is `503`. Check: `cargo test -p recipe-web pages::parameters::remove_tests`.
- [x] 9.2 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `src/pages/parameters/edit.rs`. Red test in `edit_tests.rs`: `save_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is an error. Check: `cargo test -p recipe-web pages::parameters::edit_tests`.
- [x] 9.3 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `set_value` and `replace_limits` of `src/pages/parameters/value.rs`. Red tests in `value_tests.rs`: `set_value_with_failed_read_after_commit_shows_saved_notice` and `replace_limits_with_failed_read_after_commit_shows_saved_notice` fail, because each answer is an error. Check: `cargo test -p recipe-web pages::parameters::value_tests`.
- [x] 9.4 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `src/pages/versions/rebind.rs`. Red test in `rebind_tests.rs`: `rebind_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is an error. Check: `cargo test -p recipe-web pages::versions::rebind_tests`.
- [x] 9.5 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `src/pages/approval/approve.rs`. Red test in `approve_tests.rs`: `approve_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is an error. Check: `cargo test -p recipe-web pages::approval::approve_tests`.
- [x] 9.6 (recipe-web) [sonnet] F-01, D1. Use the notice answer in the retire handler of `src/pages/versions/actions.rs`. Red test in `actions_tests.rs`: `retire_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is an error. Check: `cargo test -p recipe-web pages::versions::actions_tests`.
- [x] 9.7 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `src/pages/units/add.rs`. Red test in `add_tests.rs`: `add_unit_with_failed_read_after_commit_shows_saved_notice` fails, because the answer is an error. Check: `cargo test -p recipe-web pages::units::add_tests`.
- [x] 9.8 (recipe-web) [sonnet] F-01, D1. Use the notice answer in `set_role`, `deactivate`, `activate`, and `unlock` of `src/pages/users.rs`. Change the doc comment of `refreshed`. Red tests in `users/actions_tests.rs`: one test `<handler>_with_failed_read_after_commit_shows_saved_notice` for each of the four handlers fails, because the answer is an error. Check: `cargo test -p recipe-web pages::users::actions_tests`.
- [x] 9.9 (recipe-app) [sonnet] F-02, D2. Add the hook `on_lock_version_shared` to `src/memory/core/store.rs`. Add the second pick and the second check to `src/export/flow.rs`. Red tests in `src/export/tests.rs`: `approved_export_picks_again_after_an_approval_before_the_lock` fails with `NoApprovedVersion`. `approved_export_gives_404_after_a_second_loss_of_the_approved_state` asserts no third pick and no entry. Check: `cargo test -p recipe-app approved_export`.
- [x] 9.10 (recipe-domain) [sonnet] F-19, D8. Add `E-RCP-037` to `src/error_codes.rs`, `recipe-db/src/trigger_messages.rs`, `recipe-web/src/error/status.rs`, both main `.ftl` bundles, and `recipe-server/tests/error_registry/storage.rs`. Red test: the registry fact `error_registry_every_pair_of_row_and_variant_has_a_sample` fails, because the row of task 4.1 has no sample. Check: `cargo test -p recipe-server --test error_registry every_pair_of_row_and_variant`.
- [x] 9.11 (recipe-server) [sonnet] F-09, D10. Copy `session.secure` into `WebSettings` in `Plan::new` of `src/serve/plan.rs`. Red test: `serve_settings_copy_session_secure_into_web_settings` fails, because the value stays `true`. Check: `cargo test -p recipe-server --all-features session_secure`.
- [x] 9.12 (deploy) [codex] F-19, D8. Add the `FUNCTION` line of `recipe_version_insert_draft()` and the `TRIGGER` line of the trigger `recipe_version_insert_draft` on `recipe_version`, each with its hash, to `deploy/pgtools/restore-code.expected`. Red test: `the_query_gives_exactly_the_lines_of_the_file` in `recipe-db` fails after task 8.7, because the two lines are absent. Check: `cargo test -p recipe-db restore_code`.

## 10. Group G10: F-12 code, F-06 form

- [x] 10.1 (recipe-app) [sonnet] F-12, D3. Add `WEB_016` and `WEB_017` to `recipe-domain/src/error_codes.rs` and `CheckResultFault::code()` to `src/ports/check_results.rs`. Add `error_code` to the `WARN` line of `src/jobs/alerts.rs`. Add the type to `SCOPE` of `recipe-server/tests/error_registry.rs`. Add the two `.ftl` keys when the registry test asks for them. The fixture in `crates/recipe-web/src/error/status/tests.rs` at lines 52 and 60 uses `E-WEB-016` as a made-up code. Change that code to one that no row of `reliability.md` §1.2 uses. Red tests: `alerts_warn_line_of_a_check_fault_holds_error_code` in `src/jobs/alerts/tests.rs` fails, because the field is absent. The registry test fails after the change of `SCOPE`. Check: `cargo test -p recipe-app alerts_warn_line` and `cargo test -p recipe-server --test error_registry`.
- [ ] 10.2 (recipe-web) [sonnet] F-06, D11 (b). Add `hx-boost="true"`, `hx-sync="closest form:drop"`, and `hx-push-url="false"` to `templates/user-new.html`. Red tests: `user_new_form_drops_a_second_submit` in `src/pages/users/tests.rs` fails, because the form has no `hx-sync`. The browser test `e2e/tests/user-create.spec.ts` sends a double submit and fails, because it counts two `POST /users`. It also asserts that the page shows the one-time password. Check: `cargo test -p recipe-web user_new_form` and `cd e2e && npx playwright test user-create`.

## 11. Group G11: F-07 helper

- [x] 11.1 (recipe-app) [sonnet] F-07, D12. Add `DetachedTasks` in `src/identity/detached.rs`: a slot semaphore of `identity.detached_tasks`, one `JoinSet`, a `oneshot` for the result, and `.instrument(Span::current())`. Add the shared function that writes the event and the `WARN` line of a lost event. Reap the finished tasks at each spawn. A reaped panic only frees its slot and writes no line. Add it to `IdentityContext` in `src/identity/context.rs`, to its test literals, and to `recipe-server/src/serve/wiring.rs`. Add the new `TaskStage` with its key `task-stage-*` in both main `.ftl` bundles, and the new `TimeoutStage`. Red tests: `detached_body_ends_after_the_caller_drops_the_future`, `detached_spawn_gives_timeout_when_every_slot_is_taken`, `detached_slots_return_after_many_batches`, `detached_slot_returns_after_an_error`, `detached_reaper_frees_the_slot_of_a_panic_and_writes_no_line`, and `lost_event_gives_one_warn_line_for_a_cancel_before_the_send_after_the_send_and_after_the_receive` do not compile, because the type is absent. Check: `cargo test -p recipe-app detached`.

## 12. Group G12: F-07 use cases and drain

- [x] 12.1 (recipe-app) [sonnet] F-07, D12. Run the span from T1 to the event of `Login` in `src/identity/login.rs` in a detached task. Keep T2 in the request future. Red tests in `login/tests.rs`: `login_writes_login_failed_after_the_caller_drops_the_future` fails, because no event exists. `login_creates_no_session_after_the_caller_drops_the_future` and `login_refuses_with_one_warn_line_when_the_event_fails` follow D12. Check: `cargo test -p recipe-app identity::login::tests`.
- [x] 12.2 (recipe-app) [sonnet] F-07, D12. Run the span from T1 to the event of `ChangeOwnPassword` in `src/identity/change_own_password.rs` in a detached task. Keep `replace` in the request future. Red tests in `change_own_password/tests.rs`: `throttled_change_writes_its_event_after_the_caller_drops_the_future` fails, because no event exists. `change_keeps_the_old_password_after_the_caller_drops_the_future` follows D12. Check: `cargo test -p recipe-app identity::change_own_password::tests`.
- [x] 12.3 (recipe-app) [sonnet] F-07, D12. Run the span from T1 to the event of `UnlockLogin` in `src/identity/unlock_login.rs` in a detached task. Remove the call of `log_lost_unlock_event` from `recipe-web/src/pages/users.rs` and change its test in `users/unlock_tests.rs`. Red tests in `unlock_login/tests.rs`: `unlock_writes_its_event_after_the_caller_drops_the_future` fails, because no event exists. `unlock_writes_exactly_one_warn_line_when_the_event_fails_after_a_drop` follows ADR-0130 as ADR-0172 amends it. Check: `cargo test -p recipe-app identity::unlock_login::tests`.
- [x] 12.4 (recipe-app) [sonnet] F-07, D12. Run the span from T1 to the event of the Basic check in `src/identity/authenticate_basic.rs` in a detached task. Keep the Basic permit wait in the request future. Red test in `authenticate_basic/tests.rs`: `basic_check_writes_login_failed_after_the_caller_drops_the_future` fails, because no event exists. Check: `cargo test -p recipe-app identity::authenticate_basic::tests`.
- [x] 12.5 (recipe-server) [sonnet] F-07, D12. Drain the detached tasks in `src/serve/opened.rs` under `shutdown.drain`, before the pool close. Abort the rest with one `WARN` line that gives the count. Red tests in `src/serve/opened/tests.rs`: `stop_waits_for_detached_tasks_before_the_pool_close` fails, because the pools close first. `stop_aborts_a_detached_task_that_outlives_the_drain` uses a paused clock and a task that does not end. It asserts the shared deadline, the abort, one `WARN` line with the count, and then the pool close. Check: `cargo test -p recipe-server detached`.

## 13. Group G13: release note

- [x] 13.1 (CHANGELOG.md) [mechanical] [haiku] All findings. Add one line for each changed behavior under "Unreleased". Red test: none. Check: ste-lint on `CHANGELOG.md`.

## 14. Group G14: verification

- [ ] 14.1 (workspace) [mechanical] [haiku] Run `cargo fmt --check`. It reports no difference.
- [ ] 14.2 (workspace) [mechanical] [haiku] Run `cargo clippy --all-targets --all-features -- -D warnings`. It reports no warning.
- [ ] 14.3 (workspace) [mechanical] [haiku] Run `cargo test --workspace --all-features --locked`. Every test passes.
- [ ] 14.4 (openspec) [mechanical] [haiku] Run `openspec validate review-design-fixes --strict` and `openspec validate --specs --strict`. Both pass.
- [ ] 14.5 (docs) [sonnet] Run the ste-lint command of `CLAUDE.md` "Commands" on the scope of ADR-0023 and ADR-0135. It reports 0 hard violations.
- [ ] 14.6 (e2e) [sonnet] Run `cd e2e && npm ci && E2E_PASSWORD=<12 to 128 characters> npm test` on the dev stack. Every browser test passes.

## Workflow follow-up

- Review the code with `code-review` and fix each CRITICAL, HIGH, and MEDIUM finding.
- Archive with `openspec-archive-change` after `cargo test` and `openspec validate` pass.
