# Tasks: identity-and-sessions

Each code task starts with a failing test. Run it, see it fail, and keep the output in the report at once. Then write the code. A task that only adds tests for finished code reports "first run passed". After each code task, run the clippy, test, and fmt commands of `openspec/config.yaml`, section `operations.apply.guidance`. A `[mechanical]` task goes to a Haiku subagent. Sonnet does every other code task. `target/p25-brief.md` is the common brief of the subagents.

The ste-lint command is the last command of the section "Local checks" of `CONTRIBUTING.md`. The storage tests need Docker. `cargo test -p recipe-db --test storage <filter>` runs one area of them.

## 1. Documents

- [x] 1.1 (docs/adr) Write ADR-0035 with the items a to t of design D1. Add the index row and the decision map entries. Add an "Amended by ADR-0035" note to the status lines of ADR-0014, ADR-0022, ADR-0029, and ADR-0034. Check with the ste-lint command.
- [x] 1.2 (docs/architecture) Change `reliability.md` section 1.1 with the field `cause` of `Deadlock`, the trigger code `E-AUD-003`, and the log rule of item n. Add the rows `E-IDN-010` and `E-AUD-003` to section 1.2, and add `E-AUD-003` to the sentence on the trigger ranges. Add the key format to section 7.1. Add the rows `db.app_user`, `page.users`, and `audit.unlock_rows` to section 7.11. Check with the ste-lint command.
- [x] 1.3 (docs/architecture) Change `data-model.md` sections 1.1, 3.7, 3.8, 4.11, 5.1, 5.3, 6, 8, and 10. Add section 5.4 with the SQL of `0005` (design D15). Check with the ste-lint command.
- [x] 1.4 (docs/architecture) Change `domain-model.md` section 5.3, `api.md` sections 2.8, 7.1, 7.3, 7.4, and 8, and `overview.md` section 5 (design D3). Check with the ste-lint command.
- [x] 1.5 (CHANGELOG.md) Add ADR-0035, the identity commands and queries, the pages, `0005`, and `create-admin` under `[Unreleased]`. Check with the ste-lint command.

## 2. Domain

