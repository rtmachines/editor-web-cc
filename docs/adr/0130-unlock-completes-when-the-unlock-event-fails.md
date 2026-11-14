# ADR-0130: Unlock completes when the unlock event fails

Status: Accepted. Amends ADR-0035 (j). Amended by ADR-0172 (the detached task of `UnlockLogin` writes the `WARN` line of a lost event, not the handler)
Date: 2026-11-05
Deciders: project owner

## Context

`UnlockLogin` deletes the login attempts of one login and commits the audit entry `LoginUnlocked`. After the commit, it writes the security event `unlock` through `SecurityEventSink` in its own short transaction (`domain-model.md` §5, ADR-0015, ADR-0022).

ADR-0035 (j) states that a failed write of that event returns the storage error, and the unlock stays committed. The review of 2026-11-04 found the problem of this rule (finding D1-4).

- The Administrator gets an error page, but the login is unlocked.
- `api.md` §5 says that an error tells the user that the command did not complete.
- A retry deletes nothing more, but it commits a second entry `LoginUnlocked`.

The audit entry `LoginUnlocked` is the record of the unlock. The event `unlock` is a second record in `security_event` (A33).

## Decision drivers

- An error answer means that the command did not complete (`api.md` §5).
- The server logs a fault exactly once (`reliability.md` §1.1).
- An operator finds a lost security event in the log.
- A lower layer does not log an error (`reliability.md` §1.1).

## Considered options

### Option A: Keep the error of ADR-0035 (j)

- Pros: no code change.
- Cons: the error page contradicts `api.md` §5. A retry writes a second audit entry.

### Option B: Write the event before the commit

- Pros: a failed event stops the command before the unlock.
- Cons: a failed commit then leaves an event `unlock` without an unlock.

### Option C: Complete the command and log the lost event

- Pros: the answer tells the truth. The audit entry holds the unlock. The log holds the lost event.
- Cons: the use case returns a result with a fault inside it.

## Decision

The project uses **Option C**.

- (a) When the write of the event `unlock` fails after the commit, `UnlockLogin` completes. Its result tells that the event is lost and holds the storage error.
- (b) The handler of `POST /users/{login}/unlock` writes one `WARN` line for a lost event. The line holds the code of the storage error and the `request_id`. The handler shows the normal success answer.
- (c) This line is the one log place of that fault. The use case does not log it.
- (d) An error before the commit or of the commit stays an error of the command, as before.
- (e) This ADR amends ADR-0035 (j).

## Consequences

### Positive

- The answer matches the state of the database.
- The Administrator does not repeat a completed unlock.

### Negative

- `security_event` can miss one row `unlock`. The audit entry `LoginUnlocked` and the `WARN` line still record the unlock.

### Risks and mitigations

- Risk: a later command copies the pattern for a fault that must stop it. Mitigation: `domain-model.md` §5 names `UnlockLogin` as the only command with this rule.

## Related

- ADR-0035 (j), ADR-0015, ADR-0022
- `domain-model.md` §5, `api.md` §5, `reliability.md` §1.1
- Change `low-round-c`, capability `identity-access`
- `overview.md` §7: R20. §9: A22, A33
