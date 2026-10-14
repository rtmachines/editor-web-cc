# Design System

Status: Accepted (design stage, 2026-10-14)
Source of requirements: `general-draft.txt` (R10, R36, R39, R40), A44
Related: `overview.md`, `api.md`, `reliability.md`, ADR-0001, ADR-0012, ADR-0013, ADR-0015, ADR-0036

## 1. Scope

This document is the normative reference of the user interface of `recipe-web` (ADR-0036). It defines the tokens, the typefaces, the sizes, the icons, the states, and the components. Every page and every fragment follows it.

The values live in `crates/recipe-web/static/app.css` as CSS custom properties on `:root`. Each table with the header `| Token | Value | Use |` is a token table. Two tests of `recipe-web` keep this document and `app.css` equal:

- The token test compares each row of each token table with the custom property of `app.css` that has the same name.
- The contrast test computes the WCAG contrast ratio of each row of section 4 from the values of `app.css`. A ratio below the minimum of its row fails the test.

The UI meets WCAG 2.2 level AA (A44).

`api.md` section 9 gives the CSP, the scripts, and the HTMX configuration. ADR-0012 gives the rules of the UI text. This document does not repeat them.

## 2. Principles

- Color only for meaning. The canvas is neutral grey, and the chrome has no color. Color signals only a state, an error, a warning, a success, information, a link, or the focus. This is the idea of the high-performance HMI of ISA-101.
- One light theme. The users work in a bright plant hall. The UI has no dark theme.
- Forced colors. In the mode `forced-colors: active`, every border, every focus outline, and every icon stays visible (section 11).
- Reduced motion. Nothing on a page moves (section 11).
- Data in mono. Codes, values, limits, reference numbers, version numbers, unit paths, S7 addresses, logins, and request ids use `--font-mono`.
- No state by color alone. A state shows a color, an icon, and a word together (section 9).

## 3. Color tokens

Each value is a hex color. Only `:root` holds these values. Every other rule names a color through its token, except for the values that section 12 permits.

| Token | Value | Use |
|---|---|---|
| `--color-canvas` | `#E6E9ED` | Page ground |
| `--color-surface` | `#FFFFFF` | Panels, tables, inputs |
| `--color-sunken` | `#F2F4F6` | Table header, row hover, disabled control |
| `--color-text` | `#111418` | Body text |
| `--color-text-muted` | `#4A5361` | Hint, secondary text, table header text |
| `--color-border` | `#6B7482` | Input border, table border, panel border, form border |
| `--color-divider` | `#C5CBD3` | Row divider |
| `--color-action` | `#0B4C86` | Link, primary button |
| `--color-action-hover` | `#083962` | Hover of a link and of a primary button |
| `--color-on-action` | `#FFFFFF` | Text on an action ground and on the top bar |
| `--color-focus` | `#FFD400` | Inner focus ring |
| `--color-focus-ring` | `#111418` | Outer focus ring |
| `--color-topbar` | `#111418` | Top bar ground, toast ground |
| `--color-topbar-hover` | `#2B313A` | Top bar link hover and current page |
| `--color-topbar-edge` | `#8A93A0` | Border of a control in the top bar |
| `--color-draft-fg` | `#6E4500` | Word and icon of a `Draft` badge |
| `--color-draft-bg` | `#FFF0CC` | Ground of a `Draft` badge |
| `--color-draft-edge` | `#A86A00` | Edge of a `Draft` badge |
| `--color-approved-fg` | `#145A2A` | Word and icon of an `Approved` badge |
| `--color-approved-bg` | `#DDF1E2` | Ground of an `Approved` badge |
| `--color-approved-edge` | `#2D7A43` | Edge of an `Approved` badge |
| `--color-retired-fg` | `#3D4552` | Word and icon of a `Retired` badge and of an inactive user badge |
| `--color-retired-bg` | `#E4E7EB` | Ground of a `Retired` badge and of an inactive user badge |
| `--color-retired-edge` | `#6B7482` | Edge of a `Retired` badge and of an inactive user badge |
| `--color-error-fg` | `#8E1B14` | Text and icon of an error message and of a field error |
| `--color-error-bg` | `#FBE4E1` | Ground of an error message |
| `--color-error-edge` | `#B42318` | Edge of an error message, border of an input with an error |
| `--color-warning-fg` | `#6E4500` | Text and icon of a warning message |
| `--color-warning-bg` | `#FFF0CC` | Ground of a warning message |
| `--color-warning-edge` | `#A86A00` | Edge of a warning message |
| `--color-success-fg` | `#145A2A` | Text and icon of a success message and of an active user badge |
| `--color-success-bg` | `#DDF1E2` | Ground of a success message and of an active user badge. Icon of the toast |
| `--color-success-edge` | `#2D7A43` | Edge of a success message and of an active user badge |
| `--color-info-fg` | `#0B4C86` | Text and icon of an information message |
| `--color-info-bg` | `#E0ECF8` | Ground of an information message |
| `--color-info-edge` | `#0B4C86` | Edge of an information message |

