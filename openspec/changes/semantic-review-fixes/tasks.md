# Tasks

Each task names one finding or one doc conflict of `docs/plan/review/2026-11-17-semantic-review.md` and one decision of `design.md`. One agent does one task. Haiku does each `[mechanical]` task. Sonnet does each other code task. Opus writes the content of each doc task that is not `[mechanical]`.

Each code task first writes its test and runs the RED command. The RED command fails on the defect. Then the task writes the fix and runs the GREEN command. A task that only adds a test to code without a defect has no RED state. It names a mutation instead: apply the mutation once, see the test fail, and restore the code. A doc task has no test. It names its check command.

The `recipe-db` tests need Docker. They start their own PostgreSQL container (ADR-0101). The test target of `recipe-db` is `storage`.

The ste-lint check of a doc task is `python -I tools/ste-lint/ste-lint.py <files of the task>`. It reports 0 hard violations.

## Run order

1. Task 1.0 runs when the owner accepts ADR-0190, ADR-0191, and ADR-0192 with this file. No other task starts before it.
2. Group 2 (documents) edits no code. It can run in parallel with groups 3 to 6.
   - Task 2.1 runs before task 2.2, because both edit `docs/architecture/data-model.md`.
   - Task 2.3 edits other files and runs in parallel with 2.1 and 2.2.
   - Task 2.4 runs after task 2.2, because both edit `docs/architecture/domain-model.md`.
   - RCB-01 is a doc fix (design D2). Task 2.4 completes it. The plan has no task 5.1.
   - DC-1 has no task in group 2. Task 1.0 completes it, because ADR-0190 changes no other document (ADR-0190 (h)).
3. Group 3 (`recipe-domain`) edits one test file of `recipe-domain`. It runs in parallel with every other group. Its mutation runs in a separate copy (rule 7).
4. Groups 4, 5, and 6 compile `recipe-app`. The `recipe-db` tests also compile `recipe-app`. A task that leaves `recipe-app` not compiling stops the tests of the other tasks. In one working tree, these groups therefore run in series: 4, then 5, then 6.
   - In one working tree, group 5 runs in series: 5.2, 5.3, 5.4, then 5.5.
   - With one git worktree per agent, tasks 4.1, 5.2, 5.3, and 5.5 can run in parallel. Their files do not overlap.
   - Task 5.4 runs after task 5.3. Both edit the module `plant/define_udt`, and task 5.4 also edits `memory/types/mod.rs`, which task 6.3 edits.
   - Group 6 always runs in series: 6.1, then 6.2, then 6.3. Each task edits the memory adapter. Task 6.3 changes the callers of `state_mut` that tasks 5.4, 6.1, and 6.2 edit, so it runs last.
5. Task 7.1 runs after groups 5 and 6.
6. Group 8 runs last. It is the gate of the change.
7. A mutation of tasks 3.1, 4.1, and 5.5 runs only in a separate git worktree or a repository copy. It never runs in the shared working tree. The copy holds the new tests of the task. The tests of other tasks then see no mutated code. The task deletes the copy after it sees the test fail. It does not commit the mutation.

## 1. Acceptance

- [x] 1.0 (docs/adr) [mechanical] When the owner accepts ADR-0190, ADR-0191, and ADR-0192, make these edits:
  - Change the Status line of each of the three ADRs from "Proposed" to "Accepted".
  - In `docs/adr/README.md`, change "Proposed" to "Accepted" in the index rows of the three ADRs. Delete "proposed: " from the three decision map cells "Also proposed: 0190", "Also proposed: 0191", and "Also proposed: 0192".
  - Delete the mark " (proposed)" after "ADR-0190" in the Status lines of ADR-0035 and ADR-0172 and in their index cells.
  - Check: these three commands print nothing: `grep -rn 'ADR-0190 (proposed)\|proposed: 019' docs/adr`, `grep -n '^Status: Proposed' docs/adr/019[0-2]-*.md`, and `grep -n '| Proposed\.' docs/adr/README.md`. Also, ste-lint on `docs/adr/0035-*.md docs/adr/0172-*.md docs/adr/0190-*.md docs/adr/0191-*.md docs/adr/0192-*.md docs/adr/README.md` reports 0 hard violations.

