# Design

## Context

`proposal.md` gives the motivation. `docs/plan/review/2026-11-17-semantic-review.md` §3 and §4 give the location, the rule, and the failure scenario of each finding and doc conflict. §5 there gives the owner decisions. §7 there gives the later decision on RCB-01 after plan review R3. The design read the code of each finding at HEAD `77f5112`:

- RCB-01: `read_root` in `crates/recipe-app/src/recipe/approve_version/prepare.rs` runs `approval_replay`, then `check_approvable`. The caller then reads the snapshot and runs `check_and_hash` in `spawn_blocking`. The compare of `expected_row_version` runs only in the write transaction.
- RCB-02: `pick_lock_and_write` in `crates/recipe-app/src/export/flow.rs` calls `find(ctx, target, machine)` for the first pick and `find(ctx, target, None)` for the second pick.
- PLT-01: `insert_struct_row` in `crates/recipe-app/src/plant/define_udt.rs` maps `PLC_TYPE_MEMBER_NAME_UNIQUE | PLC_TYPE_MEMBER_NAME_CI_UNIQUE` with `input.members.first()`. Two tests pin the first member: `the_index_plc_type_member_name_ci_unique_gives_plt_021_for_the_first_member` (`define_udt/name_tests.rs`) and `the_constraint_plc_type_member_name_unique_gives_plt_021_with_the_struct_and_member` (`define_udt/tests.rs`).
- PLT-02: `lock_derived_types` in `crates/recipe-app/src/memory/types/mod.rs` journals one line for each key text in input order.
- CDX-01: `change_version` in `crates/recipe-app/src/memory/recipe/mod.rs` checks the state and bumps the row. It has no record of version locks. `PgTransaction::guard` in `crates/recipe-db/src/transaction/recipes.rs` reads `recorded_row_version` and gives `E-DB-008` without a lock.
- QRY-02: `MemoryTransaction::state_mut` in `crates/recipe-app/src/memory/core/transaction.rs` has 31 call sites in 11 files. Only `finish` refuses a read-only level. `crates/recipe-db/tests/storage/unit_of_work.rs` shows the PostgreSQL error: `StorageError::Other` with the SQLSTATE `25006` and the code `E-DB-008`.
- DOM-01: each frozen-column test in `crates/recipe-db/tests/storage/triggers/transition.rs` updates an approved version and keeps its status. `migrations/0002_triggers.sql` has the clauses at lines 199 to 203 and 206 to 209.
- DOM-02: `needs_limit` in `crates/recipe-domain/src/recipe/draft/approval.rs` names `Numeric`, `String`, `Duration`, and `Instant`. The test `bool_char_and_wchar_leaves_need_no_limit` builds one `BOOL` parameter.
- DOM-03: `check_parameter_references` in `crates/recipe-app/src/memory/draft/mod.rs` checks the unit, the type, the tag, and the key only.
- QRY-01: the test files of the six queries hold no assertion on `JournalEntry::Begin`. `only_read_only_transactions` in `crates/recipe-app/src/identity/queries/tests.rs` is the pattern.

## Goals / Non-Goals

**Goals:**

- Each code fix keeps the rules of `reliability.md` §1 and §2.
- Each code task starts with a test that fails on the defect. A task that only adds a test names the mutation that makes its test fail.
- The two adapters give the same result for each case of the contract suite that this change touches.

**Non-Goals:**

- No migration, no route, no setting, and no new error code.
- No restructure of the checks of the locked state (ADR-0192, option B).

## Decisions

### D1. Acceptance of the three ADRs

- ADR-0190, ADR-0191, and ADR-0192 have the status `Proposed`. Each "Amended by" note that names them carries the mark "(proposed)".
- Task 1.0 sets the status `Accepted` after the owner approves `tasks.md`. It follows task 1.1 of the archived change `2026-11-16-backup-manifest-hmac`.
- The change has three ADRs, because the three doc conflicts are three decisions. DC-1 is the scope of an ADR item. DC-2 is the content of the operation rows. DC-3 is a rule of `reliability.md`.
- ADR-0191 also holds RCA-01 and RCB-01. RCA-01, RCB-01, and DC-2 share one decision: the normative text follows the shipped code. ADR-0168 joined three such corrections in the same way.

### D2. Order of the approval checks (RCB-01): a doc fix

