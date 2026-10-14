# Design: identity-and-sessions

## Context

See `proposal.md`, section Why. `recipe-domain::identity` holds `Login`, `Role`, `Permission`, and `UserId`. `SecurityEvent` has only the constructor `forbidden`. The catalog of `recipe-app` already holds the 9 Identity and Access commands and the 6 identity queries, but no use case and no port of them. Migrations `0001` to `0004` hold the full DDL of `app_user`, `session`, `login_attempt`, and `security_event`. `recipe-web` is an empty crate.

The archived designs of `app-use-cases-and-ports` (D1 to D21) and `db-schema-and-adapters` (D1 to D17) are binding. ADR-0035 of this change records the decisions of D1 and the clarifications that no document holds today.

The rules come from these sources. This document does not repeat them.

| Topic | Source |
|---|---|
| Users, sessions, throttle, Argon2, bootstrap | ADR-0014, `domain-model.md` section 2.3 |
| Commands, errors, commit paths, queries | `domain-model.md` sections 5, 5.3, 5.5 |
| Statements, isolation, and locks of each command | `data-model.md` sections 1.4 and 6 |
| Tables and grants | `data-model.md` sections 3.5 to 3.8, 5.1, 5.2 |
| Routes, matrix, errors, session, CSRF, i18n, headers | `api.md` sections 2.1, 2.8, 3, 5, 7, 8, 9 |
| Error codes, logging, redaction, settings | `reliability.md` sections 1.1, 1.2, 3, 7.1, 7.11 |
| Permissions and the forbidden event | ADR-0013, ADR-0022, ADR-0029 |
| Lock order, commit paths | ADR-0015 |
| Audit objects and their guard | ADR-0034, `data-model.md` section 5.1 |
| Decisions of this change | ADR-0035 |

## Goals / Non-Goals

**Goals:**

- No path logs, returns, or audits a secret of D18.
- Each route runs the CSRF and Origin checks for a mutating request, then the password gate and the read-only gate, before its handler.
- No use case waits for an Argon2 permit while it holds a connection or a transaction. No hash runs without a permit.
- No session survives a password reset, a deactivation, or a role change, also under a race with a running login.
- One contract suite runs every new port method on `MemoryStore` and on PostgreSQL.

**Non-Goals:**

- No `serve` wiring. The router is a library function. Tests drive it with `tower::ServiceExt::oneshot`.
- No `CatchPanicLayer`, `TimeoutLayer`, JSON subscriber, or startup check of the key files (P4.1).
- No style sheet and no design system (P3.1). The pages hold plain HTML.

## Decisions

### D1. Scope (answers of the user, 2026-10-14)

- All 16 routes of `api.md` sections 2.1 and 2.8, the 9 commands of `domain-model.md` section 5.3, and the 6 identity queries of section 5.5.
- The full request pipeline of `recipe-web` (D11 to D13). A minimal `pl` and `en` bundle holds the texts of these pages and their error codes.
- S6 gets the migration `0005` with a guard trigger (D15).
- The local breached-password list comes from SecLists, filtered to 12 to 128 characters (D4).
- The log test uses an own `MakeWriter`, with no new crate (D18).
- ADR-0035 records the items a to h of the first answers and the clarifications i to l. It also records the items m to t of the plan review. The user accepted its findings F1 to F27 on 2026-10-14.

| Item | Decision | Design |
|---|---|---|
| a | `DeactivateUser` on an inactive user gives `E-IDN-010` | D8 |
| b | `ChangeOwnPassword` runs the login throttle | D7 |
| c | `StorageError::Deadlock` carries a `LockFailure` cause | D16 |
| d | A key file holds lines of 64 lower-case hex characters | D4 |
| e | `Origin` matches `https://` and the `Host` header | D12 |
| f | `Retry-After` of the per-address budget | D6 |
| g | New crate dependencies | D3 |
| h | `create-admin` as an internal operation | D17 |
| i | Row image of `LoginUnlocked` with `login_attempts` | D8 |
| j | A failed `unlock` event after the commit | D8 |
| k | The session touch runs in read-only mode | D10 |
| l | `POST /me/password` without `HX-Request` | D13 |
| m | The final transaction checks the credentials again | D6, D7 |
| n | A storage error logs no row value | D13, D15 |
| o | The rule of the field `next` | D12 |
| p | The canonical client address | D11 |
| q | The size limit of the image of `LoginUnlocked` | D8 |
| r | The bound of the per-address budget | D6 |
| s | The pre-session cookie | D12 |
| t | A failed security event of a login or a CSRF check | D6, D12 |

