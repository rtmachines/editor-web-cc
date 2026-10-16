# ADR-0039: Form of a parameter value

Status: Accepted. Amended by ADR-0045 (groups of a nested array in (l))
Date: 2026-10-15
Deciders: project owner

## Context

Step P3.3 adds the pages of `api.md` sections 2.4 to 2.6. A parameter page edits the value and the process limits of one parameter. The documents leave these points open or disagree:

- `api.md` section 2.6 says that the field `value` is JSON text and that the form builds it from one input per leaf. Only three scripts exist (`api.md` section 9). No script of at most 50 lines can build the JSON text of a type tree.
- The field `new_member_values` of `RebindTypeRevision` is JSON text too (`api.md` section 2.5, ADR-0029).
- axum 0.8.9 reads a form body as flat pairs of key and value. The project reads indexed fields from these pairs (ADR-0037 (k)).
- The router applies `http.body_limit.form` (256 KiB) to every route. `http.body_limit.value` exists only as a setting. A form with 16 384 leaves needs 320 to 640 KiB. A value of `budget.value_bytes` (1 MiB of JSON) needs up to 3 MiB in a form body.
- ADR-0001 says that each struct member or array row of the editor is one fragment that HTMX loads. The URL of a parameter holds user data, so an `hx-get` with that URL breaks ADR-0037 (f).
- `ReplaceLimits` checks the current value against the new limits. A narrower limit before a new value therefore fails with `E-RCP-004`.
- `design-system.md` section 10.5 puts the label above the input. A grid of leaves has no room for a label above each input.
- askama 0.16.1 refuses a recursive macro, so a template cannot walk a type tree by recursion.

## Decision drivers

- A user at a machine reads the value and the limit of one leaf side by side. The user wears gloves and uses a touch panel (R39, A44).
- The CSP, the three scripts, and their line limits do not change. Each `hx-*` attribute holds a literal.
- A partial value stays `E-RCP-005` (`api.md` section 2.6).
- No order of two saves can block a valid change of the value and the limits.
- A value inside the budgets of `reliability.md` section 7.11 fits the body limit of its route.

## Considered options

### Option A: One field per leaf, a leaf grid, one save

- Pros: no script. The value and the limits save together, so no save order fails. The grid shows the most leaves on one screen.
- Cons: `design-system.md` sections 10.4, 10.5, and 10.8 need rules for an input in a table.

### Option B: A value form and a limits form

- Pros: each form is short.
- Cons: some valid changes need three saves, because each save checks the value against the limits. The S7 address needs a third form.

### Option C: A disclosure tree with the limits inside each leaf row

- Pros: the rules of `design-system.md` do not change.
- Cons: the page is the longest of the three, and the values do not line up across the levels of the tree.

### Option D: JSON text in a text area

- Pros: no change of `api.md`.
- Cons: a user must write JSON by hand.

## Decision

The project uses **Option A**. The letters below record each point.

Fields of a value:

- (a) The browser sends one field per leaf. The field name is `value` followed by the text of the `LeafPath` (`domain-model.md` section 2.1). Examples: `value[3]`, `value.zone.hold`, and `value` for a parameter with a primitive type.
  - The `id` of the input is `value-<n>`. `n` is the index of the leaf in preorder, from 0.
- (b) `recipe-web` reads the pairs of the form body. A key outside the fields of the route, a malformed leaf path, a `[*]` segment, or a duplicate key gives `422 E-WEB-008`. A well-formed leaf path that the type does not hold gives `E-RCP-005` (point (c)).
  - The reader counts the pairs while it reads. More than five times `budget.leaves_per_parameter` plus 64 pairs give one line `E-WEB-008`. One value field and four limit fields per leaf fit in this count.
  - A leaf path with more segments than `json.max_depth` gives `E-WEB-008` before the parser reads it.
- (c) `recipe-domain` holds two pure functions. The first builds a `ParameterValue` from a `TypeTree` and the texts of its leaves. The second writes the text of each leaf of a `ParameterValue` for a page.
  - The `LeafClass` of a leaf sets its text. `Numeric` takes decimal text with a dot, and `Bool` takes `true` or `false`. `String` and `Char` take the string, and `Duration` and `Instant` take an `IecLiteral`.
  - A missing leaf, a leaf path that the tree does not hold, and a text that does not parse give `E-RCP-005`.
  - The page writes the canonical decimal text of `reliability.md` section 4.4 (A43) and the canonical IEC literal.
- (d) A command builds the value at the place where it parses the JSON text today. For `AddParameter` and `UpdateParameterDefinition`, that place follows the resolution of the type inside the transaction. The work stays in memory, with no I/O and no new lock (`reliability.md` section 7.4).
- (e) `recipe-web` trims the text of a `Numeric`, `Bool`, `Duration`, or `Instant` leaf. It keeps the text of a `String` or `Char` leaf as sent, with no trim and no NFC normalization. This is an exception to `reliability.md` section 4.2.
- (f) `new_member_values` uses the same fields. The name of a field is `new_member_values[<key>]` followed by the leaf path. `<key>` is the `ParameterKey`.
  - This changes the transport of ADR-0029 only. A number keeps the meaning of decimal text (ADR-0018).
