# Design

## Context

See `proposal.md`, section "Why". `docs/plan/review/2026-11-04-full-repo-review.md` holds each finding, its evidence, and its fix direction. ADR-0130 to ADR-0133 hold the four new decisions. This design links to them and does not repeat them.

The state of the code at HEAD `f7bd29a`. The author read each place in the source:

| Finding | State | Place |
|---|---|---|
| D1-2 | Three exhaustive matches give the requirement, the name, and the command flag of each `Operation` | `crates/recipe-app/src/auth/operation/requirement.rs` line 41, `mod.rs` lines 241 and 315 |
| D1-3 | One body of 65 lines holds the guard lock, the second permission check, the target lock, three target checks, the writes, and the audit entry | `crates/recipe-app/src/identity/admin_change.rs` line 40 |
| D1-4 | `execute` returns the error of `security.record(event)` after the commit of `delete_attempts`. Its doc comment cites ADR-0035 (j) | `crates/recipe-app/src/identity/unlock_login.rs` line 65 |
| D1-4 | The handler shows the message `user-unlocked` | `crates/recipe-web/src/pages/users.rs` line 419 |
| D2-2 | `SystemDiskSpace` awaits `spawn_blocking(read_free_space)` with no limit. It is `Copy` and has no field | `crates/recipe-server/src/disk.rs` line 24, built in `serve/process.rs` |
| D2-3 | `AuditWriteFailed { cause }`, `WebError::Fields { failure }`, and `MissingBasicCredentials` (`E-WEB-012`) are missing from `reliability.md` §1.1 | `crates/recipe-app/src/error/app.rs` line 140, `crates/recipe-web/src/error.rs` lines 169 and 235 |
| D3-1 | `token_kind` gives `TokenKind::Either` for `Access::Lang` | `crates/recipe-web/src/csrf.rs` line 83 |
| D4-3 | `DELETE_SESSIONS`, `PURGE_ATTEMPTS`, `DELETE_FOR_LOGIN`, and `DELETE_ALL_OF_USER` lock rows in scan order | `crates/recipe-db/src/maintenance.rs`, `transaction/identity/attempts.rs`, `transaction/identity/sessions.rs` |
| D5-3 | `logout` takes no `Form`. Every other `POST` handler takes `Result<Form<_>, FormRejection>` | `crates/recipe-web/src/pages/own.rs` line 161 |
| D5-4 | The bundles have no key `error-E-AUD-003`. A test pins the English fallback | `crates/recipe-web/i18n/`, `crates/recipe-web/src/error/tests.rs` line 603 |
| D5-5 | `change_password` stops at the first bad field. A policy fault of the new password gives `WebError::App`. The users forms already collect lines with `FieldLines` | `crates/recipe-web/src/pages/own.rs` line 204, `pages/users.rs` line 260 |
| D5-6 | `design-system.md` §1 assigns no file to sections 10.12 to 10.14 and to the multi-line field | `docs/architecture/design-system.md` line 19, `static/components.css` line 258 |
| D6-8 | The test asserts `old.is_err()` only | `crates/recipe-db/tests/storage/reset_password.rs` line 58 |

## Goals / Non-Goals

Goals:

- Close the 12 findings of group C with the owner decisions of 2026-11-05.
- Keep each fix inside its crate and its files, so that one agent does one task.

Non-Goals:

- The findings of group B.
- A new error code, a new setting, a migration, or a new dependency.

## Decisions

### D1. Unlock with a lost event (ADR-0130)

1. `UnlockLogin::execute` returns a result type of the use case. It tells "event recorded" or "event lost" and holds the `StorageError` of a lost event. An error before or of the commit stays `Err`.
2. The handler `unlock` of `pages/users.rs` writes the one `WARN` line of a lost event. The line holds `error_code` and `request_id`, as the fields of `log_error` in `src/error/log.rs`. The success answer does not change.
3. The test of `recipe-app` uses a security sink that fails. It asserts the result "event lost", no login attempt row, and one entry `LoginUnlocked`. The HTTP test asserts the success answer and one `WARN` line.

### D2. Time limit of the disk measurement (ADR-0133)

1. `SystemDiskSpace` gets the limit and a set of stalled paths: `Arc<parking_lot::Mutex<HashSet<PathBuf>>>`. It stops being `Copy`. `serve/process.rs` builds one adapter for the jobs `disk_check` and `alerts` and passes `health.disk_check_interval`. `recipe-server` adds `parking_lot = { workspace = true }`. The set is a new place of interior mutability in `reliability.md` §8.
2. A path enters the set only when its call reaches the limit. Calls that have not reached the limit run side by side, so two jobs that measure `pgdata` at the same time both get a reading.
3. `free_space` returns `DiskFault::Unreadable` at once when the path is in the set. Otherwise it spawns the call.
4. `tokio::time::timeout` wraps the await of the join handle. An elapsed wait gives `DiskFault::Unreadable` with `io::ErrorKind::TimedOut`.
5. Each call has a state `Running` or `Done` that the blocking closure and the timeout branch share. Both sides change the state and the set under the one mutex. The timeout branch adds the path only while the state is `Running`. The closure sets `Done` and takes the path out of the set. Thus a call that ends at the limit leaves no mark.
6. The reader function is a parameter of a private constructor, so a unit test can give a reader that blocks on a channel. The tests use a real limit of some milliseconds and no paused clock.