- [x] 2.1 (recipe-domain) Add the registry rows `E-IDN-010` and `E-AUD-003` and the enum `IdentityError` with `code()`. First write failing tests. The registry test finds every row of `reliability.md` section 1.2. Each variant gives its code and the message of section 1.2. No variant carries a secret. Check with `cargo test -p recipe-domain identity::error`.
- [x] 2.2 (recipe-domain) Add `Password`, `NewPassword` with its length rules, `PasswordHash`, `SessionToken`, `SessionId`, `CsrfToken`, and `SessionRef::of` (design D4). First write failing tests. `format!("{:?}", value)` gives `[redacted]` for each secret. `SessionId` is the SHA-256 of the token bytes in 64 hex characters. A PHC string without `$argon2id$` fails. Check with `cargo test -p recipe-domain identity::secret`.
- [x] 2.3 (recipe-domain) Add `tools/breached-passwords/filter.py` (standard library only, input and output paths as arguments) and the list file of design D4. Add the list check of `NewPassword` and `OneTimePassword::from_bytes`. First write failing tests. A listed password of 12 characters fails with `E-IDN-006`. A one-time password from fixed bytes has 20 characters of the alphabet and passes the policy. Report the line count and the size of the list. Check with `cargo test -p recipe-domain identity::policy`.
- [x] 2.4 (recipe-domain) Add `TypedLogin`, `ThrottlePolicy`, `AttemptCount`, and `LoginThrottle::decide` (design D6). First write failing tests with fixed instants. `n = 4` with the last failure 1 second ago gives a delay of 2 seconds and `retry_after` 1. A negative elapsed time counts as 0. The budget gives the rest of the window after the oldest failure. A property test shows that a delay never passes 60 seconds. Check with `cargo test -p recipe-domain identity::throttle`.
- [x] 2.5 (recipe-domain) Add the dependency `hmac`, `KeyRing`, and `LoginHmac::compute` (design D4). First write failing tests. A line of 63 or 65 characters, an upper-case line, and an empty file fail. The ring signs with the first key and accepts a signature of each key through `verify_slice`. A changed nonce fails. `Debug` of the ring gives `[redacted]`. Check with `cargo test -p recipe-domain identity::key_ring`.
- [x] 2.6 (recipe-domain) Add the constructors `login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, and `unlock` of `SecurityEvent` and the last-administrator rule. First write failing tests from the delta requirement "Subject of a login security event". Each constructor gives the subject of `data-model.md` section 3.8. The last active Administrator cannot lose the role or the active state. Check with `cargo test -p recipe-domain audit::security identity::last_administrator`.

## 3. Application foundation

- [x] 3.1 (recipe-app, recipe-db) Add the feature `sync` of `tokio`, `argon2`, `getrandom`, `AppError::Identity`, `LockFailure` (design D16), and `serializable_once_more` (design D8). Map `40P01`, `55P03`, and `40001` in `recipe-db` to their causes (review C6). First write failing tests. The helper runs the body once more on `Serialization`, also inside `AuditWriteFailed` and at `finish`. A second failure gives `E-DB-001`. It never repeats on `Deadlock` or `LockTimeout`. Check with `cargo test -p recipe-app identity::retry` and `cargo test -p recipe-db error`.
- [x] 3.2 (recipe-app) Add `IdentitySettings`, the port `PasswordHasher`, and `Argon2Hasher` (design D5, D20). First write failing tests. A hash starts with `$argon2id$v=19$m=19456,t=2,p=1$`. A right password passes, and a wrong password and the dummy hash fail. A `HashFailure` gives `E-WEB-011`. Each `Default` value equals its row of `reliability.md` section 7.11. Check with `cargo test -p recipe-app identity::hasher`.
- [x] 3.3 (recipe-app) Add `HashGate` and `HashPermit` with the blocking test double (design D5). First write failing tests with a paused clock. A wait above `argon2.acquire_timeout` gives `E-WEB-009`. A panic in the hash gives `E-WEB-011`. 32 parallel callers never run more than 4 checks at one time. A caller that drops its future in the middle of a hash keeps the permit taken until the hash ends. Check with `cargo test -p recipe-app identity::gate`.
- [x] 3.4 (recipe-app) Add the ports `UserRepository`, `SessionRepository`, `AttemptRepository`, `LoginAttemptStore`, and `SessionStore`, and their `MemoryStore` adapters (design D5). First write failing tests on `MemoryStore`. T1 counts the failures of the pair and of the address in the window. It inserts no row at the budget. The journal records the class-1 lock. Check with `cargo test -p recipe-app memory::identity`.
- [x] 3.5 (recipe-app) Add `IdentityContext` and `authenticate` (design D2, D10). First write failing tests. A missing session, an idle session, a session older than 12 hours, an inactive user, and another epoch give no session. A valid session moves `expires_at` and returns the role of the user row. `IdentityContext::new` loads the breached-password list. Check with `cargo test -p recipe-app identity::authenticate`.

## 4. Application use cases

- [x] 4.1 (recipe-app) Add `Login` (design D6). First write failing tests from the row `Login` of `domain-model.md` section 5.3 and the delta requirements of `identity-access`. Cover each error code, the event of each case, the deleted old session, and `UserLoggedIn` with `session_ref`. An unknown and an inactive user get the dummy check. A reset between the check and T2 gives `E-IDN-003` and no session. A failed event write gives the storage error. Use `MemoryClock` for the throttle. Check with `cargo test -p recipe-app identity::login`.
- [x] 4.2 (recipe-app) Add `Logout` and `ChangeOwnPassword` (design D7). First write failing tests. The policy error comes first with no attempt row. A throttled change gives `E-IDN-005`. A wrong current password gives `E-IDN-008` and one failed attempt. A reset between the check and the commit gives `E-WEB-004` and keeps the reset hash. A success deletes every session, creates one new session, and writes `OwnPasswordChanged`. Check with `cargo test -p recipe-app identity::logout identity::change_own_password`.
- [x] 4.3 (recipe-app) Add `CreateUser`, `ResetPassword`, and `ActivateUser` (design D8). First write failing tests. Each command checks its permission first. The one-time password has 20 characters and appears in no audit entry. `ResetPassword` increments the epoch and deletes the sessions. `ActivateUser` on an active user gives `E-IDN-009`. A duplicate login gives `E-IDN-001`. Check with `cargo test -p recipe-app identity::create_user identity::reset_password identity::activate_user`.
- [x] 4.4 (recipe-app) Add `SetUserRole` and `DeactivateUser` (design D8). First write failing tests with the fault injection of `MemoryStore`. One `40001` gives a second run that succeeds, also at `finish`. Two give `E-DB-001`. The last Administrator gives `E-IDN-002`. An inactive target of `DeactivateUser` gives `E-IDN-010` and no entry. Check with `cargo test -p recipe-app identity::set_user_role identity::deactivate_user`.
- [x] 4.5 (recipe-app) Add `UnlockLogin` and the 6 identity queries (design D8, D9). First write failing tests. The image of `LoginUnlocked` holds the newest 100 deleted attempts and `login_attempt_count`. A failed `unlock` event gives the storage error, and the unlock stays committed. `ListUsers` filters by `q`, treats `_` as a plain character, and pages by `page.users`. `GetUser` shows the failures in the window. Check with `cargo test -p recipe-app identity::unlock_login identity::queries`.
- [x] 4.6 (recipe-app) Add `bootstrap_administrator` (design D17). First write failing tests. Without an active Administrator it creates one with `must_change_password` and an entry `UserCreated` with no actor. With one it changes nothing. A `40001` gives one more run. Check with `cargo test -p recipe-app identity::bootstrap`.
- [x] 4.7 (recipe-app) [mechanical] Add the 9 commands and the 6 queries to the matrix test and the first-statement test. Add the 9 commands to the audit coverage test. First run the extended tests and see the new rows fail. Check with `cargo test -p recipe-app auth audit::coverage`.

## 5. Storage

- [x] 5.1 (recipe-db) Write `migrations/0005_security_event_guard.sql` from design D15 and `data-model.md` section 5.4. Add the function to the guard list of `01-roles.sh` and set `log_error_verbosity` of `recipe_app` to `terse`. Map `E-AUD-003` in `trigger_code()` and `trigger_messages`. First write failing tests as `recipe_app` and as the superuser of the container. Check with `cargo test -p recipe-db --test storage security_event_guard`.
  - An insert that names `occurred_at` or `count` gives `42501`, and `count = count + 1` passes.
  - `count = 1` and `count = count + 2` give `E-AUD-003`. A change of `detail` by the superuser gives `E-AUD-003`.
  - A drop of the function by `recipe_migrator` fails. The role setting of `recipe_app` shows `terse`.
- [x] 5.2 (recipe-db) [mechanical] Add the column grants of `0005` to `crates/recipe-db/tests/grants.expected`. Add `0005` to the document test of section 5.4 and the static migration check. Add the trigger and the function to the coverage meta-test lists. First run the grant test and see it fail. Check with `cargo test -p recipe-db --test storage grants`.
  - Regression (finding F11): `tests/storage/triggers/budget.rs` inserts its backdated events as the superuser and calls the purge as `recipe_app`.
  - Regression (finding F11): `grants/audit_guard.rs` (`AUDIT_OBJECTS`), `grants/checks.rs` (`AUDIT_FUNCTIONS`, the owned count of 6), and `grants/audit_hardening.rs` include `security_event_guard`.
- [x] 5.3 (recipe-db) Change the insert of `PgSecurityEventSink` (design D15). Add the sink tests of review R6 and Q4 through the port, not through SQL. First write failing tests. Each test that needs one minute waits until the second is below 50. Check with `cargo test -p recipe-db --test storage security_events`.
  - Three `login_failed` events of one address in one minute give one row with `count` 3.
  - A `login_throttled` row holds the HMAC. A `csrf_rejected` row has no subject.
  - `unlock` and `login_rejected_inactive` give one row each.
- [x] 5.4 (recipe-db) Implement `UserRepository`. Seed the users of the contract harness through the port and delete the insert as `recipe_migrator` (design D19). First write the failing contract cases. No row image holds `password_hash`. `lock_by_login` takes `FOR NO KEY UPDATE`. A duplicate gives `app_user_login_unique`. `list` orders by `COLLATE "C"` and filters with `strpos`. A `NOT NULL` failure gives an error whose `Display` chain holds no `$argon2id$`. Check with `cargo test -p recipe-db --test storage contract::users`.
- [x] 5.5 (recipe-db) Implement `SessionRepository` and `SessionStore` with the one statement of design D10. First write the failing contract cases. The session row holds only the SHA-256 of the token. An idle session, an old session, an inactive user, and another epoch give no row. A deleted session gives no row and no `E-DB-008`. A valid call moves `expires_at`. A `NOT NULL` failure on `session` gives no session id and no CSRF token in the `Display` chain. Check with `cargo test -p recipe-db --test storage contract::sessions`.
- [x] 5.6 (recipe-db) Implement `LoginAttemptStore` and `AttemptRepository` (design D6, D8). First write the failing contract cases. Check with `cargo test -p recipe-db --test storage contract::attempts`.
  - `pg_locks`, filtered by the test database, shows the class-1 key during T1 and none after the commit.
  - T1 runs the lock as its own statement and counts with one scan. The budget inserts no row. Fixed `attempted_at` values outside the window do not count.
  - The delete of `UnlockLogin` uses `DELETE ... RETURNING` and gives the newest 100 rows and the full count.
- [x] 5.7 (recipe-db) Add the midway failure tests of `Login` T2, `Logout`, and the 7 other identity commands of `data-model.md` section 6. A failure leaves no row of the command, no deleted session, and an unchanged chain head. Check with `cargo test -p recipe-db --test storage atomic::identity`.
- [x] 5.8 (recipe-db) Add the identity races of design D19. Check with `cargo test -p recipe-db --test storage race::identity`.
  - Two concurrent failed logins for one login: the count of the second includes the row of the first.
  - Parallel failed logins with other logins from one address at 99 failures: at most 103 rows.
  - Two Administrators who demote each other: one `E-IDN-002` and no `40001` to the caller.
  - `DeactivateUser` while the session of the target sends requests, and a `40001` at `finish`: one more run that succeeds.
  - `ChangeOwnPassword` and `Login` against a `ResetPassword` that commits after their check: no new session.
  - Two parallel runs of `bootstrap_administrator`: one Administrator.

## 6. Web

- [x] 6.1 (recipe-web) Add the dependencies of design D3, `WebSettings`, and the router with one `Operation` for each route. Add the request id, the security headers of `api.md` section 9, and `GET /static/htmx.min.js`. First write failing tests from the delta requirements "Security headers on every response" and "Static files". Check with `cargo test -p recipe-web headers static_files architecture`.
  - The file equals the pinned SHA-256. Each `Default` of `WebSettings` equals `reliability.md` section 7.11.
  - A route without an `Operation` fails closed. A test fails when a source file of `recipe-web` names `bootstrap_administrator`.
- [x] 6.2 (recipe-web) Add `WebError`, the error mapping of `api.md` section 5, `base.html`, `error.html`, and `field-errors.html` (design D13). First write failing tests for each family of section 5.1. Check with `cargo test -p recipe-web error_mapping`.
  - Cover the HX headers, `Retry-After`, and `E-IDN-004` as the body of `E-IDN-003`.
  - Each error writes one log line at its level with the code and the `Display` chain, never the `Debug`.
- [x] 6.3 (recipe-web) Add the `fluent` bundles, the cookie `recipe_lang`, and `POST /lang` (`api.md` section 8, design D12 item o). First write failing tests from the baseline requirements of the language cookie and the delta requirement "Language switch redirect". `/\evil.example` and a tab after the first slash give `303` to `/recipes`. Check with `cargo test -p recipe-web i18n lang`.
- [x] 6.4 (recipe-web) [mechanical] Write `i18n/en.ftl` and `i18n/pl.ftl` with the page texts and the message of each code that the routes of this change return. First write the failing test. Each English message equals its row of `reliability.md` section 1.2 without "Reference: {request_id}.". Each `en` key has a `pl` key. Check with `cargo test -p recipe-web messages`.
- [x] 6.5 (recipe-web) Add the client address, the body limit, and `TraceLayer`. Add the session layer, the password gate, and the read-only gate in the order of design D11. First write failing tests. Check with `cargo test -p recipe-web pipeline`.
  - A forwarded header from an untrusted address does not count. `::ffff:172.20.0.2` matches the CIDR `172.20.0.0/16`.
  - A forwarded entry that is not an address gives `E-WEB-008`. A missing `ConnectInfo` gives `E-WEB-010`.
  - A body above 256 KiB gives `E-WEB-003`. A user with `must_change_password` gets `303` to `/me/password`.
  - The read-only mode gives `E-WEB-006` to a command and lets `POST /login` pass. An invalid cookie on `GET /login` gives the page.
- [x] 6.6 (recipe-web) Add the CSRF layer (design D12). First write failing tests from the delta requirements "Pre-session CSRF cookie" and "Origin check for mutating requests". Check with `cargo test -p recipe-web csrf`.
  - Cover the field and the header of the session token, a forged cookie, an old-key cookie, and the cookie attributes.
  - `POST /login` with an open session accepts `csrf_pre`. `Origin: null`, `http://`, another port, and a missing `Host` fail.
  - A forged request in read-only mode writes `csrf_rejected`. A failed event write gives the storage error.
