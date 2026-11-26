# Design

## Context

See `proposal.md`, section "Why". This design checks the facts at `origin/main` `1551311`. MiB is 1 048 576 bytes. MB is 1 000 000 bytes.

This change is change A of the split of 2026-11-22. The owner split the earlier plan of 63 tasks into change A and change B. The section "Deferred to change B" names the parts that change B keeps.

The terms of this design:

- A grid answer is an answer that renders a leaf grid. The grid comes from `GridView::build` (`crates/recipe-web/src/pages/parameters/grid.rs`).
- The row model is the `GridView` of a grid answer. The HTML is the rendered text of the answer.
- A counting pass renders a template into a writer that counts the bytes and stores none (D3).
- An array level is the limit path of one array with `[*]` for its index, for example `[1][*]`. ADR-0007 and ADR-0022 use this term.
- A type level is the path of an array level with `[*]` for each index, for example `[*][*]`. One type level covers each array level of the same array of the type.
- An estimate is a number from the template texts and the measured lengths of the paused change. Group 5 of `tasks.md` measures it.

Grid answers at `1551311`:

- The parameter page: `stored_grid` (`page.rs:82`) for the edit form and for the read-only grid of `parameter.html`.
- The form answers of the value routes: `sent_shown` (`reply.rs:225`). `prepare` calls it at lines 51, 67, and 122. These answers are the switch of `fields=1`, the switch of `limit_mode`, the notice of a new type, and each refused form.
- The HTMX success of the `/definition` save: `saved` (`edit.rs:123`). It reads the parameter again and builds the clean form with `stored_shown` (`edit.rs:147`).
- The answer of `SetParameterValue`: `values_answer` (`value.rs:92`) renders `grid-values.html`.
- `added` (`new.rs:86`) and `limits_body` (`value.rs:109`) build no grid.

Facts of the paused change `value-body-memory-load`, checked against the code:

- The `GET` of the edit page of the largest valid parameter has 176 653 007 bytes in `en`. Its type has 32 type levels, 16 384 `LDT` leaves, and leaf paths of 128 bytes (ADR-0211).
  - The fixture is `largest_form_world` and `form_pairs` of `crates/recipe-web/src/pages/parameters/route_tests.rs` (module `largest_form_tests`, from line 331).
  - `largest_form_world` seeds the parameter `BIG` with the scalar type `LDT` and no limit (`route_tests.rs:568`). The nested type exists only after the test saves `form_pairs` to `/definition`.
  - `form_pairs` sends `row_version=1` (`route_tests.rs:447`). After the first save, the draft has the `row_version` 2.
  - The paused change measured this length. This design did not measure it again.
- The grid of that page has 311 295 mode rows. This count is (2^14 − 1) + 18 × 2^14 = 16 383 + 294 912. Each array level with a limit per element gets its own mode row (`walk.rs:197`, `walk.rs:317`).
- A mode row holds the level path three times (`grid.html:15`). From the template texts, the 311 295 mode rows hold 154 094 895 bytes in `en` and 159 698 205 bytes in `pl`. These are estimates.
  - A mode row of the fixture has about 495 bytes in `en`. The `pl` texts of the button and of the mode add 18 bytes to each mode row.
  - The `pl` text "nie dotyczy" is 3 bytes shorter than "not applicable" in each of 16 384 leaf rows.
  - These two values explain the difference of 5 554 225 bytes between the two refused pages of round 1 below, apart from 67 bytes.
- Round 1 of the paused change sends a body of 16 758 074 bytes to the scalar `BIG` of `largest_form_world` (`value_leg_full_form` in its worktree).
  - The body holds 4 600 000 `x` in `description_en`. Each `LDT` text has the long prefix `LDATE_AND_TIME#` of ADR-0205.
  - The refused page of that body has 181 792 082 bytes in `en` and 187 346 307 bytes in `pl`. The page shows the sent description and the sent texts again.
