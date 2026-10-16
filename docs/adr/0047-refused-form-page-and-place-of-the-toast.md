# ADR-0047: Refused form page and place of the toast

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

ADR-0041 (e) gives an HTMX request with field errors the summary in `#error-panel` and the form out of band. The form keeps the sent texts and shows each error line under its input.

The code review of step P3.3 found these gaps in the answers around a form:

- A request without HTMX gets the full error page of ADR-0036 (i). That page has no session layout and no form. The user loses every sent text and must type the form again.
- The full error page shows the label of a field without a link (ADR-0036 (o)). A keyboard user cannot go from the summary to the bad input.
- ADR-0036 (m) keeps a success message in `#toast` until the next swap. An HTMX error answer goes into `#error-panel` and does not touch `#toast`. So an old success message can stand next to a new error.
- The toast of a save at the foot of a long page can stand above the view. HTMX 2.0.11 applies no `show:` modifier to an out-of-band swap, so the page does not scroll to the toast.
- The removal of a parameter answers with no toast. The removal of a translation row already sends the toast `translation-deleted`.

The error layer runs before the session layer, so it has no session of its own. A form handler has the session when it refuses a form.

## Decision drivers

- A user never loses the sent input of a form (R39, ADR-0041 (e)).
- Each error line links to its input, and the page has one `h1` (A44, WCAG 2.4.3).
- A success message and an error never stand side by side (R39).
- A focused element never hides under the toast (A44, WCAG 2.4.11).
- No new script, no inline style, and each `hx-*` value is a literal (`api.md` section 9, ADR-0037).
- The full error page stays the answer when no session is available.

## Considered options

### Option A: A full page of the refused form in the session layout

The form handler copies the session parts of the layout into the error. The error layer renders the page with the session layout, the summary, and the form.

- Pros: the user keeps the input and sees the error lines. No new route, no new storage, and no redirect.
- Cons: the error carries the CSRF token of the session, so its `Debug` must hide the token.

### Option B: A redirect `303` to the form page with the errors in the session

- Pros: the browser shows the address of the form page.
- Cons: the session table must store the errors and the sent texts for one request. This is new state in the store for a rare path.

### Option C: Keep the full error page

- Pros: no change.
- Cons: the user loses the input, and the summary has no link to an input.

For the toast, the options are a script that removes the toast, an out-of-band swap of an empty `#toast`, or no change. A new script is forbidden (`api.md` section 9). For the place of the toast, the options are a scroll to the toast or a sticky place. HTMX 2.0.11 does not scroll for an out-of-band swap.

## Decision

The project uses **Option A**, an empty `#toast` out of band, and a sticky `#toast`. This ADR amends ADR-0036 in (i), (m), and (o).

- (a) A request without HTMX can get field errors with a form (`WebError::Fields`). With a session, it gets the page of the refused form.
  - The form handler copies the session parts into the error with `SessionFrame`. These parts are the navigation of the role, the CSRF token, and the login of the user.
  - The page has the normal layout: the skip link, the top bar with the navigation, the login, and the logout button, and `main`.
  - `<body>` holds the CSRF token of the session in `hx-headers`.
  - `#error-panel` holds the summary of `field-errors.html`. Each line of the summary links to its input.
  - `main` holds one `h1` with the text of the key `error-page-title`. The page title has the form `<code> · <error-page-title> · <app-name>`.
  - The form follows the `h1`. It keeps the sent texts and shows each error line under its input (ADR-0041 (e)). The page form has no `hx-*` attribute, so a second save is a normal `POST` also when htmx runs on the page.
  - The status of the page is the status of the error.
  - The page answers a `POST`, so it has no language switch (ADR-0036 (j)).
  - Every other error, and a request without a session, keeps the full error page of ADR-0036 (i).
  - This point amends ADR-0036 (i), the full error page with the application name only, and (o), the label without a link.
- (b) Every HTMX error answer with a body ends with an empty `#toast` element out of band: `<div id="toast" hx-swap-oob="innerHTML"></div>`.
  - The swap replaces only the content of `#toast`. The region keeps `role="status"` and `aria-live="polite"`.
  - An old success message therefore disappears when an error comes.
  - This point amends ADR-0036 (m): a success message stays until the next success answer or the next HTMX error answer.
- (c) `#toast` stays in view.
  - `app.css` gives `#toast` the rules `position: sticky`, `top: 0`, and `z-index: 1`. So the toast stands above the sticky path cells of the leaf grid.
  - `app.css` gives `html` the rule `scroll-padding-top: calc(var(--target) + var(--space-4))`. A focused element therefore does not hide under the toast (WCAG 2.4.11).
  - `role="status"` and `aria-live="polite"` of `#toast` do not change.
  - The sticky place replaces a scroll, because HTMX 2.0.11 applies no `show:` modifier to an out-of-band swap.
- (d) The removal of a parameter sends the toast `parameter-deleted` with the tag. The removal of a translation row sends the toast `translation-deleted` with the language tag.
  - An HTMX request with `HX-Target: content` gets the toast inside `#toast` of the content answer.
  - Another HTMX request gets the parameter table, then the toast and an empty `#error-panel` out of band.
  - A request without HTMX gets `303` and no toast, as before.

## Consequences

### Positive

- A user without HTMX keeps the sent input and reaches each bad input from the summary.
- No answer shows a success message next to an error.
- The toast is visible after each save, also at the foot of a long page.

### Negative

- `WebError::Fields` carries the session parts of the layout. `RenderedForm` keeps them in a box, so `WebError` stays small (`clippy::result_large_err`).
- The answer inserts the form text before the end tag `</main>` of the rendered page. The form text comes from an askama template, so no template uses `|safe`.
- Every HTMX error answer is longer by one element.
- The sticky toast covers the top of `main` while it holds a message.

### Risks and mitigations

- Risk: the session parts hold the CSRF token, and a log line could show it. Mitigation: `SessionFrame` has no `Debug`, and the `Debug` of `RenderedForm` writes only the length.
- Risk: a form handler does not copy the session parts. Mitigation: `refused` in `pages/draft_reply.rs` copies them for each request without HTMX. The form handlers of the recipe pages and of the plant pages call `refused`.
- Risk: a later HTMX version scrolls an out-of-band swap and the sticky place is then not necessary. Mitigation: the sticky place stays correct with a scroll.

## Related

- ADR-0036, amended by this ADR in (i), (m), and (o)
- ADR-0037 (z), ADR-0040, ADR-0041 (e)
- `docs/architecture/design-system.md` sections 10.1, 10.5, 10.6, and 10.7
- `api.md` sections 5.3 and 9
- `overview.md` section 7: R36, R39. Section 9: A44
