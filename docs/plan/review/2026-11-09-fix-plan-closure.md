# Fix plan closure, 2026-11-09

- Scope: `main` at `fcaa5c4`, the merge of PR #48. The 13 MEDIUM findings of `docs/plan/review/2026-11-08-full-repo-review.md` that `docs/plan/2026-11-08-fix-plan.md` covers. Also the two LOW findings D5-4 and D1-2 that joined the plan.
- D6-6 (MEDIUM) stays KNOWN by owner decision Q-B (3). This review gives it no verdict.
- Method: two independent read-only reviewers on 2026-11-09. They are Opus (effort high) and Codex (gpt-6-astra, effort high). Neither reviewer ran a `cargo` command or changed a file except this report.
- This file is not normative. It records the closure verdicts.

## Result

Both reviewers give 14 findings the verdict CLOSED and 1 finding the verdict OPEN. D1-2 is OPEN by the decision D4 of the change `type-check-before-insert`. The two reviewers agree on the verdict of each finding.

The review found 4 new defects: C-1, C-2, and X-1 are LOW, and X-2 is MEDIUM. No new defect reopens a finding in scope.

## Method

1. For each finding, the reviewer read the defect claim of the review and the fix task in sections 3 to 7 of the plan.
2. The reviewer found the fix on `fcaa5c4` in the code, test, doc, or CI file. A PR text or a checked box alone was not evidence.
3. The reviewer checked that the fix removes the failure scenario of the finding.
4. The reviewer named the test or the CI step that guards the fix.
5. Opus ran the check script of plan section 3.1 with `python -I` from the repository root.
6. Codex gave its verdicts without the Opus verdicts. It ran the migration check on the shipped migrations, probes of D4-1, mutations of the `secure` step, and `ste-lint.py` on `CHANGELOG.md` and `README.md`.
7. A fresh Opus check of X-1 reduced its severity. The orchestrator checked X-2. Opus counted the lines of X-2 again.

Evidence sources:

| Source | Result |
|---|---|
| PR #44 (WP0), merge `0519b9e` | Docs only. Commit `00865bd` |
| PR #45 (WP3), merge `9c9e4ab` | Commit `57d833e`. Archive `openspec/changes/archive/2026-11-08-type-check-before-insert/` |
| PR #46 (WP2), merge `6a9dfd4` | Commit `a6a8130`. Archive `openspec/changes/archive/2026-11-08-shown-name-and-focus-round-2/` |
| PR #47 (WP1), merge `943f5e6` | Commit `e237691`. Archive `openspec/changes/archive/2026-11-09-backup-result-alerts/` |
| PR #48 (WP4), merge `fcaa5c4` | Commits `02dd9d1` and `a28e0d5`. Archive `openspec/changes/archive/2026-11-09-test-gates-round-2/` |
| CI run `42631080488` on `fcaa5c4`, event `push` | pass. All 18 jobs green, including Test, Coverage, Migration rewrite, CI scripts, STE lint, OpenSpec specs, and Compose stack |
| CI run `42631080472` on `fcaa5c4`, workflow Browser regression | pass. Job Playwright green |
| Check script of plan section 3.1 | 153 rows, 1 number difference (ADR-0062), 16 text differences. Section "New findings" gives C-1 |
| ADRs 0150 to 0153 | Each has an index row (`docs/adr/README.md:162` to `:165`) and a decision map row (`:182`, `:184`, `:178` and `:195`, `:206`). Each amended ADR has the "Amended by" note in its Status line |

## Verdicts

The column "Opus" holds the verdict of this report. The column "Codex" holds the independent verdict of the second reviewer.

