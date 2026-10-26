# ADR-0035: Identity, sessions, and the guard of security events

Status: Accepted. Amended by ADR-0096 (the known limit of code review M7 for `SetUserRole` and `DeactivateUser`)
Date: 2026-10-14
Deciders: project owner

## Context

Step P2.5 adds the Identity and Access commands, the session store, the login throttle, the HTTP pipeline, and `recipe-server create-admin`. The design of this step and its plan review found gaps between the documents and code that keeps every rule:

- `DeactivateUser` has no result for a user that is already inactive. The command would overwrite `deactivated_at` and end the sessions again.
- `ChangeOwnPassword` checks the current password without a throttle. A stolen session can guess the password at the speed of the Argon2 permits.
- `StorageError::Deadlock` holds `40P01`, `55P03`, and `40001` in one variant. The retry of `SetUserRole` and `DeactivateUser` cannot find `40001`.
- The key files of `login_hmac_key` and `csrf_signing_key` hold "32 bytes, one key per line" without an encoding.
- The Origin check needs the origin of the site, and no setting holds it.
- The per-address budget returns 429 with `Retry-After`, but no document gives the value.
- `overview.md` section 5 and ADR-0029 close the list of crates of `recipe-app`. Argon2, random bytes, and a semaphore need more crates.
- `create-admin` writes `UserCreated` through `UnitOfWork`, but `recipe_migrator` holds no privilege on `audit_log`. No setting names the role of the application.
- `recipe_app` can insert a security event with any `occurred_at` and `count`, and it can set `count` to any value (review S6 of P2.4).
- `Login` and `ChangeOwnPassword` check the password outside a transaction. A `ResetPassword` that commits after the check does not stop them.
- The `Debug` text of a database error holds the row values of the detail, also a password hash.
- The rule for the field `next` of `POST /lang` accepts `/\evil.example`. A browser turns it into `//evil.example`.
- A listener on `[::]` reports an IPv4 client as `::ffff:a.b.c.d`. The trusted-proxy list and the throttle key then see another address.
- No document gives a rule for the row image of `LoginUnlocked` or for the pre-session cookie.
- No document gives a rule for a failed write of a login event or for the session touch in read-only mode.

## Decision drivers

- No session survives a password reset, a deactivation, or a role change (R21, R28).
- No log line, error, or audit entry holds a secret (`reliability.md` section 3.4).
- The login path cannot crash the application or lock out the plant (R30, A22).
- The audit objects stay append-only for `recipe_app` (ADR-0022, ADR-0034).
- The code follows the documents (CLAUDE.md, source of truth).

## Considered options

### Option A: One ADR with the decisions of step P2.5

- Pros: the documents and the code agree. One record holds the decisions of one design step, as in ADR-0029.
- Cons: one ADR covers many related points.

### Option B: Record the decisions only in the design of the change

- Pros: no change of the documents.
- Cons: `data-model.md`, `api.md`, and `reliability.md` then contradict the code.

## Decision

The project uses **Option A**. The letters match the items of the design of the change `identity-and-sessions`.

Commands:

- (a) `DeactivateUser` on an inactive user returns `E-IDN-010 UserAlreadyInactive`. It changes no row and writes no audit entry.
- (b) `ChangeOwnPassword` checks the policy of the new password first. Then it takes an Argon2 permit and runs T1 of `Login` with the own login and the client address. A throttled attempt writes `login_throttled` and returns `E-IDN-005`.
- (m) The final transaction of `Login` and of `ChangeOwnPassword` locks the user row `FOR NO KEY UPDATE`. It requires `is_active`, and it requires `session_epoch` and `password_hash` equal to the values of the password check. `ChangeOwnPassword` also requires the epoch of the current session. A difference rolls back. `Login` returns `E-IDN-003`, and `ChangeOwnPassword` returns `E-WEB-004`.
- `UnlockLogin` can delete the attempt row of a `Login` or a `ChangeOwnPassword` between T1 and the final transaction. The statement that marks the attempt as a success then changes no row, and the command continues.
- A typed login with a control character gives `E-WEB-008` before the Argon2 permit, because PostgreSQL cannot store the character `U+0000` in a text column.
- (h) `create-admin` calls an internal operation of `recipe-app` with no `Operation` and no permission, like `EnsureDerivedType`. It runs `SERIALIZABLE` with one more run on `40001`. It reads the login from `--login` and the password from stdin or from `--password-file`. It refuses a terminal on stdin. It checks the password policy. It connects as the role of the new setting `db.app_user`. An existing active Administrator gives one `INFO` line and exit code 0.

Errors and logs:

- Known limit (code review M7): `SetUserRole` and `DeactivateUser` delete the sessions of the target in a `SERIALIZABLE` transaction. Each request of the target updates its session row. A target that sends requests without a pause can therefore cause `40001` twice, and the command returns `E-DB-001`. `ResetPassword` runs in `READ COMMITTED`, ends every session of the target at once, and stops the requests. The Administrator then repeats the command. Step P4.2 decides on a fix without this limit.
- (c) `StorageError::Deadlock` carries `cause: LockFailure`. `40P01` gives `Deadlock`, `55P03` gives `LockTimeout`, and `40001` gives `Serialization`. The code stays `E-DB-001`. The retry runs once more only on `Serialization`, also when it comes from the audit insert or from the commit.
- (n) The log line of an error holds the code and the `Display` text of each error in the chain, never the `Debug` text. `01-roles.sh` sets `log_error_verbosity = 'terse'` and `log_parameter_max_length = 0` for `recipe_app`. The server log of `db` then holds no detail with row values and no bind value of a slow statement.
- A failed read of random bytes for a token or a one-time password gives `E-WEB-011` with the stage `Token`.
- (t) When the event `login_failed`, `login_rejected_inactive`, `login_throttled`, or `csrf_rejected` cannot be written, the request returns the storage error, as ADR-0029 does for `forbidden`. The attempt row of T1 still counts.

