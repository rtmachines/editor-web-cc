# ADR-0163: Success answer after a failed read

Status: Accepted. Amends ADR-0041 (h)
Date: 2026-11-14
Deciders: project owner

## Context

Many handlers of `recipe-web` run a command and then read the new state for the answer. The read runs in a new transaction after the commit.

The read can fail. Examples are a pool timeout (`E-DB-004`), a statement timeout, or a concurrent `ArchiveMachine` that gives `404`. The handler then returns `WebError`, and the user sees an error for a stored change.

The client then gets no `HX-Trigger` with the new `row_version`. After a parameter removal, the next save gets `409 E-WEB-001`. After `AddUnit`, the form keeps the segment, and a second submit gets a duplicate unit path.

The full repository review of 2026-11-13 reports this defect as F-01 (`docs/plan/review/2026-11-13-full-repo-review.md` §4.1). No ADR accepts an error answer after a commit. ADR-0041 (h) says that a success answer shows the saved state.

The doc comment of `refreshed` in `pages/users.rs` gives the old rule. A read error goes to the `WebError` mapping, because a second try of the command is harmless.

## Decision drivers

- A committed change never gives an error answer (`CLAUDE.md`, purpose: data safety first).
- The user knows that the page can show an old state.
- The read fault goes to the event log once.
- No new error code and no new script.

## Considered options

### Option A: A fixed notice in the success answer

- Pros: the answer is true. One shared function serves every handler. The `GET` of the page reads the state again.
- Cons: the answer shows no new state. The user must reload the page.

### Option B: One more read after the failure

- Pros: the answer can show the new state.
- Cons: the second read can fail in the same way under the same load.

### Option C: An answer from the command output alone

- Pros: no read after the commit.
- Cons: each answer shows state that only the read gives.

## Decision

The project uses **Option A**.

- (a) The rule covers each handler that reads after a committed command. The handlers are `remove` (`parameters/remove.rs`), `save` (`parameters/edit.rs`), `set_value` and `replace_limits` (`parameters/value.rs`), and `rebind` (`versions/rebind.rs`).
- (b) The rule also covers `approve` (`approval/approve.rs`), `retire` (`versions/actions.rs`), `add` (`units/add.rs`), and `set_role`, `deactivate`, `activate`, `unlock` (`users.rs`).
- (c) When the read fails, an HTMX request gets `200` with a fixed notice in `#toast`. The notice says "The change is saved. Reload the page."
- (d) The answer sets `HX-Retarget: #toast`, `HX-Reswap: innerHTML`, and `HX-Push-Url: false`.
- (e) The answer also holds an empty `#error-panel` with `hx-swap-oob="innerHTML"`, like `parameter-toast.html` and `password-changed.html`. A stale error line then goes away.
- (f) The answer sets `HX-Trigger` with `rowVersion` only when the output of the command holds the new value. `DraftOutput` holds it.
- (g) The outputs of `ApproveVersion`, `RetireVersion`, `AddUnit`, and the four user commands of item (b) do not hold it. Their notice answer has no `HX-Trigger`.
- (h) A request without HTMX gets `303` to the page of the changed object. The `GET` of that page reads the state again.
- (i) The handler writes one `WARN` line with `error_code`, `request_id`, and `chain`, like `log_lost_unlock_event` (ADR-0130 (b)). This line is a new log place of `reliability.md` §1.1.
- (j) One shared function in `recipe-web` builds the notice answer and writes the line. The `pl` and `en` bundles get the text of the notice.
- (k) The notice answer swaps no form out of band, because no read gives the saved form. This amends ADR-0041 (h) for this case only.
- (l) The doc comment of `refreshed` in `pages/users.rs` changes with the four user handlers.
- (m) A test needs a read that fails after the commit. `MemoryStore` gets a fault that fails the first `begin` after a commit.

## Consequences

### Positive

- A committed change gives a success answer. The user sees no false error.
- The read fault is in the event log once, with its code and the `request_id`.
- A draft page keeps a valid `row_version` after a notice answer of a draft command.

### Negative

- The notice answer shows no new state. The user must reload the page.
- `api.md` §4 and §5, `reliability.md` §1.1, `CLAUDE.md`, and `.claude/rules/rust.md` get the notice answer and the new log place.

### Risks and mitigations

- Risk: a user misses the notice and acts on a stale page. Mitigation: the notice tells the user to reload. A stale `row_version` gives `409 E-WEB-001`.

## Related

- ADR-0036, ADR-0037 (z), ADR-0041 (h), ADR-0130 (b)
- `api.md` §4, §5, `reliability.md` §1.1
- Change `review-design-fixes`, capability `web-ui`
- `overview.md` §7: R31, R35, R39
