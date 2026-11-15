# Fix plan closure, 2026-11-14

- Scope: `main` at `3d2141f`, the merge of PR #67. The findings of sections 3.1, 3.2, and 3.3 of `docs/plan/review/2026-11-13-fix-plan.md`. The findings and their failure scenarios are in `docs/plan/review/2026-11-13-full-repo-review.md`.
- The scope also holds the two plan items "F-24 note" and "F-03 note" (`E-PLT-020`) of section 3.2.
- Method: two independent read-only reviewers on 2026-11-14. They are Opus (effort high) and Codex (gpt-6-astra, effort high). Neither reviewer ran a `cargo` command or changed a file except this report.
- This file is not normative. It records the closure verdicts.

## Result

Opus gives 32 findings the verdict CLOSED, 3 findings the verdict KNOWN, and 0 findings the verdict OPEN.

- KNOWN: F-06 part (a), F-22, and F-25. ADR-0172 and ADR-0170 record them by the owner decisions of section 1 of the plan.
- F-06 part (b) has a fix. The verdict KNOWN applies to part (a) only.

Codex gives the same verdict to each finding. The two reviewers agree on all 35 verdicts.

The review found 6 new defects. X-1 is HIGH. N-1 to N-5 are LOW stale doc lines. No new defect reopens a finding in scope.

## Method

1. For each finding, the reviewer read the defect claim of the review and the fix row of the plan.
2. The reviewer found the fix on `3d2141f` in the code, test, doc, or CI file. A PR text, a commit message, or a checked box alone was not evidence.
3. The reviewer checked that the fix removes the failure scenario of the finding.
4. The reviewer named the test or the CI job that guards the fix. A doc-only fix names its doc line and "Decision" or "Doc".
5. For F-06, F-07, and F-22, the reviewer read design D11 to D13 of change B, ADR-0172, and ADR-0173.
6. The reviewer applied the owner decisions of plan section 1 and did not question them.
7. The reviewer compared the Status line of each new and each amended ADR with its index cell in `docs/adr/README.md`. A shell loop did the text comparison.
8. The reviewer read the required status checks of `main` with `gh api` (read only).

Evidence sources:

| Source | Result |
|---|---|
| PR #63 (change A `review-code-fixes`), merge `ba99add` | Merged 2026-11-14 |
| PR #64 (change C `review-deploy-fixes`), merge `58e77ee` | Merged 2026-11-14 |
| PR #65 (archive of A and C), merge `eba0883` | Merged 2026-11-14. Archives `openspec/changes/archive/2026-11-14-review-code-fixes/` and `openspec/changes/archive/2026-11-14-review-deploy-fixes/` |
| PR #66 (change B `review-design-fixes`), merge `a657292` | Merged 2026-11-14. The main CI run had 19 of 19 jobs green |
| PR #67 (archive of B), merge `3d2141f` | Merged 2026-11-14T21:54:00Z. Archive `openspec/changes/archive/2026-11-14-review-design-fixes/` |
| CI run `43645228244` on `3d2141f`, event `push` | "CI completed success jobs=19 non_success=[]" |
| CI run `43645228221` on `3d2141f`, workflow Browser regression | "Browser regression completed success jobs=1 non_success=[]" |
| ADR-0163 to ADR-0173 | Each has an index row (`docs/adr/README.md:175` to `:185`). Each index cell equals its Status line. ADR-0168 has no decision map row. N-3 gives the detail |
| Amended ADRs | ADR-0013, ADR-0019, ADR-0027, ADR-0031, ADR-0041, ADR-0062, ADR-0098, ADR-0109, ADR-0120, ADR-0125, ADR-0130, ADR-0133, ADR-0143, ADR-0151, ADR-0154, and ADR-0172 have the note in their Status line. Each index cell equals its Status line |
| Required status checks of `main` | `gh api` printed 13 contexts: Clippy, Coverage, Dependency policy, Documentation, Format, Migration freeze, Migration rewrite, Panic unwind on musl, Playwright, STE lint, Test, CI scripts, OpenSpec specs. The job "Release toolchain check" is not in the list |

## Verdicts

The column "Opus" holds the verdict of this report. The column "Codex" holds the independent verdict of the second reviewer.