- [x] 6.7 (recipe-web) Add the pages and handlers of `/login`, `/logout`, `/me`, and `/me/password`. First write failing tests from `api.md` sections 2.1 and 2.8 and the delta requirements "Login redirect" and "Password change response". A wrong password, an unknown login, and an inactive user give the same status and body. Logout clears the cookie with `Max-Age=0`. Check with `cargo test -p recipe-web pages::own`.
- [x] 6.8 (recipe-web) Add the pages and handlers of `/users`, `/users/new`, `POST /users`, `/users/{login}`, and the five `POST /users/{login}/...` routes. First write failing tests from `api.md` section 2.8. A malformed `{login}` gives `404 E-WEB-005` to every caller (`api.md` section 1). A well-formed unknown `{login}` gives `404 E-WEB-005` to an Administrator and `403` to an Operator. The one-time password appears only in the response of its command. Check with `cargo test -p recipe-web pages::users`.
- [x] 6.9 (recipe-web) [mechanical] Write the endpoint × role test and the route coverage test of design D19. Take the route list from the router. First run them and see the first missing row fail. Check with `cargo test -p recipe-web endpoint_role route_coverage`.
  - The endpoint × role table covers the 16 routes and the 6 principals.
  - Each of the 10 mutating routes gets the four bad requests of D19. Each gives `403 E-WEB-007`, writes `csrf_rejected`, and changes nothing.