## 4. Contrast pairs

Each row names a foreground token, a background token, and the minimum WCAG contrast ratio of the pair. The minimum is 4.5 for text (WCAG 1.4.3). The minimum is 3.0 for a UI element or an icon (WCAG 1.4.11). The contrast test computes each ratio from `app.css` (section 1).

| Foreground | Background | Minimum |
|---|---|---|
| `--color-text` | `--color-surface` | 4.5 |
| `--color-text` | `--color-canvas` | 4.5 |
| `--color-text` | `--color-sunken` | 4.5 |
| `--color-text-muted` | `--color-surface` | 4.5 |
| `--color-text-muted` | `--color-canvas` | 4.5 |
| `--color-text-muted` | `--color-sunken` | 4.5 |
| `--color-action` | `--color-surface` | 4.5 |
| `--color-action` | `--color-canvas` | 4.5 |
| `--color-action` | `--color-sunken` | 4.5 |
| `--color-on-action` | `--color-action` | 4.5 |
| `--color-on-action` | `--color-action-hover` | 4.5 |
| `--color-on-action` | `--color-topbar` | 4.5 |
| `--color-on-action` | `--color-topbar-hover` | 4.5 |
| `--color-draft-fg` | `--color-draft-bg` | 4.5 |
| `--color-draft-fg` | `--color-surface` | 4.5 |
| `--color-approved-fg` | `--color-approved-bg` | 4.5 |
| `--color-approved-fg` | `--color-surface` | 4.5 |
| `--color-retired-fg` | `--color-retired-bg` | 4.5 |
| `--color-retired-fg` | `--color-surface` | 4.5 |
| `--color-error-fg` | `--color-error-bg` | 4.5 |
| `--color-error-fg` | `--color-surface` | 4.5 |
| `--color-warning-fg` | `--color-warning-bg` | 4.5 |
| `--color-success-fg` | `--color-success-bg` | 4.5 |
| `--color-info-fg` | `--color-info-bg` | 4.5 |
| `--color-text` | `--color-draft-bg` | 4.5 |
| `--color-text` | `--color-approved-bg` | 4.5 |
| `--color-text` | `--color-retired-bg` | 4.5 |
| `--color-text` | `--color-error-bg` | 4.5 |
| `--color-text` | `--color-warning-bg` | 4.5 |
| `--color-text` | `--color-success-bg` | 4.5 |
| `--color-text` | `--color-info-bg` | 4.5 |
| `--color-text-muted` | `--color-error-bg` | 4.5 |
| `--color-text-muted` | `--color-info-bg` | 4.5 |
| `--color-text-muted` | `--color-warning-bg` | 4.5 |
| `--color-border` | `--color-surface` | 3.0 |
| `--color-border` | `--color-canvas` | 3.0 |
| `--color-border` | `--color-sunken` | 3.0 |
| `--color-draft-edge` | `--color-surface` | 3.0 |
| `--color-draft-edge` | `--color-canvas` | 3.0 |
| `--color-approved-edge` | `--color-surface` | 3.0 |
| `--color-approved-edge` | `--color-canvas` | 3.0 |
| `--color-retired-edge` | `--color-surface` | 3.0 |
| `--color-retired-edge` | `--color-canvas` | 3.0 |
| `--color-error-edge` | `--color-surface` | 3.0 |
| `--color-error-edge` | `--color-canvas` | 3.0 |
| `--color-warning-edge` | `--color-surface` | 3.0 |
| `--color-warning-edge` | `--color-canvas` | 3.0 |
| `--color-success-edge` | `--color-surface` | 3.0 |
| `--color-success-edge` | `--color-canvas` | 3.0 |
| `--color-info-edge` | `--color-surface` | 3.0 |
| `--color-info-edge` | `--color-canvas` | 3.0 |
| `--color-focus-ring` | `--color-surface` | 3.0 |
| `--color-focus-ring` | `--color-canvas` | 3.0 |
| `--color-focus` | `--color-focus-ring` | 3.0 |
| `--color-focus` | `--color-topbar` | 3.0 |
| `--color-focus` | `--color-topbar-hover` | 3.0 |
| `--color-focus` | `--color-action` | 3.0 |
| `--color-topbar-edge` | `--color-topbar` | 3.0 |
| `--color-success-bg` | `--color-topbar` | 3.0 |