| Id | Severity | Change | PR | Opus | Codex | Evidence (file:line) | Guard test or CI job |
|---|---|---|---|---|---|---|---|
| F-03 | MEDIUM | A | #63 | CLOSED | CLOSED | `crates/recipe-server/tests/error_registry.rs:61` `SCOPE` holds `AuditError`, `StorageError`, `ExportError`, `IdentityError`, `PlantError`, `RecipeError`, `AppError`, `CheckResultFault`, and `WebError`. `:54` excludes `SnapshotItem` only. A reused code fails the fact at `:149`, because the row of that code does not name the new variant | `error_registry_every_sample_code_belongs_to_a_row_that_names_its_variant` (`error_registry.rs:149`), `:137`, `:164`, `:182`, `:216`. Job Test |
| F-04 | MEDIUM | A | #63 | CLOSED | CLOSED | The 11 sites call `random_uuid()?`, for example `crates/recipe-app/src/recipe/add_parameter.rs:121`, `:122`, `plant/create_machine.rs:79`, `:80`, `plant/ensure_derived.rs:234`. `identity/issue.rs:71` maps the fault to `AppError`. The one remaining `Uuid::new_v4` is in the test adapter `memory/recipe/mod.rs:190` | `no_production_file_of_recipe_app_calls_uuid_new_v4` (`crates/recipe-app/tests/random_ids.rs:54`), `:74`, `:84`. Job Test |
| F-05 | MEDIUM | A | #63 | CLOSED | CLOSED | `crates/recipe-web/src/pages/render.rs:108` `page_url` encodes each pair. `:125` `pager_links` is the one pager rule. The callers are `machines/list.rs:175`, `recipes/list.rs:241`, `types/usage.rs:98`, `users.rs:164`, `versions/parameter_table.rs:271`, `versions/table.rs:60` | `pager_links_encode_the_search_text_and_omit_the_previous_link_after_the_end` (`pages/render/tests.rs:268`). `users_pager_links_keep_q_and_the_page_number` (`pages/users/tests.rs:243`), `:229`. Job Test |
| F-08 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-web/src/pages/types/define.rs:93` to `:116` check the name and the type apart and join both errors | `a_member_row_with_a_bad_name_and_no_type_gives_two_lines` (`pages/types/define_member_error_tests.rs:131`), `:162`. Job Test |
| F-10 | LOW | A | #63 | CLOSED | CLOSED | `rg` finds no `expect(dead_code)` or `allow(dead_code)` in non-test files of `crates/recipe-web/src`. `DraftFault` and `draft_lines` are gone. `pages/parameters/grid.rs:1` to `:12` has no inner attribute | Job Clippy with `-D warnings`. Without the attributes, a new dead item gives a warning |
| F-11 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-web/src/pages/form_fields.rs:16` (`FieldLines`), `:124` (`Choice`), `:144` (`TokenFields`) | `shared_form_items_live_in_the_form_fields_module` (`crates/recipe-web/tests/architecture.rs:135`). Job Test |
| F-20 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-server/src/listener.rs:259` to `:263` `collect` drops the `JoinError` and writes no line. The panic hook writes the one line (ADR-0058 (f)) | `a_panic_of_a_connection_task_writes_no_error_line_in_the_loop` (`listener/tests.rs:736`). Job Test |
| F-21 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-server/src/create_admin.rs:65` to `:69` read bytes. `:71` tests the length, and `:76` decodes the bytes after the test | `a_long_input_cut_inside_a_character_is_too_long` (`create_admin/tests.rs:148`). Job Test |
| F-23 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-web/src/pages/render.rs:167` refuses every ASCII control character, also the tab | `see_other_path_refuses_a_control_character` (`pages/render/tests.rs:91`). Job Test |
| F-30 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-server/tests/healthcheck.rs:40` keeps only the floor `PROBE_BUDGET_FLOOR`. `:38` says that no ceiling exists. `:220` to `:223` assert the floor only, so a slow runner only makes the span longer | Mutations of task 9.1 of change A. Job Test |
| F-31 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-app/src/blocking/tests.rs:454` to `:468` use a `oneshot`. A drop without a run closes the channel, so the test needs no sleep. A late run sends a value and fails `:468` | `a_diff_after_its_deadline_does_not_start_the_work` (`blocking/tests.rs:449`). Job Test |
| F-32 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-domain/src/recipe/limit/parse/tests.rs:457`, `:458`, `:483`, `:484` assert the parsed bounds | The two tests at `:454` and `:480`. Job Test |
| F-33 | LOW | A | #63 | CLOSED | CLOSED | `crates/recipe-domain/src/export/hash/tests.rs:160` and `:161` build both bodies with `example_parameters_with_speed_max` (`:30`). Only the bound 250 or 251 differs | `a_changed_limit_gives_another_hash` (`export/hash/tests.rs:159`). Job Test |
| F-24 | LOW | C | #64 | CLOSED | CLOSED | `deploy/db/initdb/01-roles.sh:42` to `:44` count the NUL bytes before the command substitution and stop. Change B also refuses the byte in `crates/recipe-server/src/config/mod.rs:230` | `the_script_stops_on_a_secret_file_with_a_nul_byte` (`crates/recipe-db/tests/storage/grants/init_script.rs:93`). `secret_line_refuses_a_nul_byte` (`crates/recipe-server/src/config/tests.rs:142`). Job Test |
| F-26 | LOW | C | #64 | CLOSED | CLOSED | `.github/workflows/ci.yml:704` (clippy), `:722` (test), `:761` (doc), `:809` (coverage) set `save-if` to `main` only. The new job has it at `:741` | `deploy/ci/test_rust_cache.py:14` with its self-test `:53`, run by `deploy/ci/check.sh:16`. Job CI scripts |
| F-27 | LOW | C | #64 | CLOSED | CLOSED | `.github/workflows/ci.yml:728` to `:743` add the job `release-toolchain` with no `if:` and no `needs:`. `deploy/ci/rust-tests.sh:12` to `:20` read the one version of `deploy/Dockerfile` and run `cargo check --release --locked --package recipe-server` | Job "Release toolchain check". `deploy/ci/test_release_toolchain.py:35`, `:65`, `:135` in job CI scripts. The job is not a required check yet. See "Open items" |
| F-28 | LOW | C | #64 | CLOSED | CLOSED | `deploy/ci/test_pitr_recovery.py:2106` writes `\\$`. `deploy/ci/check.sh:10` to `:15` compile each `deploy/ci/*.py` with `-W error` | Job CI scripts |
| F-29 | LOW | C | #64 | CLOSED | CLOSED | `deploy/Caddyfile:2` and `deploy/Caddyfile.dev:2` name the tls line, the `auto_https` line, and `skip_install_trust` | `caddyfiles_differ_only_in_the_named_lines` (`crates/recipe-server/tests/deploy_values/files/caddy.rs:89`). Job Test |
| F-01 | MEDIUM | B | #66 | CLOSED | CLOSED | `crates/recipe-web/src/pages/saved_notice.rs:39` writes the one `WARN` line. `:59` to `:80` give the notice and set `rowVersion` only with a value. The callers are `parameters/remove.rs:165`, `approval/approve.rs:206`, `versions/actions.rs:341`, `units/add.rs:274`, and five more handlers. ADR-0163. `docs/architecture/api.md:476` to `:485` | 13 tests `*_with_failed_read_after_commit_shows_saved_notice`, for example `pages/parameters/remove_tests.rs:214`, `pages/approval/approve_tests.rs:96`, `pages/versions/actions_tests.rs:281`, `pages/units/add_tests.rs:209`. Job Test |
| F-02 | MEDIUM | B | #66 | CLOSED | CLOSED | `crates/recipe-app/src/export/flow.rs:355` to `:372` pick a second time after a lost approved state. A second loss gives `NoApprovedVersion` at `:372`. ADR-0164. `docs/architecture/api.md:268`. `docs/architecture/data-model.md:2270` | `approved_export_picks_again_after_an_approval_before_the_lock` (`crates/recipe-app/src/export/tests.rs:540`), `:568`, `:594`. Job Test |
| F-12 | LOW | B | #66 | CLOSED | CLOSED | `crates/recipe-domain/src/plant/errors.rs:34` and `crates/recipe-app/src/ports/check_results.rs:71` have `code()`. ADR-0165. `docs/architecture/reliability.md:17`, `:18`, `:190`, `:191` | `plant_errors_code_is_code_of_first_entry` (`plant/errors/tests.rs:80`). `alerts_warn_line_of_a_check_fault_holds_error_code` (`jobs/alerts/tests/checks.rs:242`). The registry test of F-03 has `CheckResultFault` in `SCOPE`. Job Test |
| F-03 note | LOW | B | #66 | CLOSED | CLOSED | `docs/architecture/reliability.md:81` names `SnapshotItem::UnknownUnit` next to `PlantError::UnitNotFound`. `:17` lists `SnapshotItem`. ADR-0165 (j), (k) | `error_registry.rs:51` to `:54` keep `SnapshotItem` out of `SCOPE` by name. Decision |
| F-13 | LOW | B | #66 | CLOSED | CLOSED | ADR-0166. `docs/architecture/reliability.md:474`. `CLAUDE.md:69`. `.claude/rules/rust.md:48` | Doc |
| F-14 | LOW | B | #66 | CLOSED | CLOSED | `docs/adr/0013-rbac-fixed-permissions.md:3` has "Amended by ADR-0046". The index cell equals the Status line | Doc |
| F-15 | LOW | B | #66 | CLOSED | CLOSED | `docs/architecture/data-model.md:2245` (`CreateRecipe`), `:2261` (`CreateUser`), `:2265` (`ResetPassword`) name the read before the transaction. ADR-0168 | Doc |
| F-16 | LOW | B | #66 | CLOSED | CLOSED | `docs/architecture/data-model.md:22` and `.claude/rules/sql.md:14` name the one exception `audit_guard.audit_objects_dropped()`. ADR-0167 | Decision |
| F-17 | LOW | B | #66 | CLOSED | CLOSED | `docs/architecture/data-model.md:40` (`plc_type_member`) and `:41` (`draft_editor`) | Doc |
| F-18 | LOW | B | #66 | CLOSED | CLOSED | `docs/architecture/data-model.md:81` says that `TypedLogin` refuses a control character and that the database checks only the length. ADR-0168 | Doc |
| F-19 | LOW | B | #66 | CLOSED | CLOSED | `migrations/0016_recipe_version_insert.sql:3` to `:19` refuse an insert that is not a live draft with `row_version` 1. ADR-0169. `docs/architecture/reliability.md:139`. `docs/architecture/data-model.md:1697`. `crates/recipe-db/tests/grants.expected:12`. `deploy/pgtools/restore-code.expected:104`, `:135` | `recipe_version_insert_of_approved_row_is_rejected` (`crates/recipe-db/tests/storage/triggers/version_insert.rs:83`), `:66`, `:100`, `:115`, `:124`, `:133`, `:142`. Jobs Test, Migration freeze, Migration rewrite |
| F-25 | LOW | B | #66 | KNOWN | KNOWN | `docs/adr/0170-secret-mounts-of-the-db-service.md:40` to `:45` keep the four mounts and accept the risk. `:60` to `:62` name the mitigations. The operator needs no step | Decision |
| F-24 note | LOW | B | #66 | CLOSED | CLOSED | `docs/adr/0031-migrator-configuration-and-secret-files.md:3` names the NUL byte as the fourth stop case | Doc |
| F-09 | LOW | B | #66 | CLOSED | CLOSED | `crates/recipe-web/src/cookies.rs:33`, `:42`, `:80`, `:91`, `:117` and `csrf/pre_session.rs:70` to `:76` use `WebSettings::secure`. `crates/recipe-server/src/serve/plan.rs:259` copies the setting. ADR-0171. `docs/architecture/api.md:742`. `docs/architecture/reliability.md:962`, `:964` | `login_then_get_with_recipe_session_without_secure` (`crates/recipe-web/src/cookies/tests.rs:81`), `:91`. `serve_settings_copy_session_secure_into_web_settings` (`crates/recipe-server/src/serve/plan/tests.rs:132`). Job Test |
| F-06 | LOW | B | #66 | KNOWN | KNOWN | Part (a) is a known limit: `docs/adr/0172-dropped-future-after-a-committed-step.md:70` to `:72`. The next step of the Administrator is in `docs/ops/runbook.md:2538` to `:2546`. Part (b) has a fix: `crates/recipe-web/templates/user-new.html:6` has `hx-boost`, `hx-push-url="false"`, and `hx-sync="closest form:drop"`. `.claude/rules/web.md:28` | `user_new_form_drops_a_second_submit` (`crates/recipe-web/src/pages/users/new_user_tests.rs:32`) in job Test. `e2e/tests/user-create.spec.ts:60` in job Playwright |
| F-07 | LOW | B | #66 | CLOSED | CLOSED | `crates/recipe-app/src/identity/detached.rs:122` to `:169` run the span from T1 to the event in a detached task. `login.rs`, `change_own_password.rs`, `unlock_login.rs`, and `authenticate_basic.rs` use it. `crates/recipe-server/src/serve/opened.rs:149` drains the tasks. ADR-0172 (g) to (ah). ADR-0173 adds the T2 refusal event | `unlock_writes_its_event_after_the_caller_drops_the_future` (`identity/unlock_login/tests.rs:291`). `login_writes_login_failed_after_the_caller_drops_the_future` (`identity/login/detached.rs:27`). `change_own_password/tests.rs:303`, `authenticate_basic/tests.rs:629`, `detached/tests.rs:49`. `crates/recipe-server/src/serve/opened/tests.rs:357`, `:387`. Job Test |
| F-22 | LOW | B | #66 | KNOWN | KNOWN | `docs/adr/0172-dropped-future-after-a-committed-step.md:119` to `:121` record the limit for the current callers. `docs/adr/0133-*.md:3` has the note. `crates/recipe-server/src/limited_read.rs:152` and `:153` state the limit. The operator needs no step | Decision. A new caller that drops the call early needs a new ADR (ADR-0172 (ak)) |

