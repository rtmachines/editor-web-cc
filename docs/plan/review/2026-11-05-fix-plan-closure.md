# Fix plan closure, 2026-11-05

- Scope: main at `a4b5204`, the merge of PR #23. The findings of `docs/plan/review/2026-11-04-full-repo-review.md` that `docs/plan/2026-11-04-fix-plan.md` covers.
- Method: local done check on `a4b5204`, CI run `42086258008`, and two read-only reviewers on 2026-11-05: Opus and Codex (gpt-6-astra, effort high).
- This file is not normative. It records the closure verdicts.

## Result

This review gives every finding in scope of the plan a closed verdict. D6-3 stays a known gap, and D4-2 keeps a residual risk that Codex rates as partial. The plan of `docs/plan/2026-11-04-fix-plan.md` finished all its steps.

## Done check of the plan

| Check | Result |
|---|---|
| `cargo fmt --all --check` | pass |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | pass |
| `cargo test --workspace --all-features --locked` | pass. 7859 passed, 0 failed |
| `cargo deny --locked check` | pass |
| `cargo doc` with `-D warnings` | pass |
| `openspec validate --all --strict` | pass |
| `ste-lint.py` on the ADR-0023 scope | pass. 0 hard violations |
| `cargo llvm-cov` plus `tools/coverage-gate.py` | pass. `recipe-app` 97.24 %, `recipe-db` 95.20 %, `recipe-domain` 96.10 %, `recipe-export` 99.30 %, `recipe-server` 95.83 %, `recipe-web` 98.04 %. Gate: 80 % lines per crate |
| CI run `42086258008` on `a4b5204` | pass. Every job green, including Coverage, Migration rewrite, Compose stack, Load (musl), and Load (cc-debian13) |

## Verdicts

| Id | Verdict | Evidence | Note |
|---|---|---|---|
| D8-1 | CLOSED | `docs/adr/README.md` rows 27, 34, 46, 47, 90 match the Status lines. Commit `bf4a7f6` | The same defect returned for ADR-0064 and ADR-0128. This change fixes both rows |
| D7-1 | CLOSED | `deploy/db/initdb/01-roles.sh:72`. Tests `the_init_script_log_holds_no_password_of_a_role` and `every_password_line_comes_after_the_five_log_guards_and_before_the_first_connect`. Runbook check at `docs/ops/runbook.md:1668`. Commit `cfc6880` | Follow-up (LOW): CHANGELOG and the upgrade section do not point operators of existing volumes to the runbook check |
| D4-1 | CLOSED | `after_release` hook at `crates/recipe-db/src/database.rs:289`. Tests `a_released_connection_with_an_open_transaction_is_not_reused`, `a_released_connection_with_a_failed_transaction_is_not_reused`, and `a_connection_stays_open_over_a_command_and_a_query`. ADR-0124. Commit `cfc6880` | None |
| D2-1 | CLOSED | `race_stop` at `crates/recipe-server/src/serve/server.rs:108`, exit code 70. Tests `an_internal_loop_that_panics_ends_the_wait_and_the_stop_keeps_its_order` and `an_ended_accept_loop_gives_the_exit_code_70`. ADR-0125. Commit `cfc6880` | None |
| D5-1 | CLOSED | `crates/recipe-web/src/pages/versions/parameter_table.rs:62` and `crates/recipe-db/src/transaction/parameter_reads.rs:39`. Tests in `translation_fallback_tests.rs` and `parameter_page_gives_the_translated_name_of_the_requested_language`. Commit `c9f18cc` | None |
| D5-2 | CLOSED | `templates/components/parameter.html:4` sets `lang="en"`. Test `translation_fallback_english_base_text_has_the_language_tag_and_the_marker_stands_outside`. Commit `c9f18cc` | None |
| D6-1 | CLOSED | `tools/coverage-gate.py` finds test modules from the source. Test `test_inline_test_module_names_file_and_line`. ADR-0127. Commits `271e1a9` and `94d7288` | The gate reads the source instead of `--ignore-filename-regex`. The effect is the same |
| D6-7 | CLOSED | `tools/coverage-gate.py:339` fails a production crate with no reported lines. Tests `test_production_crate_missing_from_nonempty_report_fails` and `test_production_crate_with_zero_reported_lines_fails`. Commit `94d7288` | None |
| D6-5 | CLOSED | `crates/recipe-web/src/routes/matrix.rs:215` calls `assert_common_headers`, which checks the `X-Request-Id` version 7 and the 5 security headers. Tests `matrix_want_rejects_a_reply_without_a_common_header` and `matrix_want_rejects_a_request_id_of_version_4`. Commit `c3ca924` | None |
| D6-6 | CLOSED | `crates/recipe-db/tests/storage/grants/checks.rs:53` reads the rights of PUBLIC and the default ACLs. Test `each_right_of_public_is_named_as_granted_but_not_in_the_file`. Commit `f10fffb` | None |
| D4-2 | CLOSED, with a residual risk | `deploy/ci/migration_rewrite.py`, job `migration-rewrite` (`ci.yml:814`). Tests in `test_migration_rewrite.py`. The checker rejects the three forms of `data-model.md` section 8 and DO blocks. It passes `ADD COLUMN ... GENERATED ALWAYS AS (...) STORED`, `ADD CONSTRAINT ... CHECK` or `FOREIGN KEY` without `NOT VALID`, and a non-concurrent `CREATE INDEX`. Commit `94d7288` | Opus: CLOSED. Codex: PARTIAL. ADR-0127 records the passing forms as a known residual risk. This review keeps CLOSED |
| D6-2 | CLOSED | Job `load` (`ci.yml:158`), `deploy/ci/load.sh`, `load_flood.py`, `load_memory.py`, `load_seed.py`, `load_timing.py`, and the tests `test_load_*.py`. Both legs green on `a4b5204` | The `cc-debian13` leg has `continue-on-error` by ADR-0128. Only the musl leg blocks |
| D6-4 | CLOSED | `e2e/compose/end-to-end.spec.ts` through Caddy in job `compose` (`ci.yml:104`). Secret scan before upload. Commits `97a4b81` and `31e3807` | None |
| D6-3 | KNOWN | Owner decision Q-B (3). `README.md:13`, `CHANGELOG.md:75`, and `docs/ops/runbook.md:2333` list the gap. They already state that no nightly CI job exists | The optional wording point of plan section 7.3 needs no change |

## New defects of the closure review

### 1. ADR-0064 and ADR-0128 index rows

- Severity: MEDIUM.
- Status: fixed in this change.
- Detail: The rows of ADR-0064 and ADR-0128 in `docs/adr/README.md` said only "Accepted". Both rows now carry the "Amended by" text of the Status line.

### 2. DO tokens in function bodies

- Severity: LOW.
- Status: open.
- Detail: `body_forms` in `deploy/ci/migration_rewrite.py:368` reports every `do` token in a function or procedure body as a DO block. `INSERT ... ON CONFLICT DO NOTHING` or `DO UPDATE` inside a new function therefore fails the job without cause. Top-level statements are not affected.

## Open items for the next round

- The 28 LOW findings of the 2026-11-04 review that the plan did not cover.
- The LOW defect 2 above.
- The D7-1 follow-up.
