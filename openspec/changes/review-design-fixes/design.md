# Design

## Context

`proposal.md` gives the motivation. `docs/plan/review/2026-11-13-full-repo-review.md` §4 gives the location, the failure scenario, and the verifier note of each finding. `docs/plan/review/2026-11-13-fix-plan.md` §1 holds the binding decisions of the owner. The owner decided the open points of this design on 2026-11-14. The design checked each finding at HEAD `eba0883`, after the merge of change A:

- F-01: the review names four handlers. The design found seven more handlers that read after a committed command and return the read error. The list is in D1.
- F-02: `export/flow.rs` maps every state other than `Approved` under the lock to `NoApprovedVersion` (`check_exportable`). The flow does not pick again.
- F-03 note: `crates/recipe-server/tests/error_registry.rs` exists. Its list `EXCLUDED` holds `SnapshotItem`. The §1.2 row of `E-PLT-020` does not name `SnapshotItem::UnknownUnit` yet.
- F-12: `PlantErrors` has no `code()`. `CheckResultFault` implements `std::error::Error` and has no `code()`. `reliability.md` §1.1 calls it a reason, as ADR-0151 (j3) does.
- F-13: `from_stored` exists on `TypeExpr`, `TypeName`, `ProcessLimit`, `ParameterValue`, and the entity `PlcType`. `OneTimePassword::from_bytes`, `IecLiteral::from_count`, and `ParameterValue::from_texts` are three more public constructors.
- F-19: eight tests in `crates/recipe-db/tests/storage/constraints/recipe.rs` insert a row that is not a draft. D8 names them.
- F-24: change C made `01-roles.sh` refuse a NUL byte. `secret_line` in `crates/recipe-server/src/config/mod.rs` still accepts one.

## Goals / Non-Goals

**Goals:**

- Each decision has one ADR, except the two Status notes of D5.
- Each change of `docs/architecture/` comes in the same change as its ADR.
- Each code task starts with a test that fails on the defect, or states why no such test exists.

**Non-Goals:**

- No change of `deploy/` or `.github/`, except the two lines of `deploy/pgtools/restore-code.expected` that D8 needs.
- No change of a finding of change A or change C.

## Decisions

### D1. Success answer after a failed read (F-01, ADR-0163)

- The rule covers each handler that reads after a committed command. The handlers are `remove` (`parameters/remove.rs`), `save` (`parameters/edit.rs`), `set_value` and `replace_limits` (`parameters/value.rs`), and `rebind` (`versions/rebind.rs`).
- The other handlers are `approve` (`approval/approve.rs`), `retire` (`versions/actions.rs`), `add` (`units/add.rs`), and `set_role`, `deactivate`, `activate`, `unlock` (`users.rs`).
- An HTMX request gets `200` with the fixed notice of fix plan §1 in `#toast`. The answer sets `HX-Retarget: #toast`, `HX-Reswap: innerHTML`, and `HX-Push-Url: false`.
- The answer also holds an empty `#error-panel` with `hx-swap-oob="innerHTML"`, like `parameter-toast.html` and `password-changed.html`. A stale error line then goes away.
- The answer sets `HX-Trigger` with `rowVersion` only when the output of the command holds the new value. `DraftOutput` holds it. The outputs of `ApproveVersion`, `RetireVersion`, `AddUnit`, and the four user commands do not.
- A request without HTMX gets `303` to the page of the changed object. The `GET` of that page reads the state again.
- The handler writes one `WARN` line with `error_code`, `request_id`, and `chain`, like `log_lost_unlock_event` (ADR-0130 (b)). This is a new log place of `reliability.md` §1.1.
- One shared function in `recipe-web` builds the notice answer and writes the line. The `pl` and `en` bundles get the text of the notice.
- A test needs a read that fails after the commit. `MemoryStore` gets a fault that fails the first `begin` after a commit.
- Alternative: one more read after the failure. Rejected, because the second read can fail in the same way under the same load.
- Alternative: an answer from the command output alone. Rejected, because each answer shows state that only the read gives.

Owner decision 2026-11-14: D1 covers the four handlers of `users.rs`. The doc comment of `refreshed` changes with them.

### D2. Second pick of the approved version (F-02, ADR-0164)

