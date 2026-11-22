# ADR-0216: Known limits of security events, sessions, and backup health

Status: Accepted. Amends ADR-0109 (ae)
Date: 2026-11-22
Deciders: project owner

## Context

The full repository review of 2026-11-21 names two limits that no document records. `docs/plan/review/2026-11-21-full-repo-review.md` §4 holds both. The owner added a third limit on 2026-11-22.

These facts shape the decision:

- `refuse` (`crates/recipe-app/src/auth/authorize.rs`, line 43) awaits the insert of the event `forbidden` on line 59. `csrf::reject` (`crates/recipe-web/src/csrf.rs`, line 198) awaits the insert of `csrf_rejected`.
- No step commits before these inserts. A client that closes the connection drops the request future before the commit, and the insert rolls back.
- ADR-0172 (g) limits the detached task to `Login`, `ChangeOwnPassword`, `UnlockLogin`, and `AuthenticateBasic`. ADR-0190 (c) keeps the inline write of `csrf_rejected`.
- `deploy/Caddyfile` has no `log` directive. So Caddy writes no access log. ADR-0109 (ae) writes as if Caddy has an access log.
- The trace layer of `crates/recipe-web/src/pipeline.rs` writes the line "request finished" only in `on_response` (lines 158 to 162). Its `on_failure` is `()`.
- A read of the code gives the two facts above. No test forces a dropped request.
- `change_password` (`crates/recipe-web/src/pages/own.rs`) builds the cookie on line 282 and renders the answer on line 287. It appends the cookie on line 290.
- A fault on line 282 or line 287 gives `WebError::Internal`, which is `500 E-WEB-010`. The command already committed and deleted every session of the user.
- ADR-0163 (a) and (b) do not list `change_password`. ADR-0219 Risks names a set that stays untrusted after a clock fault. No ADR gives the next step of the user for these cases.

## Decision drivers

- Every authentication and authorization failure goes into `security_event` (A33). A known gap in that record must be documented.
- The system handles and records errors and unexpected behavior (R35). The event logs serve debugging (R37).
- Data safety comes before performance (R31). A fix must not put a committed step before a refusal.
- The UI must be clear and reliable (R39). So the user must know the next step after each limit.

## Considered options

### Option A: Record the limits and the next step of the user

- Pros: no code change and no new risk. The documents state what the system does.
- Cons: the limits stay. An event can stay unrecorded, and a user can lose the session.

### Option B: Move each event write into a detached task

- Pros: a dropped future no longer cancels the insert.
- Cons: ADR-0190 (c) keeps the inline write of `csrf_rejected` on purpose. A detached task also needs a slot and a drain, as ADR-0172 states.

### Option C: Build the cookie and render the answer before the commit of `ChangeOwnPassword`

- Pros: a render fault then gives an error before any change.
- Cons: the command returns the new token only at the commit. The handler cannot build the cookie earlier.

## Decision

The decision selects Option A. One ADR holds the three limits, because each limit is a known gap with no code change.

- (a) A client that closes the connection before the commit of the insert can suppress one `forbidden` or one `csrf_rejected` event.
- (b) The alert `security_events` of `reliability.md` §3.7 reads `security_event`. A lost event that was the only one in `alerts.security_window` keeps this alert inactive.
- (c) A lost `forbidden` on a permission other than `UserManage` never reaches the alert `security_events`.
- (d) A dropped request leaves no line. Caddy writes no access log, and the trace layer writes no line for a dropped future.
- (e) The next step of items (a) to (d): none for the user. No log holds a trace of the suppressed event.
- (f) A render fault or a cookie fault after the commit of `ChangeOwnPassword` drops the new session cookie.
- (g) The user then sees `500 E-WEB-010`. The next request gets `E-WEB-004`, because the command deleted every session. The database keeps the new password.
- (h) The next step of item (f): the user logs in again with the new password.
- (i) A complete set that `backup-health` reads with a manifest time ahead of the clock stays untrusted (ADR-0219 (c)).
- (j) The set stays untrusted until the clock passes its manifest time minus `alerts.interval` (ADR-0219 Risks).
- (k) A fix of the clock does not clear the set sooner. The retention of ADR-0066 (k) compares the same manifest time, so no step deletes the set sooner.
- (l) The next step of item (i): the operator fixes the clock (ADR-0219 (i)). Then the healthcheck fails until the border of item (j).
- (m) Item (d) amends ADR-0109 (ae). Caddy writes no access log, because the Caddyfile has no `log` directive. The rule of `log_credentials` applies only after a change adds that directive.
- (n) `api.md` §2 (the row `POST /me/password`) and `reliability.md` §3.7 link to this ADR.

## Consequences

### Positive

- The documents name each limit and the next step of the user.
- No code change and no migration are necessary.

### Negative

- A client can still suppress one refusal event at a time. No log shows that event.
- A user can still lose the session after a stored password change.
- A set from a time with a clock ahead still fails the healthcheck after the fix of the clock.

### Risks and mitigations

- Risk: a client suppresses many `csrf_rejected` events. Mitigation: each suppressed event needs one connection that closes before the commit of its insert. The events of the other requests still reach the alert.
- Risk: a user does not know that the database keeps the new password. Mitigation: the next login with the new password succeeds. The old password gives `E-IDN-003`.
- Risk: a later change adds a `log` directive to the Caddyfile. Mitigation: item (m) keeps the rule of `log_credentials` of ADR-0109 (ae).

## Related

- ADR-0109 (ae) (the access log of Caddy), ADR-0163 (a), (b) (a success answer after a failed read)
- ADR-0172 (g) (the detached task), ADR-0190 (c) (the inline write of `csrf_rejected`)
- ADR-0066 (k) (the retention), ADR-0219 (c), (i) (the clock rule of `backup-health`)
- `reliability.md` §3.3, §3.6, §3.7, §7.7, §7.11
- `api.md` §2
- Change `review-design-fixes-2`
- `overview.md` §7: R31, R35, R37, R39. `overview.md` §9: A33
