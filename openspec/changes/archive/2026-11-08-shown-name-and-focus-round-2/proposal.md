# Proposal

## Why

The full repo review of 2026-11-08 found four defects in `recipe-web`. `docs/plan/2026-11-08-fix-plan.md` section 5 (WP2) holds them.

- D5-3: the added and removed diff sections show `name_en` without the translation and without the marker `(en)`. ADR-0050 (d) and ADR-0126 disagree. The owner chose ADR-0126 (decision Q-E (1)).
- D5-4: the inputs `name_en` and `description_en` of the parameter form have no `lang="en"`.
- D5-2: after an HTMX approval or a parameter removal, the language switch can go back to the old URL. This defect does not reproduce. The owner accepted this result on 2026-11-08 (design, "Findings that differ from the plan", item 1).
- D5-1: the submit button of the own password form has no fixed `id`. A refused answer loses the focus.

The change satisfies R10, R27, and R39. It uses Assumptions A17 and A44.

## What Changes

- The new ADR-0152 amends ADR-0050 (d). The added and removed sections of the diff page and of the approval preview show the shown name of ADR-0126.
- The domain part of an added or a removed parameter carries the translated names of its side. No query changes.
- The English base text inputs of the parameter form get `lang="en"`.
- The submit buttons of the own password form and of the new user form get a fixed `id`.
- A template test asserts a fixed `id` on each submit button of each form whose start tag holds `hx-swap-oob`.
- The spec records the current rule of the language switch after a pushed address. No code changes for D5-2.
- `api.md` section 8 names the diff sections in the rule of the shown name.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: "Translation fallback" and "Language tag of the English base text" get scenarios. The new requirements "Language switch after a pushed address" and "Submit button of an out-of-band form" state the other two rules.

## Out of scope

- The definitions section and the translations section of the diff. ADR-0126 (f) counts their texts as labeled fields.
- A new query, statement, setting, error code, or bundle key.
- The export document. It holds no translation (ADR-0019).
- The focus after a success of the user page actions. This includes the buttons of `templates/user-actions.html`, which an answer swaps out of band in the wrapper `#user-actions`.
- A code change for D5-2.

## Impact

- `recipe-domain`: `DiffPart::Parameter` in `recipe/diff/`.
- `recipe-web`: `pages/diff_view.rs`, `pages/diff.rs`, `pages/approval/preview.rs`, and four templates: `components/diff.html`, `parameter-form.html`, `password.html`, and `user-new.html`.
- `recipe-web` tests: `approval_text_xss_tests.rs`, `review_fix_tests.rs`, the new `diff_name_tests.rs` and `components_tests/oob_forms.rs`, and the other test files of `tasks.md`.
- Documents: ADR-0152, the Status line of ADR-0050, `docs/adr/README.md`, `api.md` section 8, and `CHANGELOG.md`.