- (g) The four routes of `api.md` section 7.6 that carry a value get `http.body_limit.value`. The route that writes a translation gets `http.body_limit.translation`. Every other route keeps `http.body_limit.form`.
  - `http.body_limit.value` changes from 1 MiB to 4 MiB. A value inside `budget.value_bytes` then fits in the fields of its leaves.

Forms of a parameter:

- (h) The form of a new parameter has a second submit button "Show value fields" with `name="fields"` and `value="1"`. A change of the type in the edit form uses the same button. ADR-0037 (h) uses this pattern.
  - With `fields=1`, the server runs the query `GetParameterForm` with the sent `TypeExpr` and writes nothing. It answers `200` with the form, the sent definition fields, and the rows of the leaves of the type.
  - A form that the server renders again keeps every sent text and the sent `row_version`. Only a `GET` of the page gives a new `row_version`. A stale form therefore still gets `409 E-WEB-001` at its save (ADR-0008).
  - The route keeps the operation of its command. In read-only mode it answers `503 E-WEB-006`, as ADR-0037 (h) does for `add=1`.
- (i) The edit form of a parameter holds the definition fields, the S7 address, and the leaf grid. Its `action` is the route `/definition`. One save sends the value and the limits together (ADR-0029).
  - A page holds no second form with a definition field of the same parameter. `row-version.js` refreshes every `row_version` of the page, so a second copy of a field would overwrite a saved change.
  - The routes `/value` and `/limits` keep their fragments of `api.md` section 2.6.
  - The save button is the first submit button of the form, so Enter in an input saves. It stands at the start of the form. The button "Show value fields" and the mode buttons of point (k) follow it.
  - The form holds the hidden field `value_type` with the type of the shown grid. When `type` differs from `value_type`, the server answers as with `fields=1`, shows a notice, and writes nothing.

The leaf grid:

- (j) The leaf grid is one table per parameter. Each leaf has one row with the columns path, value, minimum, maximum, maximum length, and type. A row shows only the limit inputs of the class of its leaf (`domain-model.md` section 2.2).
- (k) Each array level has one of two limit modes.
  - "One limit for all elements": one row `[*]` stands before the elements and holds the limit inputs. An element row shows the limit of `[*]` as text, for example "0 to 3000 (from `[*]`)".
  - "A limit per element": each element row holds its own limit inputs, and no row `[*]` exists.
  - The page shows the second mode when the limits hold an index segment at that level. A line of text names the current mode.
  - One submit button after the row `[*]` of each level switches the mode, as in point (h). Its label names the level. It sends `limit_mode` with the leaf path of the level, for example `.zones[*]`. The server runs `GetParameterForm` and writes nothing.
  - A switch to "a limit per element" copies the limit of `[*]` to each element. A switch back with unequal element limits leaves `[*]` empty and shows a notice.
- (l) An array with more than `form.array_chunk` elements splits into groups of `form.array_chunk` elements (`reliability.md` section 7.11). A group is one `tbody`. Its toggle row stands in its own `tbody` before the group.
  - The toggle is a button of the disclosure pattern of `design-system.md` section 10.8 inside an element `.tree`, so `tree.js` toggles the group.
  - A group is closed, except the first group of an array and each group that holds an error line. An input in a closed group still submits.
- (m) A row header is a `th` with `scope="row"` that holds the leaf path. Each input of the row names the row header and its column header in `aria-labelledby`. This replaces the label above the input of `design-system.md` section 10.5 for a cell of the grid.
- (n) The error line of `design-system.md` section 10.5 stands under the input in its cell. The input has `aria-invalid="true"`, and `aria-describedby` names the error line.

## Consequences

### Positive

- The value editor works with the three scripts, and the CSP does not change.
- One save changes the value and the limits, so no order of saves fails.
- A user sees the value and the limit of a leaf in one row.
- The server checks every leaf with the rules of the domain. The browser holds no rule.

### Negative

- A value with 16 384 leaves gives a page of 5 to 7 MB of HTML.
- A change of the limit mode or of the type needs one more request.
- `api.md`, `reliability.md`, `domain-model.md`, `data-model.md`, and `design-system.md` change in this step.

### Risks and mitigations

- Risk: a panel PC renders a page with 16 384 inputs slowly. Mitigation: closed groups, and the load test of `reliability.md` section 6 measures the page in P4.2.
- Risk: a leaf path in a field name is long. Mitigation: `http.body_limit.value` is 4 MiB, and `budget.leaves_per_parameter` limits the count.
- Risk: a user types a decimal comma. Mitigation: the hint shows an example with a dot, and the error line names the leaf.

## Related

- ADR-0001, amended by this ADR: the editor sends the whole value in one form. It does not load a fragment per struct member or array row. This follows the mitigation of ADR-0001 (one fragment for a whole subtree).
- ADR-0029, amended by this ADR in (f): the transport of `new_member_values`.
- ADR-0036, amended by this ADR in (l), (m), and (n): `design-system.md` sections 10.4, 10.5, and 10.8.
- ADR-0007, ADR-0018, ADR-0022, ADR-0027, ADR-0028, ADR-0037
- `api.md` sections 2.5, 2.6, 7.6, 9
- `reliability.md` sections 4.2, 4.4, 7.11
- `domain-model.md` sections 2.1, 2.2
- `design-system.md` sections 10.4, 10.5, 10.8
