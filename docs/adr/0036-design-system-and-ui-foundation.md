# ADR-0036: Design system and UI foundation

Status: Accepted. Amended by ADR-0037 (style sheets in (h), navigation link in (i), whole unit tree in (n)) and ADR-0039 (inputs and groups in the leaf grid). Amended by ADR-0041 (the alert of `field-errors.html` holds only the code and the count). Amended by ADR-0047 (page of a refused form in (i) and (o), empty toast after an error in (m)). Amended by ADR-0051 (query list of the navigation in (i)). Amended by ADR-0057 (no `HX-Reswap` on the answer of the backstop panic layer in (p)) and ADR-0071 (the read-only banner before `#error-panel` in (i))
Date: 2026-10-14
Deciders: project owner

## Context

`overview.md` section 7 marks R39 as partial, because step P3.1 produces the visual design. The pages of step P2.5 hold plain HTML with no style sheet.

The CSP of `api.md` section 9 forbids inline styles and foreign origins. The fonts and the icons must therefore come from the application itself.

The users work at a machine with gloves, glare, and noise. An Operator reads a touch panel at a distance of 60 to 80 cm (`docs/plan/ux-brief.md`).

`general-draft.txt` does not ask for WCAG conformance. The conformance target is therefore a new Assumption, A44.

The pages of steps P3.2 to P3.5 need one set of rules:

- the tokens of color, type, space, and size,
- the typefaces and the icons,
- the signals of a version state and of a user state,
- the page layout, the navigation, and the language switch,
- the components that later pages use again.

## Decision drivers

- A user never reads a `Draft` as an `Approved` version (R17, R39).
- A gloved finger can hit each target. A user at 80 cm can read each code and each number (R39, A44).
- Every file comes from the origin of the application. The CSP does not change (`api.md` section 9).
- One normative document binds every reviewer. Tests keep the document and the style sheet equal (CLAUDE.md, source of truth).
- No template holds a literal word (R10, ADR-0012).

## Considered options

### Option A: One ADR and a normative reference in `docs/architecture/`

This ADR records the decisions of step P3.1. `docs/architecture/design-system.md` holds the tokens, the typefaces, the sizes, the icons, the states, and the components.

- Pros: the rules are in the normative set and bind every reviewer. `api.md` and `overview.md` change in the same change as the ADR.
- Cons: one more architecture document must stay equal to `app.css`.

### Option B: A root `DESIGN.md`, or the decisions only in the design of the change

- Pros: no new normative document.
- Cons: `api.md` and `overview.md` change anyway, and the change rule needs an ADR for that. A rule outside `docs/architecture/` binds no reviewer.

## Decision

The project uses **Option A**. The letters match the items of the design of the change `web-ui-foundation`.

Reference and look:

- (a) `docs/architecture/design-system.md` is the normative reference of the tokens, typefaces, sizes, icons, states, and components. The values live in `crates/recipe-web/static/app.css` as CSS custom properties on `:root`. A test of `recipe-web` compares each token table of the document with `app.css`. A second test computes the WCAG contrast ratio of each row of the contrast table from the values of `app.css`.
- (b) The UI has one light theme. The canvas is neutral grey, and the chrome has no color. Color signals only a state, an error, a warning, a success, information, a link, or the focus. This is the idea of the high-performance HMI of ISA-101. The style sheet supports `forced-colors: active` and `prefers-reduced-motion: reduce`. Rejected alternative: a dark theme, because the users work in a bright plant hall.
- (c) The accessibility target is WCAG 2.2 level AA (A44).
  - Every button, input, select, navigation item, and tree node is at least 44 by 44 CSS px.
  - Every link styled as a button (`.btn`) and every link in a table (`.table-wrap a`) has the same minimum size.
  - Adjacent targets have a gap of at least 8 px.
  - A link inside a sentence of text keeps the text size, because WCAG 2.5.8 exempts an inline target.
  - Rejected alternative: 24 px, the AA minimum. It is too small for a gloved finger.
  - Rejected alternative: 48 px. Each table row is then 4 px higher, so fewer parameters fit on one screen. 44 px already fits a gloved finger.
  - Rejected alternative: 44 px, and 56 px for a coarse pointer. The project must then test two layouts.
