# Design

## Context

See `proposal.md`, section "Why". ADR-0108 holds the five decisions. This design links to it and does not repeat it.

The state of the code before the change:

| Gap | State | Place |
|---|---|---|
| Value column | The table read loads no value. The doc comment of `ParameterRow` gives the reason | `recipe-db/src/transaction/parameter_reads.rs` (`SELECT_PAGE`), `recipe-app/src/ports/recipe_reads.rs` (`ParameterRow`), `recipe-app/src/recipe/queries/parameters.rs` (`ListParameters`), `recipe-web/templates/parameter-table.html`, `templates/components/parameter.html` |
| Content hash | `recipe_version.content_sha256` holds the hash. `VersionDetail` has no field for it | `recipe-db/src/transaction/version_reads.rs`, `version_rows.rs`, `recipe-app/src/ports/recipe_reads.rs` (`VersionDetail`), `recipe-web/templates/version.html` |
| Compare form | The version page shows the two links of ADR-0050 (l) | `recipe-web/src/pages/versions/actions.rs` (`compare`), `recipe-web/src/routes.rs` |
| Toast | Six draft toasts name the tag, the language, or the type | `parameter-saved`, `parameter-added`, `parameter-deleted`, `translation-saved`, `translation-deleted`, `rebind-done` in `recipe-web/src/pages/parameters/` and `pages/versions/rebind.rs` |
| S7 layout | `S7Layout::of` returns the size and the start class of a whole tree | `recipe-domain/src/plant/s7_layout.rs`, `recipe-app/src/plant/queries/types.rs` (`TypeView` holds the machine and the tree), `recipe-web/templates/type.html` |

## Goals / Non-Goals

**Goals:**

- Close the five gaps of `proposal.md` with no migration, no new error code, and no new permission.
- Keep one formatter for the text of a leaf, one rule set for the S7 layout, and one source for each number.
- Write the normative documents before the code that follows them.

**Non-Goals:**

- The limits of a parameter in the parameter table.
- The elements of an array in the S7 layout.
- A change of the export document or of the hash rules of `api.md` section 6.

## Decisions

### D1. Value of a primitive parameter (ADR-0108 (a), (b))

1. `parameter_page` and `SELECT_PAGE` stay free of values, because `GetVersion` calls `parameter_page` for a draft with no row limit. `GetVersion` loads no value.
   A new port method `parameter_values(version, keys)` reads the values of the keys of one page. Its statement in `parameter_reads.rs` reads only these keys.
   Only `ListParameters` calls it, with the keys of the primitive rows. The doc comment names the bound of ADR-0108 (b).
2. A free function `value_text(tree, value) -> Result<Option<String>, ValueFault>` lives in `recipe-app` next to `ListParameters`. It writes the leaf text of a stored value.
3. `ListParameters` already reads the type rows of the page for `type_texts`. It resolves the tree of each primitive type with `TypeTree::resolve` and calls `value_text`.
   `value_text` converts the stored value with the `TryFrom<(&StoredValue, &TypeTree)>` path of `value/stored.rs`, as `pages/parameters/form_view.rs` uses it, and then writes the leaf text.
   `ParameterList` gets `value_texts: BTreeMap<ParameterKey, String>`.
   A stored value that does not match its tree gives `ValueFault`. `ListParameters` maps it to `StorageError::invariant`, so the answer is `500 E-DB-008`, as on the diff pages.
4. One function in `recipe-web` cuts the text to `table.value_chars` characters by `char`, not by byte, and adds `…`. The cut never uses a string slice (`reliability.md` section 2).
5. `AddParameterOutput` gets the field `value_text: Option<String>`. The command builds it from the parsed canonical value with `value_text`.
   The answer row of `AddParameter` uses the same macro of `components/parameter.html` and the same cut. The input `85.50` therefore shows `85.5`.

Alternative: the SQL cuts the text with `left(p.value #>> '{}', n)`. Rejected, because then two formatters write a leaf text, and a time leaf of A43 needs the domain formatter.

Alternative: add `value` to `ParameterRow` and to `SELECT_PAGE`. Rejected, because `GetVersion` would then load the value of every parameter of a draft with no row limit.

### D2. Content hash (ADR-0108 (c))

`version_reads.rs` adds `content_sha256` to the statement that reads `VersionDetail`. The field is `Option<Sha256>` and is `None` for a `Draft`. The memory adapter of `recipe-app` fills it in the same way. The template shows it in a `dd` with the class `data hash`. `components.css` gives `.hash` the rule `overflow-wrap: anywhere` (`design-system.md` section 1 names the file).

### D3. Compare form and route (ADR-0108 (d), (e))

