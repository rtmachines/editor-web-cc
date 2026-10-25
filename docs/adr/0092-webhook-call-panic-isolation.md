# ADR-0092: Panic isolation of one webhook call

Status: Accepted
Date: 2026-10-25
Deciders: project owner

## Context

ADR-0063 (m) sends the webhook calls of one run of the job `alerts` at the same time. One `alerts.webhook_timeout` bounds all of them.

Step P4.1 builds the adapter of the port `AlertWebhook` in `crates/recipe-server/src/webhook.rs`. The function `run_bounded` differs from section 2.4 rule 6:

- The adapter spawns each call as a task of one `tokio::task::JoinSet`. It reads the results with `join_next_with_id`.
- Tokio catches the panic of a spawned task and returns it as a `JoinError`. The adapter maps that `JoinError` to `WebhookFault::Connection` for the entry of that call.
- The other entries of the run keep their results. The run ends normally, and the supervisor does not see the panic.
- Rule 6 names five places that catch a panic. This `JoinSet` is not one of them.

Facts of the call:

- A dependency of the call can panic, for example the DNS resolver of `hyper-util`.
- Without this isolation, the panic of one call ends the run of `alerts`. The supervisor of rule 4 then catches it and restarts the job after `job.restart_delay`.
- The port returns one entry for each condition, in the order of the argument. The job writes one `WARN` line for each entry with a `WebhookFault` (ADR-0063 (n)).

## Decision drivers

- A panic of one call does not stop the delivery of the other alerts of the run (R30, R37).
- A panic is a visible signal. The panic hook logs it and counts it (section 2.4 rule 2, A41).
- Rule 6 names every place that catches a panic.
- No new error code, no new number in section 7.11, and no new configuration key.

## Considered options

### Option A: The `JoinSet` of the calls isolates a panic of one call

- Pros: the other calls of the run keep their results. The job keeps its interval and gets no restart.
- Cons: rule 6 gets a sixth place. The entry of the call shows a connection fault, although the call panicked.

### Option B: The adapter raises the panic again with `std::panic::resume_unwind`

- Pros: rule 6 keeps five places. The supervisor of rule 4 catches the panic.
- Cons: the run loses the results of the other calls. The job writes no `WARN` line for them and restarts after `job.restart_delay`.

### Option C: The iteration of the job polls the calls without a spawn

- Pros: rule 6 keeps five places. Each call runs inside the span `job` of the iteration.
- Cons: a panic of one call ends the whole iteration. The run loses the results of the other calls, and the job restarts after `job.restart_delay`.

The user chose **Option A** in session 9. The user rejected Option B and Option C.

## Decision

- (a) The adapter of `AlertWebhook` spawns each webhook call of one run as a task of one `JoinSet`. The isolation solves one problem: a panic of one call must not cost the results of the other calls.
- (b) The adapter maps the `JoinError` of a call to `WebhookFault::Connection` for the entry of that call only. It calls no `catch_unwind` and no `resume_unwind`.
- (c) The other entries of the same run keep their results. The port keeps one entry for each condition, in the order of the argument.
- (d) The job `alerts` writes one `WARN` line for that entry, as for every `WebhookFault` (ADR-0063 (n)). The run ends normally, so the supervisor does not restart the job.
- (e) The panic hook of rule 2 runs before the join returns. It increments the panic counter and writes one `ERROR` event with the panic message and the location.
  - The condition `panics` of section 3.7 therefore counts a panic of a call.
  - The `ERROR` event and the `WARN` line are two events, as in ADR-0057 (f). The rule "an error is logged exactly once" of section 1.1 stays true.
- (f) The timeout keeps its rule. A call that is not ready when `alerts.webhook_timeout` ends gives `WebhookFault::Timeout`. The drop of the set then aborts that call.
- (g) The `JoinSet` of the webhook calls of one run of `alerts` is the sixth place of section 2.4 rule 6. The rule lives in that section. `.claude/rules/rust.md` names the six places and links to it.

## Consequences

### Positive

- A panic of one call does not stop the other alerts of the run.
- The job `alerts` keeps its interval, and the supervisor does not restart it.
- The event log and the condition `panics` still show the panic.

### Negative

- Rule 6 names one more place. A review checks one more place that catches a panic.
- The `WARN` line of the entry gives the text of a connection fault. Only the `ERROR` event of the panic hook shows the panic.

### Risks and mitigations

- Risk: a call panics in each run, and no alert reaches the webhook. Mitigation: the panic hook writes one `ERROR` event in each run. The condition `panics` becomes active after `alerts.panic_count` panics in `alerts.panic_window`.
- Risk: later code adds one more task set that hides a panic. Mitigation: rule 6 names every place, and a review rejects every other place (`.claude/rules/rust.md`).

## Related

- ADR-0057 (h) counts the places of rule 6. This ADR amends that item.
- ADR-0058 (f) keeps its text. The `JoinSet` of the connection tasks stays the fifth place, so ADR-0058 gets no note.
- ADR-0063 (m) and (n) keep their text. This ADR adds the result of a call that panics, so ADR-0063 gets no note.
- ADR-0060 (supervisor of the jobs), ADR-0061 (panic hook in step 1 of the start).
- `docs/architecture/reliability.md` section 1.1, section 2.4 rules 2, 4, and 6, sections 3.3 and 3.7.
- `.claude/rules/rust.md` (the list of the places that catch a panic).
- `crates/recipe-server/src/webhook.rs` (`run_bounded`), `crates/recipe-server/src/panic_hook.rs` (`install`), `crates/recipe-app/src/ports/webhook.rs` (`WebhookFault`).
- Baseline requirements: operations "Alert conditions" and "Restart of a panicked job". That requirement covers a panic of the iteration of a job. A panic of a call task stays inside the adapter.
- `overview.md` section 7: R30, R37. Section 9: A41.