Alternative: only `/login`, `/logout`, and `/me/password` over HTTP. Rejected, because no later step of the plan builds the pages of `/users`, and the security review needs the whole identity surface.

### D2. Module layout

- `recipe-domain::identity`: `IdentityError`, the secret value objects (D4), `TypedLogin`, `LoginThrottle` with `ThrottlePolicy` (D6), `KeyRing` (D4), and the last-administrator rule.
- `recipe-domain::audit`: the constructors `login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, and `unlock` of `SecurityEvent`. `LoginHmac::compute` takes a `KeyRing`.
- `recipe-app::identity`: the use cases, `HashGate`, `Argon2Hasher`, `IdentityContext`, `authenticate`, and `bootstrap_administrator`.
- `recipe-app::ports`: `UserRepository`, `SessionRepository`, and `AttemptRepository` as sub-ports of `Transaction`. `LoginAttemptStore` and `SessionStore` run their own transactions.
- `recipe-db`: one module for each new port. `security_events.rs` drops `count` and `occurred_at` from its insert (D15).
- `recipe-web`: `pipeline`, `csrf`, `client`, `cookies`, `error`, `i18n`, `pages`, `templates/`, `i18n/`, and `static/`.
- `recipe-server`: the module `create_admin` (D17).

`IdentityContext` holds an `AppContext`, the two pool ports, the `HashGate`, the login HMAC `KeyRing`, and `IdentitySettings`. `AppContext::new` does not change, so the use cases of P2.3 keep their signatures.

### D3. Dependencies (ADR-0035 g)

- `recipe-domain`: `hmac` 0.13.0.
- `recipe-app`: the feature `sync` of `tokio`, `argon2` 0.6.0, and `getrandom` 0.4.3.
- `recipe-web`: `axum` 0.8.9, `askama` 0.16.1, `tower-http` 0.7.1, `fluent` 0.17.0, `subtle` 2.6.1, `cookie` 0.18.2, `uuid` with `v7`, `serde`, `tracing`, and `time`. Dev: `tower` 0.5.3, `tracing-subscriber`, and `recipe-app` with the feature `memory`.
- `cargo info` gave the versions on 2026-10-14. Context7 has the docs of `argon2` 0.5.3 only. The API of 0.6.0 (`password-hash` 0.6) differs, so the implementer reads it in the crate source of the cargo registry.
- The router state is `Send + Sync`. The `fluent` bundle therefore uses the concurrent memoizer. The implementer checks its name with Context7 `/projectfluent/fluent-rs`.

Alternative: `axum-extra` for cookies. Rejected, because `cookie` alone builds and parses the three cookies of `api.md` sections 7 and 8.

### D4. Secrets and keys

- `Password` holds 1 to 128 characters of a form field. `NewPassword` adds the policy of A23.
- `PasswordHash` holds a PHC string with the prefix `$argon2id$`.
- `SessionToken` is 16 random bytes, 32 lower-case hex characters in the cookie. `SessionId` is the SHA-256 of these 16 bytes, 64 hex characters (`data-model.md` section 3.6). `SessionRef::of` takes the first 16 characters.
- `CsrfToken` is 32 random bytes, 64 hex characters. `recipe-web` compares it with `subtle`.
- `OneTimePassword::from_bytes` maps 20 random bytes to 20 characters of a 32-character alphabet without `0`, `O`, `1`, `l`, and `I`. It always meets the policy. `recipe-app` takes the bytes from `getrandom`.
- `KeyRing` reads one key per line, 64 lower-case hex characters, the newest first (ADR-0035 d). It signs with the first key and accepts each key (`reliability.md` section 7.1).
- The breached-password list is `crates/recipe-domain/data/breached-passwords.txt`. A static set loads it on first use, and `IdentityContext::new` forces that load at startup. The match is exact and case-sensitive. The file header names the source path, the commit, the license, and the line count.

These types hold a secret:

- `Password`, `NewPassword`, `PasswordHash`, `OneTimePassword`, and the result of `CreateUser` and `ResetPassword`.
- `SessionToken`, `SessionId`, `CsrfToken`, the pre-session nonce, and `Authenticated`.
- `KeyRing`, `LoginForm`, `PasswordForm`, `WebSettings`, and the settings of `create-admin`.

Each one has a hand-written `Debug` that prints `[redacted]` for the secret, no `Display`, and no `Serialize` (`reliability.md` section 3.4). A test proves each `Debug`.

Alternative: HIBP SHA-1 prefixes. Rejected by the user.

### D5. Ports

- `UserRepository` (in a transaction): find by login, lock by login `FOR NO KEY UPDATE`, count active administrators, insert, and the four updates of `data-model.md` section 6. Each write returns a `RowImage` without `password_hash` (ADR-0029). It also lists users and reads one user for the queries.
- `SessionRepository` (in a transaction): insert, delete one, and delete every session of a user.
- `AttemptRepository` (in a transaction): mark an attempt as a success, delete the attempts of a login, and read the throttle state of a login.
- `LoginAttemptStore` runs T1 of `Login` in its own transaction (D6). It cannot use `UnitOfWork::commit`, because T1 writes no audit entry.
- `SessionStore` authenticates a session with one statement (D10).
- `PasswordHasher` is synchronous. `Argon2Hasher` uses Argon2id, version `0x13`, and the values of `reliability.md` section 7.11. It computes its dummy hash once at construction, so a dummy check costs the same as a real check. A `HashFailure` gives `E-WEB-011` with the stage `PasswordHash`.
- `HashGate` holds a semaphore with `argon2.permits` permits and waits at most `argon2.acquire_timeout`. A timeout gives `E-WEB-009`.
- `HashPermit` holds an `OwnedSemaphorePermit`. It moves the permit into the `spawn_blocking` closure, so the permit lives until the hash ends, also when the caller drops its future. A `JoinError` gives `E-WEB-011`. With the feature `memory`, a blocking test double makes the flood test deterministic.

`recipe-app` reads the bytes of each token and one-time password with `getrandom`. A failed read gives `E-WEB-011` with the new stage `Token` of `TaskStage` (ADR-0035).

No use case waits for a permit while it holds a connection. `Login` and `ChangeOwnPassword` take the permit before T1, and read the user row after T1 outside a transaction. `CreateUser` and `ResetPassword` read before the permit and drop that read transaction first. This is how the design reads "before any database connection" of ADR-0014.

### D6. `Login`

```text
POST /login --> permit (2 s, else E-WEB-009)
                  |
                  v
                T1 (LoginAttemptStore, own transaction)
                  1. SELECT pg_advisory_xact_lock(1, hashtext(login))
                  2. one scan: pair failures, last pair failure,
                     address failures, oldest address failure, now()
                  3. INSERT attempt (succeeded = false), unless budget reached
                  4. COMMIT
                  |
         LoginThrottle::decide(count) -- throttled --> event login_throttled --> 429 E-IDN-005
                  |
                  v
                read user (read-only transaction) --> verify with permit
                  |                                  (dummy hash: unknown or inactive)
          wrong or unknown or inactive ---------> event login_failed or
                  |                               login_rejected_inactive --> 401 E-IDN-003
                  v
                T2 (UnitOfWork): lock app_user FOR NO KEY UPDATE,
                  check is_active, session_epoch, password_hash again (item m),
                  mark attempt, delete old session, insert session, commit UserLoggedIn
                  |
                  v
                303 /recipes or /me/password, Set-Cookie
