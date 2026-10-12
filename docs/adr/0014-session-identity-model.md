# ADR-0014: Session and identity model with an owned session table

Status: Accepted
Date: 2026-10-12
Deciders: project owner

## Context

ADR-0004 chose cookie sessions in PostgreSQL through the store of a third-party session library. The design review found that this store cannot support the revocation rules of the domain model. The architecture review (F1), the security review (M4 and two high findings) and the PostgreSQL review (L3 and one high finding) reported the same gaps:

- The store keeps `user_id` inside an opaque `data BYTEA` column. No SQL statement can delete the sessions of one user.
- The store writes through its own connection. It cannot take part in the UnitOfWork transaction.
- `CurrentUser` and the role come from the session. Nothing reloads `role` or `is_active` from `app_user`. A demoted or deactivated user keeps access for up to 12 hours.
- The login path runs Argon2id with no concurrency limit. About 30 parallel requests exceed the 512 MiB container limit.
- A fixed lockout after 5 failures lets any host lock out every known login forever.
- The failure counter is read before the insert. Parallel requests all pass. A rollback discards the row.
- The bootstrap administrator password stays in the environment for the life of the install.

The owner accepted a progressive delay instead of a fixed lockout. This ADR holds the session, login, and password rules. `reliability.md` section 7.11 holds their values.

## Decision drivers

- Immediate revocation on deactivation, role change and password reset (R21, R28).
- The login endpoint must not crash or block a 24/7 system (R30).
- No permanent lockout of a plant operator by an attacker (R30).
- One normative cookie and session specification. ADR-0004 and `api.md` disagreed (traceability review B48).
- All authentication data stays in the same database (R21).

## Considered options

### Option A: Keep the library store, add a shadow table with user_id

- Pros: small change, the store stays a library.
- Cons: two sources of truth for one session, the shadow write is outside the store transaction, revocation still needs a per-request reload.

### Option B: Owned session table and a `recipe-db` session adapter

- Pros: `user_id` is a column with an index. Deletes happen inside the UnitOfWork. The token is hashed at rest. The CSRF token lives in the row.
- Cons: the project maintains its own store adapter (about 200 lines) and its own cleanup task.

### Option C: Stateless signed tokens (JWT)

- Pros: no session table.
- Cons: no immediate revocation without a deny list, which is a session table again. Rejected in ADR-0004 for the same reason.

## Decision

The project uses **Option B**.

Session table and cookie:

- `session(id, user_id, epoch, csrf_token, created_at, last_seen_at, expires_at)`. `id` is the SHA-256 of the cookie token. `user_id` has a foreign key and an index. `epoch` is the copy of `app_user.session_epoch` at login.
- Cookie `__Host-recipe_session`, Secure, HttpOnly, SameSite=Strict, Path=/, 128-bit random token. The server refuses to start with `secure = false` outside a development build feature.
- Login creates a new session and a new CSRF token. The old token never stays valid after login.
- `Login` and `ChangeOwnPassword` return the new token. `recipe-web` sets the session cookie from this result. No session library is used.
- Logout deletes the current session only.
- `SetUserRole`, `DeactivateUser`, and `ResetPassword` delete every session of the user. `ChangeOwnPassword` deletes every session of the user and creates a new session for the current request.
- A pre-session CSRF token (signed double-submit cookie) protects `POST /login` and `POST /lang`. The key of the Docker secret `csrf_signing_key` (32 bytes) signs the cookie. Every other POST, PUT, and DELETE carries the session CSRF token. The server also requires `Sec-Fetch-Site: same-origin` or a matching `Origin` header.

Identity per request:

