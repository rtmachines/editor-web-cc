# Design

## Context

The audit of 2026-11-12 found F2, F4, and F5 with a browser test of an older build. A probe on `main` at `774c409` found each defect again with the release build of `examples/dev_server.rs`.

- F2: `GET /recipes/{ref}/versions/{n}?page=2` of a draft with 3 parameters renders the after-end text of `parameter-table.html` and no `#parameter-rows`. The link Add a parameter of `version.html` still loads the fragment form into `#parameter-form`. That form has `hx-target="#parameter-rows"`, so HTMX raises `htmx:targetError` and sends no `POST`.
- F4: `udt-form.html` puts `udt-add-member` (`name="add"`) before `udt-define-submit` in `.actions`. Enter sends the first submit button of the form, so Enter sends `add=1`. With `at_max`, the template drops `udt-add-member`, and Enter creates the type. ADR-0037 (h) names "Add a member" the second submit button, so the template breaks that point.
- F4, revision: `next_revision` of `crates/recipe-app/src/plant/define_udt.rs` gives the next number and does not compare the members. A revision form of `?from=` holds the members of the latest revision. After the fix of the button order, Enter in that form writes a revision with the same members.
- F5: `FormView::new` of `src/pages/units/form.rs` builds the `h2` and the proposed segment from the parent of the request. The parent `select` lets the user pick another parent. Nothing renders the `h2` and the segment again.
- F5, errors: `units/add.rs` names the input `parent_path` for a malformed parent (line 68) and for `ParentNotFound` and `UnitArchived` (line 134). The summary macro links each line that has a target.

Rules that apply: ADR-0037 (h), ADR-0039 (the save button is the first submit button), and ADR-0041 (field errors and the summary). More rules: ADR-0018 (a revision marks parameters as outdated), `api.md` section 9 (no user data in an `hx-*` attribute, the three scripts), and `design-system.md` section 10.

## Goals / Non-Goals

**Goals:**

- Each form sends its request to a region that the page holds.
- Enter in the UDT form has one meaning at each count of rows.
- Enter in an unedited revision form writes nothing.
- The unit form shows one parent in the `h2`, in a text line, in the proposed segment, and in the sent field.

**Non-Goals:**

- No new script, no new route, no new form field, and no change of the database schema.
- No change of `recipe-db`.
- F1 and F3. The change `ui-request-state` takes them.

## Decisions

### D1. Add link after the last page (F2)

`version.html` renders the link Add a parameter in one of two literal forms. `VersionPage` gets a flag from `ParameterTable::after_end`. ADR-0160 records this decision and amends ADR-0043 (i).

- A page with `#parameter-rows` keeps the link with `hx-target="#parameter-form"`, `hx-swap="innerHTML"`, and `hx-push-url="false"`.
- A page after the last page gets `hx-boost="true"`, `hx-target="#content"`, and `hx-push-url="true"`. `parameters/new.rs` answers each request without `HX-Target: parameter-form` with the page form (`Place::Page`). That form targets `#content` and gets `303` to the version page after a save.
- The first page of a draft without parameters has `after_end` false (`parameter_table.rs:270` needs `page.index > 0`). It keeps the first link. A guard test holds that.

Alternative: always render an empty `#parameter-rows`. A new row would then stand alone on a page after the last page, and the pager text would contradict it. Rejected.

Alternative: hide the link after the last page. The user then has no action and no reason. Rejected.

### D2. Order of the UDT buttons (F4)

`udt-form.html` puts `udt-define-submit` first in `.actions` and `udt-add-member` (or the notice of `at_max`) after it. The DOM order is the visual order and the Tab order, so `design-system.md` keeps one focus order. Enter then creates the type or the revision at 1, 1023, and 1024 rows. That is the rule of ADR-0037 (h) and the same rule as ADR-0039 for the save button.

Alternative: a hidden first submit button with `name="add"`. A hidden control breaks the focus order and needs a style rule. Rejected.

Browser tests press Enter in the form of a new type and in a revision form, each at 1, 1023, and 1024 rows. A test builds the large forms with `add=1` requests through `page.request` or fills the rows from a page that the server rendered. Each test counts the `POST` requests. It asserts exactly one `POST` without `add` and the type page of the expected revision. The revision tests change one member type first, so ADR-0159 does not refuse them.

### D3. Fixed parent of the unit form (F5)

`unit-form.html` replaces `fields::select(... "parent_path" ...)` with `<input type="hidden" name="parent_path" value="...">`. Before the segment input, the form shows a text line with the label `field-parent-path` and the parent path. For a main unit, the line shows the text of `unit-parent-none`. `FormView` drops `parents` and the type `ParentOption`. It keeps `selected` as the value of the hidden field and of the text line.

- `GET /machines/{code}/units/new?parent_path=U1` gives the parent `U1`. `GET /machines/{code}/units/new` gives an empty value and a new main unit.
- The form of an error answer (`Place::Refused`) keeps the sent parent in the hidden field and in the text line. A plain `POST` without HTMX gets the same form, so its page shows the parent too.
- `units/add.rs` passes the target `None` for a malformed parent (line 68) and for `ParentNotFound` and `UnitArchived` (line 134). The error then stands in the summary of `#error-panel` without a link and without a line under an input (ADR-0041).
- `GetUnitForm` keeps its list of parents. The web layer stops rendering it. A later change can remove the list from the query.

The user chose this option over two options that need an ADR: an empty segment that `AddUnit` fills, and a second submit button `refresh=1`.

### D4. Refusal of an unchanged revision (F4, ADR-0159)

- `recipe-domain` gets `PlantError::RevisionUnchanged { type_name, revision }` with `E-PLT-052` and HTTP `409`. `revision` is the latest revision.
- `DefineUdt` in the mode `Revision` compares the resolved members with the members of the latest revision. The comparison runs after `ensure_members` and before `struct_row`, under the class-2 locks. `RecordShape::Struct` holds `members: Vec<TypeMember>`, and `TypeMember` derives `PartialEq`, so the comparison is one equality of two lists.
- `next_revision` returns the members of the latest revision with its number, so the use case reads the rows once.
- `error/status.rs` maps `E-PLT-052` to `409`. The keys `error-E-PLT-052` go into `en.ftl` and `pl.ftl`.
- The UDT form shows the error in the summary without a field line and keeps the sent rows.

## Risks / Trade-offs

- [A user cannot move a new unit to another parent inside the form] → The user closes the form. The user follows the link of the other node. Each live node with a free level has that link (requirement "Unit tree with the disclosure pattern").
- [After D2, Enter in a member row creates the type] → That is the meaning of ADR-0037 (h). The same holds for every other form with a save button. The type form shows the errors of a bad row and creates nothing then.
- [After D4, an existing test writes a revision with the same members] → The task of D4 runs the full suite. It changes each such test to change one member.
- [The page form of D1 replaces the version page in `#content`] → The form pushes its address, so Back returns to the list. The save answers `303` to the version page.
- [A form that the browser loaded before the change still holds the old `select`] → The `POST` fields do not change. An old form still works.
- [Browser tests at 1024 rows run slowly] → The tests build the rows with requests and not with clicks. Each test has its own time limit.
