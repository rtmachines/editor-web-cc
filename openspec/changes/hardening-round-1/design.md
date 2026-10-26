# Design

## Context

See `proposal.md`, section "Why". The findings and their sources are in `target/p42-agents/security-findings.md` and `target/p42-agents/audit-opus.md`. The project owner fixed the severity and the direction of each finding before this design. Two plan reviews (`target/p42-agents/plan-review-opus.md`, `target/p42-agents/plan-review-codex.md`) changed this design.

Four new ADRs hold the decisions. This design links to them and does not repeat them:

| Finding | Decision | Amends |
|---|---|---|
| F1 globals file of the restore test | ADR-0095 | ADR-0067 (first risk), ADR-0078 (b) |
| F2 `SetUserRole` and `DeactivateUser` under a busy target | ADR-0096 | ADR-0015, ADR-0035 (known limit M7), ADR-0022 (retry sentence of the risks) |
| F3 drop guard after a rename of `public` | ADR-0097 | ADR-0034 |
| F6 insert columns of `audit_log`, F7 qualified operators | ADR-0098 | none |
| F4 permission check first | no ADR. The rule is in CLAUDE.md and ADR-0013 | none |
| F5 two transactions of `ChangeOwnPassword` | no ADR. ADR-0035 (b) already decides it | none |

## Goals / Non-Goals

**Goals:**

- Close F2 to F7 and narrow F1 with the smallest change that keeps every existing test meaningful.
- Make each new rule visible to a test: a stub case, a storage test, a race test, a unit test, or a source test.
- Change the normative documents before the code that follows them.

**Non-Goals:**

- A crafted entry of the dump file (ADR-0095 (h)). The project owner decides on it later. F1 is therefore narrowed, not closed.
- A change of `create-admin`. It stays `SERIALIZABLE` with one more run (ADR-0096 (h)).
- A new error code or a new setting. `reliability.md` sections 1.2 and 7.11 do not change.

## Decisions

### D1. Role grammar of the globals file (F1)

One file holds the grammar: `deploy/pgtools/lib/globals-grammar.sh` (ADR-0095 (i)). It holds the regular expressions of ADR-0095 (b) and the function `check_globals_file`. The function returns the fault and the line number. `check_globals` of `restore-test` sources the file and maps a fault to `fail`. The `Dockerfile` copies the file into the image. The path follows the pattern of `RESTORE_CODE_DIR` (`restore-test:74`), so the stub tests can use the file of the repository.

The order of the branches of ADR-0095 (a) is binding:

1. The restrict pair, then the backslash branch. The nine `META` cases of `restore-globals-cases.sh:41-49` keep their message.
2. The control-character check, also between the pair. A CR byte inside a comment stops the test (review finding HR1-01).
3. The comment, blank line, and grammar branch.

Three tests reach the grammar:

- The storage test `the_globals_file_of_pg_dumpall_has_only_the_restrict_pair_as_backslash_lines` (`restore_code.rs:253`) already runs the real `pg_dumpall` in the database container. Task 6.1 adds a test that runs `bash` in that container through `docker exec -i` and passes the grammar file and the real output to `check_globals_file`. The real output passes. Mutations of the real output fail: a line with a CR in a comment, two statements on one line, and `ALTER ROLE ... SET session_preload_libraries`.
- The stub cases of `restore-test-test` (task 6.2) feed fixed bad lines and the five pass forms. A case asserts that no PostgreSQL tool started.
- The stack test of CI runs the real `restore-test` on a real set (`deploy/ci/recovery.sh`).

The red phase of task 6.1: the test fails until task 6.2 writes the grammar file. The real output alone could pass at once, so a passing real file is not the red step. The red phase of task 6.2: the stub cases with fixed bad lines fail against the old `check_globals`.

Option B of ADR-0095 (an exact line list) needs the byte output of the tool in a file of the image. Option C (skip the file) loses the proof that the globals file restores.