| Id | Severity | WP | PR | Opus | Codex | Evidence (`file:line`) | Guard test or CI step |
|---|---|---|---|---|---|---|---|
| D8-1 | MEDIUM | WP0 | #44 | CLOSED | CLOSED | Status lines `docs/adr/0123-*.md:3`, `0124`, `0125`, `0126`, `0130`, `0132`, `0133`, `0135` (each line 3) name the amended ADR. Rows `docs/adr/README.md:135` to `:147` equal them. Rows `:55` (0043), `:72` (0060), `:110` (0098), `:116` (0104) equal their Status lines. Row `:139` (0127) equals its Status line | No CI guard. The check script of plan section 3.1 finds no number difference in these rows. C-1 names the one remaining difference |
| D8-2 | MEDIUM | WP0 | #44 | CLOSED | CLOSED | `CHANGELOG.md:5` to `:62` list ADR-0137 to ADR-0153. `:12` names the stop of migration `0011_plc_type_rules.sql`. `:27` names the flag `loadable`. `:42` and `:69` replace the stale S7 gap: 21 of 22 rows `Documented`, row A03 `Unverified`. `README.md:13` names the unreleased range. Released sections stay unchanged | STE lint job (form only). No content guard. C-2 names a stale range |
| D3-1 | MEDIUM | WP0 | #44 | CLOSED | CLOSED | `docs/adr/0150-no-per-login-limit-over-all-addresses.md:3` accepts the limit M9. `docs/adr/0035-identity-sessions-clarifications.md:3` has "Amended by ADR-0150". `docs/ops/runbook.md:2456` and `:2457` list the gap and the residual risk | Decision. The threshold of the control alert has the test `more_than_the_failed_login_count_in_the_window_makes_failed_logins_active` (`crates/recipe-app/src/jobs/alerts/tests/conditions.rs:38`) |
| D7-1 | MEDIUM | WP1 | #47 | CLOSED | CLOSED | `deploy/pgtools/lib/check-result.sh:6` to `:22` write through a temp name and `mv -T`. `deploy/pgtools/bin/restore-test:135` to `:138` and `deploy/pgtools/bin/basebackup-once:109` to `:112` write `passed` or `failed` from the exit trap. `crates/recipe-app/src/jobs/alerts.rs:381` to `:392` and `:514` to `:534` raise `restore_test` and `base_backup` on `failed`. They also raise them on an old result, on a missing file after the grace, and on a read fault. `docs/architecture/reliability.md:382`, `:383`, `:1044`. X-1 names a narrow signal window in the exit trap | `deploy/pgtools/tests/lib/restore-result-cases.sh` and `wal-base-cases.sh` in the Compose stack step "Run the pgtools tests" (`.github/workflows/ci.yml:101`). `crates/recipe-app/src/jobs/alerts/tests/checks.rs:92` to `:139`. `crates/recipe-server/src/check_results/tests.rs`. Jobs Test and Coverage |
| D2-1 | MEDIUM | WP1 | #47 | CLOSED | CLOSED | `crates/recipe-server/src/backups.rs:88` to `:102` run the scan through `read_with_limit` and map each limit fault to `BackupFault::Unreadable`. `crates/recipe-server/src/limited_read.rs:125` to `:152` wrap the join in `tokio::time::timeout` and keep the set of stalled paths of ADR-0133. `crates/recipe-server/src/serve/process.rs:134` and `:191` pass `health.disk_check_interval`. `crates/recipe-server/src/check_results.rs:133` uses the same function | `a_reader_that_blocks_gives_unreadable_after_the_limit` (`crates/recipe-server/src/backups/tests.rs:389`), `:401`, `:421`, and 13 tests in `limited_read/tests.rs`. Job Test |
| D5-3 | MEDIUM | WP2 | #46 | CLOSED | CLOSED | `crates/recipe-web/src/pages/diff_view/cells.rs:82` to `:90` call `name_cell`, and `:110` to `:119` use `shown_name`. The translated names come with `DiffPart::Parameter` (`crates/recipe-domain/src/recipe/diff/values.rs:90`), so the read stays in the diff transaction. `templates/components/diff.html:12` puts the marker outside `lang="en"`. ADR-0152. `openspec/specs/web-ui/spec.md:312` to `:323` | `crates/recipe-web/src/pages/diff_name_tests.rs:95`, `:105`, `:115`, `:125`, `:135` (diff page and approval preview). Job Test |
| D5-4 | LOW | WP2 | #46 | CLOSED | CLOSED | `crates/recipe-web/templates/parameter-form.html:13` and `:14` pass `"en"` | `parameters_page_edit_form_tags_the_english_base_inputs_with_lang_en_in_pl` (`crates/recipe-web/src/pages/parameters/page_tests.rs:342`) and `:358`. Job Test |
| D5-2 | MEDIUM | WP2 | #46 | CLOSED | CLOSED | `crates/recipe-web/src/pages/versions/page.rs:121` to `:136` set `next` of the layout to the version path for a content answer. The three callers of `with_push_url` (`approval/approve.rs:204`, `parameters/remove.rs:117`, `versions/actions.rs:338`) push the same version path. No other handler sets `HX-Push-Url` to a path. See the note below the table | `crates/recipe-web/src/pages/versions/lang_next_tests.rs:48` (approve), `:59` (retire), `:86` (parameter removal). Job Test |
| D5-1 | MEDIUM | WP2 | #46 | CLOSED | CLOSED | `crates/recipe-web/templates/password.html:12` has `id="password-submit"` | `crates/recipe-web/src/pages/own/password_tests.rs:68` and `:82`. Guard `oob_forms_every_out_of_band_form_has_a_submit_button_with_a_fixed_id` (`crates/recipe-web/src/components_tests/oob_forms.rs:387`) lists 15 out-of-band forms. Job Test |
| D1-1 | MEDIUM | WP3 | #45 | CLOSED | CLOSED | `crates/recipe-app/src/recipe/update_definition.rs:166` to `:172` run `require_current_type` and `require_no_nested_array` before `lock_live_unit` (`:173`) and `resolve_type` (`:174`). `crates/recipe-app/src/plant/queries/type_text.rs:60` decides a type change without an insert | `expr_names_row_is_true_exactly_when_ensure_expr_resolves_to_the_row` (`type_text_tests.rs:177`). `a_refused_array_of_an_array_takes_no_unit_lock_and_no_type_key` (`update_definition/nested_array_tests.rs:219`). `update_definition/tests.rs:268`, `:390`, `:445`. `a_refused_changed_type_gives_plt_013_and_does_not_wait_for_the_array_lock` (`crates/recipe-db/tests/storage/race/refused_type.rs:102`). Job Test |
| D1-2 | LOW | WP3 | #45 | OPEN | OPEN | `crates/recipe-app/src/plant/current_type.rs:16` still parses the text a second time. `crates/recipe-domain/src/plant/type_expr.rs:17` keeps no rule set. Design D4 of the change (`openspec/changes/archive/2026-11-08-type-check-before-insert/design.md:106` to `:117`) keeps D1-2 out, because both fix directions touch three crates | None. A new write path without `require_current_type` passes every test. The plan marks D1-2 optional (section 6.2) |
| D6-1 | MEDIUM | WP4 | #48 | CLOSED | CLOSED | `.github/workflows/ci.yml:721` and `:722` run `bash deploy/ci/rust-tests.sh secure` in the job `test`, which has no `if:`. `deploy/ci/rust-tests.sh:33` to `:46` fail unless exactly 1 test passes. `CONTRIBUTING.md:98` lists the step | `test_secure_refusal_step` (`deploy/ci/check.sh:433`) in the job CI scripts. Task 1.3 of the change showed a red step with `DEV_BUILD = true`. Codex ran the mutations of the step again |
| D4-1 | MEDIUM | WP4 | #48 | CLOSED | CLOSED | `deploy/ci/migration_rewrite.py:373` to `:402` report `add column of a constrained domain`, also through a base domain and for an earlier file. ADR-0153. `docs/architecture/data-model.md:2144` and `:2146` | `test_add_column_of_constrained_domain_fails` (`deploy/ci/test_migration_rewrite_forms.py:102`), `:380`, and about 40 more cases. Job Migration rewrite (`.github/workflows/ci.yml:841` to `:846`). Task 2.4 repeated the PostgreSQL 16.15 probe |
| D6-3 | MEDIUM | WP4 | #48 | CLOSED | CLOSED | `crates/recipe-web/src/pages/xss_tests.rs:235` (`password` of `POST /login`), `:273` (`current_password`), `:289` (`password` of `POST /me/password`). The tests also refuse an answer that shows an escaped password | Guard `xss_coverage_every_deserialized_field_has_a_corpus_test_or_a_reason` (`crates/recipe-web/src/pages/xss_coverage_tests.rs:1061`), table rows `:84` to `:110`. Job Test |
| D6-2 | MEDIUM | WP4 | #48 | CLOSED | CLOSED | `e2e/compose/end-to-end.spec.ts:211` to `:224` call `exportContentFaults` with the machine code, the reference, the version, the unit, `SPEED`, `INT`, `50`, `0`, and `100` (`:24`). `e2e/compose/support/export-content.ts:80` to `:91` compare each field and the one limit | Compose stack step "Run the end-to-end smoke suite" (`.github/workflows/ci.yml:121`), green in run `42631080488`. Pure test `e2e/tests/export-content.spec.ts:43` in run `42631080472` |

