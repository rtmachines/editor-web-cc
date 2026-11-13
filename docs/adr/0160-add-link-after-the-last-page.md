# ADR-0160: Add link after the last page of the parameter table

Status: Accepted. Amends ADR-0043 (i) (the link "Add a parameter" on a page after the last page)
Date: 2026-11-12
Deciders: project owner

## Context

The UI and UX audit of 2026-11-12 (finding F2) reports a fault of the form of a new parameter. On a page after the last page, the form has no target. ADR-0043 (i) loads that form into `#parameter-form` on every page.

- `ParameterTable::after_end` is true when a page has no rows and its index is above 0. An example is `?page=2` of a draft with one page of parameters.
- On such a page, `parameter-table.html` renders no `#parameter-rows`.
- The form in `#parameter-form` appends the row of the new parameter to `#parameter-rows`. HTMX then reports `htmx:targetError`, sends no request, and shows no message.
- An old address in a bookmark or in the browser history can open such a page.

## Decision drivers

- Each action on a page works, or the page tells the user why it does not work (R39).
- Each `hx-*` attribute of a page template holds a literal (`api.md` section 9, ADR-0037).
- The usual pages of the parameter table keep the fragment form of ADR-0043 (i).
- The first page of a draft without parameters keeps its link and its form.

## Considered options

### Option A: A second literal form of the link targets `#content`

On a page after the last page, the link loads the page form of a new parameter into `#content`. The save then answers `303` to the version page.

### Option B: Always render an empty `#parameter-rows`

The fragment form then has a target. A new row stands alone on a page after the last page, and the pager text contradicts it.

### Option C: Hide the link after the last page

The user then has no action and gets no reason for its absence.

## Decision

Option A.

- (a) `version.html` renders the link "Add a parameter" in one of two literal forms. `VersionPage` takes the flag `after_end` from `ParameterTable::after_end`.
- (b) A page with `#parameter-rows` keeps the link of ADR-0043 (i): `hx-target="#parameter-form"`, `hx-swap="innerHTML"`, and `hx-push-url="false"`.
- (c) A page after the last page gets the link with `hx-boost="true"`, `hx-target="#content"`, and `hx-push-url="true"`.
- (d) `parameters/new.rs` answers each request without `HX-Target: parameter-form` with the page form (`Place::Page`). The page form targets `#content`.
- (e) A save of the page form gets `303` to the version page. The boost follows the redirect and shows the version page with the new parameter.
- (f) The link of item (c) pushes its address. The browser button Back therefore returns to the parameter list.
- (g) Each `hx-*` attribute of both forms of the link stays a literal. The `href` holds the address of the form (`api.md` section 9, ADR-0037).
- (h) The first page of a draft without parameters has `after_end` false, because `after_end` needs a page index above 0. That page keeps the link into `#parameter-form`. A guard test holds this rule.

This decision amends ADR-0043 (i) for a page after the last page only. Every other page keeps ADR-0043 (i) without a change.

## Consequences

### Positive

- The link "Add a parameter" works on each page of the parameter table.
- An old page address in a bookmark gives a form that saves.
- No `hx-*` attribute holds user data.

### Negative

- The version page has two forms of one link. A template change must keep both forms in step.
- On a page after the last page, the form replaces the whole content and not one region.

### Risks and mitigations

- [A change of the template drops one form of the link] → A web test asserts each form on its page. An E2E test adds a parameter from a page after the last page.
- [The first page of an empty draft gets the page form] → The guard test of item (h) asserts the link into `#parameter-form`.

## Related

- ADR-0037 (literal `hx-*` attributes, page forms that target `#content`)
- ADR-0043 (i) (the link "Add a parameter" loads the form into `#parameter-form`)
- Change `ui-form-fixes` (design D1, audit finding F2)