- The owner decided on 2026-11-17 after plan review R3 (R3-01). The accepted requirement "Order of the approval errors" of the `web-ui` spec and the code state the intended order.
- That requirement states: the check of the draft before the transaction gives its own code, also with a stale `row_version`. Under the locks, the `row_version` compare comes before the second check.
- The code follows it. `read_root` in `approve_version/prepare.rs` runs `approval_replay`, then `check_approvable`. The compare runs only in the write transaction. The row `ApproveVersion` of `data-model.md` §6 gives the same order.
- Under the locks, the order of execution and the precedence of the answer differ. `approve_in_transaction` in `approve_version/write.rs` runs `recheck` before `Recipe::approve()` compares the `row_version`.
  - It returns the domain fault of `recheck` (`E-RCP-016`, `E-RCP-024`) only after `Recipe::approve()` gives `Approved`. A read fault of the type rows can return before the compare.
  - The precedence of the answer starts with the deleted recipe (`E-RCP-012`), the idempotent case, the row version, and the transition.
  - Then come the row version of the token, the separation of duties, and the fault of `recheck`.
  - Known note: line 1717 of `openspec/specs/web-ui/spec.md` says that the compare comes "before the second check". That word names the precedence of the answer, not the order of execution. The change keeps that wording and changes no code.
- The rule "Idempotent approve" of `domain-model.md` §4 (line 636) is too general. It gives one order for all checks. ADR-0191 (i) replaces its last sentence. Task 2.4 applies the text.
- RCB-01 therefore changes no code, no test, no spec, and no `CHANGELOG.md` line.
- Alternative: compare the `row_version` in the read of the root before `check_approvable` (the plan up to R2). Rejected, for two reasons:
  1. It contradicts the accepted `web-ui` requirement and its scenario "Stale row version and an outdated parameter" (`422 E-RCP-024`).
  2. Three test files in `crates/recipe-web/src/` expect a state code for a stale approval: `pages/approval/approve_error_tests.rs`, `pages/recipe_sweep_tests.rs`, and `pages/versions/route_tests.rs`.
- Codex R3-01 and R3-02 review that alternative. They close with it.

### D3. Machine filter of the second pick (RCB-02)

- The second call of `find` gets the same `Option<MachineCode>` as the first call. `MachineCode` is `Copy` (`machine_code.rs` line 12), so both calls get `machine` by copy. The code calls no `clone`, because `clippy::clone_on_copy` fails task 8.1.
- The `export` delta adds the filter and one scenario to "Second pick of the approved version".

### D4. Name of the repeated member (PLT-01)

- A free function in `define_udt.rs` returns the first member whose name equals the name of an earlier member under `str::to_lowercase`. `insert_struct_row` names that member.
- When no member repeats, the map keeps the first member. Today the domain check finds each repeat first, so only an index without a repeat in the request reaches this branch. The two existing tests stay green.
- The test calls `insert_struct_row` itself, not only the free function. A memory catalog with a forced fault of each unique index refuses the insert. The input has the repeat at index 2. The test asserts the field `DuplicateMemberName.member`.
- Known limit: no test can reach the new branch through `run`. `MemberName` accepts only ASCII, and the domain check finds every case-insensitive repeat of ASCII names before the insert. The direct call of `insert_struct_row` is therefore the only regression test of this map.

### D5. Claim of the memory lock journal (PLT-02)

- The test of `define_udt/tests.rs` gets a name that says "the key set after the machine lock", not "the order".
- The doc comment of `lock_derived_types` in `memory/types/mod.rs` says that only `recipe-db` proves the order of ADR-0144.
- Alternative: sort by a stand-in of `hashtext`. Rejected, because a stand-in proves nothing about PostgreSQL.

### D6. Version guard in the memory adapter (CDX-01)

- `MemoryTransaction` records the `row_version` of each version that it locks, as `PgTransaction` does: `lock_version`, `lock_versions` (all versions of a recipe), `lock_draft`, and `touch_draft`. `lock_draft` is in the list, because `recipe-db` records there too (`transaction/drafts.rs`, "Lock the version and record the `row_version`"). Without it, a status write after `lock_draft` passes on PostgreSQL and gives `E-DB-008` in memory. `lock_version_shared` records nothing on both adapters.
- A status write (`discard`, `retire`, `approve`) without a record gives the error of `without_lock` in `recipe-db`: `E-DB-008`. A record that differs from the row gives the same error. A status write updates the record, as `PgTransaction::keep` does.
- The four cases of `crates/recipe-db/tests/storage/contract/recipe_write/guard.rs` move to `contract_case!` and lose the prefix `postgres_only_`. The module comment names both adapters.
- One new `contract_case!` takes `lock_draft` and then writes a status. It passes on both adapters.
- A memory-backed test of `recipe-app` that writes a status without a lock then fails. The task changes such a test to take the lock first.