### D2. Guard lock of class 4 (F2)

The port `UserRepository` of `recipe-app` gets the method `lock_administrator_guard`. The PostgreSQL adapter runs `SELECT pg_advisory_xact_lock(4, 0)`. The memory adapter writes an entry to its journal. It also gets the fault point `FaultPoint::LockAdministratorGuard` and the fault `Fault::LockTimeout` (`StorageError::Deadlock` with `LockFailure::LockTimeout`). `admin_change::run_admin_change` opens `Isolation::ReadCommitted`, calls the guard first, and drops `serializable_once_more`. `retry.rs` stays for `bootstrap_administrator`.

Why a new class and not the row locks: ADR-0096, options B and D. Lock order: ADR-0096 (e).

Task 3.2a rewrites the race tests of `crates/recipe-db/tests/storage/race/identity.rs`. They fail against the old code:

| Race | Old assertion | New assertion |
|---|---|---|
| 3, two demotions (`identity.rs:150`) | both commands reach the gate of the `UPDATE` | the second command waits for the class-4 lock (`pg_locks`, `classid = 4`). Then it returns `E-IDN-002` |
| 4, touch during `DeactivateUser` (`identity.rs:201`) | two runs | one run, no `E-DB-001`, no session after the commit |
| new, touch during `SetUserRole` | none | one run, no `E-DB-001`, no session after the commit |
| new, touch after the delete | none | the touch waits for the commit. Then it finds no session |
| new, three Administrators, two deactivations | none | the second command waits, then commits. One active Administrator stays |
| 5, failure at the commit (`identity.rs:312`, `:323`) | one more run | deleted. The commands have no retry |

Task 3.2b changes the code and the unit tests of `recipe-app`:

- `set_user_role/tests.rs`: `:65` (one `SERIALIZABLE` run), `:201`, `:216`, `:232`, `:247` (second run after `40001`, `E-DB-001` after two), `:264` (deadlock, one run).
- `deactivate_user/tests.rs`: `:64`, `:225`, `:245`, `:261`.
- `fixtures.rs:56`: `serializable_runs` stays for the bootstrap tests. A new counter counts `Begin(Isolation::ReadCommitted)`.
- New assertions: one `ReadCommitted` run, no second run after a `Serialization` fault, and the guard as the first port call after `authorize`.
- New test: the fault `LockTimeout` at `LockAdministratorGuard` gives `E-DB-001`, no write, no audit entry, and one `Begin`.
- Doc comments: `set_user_role.rs:25`, `deactivate_user.rs:16`, `admin_change.rs:15`, `ports/isolation.rs:11`, and `recipe-domain/src/identity/last_administrator.rs:29`.
- The tests of `bootstrap_administrator` and of `retry.rs` keep their retry assertions.

A red test between 3.2a and 3.2b is normal for two TDD tasks. Task 3.2b turns both crates green.

### D3. Drop guard by name (F3)

Task 0.1 runs the negative test of ADR-0097 (f) before ADR-0097 becomes Accepted. When it passes before the change, the finding is false, and the change stops for F3. The function of `01-roles.sh` reads the last element of `address_names` and the object type (ADR-0097 (a) to (c)). The test `the_guard_list_equals_the_objects_that_audit_owner_owns` then compares bare names. A document test keeps the SQL block of `data-model.md` section 5.1 equal to the function of `01-roles.sh`, like `migration_0006_equals_the_sql_block_of_section_5_5` (`operations_schema.rs:87`).

The upgrade of an older volume follows ADR-0097 (h) to (h4). The guard step runs after `migrate` exits with code 0 and before `backup-now`. A check compares the SHA-256 of the function with field 4 of line 74 of the new `restore-code.expected`. A CI job upgrades a volume of the previous release and restores its rollback set (task 4.4). Task 4.3 writes the order into the runbook and into `reliability.md` section 7.10.

### D4. Source test of the identity use cases (F4)