- (d) The focus indicator is an inner ring of 3 px in `--color-focus` and an outer ring of 3 px in `--color-focus-ring`. `box-shadow` draws both rings. The element also has a transparent outline of 3 px, so forced-colors mode shows a system outline. The yellow and black pair is visible on every ground of the palette.
- (e) Atkinson Hyperlegible Next sets the UI text. Atkinson Hyperlegible Mono sets the data. Data means codes, values, limits, reference numbers, version numbers, unit paths, S7 addresses, logins, and request ids.
  - Both faces separate `0` and `O`, and `1`, `l`, and `I`. The mono face gives tabular figures.
  - The files come from the npm packages `@fontsource/atkinson-hyperlegible-next@5.3.0` and `@fontsource/atkinson-hyperlegible-mono@5.3.0`, license OFL-1.1.
  - The repository holds the subsets `latin` and `latin-ext`, the weights 400 and 700, and the style normal. These are 8 WOFF2 files.
  - The license texts of the two packages differ in their copyright lines. `crates/recipe-web/static/fonts/` therefore holds `OFL-atkinson-hyperlegible-next.txt` and `OFL-atkinson-hyperlegible-mono.txt`.
  - The binary embeds each file. A test compares the SHA-256 of each file with a constant, as for HTMX.
  - `default-src 'self'` of the CSP already covers fonts, so the CSP does not change.
  - Data uses `--text-base`, and a panel or a form has the border `--color-border`, because an Operator reads the screen at 80 cm through glare.
  - The operator guide of step P4.3 gives the browser zoom of 150 % for a touch panel at a machine.
  - Rejected alternative: the system font stack. Segoe UI draws `I` and `l` the same. A panel with Linux renders other faces, so one page looks different on two workstations.
  - Rejected alternative: only the sans face in the repository, with a system monospace face. The shapes of the data then depend on the workstation.
- (f) Icons are inline SVG drawn for the project. They are askama macros in `crates/recipe-web/templates/components/icons.html`.
  - Every icon tag is fixed and complete: `<svg class="icon" aria-hidden="true" focusable="false" viewBox="0 0 24 24" data-icon="<name>" fill="none" stroke="currentColor">`. So no icon has a `style` attribute.
  - The tokens `--icon-sm`, `--icon-md`, and `--icon-lg` set the size of an icon (`design-system.md` section 8).
  - An icon always stands next to a visible word.
  - The XSS test compares each `<svg` opening tag of a page with the fixed tag of its icon. An escaped payload can never produce that text.
  - Rejected alternative: a CSS mask with a `data:` URI. The icon disappears in forced-colors mode.
  - Rejected alternative: an SVG sprite file. It needs one more route, and the page still holds `<svg`.
- (g) A version state shows a color, an icon, and a word. `Draft` is amber with a pencil. `Approved` is green with a check mark in a circle. `Retired` is grey with an archive box. A user state uses the same badge. An active user is green with a check mark in a circle. An inactive user is grey with a slashed circle. The words come from the `fluent` bundles.

Pages and routes:

- (h) `GET /static/*` serves `htmx.min.js`, `app.css`, `row-version.js`, `tree.js`, and the 8 font files under `/static/fonts/`.
  - Each file is one fixed row of the route table with no session. One table of the 8 font files builds the 8 font rows.
  - An unknown name reaches the router fallback, so its `404` carries `Cache-Control: no-store`.
  - The binary embeds each file at build time. The response sets `Cache-Control: public, max-age` with the value of `static.cache_max_age`.
  - The media types are `text/css; charset=utf-8`, `application/javascript`, and `font/woff2`.
  - The font files come from the npm tarballs of the two packages of (e). The SHA-512 of each tarball must equal `dist.integrity` of `registry.npmjs.org`. Then the SHA-256 constants of the files go into the code.
- (i) Every full page has one layout. The regions come in this order: a skip link, a top bar, and `main` with `id="content"`. `main` starts with `#error-panel` and the status region `#toast`, then the one `h1` (p). The top bar holds the application name, the navigation, the language switch, the login of the user, and the logout button.
  - The navigation shows a link only when the catalog row of the query of the target page allows the role. `Operation::requirement().allows(role)` of `recipe-app` gives the answer.
  - The queries are `ListRecipes` for `/recipes`, `ListUsers` for `/users`, and `GetOwnProfile` for `/me`. `authorize` uses the same row, so the navigation and the permission check cannot disagree.
  - The use case of the target still checks the permission (ADR-0013).
  - A session with `must_change_password` shows no navigation link.
  - The current navigation link and the current language button have the ground `--color-topbar-hover` and the weight 700. They also show a bottom border of 3 px in `--color-on-action`. In forced-colors mode the border stays visible.
  - The full error page shows the top bar with the application name only, because the error layer runs before the session layer. Under the error, the page shows one link to `/recipes` with a text from the bundles.
