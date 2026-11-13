# Proposal

## Why

The UI and UX audit of 2026-11-12 (`docs/plan/review/2026-11-12-ui-ux-browser-audit.md`) found two faults of the HTMX forms. A browser probe on `main` found both faults again.

- F1: the answer of a save swaps the form with the sent values. Text that the user typed after the send goes away on `200` and on `422`. This occurs in the edit form of a parameter and in a translation row.
- F3: a network error or a lost answer gives no message. In the probe the server stored the value, and the page showed nothing.

Each fix needs a client script, so the change records ADR-0158. The change satisfies R31, R38, and R39. It uses A44.

## What Changes

- New ADR-0158 and a fourth permitted script `static/request-state.js` with at most 40 lines.
- While a request of a form is in flight, the form has `aria-busy="true"`. Its text inputs are `readonly`, and its selects and check boxes refuse a change. The focus does not move.
- After each end of the request, the script unlocks the form. An end is an answer, a network error, an abort, a timeout, or a failed answer handling. A busy form shows the cursor `progress`.
- A network error or a timeout puts a message with the new code `E-WEB-015` into `#error-panel` and empties `#toast`. The message says that the result is not known. It has no reference, because no request reached the log.
- The HTMX configuration gets `timeout` with the new setting `htmx.request_timeout` of 45 s.
- The edit form and the new form of a parameter and the form of a translation row get the indicator of `form-in-progress`.
- Documents: ADR-0158, `api.md` sections 5.2 and 9, `reliability.md` sections 1.2 and 7.11, `design-system.md` sections 10.6 and 10.7, and `.claude/rules/web.md`. The Status lines of ADR-0001, ADR-0036, ADR-0047, and ADR-0049 change too.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: new requirements for the request state of a form and for the message of a lost answer. Changed requirements for the error fragment content, the static files, the HTMX configuration, the permitted scripts, and the indicator of a save form.

## Impact

- Crate `recipe-web`: `static/request-state.js`, `static/app.css`, the static file table and its route test, `templates/base.html`, `templates/components/alert.html`, `templates/components/grid.html`, `templates/translation-row.html`. It also covers the key `error-E-WEB-015` in `en.ftl` and `pl.ftl`, the registry test of `error/status/tests.rs`, and their tests.
- Crate `recipe-domain`: the constant `E-WEB-015` in `error_codes.rs`.
- `e2e/tests/`: browser tests for the lock, the unlock, and the message.
- No change of the routes, the form fields, `recipe-app`, `recipe-db`, or the CSP.

## Out of scope

- F2, F4, and F5. The change `ui-form-fixes` takes them.
- A merge of typed text into the answer (ADR-0158, option D).
- A retry of a request by the browser.
- `hx-disabled-elt` on any form.