### D3. Own password form (ADR-0132)

1. `change_password` parses `current_password` into `Password` and `password` into `NewPassword`. Each parse runs, and each fault adds its own line. A refused form renders the own password form again with all lines, like `refuse_create`. The form never sends a password back into an input.
2. A new constructor `FieldError::identity(error, target)` keeps the code of an `IdentityError`, like `FieldError::recipe`. A policy fault of the new password gives a line `E-IDN-006` under `password`. A length fault of `Password` keeps the line `E-WEB-008`.
3. `ChangeOwnPasswordInput.new_password` gets the type `NewPassword`. The use case does not parse it again (`reliability.md` §4.3). A policy fault still stops the command before the permit and T1, so it writes no attempt row.
4. `AppError::Identity(IdentityError::InvalidCurrentPassword)` of `ChangeOwnPassword` gives `WebError::Fields` with one line under `current_password`. The status stays `422 E-IDN-008`.
5. The login form does not change.

### D4. Multi-row deletes in `id` order

1. Each statement gets a CTE that selects the ids with `ORDER BY id FOR UPDATE`. The `DELETE` then deletes the rows `WHERE id IN (SELECT id FROM locked)`.
2. `PURGE_ATTEMPTS` keeps its batch of the oldest rows. The inner query picks the batch by `attempted_at`, and the lock step orders that batch by `id`.
3. `DELETE_FOR_LOGIN` keeps its `RETURNING` and the newest rows of the audit entry.
4. A row lock needs an `UPDATE` grant on a column of the table (`data-model.md` §5.2). The task checks `grants.expected` for `session` and `login_attempt` and stops if a grant is missing.
5. The tests assert the SQL shape and the same deleted rows as before.

### D5. Small fixes

1. D1-2: one `macro_rules!` table gives each variant with its requirement, its name, and its command flag. The macro generates the enum `Operation` and the three accessors `requirement`, `name`, and `is_command`. No hand-written function holds a match of 69 arms, so each function stays under 50 lines (AGENTS.md). The macro uses no indexing and no `unwrap`. A table test pins the three facts of every variant before the refactor.
2. D1-3: lines 67 to 87 of `admin_change.rs` become one free function. The lock order of ADR-0096 (e) does not change.
3. D3-1: no code change. `api.md` §7.3 gets the sentence of ADR-0131 (a).
4. D5-3: `logout` takes `Result<Form<TokenFields>, FormRejection>`, as `deactivate` of `pages/users.rs` does. A JSON body gives `422 E-WEB-008`.
5. D5-4: both bundles get `error-E-AUD-003` with the sentence of the row in `reliability.md` §1.2. The test at `src/error/tests.rs` line 603 then asserts the Polish sentence.
6. D6-8: the test matches the code of a failed Basic check, not only `is_err()`.

### D6. Documents

| Document | Change | Source |
|---|---|---|
| `domain-model.md` §5 | The row of `SecurityEventSink::record` says that a lost event `unlock` does not fail `UnlockLogin` | ADR-0130 |
| `data-model.md` §6 | The row `UnlockLogin` says that a lost event gives the result "event lost" and no error. The row `ChangeOwnPassword` says that `recipe-web` parses the new password with the policy | ADR-0130, ADR-0132 |
| `reliability.md` §1.1 | The log rule names the `WARN` line of a lost event `unlock`. The variant lists get `AuditWriteFailed { cause }`, the field `failure` of `WebError::Fields`, and `MissingBasicCredentials` | ADR-0130, the ADRs that added the variants |
| `reliability.md` §2.4, §3.7, §7.8 | The row of `disk_check`, the row `low_disk`, and the text of §7.8 name the time limit | ADR-0133 |
| `reliability.md` §8 | The list of interior mutability gets the set of stalled paths of the disk adapter | ADR-0133 |
| `CLAUDE.md`, `.claude/rules/rust.md` | The rule "Log each error once" names the `WARN` line of a lost event `unlock` | ADR-0130 |
| `.claude/rules/web.md` | The rule of the form answer names the login form as the exception | ADR-0132 |
| `api.md` §7.3 | `POST /lang` accepts either token | ADR-0131 |
| `design-system.md` §1 | Sections 10.12 to 10.14 and the multi-line field go to `components.css` | The current `components.css` |
| ADR-0035, ADR-0041, ADR-0073 | "Amended by" notes | ADR-0130, ADR-0132, ADR-0133 |
| `CHANGELOG.md` | Lines under "Unreleased" | This change |

### D7. Run order