The last row is the success icon of the toast on its dark ground (section 10.7).

For information only: with the values of section 3, every pair passes. The lowest text ratio is 6.38, for `--color-text-muted` on `--color-canvas`. The lowest ratio of a UI element is 3.65, for `--color-draft-edge` on `--color-canvas`.

## 5. Typography

Two typefaces set every text of the UI:

- Atkinson Hyperlegible Next sets the UI text: headings, labels, buttons, messages, and body text.
- Atkinson Hyperlegible Mono sets the data: codes, values, limits, reference numbers, version numbers, unit paths, S7 addresses, logins, and request ids.

Both faces separate `0` and `O`, and `1`, `l`, and `I`. The mono face gives tabular figures.

The files come from the npm tarballs of the packages `@fontsource/atkinson-hyperlegible-next@5.3.0` and `@fontsource/atkinson-hyperlegible-mono@5.3.0`, license OFL-1.1. The repository holds the subsets `latin` and `latin-ext`, the weights 400 and 700, and the style normal. These are 8 WOFF2 files in `crates/recipe-web/static/fonts/`:

| File | Face | Subset | Weight |
|---|---|---|---|
| `atkinson-hyperlegible-next-latin-400-normal.woff2` | Next | `latin` | 400 |
| `atkinson-hyperlegible-next-latin-ext-400-normal.woff2` | Next | `latin-ext` | 400 |
| `atkinson-hyperlegible-next-latin-700-normal.woff2` | Next | `latin` | 700 |
| `atkinson-hyperlegible-next-latin-ext-700-normal.woff2` | Next | `latin-ext` | 700 |
| `atkinson-hyperlegible-mono-latin-400-normal.woff2` | Mono | `latin` | 400 |
| `atkinson-hyperlegible-mono-latin-ext-400-normal.woff2` | Mono | `latin-ext` | 400 |
| `atkinson-hyperlegible-mono-latin-700-normal.woff2` | Mono | `latin` | 700 |
| `atkinson-hyperlegible-mono-latin-ext-700-normal.woff2` | Mono | `latin-ext` | 700 |

- `OFL-atkinson-hyperlegible-next.txt` and `OFL-atkinson-hyperlegible-mono.txt` in the same directory hold the license text of each package. The route does not serve them.
- The SHA-512 of each tarball must equal `dist.integrity` of `registry.npmjs.org`. Then the SHA-256 constants of the files go into the code.
- The binary embeds each file. A test compares the SHA-256 of each file with its constant, as for `htmx.min.js`.
- Each font file is one fixed row of the route table. One table of the 8 files builds these rows. Each row serves its file with the media type `font/woff2` (`api.md` section 2.1).
- An unknown name under `/static/fonts/` reaches the router fallback. Its `404` therefore carries `Cache-Control: no-store`.
- `app.css` has one `@font-face` rule for each file. The rule sets `font-display: swap` and the `unicode-range` of its subset, as the package declares it.