- On the route `/approved` only, a state other than `Approved` under the lock of step 3 ends the first transaction. The flow then runs step 2 once more and locks the new pick.
- The second lock checks the state again. A state other than `Approved` there gives `404 E-EXP-001`. A second pick that finds no approved version gives the same answer. No third pick runs.
- An example: the pick finds v4, and an approval of v5 retires v4 before the second lock. The flow then gives `404 E-EXP-001` and writes no entry `VersionExported`.
- The route by number does not change. The Basic route and the v2 unit route share the flow. The second pick does not run the Basic check, T1, or T2 of ADR-0109 (m) again.
- The second pick runs inside `timeout.export` and keeps the same `export.permits` permit. The permit moves into the blocking task of step 5 only after the second check.
- The flow holds one version lock at a time, so the lock order of ADR-0015 holds.
- `MemoryStore` gets a hook that runs once before the next `lock_version_shared`, like `on_read_draft`. The hook makes the concurrent approval.
- Alternative: a loop until the state is stable. Rejected, because the loop has no bound. Alternative: a document that accepts the transient `404`. The owner rejected it (fix plan §1).
- ADR-0164 amends ADR-0019 and ADR-0109 (ah). `api.md` §2.7 and the row `ExportVersion` of `data-model.md` §6 change.

### D3. `code()` of every error type (F-12, F-03 note, ADR-0165)

- `PlantErrors::code()` returns the code of the first entry, like `RecipeErrors::code()`. A carrier owns no row of §1.2 of its own. Its `code()` passes on an inner code.
- `CheckResultFault` stays an error type and gets `code()` (fix plan §1). Each variant gets a new row of §1.2 with the status `-`, like `E-WEB-015`.
- The `WARN` line of the job `alerts` for this fault gets the field `error_code` (`reliability.md` §1.1).
- `reliability.md` §1.1 lists `CheckResultFault` as an error type and not as a reason. It lists `SnapshotItem`. The §1.2 row of `E-PLT-020` names `SnapshotItem::UnknownUnit`.
- The registry test adds `CheckResultFault` to `SCOPE`. `SnapshotItem` stays in `EXCLUDED`, because its `code()` passes on the inner code.
- ADR-0165 amends ADR-0027, ADR-0151 (j3), and ADR-0154 (the sentence "`PlantErrors` owns no code").

Owner decision 2026-11-14: `CheckResultFault::Unreadable` gets `E-WEB-016`, and `CheckResultFault::Malformed` gets `E-WEB-017`. `WEB` is the context of cross-cutting application results (§1.2).

### D4. Constructor for stored data (F-13, ADR-0166)

- The finding needs an ADR, because it adds an exception to the rule of `reliability.md` §4.3 and of `CLAUDE.md`.
- A value object can have a second public constructor `from_stored(input, rules)`. It checks stored data with the rule set of that data (ADR-0120 (i), ADR-0143).
- The stored form and the input form can differ. `ParameterValue::from_stored` reads a time leaf as a JSON number with the count. `ParameterValue::parse` reads it as an IEC literal.
- `RuleSet::Earlier` skips the checks that a later ADR added, for example the check of the significant digits of ADR-0117. The ADR names each skipped check.
- The ADR names each holder of the context. It also states the rule for `OneTimePassword::from_bytes`, `IecLiteral::from_count`, and `ParameterValue::from_texts`, from the doc comment of each.
- `reliability.md` §4.3, `CLAUDE.md`, and `.claude/rules/rust.md` name `from_stored`. No code changes.
- ADR-0166 amends ADR-0120 (i) and ADR-0143.

### D5. Status notes (F-14, F-24)

- F-14: the Status line of ADR-0013 gets "Amended by ADR-0046 (a form route runs its form query first)". No new ADR, because no rule changes.
- F-24: the Status line of ADR-0031 gets a note: a NUL byte is the fourth stop case of a secret file. Change C carries the check of `01-roles.sh`.

Owner decision 2026-11-14: `secret_line` refuses a NUL byte too, so `01-roles.sh` and `recipe-server` apply one rule (ADR-0031). Task 8.6 does it.

### D6. Three corrections of `data-model.md` (F-15, F-17, F-18, ADR-0168)

- These fixes change `docs/architecture/`, so they need an ADR (`CLAUDE.md`, "Source of truth"). ADR-0126 (h) recorded a correction in the same way.
- One ADR holds the three, because they share one decision. The text follows the shipped code and DDL. No migration and no code change follow (fix plan §1, F-18).
- F-15: the rows `CreateUser`, `ResetPassword`, and `CreateRecipe` of §6 name the read in a read-only transaction before the command transaction. The pattern of the row `SetMachineDataBlock` applies (`reliability.md` §7.4).
- F-17: the exception table of §1.1 gets rows for `plc_type_member` and `draft_editor`.
- F-18: §1.3 says that `LoginText` refuses a control character and that the database checks the length only.

