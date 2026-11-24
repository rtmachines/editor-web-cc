# ADR-0223: Memory of the grid answers

Status: Accepted. Amends ADR-0039 (k), ADR-0045 (b) and ADR-0138 (c). Amended by ADR-0222 (items (v) and (w): round 1 uses the 4 096-leaf body of `value-body-memory-load` D9)
Date: 2026-11-22
Deciders: project owner

## Context

The terms of this ADR:

- MiB is 1 048 576 bytes. MB is 1 000 000 bytes.
- A grid answer is an answer that renders a leaf grid. `GridView::build` in `crates/recipe-web/src/pages/parameters/grid.rs` builds the grid.
- The row model is the `GridView` of a grid answer. The HTML is the rendered text of the answer.
- An array level is the limit path of one array with `[*]` for its index, for example `[1][*]`. ADR-0007 and ADR-0022 use this term.
- A type level is the path of an array level with `[*]` for each index, for example `[*][*]`. One type level covers each array level of the same array of the type.
- A counting pass renders a template into a writer that counts the bytes and stores none.
- An estimate is a number from the template texts and the measured lengths of the change `value-body-memory-load`. The measurement tests of this ADR replace each estimate.

The grid answers at commit `1551311`:

- The parameter page. It holds the edit form or the read-only grid of `parameter.html`.
- The form answers of the value routes. These are the switch of `fields=1`, the switch of `limit_mode`, the notice of a new type, and each refused form.
- The HTMX success of the `/definition` save. It reads the parameter again and builds the clean form.
- The answer of `SetParameterValue`. It renders `grid-values.html`.

The change `value-body-memory-load` measured the edit page of the largest valid parameter. The `GET` of that page has 176 653 007 bytes in `en`. Its type has 32 type levels, 16 384 `LDT` leaves, and leaf paths of 128 bytes (ADR-0211).

The grid of that page has 311 295 mode rows, because each array level with a limit per element gets its own mode row. The count is (2^14 − 1) + 18 × 2^14. A mode row holds the level path three times. By estimate, the mode rows hold 154 094 895 bytes in `en` and 159 698 205 bytes in `pl`.

Round 1 of that change sends four value bodies at the same time, and the server refuses each form. Each body has 16 758 074 bytes. The refused page of one body has 181 792 082 bytes in `en` and 187 346 307 bytes in `pl`.

A refused form holds up to three copies of its page:

1. The handler renders the first copy while it holds the permit of ADR-0211 (n).
2. After the permit drops, `render_errors` calls `refused_page`. It makes the second copy with `page_html()`.
3. `refused_page` makes the third copy with `format!`.

The three copies of the Polish page hold 562 038 921 bytes. This sum is above `container.app_memory` (512 MiB).

Other facts at commit `1551311`:

- askama `Template::render` reserves only `SIZE_HINT`. The `String` of a large page thus grows by doubling.
- The row model holds the same texts as the HTML. This ADR estimates the row model at about the size of the HTML.
- A save that succeeds also builds the grid of the sent fields.
- The HTMX success of `/definition` and the answer of `SetParameterValue` copy the form or the fragment into the body with `push_str`.
- The fixture is not the largest page. A type with one type level of 16 384 elements and 30 type levels of one element is valid under A26.
  - Without this ADR, its grid gets about 1 + 30 × 16 384 = 491 521 mode rows.

## Decision drivers

- The page of the largest valid parameter becomes smaller, and each grid answer holds fewer copies. The change stays small (R30, R31).
- The forms keep their behavior. A form lists every bad field at once (ADR-0041). A changed form body gets one line (ADR-0138).
- The `row_version` protocol (ADR-0008) and the HTMX rules of `api.md` §9 do not change.
- A later bound on the sum of parallel grid answers starts from measured numbers.
- Each numeric setting comes from `reliability.md` §7.11.

## Considered options

P is the HTML of one grid answer. V is its row model, and V is about P. A value body uses about 48 MiB (ADR-0211 (p)).

- The handler phase holds the value body, V, and the form. The phase after the handler holds the copies of the error layer.
- With one mode row for each type level, the `GET` in `en` has about 22.6 MB. The refused page of round 1 then has about 27.7 MB in `en` and in `pl`.

Every number of this table is an estimate for the fixture of the change `value-body-memory-load`. The section "Measured numbers" below gives the measured values.

