# ADR-0037: Pages of the plant model

Status: Accepted. Amended by ADR-0038 (datalist of a new revision in (i), browser test of `E-PLT-005` in (u)) and ADR-0051 (navigation link "Audit" after (q))
Date: 2026-10-14
Deciders: project owner

## Context

Step P3.2 adds the pages of `api.md` sections 2.2 and 2.3 for machines, units, and UDT types. The design system of ADR-0036 binds every page.

The documents leave these points open or disagree:

- `design-system.md` section 10.8 says that HTMX loads the children of a tree node in step P3.2. `api.md` section 2.2 has no route for the children of a node.
- `api.md` section 9 forbids user data in an `hx-*` attribute. A URL of these pages holds a `MachineCode`, a `UnitPath`, or a `TypeName`.
- `api.md` section 2.3 has no way to add a member row to the UDT form without JavaScript. It also has no form for a new revision.
- `GET /machines` has the fields `q` and `page`. `reliability.md` section 7.11 has no page size for it, and no document says what `q` matches.
- `static/app.css` has 790 lines. The project keeps a file at 800 lines or fewer. The coding rules of the project owner and the reviews of P2.5 and P3.1 require this. The new pages need more style rules.
- `recipe-server serve` arrives in step P4.1. The test of P3.2 in a browser needs a running server before that step.
- The cycle check of a struct works on type identifiers. `DefineUdt` gives the new row a new identifier, and a member can name only an existing row. So no request can produce `E-PLT-005`.

## Decision drivers

- A user at a machine reads the unit tree and the type tree at a glance, with gloves, on a touch panel (R39, A44).
- No `hx-*` attribute holds user data, and the CSP does not change (`api.md` section 9).
- No URL holds the CSRF token.
- Each page works with the three scripts of `api.md` section 9 and their line limits.
- No new dependency (`overview.md` section 5).
- The form of a UDT member uses the field `members[i].type` with a `TypeExpr`, as `api.md` section 2.3 defines it.

## Considered options

### Option A: The server renders the whole unit tree

- Pros: no new route, no `hx-*` attribute on a node, and `tree.js` stays at 20 lines. A tree of 64 units is a small page.
- Cons: a machine with very many units gives a long page.

### Option B: HTMX loads the children of a node

- Pros: the first page is short.
- Cons: a new route in `api.md` section 2.2, four requests to reach level 5, and a URL with user data in `hx-get`. `tree.js` must also handle a load and an error.

## Decision

The project uses **Option A**. The letters below record each decision of step P3.2.

Unit tree:

- (a) The server renders the whole unit tree on the machine page and in the fragment of `GET /machines/{code}/units`. No route returns the children of one node. `tree.js` only expands and collapses a node. This replaces the last rule of `design-system.md` section 10.8.
- (b) The nodes of depth 1 and 2 are expanded. The ancestors of the focus unit are also expanded. The focus unit is the unit that an `AddUnit` request created. Its link has the attribute `autofocus`, so HTMX moves the focus to it after the swap. A full page has no focus unit.
- (c) A node holds a `button` and a link. The button shows the icon and the unit path, and it expands or collapses the group. The link shows the unit name and opens the unit page. A leaf holds the unit path and the link.
  - A user with `UnitManage` also gets the link "Add a sub-unit" on each live node of depth 1 to 4. So the form cannot ask for a sixth level. The server still returns `E-PLT-002` for a request with six levels.
  - The targets of one row keep the gap `--target-gap`. Two rows keep the larger gap `--space-4`, so a wrapped row reads as one group. Each link of a row has the class `.target-link`.
  - A group indents its nodes by `--space-4`. A row wraps at a small width, and the unit path does not break inside.
- (d) The tree of a live machine shows its live units. The page `/machines/by-id/{uuid}` of an archived machine shows every unit of the machine. It holds no form and no link to a route under `/machines/{code}`, because that route resolves the live machine. An archived unit there shows the state "Archived". A live machine opened by its identifier shows the page of the live machine.
- (e) The type tree uses the same disclosure pattern. One type can occur twice in one tree. So the `id` of a group is the prefix `tree-group-`, the UUID of the type, a hyphen, and the index of the node in preorder.

