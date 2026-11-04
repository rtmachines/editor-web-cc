# Design

## Context

See `proposal.md`, section "Why". The rule is in `api.md` section 8, ADR-0012, and the `web-ui` requirement "Translation fallback". ADR-0126 (Proposed) adds the case of the UI language `en`. This design links to them and does not repeat them.

The state of the code at HEAD `3885a26`:

| Part | State | Place |
|---|---|---|
| Read-port row | `ParameterRow` holds `name_en` and no translation. Its doc comment says that the page read loads no translation row | `crates/recipe-app/src/ports/recipe_reads.rs:118` |
| Port method | `parameter_page(version, unit, offset, limit)` takes no language | `recipe_reads.rs:218` |
| Statement | `SELECT_PAGE` joins `parameter` and `unit` only | `crates/recipe-db/src/transaction/parameter_reads.rs:31` |
| Memory adapter | `table_row` copies `name_en`. The stored row already holds `translations` by `LangCode` | `crates/recipe-app/src/memory/recipe/reads.rs:233`, `memory/recipe/rows.rs:92` |
| Query input | `ListParametersInput` has no language | `crates/recipe-app/src/recipe/queries/parameters.rs:72` |
| Table builder | `row_view` sets `name: row.name_en.to_string()` | `crates/recipe-web/src/pages/versions/parameter_table.rs:282` |
| Row of a new parameter | `added` sets `name: definition.name_en.to_string()` | `crates/recipe-web/src/pages/parameters/new.rs:101` |
| Template | `<td>{{ row.name }}</td>`, no `lang` | `crates/recipe-web/templates/components/parameter.html:4` |

### Corrections of the review and the plan

- The plan names "the row fragment of the definition update" as a third builder of `ParameterRowView`. That builder does not exist. The HTMX answer of `UpdateParameterDefinition` holds the summary, the clean form, and the base block of the translations (`pages/parameters/edit.rs:63` to `:94`). It holds no table row. The two builders are `parameter_table.rs:279` and `new.rs:98`.
- `api.md` section 3 names a "parameter row fragment" as the answer of `UpdateParameterDefinition`. The owner decided on 2026-11-04 that the code is right. Task 1.3 corrects the answer column, and ADR-0126 (h) records the correction. The change adds no third builder.
- The table of the rebind answer is not a third builder. `pages/versions/rebind.rs:264` calls `ListParameters` and renders the rows with `row_view`.
- The plan asks for "the page size test of ADR-0122". ADR-0122 has no page size test. The byte bound test of the large draft is `versions_large_draft_shows_one_unit_with_at_most_one_hundred_rows_under_a_byte_bound` (`pages/versions/large_tests.rs:33`, ADR-0043 (j)). It reads the page with the UI language `en`, so the marker does not reach it. Task 4.3 adds the same bound for `pl`. The comment of the new test records the measured size of the `pl` page.
- The plan states "No ADR" for this change. `api.md` section 8 does not settle the UI language `en`. ADR-0126 (Proposed) records the choice, as the brief of the change asks.

### Scope: a parameter name or description without a label

A search of `crates/recipe-web/templates` and `crates/recipe-web/src` gives these places that show `name_en` or `description_en`:

| Place | Kind | In scope |
|---|---|---|
| `components/parameter.html:4`, the row of the parameter table and of the row fragment | Name without a label | Yes |
| `parameter-summary.html:11` to `:14` | Labeled field `field-name-en` and `field-description-en` with `lang="en"` | No |
| `parameter-form.html:13`, `:14` | Input of the English base text | No |
| `translations.html:5`, `:6` | Base block of the translations with `lang="en"` (ADR-0111 (i)) | No |
| `translation-row.html:16` | Stored translation with its own `lang` | No |
| `pages/diff_view.rs:619`, `:631` | Diff of stored content. The base text has `lang="en"`. Each translation row shows its tag | No |

No page shows a description without a label. The table has no description column. So the change carries only the name through the read port. The type usage page and the outdated list of the version page show the tag and no name (`pages/types/usage.rs:83`, `recipe-app/src/recipe/queries/outdated.rs:22`).

## Goals / Non-Goals

**Goals:**

- Satisfy the requirement "Translation fallback" in the parameter table and in the row fragment of `AddParameter`.
- Read at most one translation row per parameter in the page read, as ADR-0012 "Consequences" states.
- Keep one function for the shown name.

**Non-Goals:**

- A change of the labeled fields of the English base text, the diff, or the export.
- A description column in the table.
- A new translation language or a new UI language.

## Decisions

### D1. Read port (ADR-0012, ADR-0121 (g))

- `ParameterRow` gets `translated_name: Option<Text>`. It holds the name of the translation row of the requested language, or `None`.
- `parameter_page` gets the parameter `lang: Option<&LangCode>` after `unit`. `None` reads no translation.
- The doc comments of `ParameterRow` and `parameter_page` name the one translation name. The page read still loads no value and no limit row (ADR-0108 (b), ADR-0121 (g)).
- `GetVersion` calls `parameter_page` with `None`. It uses the rows only for the outdated list (`versions.rs:487`).
- `rows_of` of the approval preview sets `translated_name: None`. Its rows feed only `outdated_with_trees` (`approval_preview.rs:305`).