Count: 14 CLOSED, 0 PARTIAL, 1 OPEN. Both reviewers give the same count.

Note on D5-2: Codex reports that the version page already set the pushed path before PR #46. The original claim of D5-2 was then possibly a false positive. The tests of WP2 now guard the behavior, so the verdict stays CLOSED.

## Done check of plan section 8

| Check | Result |
|---|---|
| Each finding in scope has an archived change or a merged docs commit | pass. D1-2 is OPEN by design D4 and is optional |
| Each new ADR is in the index and in the decision map | pass |
| The script of section 3.1 finds no difference | fail by 1 row at `fcaa5c4`. C-1 gives the detail. The commit of this report fixes C-1, and the script now gives 0 number differences |
| `cargo clippy`, `cargo test`, `cargo fmt --check`, `cargo deny check`, `openspec validate --all`, `ste-lint.py` | pass in CI run `42631080488` |
| `tools/coverage-gate.py` on each crate | pass in the job Coverage of run `42631080488` |
| The job `compose` on `main` with the smoke suite of section 7.4 | pass. The `push` run `42631080488` ran the job, so no manual dispatch was necessary |

## New findings

| Id | Severity | Reviewer | Location | Short text |
|---|---|---|---|---|
| X-2 | MEDIUM | Codex, checked by the orchestrator and Opus | `crates/recipe-web/src/pages/xss_coverage_tests.rs`, `crates/recipe-web/src/components_tests/oob_forms.rs:187` | A file of 1650 lines and a function of 56 lines break the limits of `AGENTS.md:50` |
| X-1 | LOW | Codex, reduced from MEDIUM by a fresh Opus check | `deploy/pgtools/bin/basebackup-once:110` to `:116`, `deploy/pgtools/bin/restore-test:136` to `:142` | A signal after the result decision gives the file `passed` and the exit code 143 |
| C-1 | LOW | Opus | `docs/adr/README.md:74` | The row of ADR-0062 drops the citation of ADR-0035 |
| C-2 | LOW | Opus | `README.md:13` | The unreleased range stops at ADR-0150 |

