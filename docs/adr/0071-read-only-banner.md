# ADR-0071: Read-only banner

Status: Accepted. Amended by ADR-0073 (text of the read-only banner)
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 7.8 says: "The UI shows a banner" in read-only mode (A24). `design-system.md` section 10.1 gives the regions of the page layout and has no banner. Step P4.1 adds the job `disk_check`, which sets the read-only mode. The plan needs a place, a look, and a text for the banner.

- `alert::notice(severity, message)` of `design-system.md` section 10.6 shows a static message with the look of an alert and no role.
- An HTMX request with `HX-Target: content` gets the content of `main` (ADR-0037). An answer that only replaces `main` must therefore hold the banner too.
- The login page and the full error page use their own layouts (`design-system.md` section 10.1). In read-only mode, a user can still log in.
- No page shows a state by color alone (`design-system.md`, `.claude/rules/web.md`).

## Decision drivers

- Each user sees that the system blocks changes, before the user sends a command (A24).
- The banner tells what still works: view, export, login, and logout.
- A screen reader does not announce the banner again at each page load.
- The banner adds no script, no inline style, and no user data (`api.md` section 9).

## Considered options

### Option A: A notice at the start of `main`

- Pros: the component exists. It stands in the full page and in the content answer of HTMX. It has no role, so a screen reader reads it with the page and does not announce it again.
- Cons: the banner moves the `h1` down by one line.

### Option B: A banner in the top bar `header`

- Pros: the banner stays out of the content.
- Cons: the content answer of HTMX does not replace the top bar, so the banner does not change after a swap. At 320 CSS px, the top bar may have no room for a sentence.

### Option C: A toast in `#toast`

- Pros: no new region.
- Cons: `#toast` has `aria-live`, so a screen reader announces the text after each swap. A toast is a short message, not a lasting state.

### Option D: An element with `role="alert"`

- Pros: a screen reader announces the state at once.
- Cons: it announces the state again at each page load. `role="alert"` belongs to the error fragment (`design-system.md` section 10.6).

## Decision

The project uses **Option A**.

- (a) The banner is `alert::notice("warning", text)` with the icon `alert-warning`. It stands directly after `<main id="content">` and before `#error-panel`.
- (b) The banner stands in the full page and in the content answer of `HX-Target: content`. The login page and the full error page show it too.
- (c) The banner has no role, no button, and no link. It shows the icon and the text, so the state does not depend on color alone.
- (d) The text comes from the bundle key `readonly-banner`:

| Key | en | pl |
|---|---|---|
| `readonly-banner` | Read-only mode: the disk is almost full. Changes are blocked. You can view, export, log in, and log out. | Tryb tylko do odczytu: dysk jest prawie pełny. Zmiany są zablokowane. Możesz przeglądać, eksportować, logować się i wylogować. |

- (e) The gate puts the flag into a request extension, and the page handlers read it there. `render_errors` reads the flag from the web state, because it runs outside the gate.
- (f) The page shows no banner when the mode is off.
- (g) The browser checks of the banner, for example the focus and the width of 320 CSS px, go to step P4.2 (ADR-0048 (f)).

## Consequences

### Positive

- Each full page and each content answer shows the mode in the UI language.
- The notice reuses the component of `design-system.md` section 10.6. The decision adds no token and no style rule.
- The banner text is a literal of the bundles, so it holds no user data.

### Negative

- An HTMX answer that does not replace `main` keeps the old state of the banner until the next content answer.
- The banner takes one line of the content area on every page in read-only mode.

### Risks and mitigations

- Risk: a later layout misses the flag. Mitigation: the HTTP tests check the banner on and off, in both languages, on the login page and on the error page. These are the HTTP tests of `recipe-web`.

## Related

- Amends ADR-0036 (i) (the banner comes before `#error-panel`).
- ADR-0036 (design system), ADR-0037 (content answer of HTMX), ADR-0048 (browser regression suite).
- `design-system.md` section 10.1 (the banner at the start of `main`), 10.6 (the notice).
- `reliability.md` section 7.8 (the banner of read-only mode).
- `api.md` section 9.
- The delta adds `web-ui` "Read-only banner".
- `overview.md` section 7: R39. Section 9: A24.