```

- T1 runs the advisory lock as its own statement. Under READ COMMITTED, the next statement then sees every attempt that a request with the same login committed before it (finding F8).
- T1 counts with one scan of `login_attempt_ip_idx`: `count(*) FILTER (WHERE login = $1)`, `max(attempted_at) FILTER (WHERE login = $1)`, `count(*)`, and `min(attempted_at)` of the failures of the address in the window, and `now()`.
- `LoginThrottle::decide` works on the counts and on the database time of T1, so the clocks of the host and the database never mix. It clamps a negative elapsed time to 0. It never throttles when `delay(n)` is 0.
- T1 inserts the throttled attempt of a pair as failure `n + 1` at the time of T1. `retry_after` of the pair is therefore the full `delay(n + 1)`. At the budget, T1 inserts no row, and `retry_after` of the address is the rest of the window after its oldest failure (ADR-0035 f). Both round up to whole seconds, at least 1.
- `TypedLogin::parse` rejects a control character, because PostgreSQL cannot store `U+0000` in a text column. The web layer then returns `422 E-WEB-008` before the permit (ADR-0035).
- `UnlockLogin` can delete the attempt row between T1 and T2. The statement that marks the attempt as a success then changes no row, and T2 continues (ADR-0035).
- (r) The class-1 lock covers one login, not one address. Parallel requests with other logins from one address can pass the budget by at most `argon2.permits − 1`, because each request holds a permit during T1. The number of rows stays bounded.
- (m) T2 locks `app_user` `FOR NO KEY UPDATE`. It checks `is_active`, and it checks that `session_epoch` and `password_hash` equal the values that the check of step 4 read. Any difference rolls back and gives `E-IDN-003`. The attempt row stays a failure.
- `TypedLogin` is the ASCII lower-case form field with 1 to 64 characters. A longer field gives `422 E-WEB-008` before the permit. A typed login outside the grammar of `Login` counts as an unknown login.
- `InvalidCredentials` carries the cause `UnknownLogin` or `WrongPassword` for the event log only. `recipe-web` renders `E-IDN-004` as the body of `E-IDN-003` and logs `E-IDN-004` (A42). The log line of an unknown login holds no typed text.
- (t) When the `login_failed`, `login_rejected_inactive`, or `login_throttled` event cannot be written, the use case returns the storage error, as ADR-0029 does for `forbidden`. The attempt row of T1 still counts.
- `Login` takes the client address as `IpAddr`, not as an option, because `login_attempt.client_ip` is `NOT NULL`.

Alternative: the clock port decides the throttle. Rejected, because the attempt rows carry the database time.

### D7. `ChangeOwnPassword` (ADR-0035 b)

1. Check the policy of the new password. A failure gives `E-IDN-006` with no attempt row and no Argon2 work.
2. Take a permit. Run T1 with the own login and the client address. A throttled attempt writes `login_throttled` and returns `E-IDN-005`.
3. Check the current password. A wrong password returns `E-IDN-008`. The attempt row of T1 stays a failure (`data-model.md` section 6).
4. Hash the new password with the same permit.
5. One transaction locks the user `FOR NO KEY UPDATE`. (m) It requires `is_active`, a `session_epoch` equal to the epoch of the current session, and the `password_hash` that step 3 checked. Any difference rolls back and gives `E-WEB-004`.
6. The same transaction marks the attempt as a success and changes the password and the epoch. It deletes every session of the user, inserts the new session, and commits `OwnPasswordChanged`.

### D8. Administrative commands

- `SetUserRole` and `DeactivateUser` run `Isolation::Serializable`. `serializable_once_more` runs the body again once when the error is `Deadlock { cause: Serialization }`, also inside `AuditWriteFailed` and at `finish`. A second serialization failure returns `E-DB-001` (C6, ADR-0035 c).
- The last-administrator rule is a pure function of `recipe-domain`. It counts the active administrators under the lock of the target.
- `DeactivateUser` on an inactive user returns `E-IDN-010` and writes nothing (ADR-0035 a).
- `CreateUser` and `ResetPassword` hash the one-time password outside the transaction. They return it once in their result. No log line and no audit entry holds it.
- (i) The row image of `LoginUnlocked` is the `app_user` row with the child array `login_attempts`, as ADR-0029 does for limits. The before-image holds the deleted attempts, and the after-image holds an empty array.
- `UnlockLogin` locks the user row `FOR NO KEY UPDATE` for its row image. It deletes the attempts with `WITH gone AS (DELETE FROM login_attempt WHERE login = $1 RETURNING ...)`, so the image holds exactly the deleted rows. It takes the client address as `IpAddr`, as `Login` does.
- (q) The array holds at most `audit.unlock_rows` attempts, the newest first, and the field `login_attempt_count` holds the full number. ADR-0035 adds the row `audit.unlock_rows` = 100 to `reliability.md` section 7.11.
- (j) `UnlockLogin` writes the `unlock` event after the commit (`data-model.md` section 6). When that write fails, the use case returns the storage error. The unlock stays committed, and a repeated request deletes nothing more.

### D9. Queries

- `ListUsers` orders by `login COLLATE "C"`, filters with `strpos(login, $q) > 0`, and returns pages of `page.users` rows. ADR-0035 adds the row `page.users` = 100 to `reliability.md` section 7.11. A bad `q` or `page` gives `422 E-WEB-008`. `strpos` treats `_` as a plain character.
- `GetUser` returns the user and the throttle state of its login. The state holds the failures in the window over every address and the time of the last failure.
- `GetUserForm`, `GetOwnProfile`, `GetLoginForm`, and `GetOwnPasswordForm` check their requirement first and read only what their page shows.

### D10. Authentication of a request

- `authenticate` is not a use case. It turns the session cookie into `Option<Authenticated>`: the `CurrentUser`, the `SessionRef`, the `CsrfToken`, and the flag `must_change_password`.
- `SessionStore::authenticate` runs one statement: `UPDATE session s SET last_seen_at = now(), expires_at = now() + $idle FROM app_user u WHERE s.id = $1 AND u.id = s.user_id AND s.expires_at > now() AND s.created_at > now() - $absolute AND u.is_active AND s.epoch = u.session_epoch RETURNING ...`.
- No row means no session. A concurrent `DELETE session` therefore gives `Unauthenticated`, never `E-DB-008`.
- A route with the requirement `NoSession` ignores a missing or invalid session cookie. Every other route needs `Some`, else `E-WEB-004`.
- (k) The touch also runs in read-only mode. It is not a command, and it changes one row of a fixed size.

### D11. Request pipeline of `recipe-web`

```text
request
  -> 1 request_id, X-Request-Id
  -> 2 TraceLayer (no header, no body)
  -> 3 security headers, Cache-Control
  -> 4 client address (canonical, trusted proxies)
  -> 5 body limit
  -> 6 language (recipe_lang)
  -> 7 authenticate (Option<Authenticated>)
  -> 8 POST, PUT, DELETE: Origin check, then token check   -> 403 E-WEB-007 + csrf_rejected
  -> 9 password gate, read-only gate                       -> 303 /me/password, 503 E-WEB-006
  -> handler (route names its Operation) -> use case (authorize first)
