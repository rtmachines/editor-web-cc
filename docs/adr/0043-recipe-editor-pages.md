# ADR-0043: Pages of the recipe editor

Status: Accepted. Amended by ADR-0044 (bound of the rebind forms)
Date: 2026-10-15
Deciders: project owner

## Context

Step P3.3 adds the pages of `api.md` sections 2.4 to 2.6. It leaves out the removal and the restore of section 2.4. It also leaves out the approval, the retirement, the discard, and the diff of section 2.5. ADR-0039 to ADR-0042 record the form of a value, the POST routes, the error lists, and the snapshot reads. The documents leave these points open or disagree:

- `GET /recipes` has the fields `q`, `machine`, `status`, `page`, and `show_deleted`. `reliability.md` section 7.11 has no page size for it, and no document says what `q` and `status` match.
- A recipe can have hundreds of versions. `GET /recipes/{ref}` and `GET /recipes/{ref}/versions` have no page.
- No document says who can read the pages of a deleted recipe. ADR-0021 permits `show_deleted=1` only to the Administrator. `api.md` section 2.7 answers `409 E-RCP-012` for the export of a version of a deleted recipe.
- ADR-0012 says that the UI offers a fixed list of language tags. No document gives the list.
- `CreateVersion` of `recipe-app` returns `404 E-WEB-005` for a `based_on` that names no version. `api.md` section 1 and `reliability.md` section 1.2 give `E-WEB-005` only to a URL value.
- `api.md` section 4 gives the content of `conflict.html`. It shows the server `row_version`, the login of the last editor, and the time of the last change. Its button "Reload version" uses `hx-get`. `AppError::ConcurrentEdit` holds no login and no time. An `hx-get` with the URL of a version breaks ADR-0037 (f).
- The step plan puts `RebindTypeRevision` in no step.

## Decision drivers

- Each list has a page size from `reliability.md` section 7.11, as ADR-0037 (l) gives for the machines.
- A deleted recipe stays out of view for the roles that cannot restore it (ADR-0021).
- Each `hx-*` attribute holds a literal (ADR-0037 (f)).
- A draft with outdated parameters can be fixed on the pages of this step (ADR-0018).

## Considered options

### Option A: Decide each point in this ADR, as ADR-0037 does

- Pros: one record for the open points of the recipe pages.
- Cons: the ADR holds many small points.

### Option B: One ADR per point

- Pros: each point has its own record.
- Cons: eleven small ADRs that refer to each other.

## Decision

The project uses **Option A**. The letters below record each point.

Lists:

- (a) `GET /recipes` shows `page.recipes` rows per page. `GET /recipes/{ref}` and `GET /recipes/{ref}/versions` take `page` and show `page.versions` rows per page (`reliability.md` section 7.11). `page` in the URL starts at 1, and the query counts pages from 0 (ADR-0037 (l)).
- (b) `q` of `GET /recipes` is a `Text`. It matches a part of the reference number or of the name, with no difference between upper and lower case.
  - `machine` is a `MachineCode` and follows the rule of the path value `{code}` (`api.md` section 1).
  - `status` is `draft`, `approved`, or `retired`. It keeps a recipe with a version in that state. Another value gives `422 E-WEB-008`.
  - The rows come in the byte order of the reference number.
- (c) A row of the recipe list shows the reference number, the name, and the machine code. It also shows the number and the state of the `Approved` version and of the live `Draft`.
- (d) `show_deleted=1` runs the query `ListDeletedRecipes` with the permission `RecipeRestore` (`api.md` section 3.3). Without the permission, the answer is `403 E-WEB-002`.
  - The list then also shows the deleted recipes with the state "Deleted". The state has the word key `status-deleted`, the tokens of `Retired`, and the icon `status-retired` (`design-system.md` section 9).

Deleted recipe:

- (e) An Administrator reads the recipe page and the version pages of a deleted recipe. The pages show the state "Deleted" and hold no form.
  - Every other role gets `409 E-RCP-012`, as the export of `api.md` section 2.7 does.

Forms:

