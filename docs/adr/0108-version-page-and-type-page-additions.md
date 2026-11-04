# ADR-0108: Additions to the version page and the type page

Status: Accepted. Amended by ADR-0118 (item (a): a stored value that breaks a newer rule shows with a field line). Amended by ADR-0119 (item (b): the version page of an `Approved` version reads every value). Amended by ADR-0121 (items (a) and (b): the parameter table reads the value of every row of one page, including an array or a struct)
Date: 2026-11-01
Deciders: project owner

## Context

Release 0.3.0 lists four gaps of the web UI in its section "Known gaps":

- The save toast of a parameter does not show the new row version.
- The version page has no picker for two free version numbers. It links only to the diff with the approved version and with the base version (ADR-0050 (l)).
- The version page does not show the content hash. The export file holds it as `content_sha256` (`api.md` section 6).
- The type page does not show the S7 layout of a type.

The project owner found a fifth gap. The parameter table of the version page shows no value (ADR-0043 (j)). A value can have 1 MiB, so the table read loads no value. A user must open each parameter page to read one value.

The domain computes the size and the start class of a type tree on S7 (`S7Layout`, ADR-0017). It does not keep the offset of each member.

## Decision

This is the decision of the user of 2026-11-01. It amends ADR-0043 (e) and (j) and ADR-0050 (l).

Value column of the parameter table:

- (a) The parameter table of the version page has the column Value after the column Type.
  - A parameter of a primitive type shows the text of its one leaf. The text is the text of the leaf grid of the parameter page (ADR-0039).
  - The cell shows at most `table.value_chars` characters (`reliability.md` section 7.11). A longer text ends with `…`. The parameter page shows the full text.
  - An empty text shows the word of `parameters-value-empty` in `--color-text-muted`.
  - An array or a struct shows the word of `parameters-value-composite` in `--color-text-muted`. The table read loads no value of an array or a struct.
  - A stored value that does not match its type tree is a defect of the storage. The read gives `ValueFault`. The use case `ListParameters` reports it as a storage defect with `500 E-DB-008`, as the diff pages do.
- (b) The new port method `parameter_values(version, keys)` reads the values of the keys of one page only. Only `ListParameters` calls it, with the keys of the primitive rows.
  - The method returns at most `page.parameters` rows. A primitive value holds at most one `WSTRING` of 16 382 UTF-16 units at up to 3 bytes each. One page therefore stays below 5 MiB.
  - `parameter_page` and its statement stay free of values. `GetVersion` loads no value, because it calls `parameter_page` for a draft with no row limit.

Content hash:

- (c) The header of an `Approved` or a `Retired` version shows `content_sha256` in `--font-mono`. The text breaks at any character, so the page has no horizontal scroll at 320 px. A `Draft` has no hash and shows no row.

Compare form and route:

- (d) The version page of a recipe with two or more versions shows a compare form after the row of links of ADR-0050 (l). The two links of ADR-0050 (l) stay.
  - The form stands after the row of links, outside `<p class="actions">`. It has `method="get"`, `action="/recipes/{ref}/compare"`, and the literals `hx-boost="true" hx-target="#content" hx-push-url="true"`, as the filter form of the audit page. It sends no `csrf_token` (ADR-0037).
  - An Administrator on a deleted recipe keeps the form, as for the links of ADR-0050 (l), because the form only reads.
  - The lists `n` and `m` hold each version number of the recipe, the highest first. `n` selects the shown version. `m` selects the base version when it exists and differs, else the highest other number.
- (e) `GET /recipes/{ref}/compare` parses `n` and `m` as version numbers and answers `303` to `/recipes/{ref}/versions/{max(n,m)}/diff/{min(n,m)}`. This is the direction of the links of ADR-0050 (l).
  - The route runs no use case, reads no data, and writes nothing. The diff route runs `DiffVersions` and checks the permission `RecipeView`, the versions, and a deleted recipe (ADR-0050).
  - A missing key, an unknown key, or a bad number gives `422 E-WEB-008`. A bad `{ref}` gives the code of `api.md` section 1.
  - A boosted request follows the `303`. HTMX pushes the address of the diff page.

Toast of a draft command:

- (f) Each success toast of a command after which the draft stays a live draft adds the sentence of `toast-change-number`. The sentence names the new `row_version` with the word "change number" of `conflict-row-version`. The number is equal to `rowVersion` in `HX-Trigger` of the same answer (ADR-0008).
  - `SetParameterValue` and `ReplaceLimits` get the new toasts `value-saved` and `limits-saved`. With the six existing toasts, eight toasts name the change number.
  - `ApproveVersion` is not in scope, because the draft does not stay a live draft after it.

S7 layout:

- (g) The domain gets the function `s7_members` next to `S7Layout`. It returns one row per struct member in the order of the tree.
  - The placement step `place(cursor, &S7Layout)` is a shared function that `struct_layout` also calls. `S7Layout::of` and `s7_members` both use it, so the S7 rules exist once.
  - A row holds the member names from the root as `Vec<MemberName>`. It also holds the type id with its S7 layout and the offset as a byte and a bit. A method gives the path as text, such as `.zone.active`. An array is one row. Its elements have no row.
  - A unit test takes the last top-level row and adds its width. The width is 1 bit for `BOOL`, else `size_bytes` × 8. The test rounds the result up to 16 bits. The result must equal `S7Layout::size_bytes` × 8.
- (h) The type page of a machine on `S7_300`, `S7_1200`, or `S7_1500` shows the table S7 layout after the type tree. The columns are path, type, and offset as `<byte>.<bit>`. A sentence after the table gives the size in bytes.
  - A sentence before the table states that the offsets apply to a data block with standard access.
  - A machine on `CodesysIec` or `TwinCat3` shows no S7 layout.

## Consequences

- A user reads the value of each primitive parameter on the version page. No new query and no new permission are necessary.
- The list of parameters reads more bytes. Point (b) keeps an upper bound for one page. `GetVersion` reads no value.
- An operator compares two versions and the exported file by the content hash on the version page.
- The compare route runs no use case. It reads no data, so it cannot show data to a user without the permission.
- The S7 layout uses the rules of ADR-0017. The reference table of the Siemens manual is not in the repository yet.
- The address checks of I20 use the same `S7Layout`. A wrong S7 rule is therefore wrong in both places, and neither place detects it. The project accepts this risk.
- `reliability.md` section 7.11 gets the row `table.value_chars`.

## Related

- Amends ADR-0043 (e) (the pages of a deleted recipe hold no form that writes). Amends ADR-0043 (j) (the columns of the parameter table) and ADR-0050 (l) (the compare links of the version page).
- ADR-0008 (`row_version`), ADR-0017 (S7 layout), ADR-0037 (user data in a URL), ADR-0039 (leaf grid), ADR-0050 (diff of two versions).
- `api.md` sections 2.3, 2.5, 2.6, and 6. `design-system.md` section 10.7. `reliability.md` section 7.11.