```

- (p) Step 4 maps each address with `IpAddr::to_canonical()` before the CIDR match of `http.trusted_proxies`, before the throttle key, and before an `inet` bind. `::ffff:a.b.c.d` and `a.b.c.d` are therefore one address.
- `X-Forwarded-For` counts only from a trusted proxy. The right-most entry that is not a trusted proxy is the client. An entry that is not an address gives `422 E-WEB-008`, and the layer skips no entry. A missing `ConnectInfo` gives `500 E-WEB-010`, never a default address. A request from a trusted proxy without `X-Forwarded-For` also gives `500 E-WEB-010` (ADR-0035).
- A mutating request has any method except `GET`, `HEAD`, and `OPTIONS` (ADR-0035).
- Step 8 runs before step 9, so every forged request writes `csrf_rejected`, also in read-only mode and for a user with `must_change_password`.
- Each route names its `Operation`. A route without one fails closed with `E-WEB-010`. A test builds the route list from the router itself (D19), because `.layer()` covers only the routes that exist before it.

Alternative: one CSRF extractor for each handler. Rejected, because a new route could omit it. The layer buffers the body, which the limit of step 5 bounds.

### D12. CSRF and cookies

- Origin (ADR-0035 e): pass when `Sec-Fetch-Site` is `same-origin`, or when `Origin` equals `https://` and the `Host` header. `Origin: null`, another scheme, another port, and a missing `Host` fail.
- Session token: the header `X-CSRF-Token` or the form field `csrf_token`. The layer compares it with the token of the session in constant time.
- (s) Pre-session token: `GET /login` sets `__Host-recipe_csrf` to a 128-bit nonce and its HMAC-SHA256 under the first key of `csrf_signing_key`. The cookie has `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, and no `Max-Age`. The form field `csrf_pre` holds the nonce.
- `POST /login` always accepts the pre-session token, also with a session. `POST /lang` accepts the pre-session token or the session token. The check uses `Mac::verify_slice` under each key and compares the nonce in constant time. The cookie has no issue time, because the key rotation and the `__Host-` prefix bound its use.
- A failed check returns `403 E-WEB-007` and writes `csrf_rejected` through the sink. The event and the log line hold no token. (t) When the event cannot be written, the layer returns the storage error.
- (o) `POST /lang` accepts `next` only when it starts with `/` and its second character is neither `/` nor `\`. It also holds no `\` and no control or white-space character. Otherwise the route redirects to `/recipes`.
- `cookie` builds the three cookies with the attributes of `api.md` sections 7.1 and 8. The session cookie has no `Max-Age`, because the server enforces both timeouts. Logout and an invalid session clear it with `Max-Age=0`.

### D13. Errors, templates, and texts

- (l) `POST /me/password` without `HX-Request` gives `303` to `/recipes` with the new session cookie. The `200` fragment with `HX-Refresh: true` of `api.md` section 2.8 is the HTMX answer.
- `WebError` holds the variants of `reliability.md` section 1.2 and `App(AppError)`. One function gives the status, the fragment, the headers of `api.md` section 5, and `Retry-After`.
- That function writes the only log line of an error, at the level of `reliability.md` section 3.3, with `error_code` and `request_id`. `E-IDN-004` logs its own code and renders `E-IDN-003`. `E-IDN-007` gives `303` to `/me/password`.
- (n) The log line holds the code and the `Display` chain of the error, never its `Debug`. The `Debug` of a `sqlx` source holds the row values of the DETAIL, for example a `password_hash`.
- The askama templates live in `crates/recipe-web/templates/`. `base.html` holds the `htmx-config` meta tag of `api.md` section 9, `hx-history="false"`, the `hx-headers` with the CSRF token of the session, and `#error-panel`.
- No template holds an inline script, an inline style, or an event handler. No template applies `|safe` to a value that is not a literal.
- `static/htmx.min.js` is the newest HTMX 2 release, unchanged. A test compares its SHA-256 with a constant. `include_bytes!` serves it, so no file system access exists at run time.
- `i18n/en.ftl` and `i18n/pl.ftl` hold the page texts and the message of each code that these routes return. A test compares each English message with `reliability.md` section 1.2, without the sentence "Reference: {request_id}.".