Owner decision 2026-11-14: one ADR 0168 holds F-15, F-17, and F-18.

### D7. `search_path` of the audit guard function (F-16, ADR-0167)

- The event-trigger function of `audit_owner` keeps `search_path = pg_catalog, pg_temp` (`data-model.md` §5.1). The narrower path is safer (fix plan §1).
- `data-model.md` §1.1 and `.claude/rules/sql.md` name this exception. No migration.
- ADR-0167 amends ADR-0098.

### D8. A new version row is a draft (F-19, ADR-0169)

- Migration `0016_recipe_version_insert.sql` adds the function `recipe_version_insert_draft()` and a `BEFORE INSERT` trigger on `recipe_version`.
- The trigger refuses a row with a status other than `draft`, a `row_version` other than 1, or a `deleted_at`. The `CHECK recipe_version_status_fields` then covers the approval columns and the hash.
- The new code `E-RCP-037` has the status 409. `reliability.md` §1.2 gets the row first. `data-model.md` §4 gets a new section and a row of §4.11. `domain-model.md` §3 extends I4 to an insert.
- `data-model.md` §5.2 gets one sentence: the trigger limits the `INSERT` grant of `recipe_app` on `recipe_version` to a new draft.
- The function gets the default `EXECUTE` grant of `PUBLIC`, like `recipe_version_transition()`. `grants.expected` gets that line.
- `deploy/pgtools/restore-code.expected` gets two lines, each with its hash (ADR-0076 (a) to (c)). One is the `FUNCTION` line of the new function. The other is the `TRIGGER` line of the new trigger, which `restore-code.sql` collects.
- The trigger fires before each `CHECK`. These tests of `constraints/recipe.rs` then get `E-RCP-037` and need a change:
  - `recipe_version_approved_by_fkey_rejects_an_unknown_approver`
  - `recipe_version_status_value_rejects_an_unknown_status`
  - `an_unknown_status_is_reported_by_status_fields_when_both_checks_exist`
  - `recipe_version_row_version_positive_rejects_a_row_version_of_zero`
  - `recipe_version_hash_format_rejects_a_hash_that_is_not_lowercase_hex`
  - `recipe_version_status_fields_rejects_an_approved_version_without_a_hash`
  - `recipe_version_status_fields_rejects_a_retired_version_without_a_retirement_time`
  - `recipe_version_deleted_only_draft_rejects_a_discarded_approved_version`
- Such a test inserts a draft and moves it through the allowed transitions. When a test must break a `CHECK` on insert, it disables the new trigger only inside its own transaction, and that transaction rolls back.
- `pg_dump` writes triggers in the post-data section, so a restore of a dump does not fire the trigger. WAL replay fires no trigger.
- Alternative: a column-level `INSERT` grant without `status`. Rejected, because it binds only `recipe_app` and gives no coded error. The trigger has the shape of trigger 4.7.

### D9. Secret mounts of the service `db` (F-25, ADR-0170)

- The service `db` keeps the mounts of the four login role secrets (fix plan §1). No change of `deploy/`.
- The ADR records three reasons. `01-roles.sh` needs the files on each empty volume (ADR-0103). Compose has no mount for the first start only. The superuser of `db` can already set each role password.
- The ADR records the accepted risk: a compromise of `db` gives the clear text of the four passwords for a use outside `db`.

### D10. Cookies when `session.secure` is false (F-09, ADR-0171)

- Only a build with the feature `dev` accepts `session.secure=false` (ADR-0062 (b)). With `false`, no cookie has `Secure`.
- A browser refuses a cookie with the prefix `__Host-` and without `Secure`. The session cookie is then `recipe_session`, and the pre-session cookie is `recipe_csrf`.
- `recipe_lang` drops `Secure` and keeps its name. `HttpOnly`, `SameSite=Strict`, and `Path=/` stay on each cookie.
- The server keeps `Strict-Transport-Security`. A browser ignores it on plain HTTP.
- `WebSettings` holds the value. `WebSettings::default()` keeps `secure = true`. `ServeSettings::new` in `serve/plan.rs` copies the configured value.
- Each writer and each reader of the cookies takes the name from `WebSettings`, so one process uses one set of names. The writers are in `cookies.rs`, `csrf/pre_session.rs`, and `i18n.rs`. `CLEAR_SESSION_FALLBACK` in `cookies.rs` follows the value too.
- The readers are `session.rs` (`SESSION_COOKIE`), `pages/request.rs` (`SESSION_COOKIE`), and `csrf.rs` (`CSRF_COOKIE`).
- Round-trip tests run for both values: a login, then a `GET` with the session cookie, then a logout. Also `GET /login`, then `POST /login` with the pre-session cookie.
- Alternative: no effect of the key. Rejected, because the key then records a value that no code applies. The owner asked for a code change (fix plan §1).
- ADR-0171 amends ADR-0062 (b). `api.md` §7.1, §7.3, §8, and the rows `session.cookie_name` and `session.secure` of `reliability.md` §7.11 change.

