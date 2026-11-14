# ADR-0173: Detached event of a T2 refusal

Status: Accepted. Amends ADR-0172 (k), (l), (n), and (w)
Date: 2026-11-14
Deciders: project owner

## Context

ADR-0172 moves the span from T1 to the security event of four use cases into a detached task (`crates/recipe-app/src/identity/detached.rs`). Item (j) keeps T2 of `Login` and the step after the judge of the Basic check in the request future.

The code review of 2026-11-14 found four points in that design. The owner decided each point on 2026-11-14.

T2 can refuse after a passed judge. The request future then writes the refusal event itself:

- `Login` writes it in `refuse_changed` (`login.rs:157`, called at `:196`). This happens when the user changed between the judge and T2.
- The Basic check writes it in `confirm` (`authenticate_basic.rs:177`, called at `:89`). `confirm` calls `refuse` at `:214`.

A drop of the request future during that write loses the event and gives no `WARN` line. This breaks ADR-0172 (n). `ChangeOwnPassword` writes no event after its judge, because `replace` refuses with `E-WEB-004` only. `UnlockLogin` writes its event inside the task.

The task drops a result with an `Err` when the receiver is closed (`detached.rs:115`, `let _closed = sender.send(output);`). No line records that error.

The request future takes the gate permit and then the slot (ADR-0172 (h), (w)). A login can therefore hold an idle Argon2 permit for up to `argon2.acquire_timeout` while it waits for a slot. `UnlockLogin` takes a slot and no permit.

`drain` closes the task set and the slot semaphore in one lock section (`detached.rs:140`). A later `acquire` or `spawn` fails at once and runs no work. ADR-0172 (w) does not state this case.

## Decision drivers

- `security_event` records every authentication failure (A33).
- The server logs a fault exactly once (`reliability.md` §1.1).
- T2 stays in the request future (ADR-0172 (j)). No step after the judge runs for a client that is gone.
- `UnlockLogin` stays available during a login flood.
- The fix uses the existing slots and the existing task set.

## Considered options

### Option A: The judge task waits for the verdict of T2

- Pros: the task and its slot exist already. Only the event write moves into the task. T2 stays in the request future.
- Cons: the slot stays held during T2.

### Option B: A second detached task for the refusal event

- Pros: the judge task ends at once.
- Cons: the request future needs a second slot. A slot before T1 halves the capacity. A slot after T2 can fail, and then the event is lost.

### Option C: Run T2 in the detached task

- Pros: one task holds the full span.
- Cons: T2 then runs for a client that is gone. This breaks ADR-0172 (j).

### Option D: Record the T2 refusal as a known limit

- Pros: no code change.
- Cons: a T2 refusal can leave no event and no `WARN` line.

### Option E: Keep the order gate permit, then slot

- Pros: a login that waits for a permit holds no slot. `UnlockLogin` therefore gets a slot during a login flood.
- Cons: a login that waits for a slot holds an idle permit, at most for `argon2.acquire_timeout`.

### Option F: Take the slot, then the gate permit

- Pros: no permit waits idle.
- Cons: logins that wait for a permit can hold all slots. `UnlockLogin` then gets `E-WEB-009`.

## Decision

The project uses **Option A** for the T2 refusal and **Option E** for the order of the permit and the slot.

### Event of a T2 refusal (Option A)

- (a) The rule covers T2 of `Login` (`refuse_changed`) and the step after the judge of the Basic check (`confirm`).
- (b) T2 stays in the request future, as ADR-0172 (j) states. Only the write of the refusal event of T2 moves into the detached task.
- (c) After a passed judge, the task makes a second `oneshot`. It sends the sender of that `oneshot` with its judge result.
- (d) Then the task waits on the receiver and keeps its slot until T2 decides. This amends ADR-0172 (k).
- (e) On a refusal, the request future sends the event through the second `oneshot`. The task writes it with the shared function of ADR-0172 (m).
- (f) On a grant, the request future drops the sender. The task then ends with no write.
- (g) The request future waits until the task reports the end of the write. The answer therefore comes after the event, as before this ADR.
- (h) A drop of the request future after the send does not stop the write. The event comes exactly once. A failed write gives exactly one `WARN` line. This extends ADR-0172 (n).
- (i) When the task ends without its report, the request future still gives its refusal. The panic hook or the drain of `reliability.md` §7.3 writes the line of that end.
- (j) The wait of the task needs no new timer. A drop of the request future drops the sender and ends the wait.
- (k) Known limit. A drop during an await of T2 before its decision writes no event. The awaits are `begin` and `lock_by_login` of `Login`, and `confirm_basic` of the Basic check.
- (l) In that window, the attempt row of T1 can stay without a success mark and without an event. A drop after a passed judge leaves the same state (ADR-0172 (l)).