Count of the column "Opus": 32 CLOSED, 3 KNOWN, 0 OPEN.

Note on F-06: part (b) removes the double-submit scenario. Part (a) stays, because the password cannot reach a client that is gone (ADR-0172 (a)).

Note on F-07: ADR-0173 (k) keeps one narrow window as a known limit. A drop during an await of T2 before its decision writes no event. In that window, the judge passed, so no failed login goes missing. The window is outside the failure scenario of F-07.

## New findings

| Id | Severity | Status | Found by | Location | Short text |
|---|---|---|---|---|---|
| X-1 | HIGH | `CONFIRMED` | Codex | `crates/recipe-app/src/identity/detached.rs:311` | The drop of `DetachedResult` can lose an unreceived `Err` with no `WARN` line |
| N-1 | LOW | `CONFIRMED` | Opus | `README.md:13` | The unreleased range stops at ADR-0153 |
| N-2 | LOW | `CONFIRMED` | Opus | `CHANGELOG.md:5` to `:107` | The section `[Unreleased]` has no line for the behavior of ADR-0173 |
| N-3 | LOW | `CONFIRMED` | Opus, Codex | `docs/adr/README.md:187` to the end | The decision map has no row that names ADR-0168 |
| N-4 | LOW | `CONFIRMED` | Opus | `CLAUDE.md:67`, `.claude/rules/rust.md:41` | The list of log places omits the `WARN` line of ADR-0173 (m) |
| N-5 | LOW | `CONFIRMED` | Codex | `openspec/changes/archive/2026-11-14-review-design-fixes/proposal.md:22` | The proposal names ten ADRs, 0163 to 0172, and omits ADR-0173 |