Wave 1 runs the document tasks and the code tasks with disjoint files in parallel. The tasks on `pages/own.rs` run in sequence: D5-3, then D5-5. The task of the handler `unlock` runs after the task of the use case. The checks of section 7 of `tasks.md` run last.

## Risks / Trade-offs

- A blocking reader in a unit test can leave a thread. The test ends the reader through its channel before the test ends.
- The CTE with `FOR UPDATE` adds a lock step to the cleanup job. The batch size limits its cost.
- The macro of D5 item 1 hides the enum behind a table. The doc comment of each variant stays in the table, so `cargo doc` still shows it.
- After a committed user command, `refreshed` can fail on a lost database. The user then sees an error after a successful command. Five handlers of `pages/users.rs` share this behavior. This change keeps it. A repeated unlock deletes nothing more.

## Not in scope

- The findings of group B (CI and deploy).
- A multi-field answer of the login form (ADR-0132).
- A retry of the event `unlock` (ADR-0130).
- A new setting for the time limit of `statvfs` (ADR-0133).
- A success answer when the refresh query of a user command fails after the commit. It is a shared behavior of five handlers and needs its own change.
- A PostgreSQL test with two sessions that proves the lock order of D4. The shape test and the behavior tests of task 3.1 cover D4.

## What already exists

- `FieldLines`, `refuse_create`, and `FieldError::recipe` of `recipe-web` give the pattern of D3.
- `log_error` of `src/error/log.rs` gives the fields of the `WARN` line of D1.
- The shared adapter `SystemDiskSpace` of `serve/process.rs` serves `disk_check` and `alerts`. D2 keeps one adapter.
- The behavior tests of the four deletes exist: `maintenance_attempt_purge_deletes_the_oldest_rows_of_a_batch_first`, `maintenance_session_delete_removes_expired_and_too_old_sessions_only`, `delete_for_login_gives_the_newest_100_of_250_and_the_count`, `delete_for_login_with_keep_zero_still_gives_the_count`, and `delete_all_of_user_deletes_only_the_rows_of_that_user`.
- `grants.expected` gives `recipe_app` an `UPDATE` grant on a column of `session` and of `login_attempt`, so the lock step of D4 needs no new grant.

## Failure modes

| Path | Failure | Test | Handling | User sees |
|---|---|---|---|---|
| D1 unlock | Event write fails after the commit | Task 2.3, task 5.4 | Result "event lost", one `WARN` line | Success |
| D1 unlock | Commit fails | Task 2.3 | `Err`, no `WARN` line of a lost event | Error with code |
| D2 disk | `statvfs` stalls | Task 4.1 | `Unreadable` after the limit, read-only mode | Read-only banner |
| D2 disk | Two jobs measure at the same time | Task 4.1 | Both calls run | No change |
| D2 disk | Call ends at the limit | Task 4.1 | No mark stays | No change |
| D3 password | Two bad fields | Task 5.3 | One line for each field | Both lines |
| D3 password | New password on the breach list, wrong current password | Task 5.3 | Line `E-IDN-006`, no attempt row | One line under `password` |
| D4 deletes | Concurrent cleanup and unlock | Shape test, behavior tests | Rows lock in `id` order | No change |
| D5-3 logout | JSON body | Task 5.1 | `422 E-WEB-008` | Error with code |

No row has a failure with no test, no handling, and a silent answer.

## Rejected review points

- The Codex review asked for a PostgreSQL test of the lock order of D4. The owner kept the shape test and the behavior tests (decision 4A).
- The Codex review found that the stall set of the first draft turned a normal concurrent read into a fault. Decision 1A had already changed D2.

## Open Questions

None.

## GSTACK REVIEW REPORT

Review: `plan-eng-review` (Opus) with a Codex outside voice (gpt-6-astra, effort high), 2026-11-05.

| Id | Finding | Decision |
|---|---|---|
| 1 | The stall set marks a normal concurrent read | 1A: mark only a call that reaches the limit |
| 2 | `reliability.md` §3.7 row `low_disk` is missing from task 1.3 | 2A: add the row |
| 3 | No test proves that the form sends no password back | 3A: assert no echo in task 5.3 |
| 4 | Deadlock proof of D4 | 4A: shape test and behavior tests |
| 5 | The parsers of D3 stop at one field and lose `E-IDN-006` | Parse `Password` and `NewPassword` in `recipe-web`, add `FieldError::identity` |
| 6 | `refreshed` can fail after the commit of an unlock | Not in scope |
| 7 | `parking_lot` edge and `reliability.md` §8 | Add to tasks 1.3 and 4.1 |
| 8 | `data-model.md` §6 and the rule copies contradict ADR-0130 and ADR-0132 | Add to tasks 1.2 and 1.5 |
| 9 | Lock-order test of D4 (Codex) | Keep 4A |
| 10 | A match of 69 arms breaks the limit of 50 lines | A `macro_rules!` table |
| 11 | A red step with a wrong expected value proves nothing | A red step on a code mutation |

NO UNRESOLVED DECISIONS
