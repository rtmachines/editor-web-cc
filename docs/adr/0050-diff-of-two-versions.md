# ADR-0050: Diff of two versions

Status: Accepted
Date: 2026-10-16
Deciders: project owner

## Context

`api.md` section 2.5 has the route `GET /recipes/{ref}/versions/{n}/diff/{m}` with the query `DiffVersions`. The approval preview `GetApprovalPreview` shows the diff of the draft against the current Approved version, with the limit changes highlighted (ADR-0022). No code computes a diff of two versions. `recipe-app/src/audit/diff` builds only the state diff of one audit entry.

The documents do not say these points:

- which parts of a parameter the diff compares,
- where the diff runs,
- how it groups the changes,
- how it bounds its size.

A version can hold 4 000 parameters, 262 144 leaves, and values of up to 1 MiB each (`reliability.md` section 7.11). The `app` container has 512 MiB of memory. Every role has `RecipeView`, so every role can ask for a diff.

ADR-0042 (c) puts every query except six draft queries into `Isolation::ReadOnly`. The preview renders the `row_version` of the draft into the approve form. It reads the data from several statements, so it has the race that ADR-0042 describes.

The design shotgun of step P3.4 drew three layouts: a table per parameter, the two versions side by side, and the changes grouped by kind. The owner chose the grouping by kind.

## Decision drivers

- A second person sees each changed limit in one place before the approval (ADR-0022, R27).
- The diff never holds a whole version of 4 000 large values in memory for one page (R30, A26).
- The approve form and the shown diff come from one state of the database (ADR-0042, ADR-0008).
- A value shows as canonical decimal text and a time as an IEC literal (A43, ADR-0039).
- One rule lives in one place. The domain owns the value texts and the leaf paths.

## Considered options

### Option A: A pure diff in `recipe-domain`, pages per kind, SQL change flags

- Pros: the diff reuses `ParameterValue::texts` and the leaf paths of the domain. SQL finds the changed parameters without a load of the values. Only the pairs of one page load.
- Cons: a new module in `recipe-domain` and two queries in `REPEATABLE READ`.

### Option B: The diff in `recipe-app`

- Pros: no change in `recipe-domain`.
- Cons: `recipe-app` must copy private domain functions such as `LeafPath::concrete_paths`.

### Option C: Load both versions and page the computed diff

- Pros: the simplest code.
- Cons: each page request loads two whole versions. A worst case does not fit into the container.

For the layout, the options were a table per parameter, the two versions side by side in a disclosure, and the changes grouped by kind.

## Decision

The project uses **Option A** with the grouping by kind.

- (a) `recipe-domain` gets the module `recipe/diff`. Its pure function takes the parameters of the old and the new side of one page and the type trees. It returns the diff rows. It reads no port and knows no page.
- (b) The diff matches the parameters by `ParameterKey` (A40). A key only on the new side gives an added parameter. A key only on the old side gives a removed parameter. A key on both sides gives a changed parameter when one of its parts differs. The pages leave out a parameter with no difference.
- (c) The diff compares these parts:
  - the definition: tag, unit path, type text, S7 address, ordinal, `name_en`, and `description_en`,
  - the value per leaf, with the texts of `ParameterValue::texts` (ADR-0039): canonical decimal text, IEC literal for a duration or an instant,
  - the limit rows by their stored leaf path, with `min`, `max`, and `max_length`. A `[*]` row is one row. A change of the limit mode shows as a removed row and an added row,
  - the translations per language tag, with the name and the description.
  When the type changes, the diff shows every value leaf and every limit row of both sides. This includes a leaf or a row with an equal text. A new type can give a stored bound a new meaning, for example `TIME` to `LTIME`. A leaf or a row on one side only shows with an empty other side.
  The diff checks each stored value and each stored limit row of both sides against its type tree first. Every write path checks them too, so a row that does not fit is corrupt data. The diff then stops with a `StorageError` of the code `500 E-DB-008`, and shows no raw stored text. That error keeps the fault of the domain as its source, so the log line names the parameter (`reliability.md` section 1.1).
- (d) The pages group the rows by kind in this order: limits, values, definitions, translations, added parameters, removed parameters.
  - The limits section holds every changed limit row. This includes the rows of an added parameter (empty old side) and of a removed parameter (empty new side). The section has a frame in `--color-error-edge`.
  - Each value row also shows the limit of the new version for that leaf.
  - The added and the removed sections show one row per parameter: tag, `name_en`, unit path, type, and the value of a scalar. A structured value shows the count of its leaves and a link to the parameter page.
  - Inside a section, the rows come in the order of the parameter table (unit path, ordinal, tag), then in the preorder of the leaves.
  - A summary at the top gives these counts. Each count links to its section.
    - the parameters of each section,
    - the rows of the limits, the definitions, and the translations,
    - the parameters without a change.
- (e) One page loads and shows at most `page.diff` parameters over all its sections. A section shows at most `diff.leaf_rows` rows of one parameter. The rest shows as a count with links to the parameter pages of both versions.
  - The query keys are `kind` (`limits`, `values`, `definitions`, `translations`, `added`, `removed`) and `page`. Without `kind`, the page shows the summary, and the sections fill the page in their order up to the bound of `page.diff`. Each section that the bound cuts links to its own pages with `kind`.
  - With `kind`, the page shows the summary and the requested page of that section, with the page pager of `design-system.md` section 10.12.
  - Another value of `kind` gives `422 E-WEB-008`. A page after the last page shows the text of `list-page-after-end`.
  - No page with a diff shows an input `kind`. So the bad value gives the variant `WebError::BadQuery { field, reason }`. It has the code, the label, and the reason of `BadRequest`, but its line has no link. This amends ADR-0041 (d).