### D14. Read-only gate

`ReadOnlyMode` is a shared flag that starts off. The gate returns `503 E-WEB-006` for a route whose operation does not run in read-only mode. The disk check of P4.1 sets the flag.

### D15. Migration `0005_security_event_guard.sql` (S6, ADR-0035)

- The file runs the five steps of `data-model.md` section 5.1. The `REVOKE` and the `GRANT` run inside `SET ROLE audit_owner`, and the `REVOKE` comes first.
- Step 3 revokes the table `INSERT` of `recipe_app` and grants `INSERT` on every column except `id`, `occurred_at`, and `count`. The upsert of the aggregated kinds still works, because the arbiter columns need only `SELECT` and the `SET` needs only `UPDATE (count)`.
- Step 3 also creates `security_event_guard()` and a `BEFORE UPDATE` trigger. The function is `SECURITY INVOKER` with `SET search_path = pg_catalog, public, pg_temp`. Its `EXECUTE` is revoked from `PUBLIC`.
- An update passes only when `NEW.count = OLD.count + 1` and `to_jsonb(NEW) - 'count'` equals `to_jsonb(OLD) - 'count'` under `IS DISTINCT FROM`. Otherwise the function raises `E-AUD-003` with SQLSTATE `23514`. Every operator is schema-qualified, as in ADR-0034.
- `recipe-db` maps `E-AUD-003` as a trigger code in `trigger_code()` and `trigger_messages`, in addition to the ranges of the archived design D4.
- `01-roles.sh` adds the function to the guard list of ADR-0034. (n) It also runs `ALTER ROLE recipe_app SET log_error_verbosity = 'terse'`, so the server log of `db` holds no DETAIL with row values.
- `grants.expected` gets the column grants. The static migration check and the coverage meta-test list the trigger and the function.
- The existing tests that insert a backdated `occurred_at` as `recipe_app` insert it as the superuser of the container. The lists of audit objects and their count grow by one (finding F11).
- `data-model.md` gets the SQL as the new section 5.4, and a document test compares it with the file. The SQL block of section 5.2 stays byte for byte, because `0003` shipped. Prose after it points to section 5.4.
- `01-roles.sh` runs only on an empty data directory. After the first release, a new audit object therefore needs a runbook step of the superuser for the guard list.

