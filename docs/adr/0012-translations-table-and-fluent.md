# ADR-0012: Parameter translations in a table and UI text with Fluent

Status: Accepted. Amended by ADR-0019 (translations are not exported) and ADR-0022 (description_en required)
Date: 2026-10-12
Deciders: project owner

## Context

The draft says: "Provide parameter names and descriptions in English. Permit translations into other languages. Use Polish as the default user interface language". Two kinds of text exist: data text (parameter names and descriptions, entered by users) and UI text (labels, buttons, messages, written by developers).

## Decision drivers

- English is the base for data text. It is always present.
- A translation must be unique per parameter and language.
- UI text must be Polish by default and must not fall through to a blank label.
- Translations are part of the version snapshot (ADR-0009).

## Considered options

### Option A: Translation table plus Fluent for UI text

`parameter_translation(parameter_id, lang, name, description)` with a unique constraint. UI text in `.ftl` bundles loaded by `fluent-rs`.

- Pros: foreign keys and unique constraints work, a missing translation is a missing row, Fluent handles plurals and gender for Polish.
- Cons: one more table to copy per version.

### Option B: JSONB column with a language map on the parameter row

- Pros: one column, no join.
- Cons: no constraint on language codes or duplicates, a partial update rewrites the whole map, no index on "parameters without a Polish name".

## Decision

We use Option A.

- `parameter.name_en` and `parameter.description_en` are the base text. Both are `NOT NULL` and not empty (amended by ADR-0022, Assumption A36).
- `parameter_translation` has one row per `(parameter_id, lang)`. `lang` matches `^[a-z]{2,3}(-[A-Z]{2})?$`, checked by the domain and by a pattern in the database (Assumption A17). `lang = 'en'` is not allowed in the table. English lives in the base columns.
- `CreateVersion` copies the translation rows with the parameter rows (ADR-0009).
- Translations and descriptions are not exported (amended by ADR-0019).
- UI text uses `fluent-rs`. Bundles exist for `pl` and `en`. The default is `pl`. A missing `pl` message falls back to `en`. A missing `en` message is a build error, checked by a test that loads every bundle and every message ID.
- The user picks the UI language with `POST /lang`. The plain cookie `recipe_lang` stores the choice, with the value `pl` or `en`. The session and `app_user` do not store it (`api.md` section 8). `POST /lang` works before login. Without a valid cookie the UI uses `pl`.
- UI English text follows ASD-STE100 (R40). Polish text follows the same sentence rules where the language allows it.
- The display name of a parameter in the UI is: translation for the UI language if present, else `name_en`.

## Consequences

### Positive

- Database constraints guarantee one translation per language.
- Queries such as "parameters without a Polish name" are simple SQL.
- Polish plural rules work correctly in UI messages.

### Negative

- A parameter read joins one more table. The read model loads translations for one language in one query.
- Developers must maintain two `.ftl` files per message.

### Risks and mitigations

- Risk: a template uses a raw string instead of a message ID. Mitigation: a lint test scans templates for Polish or English words outside Fluent calls.
- Risk: a translator enters an invalid language tag. Mitigation: the UI offers a fixed list of tags. The domain checks the tag.

## Related

- ADR-0001 (templates)
- ADR-0009 (translations in the snapshot)
- ADR-0019 (amendment: no translations in the export), ADR-0022 (amendment: `description_en` required)
- `domain-model.md` section 2.2 (`Translation`)
- `data-model.md` (`parameter_translation`)