- (j) Every full page that answers a `GET` request has a language switch, except the full error page. The switch is a form `POST /lang` with the hidden field `next`. `next` holds the path and the query of the request with no length limit. The form sends `csrf_token` with a session and `csrf_pre` on the login page.
  - A full page that answers another method has no switch. So a user cannot leave the page with a one-time password by accident.
  - The full error page has no switch, because it has no token.
  - The labels "Polski" and "English" carry `lang="pl"` and `lang="en"` (WCAG 3.1.2).
  - The button of the current language has `aria-current="true"` and the look of the current navigation link (i).
- (k) No template holds a literal word. A test removes the askama tags and the markup of each template and finds no letter. ADR-0012 names this test as the mitigation of a risk.
- (l) `GET /recipes` is a skeleton in step P3.1. The query `ListRecipes` of `recipe-app` checks `RecipeView` first. It returns a list with no rows, because no port reads recipes yet. The page shows the heading and the empty state. Step P3.3 adds the port, the filters, the paging, and the rows.
- (m) A success message goes into the status region `#toast` with the `alert-success` icon. `#toast` has `role="status"`. No element inside `#toast` has its own `role="status"`.
  - The same answer clears `#error-panel` with an out-of-band swap (`hx-swap-oob`). The value of that attribute is a literal, never a variable.
  - The message stays until the next swap, because no script may remove it.
  - A one-time password stays in the page content. Its container has no live role.

Components and feedback:

- (n) The unit tree and the type tree use the disclosure pattern of WAI-ARIA, not the treeview pattern.
  - The tree is a set of nested lists. No element has `role="tree"` or `role="treeitem"`.
  - Each node with children is a `button` with `aria-expanded` and `aria-controls`.
  - Click, Enter, and Space work through the native button. Tab moves the focus between the nodes.
  - `tree.js` toggles `aria-expanded` of the button and the attribute `hidden` of the group. The focus stays on the button.
  - `tree.js` finds the group with `document.getElementById`. Each group `id` is a fixed prefix and the UUID of the unit or the type.
  - Rejected alternative: the treeview pattern of the WAI-ARIA Authoring Practices, with a roving `tabindex` and the arrow keys. It is hard to keep in 50 lines when HTMX loads the children. A touch panel gains nothing from it.
- (o) `api.md` section 5.3 stays. An HTMX answer with the status `422` goes into `#error-panel`.
  - Each line of `field-errors.html` shows the label of the field from the bundles. The key is `field-` and the field name with each `_` replaced by `-`, for example `field-current-password`.
  - In an HTMX fragment and on the login page with its form, the line links to `#<field>`, the `id` of the input.
  - The full error page shows the label without a link.
  - Step P3.3 adds an error line under the input with `aria-invalid="true"`. A form that renders with its own errors shows this line.
- (p) `#error-panel` and `#toast` stand inside `main`, so both regions are inside a landmark. The skip link therefore reaches an error of the login page.
  - `#error-panel` has no role and no `aria-live`.
  - Each fragment `error.html` and `field-errors.html` is one `div` with `role="alert"`. The `div` holds the icon, the code, the sentence, the field lines, and the reference.
  - An HTMX error answer sets `HX-Reswap: innerHTML show:#error-panel:top` (`api.md` sections 4 and 5.3). The error then scrolls into view.
  - The focus does not move, so a screen reader reads the alert once.
  - The full error page has one `h1` with the text of the key `error-page-title`. Its page title has the form `<code> · <error-page-title> · <app-name>`.

## Consequences

### Positive

- R39 is complete.
- One reference binds every page of steps P3.2 to P3.5.
- Tests keep the document and the style sheet equal.

### Negative

- The fonts add about 80 KB to the binary.
- The project must update two font families.
- A static CSS test reads the document.

### Risks and mitigations

- Risk: a new color value lowers a contrast ratio below its minimum. Mitigation: the contrast test computes every pair from `app.css`.
- Risk: a new page does not use the layout. Mitigation: the page tests of each route check the layout.
- Risk: the skeleton page of `/recipes` goes into a release. Mitigation: step P3.3 replaces it before a release.

## Related

- `docs/architecture/design-system.md`
- `overview.md` sections 7 and 9
- `api.md` sections 2.1, 4, 5.3, and 9
- ADR-0001 (frontend), ADR-0012 (UI text), ADR-0013 (permissions), ADR-0015 (`row-version.js`), ADR-0035 (`POST /lang`, pre-session token)
- WAI-ARIA Authoring Practices, disclosure pattern and treeview pattern
- `docs/plan/ux-brief.md` (non-normative input of step P3.1)