Alternative: a change of `0002` and `0003` in place. Rejected, because both files are on `origin/main`.

### D16. Lock failures (C6, ADR-0035 c)

`StorageError::Deadlock` gets the field `cause: LockFailure`. `40P01` gives `Deadlock`, `55P03` gives `LockTimeout`, and `40001` gives `Serialization`. The code stays `E-DB-001`, and `Display` does not change.

### D17. `create-admin` (ADR-0035 h)

- `recipe-server create-admin --login <login> [--password-file <path>] [--config <path>]`. Without `--password-file`, the subcommand reads one line from stdin. The secret-file rules of ADR-0031 apply to both sources.
- When stdin is a terminal (`std::io::IsTerminal`), the subcommand refuses to read it, because the password would show on the screen.
- It connects as `db.app_user`, a new row of `reliability.md` section 7.11, with `secrets.app_password_file`.
- `bootstrap_administrator` has no `Operation` and no permission, like `EnsureDerivedType`. It runs `Isolation::Serializable` with one more run on `40001`. It inserts the administrator with `must_change_password` and commits `UserCreated` with no actor.
- A test of `recipe-web` fails when its sources name `bootstrap_administrator`, because only `recipe-server` may call it (ADR-0013).
- An existing active administrator gives one `INFO` line and exit code 0. Every error gives one `ERROR` line and exit code 1.