### D11. One-time password after a dropped request (F-06, ADR-0172)

Verdict: the defect is real.

- (a) `request_timeout` in `crates/recipe-web/src/timeout.rs:44-47` drops the inner future. `reset_password.rs:105` and `create_user.rs:112` await the commit as their last await. After a COMMIT reply that comes too late, the client gets `503 E-WEB-009` and no password.
- (b) The form `user-new-form` in `crates/recipe-web/templates/user-new.html:6` has no `hx-boost` and no `hx-sync`. A second submit gets the refusal of a duplicate login.

Owner decision 2026-11-14:

- (a) Known limit. The window is one COMMIT round trip. A task that the request cannot cancel does not help, because the password cannot reach a client that is gone. The next step of the Administrator: open the user list. When the user exists, run `ResetPassword` to get a new one-time password. `docs/ops/runbook.md` states this step.
- (b) Fix. The form gets `hx-boost="true"`, `hx-sync="closest form:drop"`, and `hx-push-url="false"`. The answer stays the full result page (`api.md` §2.8). `.claude/rules/web.md` adds the form to the list of `hx-sync`.
- A browser test sends a double submit. It finds one `POST /users`, and the page shows the one-time password.

### D12. Security event after a dropped request (F-07, ADR-0172)

Verdict: the defect is real.

- `unlock_login.rs:79` commits `LoginUnlocked` (commit at :127). Then :87 awaits `security.record` with no detached task.
- `login.rs:131` commits the login attempt in T1 (`begin_attempt`). Then :138 runs the Argon2 check, and `refuse` (:192-193) awaits `security.record`.
- `change_own_password.rs:86` commits the attempt. Then `reject_throttled` (`throttle.rs:103`) awaits `security.record`.
- `authenticate_basic.rs:84` commits the attempt in T1. Then :98 runs the Argon2 check, and `refuse` (:186) writes the event. The export paths skip `request_timeout` (`timeout.rs:41-43`), so only a closed connection drops this future.
- `request_timeout` and a client that closes the connection drop the future. A client with a short timeout leaves login attempt rows without an event `login_failed` (A33).

Owner decision 2026-11-14: the narrow fix.