- The value body permit covers only a `POST` on the four value routes (`limits.rs:71`). It drops at `limits.rs:100`.
- `render_errors` is the outer layer (`routes.rs:383`). It renders the page of a refused form after the permit drops.
- The handler renders the first copy of a refused form (`RenderedForm::from_template`, `form.rs:47`, called from `draft_reply.rs:48` and `draft_reply.rs:105`). This copy thus exists while the handler holds the permit.
- `refused_page` makes the second copy with `page_html()` (`form.rs:82`) and the third copy with `format!` (`answer.rs:407`). It takes the frame of the session from the form (`answer.rs:396-397`).
- The three copies of the Polish refused page of round 1 hold 562 038 921 bytes. This sum is above `container.app_memory` (536 870 912 bytes).
- The row model holds the same texts as the HTML. Each `LeafRow` of the fixture also holds about 22 `String` values of 24 bytes each. This design estimates the row model at about the size of the HTML.
- `ready` (`prepare.rs:122`) builds the grid also for a save that succeeds. A save without `HX-Request` answers `303` and renders no grid.
- `with_form` (`draft_reply.rs:100-108`) renders the clean form into its own `String` and copies it into the body with `push_str`. `saved` then appends the base block of the translations with a second `push_str` (`edit.rs:160`).
- `values_answer` renders the fragment and then appends the toast with `push_str` (`value.rs:99-101`).
- askama `Template::render` reserves only `SIZE_HINT`, so a large page grows its `String` by doubling (`pages/render.rs:19-25`).
- Each request gets a new request id (`pipeline.rs:58`). An error fragment shows it, so a hash of an error answer changes on each run.
- The in-memory store copies its state at the start of each transaction (`recipe-app/src/memory/core/store.rs:241`). A heap measurement of a route test thus includes that copy.
- A stored value has at most 1 048 576 bytes (`migrations/0001_schema.sql:431`). The query result of a grid answer stays alive during the render (`page.rs:195-214`, `value.rs:93-98`).
- The domain refuses a `[*]` row and an index row for the same array level (`limit/list.rs:101`, ADR-0007). It accepts `[1][*]` with `[2][1]`, because these are two array levels.
- The switch of `limit_mode` scans every limit row once (`split` and `join` of `grid/limits.rs`).
- A line of a bad `max_length` names the input `limits-<i>-max_length` with the sent row number `i` (`fields.rs:605`). The walk gives each limit row a new number (`walk.rs:427`).

The fixture is not the largest page. A type with one type level of 16 384 elements and 30 type levels of one element is valid under A26. Before D2, its grid gets about 1 + 30 × 16 384 = 491 521 mode rows.

### Measured numbers

Group 5 measured each grid answer of the fixture. The runs use a debug build with `--cfg heap_measure` and `dhat`. ADR-0223 holds the same numbers.

- A peak counts the bytes above the heap at the start of its interval. The request peak covers the whole request. The render peak covers the render interval.
- The sum of change A is 201 326 592 bytes plus 4 × the render peak. Here 201 326 592 is 4 × 48 MiB (ADR-0211 (p)).
- Request peaks vary by at most 1 864 bytes between runs. Lengths, SHA-256 values, and render peaks stay equal across runs.
- The tests pin each length and each SHA-256 exactly. They pin each peak as an upper bound: the measured value plus 1 048 576 bytes.
- The route tests hold the SHA-256 values. Neither this design nor ADR-0223 holds them.