- `actions.rs` builds the compare form next to the compare links. The version numbers come from the recipe headers that `ListParameters` already returns. The page runs no new query.
- The form stands after the row of links, outside `<p class="actions">`. It has the literals `hx-boost="true" hx-target="#content" hx-push-url="true"`, as in `templates/audit.html`. An Administrator on a deleted recipe keeps the form, as for the links of ADR-0050 (l).
- The design keeps two lists of version numbers, so the user can select any pair. The form holds 2 × V options, and V is the count of versions of the recipe.
- The route handler lives in `recipe-web/src/pages/versions/compare.rs`. A request struct with `#[serde(deny_unknown_fields)]` reads `n` and `m`. The path value `{ref}` parses into `ReferenceNumber` as on the other version routes.
- The answer is `303` with `Location` `/recipes/{ref}/versions/{max(n,m)}/diff/{min(n,m)}`. This is the direction that `compare_links` builds. The route needs a session like every other page route, so the session layer answers `E-WEB-004` without a session.

### D4. Change number in the toast (ADR-0108 (f))

- A new bundle key `toast-change-number` holds "Change number: { $row_version }." and "Numer zmiany: { $row_version }.". The new keys `value-saved` and `limits-saved` hold the toasts of `SetParameterValue` and `ReplaceLimits`. Both bundles hold all three keys.
- One function in `pages/draft_reply.rs` joins the toast sentence and the change number. It takes `&DraftOutput`, like `success()`. Each of the eight draft toasts calls it.
- `rebind-done` passes the same `row_version` that its `HX-Trigger` holds.
- A test reads the toast and `HX-Trigger` of the same answer and compares the two numbers.

### D5. Member offsets of the S7 layout (ADR-0108 (g), (h))

- `s7_layout.rs` first extracts the shared placement step `place(cursor, &S7Layout)` from `struct_layout`. Then `S7Layout::of` and `s7_members` both use it, so the S7 rules exist once.
- `s7_layout.rs` then gets `pub fn s7_members(tree: &TypeTree) -> Result<Vec<S7Member>, PlantError>`. `S7Member` holds the member names from the root as `Vec<MemberName>`, the type id with its S7 layout, and the offset as a byte and a bit.
- `S7Member` has accessors for the byte and the bit, and a method that gives the path as text, such as `.zone.active`. It does not use `LeafPath`, because `LeafPath` names one scalar.
- The file stays below 800 lines. When it grows past that, the walk moves to `s7_layout/members.rs`.
- The type page calls `s7_members` only for an S7 platform. `PlantError` from the function maps to `WebError` as in the other plant pages.
- A golden unit test holds the rows of the spec scenarios and a struct of 16 `BOOL` with the size 2 bytes. A property test in `s7_layout/prop.rs` checks the size rule of ADR-0108 (g) on generated trees.

### D6. Documents

| Document | Change |
|---|---|
| `docs/adr/README.md` | Index row of ADR-0108. ADR-0108 joins the decision map rows "Design system, accessibility, UI layout" and "Diff of two versions" |
| ADR-0043, ADR-0050 | "Amended by ADR-0108" notes |
| `api.md` section 2.3 | The type page row names the S7 layout |
| `api.md` section 2.5 | New row `GET /recipes/{ref}/compare`. The version page row names the content hash and the compare form |
| `api.md` section 2.6 | The text of the parameter table names the column Value |
| `reliability.md` section 7.11 | Row `table.value_chars` with the value 64 characters |
| `design-system.md` | The class `hash`, the muted words of the value cell, and the compare form in the row of links |
| `CHANGELOG.md` | Section "Unreleased" |

### D7. Agents and run order

Each code task is for one Sonnet agent with effort high. A `[mechanical]` task is for one Haiku agent. Opus reviews. One agent gets one task.

1. Wave A: 1.1, then 1.2. The normative documents come first.
2. Wave B: run 2.1 in parallel with the chain 3.1, 3.2, 3.3, 3.4.
   Each task that changes a port struct or a port trait also updates the `recipe-db` constructors and the memory adapter. The workspace then compiles after each task.
3. Wave C: 4.1, 4.2, 4.3, 4.4, 4.5, and 4.6, one after the other. They share `recipe-web` files and both bundles. Task 4.6 needs 2.1. Task 4.1 needs 3.4.
4. Wave D: 5.1 and 6.1 in parallel. Then 6.2, then 6.3.

At most two cargo agents still run at the same time.

## Risks / Trade-offs

- Risk: a page of 100 parameters of the type `WSTRING(16382)` reads about 5 MiB through `parameter_values`. Mitigation: ADR-0108 (b) gives the bound, and the type size limits of ADR-0017 keep it. `GetVersion` loads no value.
- Risk: the column Value makes the table wider than 320 px. Mitigation: the cell breaks a long text, and the Playwright suite checks the width at 320 px (task 6.1).
- Accepted risk: the S7 rules of ADR-0017 have no reference table of the Siemens manual. The address checks of I20 in `recipe/draft/s7.rs` use the same `S7Layout`. A wrong S7 rule is therefore wrong in both places, and neither place detects it.
- Trade-off: the compare route runs no use case. The diff route checks the permission one request later. The compare route reads no data, so no data reaches a user without the permission.
