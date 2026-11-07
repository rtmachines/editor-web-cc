# Design

## Context

See `proposal.md`, section "Why". ADR-0138 holds the rules. `api.md` §2.6, ADR-0039 (b), ADR-0041 (a), and ADR-0132 hold the context. This design links to them and does not repeat them.

The state of the code at HEAD `c18b201`. The author read each place in the source:

| Topic | State | Place |
|---|---|---|
| Parameter reader | `read_each` puts each pair in place and returns at the first `Err`. The definition check runs only after the reader returns | `crates/recipe-web/src/pages/parameters/fields.rs` lines 250 to 262 |
| Bad keys of the parameter reader | An unknown key, a key twice, and a second token give the reason `Form`. A bad value key or rebind field gives `ValueKey`. A bad limit key gives `LimitRow` | `fields.rs` lines 313 to 343 and the three `place_*` functions after them |
| Bad hidden fields of the parameter reader | `finish` checks the limit rows, the flag `fields`, `limit_mode`, and `value_type`. Each fault gives one line | `fields.rs` line 395 |
| Add and edit | The form query runs first (ADR-0046). Then `read_form` runs, and then `prepare` checks the definition fields | `crates/recipe-web/src/pages/parameters/new.rs` lines 169 to 177, `edit.rs` lines 115 to 131 |
| Value, limits, rebind | The same reader with `FieldSet::Value`, `FieldSet::Limits`, and `FieldSet::Rebind` | `crates/recipe-web/src/pages/parameters/value.rs` lines 125 and 174, `crates/recipe-web/src/pages/versions/rebind.rs` line 301 |
| UDT reader | `read` returns at the first bad key. The member rows get their check after the reader returns | `crates/recipe-web/src/pages/types/form_rows.rs` lines 128 to 152 |
| `serde` readers | Each struct has `deny_unknown_fields`. A `FormRejection` gives one line with the field `form` | for example `crates/recipe-web/src/pages/machines/form.rs` line 38, `crates/recipe-web/src/error.rs` line 242 |
| Two stages of ADR-0041 | `prepare` collects the lines of the definition fields, `row_version`, the leaf texts, and the limit list. A non-empty list refuses before the command runs. Only then does the command list the faults of the leaves and the limit rows | `crates/recipe-web/src/pages/parameters/prepare.rs` lines 120 to 133 and 161 to 163, `new.rs` lines 169 to 183 |
| Rebind fields | `hidden_fields` lists a bad `row_version` and one shared line for the two type rows. A well-formed member path that the planned tree does not hold gives one line `E-WEB-008` and no form | `crates/recipe-web/src/pages/versions/rebind.rs` lines 91 to 113 and 187 to 200 |
| `serde` typed value | `row_version: i64` and `allow_outdated: bool` of the approve form. A value that does not parse gives a `FormRejection` and one line with the field `form` | `crates/recipe-web/src/pages/approval/approve.rs` lines 49 and 52 |
| Answer of one line | `WebError::BadRequest` gives one line. Only `WebError::Fields` carries a rendered form, so the answer holds no out-of-band form | `crates/recipe-web/src/error/answer.rs` lines 327 and 377 |

The code already follows ADR-0138. The input of finding #11 gives one line `E-WEB-008` on the add route and on the edit route. The two stages of ADR-0041 (a) and (b) are older than this change, and ADR-0138 (f) only states them. A line of the web layer and a fault of the command never share one answer. Thus the change needs only the documents and pinning tests.

## Goals / Non-Goals

Goals:

- Pin ADR-0138 for each form reader: the parameter reader on its four routes and on the rebind route, the UDT reader, and one `serde` reader.
- Each pinning test sends a bad key together with a bad typed field, so that a reader that collects would give a second line.

Non-Goals:

- A change of a reader or of the answer.
- A pinning test for each `serde` form. They share the extractor of `axum` and one `From<FormRejection>`.
- A change of the answer to a missing field of a `serde` form.

## Decisions

### D1. One shape of a pinning test

Each test sends the body of the template of its form with two changes. One typed field gets a bad value. The body also gets one bad key. The test asserts these points:

- the status `422` and the code `E-WEB-008` in `#error-panel`
- exactly one line in the summary
- no element `field-error` and no `hx-swap-oob="true"` in the body
- an unchanged fixture: `world.fixture.snapshot()` before and after are equal

Test notes:

- `changed` of `test_kit.rs` replaces a key that the base holds. For a second `ordinal`, append the duplicate pair to the result of `changed`.
- No shared line counter exists. `summary_lines` of `versions/rebind_world.rs` is `pub(super)`, and `count_lines` of `value_tests.rs` is private. Each test file uses its own counter or a local copy.
- Every HTMX error answer holds `TOAST_CLEAR` with `hx-swap-oob="innerHTML"` (`error/answer.rs` line 57). Thus assert the absence of the literal `hx-swap-oob="true"`, not of `hx-swap-oob`.

Alternative: assert only the code. Rejected, because a reader that collects would also give `E-WEB-008` first.

### D2. Red step by a skip of the bad pair

The red step makes the reader skip a bad pair and continue. Then the bad typed field gives its own line, the out-of-band form, or another code. Each task names its mutation:

- M1, the parameter reader: in `read_each` of `fields.rs` line 260, replace `place(&mut parts, pair)?;` with `let _ = place(&mut parts, pair);`.
- M2, the UDT reader: in `read` of `form_rows.rs` line 138, replace `other => place_row(&mut rows, other, value)?,` with `other => { let _ = place_row(&mut rows, other, value); }`.
- M3, the machine form: remove `#[serde(deny_unknown_fields)]` from `CreateFields` in `machines/form.rs` line 38.

Alternative: a mutation of the answer. Rejected, because the answer is not the code under test.

### D3. Bodies of the tests

| Task | Route | Body changes |
|---|---|---|
| 1.1 | `POST .../parameters` | `tag=bad tag`. Four rows of bad keys: `bogus=1` with `value[*]=2` (the input of #11), `bogus=1` only, `value[*]=2` only, and a second `ordinal` |
| 1.2 | `POST .../parameters/{key}/definition` | `tag=bad tag`, `bogus=1`, `value[*]=2` |
| 1.3 | `POST .../parameters/{key}/value` and `.../limits` of `SPEED_KEY` | value: `row_version=1&value=abc&bogus=1`. Under M1 it gives `E-RCP-005` (`value_tests.rs` line 32). Limits: `row_version=1&limits[0].leaf_path=&limits[0].min=10&limits[0].max=5&bogus=1`. Under M1 it gives `E-RCP-018` (`value_tests.rs` line 208) |
| 1.4 | `POST .../versions/{n}/rebind` | a bad text of a new member leaf and `bogus=1` |
| 1.5 | `POST /machines/{code}/types` | the member names `9a` and `9b` and `bogus=1` |
| 1.6 | `POST /machines` | an empty machine code and `bogus=1` |

The agent of task 1.4 takes the route and the body from `rebind_error_tests.rs`.

## Risks / Trade-offs

- [The mutation of a task also breaks other tests] → The agent runs only the new test on the mutation. Then it reverts the mutation and runs `cmp`.
- [Tasks 1.1 to 1.4 apply M1 to one file] → The tasks run one after the other.
- [A test file passes 800 lines] → The test goes into a sibling test file.

## Migration Plan

None. The change adds tests and documents.