| Task | Answer | Status | Length (bytes) | Request peak (bytes) | Render peak (bytes) | Sum of change A (bytes) |
|---|---|---|---|---|---|---|
| 5.1 | GET edit page `en` | 200 | 22 569 867 | 194 881 859 | 106 201 524 | none |
| 5.1 | GET edit page `pl` | 200 | 22 521 311 | 194 881 859 | 106 201 524 | none |
| 5.1 | GET fragment `pl` (HX-Target content) | 200 | 22 518 784 | 194 881 866 | 106 201 524 | none |
| 5.2 | read-only page `pl` (Operator) | 200 | 6 331 619 | 164 015 093 | 106 200 912 | none |
| 5.2 | HTMX value save `/value` `pl` | 200 | 6 327 677 | 318 200 978 | 6 327 757 | none |
| 5.3 | refused round 1, page `en` | 422 E-WEB-008 | 27 708 942 | 232 947 212 | 173 268 053 | 894 398 804 |
| 5.3 | refused round 1, page `pl` | 422 | 27 660 433 | 232 947 212 | 173 268 053 | 894 398 804 |
| 5.3 | refused round 1, HTMX `en` | 422 | 27 706 793 | 232 947 212 | 173 268 053 | 894 398 804 |
| 5.3 | refused round 1, HTMX `pl` | 422 | 27 658 249 | 232 947 212 | 173 268 053 | 894 398 804 |
| 5.4 | 16 384 lines E-RCP-004, page `pl` | 422 | 33 318 063 | 239 892 126 | 106 203 306 | 626 139 816 |
| 5.4 | 16 384 lines E-RCP-004, HTMX `pl` | 422 | 33 315 879 | 239 892 126 | 106 203 306 | 626 139 816 |
| 5.5 | switch join `[*]`, page `pl` | 200 | 31 841 238 | 218 480 985 | 127 402 702 | 710 937 400 |
| 5.5 | switch join `[*]`, HTMX `pl` | 200 | 31 838 777 | 218 480 985 | 127 402 702 | 710 937 400 |
| 5.5 | switch `fields=1`, page `pl` | 200 | 22 518 742 | 218 481 018 | 106 203 306 | 626 139 816 |
| 5.5 | switch `fields=1`, HTMX `pl` | 200 | 22 516 281 | 218 481 018 | 106 203 306 | 626 139 816 |
| 5.6 | notice of a new type, page `pl` | 200 | 22 453 500 | 218 483 358 | 106 203 308 | 626 139 824 |
| 5.6 | notice of a new type, HTMX `pl` | 200 | 22 451 039 | 218 483 358 | 106 203 308 | 626 139 824 |
| 5.6 | refused command DuplicateTag, page `pl` | 409 E-RCP-014 | 22 519 615 | 243 761 195 | 106 203 310 | 626 139 832 |
| 5.6 | refused command DuplicateTag, HTMX `pl` | 409 | 22 517 431 | 243 763 059 | 106 203 310 | 626 139 832 |
| 5.7 | valid save, no HTMX | 303 | 0 | 345 978 140 | none (no render) | none |
| 5.7 | valid save, HTMX | 200 | 22 515 938 | 345 980 004 | 22 516 579 | none |
| 5.8 | wide shape GET `pl` (2 mode rows) | 200 | 22 222 695 | 68 457 492 | 64 723 738 | none |
| 5.9 | deep shape GET `pl` (31 mode rows) | 200 | 29 362 342 | 214 406 323 | 116 729 361 | none |
| 5.10 | refused STRING(254) x 254 `"`, page `pl` | 422 E-WEB-008 | 27 510 269 | 65 133 372 | 57 740 881 | 432 290 116 |

- The render peak of the refused page of round 1 holds the row model and the HTML of the form. It is 173 268 053 bytes in `en` and in `pl`.
  - The estimate was about 2 × P = 55.4 MB. The measured value is about 3.1 times that.
- The refused page of round 1, the refused form with a line under each leaf, and the refused `STRING(254)` form give three render peaks. The largest is 173 268 053 bytes.
- The request side of the saves is larger than the render side.
  - The valid save without `HX-Request` answers `303` and renders no grid. Its request peak is 345 978 140 bytes.
  - The value save peaks at 318 200 978 bytes. Its render peak is 6 327 757 bytes.
- Change A changes only the render. These request peaks thus stay for change B.

## Goals / Non-Goals

**Goals:**

- Make the page of the largest valid parameter about 8 times smaller: one mode row for each type level (D2).
- Make each grid answer hold at most two of its parts at the same time. Each grid answer renders into a buffer of its exact length (D3).
- Pin the length and the peak heap of each grid answer of the fixture with `dhat` (D6). Change B and the paused change then start from measured numbers.
- Keep the behavior of the forms. This covers every bad field at once (ADR-0041) and one line for a changed form body (ADR-0138).
- Keep the `row_version` protocol (ADR-0008) and the HTMX rules of `api.md` §9.

**Non-Goals:**

- No bound on the sum of parallel grid answers and no write check. Change B owns them.
- No change to `http.body_limit.value`, to `http.value_body_permits`, or to the parse of a value body.
- No streaming of an answer body (D8).
- No change to the rebind forms of the version page. They do not use the leaf grid. `form.rebind_leaves` bounds them (ADR-0044).

## Decisions

The owner decided D1 to D8 on 2026-11-22. On the same day, the owner split the plan. Change A keeps D2, D3, D6, D7, and D8. ADR-0223 records D2, D3, D6, and D8. D7 gives the number of that ADR.