| Option | One `GET` (estimate) | One refused form (estimate) | Four refused forms (estimate) | Verdict |
|---|---|---|---|---|
| Today | V + P ≈ 353 MB | V + P in the handler, 3 × 187 MB = 562 MB after the handler | above `container.app_memory` | fails |
| Option A: One buffer only | V + P ≈ 353 MB | V + P ≈ 375 MB in the handler | 4 × (48 MiB + 375 MB) | fails |
| Option B: Level path once in a mode row | P ≈ 177 − 57 = 120 MB | about 3 × 125 MB after the handler | above the ceiling | fails |
| Option C: One mode row for each type level, without one buffer | V + P ≈ 45.1 MB in `en` | 2 × 27.7 MB = 55.4 MB in the handler, 3 × 27.7 MB = 83.1 MB after the handler | about 423 MB in the handler | above the ceiling |
| Option D: Options A and C together | V + P ≈ 45.1 MB in `en` | 55.4 MB in the handler, 2 × 27.7 MB = 55.4 MB after the handler | 423.0 MB in `en`, 422.6 MB in `pl`, in the handler | above the ceiling, see item (v) |
| Option E: Render budget only | grid of the fixture not shown | no grid | bounded | change B |
| Option F: Permit for the `GET` only | V + P for each holder | not covered | not covered | change B |
| Option G: Streaming | about one row | about one row | 4 × 48 MiB | works, but item (u) rejects it |

The ceiling is `soak.memory_ceiling` (384 MiB). Option B saves about 182 bytes in each of 311 295 mode rows, which is about 56.7 MB. Option C removes 311 263 mode rows of the fixture.

## Decision

The decision selects Option D. The owner decided each item below on 2026-11-22.

One mode row for each type level:

- (a) The grid shows one mode row for each type level. This amends ADR-0039 (k), which gives one mode button to each array level.
  - The mode row of a type level stands where the walk meets the type level for the first time.
  - The button of the row sends `limit_mode` with the type level, for example `[*][*]`. The switch applies to each array level of the type level.
  - ADR-0039 (k) keeps its two modes and the copy of the `[*]` limit at a switch to "a limit per element".
  - ADR-0007 and ADR-0022 keep the term "array level" for the domain check.
- (b) The reader treats a `limit_mode` with an index segment, for example `[1][*]`, as malformed. It gives the one `E-WEB-008` line of ADR-0138 (c).
  - This amends ADR-0138 (c). Its list of a malformed `limit_mode` now holds a `limit_mode` with an index segment.
- (c) A type level shows "a limit per element" when a stored or sent limit holds an index at one of its array levels. This replaces the rule "at that level" of ADR-0039 (k) for the shown mode.
- (d) A switch back to one limit joins the limits of each array level alone. An array level with unequal limits gets an empty `[*]` row.
  - One notice of ADR-0039 (k) stands in the mode row of the type level. It names the type level and the count of the array levels with unequal limits.
  - The key `grid-mode-join-notice` gets the argument `$count` in the bundles `pl` and `en`.
  - The switch partitions the limit rows by their array level in one pass. It does not scan the rows once for each array level.
- (e) ADR-0045 (b) places the row `[*]` and the mode button of a nested level inside each group. This ADR amends the mode button part of ADR-0045 (b).
  - The mode button stands once for each type level, in the mode row of item (a).
  - The row `[*]` of an array level still stands inside its group.

Mixed modes in one type level:

- (f) The domain permits limits that mix both modes in one type level, for example `[1][*]` and `[2][1]` (`domain-model.md` §2.1, ADR-0007 as amended by ADR-0022). The edit form then shows a limit per element at that type level.
  - The edit form copies each `[*]` limit to the element rows of its array level, as the switch of ADR-0039 (k) does.
  - A notice names the type level. The next save stores the copied limits.
- (g) A line of a sent limit row names its input by the leaf path and the field, not by the sent row number. The copied rows thus get new row numbers safely.
  - When the grid has no input of that field for the leaf path, the line keeps the id of the sent row number. The line then stands only in the summary, as today.
  - The copy can give the sent row number to an input of another leaf. Then the line has no id. It stands only in the summary, with no link and no line under an input.
- (h) The copied limits add limit rows. A save can then exceed `budget.limit_rows_per_version` (32 768). The command then refuses the save with `E-RCP-021` and writes nothing.
- (i) The read-only grid of `parameter.html` and `grid-values.html` shows the mode text of each type level. It shows the copied limits as text.
  - The read-only grid has no button and no notice, because a reader cannot save.
