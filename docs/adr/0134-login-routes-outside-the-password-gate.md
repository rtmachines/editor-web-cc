# ADR-0134: Login routes outside the password gate

Status: Accepted
Date: 2026-11-06
Deciders: project owner

## Context

`api.md` §7.4 says that a session with `must_change_password` can open only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang`. Every other route returns `303` to `/me/password`. The text names `GET /static/*` as outside the gate.

`GET /login` and `POST /login` are not in that list. They are also outside the gate in practice. These routes do not authenticate the session cookie (requirement `NoSession`), so the gate sees no session on them. `POST /login` reads the cookie only to delete that session row (`api.md` §7.3). A Basic row of the export routes also counts as open, because it has no session (ADR-0109 (l)).

A successful login deletes the session row of the old cookie (`api.md` §7.3, "Session fixation"). A failed login keeps that row. No normative document states that the login routes are outside the gate. The review of 2026-11-06 found this gap.

## Decision drivers

- The normative documents state the rule that the code applies.
- A user with the flag must be able to leave the browser session, for example to log in as another user.
- A login does not reuse the old session. A successful login deletes the old row.

## Considered options

### Option A: Add the login routes to the gate

The gate would read the session on the login routes and refuse them while the flag is on.

- Pros: the rule "a session with the flag opens only four routes" has no exception for `/login`.
- Cons: the code changes. A user with the flag cannot log in as another user from the same browser until the user logs out or changes the password.

### Option B: Record the current behavior

The login routes do not authenticate the session and stay outside the gate, like `GET /static/*`.

- Pros: no code change. The documents then match the code. A user with the flag can log in again, for example as a different user.
- Cons: the list of open routes in `api.md` §7.4 needs one more entry.

## Decision

The project uses **Option B**.

- (a) `GET /login` and `POST /login` do not authenticate the session cookie (requirement `NoSession`), so the gate sees no session. They stay outside the `must_change_password` gate. `POST /login` reads the cookie only to delete that session row.
- (b) A user with the flag can log in again, for example as a different user. A successful login deletes the session row of the old cookie (`api.md` §7.3). A failed login keeps it.
- (c) `api.md` §7.4 names the login routes next to `GET /static/*`, with the reason and a link to this ADR. This ADR also aligns the password gate requirement of `openspec/specs/identity-access/spec.md` and the rule line of `.claude/rules/web.md` with `api.md` §7.4.

## Consequences

### Positive

- The normative documents hold the rule. The code is no longer the only source.
- A user with the flag is never locked into a session.

### Negative

- None. The behavior does not change.

### Risks and mitigations

- Risk: a later change makes the login routes authenticate the session, and the gate then blocks them. Mitigation: item (a) states that these routes do not authenticate the session.
- Risk: no test checks that a session with the flag gets no `303` on `GET /login` or `POST /login`. This decision makes no code change and adds no test. A later change can break the rule without a failing test.

## Related

- ADR-0035 (s) (pre-session token)
- ADR-0109 (l) (Basic row counts as open)
- `api.md` §7.3 (session fixation: a successful login deletes the old session row), §7.4
- `openspec/specs/identity-access/spec.md` (password gate requirement), `.claude/rules/web.md`
- Code: `crates/recipe-web/src/gates.rs`, `crates/recipe-web/src/session.rs`, `crates/recipe-web/src/pages/own.rs`
- `overview.md` §7: R21