## 2. Documents

- [x] 2.1 (docs/architecture) RCA-01, design D10, ADR-0191 (f) to (h). In `docs/architecture/data-model.md` §6, change the lock column and the statement column of the rows `AddParameter` (line 2251), `UpdateParameterDefinition` (line 2252), and `RebindTypeRevision` (line 2258). Keep every other text of the cells. Make these edits:
  - `AddParameter`, lock column: delete ", touch" after "with the `row_version` compare".
  - `AddParameter`, statement column: delete the first sentence "`INSERT draft_editor`.". After "`EnsureDerivedType` resolves it to `plc_type_id`.", add "The domain checks the new parameter. Then the touch and `INSERT draft_editor` run."
  - `UpdateParameterDefinition`, lock column: delete ", touch, then" after "with the `row_version` compare" and write ", then".
  - `UpdateParameterDefinition`, statement column: delete the first sentence "`INSERT draft_editor`.". After "`EnsureDerivedType` resolves it to `plc_type_id`.", add "The domain checks the changed parameter. Then the touch and `INSERT draft_editor` run."
  - `RebindTypeRevision`, lock column: replace "then as `SetParameterValue`" with "`recipe FOR UPDATE`, `recipe_version FOR UPDATE` with the `row_version` compare".
  - `RebindTypeRevision`, statement column: delete the first sentence "`INSERT draft_editor`.". Move the sentence "`EnsureDerivedType` gives each new array type." to the place after "Compute the dimensions of each new array type first.".
  - In the same cell, replace "and checks every parameter before the first write." with "and checks every parameter. Then the touch and `INSERT draft_editor` run.". Move these two sentences of the domain mapping before "For every parameter of the draft whose type uses `from_type_id`".
  - In the rule "Every draft command then runs ..." (line 2225), add the two sentences of ADR-0191 (h) at the end, word for word.
  - Red test: none, because the task changes a document. Check: ste-lint on `docs/architecture/data-model.md`. The three rows and the rule name the order of `crates/recipe-app/src/recipe/add_parameter.rs` (line 128), `update_definition.rs` (line 131), and `rebind.rs` (line 91). `grep -c 'compare, touch, \|then as .SetParameterValue.\|INSERT draft_editor.. Parse\|INSERT draft_editor.. Check that' docs/architecture/data-model.md` prints 0.
- [x] 2.2 (docs/architecture) [mechanical] DC-2, design D10, ADR-0191 (a) to (e). After 2.1, add the missing codes:
  - `domain-model.md` §5.3: `E-WEB-005 NotFound` (unknown login) in the rows `SetUserRole`, `DeactivateUser`, `ActivateUser`, `ResetPassword`, and `UnlockLogin`.
  - `domain-model.md` §5.4: `E-WEB-011 TaskFailed` (export) in the row `ExportVersion`.
  - `data-model.md` §6, guard column: `PrecisionExceeded` (`E-RCP-055`), `S7AddressPastBlockEnd` (`E-RCP-056`), and `S7DataBlockOutOfRange` (`E-RCP-057`) in the rows `AddParameter` and `RebindTypeRevision`. `S7AddressOverlap` (`E-RCP-022`, ADR-0149) and `PrecisionExceeded` (`E-RCP-055`) in the rows `SetParameterValue` and `ReplaceLimits`.
  - Red test: none, because the task changes documents. Check: ste-lint on `docs/architecture/domain-model.md docs/architecture/data-model.md`, and each code of ADR-0191 (a) to (e) is in its row.