### Error result with a closed receiver

- (m) Owner decision 2026-11-14: when the receiver is closed and the result is an `Err`, the task writes one `WARN` line. This amends ADR-0172 (l).
- (n) The line holds `error_code` and `chain`. The span of ADR-0172 (t) gives it the `request_id` of the request.
- (o) The task is the one log place of that error. The `WebError` mapping never sees the error, because no request future receives it.
- (p) `record_security_event` logs only the storage fault of the event write. That fault is a different error, so the rule of `reliability.md` §1.1 holds.
- (q) A `spawn_blocking` panic inside the task gives the line of the panic hook and the `WARN` line of the `TaskFailed`. These are two events, as in ADR-0172 (ac).
- (r) A result with `Ok` and a closed receiver gives no line.

### Order of the permit and the slot (Option E)

- (s) Owner decision 2026-11-14: the request future takes the gate permit or the Basic permit first. Then it takes the slot. ADR-0172 (h) and (w) keep this order.
- (t) A login that waits for a permit holds no slot. Logins that wait for a permit therefore cannot block `UnlockLogin`, which takes a slot and no permit.
- (u) The cost is an idle permit while all slots are in use. `argon2.acquire_timeout` bounds that wait, and the end of the wait gives `E-WEB-009`.
- (v) No holder of a slot waits for a permit. The two waits therefore cannot form a cycle.

### Slot after the drain

- (w) `drain` closes the task set and the slot semaphore in one lock section (`detached.rs:140`).
- (x) After the drain, `acquire` fails at once with `Timeout { stage: DetachedSlot }` (`503 E-WEB-009`). It does not wait for `argon2.acquire_timeout`. This amends ADR-0172 (w) for the stop.
- (y) A `spawn` after the drain runs no work and gives the same error. T1 therefore does not run, and nothing commits.
- (z) The close makes sure that no task starts that the drain does not see. A task that started before the drain stays in the drain of ADR-0172 (af) to (ah).
- (aa) The drain of the detached tasks starts after the join of the public listener (`serve/opened.rs:187`). A task that waits for a T2 verdict then has no sender, so it ends with no write.

### Tests of the code tasks

- (ab) `Login`: a drop of the request future during the write of the T2 refusal event still writes the event exactly once.
- (ac) `Login`: the same drop with a failing sink gives exactly one `WARN` line of a lost event.
- (ad) The Basic check: the two tests of items (ab) and (ac) for the refusal of `confirm`.
- (ae) A grant of T2 ends the task with no write, and the slot returns.
- (af) A drop during an await of T2 before its decision ends the task with no write, and the slot returns.
- (ag) A closed receiver and an `Err` result give one `WARN` line with `error_code` and `request_id`. An `Ok` result gives no line.
- (ah) With all Argon2 permits held and a login that waits for a permit, `UnlockLogin` gets a slot at once.
- (ai) After the drain, `acquire` and `spawn` fail at once with `E-WEB-009`, and `Login` commits no attempt row.

## Consequences

### Positive

- A T2 refusal keeps its event after a dropped request.
- An error result that no request receives gets exactly one `WARN` line.
- A login flood does not take the slots of `UnlockLogin`.
- After the drain, an identity command commits nothing and gets an answer at once.

### Negative

- `Login` and the Basic check hold their slot during T2.
- A drop during an await of T2 before its decision still writes no event.
- A login can hold an idle permit for up to `argon2.acquire_timeout`.

### Risks and mitigations

- Risk: the slots run out sooner during a login flood. Mitigation: T2 is one short transaction. `identity.detached_tasks` is two times `argon2.permits`, and a late slot commits nothing.
- Risk: a request future that never sends and never drops holds a slot. Mitigation: `http.request_timeout`, a closed connection, and the stop of `reliability.md` §7.3 drop the request future.

## Related

- ADR-0035 (m), ADR-0109 (m), ADR-0130 (a), ADR-0172 (h) to (n), (t), (w), (ab) to (ah)
- `reliability.md` §1.1, §2.4, §7.3, `domain-model.md` §5
- Change `review-design-fixes`, capability `identity-access`
- `overview.md` §7: R20, R30, R35. §9: A33
