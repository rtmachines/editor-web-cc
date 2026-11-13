# ADR-0158: Request state of a form and the message of a lost answer

Status: Accepted
Date: 2026-11-12
Deciders: project owner

## Context

The UI and UX audit of 2026-11-12 (findings F1 and F3) reports two faults of the HTMX forms. A probe on `main` at `774c409` found both faults again.

- A form stays editable while its request is in flight. The answer swaps the form with the sent values. The edit forms of a parameter and of a translation row lose the text that the user typed after the send. This occurs on `200` and on `422`.
- `hx-sync="closest form:drop"` drops a second request. It does not stop the input.
- No code handles `htmx:sendError` or `htmx:timeout`. When the connection fails, `#error-panel` and `#toast` stay empty. When the server ran the command and the answer got lost, the page also shows nothing.
- The HTMX configuration of `api.md` section 9 has no `timeout`, so HTMX waits for an answer without limit.
- The save forms of a draft show no indicator. ADR-0049 gives an indicator only to the approve form and to the lifecycle forms.

Every fix needs a client script. `api.md` section 9 permits three scripts with fixed purposes. `hx-disabled-elt` needs no script, but a disabled control loses the focus. ADR-0049 (j) and `design-system.md` section 10.6 keep the focus where it is.

## Decision drivers

- A user never loses typed text without a warning (R31, R39).
- Every action gives a visible result, also when the answer does not arrive (R39).
- The focus stays on the control of the user (A44, ADR-0049 (j)).
- No user data in a script, no inline script, and an unchanged `htmx.min.js` (`api.md` section 9, ADR-0001).
- One purpose per script, and a small script that a review can read in full.

## Considered options

### Option A: A fourth script for the request state

- Pros: one small file holds the input lock, the busy state, and the message of a lost answer. The other scripts keep their purposes.
- Cons: one more permitted script and one more line limit.

### Option B: `hx-disabled-elt` on the controls of a form

- Pros: no script.
- Cons: a disabled control loses the focus, and Enter leaves the focus on `body`. The fix of F3 still needs a script.

### Option C: Extend `row-version.js`

- Pros: the count of scripts stays three.
- Cons: one file holds two purposes. Its limit of 20 lines must grow.

### Option D: A script that keeps the text typed after the send

- Pros: the user can type during the request.
- Cons: the script must merge the answer with later input. The canonical values of the answer then conflict with the typed text. The script needs about 60 lines.

## Decision

The project uses **Option A**.

- (a) A fourth permitted script `static/request-state.js` has at most 40 lines. `base.html` loads it with `defer` after `row-version.js`. It reads and writes no user data.
- (b) On `htmx:beforeRequest`, the script finds the form with `event.detail.elt.closest("form")`. A request of an element outside a form gets no lock. The script locks that form and records it under the `XMLHttpRequest` of the event (`event.detail.xhr`).
  - The script counts the open requests of each form, because two requests of one form can run at the same time. For example, a boosted link can sit inside a boosted form.
  - The lock sets `aria-busy="true"` on the form.
  - The lock sets `readonly` on each `textarea` and on each `input` whose type is not `hidden`, `checkbox`, `radio`, `submit`, `button`, or `reset`. It skips a control that is `readonly` already.
  - The lock marks each control that it changed with `data-request-lock`.
- (c) While a form has `aria-busy="true"`, the script refuses each change of a `select`, a check box, and a radio button of that form.
  - The script lets through Tab, each key with Ctrl, Alt, or Meta, and the function keys F1 to F12. So Shift+Tab, F5, Ctrl+R, and Alt+Left still work.
  - The script refuses each other key and each mouse click on those controls. The focus never moves because of the lock.
- (d) The script finds the form of the same `XMLHttpRequest` on `htmx:afterRequest` and on `htmx:onLoadError`. It lowers the count of open requests of that form. The count never goes below zero.
  - The form stays locked while a request of that form is open. The unlock runs when the last open request ends. It removes `aria-busy` and each `readonly` with `data-request-lock`.
  - HTMX fires `htmx:afterRequest` after an answer, a network error, an abort, and a timeout. After an answer, it fires the event after the swap.
  - When the answer handling throws, for example on a missing `HX-Retarget` target, HTMX fires `htmx:onLoadError` and no `htmx:afterRequest`.
  - A swap can replace the form. The unlock of the replaced form then changes no element of the page.