- [x] 6.10 (recipe-web) [mechanical] Write the XSS corpus test for each text field of the new pages. The fields are `login`, `q`, the login of `CreateUser`, `next` of `POST /lang`, and each echoed path value. First run it on one field and see it fail without the corpus file. Check with `cargo test -p recipe-web xss`.
- [x] 6.11 (recipe-web) Add the redaction test of design D18 and the HTTP tests of the session end. First write failing tests. Check with `cargo test -p recipe-web redaction session_end`.
  - After `SetUserRole`, `DeactivateUser`, `ResetPassword`, and `ChangeOwnPassword`, the old session gets `401 E-WEB-004`.
  - A throttled login gets `429` with `Retry-After`.
  - The captured log holds no secret of D18, also not the typed login of an unknown account.

## 7. Subcommand `create-admin`

- [x] 7.1 (recipe-server) Add the arguments and the configuration of design D17 with the key `db.app_user`. First write failing tests. `--login` is required. Stdin and `--password-file` give the same password. An empty file, a file with two lines, a carriage return, and a terminal stdin fail. `Debug` of the settings gives `[redacted]` for the password. Check with `cargo test -p recipe-server config create_admin`.
- [x] 7.2 (recipe-server) Connect `create-admin` to `bootstrap_administrator`. First write the failing binary test against a container of the storage harness. Check with `cargo test -p recipe-server --test create_admin`.
  - A first run creates the Administrator. A second run writes one `INFO` line and exits with 0.
  - A short password gives `E-IDN-006` and exit code 1. The stdout holds no password and no `$argon2id$`.

