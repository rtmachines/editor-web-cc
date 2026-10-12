# ADR-0004: Authentication with server-side sessions in PostgreSQL

Status: Superseded by ADR-0014
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "Store user logins and password hashes in the same database". It does not say how a logged-in user is identified on later requests. The UI is server-rendered (ADR-0001). An export endpoint for a transfer system may need machine access later (ADR-0002).

## Decision drivers

- Immediate revocation: an Administrator must be able to deactivate a user and end their access now (R24, R28).
- Same database for all authentication data (R21).
- Cookies are the natural transport for server-rendered pages and HTMX.
- No new service (ADR-0003).

## Considered options

### Option A: Cookie sessions stored in PostgreSQL

- Pros: revocation by deleting a row, works with HTMX without code, session data is in the backup, no key rotation problem.
- Cons: one database read per request, not suitable for a machine client.

### Option B: JWT bearer tokens

- Pros: stateless, works for machine clients.
- Cons: revocation needs a block list, which is a session store again. The browser needs JavaScript to attach the token. Key rotation is a new operational task.

### Option C: Sessions for the UI and API tokens for the export endpoint

- Pros: each client type gets the right mechanism.
- Cons: two mechanisms to maintain from day one, and no machine client exists yet.

## Decision

We use Option A now. We move to Option C when a transfer system needs the export endpoint.

- Sessions use a third-party session library with a PostgreSQL store (sqlx). The table is `session` (see `data-model.md`).
- The cookie is `HttpOnly`, `Secure`, `SameSite=Lax`. The session ID is random, 128 bits or more.
- A session expires after 30 minutes of inactivity and after 12 hours in total. Both values are configuration values (Assumption A21, `api.md` section 7).
- Passwords are hashed with Argon2id. The parameters follow the OWASP recommendation at the time of release. The PHC string is stored in `app_user.password_hash`.
- A failed login is recorded in the audit log without the password. After 5 failed logins for one login value in 15 minutes the next attempt gets HTTP 429. The counter is in the `login_attempt` table (Assumption A22, `api.md` section 7).
- `DeactivateUser` deletes all sessions of that user in the same transaction.
- Every state-changing request carries a CSRF token bound to the session.

## Consequences

### Positive

- Revocation is immediate and simple.
- The backup contains the sessions. A restore does not log everyone out.
- No JavaScript is needed for authentication.

### Negative

- Every request reads the session row. At the expected scale this cost is small (Assumption A19).
- A machine client cannot use the export endpoint until Option C is added.

### Risks and mitigations

- Risk: the session table grows without limit. Mitigation: a periodic task deletes expired sessions.
- Risk: the session cookie is sent over plain HTTP. Mitigation: `Secure` flag and TLS by the proxy (ADR-0003).

## Related

- ADR-0002 (export endpoint, future API tokens)
- ADR-0005 (login events in the audit log)
- ADR-0013 (permissions are checked per use case, not per route)
- `data-model.md` (`app_user`, `session`), `api.md` section 2