- (e) `base.html` holds `<template id="transport-error">` after `main` in each full page. The server renders it in the UI language. It holds the error fragment of `design-system.md` section 10.6 with the code `E-WEB-015` and its sentence.
- (f) On `htmx:sendError` and on `htmx:timeout`, the script puts a copy of that template into `#error-panel`. It empties `#toast` and shows the top of the panel. It does not use `innerHTML`.
- (g) The sentence says that the result is not known. It never says that the server did not do the action. The server may have run the request before the answer got lost.
- (h) On `htmx:beforeRequest`, the script removes a message of a lost answer from `#error-panel`.
- (i) `E-WEB-015` has a row in `reliability.md` section 1.2 and a constant in `recipe-domain/src/error_codes.rs`.
  - No Rust error variant returns the code, because no server answer exists. The HTTP column of the row holds `-`. The tests that read section 1.2 skip a row with `-`.
  - The sentence is the bundle key `error-E-WEB-015`, as for every other code.
  - The fragment has no reference, because the browser got no answer and so has no `request_id`. This is the second exception to `api.md` section 5.2, after the backstop of section 5.1.
- (j) The HTMX configuration gets `"timeout"` with the value of the new setting `htmx.request_timeout` of `reliability.md` section 7.11. That value is 45 s. It is above `http.request_timeout`, so the server answer `503 E-WEB-009` arrives first.
- (k) Each save form of a draft has the indicator of `form-in-progress`, as the approve form has. These forms are the edit form and the new form of a parameter, and the save form of a translation row. The rule of `design-system.md` for `.htmx-indicator` stays: it changes only `opacity`.
- (l) No form gets `hx-disabled-elt`. The focus rule of ADR-0049 (j) stays.
- (m) The CSP stays as it is. `script-src 'self'` covers the new file.
- (n) `app.css` gives `form[aria-busy="true"]` the rule `cursor: progress`. A form without an indicator therefore also shows its request.

This ADR amends these decisions:

- ADR-0001: the list of scripts.
- ADR-0036 (p): a fragment of a lost answer has no reference.
- ADR-0047 (b) and `design-system.md` section 10.7: the script empties `#toast` on a lost answer.
- ADR-0049 (j): the list of forms with an indicator.

It changes `api.md` sections 5.2 and 9, `reliability.md` sections 1.2 and 7.11, and `design-system.md` sections 10.6 and 10.7.

## Consequences

### Positive

- A user cannot type text that an answer then removes. The rule covers every HTMX form, also forms that a later change adds.
- A lost answer and a timeout give a visible message with a code. No earlier success message stays beside it.
- The save forms show the request in flight.

### Negative

- The user cannot type while a request is in flight. A slow answer blocks the input up to `htmx.request_timeout`.
- The project has four scripts. Each new purpose of a script still needs an ADR.
- A user cannot report a reference for `E-WEB-015`. The operator may find a log line for the request by its time and the user.

### Risks and mitigations

- Risk: the lock blocks a shortcut of the browser. Mitigation: point (c) lets through Tab, each key with Ctrl, Alt, or Meta, and F1 to F12.
- Risk: a screen reader does not announce the message, or `aria-busy` changes how it reads the focused field. Mitigation: the fragment has `role="alert"` (`design-system.md` section 10.6). The task list holds a manual check with a screen reader.
- Risk: the script leaves a form locked. Mitigation: point (d) unlocks on each end that HTMX reports. A browser test checks the unlock after `200`, `422`, a network error, an abort, a timeout, and a missing target. Another test checks that a form with two open requests stays locked until the second request ends.
- Risk: the script removes a `readonly` that the server set. Mitigation: the script unlocks only the controls with `data-request-lock`.

## Related

- ADR-0001, ADR-0036 (p), ADR-0041, ADR-0047 (b), ADR-0049 (j)
- `docs/architecture/api.md` sections 5.2 and 9, `docs/architecture/design-system.md` sections 10.6 and 10.7, `docs/architecture/reliability.md` sections 1.2 and 7.11
- `overview.md` section 7: R31, R38, R39. Section 9: A44