`src/audit/queries/source_tests.rs` already checks that each `execute` body starts with `authorize(Self::OPERATION`. A copy for `src/identity/` lists the use case files of the module. `login.rs` is the one exception, with the first statement `require_no_session(Self::OPERATION)`. The test fails on `change_own_password.rs` and `logout.rs` until the call moves.

### D5. Two migrations of the audit objects (F6, F7)

`0007_audit_definer_operators.sql` and `0008_audit_log_insert_columns.sql` follow ADR-0098 (a) to (c) and the five steps of `data-model.md` section 5.1. Each migration has its own task. Each migration equals the SQL block of a new section of `data-model.md` (5.6 and 5.7), like `0006` and section 5.5.

- `0007` keeps the grouping of each expression with parentheses (ADR-0098 (a)). It repeats `SECURITY DEFINER` and the `SET search_path` clause.
- The scan of ADR-0098 (f) reads `prosrc` of every function that `audit_owner` owns. It skips comments, string literals, `$$`, `::`, `:=`, `=>`, the `=` of `GET DIAGNOSTICS`, and each `OPERATOR(pg_catalog.…)` group.
- After `0007`, a test asserts `prosecdef`, `proconfig`, the owner, and the ACL of both functions (ADR-0098 (g)).
- The purge tests `maintenance_event_purge_deletes_batches_of_5000_and_keeps_recent_rows` and `maintenance_a_failure_in_the_second_event_batch_keeps_the_first_batch` stay (`maintenance.rs`). Task 5.1 adds a boundary case at 399 and 401 days when no test covers it.
- `0008` changes the I13 tests as ADR-0098 (i) gives. The test at `hash_chain.rs:206` becomes a `42501` test. A new superuser test keeps the proof that trigger 4.9 overrides forged chain columns.

### D6. Task agents

| Tasks | Agent | Reason |
|---|---|---|
| 0.1, 2.1, 3.1, 3.2a, 3.2b, 4.1a, 5.1, 5.2, 6.1, 7.2 | Sonnet, effort high | Rust and SQL with tests, and the verification gate (memory rule: code and tests on Sonnet) |
| 4.1b, 4.4, 6.2 | Codex (`GPT6.1-Sol`) | `01-roles.sh`, `restore-code.expected`, CI, `restore-test`, and the image (memory rule: deploy tasks on Codex) |
| 4.3 | Sonnet, effort high | A new runbook procedure, not a fixed text |
| `[mechanical]` tasks 1.1, 1.2, 2.2, 3.3, 3.4, 4.2, 5.3, 6.3, 7.1 | Haiku, effort medium | Document edits with a text that this design or the ADRs fix |
| 8.1, 8.2, 8.3 | Sonnet, effort high | One STE check after each wave of Haiku documents (memory rule: STE checks on Sonnet) |
| Code review after group 7 | Opus and Codex in parallel | Review only. No Opus agent writes code |

Each code agent gets exactly one task. A Haiku agent runs no `ste-lint.py`. A Haiku agent applies the exact lines of the section "Exact new lines for instruction files" and changes no word of them.

### D7. Order of the tasks

- Wave A runs first. Task 0.1 can stop the F3 part of the change before any ADR becomes Accepted.
- Wave B sets the ADRs to Accepted and adds the "Amended by" notes. Task 8.1 checks them.
- Wave C changes the normative documents right after wave B, before the code: tasks 2.2, 3.3, 3.4, 5.3, 6.3, and 4.3. They share `data-model.md`, `reliability.md`, and `sql.md`, so they run one after the other. Task 8.2 checks them.
- Task 4.2 copies the code text of task 4.1b, so it runs after 4.1b.
- Tasks 4.1b and 5.1 both edit `deploy/pgtools/restore-code.expected`. They run one after the other, 4.1b first. Task 5.2 runs after 5.1, because both add a migration and a section of `data-model.md`.
- The pairs 3.2a and 3.2b, 4.1a and 4.1b, and 6.1 and 6.2 run one right after the other. A red test between the two tasks of a pair is normal.
- Task 4.4 runs after 4.2 and 5.2, because the upgrade job needs every migration.
- At most two cargo agents run in parallel. Agents that share a file run one after the other.