## 8. Verification

- [x] 8.1 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` with `cargo llvm-cov --jobs 2`, and `openspec validate identity-and-sessions --strict`. Each crate reaches 80 % line coverage. Check with the output of each command.

## 9. Code review fixes

The user accepted these fixes of the code review on 2026-10-14. ADR-0035 holds the decisions.

- [x] 9.1 (recipe-domain, recipe-web) `TypedLogin::parse` rejects a control character (review M1). First write failing tests. A login with `U+0000`, a tab, `U+007F`, or `U+0085` gives the new reason. `POST /login` with such a login gives `422 E-WEB-008` and inserts no attempt row. Check with `cargo test -p recipe-domain typed_login` and `cargo test -p recipe-web login`.
- [x] 9.2 (recipe-domain, recipe-app) `retry_after` of the pair is the full `delay(n + 1)` (review M8). First write failing tests from the scenarios "Login inside the delay" and "Login after the Retry-After time". A login that waits the `Retry-After` time gets a password check. Check with `cargo test -p recipe-domain throttle` and `cargo test -p recipe-app identity::login`.
- [x] 9.3 (recipe-app, recipe-db) Mark an attempt as a success also when `UnlockLogin` deleted its row (review M2). First write failing tests. The contract case runs on both adapters. A `Login` and a `ChangeOwnPassword` with an unlock between T1 and T2 succeed. Check with `cargo test -p recipe-app identity::login identity::change_own_password` and `cargo test -p recipe-db --test storage contract::attempts`.
- [x] 9.4 (recipe-app) `HashFailure` becomes a reason without `std::error::Error` (review M3). First write the failing test: each `thiserror::Error` type in `crates/*/src` is a type of `reliability.md` section 1.1. Check with `cargo test -p recipe-app error_types`.
- [x] 9.5 (recipe-app, recipe-web) Create the id of a new user and the request id from `random_bytes`, with no panic path (review M4). First write failing tests. The request id stays a UUID v7. A failed read of random bytes for the request id uses a process counter. Check with `cargo test -p recipe-app identity::create_user identity::bootstrap` and `cargo test -p recipe-web request_id`.
- [x] 9.6 (recipe-app) Write `attempted_at` of the image of `LoginUnlocked` in the format of the other time values of that image (review D2). First write the failing test. Check with `cargo test -p recipe-app identity::unlock_login`.
- [x] 9.7 (recipe-web) A `{login}` that is not UTF-8, for example `/users/%FF`, gives `404 E-WEB-005` on each of the six routes (review M5). First write failing tests. Check with `cargo test -p recipe-web pages::users`.
- [x] 9.8 (recipe-web) Every method except `GET`, `HEAD`, and `OPTIONS` passes the Origin and token checks (review D3). The route coverage test uses the same rule. First write failing tests. Check with `cargo test -p recipe-web csrf route_coverage`.
- [x] 9.9 (recipe-web, recipe-app, recipe-db) A trusted proxy without `X-Forwarded-For` gives `500 E-WEB-010`. Both adapters store the canonical form of each client address (review D4). First write failing tests from the scenario "Trusted proxy without a forwarded header" and a contract case with `::ffff:10.0.0.5`. Check with `cargo test -p recipe-web client` and `cargo test -p recipe-db --test storage contract::attempts security_events`.
- [x] 9.10 (recipe-server) An unknown argument of `create-admin` does not appear in the error text. `--password-file` reads at most 1 KiB and 1 byte (review M6, D5). First write failing tests. Check with `cargo test -p recipe-server config create_admin`.
- [x] 9.11 (recipe-db) `01-roles.sh` sets `log_parameter_max_length = 0` for `recipe_app` (review D6). First write the failing storage test. Check with `cargo test -p recipe-db --test storage grants`.
- [x] 9.12 (recipe-db) Add a second gate to the races 3 and 7 of `race::identity` (review D7). Then the first transaction commits before the second goes on. Check with `cargo test -p recipe-db --test storage race::identity`.
- [x] 9.13 (recipe-web, recipe-app) [mechanical] Delete the duplicates of review D8. These are the fallback of `request_id`, `HX_REQUEST`, the hex codec, `ONE_TIME_BYTES`, and `NONCE_BYTES`. Build the cookie `recipe_lang` with the `cookie` crate. Split `routes::table` into functions of at most 50 lines. The existing tests stay green. The audit insert of `recipe-db` also stores the canonical client address, after a failing storage test. Check with `cargo test -p recipe-web` and `cargo test -p recipe-db --test storage audit`.
- [x] 9.14 (workspace) Run every command of the section "Local checks" of `CONTRIBUTING.md` again with `cargo llvm-cov --jobs 2`, and `openspec validate identity-and-sessions --strict`. Check with the output of each command.

## Workflow follow-up

- Step P4.2 decides on the known limits M7 and M9 of ADR-0035 in new ADRs.
- Step P4.2 also decides on these review findings:
  - the expiry of a one-time password (C4)
  - the HMAC of the login in `login_attempt` (C7)
  - a second check of the actor in an administrator command (C2)
  - `422` before `403` for a bad form of an Operator (R2-2)
- Step P4.2 replaces `Uuid::new_v4` in the use cases of P2.3 with a function without a panic path.
- Step P4.1 adds these items of the review:
  - the span name `http.request` with the field `error_code` (R2-5)
  - the error answer of the router fallback (R2-3)
  - a smaller default of the trusted proxies (C6)
- Step P4.2 also handles the low findings of review D9:
  - `error.html` for `429` and `503` of a login without HTMX (R2-6)
  - the error cause of a login that loses the race with a deactivation (R1-4)
  - a newtype for the pre-session nonce (R1-8)
  - the fault point `Begin` in `pool_transaction` of `MemoryStore` (CR10)
  - the cleanup of a session row with an old epoch (Q2-2)
  - one token check in the CSRF layer (CR6)
  - a router that takes no more routes after `router()` (R2-9)
  - a smaller visibility of `bootstrap_administrator` (S10)
- Step P3.1 adds the page `/recipes`, which is the target of the login redirect.
- Run the ste-lint command on each base spec merged with its delta before the archive.
- Archive with the `openspec` CLI. A hook runs `cargo test --workspace` first.
- At the commit, keep the executable bit of `deploy/db/initdb/01-roles.sh`.
- P4.1 adds the startup check of the key files, `serve`, `CatchPanicLayer`, `TimeoutLayer`, the JSON subscriber, and `cleanup`.