| Token | Value | Use |
|---|---|---|
| `--font-sans` | `"Atkinson Hyperlegible Next", "Segoe UI", "Noto Sans", "DejaVu Sans", sans-serif` | UI text |
| `--font-mono` | `"Atkinson Hyperlegible Mono", "Cascadia Mono", "DejaVu Sans Mono", monospace` | Data |
| `--text-sm` | `0.9375rem` | 15 px: table header, hint, reference |
| `--text-base` | `1.0625rem` | 17 px: body text, controls, data |
| `--text-lg` | `1.375rem` | 22 px: `h2` |
| `--text-xl` | `1.75rem` | 28 px: `h1` |
| `--leading` | `1.5` | Line height |

The px values apply to the default root size of 16 px.

- The UI uses the weights 400 and 700 only.
- No text is smaller than `--text-sm`.
- `--text-sm` stays only for a table header, a hint, a reference, and the normal badge (section 10.9).
- Headings use `--font-sans` at weight 700.
- The `body` rule sets `font-variant-numeric: tabular-nums`.
- Data uses `--font-mono` at `--text-base`. A table cell with a number aligns its content to the right.
- A date and time shows as `2026-10-14 09:12 UTC` in `--font-mono`: the date, the time to the minute, and the word `UTC`.
- In a table cell, data breaks only between words (`overflow-wrap: break-word`). The table then scrolls in its frame, and no login breaks inside the word.
- A data value in `--font-mono` breaks with `overflow-wrap: anywhere`. A flex item that holds text has `min-width: 0`. So a login or a hash of 64 characters fits a width of 320 CSS px.
- The class `.data` marks a data value, also in an input and in a table cell. Its rule sets `--font-mono`, `--text-base`, and `overflow-wrap: anywhere`.
- In a table cell, the rule of the cell sets `overflow-wrap: break-word`. That rule wins over `anywhere`.

## 6. Spacing and layout

The use column gives typical uses. A rule of `app.css` can use a space token in another place.

| Token | Value | Use |
|---|---|---|
| `--space-1` | `4px` | Gap between an icon and its word |
| `--space-2` | `8px` | Padding of a badge, gap between adjacent targets |
| `--space-3` | `12px` | Side padding of a table cell and of an input |
| `--space-4` | `16px` | Padding of a panel, gap between form fields |
| `--space-5` | `24px` | Gap between the sections of a page |
| `--space-6` | `32px` | Padding of `main` |
| `--space-7` | `48px` | Padding of the empty state |
| `--radius` | `4px` | Corner of a control, a panel, and a badge |
| `--edge` | `1px` | Border of a panel and of a table frame, row divider |
| `--edge-control` | `2px` | Border of an input, a select, and a button |
| `--edge-badge` | `1.5px` | Edge of a status badge |
| `--edge-current` | `3px` | Bottom border of the current navigation link and of the current language button |
| `--topbar-height` | `56px` | Minimum height of the top bar |
| `--width-form` | `40rem` | Maximum width of a form |
| `--width-text` | `70ch` | Maximum width of a text paragraph |
| `--width-page` | `90rem` | Maximum width of the page content |

A full page has these regions in this order (section 10.1):

1. The skip link.
2. The top bar.
3. `main` with `id="content"`. It starts with `#error-panel` and the status region `#toast`, then the one `h1`.

- A panel and a form with fields in `main` have the border `--color-border`. `--color-divider` is only for a row divider (section 10.4).
- A form that holds only one action button inside `.actions` has no border of its own.
- A table uses the full width of `main`.
- A form uses at most `--width-form`.
- A text paragraph uses at most `--width-text`.
- The page works from a width of 320 CSS px (WCAG 1.4.10). A table scrolls inside its own frame. The page never scrolls sideways.

## 7. Touch targets and focus

| Token | Value | Use |
|---|---|---|
| `--target` | `44px` | Minimum height and width of a target |
| `--target-gap` | `8px` | Minimum gap between adjacent targets |
| `--focus-width` | `3px` | Width of each focus ring and of the transparent outline |

Target rules:

- These elements have a minimum height and a minimum width of `--target`:
  - every button, input, and select,
  - every navigation item and every tree node,
  - every link styled as a button (`.btn`),
  - every link in a table (`.table-wrap a`),
  - every stand-alone link (`.target-link`).
