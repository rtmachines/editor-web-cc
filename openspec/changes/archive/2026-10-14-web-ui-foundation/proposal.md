# Proposal

## Why

The pages of step P2.5 hold plain HTML with no style sheet, so R39 is still partial. The login redirects to `/recipes`, but no route serves it. Operators read the screens at a machine with gloves and glare, so states, numbers, and targets must be clear at arm's length. This change satisfies R39, R10, R36, R38, and R40. It adds A44 (WCAG 2.2 AA) and uses A17.

## What Changes

- ADR-0036 and the normative reference `docs/architecture/design-system.md`. `overview.md` gets A44 and a complete R39. `api.md` section 2.1 lists the new static files.
- `static/app.css` holds every token. Tests compare the tokens with the document and compute the WCAG ratio of each contrast pair.
- The typefaces Atkinson Hyperlegible Next and Mono (OFL-1.1, 8 WOFF2 files) under `/static/fonts/`, each with a SHA-256 test.
- `static/row-version.js` and `static/tree.js` in the line limits of `api.md` section 9.
- One layout for every full page: skip link, top bar, and `main` with `#error-panel` and the status region `#toast` first. The top bar holds the navigation, language switch, and logout.
- Askama components: table, form field, tree, status badge, error fragment, toast, and empty state. Inline SVG icons stand next to a word. No state shows by color alone.
- The existing pages use the new layout and components.
- `GET /recipes` as a skeleton. The query `ListRecipes` of `recipe-app` checks `RecipeView` first and returns no rows.
- New texts in `pl.ftl` and `en.ftl`. A test finds no literal word in a template.
- The XSS corpus also goes through the field `next` of the language switch.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: layout, navigation, language switch, state signals, contrast, targets, and texts. Also the recipe list page, the status region, static files, and scripts.

## Out of scope

- The rows, filters, and paging of `/recipes` (P3.3).
- The pages of machines, units, and UDTs (P3.2), of versions and parameters (P3.3), of approval and diff (P3.4), and of export (P3.5).
- The review items R2-3, R2-5, and C6 (P4.1). The list D9, M7, and M9 (P4.2).
- A dark theme, a JavaScript test runner, and CI browser tests.

## Impact

- `crates/recipe-web`: templates, `static/`, `i18n/`, routes, and tests.
- `crates/recipe-app`: the query `ListRecipes`.
- No new crate, no migration, and no change of the CSP or of a setting.
