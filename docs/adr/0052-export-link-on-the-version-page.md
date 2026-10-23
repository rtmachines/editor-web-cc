# ADR-0052: Export link on the version page

Status: Accepted
Date: 2026-10-22
Deciders: project owner

## Context

Step P3.5 adds the two export routes of `api.md` section 2.7. A user needs a control in the UI that starts an export. ADR-0049 (i) lists the items of the action area of the version page. The list has no export item.

The answer of an export route is a JSON file with `Content-Disposition: attachment`. The page must not show it as HTML. These facts constrain the control:

- The vendored HTMX 2.0.11 boosts each same-host anchor whose `target` is empty or `_self`. It has no exception for the attribute `download`.
- The row of links of the version page has `hx-boost="true"`. A boosted export link would swap the JSON into `#content`.
- `api.md` section 5.3 gives the export routes JSON errors. With the attribute `download`, a browser can hide an error body, and the user then loses the code and the `request_id`.
- The icon list of `design-system.md` section 8 has no export icon.

## Decision drivers

- Each role sees only the export that the matrix of `api.md` section 3.3 allows (R24, R28).
- Every action gives a visible result or an error with its code (R36, R39).
- No new script and no new icon (`api.md` section 9, `design-system.md` section 8).
- Each target is at least 44 by 44 CSS px (A44).

## Considered options

### Option A: A plain link in its own row after the header of the version

- Pros: no HTMX request. A success saves the file. An error shows the JSON body with the code and the `request_id` in the same tab.
- Cons: one more row on the page.

### Option B: The same link inside the row of links with `hx-boost="false"`

- Pros: one row of links.
- Cons: the row mixes page links and a download. One missing attribute swaps JSON into the page.

### Option C: A link with the attribute `download`

- Pros: the browser never leaves the page.
- Cons: the browser can show a failed download without the code and the `request_id`.

### Option D: A link in each row of the version list too

- Pros: an export without the version page.
- Cons: more targets in a table. A table cell has no room for the notice of a retired version.

## Decision

The project uses **Option A**.

- (a) Only the version page shows the export link. The version list shows none.
- (b) The page of an `Approved` version shows the link to `/export/v1/recipes/{ref}/approved` when the matrix row of `ExportApprovedVersion` allows the role. The page of a `Retired` version shows the link to `/export/v1/recipes/{ref}/versions/{n}` when the row of `ExportVersionByNumber` allows the role. A `Draft`, a discarded draft, and each version of a deleted recipe show no link. The link stays in read-only mode (A24).
- (c) The link is `<a class="btn btn-secondary">` in its own `<p class="actions">` after the header `dl`. Only the notice of point (e) can stand between the `dl` and this `p`. No ancestor has `hx-boost="true"`. The anchor also carries the literal `hx-boost="false"`. It has no `download` and no `target` attribute. It holds no icon.
- (d) The visible text names the version and the file type. It is also the accessible name, so the link has no `aria-label`.
- (e) A `Retired` version gets an info notice in `<div id="export-notice">` directly before the `<p class="actions">` of the link, as `components/danger.html` does. A `div` never stands inside a `p`. The notice says that the file name starts with `RETIRED_` and that the file marks the version as not loadable. The link names the notice in `aria-describedby`. A page holds at most one export link, so the id is unique. The link of an `Approved` version has no `aria-describedby`.
- (f) The page shows no toast after an export. The browser shows the download. An error answer shows the JSON body in the tab, and the user goes back with the browser.
- (g) This ADR amends ADR-0049 (i): the action area of the version page also holds the export link of point (b). The export link is not a danger action.

## Consequences

### Positive

- An error keeps its code and its `request_id` on the screen (R36).
- The link uses an existing component and the existing rules for the target size, the focus, and the gap.
- No template writes user data into an `hx-*` attribute.

### Negative

- After an error, the user leaves the page and goes back with the browser.
- The version page has one more row.

### Risks and mitigations

- Risk: a later change moves the link into a boosted container. HTMX then swaps the document into `#content` as HTML. A stored text with `<` becomes live markup that inherits the CSRF header of `<body>`. Mitigation: a page sweep checks that each `/export/` anchor of each page has `hx-boost="false"` and no boosted ancestor. The guard of ADR-0055 also refuses each request with `HX-Request: true`.
- Risk: another approval between the page load and the click gives a newer version on `/approved`. Mitigation: the file name and `version.number` name the version, and the file stays loadable with a matching hash.

## Related

- ADR-0019, ADR-0036, ADR-0037 (f), ADR-0043 (e), ADR-0053, ADR-0055
- ADR-0049, amended by this ADR in (g): the export link joins the action area of point (i)
- `docs/architecture/api.md` sections 2.7, 3.3, 5.3, 6.3, 9
- `docs/architecture/design-system.md` sections 7, 8, 10
- `overview.md` section 7: R24, R28, R36, R39. Section 9: A4, A24, A44
