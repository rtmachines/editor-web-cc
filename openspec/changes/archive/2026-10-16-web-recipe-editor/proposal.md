# Proposal

## Why

Users cannot edit a recipe yet. Step P3.3 adds the pages of recipes, versions, and parameters. A process engineer can then create a recipe and a draft. The engineer edits the definition, the value, the limits, and the translations of each parameter. The change satisfies R02, R08, R09, R10, R13, R14, R16, R17, R19, R20, R23, R24, R25, R28, R29, R30, R31, R36, R38, R39, and R40.

The change uses A7, A11, A14, A17, A19, A24, A26, A29, A31, A36, A40, A43, and A44.

## What Changes

- Read ports with in-memory and `sqlx` adapters, and the twelve recipe queries of `api.md` sections 2.4 to 2.6. Each query checks its permission first. Draft queries read one snapshot (ADR-0042).
- `recipe-domain` builds a value from the texts of its leaves and writes these texts for a page (ADR-0039). The value check and the limit check return every fault of one parameter (ADR-0041).
- Pages in `pl` and `en`: the recipe list, the recipe form and page, the version form and page, and the parameter page. The parameter page has the leaf grid, the translations, and the removal. The draft page has the rebind (ADR-0043).
- **BREAKING** for `api.md`: the routes of a translation and of a parameter removal use `POST` (ADR-0040). A value travels as one field per leaf, not as JSON text (ADR-0039).
- A form answer lists every bad field at once, with a line under each input. This also applies to the plant forms (ADR-0041).
- `conflict.html`, `HX-Trigger` with the new `row_version`, body limits per route, and a message in `pl` and `en` for each code of the new routes.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: recipe, version, and parameter pages, the leaf grid, form error lists, the conflict fragment, and body limits.
- `recipe-management`: recipe queries, snapshot reads, values from leaf texts, and lists of value faults.

## Impact

- Crates `recipe-domain`, `recipe-app`, `recipe-db`, and `recipe-web`. No migration, grant, or dependency.
- ADR-0039 to ADR-0043 and five documents of `docs/architecture/`.

## Out of scope

- Approval, retirement, discard, removal and restore of a recipe, history, audit pages, diff, and the approval preview (step P3.4).
- Export (step P3.5).
- `serve`, Compose, and the supervised jobs (step P4.1).
- The deferred items of step P3.2 for step P4.2, and the error lists of the login, password, and user forms.
