# ADR-0111: Answer of the deletion of one translation row

Status: Accepted
Date: 2026-11-03
Deciders: project owner

## Context

The parameter page shows one translation row per tag (ADR-0043 (f)). Each row of a live draft is one form. A stored translation also has a delete form in a danger disclosure (ADR-0110).

- The save of one row targets `closest .translation-row` with `outerHTML`. The answer holds only that row.
- The delete form targets `#translations` with `innerHTML`. The answer renders every row again from the stored data.
- A user can type into the row `pl` and then delete the row `de` before a save of `pl`. The answer replaces the row `pl`, and the typed text is lost.
- After the swap, the delete button no longer exists. The browser then puts the focus on `body`, and a keyboard user loses the position.
- The English base text stands once above the rows. A save of the definition changes `name_en` and `description_en`. Its HTMX answer replaces the summary and the form, but not the base text. The base text shows the old texts until the next page load.

The UI review of 2026-11-02 (findings A3, UI-02, and B4, UI-03) reports these faults. A check of `translation-row.html`, `translation_routes.rs`, `translations.html`, and `edit.rs` finds the same faults.

## Decision drivers

- The answer of one form never replaces the unsaved input of another form (R31, R39).
- Each `hx-*` value stays a literal, and no script is added (`api.md` section 9, ADR-0037 (f)).
- The focus stays at a known place after a swap (A44, `design-system.md` section 7).
- The sibling forms stay valid for the next save (ADR-0008, ADR-0015).
- After a save, every place of the page shows the saved English texts (R08).

## Considered options

### Option A: Swap only the row of the deleted tag

- Pros: the other rows and their input stay. The save and the delete use the same target.
- Cons: the answer must give the focus a new place, because the button goes away.

### Option B: Keep the full answer and add a warning to the disclosure

- Pros: no change of the answer.
- Cons: the input of the other rows is still lost. A warning does not prevent the loss.

### Option C: Send the unsaved input of every row with the deletion

- Pros: the answer can render the sent texts again.
- Cons: the form must include the fields of other forms. That needs a script or `hx-include` with selectors, and the server must read fields that the command does not use.

## Decision

The project uses **Option A**.

- (a) The delete form of a translation row has `hx-target="closest .translation-row"` and `hx-swap="outerHTML"`. Both values are literals. The form keeps `hx-boost="true"`, `hx-push-url="false"`, and `hx-sync="closest form:drop"`.
- (b) The HTMX answer of a successful `RemoveTranslation` holds one row: the row of the deleted tag with no stored translation.
  - A tag of `i18n.translation_langs` gets an empty form, the same row that the page shows for a tag without a translation.
  - A tag outside the list gets the row with its heading and the sentence `translation-none`. The row holds no form. The next page load omits the row (ADR-0043 (f)).
- (c) The heading `h3` of the answer row has `tabindex="-1"` and `autofocus`. HTMX 2 then moves the focus to the heading, as for the `h1` of a content answer (`design-system.md` section 10.1).
- (d) The answer also holds the toast and an empty `#error-panel` out of band, as today. The toast names the tag and the new `row_version` (ADR-0108 (f)).
- (e) The answer sets `HX-Trigger` with the new `row_version`. `row-version.js` writes it into every `row_version` field of the page (ADR-0015). The other rows keep their unsaved input and get the new `row_version`.
  - ADR-0008 compares the sent `row_version` of every request with the stored value. Any mismatch gives `409`, also a stale retry of the same session.
  - A later save of another row sends the new value from its hidden field. That save therefore gives no `409` for this deletion. This is the same rule as for a save of one row today.
- (f) An error answer keeps the rules of `api.md` section 5.1. HTMX puts the fragment into `#error-panel` (ADR-0036), and no row changes. A `409 E-WEB-001` shows `conflict.html` (ADR-0043 (n)).
- (g) A request without `HX-Request` keeps the `303` to the parameter page.
- (h) The route `GET .../translations` keeps its answer of every row. The page and this route render `translations.html`. No template sends a request to this route. This ADR changes only the answer of the deletion.
- (i) The English base text is one block with the literal `id="translations-base"`. One macro of `translations.html` renders it for the page and for the answer of a save of the definition.
  - The HTMX answer of a successful `UpdateParameterDefinition` also holds that block with the literal `hx-swap-oob="true"` and the new texts.
  - That answer holds no translation row. The rows and their unsaved input stay.
  - A request without `HX-Request` keeps the `303` to the parameter page. The next page shows the new base text.

## Consequences

### Positive

- A deletion keeps the unsaved input of every other row.
- The focus lands on the heading of the changed row, so a screen reader reads the tag.
- The save and the delete of a row use the same target.
- After a save of the definition, the base text above the rows shows the new English texts.

### Negative

- The row of a tag outside the list stays visible with the sentence `translation-none` until the next page load.
- The answer of the deletion and the answer of `GET .../translations` now differ. Each has its own test.
- The answer of a save of the definition now holds three parts: the summary, the form, and the base text.

### Risks and mitigations

- Risk: HTMX 2 ignores `autofocus` in the swapped row. Mitigation: a Playwright test checks the focus after a deletion (ADR-0048).
- Risk: a later save of another row gets `409` after a deletion. Mitigation: a Playwright test saves the row `pl` after the deletion of `de` and expects success.

## Related

- ADR-0008, ADR-0015, ADR-0036, ADR-0037 (f), ADR-0043 (f), (n), ADR-0048, ADR-0108 (f), ADR-0110
- `docs/architecture/api.md` section 2.6, `docs/architecture/design-system.md` section 10.1
- `overview.md` section 7: R08, R10, R31, R39. Section 9: A17, A44
