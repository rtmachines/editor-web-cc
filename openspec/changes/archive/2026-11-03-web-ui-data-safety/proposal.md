# Proposal

## Why

The UI review of 2026-11-02 found three places where a user loses typed input or cannot enter valid data (findings A3, A4, A6).
Each one breaks the data safety of the UI (R31, R39) and needs a new ADR.
The quick-wins change moved B4 (UI-03) to this change. After a save of the definition, the English base text above the translations shows the old texts.
The change uses A17 (translation tags), A24 (read-only mode), and A44 (WCAG 2.2 AA).
It also serves R08 and R10 (descriptions and translations) and R24 and R28 (controls by permission).

## What Changes

- A3, ADR-0111: the deletion of one translation swaps only its own row. The other rows keep their unsaved input. The focus moves to the heading of the changed row.
- B4, ADR-0111 (i): the HTMX answer of a save of the definition also replaces the English base text out of band. It changes no translation row.
- A4, ADR-0112: a shared multi-line text field (`textarea`) for `description_en`, the description of a translation, and `change_note`.
  - `recipe-web` converts CRLF to LF before the parse. The browser limit and the server limit then count a line end as one character.
  - The browser counts UTF-16 code units, and the server counts Unicode characters. The server therefore accepts every text that the browser accepts.
  - A page shows a stored multi-line text with its line ends.
- A6, ADR-0113: in read-only mode a page hides every control of a blocked command. A form route shows a notice in place of the form. The login, the logout, the language switch, and the export stay.
- The example `dev_server` gets a read-only switch file for the browser tests.
- `api.md` §1, §2, §2.6 and `design-system.md` §10.1, §10.5 change with these ADRs.
- `CHANGELOG.md` `[Unreleased]` and `docs/user/` describe the new behavior.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: new requirements. They cover the answer of a translation deletion and the English base text after a save of the definition. They cover the multi-line text field and multi-line text outside a form. They also cover the pages in read-only mode.

## Impact

- `recipe-web` views: templates, `components/field.html`, the bundles `pl` and `en`, `static/app.css`, and `static/components.css`.
- `recipe-web` code: `PageRequest`, `UnitTree`, the page views, the form extractors, a source guard, and the example `dev_server`.
- `e2e/`: a second Playwright project and server, and new checks in three spec files.
- No change of a route, a status code, an error code, a migration, the domain, or the export document.

## Out of scope

- A live character counter. It needs a script (`api.md` §9).
- A conflict recovery view (F13), busy indicators (B1), and the other ideas of section D of the review.
- A change of the domain rule "LF or CRLF" of a long text (ADR-0028).