### Options with their numbers

The numbers are estimates for the fixture of the paused change. The section "Measured numbers" in "Context" gives the measured values. P is the HTML of one grid answer. V is its row model. V is about P. A value body uses 48 MiB (ADR-0211 (p)).

- The handler phase holds the value body, V, and the form. The phase after the handler holds the copies of the error layer.
- After D2, the `GET` in `en` has about 22.6 MB. The refused page of round 1 has about 27.7 MB in `en` and in `pl`.

| Option | One `GET` | One refused form | Four refused forms | Verdict |
|---|---|---|---|---|
| Today | V + P ≈ 353 MB | V + P in the handler, 3 × 187 MB = 562 MB after the handler | above `container.app_memory` | fails |
| O1. One buffer only | V + P ≈ 353 MB | V + P ≈ 375 MB in the handler | 4 × (48 MiB + 375 MB) | fails |
| O2a. Level path once in a mode row | P ≈ 177 − 57 = 120 MB | about 3 × 125 MB after the handler | above the ceiling | fails |
| O2b. One mode row for each type level, without D3 | V + P ≈ 45.1 MB in `en` | 2 × 27.7 MB = 55.4 MB in the handler, 3 × 27.7 MB = 83.1 MB after the handler | about 423 MB in the handler | above the ceiling |
| Change A: O1 + O2b (D2 and D3) | V + P ≈ 45.1 MB in `en` | 55.4 MB in the handler, 2 × 27.7 MB = 55.4 MB after the handler | 423.0 MB in `en`, 422.6 MB in `pl`, in the handler | above the ceiling, see "The sum of change A" |
| O3. Render budget only | grid of the fixture not shown | no grid | bounded | change B |
| O4. Permit for the `GET` only | V + P for each holder | not covered | not covered | change B |
| O5. Streaming | about one row | about one row | 4 × 48 MiB | works, but D8 rejects it |

O2a saves about 182 bytes in each of 311 295 mode rows, which is about 56.7 MB. O2b removes 311 263 mode rows of the fixture.

### D2. One mode row for each type level (decided)

On 2026-11-22 the owner chose D2, one mode row for each type level.

- The grid shows one mode row for each type level. This amends ADR-0039 (k). ADR-0007 and ADR-0022 keep the term "array level" for the domain check.
- The mode row of a type level stands where the walk meets the type level for the first time.
- The button of the row sends `limit_mode` with the type level. The switch applies to each array level of the type level.
- The reader treats a `limit_mode` with an index segment, for example `[1][*]`, as malformed. It gives the one line of ADR-0138 (c). This amends ADR-0138 (c).
- A type level shows "a limit per element" when a stored or sent limit holds an index at one of its array levels.
- A switch back to one limit joins the limits of each array level alone. An array level with unequal limits gets an empty `[*]` row.
  - One notice of ADR-0039 (k) stands in the mode row of the type level. It names the type level and the count of the array levels with unequal limits.
  - The key `grid-mode-join-notice` gets the argument `$count` in both bundles.
- The switch partitions the limit rows by their array level in one pass. It does not scan the rows once for each array level.
- The domain permits limits that mix both modes in one type level, for example `[1][*]` and `[2][1]` (`domain-model.md` §2.1). The edit form then shows a limit per element at that type level.
- ADR-0045 (b) places the mode button of a nested level inside each group. D2 amends this item, because the button stands once for each type level.
  - The edit form copies each `[*]` limit to the element rows of its array level, as the switch of ADR-0039 (k) does.
  - A notice names the type level. The next save stores the copied limits.
  - A line of a sent limit row names its input by the leaf path and the field, not by the sent row number. The copied rows thus get new numbers safely.
  - When the grid has no input of that field for the leaf path, the line keeps the id of the sent row number. It then stands only in the summary, as today.
  - The copy can give the sent row number to an input of another leaf. Then the line has no id. It stands only in the summary, with no link and no line under an input (ADR-0223 (g)).
  - The copied limits add limit rows. A save can then pass `budget.limit_rows_per_version` (32 768). The command then refuses it with `E-RCP-021` and writes nothing.