- [x] 2.3 (docs/architecture) [mechanical] DC-3, design D10, ADR-0192 (d), (e). Replace the last sentence of the row "Idle in transaction" of `docs/architecture/reliability.md` §7.4 with the text of ADR-0192 (d). Replace "Validation runs before it" in the hard rules of `CLAUDE.md` with the same text. Red test: none, because the task changes documents. Check: `grep -n 'Validation runs before the transaction\|Validation runs before it' docs/architecture/reliability.md CLAUDE.md` prints nothing, and ste-lint on `docs/architecture/reliability.md CLAUDE.md` reports 0 hard violations.
- [x] 2.4 (docs/architecture) [mechanical] RCB-01, design D2, ADR-0191 (i). After 2.2, in the rule "Idempotent approve" of `docs/architecture/domain-model.md` §4 (line 636), replace the last sentence with the two sentences of ADR-0191 (i), word for word. Red test: none, because the task changes a document. Check: `grep -c 'gives its own code, also with a stale .row_version.. Under the locks, .Recipe::approve(). checks the deleted recipe (.E-RCP-012.), then the idempotent case' docs/architecture/domain-model.md` prints 1, and ste-lint on `docs/architecture/domain-model.md` reports 0 hard violations.
   - The new text: "The check of the draft before the transaction gives its own code, also with a stale `row_version`. Under the locks, `Recipe::approve()` checks the deleted recipe (`E-RCP-012`), then the idempotent case, then the row version, then the transition (`api.md` section 4)."

## 3. recipe-domain

- [x] 3.1 (recipe-domain) DOM-02, design D9. In `crates/recipe-domain/src/recipe/draft/approval/tests.rs`, add four tests: `a_time_leaf_without_a_limit_gives_one_e_rcp_016_entry`, `a_date_leaf_without_a_limit_gives_one_e_rcp_016_entry`, `a_char_leaf_without_a_limit_passes_the_limit_check`, and `a_wchar_leaf_without_a_limit_passes_the_limit_check`. Rename `bool_char_and_wchar_leaves_need_no_limit` to `a_bool_leaf_needs_no_limit`. Red test: none, because `needs_limit` has no defect. Mutation, in a separate copy (run order, rule 7): delete `LeafClass::Duration | LeafClass::Instant` from `needs_limit` once and see the two `TIME` and `DATE` tests fail. Add `LeafClass::Char` once and see the `CHAR` and `WCHAR` tests fail. Delete the copy. The shared tree keeps `approval.rs` unchanged. GREEN: `cargo test -p recipe-domain approval::tests`.

## 4. recipe-db trigger tests

- [x] 4.1 (recipe-db) DOM-01, design D9. In `crates/recipe-db/tests/storage/triggers/transition.rs`, add the four tests of D9 with an allowed transition: `transition_a_retire_cannot_change_the_content_hash`, `transition_a_retire_cannot_change_the_change_note`, `transition_a_draft_update_cannot_change_the_number`, and `transition_a_draft_update_cannot_change_the_creator`. Each asserts the trigger `TRANSITION` (`E-RCP-033`, SQLSTATE `23514`). In `crates/recipe-db/tests/storage/grants/app_role.rs`, add `app_a_retire_cannot_change_the_content_hash` as `recipe_app`. Red test: none, because the trigger has no defect. Mutation, in a separate copy (run order, rule 7), because a shipped migration never changes in the shared tree. Each mutation deletes one whole clause, so the function stays valid SQL:
  - Mutation A: delete lines 206 to 209 of `migrations/0002_triggers.sql`, from `       OR (OLD.status <> 'draft'` to `               (OLD.approved_by, OLD.approved_at, OLD.change_note, OLD.content_sha256))`. Line 205 `OR (OLD.status = 'approved' AND NEW.status = 'retired'))` is then followed by line 210 `OR (OLD.status = 'approved' AND NEW.deleted_at IS NOT NULL)`. See the two retire tests and `app_a_retire_cannot_change_the_content_hash` fail.
  - Mutation B: restore the file, then delete lines 199 to 203, from `       OR (NEW.id, NEW.recipe_id, NEW.plc_platform, NEW.number, NEW.based_on_version_id,` to `           OLD.created_by, OLD.created_at)`. Line 198 `OR OLD.status = 'retired'` is then followed by line 204 `OR NOT (...`. See the two draft tests fail.
  - Delete the copy.
  - GREEN (Docker): `cargo test -p recipe-db --test storage -- triggers::transition grants::app_role`.

