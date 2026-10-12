# Documentation rules

Source: `docs/architecture/reliability.md` §8, `docs/adr/README.md`, `overview.md` §8, §9. Load the skills `asd-ste100` and `documentation-writer` before you write docs.

## ASD-STE100 structural rules

- A description sentence has at most 25 words. A procedure step has at most 20 words.
- Use the active voice and name the actor.
- Write one idea or one instruction in one sentence. Do not use semicolons.
- Use a numbered list for a sequence of three or more steps.
- Use simple tenses. Keep a compound tense only when current relevance is the point.
- Do not use soft phrasal verbs (`spin up`, `kick off`), marketing adjectives (`seamless`, `robust`), or nominalizations (`perform an analysis`).
- Use one word for one meaning in one file. Do not rotate synonyms such as `check`, `verify`, and `confirm`.
- A list item does not end with "and" or "or".
- Keep every hedge. "May fail" never becomes "fails".
- A paragraph has one topic and at most six sentences.
- `tools/ste-lint/ste-lint.py` reports 0 hard violations on the scope of ADR-0023. Passive voice and compound tenses are advisory.

## Diátaxis for docs/

- Each page is one Diátaxis type: tutorial, how-to guide, reference, or explanation. Do not mix two types in one page.
- `docs/architecture/` is reference and explanation. `docs/adr/` is explanation of decisions. Both are normative.
- Operator runbooks are how-to guides in `docs/ops/`. User guides and tutorials go to `docs/user/`.
- `docs/plan/` holds operator prompts in Polish and the review history. It is not normative.

## ADRs (docs/adr/README.md)

- Write one ADR for one decision. Copy the shape of an existing ADR.
- Use the next free number and a short slug: `NNNN-<slug>.md`. Add a row to the index and update the decision map.
- An accepted ADR changes only by its Status line and an "Amended by" note.
- A full replacement is a new ADR. The old status becomes "Superseded by ADR-NNNN".
- A change of `docs/architecture/` goes in the same change as the ADR that records it.

## Assumptions (overview.md §9)

- An Assumption is a requirement that is not in `general-draft.txt`.
- Each Assumption is one row `A<n>` in `overview.md` §9. That row is the authority for the number and the text.
- When `domain-model.md` uses an Assumption, §8 there holds an identical copy of the row.
- Use the next free number. Never reuse a number.

## Design documents

- A document in `docs/architecture/` holds no N-numbers and no review ids (finding ids, gate ids). `docs/plan/review/` keeps that history.
- Cite with a document and a section, for example `reliability.md` §1.2, ADR-0015, R28, A43, or I12.
- Each rule lives in one document. Other documents link to it and do not repeat it.
- A number in a document must equal `reliability.md` §7.11.
- Use the terms of `domain-model.md` §7 and the permission names of `api.md` §3.