### D18. Redaction tests (answer of the user)

- `recipe-web` tests install a `tracing-subscriber` with an own `MakeWriter` that copies every line into a buffer. They run the full router through: good and bad login, logout, a CSRF rejection, a password change, a reset, and `CreateUser`.
- The buffer must not hold a secret. The secrets are the password, the one-time password, `$argon2id$`, the session token, and the session id. They are also both CSRF tokens, the pre-session nonce, each cookie value, and the typed login of an unknown account.
- A response body can hold three secrets only. They are the one-time password of its own command, the CSRF token of its session, and the nonce `csrf_pre`.
- `recipe-server` reads the stdout of `create-admin` in its binary test with the same needles.
- Unit tests cover each `Debug` of D4 and each `Display` of `IdentityError`. The contract suite checks that no row image holds `password_hash`.
- A storage test forces a `NOT NULL` failure on `app_user` and on `session`. The `Display` chain of the error holds no `$argon2id$`, no session id, and no CSRF token.

### D19. Tests

- Endpoint × role: one row for each route and each principal. The rows come from `api.md` sections 2.1, 2.8, and 3.3.
- The principals are no session, Administrator, ProcessEngineer, Operator, a user with `must_change_password`, and a session of a deactivated user. An Operator on `/users/{unknown}` gets `403`, because the permission check comes before the lookup.
- Route coverage: the test takes the route list from the router. Each of the 10 mutating routes gets four bad requests: no token, the token of another session, a foreign `Origin`, and no `Origin` with no `Sec-Fetch-Site`. Each gives `403 E-WEB-007`, writes `csrf_rejected`, and changes nothing.
- Throttle: unit tests of `LoginThrottle::decide` with fixed instants, a negative elapsed time, and a property test that the delay never passes 60 seconds. Use case tests on `MemoryStore` with `MemoryClock`. Storage tests insert attempt rows with a fixed `attempted_at` as `recipe_migrator`.
- Flood: 32 parallel `Login` calls with the blocking test double. At most 4 checks run at one time, and the others get `E-WEB-009`. A dropped caller keeps its permit until the hash ends.
- Storage: the midway failure of each identity row of `data-model.md` section 6.
- Storage races:
  1. Two concurrent failed logins for one login: the count of the second includes the row of the first.
  2. Parallel failed logins with other logins from one address: at most 103 rows in the window.
  3. Two Administrators who demote each other: one `E-IDN-002`, and no `40001` reaches the caller.
  4. `DeactivateUser` while the session of the target sends requests: it succeeds, with at most one more run.
  5. A `40001` at `finish`: one more run.
  6. `ChangeOwnPassword` and `Login` against a `ResetPassword` that commits after their check: no new session.
  7. Two parallel `create-admin` runs: one administrator.
