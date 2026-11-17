# ADR-0190: A lost refusal event of Login, ChangeOwnPassword, and the Basic check amends ADR-0035 (t)

Status: Accepted. Amends ADR-0035 (t)
Date: 2026-11-17
Deciders: project owner

## Context

The semantic review of 2026-11-17 found a conflict between two accepted ADRs. The review record lists it as DC-1 (`docs/plan/review/2026-11-17-semantic-review.md`).

- ADR-0035 (t) says: "When the event `login_failed`, `login_rejected_inactive`, `login_throttled`, or `csrf_rejected` cannot be written, the request returns the storage error."
- ADR-0172 (q) gives another rule. After a failed event write, `Login`, `ChangeOwnPassword`, and the Basic check give their refusal or their throttle answer.
- `reliability.md` §1.1 and `domain-model.md` §5 state the rule of ADR-0172 (q).

The Status line of ADR-0035 does not name ADR-0172. The Status line of ADR-0172 does not name ADR-0035. The decision map row "Sessions, login, passwords, bootstrap" of `docs/adr/README.md` has the same gap.

The code follows ADR-0172 (q):

- `crates/recipe-app/src/identity/login.rs`, function `refuse`: a failed write gives one `WARN` line and no storage error.
- `crates/recipe-app/src/identity/change_own_password.rs`, the doc comment of the T1 step names ADR-0172 (o) and (q).
- `crates/recipe-app/src/identity/authenticate_basic.rs`, function `refuse`: a failed write gives the same answer.

The event `csrf_rejected` still follows ADR-0035 (t). `crates/recipe-web/src/csrf.rs`, function `reject`, returns the storage error when the write fails. The CSRF layer and the guard of the export routes (ADR-0055) call it.

## Decision drivers

- A reader finds the current rule of each event from the Status lines and the decision map (`docs/adr/README.md`).
- The documents follow the shipped code when the code meets the requirements (R34, R35).
- An accepted ADR changes only by its Status line and an "Amended by" note (`CLAUDE.md`).

## Considered options

### Option A: Record the amendment and its scope

- Pros: no code changes. The Status lines and the decision map then give the rule of each event.
- Cons: the rule of item (t) is then in two ADRs.

### Option B: Change the code back to ADR-0035 (t)

- Pros: one rule for the four events.
- Cons: a lost event of a refusal then gives `E-DB-*` in place of the refusal. ADR-0172 (q) rejected this, because the user then cannot see the reason of the refusal.

## Decision

The project uses **Option A**.

- (a) ADR-0172 (q) replaces ADR-0035 (t) for the events `login_failed`, `login_rejected_inactive`, and `login_throttled` of `Login`, `ChangeOwnPassword`, and the Basic check of the export routes.
- (b) After a failed write of one of these events, the request gives its refusal or its throttle answer. The detached task of ADR-0172 writes the one `WARN` line.
- (c) ADR-0035 (t) stays in force for the event `csrf_rejected`. A failed write of this event gives the storage error of the write.
- (d) The Status line of ADR-0035 gets the note "Amended by ADR-0172 and ADR-0190". The note names item (t) and the scope of items (a) and (c).
- (e) The Status line of ADR-0172 gets the note "Amends ADR-0035 (t), scope in ADR-0190".
- (f) Until the owner accepts this ADR, both notes carry the mark "(proposed)" after "ADR-0190".
- (g) The decision map row "Sessions, login, passwords, bootstrap" names ADR-0190.
- (h) No code, no spec, and no document in `docs/architecture/` changes.

## Consequences

### Positive

- The Status lines and the decision map agree with ADR-0172 (q) and with the code.
- The rule of `csrf_rejected` stays explicit.

### Negative

- A reader of ADR-0035 (t) must read ADR-0190 for the scope.

### Risks and mitigations

- Risk: a later change moves `csrf_rejected` to the detached task of ADR-0172. Then item (c) is false. Mitigation: that change needs a new ADR that amends ADR-0190 (c).

## Related

- ADR-0035 (t), ADR-0055, ADR-0172 (o), (q), ADR-0173
- `reliability.md` §1.1, `domain-model.md` §5
- Change `semantic-review-fixes`, doc conflict DC-1 of the semantic review of 2026-11-17
- `overview.md` §7: R34, R35