- The read-only grid of `parameter.html` and `grid-values.html` shows the mode text of each type level. It shows the copied limits as text. It has no button and no notice, because a reader cannot save.
- The fixture then has 32 mode rows and 16 384 leaf rows. Its HTML shrinks from about 176.7 MB to about 22.6 MB in `en`. This is an estimate: 176 653 007 − 154 094 895 + 11 751 = 22 569 863 bytes.
- `design-system.md` §10.4 and the requirement "Limit modes of an array level" of `web-ui` stay valid. Task 1.2 adds one bullet to §10.4. The delta spec adds two requirements and changes the button sentence of "Limit modes of an array level".
- Rejected: O2a alone. It keeps 311 295 mode rows.

### D3. One buffer, and no grid without a grid answer (decided)

On 2026-11-22 the owner chose D3, one buffer and no grid without a grid answer. On the same day, the owner chose the counting pass for the grid answers only.

- A grid answer holds at most two parts at the same time. The parts are the row model, the HTML of the form, and the HTML of the page.
- `refused_page` writes the head, the form, and the tail into one buffer with the length of the page. It removes the `hx-*` attributes of the first tag during the copy of the form. No third copy exists. The form drops with the error at the end of `render_errors`.
- The HTMX answer of a refused form writes the fragment, the form, and the empty `#toast` into one buffer with their exact length.
- A grid answer renders each template with `render_into` into a buffer of its exact length. A counting pass gives that length. The buffer thus does not grow by doubling.
- Only these call sites use the counting pass:
  - The parameter page: `render` of `ParameterPage` at `page.rs:214`.
  - The switch answers and the notice of a new type: `form_page` (`reply.rs:381`) and `form_fragment` (`reply.rs:343`).
  - The refused grid form: `refuse` (`reply.rs:424`). A variant of `refused` (`draft_reply.rs:34`) gives it the exact form.
  - The HTMX success of `saved` (`edit.rs:138-160`) and the answer of `values_answer` (`value.rs:99`).
- Every other call of `render` and `RenderedForm::from_template` keeps one pass and its current buffer. The counting pass thus doubles the render time of the grid answers only.
- `with_form` renders the form straight into the buffer of the body and checks the first tag there. No form string of its own exists. It adds no counting pass for its other five callers.
- `saved` writes the summary, the clean form, and the base block of the translations into one buffer with their exact length.
- `values_answer` writes the fragment and the toast into one buffer with their exact length.
- `ready` builds the grid only when the answer renders it. A save that answers `303` and an HTMX success build no sent grid.
- `failed` (`reply.rs:512`) builds the sent grid only after the command refused the save. The grid gives `input_of` for the lines of the command.
- The bytes of each page do not change. Task 3.7 pins the SHA-256 of a refused page and of an HTMX refused answer before group 4. Tasks 4.3 and 4.4 keep these pins.
  - Each request gets a new id. The hash input thus replaces each copy of the request id with a fixed text of the same length.
- `dhat` counts the capacity of a buffer. On musl, the unused tail of a grown buffer may not be resident. The paused change measures the resident effect with `memory.peak`.

### The sum of change A against the ceiling

This section gives the sum of change A for round 1 of the paused change. Round 1 sends four value bodies at the same time, and the server refuses each form. Each handler holds its value body, the row model, and the HTML of the form at the same time (D3).

| Part | Bytes |
|---|---|
| `soak.memory_ceiling` (384 MiB), a setting | 402 653 184 |
| Four value bodies, 4 × 48 MiB (ADR-0211 (p)), not measured by the route tests | 201 326 592 |
| Four render peaks of the refused page of round 1, 4 × 173 268 053, measured | 693 072 212 |
| Sum of the two parts above | 894 398 804 |
| Shortfall: the sum minus the ceiling, measured | 491 745 620 |

- The sum is 894 398 804 bytes. The ceiling is 402 653 184 bytes. The shortfall is thus 491 745 620 bytes, about 469.0 MiB.
- Change A alone thus does not close the finding of the paused change. The rest of the process is not in this sum.
- The sum stays at or below the ceiling only when V + P of each refused form is at most 48 MiB. Then the rest of the process gets 0 bytes.
- The measured V + P of a refused page of round 1 is 173 268 053 bytes. 48 MiB is 50 331 648 bytes.
- Task 5.3 measured the refused page of round 1 and its render interval. Task 5.4 measured a refused form with a line under each leaf. Task 5.10 measured a refused `STRING(254)` form.
- The render peak of task 5.3 is the largest of the three. It gives the sum.