- (j) The fixture then has 32 mode rows and 16 384 leaf rows. By estimate, its HTML shrinks from about 176.7 MB to about 22.6 MB in `en`.
  - The estimate is 176 653 007 − 154 094 895 + 11 751 = 22 569 863 bytes. The 32 new mode rows hold about 11 751 bytes.

One buffer, and no grid without a grid answer:

- (k) A grid answer holds at most two of its parts at the same time. The parts are the row model, the HTML of the form, and the HTML of the page.
- (l) `refused_page` writes the head, the form, and the tail into one buffer with the length of the page.
  - It removes the `hx-*` attributes of the first tag during the copy of the form. No third copy exists.
  - The form drops with the error at the end of `render_errors`.
- (m) These answers write their parts into one buffer with their exact length:
  - The HTMX answer of a refused form writes the fragment, the form, and the empty `#toast`.
  - `saved` writes the summary, the clean form, and the base block of the translations.
  - `values_answer` writes the fragment and the toast.
  - `with_form` renders the form straight into the buffer of the body and checks the first tag there. It keeps no form string of its own.
  - `with_form` adds no counting pass for its other five callers.
- (n) A grid answer renders each template with `render_into` into a buffer of its exact length. A counting pass gives that length. The buffer thus does not grow by doubling.
- (o) Only these call sites in `crates/recipe-web/src/pages/` use the counting pass:
  - The parameter page: `render` of `ParameterPage` in `parameters/page.rs`.
  - The switch answers and the notice of a new type: `form_page` and `form_fragment` in `parameters/reply.rs`.
  - The refused grid form: `refuse` in `parameters/reply.rs`. A variant of `refused` in `draft_reply.rs` gives it the exact form.
  - The HTMX success of `/definition`: `saved` in `parameters/edit.rs`.
  - The answer of `SetParameterValue`: `values_answer` in `parameters/value.rs`.
  - Every other call of `render` and `RenderedForm::from_template` keeps one pass and its current buffer. The counting pass thus doubles the render time of the grid answers only.
- (p) `ready` builds the grid only when the answer renders it. A save that answers `303` and an HTMX success build no grid of the sent fields.
  - `failed` builds the sent grid only after the command refused the save. The grid gives `input_of` for the lines of the command.
  - The HTMX success still swaps the clean form out of band (ADR-0041 (h)). Only the grid of the sent fields and the copies go away.
- (q) The bytes of each page do not change. Route tests pin the SHA-256 of a refused page and of an HTMX refused answer.
  - The tests pin both hashes before the change of the buffers. The tests keep the same pins after the change.
  - Each request gets a new request id. The hash input thus replaces each copy of the request id with a fixed text of the same length.
- (r) `dhat` counts the capacity of a buffer. On musl, the unused tail of a grown buffer may not be resident. The change `value-body-memory-load` measures the resident effect with `memory.peak`.

The measurement of the peak:

- (s) The dev-dependency `dhat` measures the peak heap of `recipe-web`. `#![forbid(unsafe_code)]` forbids an own `GlobalAlloc` in the crate, and `dhat::Alloc` needs no `unsafe` there.
  - The module `heap.rs` is `#[cfg(test)]`, so clippy and the normal build compile `peak_in`, `peak_of`, and the measurement tests.
  - Only `#[global_allocator] static ALLOC: dhat::Alloc` has `#[cfg(heap_measure)]`. The compiler flag `--cfg heap_measure` comes from `RUSTFLAGS`. A Cargo feature is not possible, because CI runs the tests with `--all-features`.
  - `peak_in` and `peak_of` start with an `assert!` on `cfg!(heap_measure)`, through a plain function, because clippy denies a constant assertion. Without the flag, an ignored measurement test fails with a clear message and never passes on a peak of 0.
  - `[workspace.lints.rust]` in the root `Cargo.toml` sets `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(heap_measure)'] }`.
  - The measurement tests have `#[ignore]` and run alone with `--ignored --test-threads=1`. Thus no other test adds to the peak.
  - A measurement run uses its own target directory, so the normal build cache stays valid: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked <filter> -- --ignored --test-threads=1`.
  - The profiler builder writes no file into the crate directory. `cargo deny check` passes with `dhat` 0.3.3.
  - The allocator compiles under `#![forbid(unsafe_code)]`. One test process starts two `dhat::Profiler` one after the other without failure.
  - Without the allocator, `cargo test -p recipe-web --lib` takes 31 s. With it, the run did not end in 400 s. Therefore the allocator exists only in a build with `--cfg heap_measure`.