- (f) `DiffVersions` and `GetApprovalPreview` read in one `Isolation::RepeatableRead` transaction. This amends ADR-0042 (b) and (c).
  - In that snapshot, SQL compares the two versions per key and gives the change flags of each part and the counts of the summary. Then it reads the pairs of the requested page only.
  - The transaction ends before the diff runs (ADR-0042 (f)).
  - In a `RepeatableRead` transaction, the in-memory adapter reads the draft from the state of the transaction start, as PostgreSQL does. A test can then run a draft command between two reads of the preview. This amends ADR-0042 (e).
- (g) At most `diff.permits` diffs run at the same time. Each of the two queries takes the permit before the `RepeatableRead` transaction of point (f) and holds it to the end of the render. The result carries the permit, and `recipe-web` drops it after the page renders.
  - `AppContext::new` builds the semaphore from the settings. Every clone of the context shares it.
  - The diff runs in `spawn_blocking`. The wait for a permit and the run in `spawn_blocking` share the limit `timeout.validation`. A timeout gives `503 E-WEB-009`. A failed task gives `500 E-WEB-011` with the stage of the diff.
  - `db.statement_timeout` bounds each read of the transaction (`reliability.md` section 7.4).
- (h) The preview carries the row version of its first page. Every link of the preview to another page adds the query key `rv` with that value.
  - The approve form of each preview page sends `rv` as `row_version`.
  - A later page can find another `row_version` in its snapshot. Then the page shows a notice that the draft changed and a link to the first page. It shows no approve form. The approver therefore never approves a state that the review did not start with.
  - A page without `rv` uses the `row_version` of its own snapshot.
- (i) In `GET .../versions/{n}/diff/{m}`, `m` is the old side and `n` the new side. The preview compares the draft (new side) with the current Approved version (old side). Without an Approved version, the diff gives every parameter of the draft as added. Each of their limit rows stands in the limits section. A diff of a version with itself shows the empty state.
- (j) For a recipe with `deleted_at` set, the diff route follows `api.md` section 2.4. The Administrator reads it, and every other role gets `409 E-RCP-012`. The preview of such a recipe gives `409 E-RCP-012` to every role with `VersionApprove` (ADR-0049 (d)).
- (k) `design-system.md` gets the states and icons of the diff (sections 8 and 9):
  - "Limit changed": the tokens `--color-error-*` and the new icon `diff-limit`,
  - "Value changed", "Definition changed", and "Translation changed": the tokens `--color-info-*` and the new icon `diff-changed`,
  - "Added": the tokens `--color-success-*` and the new icon `diff-added`,
  - "Removed": the tokens of `Retired` and the new icon `diff-removed`.
  `alert-warning` keeps its meaning "Outdated". Each state shows the color, the icon, and the word.
- (l) The version page links to the diff with the current Approved version and with the base version (ADR-0049 (i)). A link shows only when that version exists and differs from the shown version.
- (m) `reliability.md` section 7.11 gets the rows `page.diff` (50 parameters of one page, over all its sections), `diff.leaf_rows` (1 000 rows of one parameter), and `diff.permits` (2).

## Consequences

### Positive

- Every changed limit stands in one table, also the limits of added and removed parameters.
- A page loads only its own pairs. The memory of one diff request has a bound.
- The approve form and the shown rows come from one snapshot.

### Negative

- One parameter can appear in several sections.
- The SQL of the change flags compares the values as JSONB in the database. The statement reads every value of both versions inside PostgreSQL. It sends no value to the application.
- The preview still reads the whole draft for the checks of ADR-0049 (c).

### Risks and mitigations

- Risk: a page of 50 large parameters needs much memory. Mitigation: `diff.permits` bounds the parallel diffs from the first read to the end of the render. `page.diff` bounds the parameters of one page, and `diff.leaf_rows` bounds the rows of one parameter. The load test of step P4.2 measures the worst case.
- Risk: the SQL of the change flags and the full draft load of the preview read much data. Mitigation: the permit of point (g) and `db.statement_timeout`. The load test of step P4.2 measures both on a version of the budgets of A26.
- Risk: the SQL flags and the domain diff disagree. Mitigation: a contract test runs the same cases on the memory adapter and on PostgreSQL. It compares the flags with the rows of the domain diff.
- Risk: the draft changes during a review. Mitigation: point (h).

## Related

- ADR-0007, ADR-0008, ADR-0018, ADR-0022, ADR-0039, ADR-0042 (amended by this ADR in (b), (c), and (e)), ADR-0049, ADR-0051
- `docs/architecture/api.md` sections 2.5 and 4
- `docs/architecture/reliability.md` sections 7.4 and 7.11
- `docs/architecture/design-system.md` sections 8, 9, 10.4, and 10.12
- `overview.md` section 7: R19, R27, R30, R31. Section 9: A26, A40, A43