Requests and forms:

- (f) Every HTMX request of these pages comes from a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute that a template of these pages writes holds a literal. The attribute `hx-headers` of `base.html` stays as `api.md` section 7.3 requires it. `api.md` section 9 does not change.
- (g) A create form, an archive form, and the UDT form have `hx-target="#content"` and `hx-push-url="true"`. A rename form targets the region of the name. The form of a new unit targets `#unit-tree`. An error goes into `#error-panel`, and the form keeps the input of the user.
  - An archive form stands inside a `details` element. Its `summary` names the action, so an archive needs two clicks.
  - A request without `HX-Request` gets the full page of `api.md` section 1, or the `303` of its route.
- (h) The button "Add a member" of the UDT form is a second submit button with `name="add"` and `value="1"`. It sends the form with `POST` to the `action` of the form.
  - With `add=1`, the server runs the query `GetUdtForm` and creates no type. It answers `200` with the form, the sent rows, and one more empty row.
  - `GET /machines/{code}/types/new` accepts only `from`. So no URL holds the CSRF token or a member row.
  - `POST` ignores a row whose name and type are both empty. A form holds at most `type.max_members` rows.
- (i) The field of a member type is one text input with a `TypeExpr` and `autocomplete="off"`. A `datalist` proposes the primitives of the platform and the latest revision of each struct of the machine as `<name>@<revision>`. One hint above the rows gives examples such as `STRING(20)` and `ARRAY[1..8] OF REAL`.
- (j) The form of a new revision is `GET /machines/{code}/types/new?from={name}`. It holds the members of the latest revision of `{name}`. Its `action` is `POST /machines/{code}/types/{name}/revisions`. The type page links to it for a user with `UdtManage`.
- (k) `recipe-web` reads the indexed fields `members[i].name` and `members[i].type` from the pairs of the form body.
  - An index is a decimal number with no leading zero and is below `type.max_members`. Two keys for one field of one row are an error.
  - The input of row `i` has the `id` `members-<i>-name` or `members-<i>-type`.
  - A field error of a row names the row number and the field, and it links to the input of the row.

Lists and queries:

- (l) `GET /machines` shows `page.machines` rows per page (`reliability.md` section 7.11). `page` in the URL starts at 1, and the query counts pages from 0, as `ListUsers` does.
  - `q` is a `Text`. It matches a part of the code or of the name, with no difference between upper and lower case.
  - The rows come in the order of the code number. `show_archived=1` adds the archived machines.
- (m) `GET /machines/{code}/types` shows one row per struct revision of the machine. The rows come in the byte order of the name, and inside one name the latest revision comes first.
- (n) `GET /machines/{code}/types/{name}/usage` shows one row per parameter that uses the revision, directly or through another type. A parameter gives one row, also when its type holds the revision twice.
  - A row names the recipe, the version, its state, the tag, and the type of the parameter.
  - The page shows the versions `Draft`, `Approved`, and `Retired` of the recipes that are not deleted. It shows `page.type_usage` rows per page.
- (o) A malformed `{code}` or `{uuid}` gives `404 E-WEB-005` with a new variant `Missing::MalformedMachine`. A malformed `{name}`, `from`, or `rev` gives `Missing::MalformedType`.
  - A well-formed `rev` that the struct does not have gives `Missing::TypeRevision`.
  - A malformed `parent_path` gives the code of `UnitPath` (`api.md` section 1).

Look and navigation:

- (p) The state "Archived" of a machine or a unit has the word key `status-archived`, the tokens of `Retired`, and the icon `status-retired`. It is a state of `design-system.md` section 9.
- (q) The navigation shows the link "Machines" to `/machines` after "Recipes". The query of the link is `ListMachines`.
- (r) The style rules live in two files.
  - `static/app.css` holds the tokens, the fonts, the base elements, the targets, and the focus. It also holds the page layout, the top bar, and the base rules of the fields and the buttons. It holds the utilities `.actions`, `.target-link`, `.data`, and `.htmx-indicator` too.
  - `static/components.css` holds the components of `design-system.md` sections 10.4 and 10.6 to 10.11. It also holds the parts of section 10.5 that step P3.2 adds. These are the select, the checkbox, the list proposals, the repeated rows, and the archive disclosure.
  - Every full page links both files.