- Every target has `display: inline-flex` or `display: flex` with `align-items: center`, because `min-height` has no effect on an inline element.
- A stand-alone link does not stand inside a sentence. It has the class `.target-link`.
- Adjacent targets keep a gap of at least `--target-gap`.
- Exception: the links in adjacent rows of a table need no gap. Each of them is already `--target` high, and WCAG 2.5.8 asks for a gap only around a smaller target.
- A container of adjacent targets sets `gap` to `var(--target-gap)`. These containers are `.actions`, `.field-lines`, the navigation list, and the language switch.
- A link inside a sentence of text keeps the text size. WCAG 2.5.8 exempts an inline target.

Focus rules (ADR-0036):

- `:focus-visible` gets the focus indicator:
  - an inner ring of `--focus-width` in `--color-focus`,
  - an outer ring of `--focus-width` in `--color-focus-ring`,
  - one `box-shadow` that draws both rings,
  - an `outline` of `--focus-width` in the color `transparent`.
- In forced-colors mode the browser draws no `box-shadow`. It draws the outline in a system color, so the focus stays visible.
- The yellow and black pair is visible on every ground of the palette (section 4).

## 8. Icons

Every icon tag is fixed and complete. `<name>` is the name of the icon in the icon table below:

```html
<svg class="icon" aria-hidden="true" focusable="false" viewBox="0 0 24 24" data-icon="<name>" fill="none" stroke="currentColor">
```

- Each icon is an inline SVG drawn for the project. It is an askama macro in `crates/recipe-web/templates/components/icons.html`.
- The tag has no other attribute. No icon has a `style` attribute.
- An icon always stands next to a visible word. An icon never carries a meaning alone.
- The XSS test compares each `<svg` opening tag of a page with the fixed tag of its icon. An escaped payload can never produce that text.

| Token | Value | Use |
|---|---|---|
| `--icon-sm` | `18px` | Icon in a badge |
| `--icon-md` | `20px` | Icon next to text, icon in a large badge |
| `--icon-lg` | `24px` | Icon in an alert |

| Icon | Meaning | Shape |
|---|---|---|
| `status-draft` | Version state `Draft` | Pencil |
| `status-approved` | Version state `Approved` | Check mark in a circle |
| `status-retired` | Version state `Retired` | Archive box |
| `user-active` | Active user | Check mark in a circle |
| `user-inactive` | Inactive user | Slashed circle |
| `alert-error` | Error message | Octagon with an exclamation mark |
| `alert-warning` | Warning message | Triangle with an exclamation mark |
| `alert-info` | Information message | Circle with the letter i |
| `alert-success` | Success message | Check mark |
| `tree-collapsed` | Collapsed tree node | Chevron that points right |
| `tree-expanded` | Expanded tree node | Chevron that points down |

## 9. States

The word key is the message id of the word in the `fluent` bundles `pl` and `en`.

| State | Word key | Tokens | Icon |
|---|---|---|---|
| `Draft` | `status-draft` | `--color-draft-fg`, `--color-draft-bg`, `--color-draft-edge` | `status-draft` |
| `Approved` | `status-approved` | `--color-approved-fg`, `--color-approved-bg`, `--color-approved-edge` | `status-approved` |
| `Retired` | `status-retired` | `--color-retired-fg`, `--color-retired-bg`, `--color-retired-edge` | `status-retired` |
| User active | `user-status-active` | `--color-success-fg`, `--color-success-bg`, `--color-success-edge` | `user-active` |
| User inactive | `user-status-inactive` | `--color-retired-fg`, `--color-retired-bg`, `--color-retired-edge` | `user-inactive` |

| Feedback | Tokens | Icon |
|---|---|---|
| Error | `--color-error-fg`, `--color-error-bg`, `--color-error-edge` | `alert-error` |
| Warning | `--color-warning-fg`, `--color-warning-bg`, `--color-warning-edge` | `alert-warning` |
| Information | `--color-info-fg`, `--color-info-bg`, `--color-info-edge` | `alert-info` |
| Success | `--color-success-fg`, `--color-success-bg`, `--color-success-edge` | `alert-success` |

Rule: a state and a feedback message show the color, the icon, and the word together. Color alone never carries a meaning (WCAG 1.4.1).

## 10. Components

### 10.1 Page layout