- The four use cases are `Login`, `ChangeOwnPassword`, `UnlockLogin`, and `AuthenticateBasic`. The request future takes the `HashGate` permit or the Basic permit, as today. Then the gate wait stays cancellable.
- A detached task covers only the span from T1 to the write of the security event. The request future moves the permit into the task, so `argon2.permits` still bounds the Argon2 work.
- The task does not cover T2 of `Login`, `replace` of `ChangeOwnPassword`, or the step after the judge of `AuthenticateBasic`.
- The task sends its judge result through a `oneshot`. The request future then runs T2 or `replace`. When the receiver is closed, the task skips that step, because no future waits for it.
- The task writes the one `WARN` line of a lost event itself, before it sends its result (ADR-0130 (b)). The handler never logs a lost event. A cancel before the send, after the send, or after the receive then gives exactly one line.
- The four use cases share one function that writes the event and writes the `WARN` line on a failure. The use case does not return the storage fault of the event.
- `UnlockLogin` completes as ADR-0130 states. `Login`, `ChangeOwnPassword`, and the Basic check give their refusal or their throttle answer, as without the fault. Owner decision 2026-11-14: accepted, with the one `WARN` line of the task.
- The handler of `POST /users/{login}/unlock` loses its call of `log_lost_unlock_event`. ADR-0172 amends ADR-0130 (b) and (c) for this log place.
- `reliability.md` §1.1, `CLAUDE.md`, and `.claude/rules/rust.md` name the log place of the task.
- The task runs with `.instrument(Span::current())`, so its lines keep the span and the `request_id` of the request.
- A new setting `identity.detached_tasks` bounds the count of open detached tasks. The request future takes a slot before T1. A slot that does not come within `argon2.acquire_timeout` gives `503 E-WEB-009`, and nothing commits.
- `reliability.md` §7.11 gets the row of the new setting. Owner decision 2026-11-14: `identity.detached_tasks` is 8, two times `argon2.permits`.
- The detached tasks run in one `JoinSet` behind a `parking_lot::Mutex`. Its join is a ninth catch place of `reliability.md` §2.4 rule 6.
- A panic closes the sender. The use case maps that to `AppError::TaskFailed` with a new stage (`E-WEB-011`).
- The slot is an owned semaphore permit that the task holds. The permit drops when the task ends, also after an error and during the unwind of a panic.
- Each spawn first reaps every finished task with `try_join_next`. The `JoinSet` then holds at most `identity.detached_tasks` open tasks and the tasks that ended since the last spawn.
- Owner decision 2026-11-14: the reaper writes no line for a panic. A reaped panic only frees its slot. The panic hook line stays. While the client waits, the `WebError` mapping also logs the `TaskFailed` of the request, as for `spawn_blocking` today.
- `reliability.md` §2.2 (row `JoinHandle`) and §2.4 rule 7 name the join of a detached task next to `spawn_blocking`. ADR-0172 amends ADR-0125 (f), which counts eight places.
- The shutdown drains the detached tasks before it closes the pools (`serve/opened.rs`). They run in step 3 of `reliability.md` §7.3 under `shutdown.drain`. The connections and the detached tasks share the one deadline of `shutdown.drain`. Step 4 aborts the rest with one `WARN` line that gives the count. Their transactions roll back, and then step 5 closes the pools.
- Alternative: detach the whole body of each command. Rejected, because T2 and `replace` then run after the client is gone, and the gate wait stops being cancellable.
- Alternative: a known limit. Rejected, because the gap hides failed logins from `security_event`.

### D13. Dropped call of `read_with_limit` (F-22, ADR-0172)

Verdict: the mechanism is real. No call path reaches it in normal operation.

- `crates/recipe-server/src/limited_read.rs:170-176` sets the mark only in the arm `Err(_elapsed)`. A caller that drops before `limit` leaves a running call without a mark.
- Each job waits for each read (ADR-0161 (b)). Only two paths drop a job run. `Supervisor::drain` aborts the open runs after its deadline (`supervisor.rs:279-296`). The start race drops the start steps on the stop signal (`serve/start.rs:164-168`).
- `shutdown_timeout(shutdown.runtime_stop)` in `serve/opened.rs:133` bounds the exit with a blocked thread.

Owner decision 2026-11-14: known limit. ADR-0172 amends ADR-0133 (c): the mark covers a call whose caller waits to the limit. The operator needs no step. The doc comment of `read_with_limit` states the limit.

- The decision holds for the current callers only. A new caller that can drop the call early needs a new ADR.
- Alternative: a timer task that marks the path after the limit. Rejected, because it adds a catch place and a task for a case of the stop only.

### D14. Order of the work

- The ADR tasks of section 1 come first. They write disjoint files. Each one also edits the Status lines of the ADRs that it amends.
- The tasks of sections 2 to 7 change `docs/architecture/`, `CLAUDE.md`, and `.claude/rules/`. Each section edits disjoint files.
- The code tasks come after the documents. A test of a code task reads a row of §1.2 that a document task adds.
- The rows of tasks 3.2 and 4.1 make facts of the registry test fail until tasks 10.1 and 9.10 land. The Check of 9.10 runs only the fact of its row. Task 10.1 runs the full registry test.

## Risks / Trade-offs

- [D1 notice] A user can miss the notice and act on a stale page. → The notice tells the user to reload. A stale `row_version` gives `409 E-WEB-001`.
- [D8 tests] The new trigger breaks the eight tests of D8. → Task 8.7 changes those tests as D8 states.
- [D10 names] A browser keeps the old cookies after a change of `session.secure`. → The server ignores a cookie with the other name. The user logs in again.
- [D12 tasks] A detached task can outlive its request. → `identity.detached_tasks`, `argon2.permits`, and the drain of §7.3 bound the count and the time.
- [D12 abort] Step 4 of the stop can abort a detached task before its event. → The `WARN` line of the drain gives the count. This case needs a stop of the server.

## Migration Plan

- Migration `0016_recipe_version_insert.sql` is forward-only. It adds one function and one trigger. It does not scan or rewrite a table.
- A rollback is a new forward migration that drops the trigger and the function.
