# ADR-0126: Marker of the English base text in each UI language

Status: Accepted. Amends ADR-0012 and the translation sentence of `api.md` section 8
Date: 2026-11-04
Deciders: project owner

## Context

ADR-0012 gives the shown name of a parameter: the translation for the UI language if present, else `name_en`. `api.md` section 8 adds a marker. Without a translation, the English base text appears with the marker `(en)`.

The UI has two languages, `pl` and `en` (A17). ADR-0012 does not permit a translation row with the language `en`, because the base columns hold the English text. With the UI language `en`, a translation never exists.

`api.md` section 8 states the marker for each UI language. The `web-ui` requirement "Translation fallback" states the same rule. This rule puts the marker on every name of an English page. The marker then tells the user nothing, because every name on that page is the English base text.

This ADR amends that stated rule of `api.md` section 8 and of the current requirement. It does not fill a gap.

The review of 2026-11-04 (D5-1) found that no page shows the marker. A fix must choose the behavior for `en` first.

## Decision drivers

- The marker tells the user that a translation is missing for the UI language (R10).
- A page shows no text that carries no information (R39).
- A screen reader reads each text in its own language (WCAG 2.2 SC 3.1.2, A44).
- No new setting, no new error code, and no new table.

## Considered options

### Option A: No marker when the UI language is `en`

- Pros: an English page shows clean names. The marker shows only where it tells the user something.
- Cons: the rule of `api.md` section 8 gets more sentences.

### Option B: The marker on every English base text, also in the UI language `en`

- Pros: the literal text of `api.md` section 8 stays.
- Cons: each row of an English page shows `(en)`. The marker carries no information there.

### Option C: No marker in any UI language

- Pros: no bundle key and no template branch.
- Cons: a Polish user cannot see which names have no translation. The `web-ui` requirement "Translation fallback" fails.

## Decision

The project uses **Option A**. It amends the rule of the shown name in ADR-0012 and the translation sentence of `api.md` section 8.

- (a) The translation language of a page comes from the UI language. The UI language `pl` gives the translation language `pl`. The UI language `en` gives no translation language. The tag of a stored translation must equal the translation language exactly. A regional tag such as `pl-PL` does not match `pl`. A regional match needs a new ADR.
- (b) A page shows the stored translation of the translation language when one exists. The translation then shows without a marker.
- (c) Without a translation, a page in a UI language other than `en` shows `name_en` with the marker. The marker text comes from the bundle key `parameter-name-base-marker`.
- (d) In that case, the element that holds `name_en` gets `lang="en"`. The marker stands outside that element, so the screen reader reads it in the UI language.
- (e) With the UI language `en`, a page shows `name_en` without the marker and without an extra `lang` attribute.
- (f) The rule applies to each place that shows a parameter name or description without a label. A labeled field of the English base text is not in its scope. It keeps its label and its `lang="en"`.
- (g) A third UI language follows items (a) to (e). Its translation language is the tag of that UI language, with the exact match of item (a). It needs a new ADR only for its own translation language.
- (h) The HTMX answer of `UpdateParameterDefinition` holds no row of the parameter table. It holds the parameter summary, the clean form out of band (ADR-0041 (h)), and the block `#translations-base` (ADR-0111 (i)). These parts are labeled fields of item (f), so the rule of the shown name does not apply to them. `api.md` section 3 names a parameter row fragment as that answer, and the code never sends one. This ADR sets that answer column to the answer of the code.

## Consequences

### Positive

- A Polish user sees which names have no `pl` translation.
- An English page shows each name once and without noise.
- A screen reader reads the English base text with English pronunciation.

### Negative

- The page read of the parameter table reads one translation row per parameter in a Polish page.
- `api.md` section 8 gets three more sentences. `api.md` section 3 gets the answer of `UpdateParameterDefinition` that the code sends. ADR-0012 gets an "Amended by" note.

### Risks and mitigations

- Risk: a new page shows a parameter name without the rule. Mitigation: one function in `recipe-web` builds the shown name, and each builder of a table row calls it.

## Related

- ADR-0012 (translations, shown name), ADR-0019 (no translations in the export), ADR-0043 (f) (fixed list of language tags)
- ADR-0121, ADR-0122 (the reads of the parameter table)
- ADR-0041 (h), ADR-0111 (i) (the answer of a save of the definition)
- `api.md` sections 3 and 8, `design-system.md` section 1
- `overview.md` section 7: R10, R39. Section 9: A17, A44