Every full page uses one layout (ADR-0036). The regions come in this order:

1. A skip link to `#content`. It is the first focusable element of the page. It is visible when it has the focus.
2. The top bar `header`. It holds the application name, the navigation (section 10.2), the language switch (section 10.3), the login of the user, and the logout button.
3. `main` with `id="content"`. It starts with `#error-panel` and `#toast`, then holds exactly one `h1`.

- `#error-panel` is the target of `HX-Retarget` for an error (`api.md` section 5.3). It has no role and no `aria-live`. The fragment inside it has `role="alert"` (section 10.6).
- `#toast` is the status region (section 10.7).
- Both regions stand inside the landmark `main`. The skip link therefore reaches an error on the login page.
- The top bar has the ground `--color-topbar`, the text `--color-on-action`, and a minimum height of `--topbar-height`. The login of the user uses `--font-mono`.
- A control in the top bar has the border `--color-topbar-edge`. Its hover ground is `--color-topbar-hover`.
- A session with `must_change_password` gets the layout with no navigation link.
- The login page has no session. On `GET /login`, its top bar holds the application name and the language switch with `csrf_pre`.
- The full error page shows the top bar with the application name only, because the error layer runs before the session layer. It has no navigation and no language switch.
- The `main` of the full error page holds the error in `#error-panel` and the `h1` of section 10.6. Then it shows one link to `/recipes` with a text from the bundles.

### 10.2 Navigation

- The navigation is a `nav` element. It holds a list of links.
- The accessible name of the `nav` is the word "Main" from the bundles. A screen reader therefore does not say "navigation" twice.
- The navigation shows a link only when the catalog row of the query of the target page allows the role. `Operation::requirement().allows(role)` of `recipe-app` gives the answer.
- The queries are `ListRecipes` for `/recipes`, `ListUsers` for `/users`, and `GetOwnProfile` for `/me`. `authorize` uses the same row, so the navigation and the permission check cannot disagree.
- The use case of the target page still checks the permission (ADR-0013).
- The link of the current page has `aria-current="page"`. It shows the ground `--color-topbar-hover`, a bottom border of `--edge-current` in `--color-on-action`, and the weight 700.
- Hover: the ground `--color-topbar-hover`. Focus: section 7.

### 10.3 Language switch

- Every full page that answers a `GET` request has the switch, except the full error page (ADR-0036).
- A full page that answers another method has no switch. A user therefore cannot leave the page with a one-time password by accident.
- The switch is a form with `method="post"` and `action="/lang"` (`api.md` section 8).
- The hidden field `next` holds the path and the query of the request, with no length limit. Askama escapes the value.
- With a session, the form sends the hidden field `csrf_token`. On the login page, it sends the hidden field `csrf_pre`.
- The form has two submit buttons with `name="lang"` and the values `pl` and `en`. Their labels come from the bundles.
- The labels "Polski" and "English" carry `lang="pl"` and `lang="en"` (WCAG 3.1.2).
- The button of the current language has `aria-current="true"`. It has the look of the current navigation link (section 10.2).
- The full error page has no switch, because it has no token.

### 10.4 Table

- A frame with the class `table-wrap` holds the `table`. The frame scrolls sideways when the table is wider than `main`.
- The frame has `tabindex="0"` and `role="region"`, so a keyboard user can scroll it.
- The table has a `caption`. The frame takes its accessible name from the `caption` with `aria-labelledby`.
- Each column header is a `th` with `scope="col"`.
- Each row has a minimum height of `--target`. A cell has a side padding of `--space-3` and no padding above or below, so a row with a link stays at `--target`.
- The header row has the ground `--color-sunken` and the text `--color-text-muted` at `--text-sm`. It is not sticky, because a sticky row inside a frame that scrolls sideways sticks only to that frame.
- The table border is `--color-border`. A row divider is `--color-divider`. The hover ground of a row is `--color-sunken`.
- A cell with data uses `--font-mono`. A cell with a number aligns its content to the right.
- A table with no rows shows the empty state (section 10.10) in place of the table body.

### 10.5 Form field and buttons

Form field:

- The `label` stands above the input. Its `for` names the `id` of the input. The `id` of an input is the name of its field, for example `current_password`.
- A hint stands under the label at `--text-sm` in `--color-text-muted`. `aria-describedby` of the input names the hint.
- The label of a required field holds the word for "required" from the bundles. The input has `aria-required="true"`.
- An input has no attribute `placeholder`. A placeholder disappears when the user types, and its text has low contrast. The label and the hint carry the text.
- An input has no attribute `required`, `pattern`, or `minlength`. The browser would show its own message in the language of the operating system. The server checks each field and answers with `field-errors.html` (`api.md` section 5.1).
- An input keeps the attribute `maxlength`.
- An input has the ground `--color-surface`, the border `--color-border`, and the text size `--text-base`. An input for data uses `--font-mono`.
- An input for data also has `autocapitalize="none"` and `spellcheck="false"`. A touch keyboard then does not change the first letter of a login or a code.
- A disabled input has the ground `--color-sunken` and the text `--color-text-muted`.

Field errors (ADR-0036):

- An HTMX answer with the status `422` goes into `#error-panel` (`api.md` section 5.3).
- Each line of `field-errors.html` shows the label of the field from the bundles. The key of the label is `field-` and the field name with each `_` replaced by `-`, for example `field-current-password`.
- In an HTMX fragment and on the login page with its form, the line links to `#<field>`, the `id` of the input.
- The full error page shows the label without a link.
- Step P3.3 adds an error line under the input. A form that renders with its own errors shows this line.
- The error line shows the `alert-error` icon, the code in `--font-mono`, and one sentence in `--color-error-fg`.
- An input with an error line has `aria-invalid="true"` and the border `--color-error-edge`. `aria-describedby` also names the error line.

Buttons:

- A primary button has the ground `--color-action` and the text `--color-on-action`. Its hover ground is `--color-action-hover`.
- A secondary button has the ground `--color-surface`, the text `--color-action`, and the border `--color-border`.
- A danger button has the ground `--color-surface`, the text `--color-error-fg`, and the border `--color-error-edge`.
- A form has at most one primary button.
- A disabled button has the attribute `disabled`, the ground `--color-sunken`, the text `--color-text-muted`, and the border `--color-border`.
- Every button shows a word. An icon in a button stands next to the word.

### 10.6 Error fragment

`api.md` section 5.2 gives the content of an error fragment: the code, one sentence, and the `request_id`. This section gives the structure and the look.

- Each fragment `error.html` and `field-errors.html` is one `div` with `role="alert"`. The `div` holds the icon, the code, the sentence, the field lines, and the reference. Only `field-errors.html` has field lines.
- `#error-panel` itself has no role and no `aria-live` (section 10.1).
- An HTMX error answer sets `HX-Reswap: innerHTML show:#error-panel:top` (`api.md` section 5.3). The error therefore scrolls into view.
- The focus does not move. A screen reader therefore reads the alert once.

The look of an error fragment:

- the icon of the severity (section 9) at `--icon-lg`,
- the code in `--font-mono` at weight 700,
- one sentence in `--font-sans`,
- the reference with the `request_id` in `--font-mono` at `--text-sm`.

An error message has the ground `--color-error-bg`, the edge `--color-error-edge`, and the icon in `--color-error-fg`.

The notice:

- `alert::notice(severity, message)` has the look of the alert and no role.
- It shows a static message, for example the notice of a one-time password or of a password change.
- A screen reader therefore does not announce it when the page loads.

The full error page:

- It shows the same `div` in `#error-panel`.
- It has one `h1` with the text of the key `error-page-title`.
- Its page title has the form `<code> · <error-page-title> · <app-name>`.

### 10.7 Status region and toast

- `#toast` has `role="status"` and `aria-live="polite"`. It is on every full page, also when it is empty, so a screen reader announces a new message.
- No element inside `#toast` has its own `role="status"`.
- A success message goes into `#toast` with the `alert-success` icon (ADR-0036).
- The same answer clears `#error-panel` with an out-of-band swap (`hx-swap-oob`). The value of that attribute is a literal, never a variable.
- The toast has the ground `--color-topbar` and the text `--color-on-action`. The `alert-success` icon has the color `--color-success-bg`.
- The message stays until the next swap, because no script may remove it.
- A one-time password stays in the page content. Its container has no live role.