- (t) Two functions measure a peak. Each result counts the bytes above the heap at the start of its interval. Memory that exists before the interval does not count.
  - `peak_of` measures one request. It takes a closure that builds the request, so the closure allocates the request body inside the interval.
  - The interval of `peak_of` ends after the drop of the answer body. It reads the body frame by frame into a length counter and a SHA-256, and keeps no copy of the body.
  - The hash replaces each copy of the request id, as item (q) says. A carry of at most 35 bytes holds a copy that crosses two frames.
  - `peak_in` measures a closure, for example the build of the row model and the render on a loaded view. This is the render interval.
  - The request allowance is the peak of the request minus the peak of the render interval. It is a difference of two peaks, not a measured overlap.
  - The allowance holds the request body, the query result, and the copy of the in-memory store. Change B uses the allowance as an estimate only.
  - The route tests pin the page length and the SHA-256 exactly. They pin each peak as an upper bound: the measured value plus 1 MiB.
  - The specs require the pins of the tests and name no number. The measurement then adds the pins to this ADR, and its status becomes "Accepted".
  - The tests of the fixture first save the nested definition. Then they check the status, the 32 mode rows, and the 16 384 leaf rows before they pin a number.

No streaming:

- (u) The server does not stream the body of a grid answer. The section "Rejected options" below gives the reasons.

The sum of this ADR against the ceiling:

- (v) Round 1 sends four value bodies at the same time, and the server refuses each form. Each handler holds its value body, the row model, and the HTML of the form at the same time (item (k)).

| Part | Bytes |
|---|---|
| `soak.memory_ceiling` (384 MiB), a setting | 402 653 184 |
| Four value bodies, 4 × 48 MiB (ADR-0211 (p)), not measured by the route tests | 201 326 592 |
| Four render peaks of the refused page of round 1, 4 × 173 268 053, measured | 693 072 212 |
| Sum of the two parts above | 894 398 804 |
| Shortfall: the sum minus the ceiling, measured | 491 745 620 |

- The render peak of one refused page of round 1 is 173 268 053 bytes. The section "Measured numbers" gives it with the other peaks.
- The sum is 894 398 804 bytes. The ceiling is 402 653 184 bytes. The shortfall is thus 491 745 620 bytes, about 469.0 MiB.
- This ADR alone thus does not keep round 1 below the ceiling. The rest of the process is not in this sum.
- The sum stays at or below the ceiling only when V + P of each refused form is at most 48 MiB. The rest of the process then gets 0 bytes.
- The measured V + P of a refused page of round 1 is 173 268 053 bytes. 48 MiB is 50 331 648 bytes.

Deferred to change B:

- (w) On 2026-11-22 the owner moved these decisions to change B. The saved plan of change B keeps their text.
  - The memory frame of `soak.memory_ceiling` for the value bodies and the grid answers.
  - The budget `form.grid_memory` with its share, queue, wait, waiter limit, write deadline, and counting walk.
  - The write check of five commands with the port `GridSize` and the code `E-WEB-018`.
  - The Assumption A54 of the write check.
  - Change B starts after the pins of this ADR and after the change `value-body-memory-load` measures a real `memory.peak`.
- (x) `form.grid_memory`, `E-WEB-018`, and A54 have no row in `reliability.md` §7.11, `reliability.md` §1.2, or `overview.md` §9. Change B proposes them.

Items that keep their text:

- (y) ADR-0039 (i), ADR-0041 (h), and ADR-0211 (p) keep their text.
  - `design-system.md` §10.4 stays valid. It gets one new bullet for the mode row of each type level.
  - The requirement "Limit modes of an array level" of `web-ui` stays valid. Its button sentence names the button of the type level.

## Rejected options