## Risks / Trade-offs

- [The grammar misses a line of `pg_dumpall`] → The storage test of D1 fails. The stack test of CI fails too.
- [A comment hides SQL behind a CR byte] → The control-character check runs inside the pair, before the comment check (D1).
- [Tasks 3.2a and 3.2b leave the workspace red between them] → No other task runs between the two tasks. Task 3.1 adds the port method and the fault point first.
- [`address_names` of a dropped function has another form than expected] → The negative test of task 4.1a and the existing drop tests fail.
- [An older volume keeps the old guard] → `restore-code.expected` then differs from the restore, and the restore test stops. The upgrade order of ADR-0097 (h) and its hash check prevent this.
- [`0007` loses `SECURITY DEFINER` or changes a grouping] → The tests of ADR-0098 (g) and (h).
- [Two versions of `app` run at the same time] → The old one takes no class-4 lock. The runbook rule of ADR-0096 forbids two `app` containers.

## Migration Plan

1. Task 0.1 shows that the F3 attack works. The project owner approves `tasks.md`. Task 1.1 sets the four ADRs to "Accepted".
2. The release adds `0007` and `0008`. The release notes name the manual superuser step of ADR-0097 (h).
3. The upgrade of an older volume runs in the order of ADR-0097 (h):
   1. The restore test of the old image checks the newest set.
   2. `backup-premigrate` writes the rollback set. The operator waits for exit code 0.
   3. `migrate` runs with `--no-deps`. The operator waits for exit code 0.
   4. The superuser replaces the guard function and checks its SHA-256 against the new `restore-code.expected`.
   5. The new `app` starts. The new image of `recipe-pgtools` replaces the old one (ADR-0083).
   6. `backup-now` writes a new set, and the restore test of the new image checks it.
4. A failure between two steps follows the recovery list of ADR-0097 (h2).
5. Rollback: a forward migration or a restore (`reliability.md` section 5). `0008` has no destructive step.

## Open Questions

Each question waits for the project owner. The recommendation comes from the plan review.

| # | Question | Recommendation |
|---|---|---|
| 1 | ADR-0095 (h): a crafted TOC entry still runs SQL in `pre-data` | Keep it out of this change. The CHANGELOG says that F1 is narrowed, not closed. Open the follow-up ADR now. Candidate: `pg_restore --file` and a comparison with a `restore-schema.expected` that CI generates |
| 2 | ADR-0078 line 56: clarify the scope or withdraw the sentence | Clarify the scope only (ADR-0095 (g)). The sentence is true for a stop on a refused kind |
| 3 | `data-model.md` section 1.1 "names" wording | Use the text of ADR-0098 (e) |
| 4 | Task 5.1 stops when another `audit_owner` function fails the scan | Keep the stop rule for a scanner fault. Pre-authorize: a real unqualified operator in another `audit_owner` function goes into `0007`, and the agent reports it |
| 5 | The F3 negative test has not run on PostgreSQL | Run it first as task 0.1. Keep the stop rule |
| 6 | `create-admin` stays `SERIALIZABLE` | Keep it. A later change can take class 4 and delete `retry.rs` |
| 7 | Tasks 2.2, 3.4, 5.3, and 6.3 edit CLAUDE.md, `.claude/rules/sql.md`, and `AGENTS.md` | Approve the exact lines of the next section with `tasks.md`. A subagent message does not authorize these edits |

## Exact new lines for instruction files

The project owner approves these lines with `tasks.md`. The line numbers are of the current files. A task applies each line without a change of wording. The line numbers shift after an insert in the same file, so each task finds its line by the old text.

### CLAUDE.md, line 59 (task 2.2)

