# ADR-0132: Field answer of the own password form

Status: Proposed
Date: 2026-11-05
Deciders: project owner

## Context

ADR-0041 states the answer of a form with bad fields. The answer lists every bad field at once and puts a line under each field. ADR-0041 (g) kept one line per answer for the forms of the login, the own password, and the users "until step P4.2". Step P4.2 ran as the change `2026-10-26-hardening-round-1`, and no document states the rule after it (finding D5-5).

The state of the code:

- The forms of the users (`POST /users`, `POST /users/{login}/role`) already follow ADR-0041.
- `POST /me/password` stops at the first bad field. A bad new password gives an error page, not a field line.
- A wrong current password (`E-IDN-008`) marks no input with `aria-invalid`.
- `POST /login` stops at the first bad field. A wrong login or password gives the generic `E-IDN-003`.

## Decision drivers

- A user sees every bad field at once (A44, WCAG 2.2 SC 3.3.1).
- The login answer does not tell which value is wrong (`api.md` §7.4).
- One form has one answer rule.

## Considered options

### Option A: Keep one line for the three forms

- Pros: no code change.
- Cons: the own password form stays below A44.

### Option B: All three forms follow ADR-0041

- Pros: one rule for every form.
- Cons: the login form gains little. Its main error `E-IDN-003` names no field.

### Option C: The own password form follows ADR-0041, the login form keeps one line

- Pros: the form with three inputs gets the full answer. The login form keeps its generic answer.
- Cons: the login form stays an exception of ADR-0041.

## Decision

The project uses **Option C**.

- (a) `POST /me/password` checks the current password and the new password before it stops. Its answer lists each bad field with a line under the field (ADR-0041).
- (b) A new password against the password policy gives a line under the field `password`. The line keeps the code of the policy reason.
- (c) `E-IDN-008` gives a line under the field `current_password`, and that input gets `aria-invalid="true"`.
- (d) The forms of the users follow ADR-0041, as they do today.
- (e) `POST /login` keeps one line per answer. `E-IDN-003` names no field.
- (f) `recipe-web` parses the new password with the policy, so both fields get their lines in one answer. The use case `ChangeOwnPassword` takes the parsed `NewPassword` and does not check it again.
- (g) This ADR amends ADR-0041 (g).

## Consequences

### Positive

- The own password form meets A44.
- ADR-0041 has no open time limit.

### Negative

- The login form stays the one exception of ADR-0041.

### Risks and mitigations

- Risk: the line under `current_password` tells an attacker that the current password is wrong. Mitigation: only the owner of the session sees the form, and `E-IDN-008` already tells this today.

## Related

- ADR-0041 (g), ADR-0036
- `api.md` §5, §7.4, `design-system.md`
- Change `low-round-c`, capability `web-ui`
- `overview.md` §7: R36. §9: A44