### X-1 The drop of `DetachedResult` can lose an unreceived `Err` with no `WARN` line

- Severity: HIGH.
- Location: `crates/recipe-app/src/identity/detached.rs:310` to `:315`, the `Drop` of `DetachedResult`.
- Rule: ADR-0173 (m) requires one `WARN` line for an `Err` that no future receives.
- Detail: the drop calls `try_recv()` and then drops the `oneshot::Receiver`. The receiver stays open between the two steps.
- Failure scenario: `try_recv()` returns `Empty`. The task then sends an `Err`, and the send succeeds, so the task writes no line. The drop of the receiver then discards the `Err`.
- Fix direction: call `close()` on the receiver before `try_recv()`. A later send then fails, and the task writes the line.
- State: a separate fix pull request from the branch `fix/detached-result-drop-race` carries the fix and its tests.

### N-1 `README.md` names the unreleased range up to ADR-0153 only

- Severity: LOW.
- Location: `README.md:13`. The line says that `main` holds "the unreleased changes of ADR-0137 to ADR-0153".
- Detail: the section `[Unreleased]` of `CHANGELOG.md` names ADR-0156, ADR-0161, and ADR-0163 to ADR-0172. `main` also holds ADR-0173. The last change of `README.md` is `cdbba1b`, before this plan.
- Failure scenario: an operator reads the README before an upgrade. The line does not name migration 0016, the setting `identity.detached_tasks`, or the cookie names of a `dev` build. The changelog names them, so the effect is small.
- Fix direction: change the range, or remove the range and keep only the link to the changelog.