### D7. Read-only memory transactions (QRY-02)

- A fallible accessor replaces `state_mut`. For `ReadOnly` and `RepeatableRead`, it gives `StorageError::Other` with the SQLSTATE `25006`, as PostgreSQL does. `append_audit` gives the same error for these levels.
- A test of the memory store asserts the error for both levels and for both calls.
- D6 and D8 edit callers of `state_mut`, so D7 comes after them (`tasks.md`, run order).

### D8. Parameter triggers in the memory adapter (DOM-03)

- `check_parameter_references` adds four checks. Each gives its trigger code as `StorageError::Constraint`:
  1. A unit of another machine gives `E-RCP-032`.
  2. A type of another platform or machine gives `E-PLT-031` or `E-PLT-032`.
  3. An address that breaks `parameter_s7_address_platform` gives the code of that constraint.
  4. A budget overflow gives `E-RCP-035` or `E-RCP-036`.
- `MemoryTransaction::replace_limits` (`memory/draft/mod.rs` lines 242 to 251) also checks the limit budget. On PostgreSQL this path runs `DELETE` and `INSERT parameter_limit` (`transaction/draft_writes.rs` line 263), and the statement trigger `parameter_limit_size_budget` (`0002_triggers.sql` line 357) counts the rows after the insert. The memory check therefore counts the limit rows of the other parameters of the version plus the new rows. The old rows of the parameter do not count.
- One contract case for each code in `crates/recipe-db/tests/storage/contract/draft/` runs on both adapters.
- Two more contract cases in `contract/draft/narrow_writes.rs` run on both adapters. (1) A `replace_limits` call that brings the version above 32 768 limit rows gives `E-RCP-036`. (2) A `replace_limits` call that gives exactly 32 768 rows after it subtracts the old rows of the parameter passes. The old rows plus the new rows exceed 32 768 in case (2).

### D9. Tests that pin rules (QRY-01, DOM-01, DOM-02)

- QRY-01: one test per query runs it on the memory store and asserts `[JournalEntry::Begin(Isolation::ReadOnly)]` after the mark of the fixture.
- DOM-01: four negative tests with an allowed transition. (1) `approved -> retired` with a new `content_sha256`. (2) `approved -> retired` with a new `change_note`. (3) A draft that stays a draft with `number = 2`. (4) A draft that stays a draft with another `created_by`. Each asserts `E-RCP-033` and SQLSTATE `23514`. Test (1) also runs as `recipe_app` in `grants/app_role.rs`.
- DOM-02: a `TIME` leaf and a `DATE` leaf without a limit each give one `E-RCP-016` entry. A `CHAR` leaf and a `WCHAR` leaf without a limit pass `check_limits`. The current test `bool_char_and_wchar_leaves_need_no_limit` becomes `a_bool_leaf_needs_no_limit`.

### D10. Text of the documents (RCA-01, DC-1, DC-2, DC-3)

- RCA-01 and DC-2: ADR-0191 items (a) to (h).
- RCB-01: ADR-0191 item (i) (D2).
- DC-1: ADR-0190. The notes on ADR-0035, ADR-0172, and the decision map exist with the mark "(proposed)".
- DC-3: ADR-0192 items (d) and (e).

### D11. Spec deltas

- Only RCB-02 changes a behavior that a requirement states. It gets a MODIFIED requirement with a new scenario, as change `2026-11-14-review-code-fixes` did for F-08.
- RCB-01 changes no behavior (D2). The accepted requirement "Order of the approval errors" of the `web-ui` spec already states the order of the code.
- The other findings change a test, a test double, or a document. No requirement changes for them.

## Risks / Trade-offs

- [D6 and D7 change the memory adapter that many tests use] → Each task runs `cargo test -p recipe-app`. It changes only the tests that wrote without a lock or wrote in a read-only level. A changed test names the reason in its body.
- [A mutation changes non-test code or a migration] → Tasks 3.1, 4.1, and 5.5 run each mutation in a separate worktree or copy. It never runs in the shared working tree (`tasks.md`, run order).
- [D4 keeps the first member when no member repeats] → The branch is unreachable today. The doc comment names the fallback.
- [DOM-01 tests need Docker] → The `recipe-db` tests start their own container (ADR-0101).

## Migration Plan

No migration. The change ships with the next release. A rollback is a revert of the code commits. The ADRs stay.