## 5. recipe-app use cases

- [x] 5.2 (recipe-app) RCB-02, design D3, `export` delta. In `crates/recipe-app/src/export/machine_filter_tests.rs`, add `a_reused_machine_code_at_the_second_pick_gives_web_005_and_no_entry`. Use the hook `on_lock_version_shared` (ADR-0164 (h)) to approve the draft, archive the machine, and create a machine with the same code. Assert `E-WEB-005` and no `VersionExported` entry. RED: `cargo test -p recipe-app machine_filter_tests` fails, because the export writes version 4. Then apply D3 in `export/flow.rs`. GREEN: `cargo test -p recipe-app export::`.
- [x] 5.3 (recipe-app) PLT-01, design D4. In `crates/recipe-app/src/plant/define_udt/name_tests.rs`, add `insert_struct_row_names_the_repeated_member_for_each_unique_index`. The test calls `insert_struct_row` itself on a memory transaction, not `run` (D4, known limit). It runs twice: once after `fail_insert_struct(Fault::DuplicateMemberName)` and once after `fail_insert_struct(Fault::DuplicateMemberNameAnyCase)`. The input has the members `Alpha`, `Speed`, and `SPEED`, so the repeat is at index 2. Each run asserts `AppError::Plant(PlantError::DuplicateMemberName { member, .. })` with `member.as_str() == "SPEED"`. RED: `cargo test -p recipe-app define_udt` fails, because `insert_struct_row` names `Alpha`. Then apply D4 in `define_udt.rs` and its doc comment. `insert_struct_row` calls the free function of D4. GREEN: `cargo test -p recipe-app define_udt`. The two tests of the first member in `design.md` Context stay green.
- [x] 5.4 (recipe-app) [mechanical] PLT-02, design D5. After 5.3, rename the test `the_journal_holds_the_machine_lock_and_then_the_advisory_keys_in_the_order_of_the_texts` in `crates/recipe-app/src/plant/define_udt/tests.rs` to `the_journal_holds_the_machine_lock_and_then_the_advisory_key_set`. Add the sentence of D5 to the doc comment of `lock_derived_types` in `crates/recipe-app/src/memory/types/mod.rs`. Red test: none, because no behavior changes. Check: `rg -n 'in_the_order_of_the_texts' crates/recipe-app` prints nothing, and `cargo test -p recipe-app define_udt` passes.
- [x] 5.5 (recipe-app) QRY-01, design D9. Add one test per query that asserts `[JournalEntry::Begin(Isolation::ReadOnly)]`, as `only_read_only_transactions` of `identity/queries/tests.rs` does:
  - `list_audit_entries_runs_in_one_read_only_transaction` in `audit/queries/list_tests.rs`.
  - `get_audit_entry_runs_in_one_read_only_transaction` in `audit/queries/entry_tests.rs`.
  - `get_recipe_history_runs_in_one_read_only_transaction` in `audit/queries/history_tests.rs`.
  - `list_security_events_runs_in_one_read_only_transaction` in `audit/queries/security_event_tests.rs`.
  - `list_exportable_recipes_runs_in_one_read_only_transaction` in `export/list_exportable_recipes_tests.rs`.
  - `get_type_usage_runs_in_one_read_only_transaction` in `plant/queries/usage_tests.rs`.
  - Red test: none, because the six queries have no defect. Mutation, in a separate copy (run order, rule 7): change `Isolation::ReadOnly` to `Isolation::ReadCommitted` in one query file at a time and see its test fail. Delete the copy. GREEN: `cargo test -p recipe-app -- audit::queries list_exportable_recipes usage_tests`.

