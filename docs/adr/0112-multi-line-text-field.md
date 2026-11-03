# ADR-0112: Multi-line text field

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

A description and a change note are long texts. They have at most 4 000 characters and can hold line ends (`reliability.md` section 4.3, ADR-0028). A line end is LF or CRLF.

- The forms show these texts in `input type="text"`. The fields are `description_en` of the parameter form, `description` of a translation row, and `change_note` of the recipe form and the version form.
- An `input type="text"` holds one line. The HTML value sanitization removes every LF and CR from its value. A user therefore cannot type a line end.
- The edit form of a parameter sends `description_en` with each save of the value (ADR-0039). A stored line end therefore goes away at the next save of an unrelated value.
- A browser sends each line end of a `textarea` as CRLF. `maxlength` counts a line end as one character, but the domain counts CRLF as two characters. A text inside `maxlength` can then fail as too long.
- The pages show a stored description in a `dd` element. HTML collapses a line end there into a space.

The UI review of 2026-11-02 (finding A4, UI-01) reports the problem. A check of `components/field.html`, `parameter-form.html`, `translation-row.html`, `recipe-new.html`, `version-new.html`, and `recipe-domain/src/text.rs` finds the same fault.

## Decision drivers

- A form keeps every character of a stored text through a save (R31, R10).
- The browser limit and the server limit count a line end as one character (R38).
- No script, no inline style, and no `|safe` (`api.md` section 9, `design-system.md` section 12).
- One component for every multi-line field, with the label, hint, and error rules of `design-system.md` section 10.5.

## Considered options

### Option A: One shared `textarea` macro and LF on input

- Pros: one macro gives every long text the same label, limit, error line, and escaping. The server sees one line end form.
- Cons: the web layer changes the line ends before the parse.

### Option B: A `textarea` in each template

- Pros: no new macro.
- Cons: four copies of the label, error, and `aria-describedby` rules drift apart.

### Option C: Keep `input` and show a hint that line ends are not possible

- Pros: no change of the forms.
- Cons: the domain permits line ends, and a save still removes stored line ends.

## Decision

The project uses **Option A**.

- (a) `components/field.html` gets the macro `lines`. It renders a `textarea` with the contract of the macro `named`. That contract covers the `id`, the `name`, the label, the hint, the required mark, `maxlength`, `lang`, the error lines, `aria-invalid`, and `aria-describedby`.
- (b) The multi-line fields are `description_en`, the `description` of a translation row, and `change_note`. Every other text field stays a single-line `input`.
- (c) The `textarea` has `rows="4"` and `maxlength="4000"` (`text.description_max` and `text.change_note_max`, `reliability.md` section 7.11). It has no `placeholder`, `required`, `pattern`, or `minlength` (`design-system.md` section 10.5).
- (d) The template writes one LF directly after the start tag of the `textarea`. The HTML parser removes that LF. A stored text that starts with a line end therefore keeps it.
- (e) The value goes through the default HTML escaping of askama. No filter `|safe` applies.
- (f) Each `textarea` has a limit line with the id `<id>-limit`. The line shows the bundle key `field-lines-limit` with the maximum count. `aria-describedby` names the hint, the limit line, and the error line, in this order. The page shows no live counter, because a counter needs a script.
- (g) `recipe-web` converts each CRLF of a multi-line field to LF before the parse of the value object. One free function does the conversion for every multi-line field. A lone CR stays, and the parse refuses it as a control character (ADR-0028).
  - The domain rule stays "LF or CRLF". Other callers of the domain do not change.
  - A stored text with CRLF stays in the store. The next save of the form stores LF.
  - `maxlength` counts UTF-16 code units, and `LongText::parse` counts Unicode characters. A character outside the Basic Multilingual Plane counts two in the browser and one on the server.
  - The server therefore accepts every text that the browser accepts. A test sends 4 000 such characters and expects success.
- (h) A page that shows a multi-line text outside a form puts it in an element with the class `text-lines`. The class sets `white-space: pre-wrap` and `overflow-wrap: anywhere`. The places are:
  - the parameter summary, the English base text, and the read-only translation rows,
  - the change note of the version page and the change notes of the version table,
  - the description cells of the diff.
- (i) Enter in a `textarea` adds a line end and does not send the form. The rule of ADR-0039 (i), that Enter saves, applies to the single-line inputs of the form.
- (j) A `textarea` has the look of an input of `design-system.md` section 10.5 and the text font, not `--font-mono`. It can change its height (`resize: vertical`) and keeps the target size of section 7.

## Consequences

### Positive

- A user can type a line end, and a save keeps it.
- The browser limit and the server limit count each line end as one character.
- One macro holds the rules of every long text field.

### Negative

- A stored text with CRLF changes to LF at its next save. The diff then shows a changed text. The UI could not create such a text before this ADR.
- The parameter form is longer, because each `textarea` has four rows.

### Risks and mitigations

- Risk: a template adds whitespace inside the `textarea` and so changes the value. Mitigation: a unit test renders a text with a leading LF, a trailing LF, and `<`, and parses the value back.
- Risk: the browser and the server count in different units. Mitigation: after point (g) the server counts one character for each line end and for each Unicode character. The browser counts at least as many units, so the server accepts every text that the browser accepts.

## Related

- ADR-0028, ADR-0036, ADR-0039 (i), ADR-0041, ADR-0043 (f)
- `docs/architecture/api.md` section 1, `docs/architecture/design-system.md` section 10.5, `docs/architecture/reliability.md` sections 4.3 and 7.11
- `overview.md` section 7: R08, R10, R31, R38, R39. Section 9: A44
