# ADR-0152: Shown name in the diff sections

Status: Accepted. Amends ADR-0050 (d) (the parameter name of two diff sections)
Date: 2026-11-08
Deciders: project owner

## Context

ADR-0050 (d) gives each row of the added and the removed sections these cells: tag, `name_en`, unit path, type, and the value of a scalar. The diff page and the approval preview share these sections.

ADR-0126 (b) to (e) give the shown name of a parameter. A page shows the translation of the translation language when one exists. Otherwise a page in the UI language `pl` shows `name_en` with the marker `(en)`. ADR-0126 (f) applies this rule to each place that shows a parameter name without a label. The web-ui requirement "Translation fallback" states the same rule.

The two ADRs disagree for the added and the removed sections. The code follows ADR-0050 (d). The review of 2026-11-08 (D5-3) found these effects:

- A Polish Approver sees `Line speed` in the added section and `Prędkość linii` in the parameter table of the same version.
- A name without a translation shows without the marker `(en)`.
- On an English page, the name cell has `lang="en"`. ADR-0126 (e) permits no extra `lang` attribute there.

These facts shape the decision:

- The diff reads the pairs of one page with their limits and their translations in its `RepeatableRead` transaction (ADR-0050 (f)). The preview reads the whole draft with its translations in the same transaction.
- The domain part of an added or a removed parameter holds only `name_en`. The translations stop in the domain diff.
- One function of `recipe-web` builds the shown name (ADR-0126, "Risks and mitigations").

The project owner decided on 2026-11-08 (decision Q-E (1) of `docs/plan/2026-11-08-fix-plan.md`).

## Decision drivers

- One rule of the shown name applies to every page (R10, ADR-0126 (f)).
- A second person sees each added and each removed parameter with the name of the parameter table (R27).
- A screen reader reads each text in its own language (R39, A44, WCAG 2.2 SC 3.1.2).
- The approve form and the shown rows come from one snapshot (ADR-0050 (f)).
- No new query, no new setting, no new error code, and no new bundle key.

## Considered options

### Option A: The shown name of ADR-0126 in the added and the removed sections

- Pros: every page shows one name for one parameter. A Polish user sees which names have no translation.
- Cons: the domain part of an added or a removed parameter carries more data.

### Option B: Keep `name_en` and exempt the two sections from ADR-0126 (f)

- Pros: no code change.
- Cons: the diff and the parameter table show two names for one parameter. The marker does not tell the user about a missing translation.

## Decision

The project uses **Option A**. It amends ADR-0050 (d).

- (a) Each row of the added and the removed sections shows the shown name of ADR-0126 (b) to (e) in place of `name_en`. The other cells of ADR-0050 (d) stay.
- (b) The added section takes the name from the new side. The removed section takes the name from the old side.
- (c) The name comes from the translations that the pair read already holds. The diff page and the approval preview read them in their `RepeatableRead` transaction of ADR-0050 (f). This ADR adds no statement and no query.
- (d) The domain part of an added or a removed parameter carries the translated name of each language of its side. The domain diff knows no UI language.
- (e) `recipe-web` selects the translation of the translation language with the exact match of ADR-0126 (a). It then calls the one function of the shown name.
- (f) Without a translation in the UI language `pl`, the element that holds `name_en` has `lang="en"`. The marker stands outside that element (ADR-0126 (d)).
- (g) In the UI language `en`, the name shows without the marker and without a `lang` attribute (ADR-0126 (e)).
- (h) The definitions section and the translations section keep their texts. A row of these sections names its field or its language. ADR-0126 (f) therefore counts its texts as labeled fields.

## Consequences

### Positive

- The diff, the approval preview, and the parameter table show one name for one parameter.
- A Polish Approver sees which added parameters have no translation before the approval.

### Negative

- The domain part of an added or a removed parameter holds one name per stored translation. The fixed list of language tags (ADR-0043 (f)) bounds that count.
- `api.md` section 8 gets one more sentence. ADR-0050 gets an "Amended by" note.

### Risks and mitigations

- Risk: a later section shows a parameter name without the rule. Mitigation: a web test of each UI language covers the added and the removed sections of both pages.
- Risk: the translation and the rows come from two states of the database. Mitigation: item (c) reads no translation outside the transaction of ADR-0050 (f).

## Related

- ADR-0012 (translations), ADR-0043 (f) (fixed list of language tags), ADR-0050 (amended by this ADR in (d)), ADR-0126 (marker of the English base text)
- `docs/architecture/api.md` sections 2.5 and 8
- `overview.md` section 7: R10, R27, R39. Section 9: A17, A44
