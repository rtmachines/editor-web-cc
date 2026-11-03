# Proposal

## Why

A UI review of 2026-11-02 found pages that show a stale state, hide the way back, or invite a wrong reading of data (R39, A44).
This change fixes the findings that need no new ADR (R10, R13, R16, R24, R28, A17, A22).
It also ships ADR-0110, which permits several danger actions on one page (finding A1, F02).

## What Changes

- User page: Reset password in a danger disclosure (A1, F02), fresh state after a command, only applicable actions.
- Danger notices: each danger disclosure gets a notice id that is unique in the page (ADR-0110).
- Parameter table: the column "Unit" becomes "Machine unit".
- Parent link in the page header, after the `h1` and the state badge. The parameter heading names the recipe and the version.
- Type usage rows link to their pages.
- `lang` on translated text and inputs.
- Link after the leaf grid to the save button.
- A setting of the dev server for the e2e suite: the list of translation languages holds `de`. The production default stays.
- Clearer empty states.
- Machine page links to its recipes.
- Polish terms: "Wersja robocza" for the state Draft and "Tag" for a parameter tag.
- Small fixes: user pager, role selects, login focus, revision words, unit empty state, delete notices, colons, unused CSS.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: new requirements and three modified requirements. Two of them change only the Polish word for the state Draft. The requirement "Notice of a danger action" follows ADR-0110.
- `plant-model`: one modified requirement, "Type usage". A usage row also names the key of the parameter, because the page links to the parameter.

## Impact

`recipe-web` (templates, the bundles `pl` and `en` with their approval files, the example `dev_server`), the type usage row in `recipe-app` and `recipe-db`, `docs/user/reference-screens.md`, and `e2e/`. No route or migration changes. The change adds ADR-0110, which amends ADR-0049 (f), and one rule of `design-system.md` §10.5.

## Out of scope

Deferred (needs ADR), input for `web-ui-data-safety`:

- A3 (UI-02): ADR-0043 (f) swaps every translation row.
- B4 (UI-03): refresh of the English source in the translation section. ADR-0043 (f) puts the English base text in each translation row, but the template shows one shared block. A3 and B4 go to `web-ui-data-safety` with one ADR on the translation section.
- A4 (UI-01): a textarea is a new component (`design-system.md` §10.5).
- A6 (F06, UI-04): changes A24 and `design-system.md` §10.1.
- B7 (F13, UI-12): `api.md` §4 and ADR-0008 fix the conflict fragment.
- B8 (F14): ADR-0049 (i) fixes the link row.
- B9 (UI-10, UI-11): ADR-0043 (l) fixes the rebind form.
- B12 (F09): ADR-0043 (j), (k) fix the parameter page.
- F08 indicator visibility: `design-system.md` line 620 permits only `opacity`.
- B1 (busy indicators on more forms): the indicators come together with the ADR that changes `design-system.md` line 620 from `opacity` to `visibility`.
- F19, UI-17 h3 style: the type scale of `design-system.md` §5 has no `h3`.
- F18 boosted user form: `design-system.md` §10.3 keeps the language switch off a one-time password.
- F10, F16, F22, F25, F27: each changes `design-system.md` §10.
- Section D ideas: breadcrumb, read-only pages, value editing, conflict recovery, unit of measure, search, rebind preview.

Not done: UI-18. `design-system.md` §10.11 requires the `.hash` rule.

## Decisions of the user

The user decided these points on 2026-11-02.

1. The Polish term for the state Draft is "Wersja robocza" everywhere. The badge "Szkic" changes to "Wersja robocza".
2. The Polish term for a parameter tag is "Tag" everywhere. The keys that say "Znacznik" change to "Tag".
3. The busy indicators (B1) leave this change. They wait for the ADR that fixes F08.
4. A page can hold more than one danger action. Each danger notice gets an id that is unique in the page (ADR-0110).