### N-2 The changelog has no line for ADR-0173

- Severity: LOW.
- Location: `CHANGELOG.md:5` to `:107`, section `[Unreleased]`. The section names ADR-0172 and does not name ADR-0173.
- Rule: `docs/architecture/reliability.md:1176` requires an entry in `CHANGELOG.md` when behavior changes.
- Detail: ADR-0173 changes three behaviors. The task writes the event of a T2 refusal (ADR-0173 (b) to (h)). The task writes one `WARN` line for an error that no future receives (ADR-0173 (m)). A slot after the drain gives `E-WEB-009` at once (ADR-0173 (x)).
- Failure scenario: an operator sees the new `WARN` line "error of a detached task that no future received" (`crates/recipe-app/src/identity/detached.rs:59`). The changelog gives no reason for the line.
- Fix direction: add one line for each of the three behaviors under `[Unreleased]`.

### N-3 The decision map has no row for ADR-0168

- Severity: LOW.
- Location: the decision map of `docs/adr/README.md` starts at line 187. A search for "0168" after that line finds nothing. The index row is at `:180`.
- Rule: `.claude/rules/docs.md` says "Add a row to the index and update the decision map". ADR-0163 to ADR-0173 except ADR-0168 each appear in the map.
- Failure scenario: a reader uses the decision map to find the current ADR of the data model text. The reader does not find ADR-0168 and can change `data-model.md` §1.3 back to a database limit.
- Fix direction: add ADR-0168 to the map row of the data model or the schema conventions.