- The request middleware loads `role`, `is_active`, `session_epoch` and `must_change_password` from `app_user` on every request. The session carries identity only.
- `DeactivateUser`, `ResetPassword`, `SetUserRole` and `ChangeOwnPassword` increase `app_user.session_epoch` and delete the sessions of the user in the same transaction. A session with an old epoch is invalid.
- `CreateUser` takes no password. Like `ResetPassword`, it generates a one-time password, shows it once, and sets `must_change_password = true`. The administrator never chooses a password for another user.
- `app_user.must_change_password` is set by `create-admin`, `CreateUser`, and `ResetPassword`. Until the user clears it, the session can reach only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang` (`api.md` 7.4).

Login throttling (values in `reliability.md` 7.11):

- After 3 failures per (login, client IP) in 15 minutes, an attempt within `2^(n-3)` seconds after the last failure gets 429 with `Retry-After`. The cap is 60 seconds. There is no permanent lockout.
- A per-IP budget of 100 failures in 15 minutes returns 429.
- Login order. First, the use case takes an Argon2 permit. A permit timeout returns 503 `E-WEB-009` (`AppError::Timeout`). Then T1 runs under `pg_advisory_xact_lock(1, hashtext(login))`, the advisory lock class 1 of `data-model.md` 1.4. T1 counts the failures, inserts the `login_attempt` row with `succeeded = false`, and commits.
- A throttled attempt releases the permit and gets 429. Otherwise the Argon2 check runs and the permit is released.
- The port `LoginAttemptStore` runs T1. T2 on success is one `UnitOfWork` transaction: it sets `succeeded = true`, creates the session, and writes `UserLoggedIn`. T2 on failure writes a `security_event` through the port `SecurityEventSink`. A failed attempt below the per-IP budget therefore always counts, also under parallel requests (`data-model.md` 6).
- Flood bound. After the per-IP budget is reached, T1 inserts no `login_attempt` row. The 429 response comes from the count alone.
- `security_event` rows of the kinds `login_failed`, `login_throttled` and `csrf_rejected` aggregate per `(client_ip, kind, minute)`. One row holds a `count`, and later events of that minute increment it (`data-model.md` 3.8).
- In read-only mode (Assumption A24) the login path writes `login_attempt` and `security_event` only for a successful login. A failed login writes only an event log line.
- `security_event.login_hmac` is the HMAC-SHA256 of the login with the key of the Docker secret `login_hmac_key` (32 bytes). `reliability.md` 7.1 gives the key rotation.
- `UnlockLogin` (Administrator) clears the counters of one login and writes a `security_event`.
- Unknown, inactive and wrong-password cases return the same 401 body and run the same hash work.

Password hashing:

- Argon2id with m = 19 MiB, t = 2, p = 1.
- One global semaphore with 4 permits guards every Argon2 call. The acquire timeout is 2 seconds, then the server returns 503 `E-WEB-009` with `Retry-After`. The semaphore is acquired before any database connection.
- Passwords have 12 to 128 characters and are checked against a local breached-password list.

Bootstrap: `recipe-server create-admin` reads the password from stdin or a secret file. It does nothing when an active Administrator exists.

## Consequences

### Positive

- Revocation is one `UPDATE` and one `DELETE` in the same transaction as the business change (I13 holds).
- A stolen database dump does not contain usable session tokens.
- The login path has a fixed memory budget: 4 × 19 MiB.
- An attacker can slow down one login, but cannot lock it.

### Negative

- The project owns a session store adapter and its tests.
- Every request costs one primary-key read on `app_user`.
- A throttled login gets 429 at once. The server holds no request open for the delay. The delay response takes no permit. T1 does.

### Risks and mitigations

- Risk: the per-request reload is skipped on one route. Mitigation: the permission-matrix test runs every route with a deactivated user and expects 401.
- Risk: the breached-password list grows stale. Mitigation: the list is a versioned file in the image, updated with each release.

## Related

- Supersedes ADR-0004.
- ADR-0013 (permissions), ADR-0022 (approval policy, security_event table).
- `api.md` section 7, `data-model.md` sections 3.5 to 3.8, `reliability.md` section 7.11.
- Review findings: architecture F1, F9. Security M1, M2, M4, M5 and two high findings. PostgreSQL L3 and one high finding. Traceability B48, B51.