Tests and tools:

- (s) The tests of the pages and fragments use structural assertions on the HTML and on the headers. The project adds no snapshot crate.
- (t) `crates/recipe-web/examples/dev_server.rs` runs the router on the memory adapters for the test in a browser.
  - It listens only on `127.0.0.1`. It uses only the dev dependencies of `recipe-web`.
  - It reads the password of its three seeded users from the environment and never writes it to a log.
  - It makes a new signing key for the pre-session cookie at each start.
- (u) A test of `recipe-web` seeds two structs that contain each other into the memory store. A `DefineUdt` request with a member of one of them then gets `E-PLT-005`. The test in a browser shows `E-PLT-013` and `E-PLT-021` in its place.

Answers and field names:

- (v) Two forms of one page can have a field with one name. Then each input `id` and each field error use a name of the form, for example `machine_name` and `unit_name`. The `name` attribute keeps the field name of `api.md` section 2.
- (w) An HTMX request with `HX-Target: content` gets the content of `main`, the `title` element, and an out-of-band update of the field `next` of the language switch. Its `h1` gets the focus.
  - Another HTMX request of a page route gets the fragment of its target, for example the unit form for `#unit-form`.
  - A request without `HX-Request`, or with another target, gets the full page.

Archive message and reserved name:

- (x) After an archive, the server answers `303` with the field `archived`: `/machines?archived={code}` or `/machines/{code}?archived={path}`. The page then shows a success message in `#toast`. The query shows the message only when it finds that machine or unit archived.
  - `archived` only selects a message, so it is a request field and not a path value. A malformed code gives `422 E-WEB-008`. A malformed path gives the code of `UnitPath`.
- (y) A struct with the name `new` would have no page, because the route `/types/new` comes first. `recipe-web` refuses the name `new` with `E-WEB-008`.
- (z) Each HTMX error answer sets `HX-Push-Url: false`. So a refused form with `hx-push-url="true"` keeps the URL of the page that holds the form. A reload then shows the same form page.

## Consequences

### Positive

- The unit tree needs no new route and no network request to expand a node.
- No `hx-*` attribute holds user data. The CSP and the script limits do not change.
- The UDT form works without JavaScript, and the member type keeps the full `TypeExpr` grammar.
- No URL holds the CSRF token or the rows of a form.
- The test in a browser runs before step P4.1 and never reaches the release binary.

### Negative

- A machine with several hundred units gives a long tree page.
- A member row needs a full form request, and in read-only mode the form cannot add a row.
- A second style sheet adds one request on the first page load.
- A page route answers three shapes: the full page, the content of `main`, and the fragment of its target.

### Risks and mitigations

- Risk: the two style sheets drift from the design system. Mitigation: the style tests read both files.
- Risk: `examples/dev_server.rs` grows into a second server. Mitigation: it holds no setting of `reliability.md` section 7.11 that a production server reads, and step P4.1 adds the real `serve`.
- Risk: a user cannot find why a member row fails. Mitigation: a field error names the row, and `E-PLT-006` and `E-PLT-021` name the member.
- Risk: `autofocus` after an HTMX swap does not move the focus in one browser. Mitigation: the test in a browser checks it, and the toast still announces the result.

## Related

- ADR-0036 (design system), amended by this ADR in (h), (i), and (n)
- ADR-0038 (struct cycle across revisions), which amends (i) and (u)
- ADR-0001, ADR-0006, ADR-0012, ADR-0013, ADR-0018, ADR-0021, ADR-0025
- `api.md` sections 1, 2.1, 2.2, 2.3, 5.3, 7.3, 9
- `design-system.md` sections 1, 9, 10.1, 10.2, 10.5, 10.8, 11, 12
- `reliability.md` section 7.11
- WAI-ARIA Authoring Practices, disclosure pattern