### N-4 The log places of `CLAUDE.md` omit ADR-0173 (m)

- Severity: LOW.
- Location: `CLAUDE.md:67` and `.claude/rules/rust.md:41`.
- Detail: both lines name five log places. They are the `WebError` mapping, the backstop panic layer, the job supervisor, the lost security event, and the failed read of ADR-0163. They do not name the `WARN` line of an error that no future receives. `docs/architecture/reliability.md:44` names it (ADR-0173 (m) to (p)). The code is `log_unreceived` at `crates/recipe-app/src/identity/detached.rs:54`.
- Failure scenario: a reviewer applies the rule "Log each error once" of `CLAUDE.md`. The reviewer rejects `log_unreceived` as a log place that the rule does not permit, or a later change removes it.
- Fix direction: add the log place of ADR-0173 (m) to both lines.

### N-5 The proposal of change B omits ADR-0173

- Severity: LOW.
- Location: `openspec/changes/archive/2026-11-14-review-design-fixes/proposal.md:22`. The line says "Ten new ADRs, 0163 to 0172".
- Failure scenario: a reader collects the decisions of change B from the proposal. The reader misses the rules of a T2 refusal in ADR-0173.
- Fix direction: name eleven ADRs, 0163 to 0173.

## Open items

- F-27: the owner adds the job "Release toolchain check" to the branch protection of `main` (plan section 1, row F-27). On 2026-11-14 the required status checks do not hold it. Until then, a red job does not stop a merge.
- F-06 part (a), F-22, and F-25 stay KNOWN by ADR-0172 and ADR-0170.
- Plan section 5, DEP-01: the follow-up ADR of ADR-0095 (h) about the dump SQL that `restore-test` runs as the scratch superuser. This review gives it no verdict.
- Plan section 5: a semantic review of the commands of `recipe-app` and of the invariants of `recipe-domain`, on the HEAD after change B. This review gives it no verdict.
- Plan section 5, SEC-01: the finding stays rejected. No item is open.
- N-1 to N-5 (LOW): the commit of this report fixes them, by owner decision of 2026-11-14.
- X-1 (HIGH): the fix pull request of the branch `fix/detached-result-drop-race` closes it.