Throttle:

- (f) T1 records a throttled attempt as a failure, so the attempt starts a new delay. `Retry-After` of the login delay is therefore the full delay of that attempt, `delay(n + 1)`, counted from the database time of T1. `Retry-After` of the per-address budget is the time until the oldest failure of the address leaves the window. Both round up to whole seconds, at least 1.
- Known limit (code review M9): the throttle counts failures per login and address and per address, but not per login over all addresses. A client with many addresses gets 3 free attempts at each address. Step P4.2 decides on a per-login limit in a new ADR.
- (r) The class-1 lock covers one login. Parallel attempts with other logins from one address can pass the budget by at most `argon2.permits − 1` rows, because each attempt holds a permit during T1. The number of rows stays bounded.

Sessions, CSRF, and HTTP:

- (d) A line of a key file holds the 32 bytes of a key as 64 lower-case hex characters.
- (e) A mutating request passes the Origin check when `Sec-Fetch-Site` is `same-origin`, or when `Origin` equals `https://` and the `Host` header.
- (s) `GET /login` sets `__Host-recipe_csrf` to a 128-bit nonce and its HMAC-SHA256 under the first key, with `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, and no `Max-Age`. `POST /login` always accepts this token, also with a session.
- (o) `POST /lang` accepts `next` only when it starts with `/` and its second character is neither `/` nor `\`. The path also holds no `\` and no control or white-space character.
- (p) The server maps each client address to its canonical form before the trusted-proxy match, the throttle key, and the database. A forwarded entry that is not an address gives `E-WEB-008`. A request from a trusted proxy without `X-Forwarded-For` gives `E-WEB-010`, because the address of the proxy must not become the throttle key of every client.
- Every method except `GET`, `HEAD`, and `OPTIONS` passes the CSRF and Origin checks, so a later `PATCH` route cannot skip them.
- (k) The session touch (`last_seen_at`, `expires_at`) also runs in read-only mode. It is not a command, and it changes one row of a fixed size.
- (l) `POST /me/password` without `HX-Request` gives `303` to `/recipes` with the new session cookie. The HTMX answer stays the `200` fragment with `HX-Refresh: true`.

Audit:

- (i) The row image of `LoginUnlocked` is the `app_user` row without `password_hash`, with the child array `login_attempts`. `UnlockLogin` locks the user row `FOR NO KEY UPDATE`, because the lock statement gives the row image (ADR-0029). The before-image holds the deleted attempts, and the after-image holds an empty array.
- (q) The array holds at most `audit.unlock_rows` attempts, the newest first. The field `login_attempt_count` holds the number of all deleted attempts.
- (j) `UnlockLogin` writes the `unlock` event after the commit. When that write fails, the use case returns the storage error, and the unlock stays committed.
- The migration `0005_security_event_guard.sql` grants `INSERT` to `recipe_app` only on the columns of `security_event` except `id`, `occurred_at`, and `count`.
- The same migration adds the trigger function `security_event_guard()`. An update passes only when `count` grows by 1 and no other column changes. Otherwise the function raises `E-AUD-003`.
- `01-roles.sh` adds the function to the guard list of ADR-0034. After the first release, a new audit object needs a runbook step of the superuser for that list.

Settings and dependencies:

- (g) `recipe-domain` adds `hmac`. `recipe-app` adds the feature `sync` of `tokio`, `argon2`, and `getrandom`. `recipe-app` also uses `time` with the feature `formatting` outside the feature `memory`, because the image of `LoginUnlocked` holds RFC 3339 text. `recipe-web` adds `subtle` and `cookie`.
- `reliability.md` section 7.11 gets the rows `db.app_user`, `page.users`, and `audit.unlock_rows`.

## Consequences

### Positive

- A race with an administrator command cannot create a session with old credentials.
- The throttle covers both password checks of the application.
- The audit objects of `security_event` stay append-only also for `count` and `occurred_at`.
- Every request has one defined result, also a repeated deactivation and an unlock with a failed event.

### Negative

- `Login` T2 and `ChangeOwnPassword` take one more row lock on `app_user`.
- The per-address budget is a bound of `100 + argon2.permits − 1` rows, not exactly 100.
- `recipe-app` has more dependencies.

### Risks and mitigations

- Risk: a later route skips the CSRF layer. Mitigation: a test builds the route list from the router and sends four bad requests to each mutating route.
- Risk: a later audit object misses the guard list. Mitigation: the grant test compares the guard list with the objects of `audit_owner` (`data-model.md` section 5.3).

## Related

- `overview.md` section 5
- `domain-model.md` section 5.3
- `data-model.md` sections 1.1, 3.7, 3.8, 4.11, 5.1, 5.3, 5.4, 6, 8, 10
- `api.md` sections 2.8, 7.1, 7.3, 7.4, 8
- `reliability.md` sections 1.1, 1.2, 7.1, 7.11
- ADR-0013 (permissions), ADR-0014 (sessions), ADR-0022 (security events), ADR-0029 (application layer), ADR-0031 (secret files), ADR-0034 (audit objects)