### D6. The measurement of the peak (decided)

On 2026-11-22 the owner chose option (a), the dev-dependency `dhat`.

- `#![forbid(unsafe_code)]` forbids an own `GlobalAlloc` in the crate.
- Option (a), recommended: the dev-dependency `dhat`. Its `dhat::Alloc` needs no `unsafe` in the crate.
  - The module `heap.rs` is `#[cfg(test)]`, so clippy and the normal build compile `peak_in`, `peak_of`, and the measurement tests.
  - Only `#[global_allocator] static ALLOC: dhat::Alloc` has `#[cfg(heap_measure)]`. The compiler flag `--cfg heap_measure` comes from `RUSTFLAGS`. A Cargo feature is not possible, because CI runs the tests with `--all-features`.
  - `peak_in` and `peak_of` start with an `assert!` on `cfg!(heap_measure)`, through a plain function, because clippy denies a constant assertion. Without the flag, an ignored measurement test fails with a clear message and never passes on a peak of 0.
  - `[workspace.lints.rust]` in the root `Cargo.toml` sets `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(heap_measure)'] }`.
  - The measurement tests have `#[ignore]` and run alone with `--ignored --test-threads=1`. Thus no other test adds to the peak.
  - A measurement run uses its own target directory, so the normal build cache stays valid: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked <filter> -- --ignored --test-threads=1`.
  - The profiler builder writes no file into the crate directory. `cargo deny check` must pass with the new dependency.
- Task 2.2 measured the two open points on 2026-11-23.
  - `#[global_allocator]` with `dhat::Alloc` in a `#[cfg(test)]` module compiles under `#![forbid(unsafe_code)]`.
  - One test process starts two `dhat::Profiler` one after the other without failure. No file `dhat-heap.json` appears.
  - `cargo deny check` passes with `dhat` 0.3.3.
  - Without the allocator, `cargo test -p recipe-web --lib` takes 31 s. With the allocator, the run did not end in 400 s, and the measurement ended there.
  - Therefore the owner decided on 2026-11-23 that the allocator exists only in a build with `--cfg heap_measure`.
- `peak_of` measures one request. It takes a closure that builds the request, so the closure allocates the request body inside the interval. The interval ends after the drop of the answer body.
  - It reads the body frame by frame into a length counter and a SHA-256. It keeps no copy of the body.
  - The hash replaces each copy of the request id, as D3 says. A carry of at most 35 bytes holds a copy that crosses two frames.
- `peak_in` measures a closure, for example the build of the row model and the render on a loaded view. This is the render interval.
- Each result counts the bytes above the heap at the start of its interval. Memory that exists before the interval thus does not count.
- The tests pin both peaks on their own. The request allowance is the peak of the request minus the peak of the render interval.
  - The allowance is a difference of two peaks, not a measured overlap. It holds the request body, the query result, and the copy of the in-memory store.
  - Change B uses the allowance as an estimate only.
- The route tests pin the page length and the SHA-256 exactly. They pin each peak as an upper bound: the measured value plus 1 MiB. ADR-0223 copies the pins. The specs require the pins of the tests and name no number.
- The tests of the fixture first save the nested definition. Then they check the status, the 32 mode rows, and the 16 384 leaf rows before they pin a number.
- Option (b): `VmHWM` of `/proc/self/status` after a reset through `/proc/self/clear_refs`. It needs no dependency. It measures the resident pages of the process, not the heap.
  - Option (b) changes the metric. The requirement, the bounds of 64 KiB, and the pins then change with it. Only the owner can make that change.

### D7. The number of the ADR (decided)

On 2026-11-22 the owner chose ADR-0223 for this change.

- `origin/main` ends at ADR-0221. The paused change holds ADR-0222 in its worktree, without a commit.
- This change writes ADR-0223. The paused change keeps ADR-0222. The index then has a gap until the paused change merges.
- ADR-0223 keeps the status "Proposed" until task 5.11 adds the pins. Task 5.11 then sets the status "Accepted".

### D8. Rejected: streaming of the answer body (decided)

On 2026-11-22 the owner rejected the streaming of the answer body.

