# Proposal

## Why

The parameter table shows the English name `name_en` in every UI language. It ignores a stored translation and never shows the marker `(en)`. The review of 2026-11-04 reports this fault as D5-1 (HIGH). The requirement "Translation fallback" fails, and no test covers it.

The English name has no `lang="en"` in a Polish page, so a screen reader uses Polish pronunciation (D5-2, LOW).

The change satisfies R10 and R39. It uses A17 and A44.

## What Changes

- `ParameterRow` gets the translated name for one requested language. `parameter_page` takes that language and reads at most one translation row per parameter.
- `ListParameters` takes the translation language. The UI language `pl` gives `pl`. The UI language `en` gives none, because English is the base text (ADR-0012).
- Each row shows the translation when one exists. Otherwise it shows `name_en` with `lang="en"` and the marker `(en)`.
- The row fragment of `AddParameter` follows the same rule.
- With the UI language `en`, the row shows `name_en` without a marker. ADR-0126 (Proposed) records this decision. `api.md` section 8 gets the scope, the case `en`, and the language tag.
- `api.md` section 3 gets the real answer of `UpdateParameterDefinition`: the summary and the clean form (ADR-0126 (h)).

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: the requirement "Translation fallback" names its scope, a name or description without a label (ADR-0126 (f)). It gets four new scenarios: "Stored Polish translation", "English UI language", "Language tag of the English base text", and "Row of a new parameter".

## Impact

- Crates: `recipe-app` (port, query input, memory adapter), `recipe-db` (`SELECT_PAGE`), and `recipe-web` (the two row builders, `components/parameter.html`, the bundles).
- Documents: ADR-0126, `api.md` sections 3 and 8, the index and the decision map of `docs/adr/README.md`, an "Amended by" note on ADR-0012, and `CHANGELOG.md`.
- No migration, error code, permission, setting, or dependency.

## Out of scope

- A description column. No page shows a description without a label.
- The labeled fields of the English base text: the parameter form, the parameter summary, the base block of the translations, and the diff view.
- The export document. ADR-0019 excludes translations from it.
- A translation language other than `pl` in the UI. `i18n.translation_langs` does not change.
- A filter or a sort of the parameter table by the shown name.