## 6. Memory adapter

- [x] 6.1 (recipe-app) CDX-01, design D6. In `crates/recipe-db/tests/storage/contract/recipe_write/guard.rs`, change the four `postgres_only_` cases to `contract_case!` cases on both adapters. Change the module comment. Add one `contract_case!` that takes `lock_draft` and then writes a status, and expects success on both adapters (D6). RED (Docker): `cargo test -p recipe-db --test storage contract::recipe_write::guard` fails for the four memory tests. Then apply D6 in `crates/recipe-app/src/memory/recipe/mod.rs` and in `lock_version`, `lock_versions`, `lock_draft`, and `touch_draft` of the memory adapter. The `lock_draft` case stays green after the fix. Change each `recipe-app` test that writes a status without a lock. GREEN: the RED command, then `cargo test -p recipe-app`.
- [x] 6.2 (recipe-app) DOM-03, design D8. After 6.1, add one contract case for each check of D8 in `crates/recipe-db/tests/storage/contract/draft/insert.rs`. Each case asserts the trigger code of `StorageError::Constraint` on both adapters. In `contract/draft/narrow_writes.rs`, add the two `replace_limits` cases of D8: `replace_limits_above_the_limit_row_budget_gives_e_rcp_036` and `replace_limits_counts_the_new_rows_and_not_the_old_rows_of_the_parameter`. RED (Docker): `cargo test -p recipe-db --test storage contract::draft` fails for the memory tests of `insert.rs` and for the memory test `replace_limits_above_the_limit_row_budget_gives_e_rcp_036`. Then apply D8 in `crates/recipe-app/src/memory/draft/mod.rs`: `check_parameter_references` and `MemoryTransaction::replace_limits`. The second `replace_limits` case is green before and after the fix. It fails a memory check that also counts the old rows. GREEN: the RED command, then `cargo test -p recipe-app`.
- [x] 6.3 (recipe-app) QRY-02, design D7. After 5.4, 6.1, and 6.2, add `a_write_in_a_read_only_transaction_gives_25006` and `a_write_in_a_repeatable_read_transaction_gives_25006` to `crates/recipe-app/src/memory/core/tests.rs`. Each test calls the write accessor and `append_audit` and asserts `StorageError::Other` with the SQLSTATE `25006` and `E-DB-008`. RED: `cargo test -p recipe-app memory::core` fails, because both calls succeed. Then apply D7 in `memory/core/transaction.rs` and its 31 call sites. GREEN: `cargo test -p recipe-app` and `cargo test -p recipe-db --test storage contract::` (Docker).

## 7. Changelog

- [x] 7.1 (CHANGELOG.md) [mechanical] Add one line under `[Unreleased]`, section `Fixed`, for RCB-02 (`reliability.md` §8). RCB-01 gets no line, because it changes no behavior (design D2). Red test: none, because the task changes a document. Check: ste-lint on `CHANGELOG.md`, and the line of RCB-02 holds `E-WEB-005`.

## 8. Integration checks

- [ ] 8.1 (.) Run the lint gate. Check: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] 8.2 (.) Run every test (Docker). Check: `cargo test --workspace --all-features --locked`
- [ ] 8.3 (.) Check the format and the docs. Check: `cargo fmt --check` and `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --all-features --locked`
- [ ] 8.4 (.) Check the specs and the STE scope. Check: `openspec validate semantic-review-fixes --strict` prints that the change is valid, and the ste-lint command of `CLAUDE.md` with `openspec/changes/semantic-review-fixes/**/*.md` reports 0 hard violations.

## Workflow follow-up

- Run `code-review` (Opus) after group 6. Fix each CRITICAL, HIGH, and MEDIUM finding.
- Archive with `openspec-archive-change` after group 8 passes.