- Today: the three copies of one Polish refused page exceed `container.app_memory`.
- Option A alone keeps 311 295 mode rows. The page of the fixture then stays at about 177 MB.
- Option B alone keeps 311 295 mode rows. Four refused forms stay above the ceiling.
- Option C alone keeps the third copy of the error layer.
- This ADR does not select Options E and F. Change B decides them.
- Option G, the streaming of the answer body, has these faults:
  - A streamed grid uses about one row of memory for each answer. It needs a walk that writes rows into a bounded channel.
  - The status and the headers leave before the last row. A fault after the first byte can then only cut the connection, and no error fragment arrives.
  - The body needs its own deadline after the handler returns. The browser still gets 177 MB for the fixture without item (a).
  - Items (a) and (k) make each page smaller with less change.
- The resident peak as the metric: `VmHWM` of `/proc/self/status` after a reset through `/proc/self/clear_refs`. It needs no dependency.
  - It measures the resident pages of the process, not the heap. It thus changes the metric.
  - The requirement, the bounds of 64 KiB of the unit tests, and the pins then change with it. Only the owner can make that change.

## Measured numbers

The measurement tests of item (t) measure each grid answer of the fixture. They run in a debug build with `--cfg heap_measure` and `dhat`.

- A peak counts the bytes above the heap at the start of its interval. The request peak covers the whole request. The render peak covers the render interval.
- The sum of change A is 201 326 592 bytes plus 4 × the render peak. Here 201 326 592 is 4 × 48 MiB (ADR-0211 (p)).
- Request peaks vary by at most 1 864 bytes between runs. Lengths, SHA-256 values, and render peaks stay equal across runs.
- The tests pin each length and each SHA-256 exactly. They pin each peak as an upper bound: the measured value plus 1 048 576 bytes.
- This ADR holds no SHA-256 value. The route tests of the files below hold them.

| Answer | Status | Length (bytes) | Request peak (bytes) | Render peak (bytes) | Sum of change A (bytes) |
|---|---|---|---|---|---|
| GET edit page `en` | 200 | 22 569 867 | 194 881 859 | 106 201 524 | none |
| GET edit page `pl` | 200 | 22 521 311 | 194 881 859 | 106 201 524 | none |
| GET fragment `pl` (HX-Target content) | 200 | 22 518 784 | 194 881 866 | 106 201 524 | none |
| read-only page `pl` (Operator) | 200 | 6 331 619 | 164 015 093 | 106 200 912 | none |
| HTMX value save `/value` `pl` | 200 | 6 327 677 | 318 200 978 | 6 327 757 | none |
| refused round 1, page `en` | 422 E-WEB-008 | 27 708 942 | 232 947 212 | 173 268 053 | 894 398 804 |
| refused round 1, page `pl` | 422 | 27 660 433 | 232 947 212 | 173 268 053 | 894 398 804 |
| refused round 1, HTMX `en` | 422 | 27 706 793 | 232 947 212 | 173 268 053 | 894 398 804 |
| refused round 1, HTMX `pl` | 422 | 27 658 249 | 232 947 212 | 173 268 053 | 894 398 804 |
| 16 384 lines E-RCP-004, page `pl` | 422 | 33 318 063 | 239 892 126 | 106 203 306 | 626 139 816 |
| 16 384 lines E-RCP-004, HTMX `pl` | 422 | 33 315 879 | 239 892 126 | 106 203 306 | 626 139 816 |
| switch join `[*]`, page `pl` | 200 | 31 841 238 | 218 480 985 | 127 402 702 | 710 937 400 |
| switch join `[*]`, HTMX `pl` | 200 | 31 838 777 | 218 480 985 | 127 402 702 | 710 937 400 |
| switch `fields=1`, page `pl` | 200 | 22 518 742 | 218 481 018 | 106 203 306 | 626 139 816 |
| switch `fields=1`, HTMX `pl` | 200 | 22 516 281 | 218 481 018 | 106 203 306 | 626 139 816 |
| notice of a new type, page `pl` | 200 | 22 453 500 | 218 483 358 | 106 203 308 | 626 139 824 |
| notice of a new type, HTMX `pl` | 200 | 22 451 039 | 218 483 358 | 106 203 308 | 626 139 824 |
| refused command DuplicateTag, page `pl` | 409 E-RCP-014 | 22 519 615 | 243 761 195 | 106 203 310 | 626 139 832 |
| refused command DuplicateTag, HTMX `pl` | 409 | 22 517 431 | 243 763 059 | 106 203 310 | 626 139 832 |
| valid save, no HTMX | 303 | 0 | 345 978 140 | none (no render) | none |
| valid save, HTMX | 200 | 22 515 938 | 345 980 004 | 22 516 579 | none |
| wide shape GET `pl` (2 mode rows) | 200 | 22 222 695 | 68 457 492 | 64 723 738 | none |
| deep shape GET `pl` (31 mode rows) | 200 | 29 362 342 | 214 406 323 | 116 729 361 | none |
| refused STRING(254) x 254 `"`, page `pl` | 422 E-WEB-008 | 27 510 269 | 65 133 372 | 57 740 881 | 432 290 116 |