### X-2 A test file and a test function break the size limits

- Severity: MEDIUM. `AGENTS.md:50` sets at most 800 lines for a file and 50 lines for a function. The same line calls a violation a medium finding.
- Location 1: `crates/recipe-web/src/pages/xss_coverage_tests.rs` has 1650 lines. Commits `02dd9d1` and `a28e0d5` of WP4 added it.
- Location 2: `parse_template` at `crates/recipe-web/src/components_tests/oob_forms.rs:187` to `:242` has 56 lines. Commit `a6a8130` of WP2 added it. Opus counted both values again on `fcaa5c4`.
- Failure scenario: the coverage table, the source parser, and the unit tests of the parser are in one file. A reviewer of a later change to the guard must read 1650 lines. A defect in the parser can then pass the review.
- Fix direction: split the table, the parser, and the parser tests into separate modules. Move the read of the import lines out of `parse_template`.

### X-1 A signal after the result decision gives `passed` with a failed exit code

- Severity: LOW. Codex reported MEDIUM. A fresh Opus check found the scope narrower.
- Location: `deploy/pgtools/bin/basebackup-once:110` to `:116` and `deploy/pgtools/bin/restore-test:136` to `:142`, after the line `[ "$status" -ne 0 ] || result=passed`.
- Detail: the exit trap sets the handler `status=143` for `TERM` (`basebackup-once:90`). Bash uses dynamic scope, so the handler changes the local `status` of `cleanup`. The variable `result` already holds `passed`.
- Failure scenario:
  1. A check passes, and `cleanup` sets `result=passed`.
  2. A `TERM` arrives while `write_check_result` runs, for example during a slow `mv`.
  3. The handler sets `status` to 143.
  4. The file says `passed`, and the process exits with the code 143.