- A streamed grid uses about one row of memory for each answer. It needs a walk that writes rows into a bounded channel.
- The status and the headers leave before the last row. A fault after the first byte can then only cut the connection, and no error fragment arrives.
- The body needs its own deadline after the handler returns. The browser still gets 177 MB for the fixture without D2.
- D2 and D3 make each page smaller with less change. Change B decides the bound of the sum.

## Deferred to change B

On 2026-11-22 the owner moved these decisions to change B. The saved plan of change B keeps their text.

- D1: the memory frame of `soak.memory_ceiling` for the value bodies and the grid answers.
- D4: the budget `form.grid_memory` with its share, queue, wait, waiter limit, write deadline, and counting walk.
- D5: option (d), the write check of five commands with the port `GridSize` and the code `E-WEB-018`.
- A54: the Assumption of the write check.

Change B starts after the pins of change A and after the paused change measures a real `memory.peak`.

Change A does not give two things:

- No bound on the count or the sum of parallel `GET` pages.
- No write check. A command can still store a parameter with a very large grid.

The exposure thus stays as it is today, but each page of the fixture is about 8 times smaller.

## Effect on the paused change

After this change merges, the paused change `value-body-memory-load` must check these items:

- Its ADR-0222, items (c) to (f), stay valid. Change A adds no share and no `503`, so each answer of round 1 stays a full `422` render.
- Its ADR-0222, item (c), pins line 125 of `prepare.rs`. Task 4.5 of this change moves that code, so the paused change updates the pin.
- Its `design.md` D6: `ANSWER_LIMIT` (192 MiB) can shrink to the length of the page that task 5.3 measures, with a margin.
- Its `design.md` D3 and D5: the round 2 save builds no grid (D3), so the peak of round 2 drops.
- Its pinned bodies do not change. A form body holds no mode row.
- Round 1 stays above the ceiling by the measured sum of the heap peaks (see "The sum of change A"). Its measured `memory.peak` then goes to the owner and to change B.
- The three items of the first review (Opus R16):
  - Its worktree holds `call_bounded` and `post_fields_bounded` in `test_support.rs` without a commit. Task 2.1 keeps their signatures, and the paused change removes its own copy.
  - Task 4.5 moves the code that ADR-0222 (c) pins at line 125 of `prepare.rs`, as the second item says.
  - Tasks 2.4 and 5.1 to 5.10 edit `route_tests.rs`, which the paused change also edits. The change that merges second rebases the file.
- Its test `value_leg_refusal_order` asserts `href="#limits-0-max_length"` for an `LDT` row without that input. Under D2, that line keeps its sent id.
- The number of its ADR stays 0222 (D7).

## Risks / Trade-offs

- [Four refused forms stay above the ceiling in `en` and in `pl`.] → The paused change measures `memory.peak`. Change B bounds the sum.
- [The count of parallel `GET` pages has no bound.] → The exposure is the same as today, with pages about 8 times smaller. Change B bounds it.
- [The counting pass doubles the render time of each grid answer.] → Group 5 records the render time of each grid answer. Other pages keep one pass.
- [The row model is larger than the HTML.] → Group 5 measures the render interval of each answer. Change B uses the measured numbers.
- [A save of a type level with mixed modes changes the stored limits.] → A notice names the type level before the save.
- [The copied limits pass `budget.limit_rows_per_version`.] → The command refuses the save with `E-RCP-021`. Task 3.6 tests both sides of the budget.
- [`dhat::Alloc` slows each allocation, also without a profiler.] → Task 2.2 measured it. The run of `cargo test -p recipe-web --lib` took 31 s without it and more than 400 s with it. The owner decided on 2026-11-23 that only a build with `--cfg heap_measure` has the allocator.
- [The measurement tests are slow under the profiler.] → Task 5.12 runs them in a CI job of their own with its own timeout.
- [`cargo test --workspace` skips the ignored measurement tests.] → Task 5.12 adds a CI job that runs them.
- [The fixture is not the largest page.] → Tasks 5.8 to 5.10 pin three other shapes.

## Migration Plan

No migration. No stored data changes. The change adds no setting and no error code.

## Open Questions

None. On 2026-11-22 the owner decided open question 1: the counting pass, only at the grid answers of D3.