The route tests are in `crates/recipe-web/src/pages/parameters/route_tests/`:

- `largest_measure_tests.rs`: the edit page, the fragment, the read-only page, and the value save.
- `largest_refused_tests.rs`: the refused forms of round 1 and the refused form with a line under each leaf.
- `largest_switch_tests.rs`: the switch of `limit_mode` and the switch of `fields=1`.
- `largest_again_tests.rs`: the notice of a new type and the refused command `DuplicateTag`.
- `largest_save_tests.rs`: the valid save.
- `shape_tests.rs`: the wide shape, the deep shape, and the refused `STRING(254)` form.

- The render peak of the refused page of round 1 holds the row model and the HTML of the form. It is 173 268 053 bytes in `en` and in `pl`.
  - The estimate was about 2 × P = 55.4 MB. The measured value is about 3.1 times that.
- The refused page of round 1, the refused form with a line under each leaf, and the refused `STRING(254)` form give three render peaks. The largest is 173 268 053 bytes.
- The request side of the saves is larger than the render side.
  - The valid save without `HX-Request` answers `303` and renders no grid. Its request peak is 345 978 140 bytes.
  - The value save peaks at 318 200 978 bytes. Its render peak is 6 327 757 bytes.
- Change A changes only the render. These request peaks thus stay for change B.

## Consequences

### Positive

- The page of the fixture becomes about 8 times smaller, by estimate.
- Each grid answer holds at most two of its parts. No buffer of a grid answer grows by doubling.
- A valid save builds no grid of the sent fields.
- Route tests pin the length, the SHA-256, and the peak heap of each grid answer of the fixture. Change B starts from measured numbers.
- The change adds no setting, no error code, and no migration. No stored data changes.

### Negative

- This ADR does not bound the count or the sum of parallel `GET` pages.
- This ADR adds no write check. A command can still store a parameter with a very large grid.
- The counting pass doubles the render time of each grid answer. Other pages keep one pass.
- A save of a type level with mixed modes changes the stored limits.
- The CI must run the ignored measurement tests in a job of their own.

### Risks and mitigations

- Risk: four refused forms stay above the ceiling in `en` and in `pl`.
  - Mitigation: the change `value-body-memory-load` measures `memory.peak`. Change B bounds the sum.
- Risk: the count of parallel `GET` pages has no bound.
  - Mitigation: the exposure is the same as today, with pages about 8 times smaller. Change B bounds it.
- Risk: the row model is larger than the HTML.
  - Mitigation: the measurement tests measure the render interval of each answer. Change B uses the measured numbers.
- Risk: the copied limits exceed `budget.limit_rows_per_version`.
  - Mitigation: the command refuses the save with `E-RCP-021`. A test checks both sides of the budget.
- Risk: `dhat::Alloc` can take a lock for each allocation, also without a profiler.
  - Mitigation: a test run compares the time of `cargo test -p recipe-web` with and without it. Above 10 % more time, the owner decides.
- Risk: the measurement tests are slow under the profiler, and `cargo test --workspace` skips them.
  - Mitigation: a CI job of their own runs them with its own timeout.
- Risk: the fixture is not the largest page.
  - Mitigation: the route tests pin three other shapes.

## Related

- ADR-0007, ADR-0022: the term "array level" and the domain check of a `[*]` row
- ADR-0008: the `row_version` protocol
- ADR-0039 (i), (k), amended by this ADR in (k): the edit form and the limit modes
- ADR-0041 (h): the form of a success answer
- ADR-0045 (b), amended by this ADR: the mode button of a nested level
- ADR-0138 (c), amended by this ADR: a malformed `limit_mode`
- ADR-0211 (n), (p): the permits and the memory of a value body
- `reliability.md` §1.2, §7.11
- `api.md` §9
- `design-system.md` §10.4
- `domain-model.md` §2.1
- `overview.md` §7: R09, R30, R31, R39. `overview.md` §9: A26