```text
OLD: - A command runs in one transaction (`Login` uses T1 and T2). It locks rows in the order `machine` → `recipe` → `recipe_version` → `unit` → children (ADR-0015).
NEW: - A command runs in one transaction. `Login` and `ChangeOwnPassword` use the two transactions T1 and T2 (ADR-0035 (b)). A command locks rows in the order `machine` → `recipe` → `recipe_version` → `unit` → children (ADR-0015).
```

### .claude/rules/sql.md, line 14 (task 5.3)

```text
OLD: - Every function sets `search_path = pg_catalog, public, pg_temp`. A `SECURITY DEFINER` function also schema-qualifies every name.
NEW: - Every function sets `search_path = pg_catalog, public, pg_temp`. A `SECURITY DEFINER` function also schema-qualifies every table, function, operator, and type name. A type keyword such as `BIGINT` needs no schema (ADR-0098).
```

### .claude/rules/sql.md, new line after line 31 (task 6.3)

```text
NEW: - A new role statement in `01-roles.sh` needs a form of the role grammar in `deploy/pgtools/lib/globals-grammar.sh` (ADR-0095).
```

### .claude/rules/sql.md, line 42 (task 2.2)

```text
OLD: - One transaction per command, except `Login` (T1 and T2). Lock order: `machine` → `recipe` → `recipe_version` → `unit` → children.
NEW: - One transaction per command, except `Login` and `ChangeOwnPassword` (T1 and T2, ADR-0035 (b)). Lock order: `machine` → `recipe` → `recipe_version` → `unit` → children.
```

### .claude/rules/sql.md, new line after line 43 (task 3.4)

```text
NEW: - An identity command locks in the order class-4 lock → `app_user` → `session` → `audit_chain_head`. Only `SetUserRole` and `DeactivateUser` take the class-4 lock (ADR-0096 (e)).
```

### .claude/rules/sql.md, line 48 (task 3.4)

```text
OLD: - Advisory locks use `pg_advisory_xact_lock(class, key)`. Class 1 is `Login` T1. Class 2 is the class of derived types. No code takes class 3.
NEW: - Advisory locks use `pg_advisory_xact_lock(class, key)`. Class 1 is `Login` T1. Class 2 is the class of derived types. No code takes class 3. Class 4 with the key 0 is the guard lock of the last Administrator (ADR-0096).
```

### .claude/rules/sql.md, line 51 (task 3.4)

```text
OLD: - Isolation is `READ COMMITTED` with no retry. `SetUserRole` and `DeactivateUser` run `SERIALIZABLE` and retry once on `40001`.
NEW: - Isolation is `READ COMMITTED` with no retry. Only `create-admin` runs `SERIALIZABLE` and runs once more on `40001` (ADR-0035 (h), ADR-0096).
```

### AGENTS.md, line 32 (task 2.2)

```text
OLD: | critical | A security fault, a loss of data, a broken audit hash chain, or a leaked secret. Also a command that writes in more than one transaction |
NEW: | critical | A security fault, a loss of data, a broken audit hash chain, or a leaked secret. Also a command that writes in more than one transaction, except `Login` and `ChangeOwnPassword` (ADR-0035 (b)) |
```

### AGENTS.md, line 43 (task 2.2)

```text
OLD: - A command runs in one transaction and writes its audit entries through `UnitOfWork::commit` (ADR-0015).
NEW: - A command runs in one transaction and writes its audit entries through `UnitOfWork::commit` (ADR-0015). `Login` and `ChangeOwnPassword` use two transactions (ADR-0035 (b)).
```

### AGENTS.md, new line after line 48 (task 6.3)

```text
NEW: - A new role statement in `deploy/db/initdb/01-roles.sh` has a form of the role grammar in `deploy/pgtools/lib/globals-grammar.sh` (ADR-0095).
```

The prefixes `OLD: ` and `NEW: ` are not part of the line.