- Codex reproduced the case with a slow `mv` stub in both scripts.
- Effect: no failed check gets `passed`. `crates/recipe-app/src/jobs/alerts.rs:530` reads only the file. So no alert for a real failure goes missing.
- Rule: the case breaks the literal text of ADR-0151 (f), "Every other exit code gives `failed`" (`docs/adr/0151-result-files-and-alerts-of-the-weekly-backup-checks.md:62`).
- Guard: no test sends a signal after the decision line. `case_restore_result_term` (`deploy/pgtools/tests/lib/restore-result-cases.sh:35`) sends `TERM` during the data step only.
- Fix direction: run `trap '' TERM INT HUP` after the decision line, or amend ADR-0151 (f). Add a test with a signal during the write.

### C-1 The row of ADR-0062 drops the citation of ADR-0035

- Severity: LOW.
- Location: `docs/adr/README.md:74`, against `docs/adr/0062-configuration-keys-and-secret-files.md:3`.
- Detail: the Status line holds the note "Each of these two files holds a key ring (ADR-0035)". The row gives the short form "key files hold a key ring" without "(ADR-0035)". The file last changed on 2026-10-30, before the review of 2026-11-08.
- Assessment: the citation is not an amendment relation. A reader of the row gets no wrong amendment. So C-1 does not keep D8-1 open.
- Failure scenario:
  1. The owner adds the script of plan section 3.1 as a CI step.
  2. The step fails on its first run, because the row and the Status line name different ADR numbers.
  3. The done check of plan section 8 also fails by this one row.
- Fix direction: add "(ADR-0035)" to the row, or let the script ignore a citation inside a note.
- Status: fixed in the commit of this report.

### C-2 `README.md` names the unreleased range up to ADR-0150 only

- Severity: LOW.
- Location: `README.md:13`.
- Detail: the line says that `main` holds "the unreleased changes of ADR-0137 to ADR-0150". PRs #46, #47, and #48 added ADR-0151, ADR-0152, and ADR-0153 to `CHANGELOG.md` `[Unreleased]` and did not update this line. The last change of `README.md` is `00865bd`.
- Failure scenario: an operator reads the status of the README before an upgrade. The line does not name the new alerts `restore_test` and `base_backup` of ADR-0151 or the mode `0711` of `/wal`. The changelog names them, so the effect is small.
- Fix direction: change the range to "ADR-0137 to ADR-0153", or remove the range and keep only the link to the changelog.
- Status: fixed in the commit of this report.

## Open items

- D6-6 (MEDIUM) stays KNOWN by owner decision Q-B (3). No soak script and no nightly job exist. `CHANGELOG.md:66` and `README.md:17` list the gap.
- D1-2 (LOW) stays OPEN. A later change needs its own ADR that amends ADR-0143 (i2).
- X-2 (MEDIUM): split `xss_coverage_tests.rs` and shorten `parse_template`.
- X-1 (LOW): close the signal window of the two exit traps, or amend ADR-0151 (f).
- C-1 (LOW) and C-2 (LOW): fixed in the commit of this report.
- The owner scans G-1 and G-2 stay open. No production server exists yet.
- The optional CI step of plan section 3.1 does not exist. The owner decides whether a later change adds it. C-1 must close first, or the step fails at once.
- The Status line of `docs/plan/2026-11-08-fix-plan.md:8` still says "No work package started". The plan is not normative, so this item needs no finding.
