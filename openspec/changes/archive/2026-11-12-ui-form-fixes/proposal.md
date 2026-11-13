# Proposal

## Why

The UI and UX audit of 2026-11-12 (`docs/plan/review/2026-11-12-ui-ux-browser-audit.md`) found three form defects. A test in a browser on the current `main` showed each defect.

- F2: on a parameter page after the last page, the form of a new parameter sends no request. The console shows `htmx:targetError` and the page shows no error.
- F4: Enter in the UDT form sends `add=1`, because "Add a member" is the first submit button. At 1024 rows that button is gone, so Enter creates the type or the revision.
- F5: a change of the parent in the unit form keeps the heading and the proposed segment of the first parent. The save then fails with `E-PLT-004`.

The fix of F4 makes Enter save a revision form at once. Without a check, Enter in an unedited revision form writes a revision with the same members. The change therefore records ADR-0159 and the new code `E-PLT-052`.

The change satisfies R06, R07, R08, R13, R25, and R39. It uses A30 and A44.

## What Changes

- The version page after the last page links "Add a parameter" to the page form of a new parameter. The link targets `#content`, and the page form needs no `#parameter-rows`.
- The UDT form puts the button "Create type" before "Add a member" in the DOM order. Enter then always sends the form without `add`, as ADR-0037 (h) requires. The visual order follows the DOM order.
- New ADR-0159: `DefineUdt` in the revision mode refuses members that equal the latest revision with `409 E-PLT-052` and writes nothing.
- The unit form has no parent select. The form keeps the parent of its link in a hidden field `parent_path` and shows that parent in a text line. The `h2` and the proposed segment always belong to that parent. The field `parent_path` of `POST /machines/{code}/units` does not change.
- An error of the parent (`E-PLT-003`, `E-PLT-009`, `E-PLT-017` on `parent_path`) stands in the summary without a link, because the form has no parent input.
- Documents: ADR-0159, ADR-0160 (it amends ADR-0043 (i) for the add link after the last page), `domain-model.md` section 5.1, `data-model.md` section 6, `reliability.md` section 1.2, and `api.md` section 2.3.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the requirements "Form of a new parameter", "UDT form with member rows", and "Addition of a unit" get new scenarios. "UDT form with member rows" also states the HTTP answer to the refusal of an unchanged revision. "Addition of a unit" also states the fixed parent.
- `plant-model`: the requirement "Struct revision" states the refusal of an unchanged revision with `E-PLT-052`.

## Impact

- Crate `recipe-domain`: the variant `PlantError::RevisionUnchanged` and the constant `E-PLT-052`.
- Crate `recipe-app`: the comparison of the members in `plant/define_udt.rs`.
- Crate `recipe-web`: `templates/version.html`, `templates/udt-form.html`, `templates/unit-form.html`, `src/pages/versions/page.rs`, `src/pages/units/form.rs`, `src/pages/units/add.rs`, `src/error/status.rs`, the key `error-E-PLT-052` in `en.ftl` and `pl.ftl`, and their tests.
- `e2e/tests/`: new browser tests for F2, F4, and F5.
- No change in `recipe-db`, the routes, the form fields, `static/`, or the database schema.

## Out of scope

- F1 and F3 of the audit. The change `ui-request-state` takes them with ADR-0158.
- The other gaps of the audit, for example a full axe run and a test with a screen reader.
- A choice of another parent inside the unit form. The link of each node of the tree makes that choice.
- A removal of the list of parents from the query `GetUnitForm`.
- A visual redesign of the three forms.