### 10.8 Tree

The unit tree and the type tree use the disclosure pattern of WAI-ARIA, not the treeview pattern (ADR-0036).

- The tree is a set of nested lists. No element has `role="tree"` or `role="treeitem"`.
- The root list of a tree has the class `tree`, because `tree.js` reacts only inside `.tree`.
- Each node with children is a `button` with `aria-expanded` and `aria-controls`. `aria-controls` names the `id` of the group, the list of its children.
- The `id` of a group is a fixed prefix and the UUID of the unit or the type.
- A collapsed group has the attribute `hidden`.
- The button shows the icon `tree-collapsed` or `tree-expanded` next to the words of the node.
- A node without children has no `aria-expanded`.
- The unit path of a node uses `--font-mono`.
- Click, Enter, and Space work through the native button. Tab moves the focus between the nodes.
- `tree.js` toggles `aria-expanded` of the button and the attribute `hidden` of the group. It finds the group with `document.getElementById`. The focus stays on the button.
- In step P3.2, HTMX loads the children of a node.

### 10.9 Status badge

- A badge shows the icon and the word of a state (section 9).
- A badge has the ground of its state. The word and the icon use the `-fg` token. The edge has the width `--edge-badge` and the color of the `-edge` token.
- The normal badge uses `--text-sm` and an icon of `--icon-sm`.
- The large badge stands in the page header next to the `h1`. It uses `--text-base` and an icon of `--icon-md`.
- A badge is not a target.

### 10.10 Empty state

- The empty state is one element `.empty` with one sentence from the bundles. It has no picture and no other panel around it.
- `.empty` has the ground `--color-surface`, the border `--color-border`, and the full width of `main`.
- In a table, the empty state stands in place of the table body (section 10.4).

### 10.11 Description list

- A `dl` shows the fields of one entity, for example a user.
- A `dt` uses the weight 700 and `--color-text-muted` at `--text-sm`. A `dd` has no indent and follows its `dt`.
- A `dd` with data uses the class `.data`. A date uses the format of section 5.

## 11. Motion and user preferences

- No rule of `app.css` sets an animation or a transition.
- The rules for `.htmx-indicator` live in `app.css` (`api.md` section 9). They change only `opacity`.
- A rule for `forced-colors: active` keeps every border and the focus outline. An element with a colored ground also has a border, so its shape stays visible. Where the border must not show in normal mode, its color is `transparent`.
- In forced-colors mode, the bottom border of the current navigation link and of the current language button stays visible (sections 10.2 and 10.3).
- `prefers-reduced-motion: reduce` needs no rule, because nothing on a page moves.

## 12. Rules for templates

`api.md` section 9 gives the rules for scripts, `hx-*` attributes, and the filter `|safe`. ADR-0012 gives the rules for UI text. This section adds these rules only:

- A template gets its look only from the classes of `app.css`. A rule of `app.css` uses a font family, a text size, or a space only through its token.
- A rule outside `:root` names a color only through its token. The only other color values are `transparent`, `currentColor`, and the system colors of forced-colors mode.
- These system colors are `Canvas`, `CanvasText`, `LinkText`, `ButtonText`, `ButtonBorder`, `Highlight`, `HighlightText`, and `GrayText`.
- No template holds a literal word (ADR-0036). A test removes the askama tags and the markup of each template and finds no letter.
- Every icon uses the fixed tag of section 8.
- Every full page uses the layout of section 10.1. A fragment does not repeat the layout.
- A link outside `nav` and outside `.btn` keeps its underline, because `--color-action` on `--color-text` has only 2.10:1 (WCAG 1.4.1).
- A data value in `--font-mono` breaks with `overflow-wrap: anywhere`. A flex item that holds text has `min-width: 0` (section 5).
- Besides `|safe` (`api.md` section 9), no template uses `{% filter safe %}`. No template uses an `escape` or `e` filter with an argument.
- No Rust source of `recipe-web` uses `escape =`, `ext =`, `Safe(`, `HtmlSafe`, or `MaybeSafe` on a template.
- A test reads every template and every Rust source of `recipe-web` and checks the last three rules.