ADR-0121 and ADR-0122 permit the join. ADR-0121 (g) keeps `parameter_page` free of values, and a translation name is not a value. ADR-0122 bounds the values of the page, not the names. A translation name has at most 200 characters, as `name_en` has (`data-model.md` section 1.3).

### D2. Statement (`data-model.md` section 3.14)

- `SELECT_PAGE` adds `LEFT JOIN parameter_translation t ON t.parameter_id = p.id AND t.lang = $5` and the column `t.name AS translated_name`.
- The primary key `(parameter_id, lang)` gives at most one row for each parameter. So `OFFSET` and `LIMIT` still count parameters.
- A bound `NULL` for `$5` matches no row. The statement count of the method stays one.
- `PAGE_COLUMNS` of the storage test `postgres_only_the_page_statement_returns_exactly_the_row_columns` gets `translated_name`.
- The memory adapter reads `translations.get(lang)` of the stored row. A contract case runs on both adapters. It requests `pl`, `de-AT`, a tag without a stored translation, and `None`.
- The match of the tag is exact (ADR-0126 (a)). A stored `pl-PL` row does not match the request `pl`.

### D3. Query input

- `ListParametersInput` gets `lang: Option<LangCode>`. `ListParameters` passes it to `parameter_page`.
- `Lang` of `recipe-web` (`i18n.rs:42`) gets `translation_lang(self) -> Result<Option<LangCode>, RecipeError>`. `Pl` gives `LangCode::parse("pl")`. `En` gives `None` (ADR-0126 (a)). `En` has no literal, because `LangCode::parse` refuses `en` (ADR-0012).
- The three callers of `ListParametersInput` pass the value of `translation_lang`. They are `pages/versions/page.rs:239`, `pages/parameters/table.rs:71`, and `pages/versions/rebind.rs:264`.
- A failed parse of the literal is a program fault, not a fault of the user input. The owner decided on 2026-11-04 that it gives `500 E-WEB-010`. Each caller maps the `Err` with `map_err` to `WebError::Internal` and its `request_id`. `reliability.md` section 1.2 gets no new row.
- `WebError` has no `From<RecipeError>`, so a bare `?` does not compile. `422 E-RCP-027` of `path_values.rs:180` is a code for user input, and it does not apply.
- A unit test of each arm shows that `Pl` gives `Some(pl)` and `En` gives `None`. The `Pl` arm parses the literal, so a bad literal fails the test run.

### D4. View (ADR-0126 (b) to (e))

- A free function `shown_name(t, translated, name_en) -> ShownName` lives in `pages/versions/parameter_table.rs`. `ShownName` holds `text: String` and `marker: Option<String>`.
- `shown_name` reads the UI language from `Texts`. `Texts` gets the accessor `lang()` for the private field `lang` (`pages/texts.rs:19`). One argument then holds the language, and a caller cannot pass a marker text of one language with the rule of the other.
- A translation gives its name and no marker. Without a translation, `Pl` gives `name_en` and the text of `parameter-name-base-marker`. `En` gives `name_en` and no marker.
- `ParameterRowView.name` becomes `ShownName`. `row_view` and `added` call `shown_name`. `added` passes `None`, because a new parameter holds no translation.
- `components/parameter.html` renders `<td><span lang="en">{{ name }}</span> {{ marker }}</td>` when a marker exists, else `<td>{{ name }}</td>`. The text of the cell is then `Line speed (en)`. Askama escapes both values. No `|safe` filter applies.
- `pl.ftl` and `en.ftl` get `parameter-name-base-marker = (en)`. ADR-0012 requires the key in `en` too. With the UI language `en`, no page uses it. The existing key test of `pages/texts/tests.rs:12` covers the new key, because `shown_name` names it as a literal in `t.get`.

### D5. Order of the work

Tasks 2.1, 3.1, 4.1, 4.2, and 4.3 run in this order. Task 2.1 changes the port and the input and keeps the workspace compiling. The `recipe-db` adapter takes the language and returns no translated name, and the web callers pass `None`. Task 3.1 then writes and tests the statement. Task 4.1 adds the view. Task 4.2 sets the language in the version page. Task 4.3 sets it in the table fragment and the rebind answer.

Tasks 1.2 and 1.3 both change `api.md`, so task 1.3 runs after task 1.2. The tasks of section 1 do not depend on the code tasks.

## Risks / Trade-offs

- [The page read gets one join for each row] → The primary key gives one index lookup per row. The `en` UI binds `NULL` and reads no translation.
- [Tests that build `ParameterRowView` by hand break] → Task 4.1 updates `parameter_table_tests.rs`.
- [ADR-0126 stays Proposed] → The owner accepts it in task 1.1 before the code. A rejection changes D3, D4, and the scenario "English UI language".
- [A new page shows a name without the rule] → `shown_name` is the one function. The template comment of `components/parameter.html` names it.

## Rejected review points

None. The plan review of 2026-11-04 (Opus and Codex) gave 15 findings. This revision applies each one.