- (f) `i18n.translation_langs` holds the fixed list of language tags of ADR-0012 (`reliability.md` section 7.11). The default is `pl`.
  - The parameter page shows one translation row per tag of the list, and one row per other tag that the parameter holds. A row holds the English base text, the name, and the description.
  - Each row is one form. Its `action` holds the tag (ADR-0040 (a)), so the form needs no select.
  - A tag outside the list gives `422 E-WEB-008` on the route that writes a translation. The domain still checks the `LangCode`.
  - The route that deletes a translation accepts every tag that the parameter holds, also a tag outside the list.
- (g) The S7 address field follows A31. The field is required on `S7_300` and optional on `S7_1200` and `S7_1500`. `CodesysIec` and `TwinCat3` show no field.
- (h) The form of a new version offers a select of the version numbers of the recipe and the choice "empty draft". The default is the `Approved` version, else the newest version, else "empty draft".
  - A `based_on` that names no version gives `422 E-WEB-008` with the field `based_on`. `recipe-web` maps the result of `CreateVersion` to that field. `api.md` section 2.5 states the rule.
- (i) The link "Add a parameter" of the version page loads the form of a new parameter into `#parameter-form`. The link "Add a sub-unit" of ADR-0037 (c) works in the same way.
  - `AddParameter` answers the row of the new parameter for the parameter table (`api.md` section 2.6). The same answer clears `#parameter-form` and `#error-panel` out of band.

Pages of a version and of a parameter:

- (j) The version page shows the header of the version and the parameter table with `page.parameters` rows. The rows are grouped by unit and come in the order unit path, ordinal, and tag.
  - A draft with more than `page.parameters` parameters shows one unit per page by default (`api.md` section 2.6). The page links to each unit. Each link names the unit path and the count of its parameters. The link of the shown unit has `aria-current="page"`.
  - A draft with outdated parameters shows them in a list with the rebind form of point (l).
  - An outdated parameter shows the state "Outdated" with the word key `status-outdated`, the tokens of the feedback "Warning", and the icon `alert-warning` (`design-system.md` section 9).
- (k) The parameter page shows the definition, the value and the limits in the leaf grid of ADR-0039, and the translations. Only a live `Draft` and a user with `ParameterEdit` get the forms.

Rebind:

- (l) `RebindTypeRevision` is part of step P3.3. The draft page shows one rebind form per outdated struct revision and its newest revision.
  - The form holds the two type identifiers and one field per new member leaf of each affected parameter (ADR-0039 (f)).
  - `RebindNeedsValue` and `RebindDropsLimit` show one line per entry (ADR-0027, ADR-0041).

Conflict:

- (m) `conflict.html` shows the `row_version` of the server, the login of the last editor, and the time of the last change. `GetVersion` returns these values with the permission `RecipeView`, as `GetRecipeHistory` shows the actors of a recipe (`api.md` section 2.9).
  - The time is `updated_at` of the version. The last editor is the actor of the audit entry of the recipe with `occurred_at` equal to that `updated_at`. One transaction writes both values with one `now()` (`data-model.md` section 1).
- (n) The button "Reload version" is a link with `href` to the version page and `hx-boost="true"` (ADR-0037 (f)). The unsaved input stays in the form until the user follows the link. The fragment says that the reload discards the unsaved input.

## Consequences

### Positive

- Every list of the recipe pages has a page size and an order.
- A deleted recipe stays visible only to the role that can restore it.
- A user can fix an outdated draft before step P3.4 adds the approval.

### Negative

- `api.md`, `reliability.md`, and `design-system.md` change in this step.
- The page of an outdated draft is longer, because it holds the rebind forms.

### Risks and mitigations

- Risk: two audit entries of a recipe share one `occurred_at`. Mitigation: the query takes the entry with the highest `id`, and only one transaction writes both values.
- Risk: the list of language tags is too short for a plant. Mitigation: the setting changes the list without a code change.

## Related

- ADR-0008, ADR-0012, ADR-0018, ADR-0021, ADR-0027, ADR-0037, ADR-0039, ADR-0040, ADR-0041, ADR-0042
- `api.md` sections 1, 2.4, 2.5, 2.6, 2.7, 2.9, 3.3, 4
- `reliability.md` sections 1.2, 7.11
- `design-system.md` section 9
- `overview.md` section 9: A17, A31