- A test with `pg_locks` filters by the test database and waits for the release after `drop(tx)`. The aggregation test waits until the second of the minute is below 50, so the three events share one minute.
- The contract harness seeds users through `UserRepository`. It no longer inserts `app_user` as `recipe_migrator`.
- An XSS corpus goes through each text field of the new pages. The field `next` also gets `/\evil.example` and `/<TAB>/evil.example`.
- The sink tests of R6 and Q4 write `login_failed`, `login_throttled`, and `csrf_rejected` through the port, not through SQL.

### D20. Settings

`IdentitySettings` holds the rows of `reliability.md` section 7.11 for Argon2, the throttle, the sessions, `page.users`, and `audit.unlock_rows`. `WebSettings` holds `http.trusted_proxies`, the body limits, `static.cache_max_age`, `http.hsts_max_age`, `http.retry_after`, and the cookie values. A test compares each `Default` with the table of section 7.11.

### Signatures

```rust
#[async_trait] pub trait LoginAttemptStore: Send + Sync {           // T1 of Login, own transaction
    async fn begin_attempt(&self, key: &AttemptKey, policy: &ThrottlePolicy)
        -> Result<AttemptCount, StorageError>; }
#[async_trait] pub trait SessionStore: Send + Sync {                // one UPDATE ... RETURNING
    async fn authenticate(&self, id: &SessionId, limits: &SessionLimits)
        -> Result<Option<SessionRow>, StorageError>; }
pub trait PasswordHasher: Send + Sync {                             // runs only under a HashPermit
    fn hash(&self, password: &NewPassword) -> Result<PasswordHash, HashFailure>;
    fn verify(&self, password: &Password, hash: Option<&PasswordHash>) -> bool; }
impl HashGate { pub async fn acquire(&self) -> Result<HashPermit, AppError>; }
impl LoginThrottle { pub fn decide(&self, count: &AttemptCount) -> ThrottleDecision; }
pub enum LockFailure { Deadlock, LockTimeout, Serialization }
```

```sql
CREATE TRIGGER security_event_guard BEFORE UPDATE ON security_event
    FOR EACH ROW EXECUTE FUNCTION security_event_guard();   -- only count = OLD.count + 1
```

## Risks / Trade-offs

- [About 45 tasks in one change] → Subagents work in sequence, three to four tasks each. The task groups follow the crate graph.
- [Argon2 with `m = 19 MiB` slows the HTTP tests] → The HTTP tests use `Argon2Hasher` with test values. A unit test checks that `Default` equals section 7.11.
- [HTTP tests run only on `MemoryStore`, because `recipe-web` cannot depend on `recipe-db`] → The storage tests run every identity use case on PostgreSQL. The contract suite keeps both adapters equal.
- [The `Host` header decides the Origin match] → A forged `Host` comes only from the client itself, and `SameSite=Strict` stays the first line (`api.md` section 7.3).
- [The per-address budget can pass 100 by 3] → The overshoot is bounded and documented (item r). A second lock per address would need a new advisory lock class.
- [The session touch writes one row on each request] → `app_user` stays below 100 users (A19). A rate limit of the touch would need a new row of section 7.11.
- [The breached-password list is a large file] → The filter keeps only lines that the policy length allows. The task reports the size before the commit.
- [`0005` changes the audit objects] → The five steps of section 5.1, the guard list of ADR-0034, and the grant test cover it.

## Migration Plan

The first deployment applies `0001` to `0005` to an empty database. `create-admin` then creates the first administrator. No configuration file changes, except the new key `db.app_user` with the default `recipe_app`. A rollback before the first release is a revert of the commits.

## Open Questions

None.
