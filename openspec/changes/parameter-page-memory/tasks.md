# Tasks

Marks: `[opus]` is an ADR text (Opus). `[sonnet]` is a Sonnet task. `[mechanical]` is a Haiku task. `[deploy]` is a Codex task. Run every command from the root of the worktree `/home/rtm/editor-web-cc-render-memory`. Each task takes at most 2 hours.

## Order and shared files

- The owner decided D2, D3, D6, D7, and D8 of `design.md` on 2026-11-22. The owner also decided open question 1: the counting pass, only at the grid answers.
- Group 1 comes first, because the ADR is the authority of the code. Task 1.2 copies the text of task 1.1, so it runs after task 1.1.
- Group 2 comes before groups 3 to 5, because they use the bounded calls, `peak_of`, `peak_in`, and the nested fixture.
- Tasks 2.1 to 2.3 change `crates/recipe-web/src/test_support.rs` or its module `heap.rs`. Run them one after the other.
- Task 2.2 also changes the root `Cargo.toml`.
- Task 2.4 uses `get_bounded` of task 2.1 and `peak_of` of task 2.3, and changes `route_tests.rs`. It runs after task 2.3.
- Task 2.5 comes before task 3.1, so that its red step fails. Task 3.1 runs the browser case again.
- Tasks 3.1, 3.2, and 3.4 change `grid/walk.rs`, `grid/limits.rs`, `templates/components/grid.html`, and both `.ftl` bundles. Run them one after the other.
- Task 3.3 changes only `fields.rs` and `fields_tests.rs`. It runs after task 3.2, because both change the tests of `limit_mode`.
- Task 3.5 changes `fields.rs` and `grid.rs`. It runs after tasks 3.3 and 3.4, because it maps the lines of the copied rows.
- Task 3.6 adds the test file `mixed_budget_tests.rs`. It runs after task 3.4, because it needs the copy of D2.
- Task 3.7 pins the SHA-256 of two refused answers after D2. It runs after task 3.5 and before group 4.
- Group 4 runs after group 3. Tasks 4.3 and 4.4 keep the pins of task 3.7.
- Tasks 4.1 to 4.4 change `pages/render.rs`, `error/form.rs`, `pages/draft_reply.rs`, `pages/parameters/reply.rs`, `pages/parameters/page.rs`, and `error/answer.rs`. Run them one after the other.
- Task 4.5 changes `prepare.rs`, `reply.rs`, `edit.rs`, `new.rs`, and `grid.rs`, because both handlers read `Ready` and call `failed`. Task 4.6 also changes `edit.rs`, so it runs after task 4.5.
- Task 4.7 changes only `value.rs`. It can run in parallel with tasks 4.5 and 4.6 after task 4.4.
- Tasks 2.4 and 5.1 to 5.10 add tests to `crates/recipe-web/src/pages/parameters/route_tests.rs`. Run them one after the other. Group 5 runs after group 4.
- Task 5.11 runs after task 5.10. Task 5.12 runs after task 5.11, because it names the filters of group 5.

## 1. ADR and documents

- [x] 1.1 (docs/adr) [opus] Write `docs/adr/0223-memory-of-the-grid-answers.md` in the shape of ADR-0211. Its status is "Proposed" until task 5.11.
  - Write its Context and the decisions D2, D3, D6, and D8 of `design.md`. D7 gives the number of the ADR.
  - Define the terms "array level" (ADR-0007, ADR-0022) and "type level" as in the Context of `design.md`.
  - From D2, copy four rules: a `limit_mode` with an index, the notice with its count, the read-only grid, and the line of a copied row.
  - State the consequence of D2 on `budget.limit_rows_per_version` and `E-RCP-021`.
  - From D3, copy the call sites of the counting pass.
  - Name each option of the table "Options with their numbers" with its numbers. Copy the section "The sum of change A against the ceiling" and mark each number as an estimate.
  - Copy the section "Deferred to change B": D1, D4, D5, and A54, with the owner date 2026-11-22.
  - Name the amended items ADR-0039 (k), ADR-0045 (b), and ADR-0138 (c).
  - State that ADR-0039 (i), ADR-0041 (h), and ADR-0211 (p) keep their text. `design-system.md` §10.4 gets one new bullet in task 1.2.
  - Red test: none, because the task writes no code.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/0223-memory-of-the-grid-answers.md` gives 0 hard violations.
- [x] 1.2 (docs/adr, docs/architecture) [mechanical] Copy the decided text of ADR-0223 into the documents.
  - `design-system.md` §10.4, the list "The leaf grid": add one mode row for each type level (ADR-0223).
  - Add the note "Amended by ADR-0223" to the Status lines of ADR-0039, ADR-0045, and ADR-0138.
  - Add the index row of ADR-0223 to `docs/adr/README.md`. Add ADR-0223 to the map entries "Frontend" and "Pages of recipes, versions, and parameters".
  - Red test: none, because the task writes no code.
  - Check: `grep -c 'ADR-0223' docs/adr/README.md docs/adr/0039-*.md docs/adr/0045-*.md docs/adr/0138-*.md docs/architecture/design-system.md` gives at least 1 for each file.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/README.md docs/adr/0039-*.md docs/adr/0045-*.md docs/adr/0138-*.md docs/architecture/design-system.md` gives 0 hard violations.

## 2. Test support and the fixture

- [x] 2.1 (recipe-web) [sonnet] Add `call_bounded`, `post_fields_bounded`, and `get_bounded` to `crates/recipe-web/src/test_support.rs`. Each reads an answer up to a named limit.
  - Keep the signatures of the paused worktree: `call_bounded(router, request, limit)` and `post_fields_bounded(router, path, session, headers, body, limit)`.
  - Red test: a test reads an answer of 2 MiB that `call` refuses (`BODY_LIMIT` is 1 MiB, `test_support.rs:62`). It fails until the functions exist.
  - Check: `cargo test -p recipe-web --all-features --locked bounded`.
- [x] 2.2 (recipe-web) [sonnet] Add the dev-dependency `dhat` (D6) and the `#[cfg(test)]` module `crates/recipe-web/src/test_support/heap.rs`.
  - Declare `#[cfg(heap_measure)] #[global_allocator] static ALLOC: dhat::Alloc`. Only this line has the `#[cfg(heap_measure)]` attribute.
  - Add `peak_in`. It starts with an `assert!` on `cfg!(heap_measure)` and a clear message. It measures one closure. Its result counts only the bytes above the heap at the start of its interval.
  - Add `unexpected_cfgs = { level = "warn", check-cfg = ['cfg(heap_measure)'] }` to `[workspace.lints.rust]` in the root `Cargo.toml`.
  - Set the profiler builder so that it writes no file. Read the builder options in the `dhat` docs with `rustdoc-text`.
  - The measurement tests have `#[ignore]`. They run alone with the measurement run of D6.
  - Red test: an ignored test allocates 8 MiB inside `peak_in` and requires a peak of at least 8 MiB.
  - Red test: one ignored test runs `peak_in` two times, one after the other, in one process.
  - Red test: after the run, no file `dhat-heap.json` exists in `crates/recipe-web`.
  - Red test: without the flag, `peak_in` fails with the message of the `assert!`. Run `cargo test -p recipe-web --all-features --locked peak_in -- --ignored --test-threads=1`. Expect the failures with that message.
  - Record the time of `cargo test -p recipe-web --all-features --locked` in the task report.
  - Check: the normal `cargo test -p recipe-web --all-features --locked` passes in about the time without the allocator. The measurement run of D6 passes `peak_in`. `cargo deny check` passes.
- [x] 2.3 (recipe-web) [sonnet] Add `peak_of` to `heap.rs`. It takes a closure that builds the request, so the request body is inside the interval.
  - `peak_of` starts with the same `assert!` as `peak_in`, through `has_heap_measure_flag`.
  - It runs the request and reads the body frame by frame into a length counter and a SHA-256. It keeps no copy.
  - The hash input replaces each copy of the request id with a fixed text of the same length. A carry of at most 35 bytes covers a copy across two frames.
  - Its result counts only the bytes above the heap at the start of the closure.
  - Red test: a body of two frames holds a request id across the frame border. Its hash equals the hash of the whole body with the fixed text.
  - Red test: an ignored test builds a request body of 4 MiB inside the closure. The peak is at least 4 MiB.
  - Red test: an ignored test answers a stream body of 4 MiB. The stream makes each of its 64 frames on demand. The peak of `peak_of` stays below 1 MiB.
  - In the same test, a reader that keeps every frame gives a peak of at least 4 MiB. Thus the bound tells the two readers apart.
  - The red tests fail before the task, because `peak_of` does not exist.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked peak_of -- --ignored --test-threads=1`.
- [x] 2.4 (recipe-web) [sonnet] Add the helper `nested_form_world` to `route_tests.rs`. It calls `largest_form_world` and saves `form_pairs` to `/definition`.
  - The helper requires `303`, a stored type of 32 type levels, and a limit per element at each array level.
  - It returns the committed `row_version` of the draft, which is 2. Each later save of group 5 sends that `row_version`.
  - Add the ignored test `largest_mode_rows`. It reads the `GET` of the page with `get_bounded` and counts the leaf rows and the `name="limit_mode"` buttons.
  - Record the time of one `GET` of the fixture under `peak_of` in the task report.
  - Red test: none. The test is a guard of the base: at `1551311` it requires 16 384 leaf rows and 311 295 mode rows. Task 3.1 changes the expected count.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_mode_rows -- --ignored --test-threads=1`.
- [x] 2.5 (e2e) [sonnet] Change `e2e/tests/support/seed.ts` and `e2e/tests/leaf-grid.spec.ts` for the mode rows of D2.
  - The seed adds the nested array `NEST` of type `ARRAY[1..2] OF ARRAY[1..3] OF INT` with a limit on each leaf.
  - Red test: the spec expects two mode buttons for `NEST`, one for each type level. Today the page shows three, so the test fails before task 3.1.
  - Check: `cd e2e && npm ci && npm test`, with `E2E_PASSWORD` exported in the shell. Only the new case of `leaf-grid` fails before task 3.1.

## 3. One mode row for each type level (D2)

- [x] 3.1 (recipe-web, e2e) [sonnet] The walk writes one mode row for each type level (`grid/walk.rs`). The row stands where the walk meets the type level for the first time.
  - Its button sends the type level. The read-only grid shows the mode text of each type level and no button.
  - Change `nested_group_tests.rs:160`, which expects the two mode rows `[1].arr[*]` and `[2].arr[*]`. Change `largest_mode_rows` of task 2.4 to 32 mode rows.
  - Change `grid_tests.rs:650`, which presses the array level `[1].hold[*]`. A grid gets only type levels after this task.
  - Red test: the scenario "Nested array with a limit per element" of the `web-ui` delta. Today the grid of `NEST` has three mode rows.
  - Red test: the scenario "Read-only grid of a mixed type level", part one. The page of the Operator shows two mode texts and no element `name="limit_mode"`.
  - Check: `cargo test -p recipe-web --all-features --locked grid` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_mode_rows -- --ignored --test-threads=1`.
  - Check: `cd e2e && npm test -- leaf-grid` passes, with `E2E_PASSWORD` exported in the shell.
- [x] 3.2 (recipe-web) [sonnet] The switch of a type level moves the limits of each of its array levels (`grid/limits.rs`). A switch back joins the limits of each array level alone.
  - `split` and `join` partition the rows by their array level in one pass. They do not scan the rows once for each array level.
  - `LimitSet.unequal` becomes the count of the array levels with unequal limits. The notice in the mode row of the type level shows that count.
  - Give the key `grid-mode-join-notice` of `en.ftl:729` and `pl.ftl:729` the argument `$count`. Use no plural form.
  - Change `grid/index_level_tests.rs:84-125`. Each case then tells the mode of an array level from the mode that the button of its type level shows.
  - Red test: the scenarios "Switch of an inner type level" and "Switch back with unequal limits in one array level" of the `web-ui` delta.
  - Red test: a grid has 2^10 array levels at `[*][*]` and 2^14 limit rows. A switch of `[*][*]` moves the limits of each array level. A test build counts the row visits. The switch makes at most 2 × 2^14 of them.
  - Check: `cargo test -p recipe-web --all-features --locked limit_mode`, `cargo test -p recipe-web --all-features --locked grid_index_levels`, and `cargo test -p recipe-web --all-features --locked i18n`.
- [x] 3.3 (recipe-web) [sonnet] The reader refuses a `limit_mode` with an index segment (`fields.rs:432`). It gives the one line of ADR-0138 (c).
  - Change `fields_tests.rs:408`. The text `[1].ramps[*]` moves from the good list to the bad list.
  - Red test: the scenario "A switch of one array level" of the `web-ui` delta. Today the reader accepts `[1][*]`.
  - Check: `cargo test -p recipe-web --all-features --locked fields_the_limit_mode`.
- [x] 3.4 (recipe-web, recipe-domain) [sonnet] A type level with mixed modes shows a limit per element in the edit form. It copies each `[*]` limit to its element rows and shows one notice. Add the notice to both `.ftl` bundles.
  - The read-only grid shows the copied limits as text and no notice.
  - Red test: the scenario "Mixed modes in one type level" of the `web-ui` delta.
  - Red test: the scenario "Read-only grid of a mixed type level", part two. The page of the Operator shows the copied limits and no notice.
  - Guard: the new domain test `limit_list_star_and_index_per_array_level` accepts `[1][*]` with `[2][1]`. It refuses `[1][*]` with `[1][1]` (`limit/list.rs:101`).
  - Check: `cargo test -p recipe-web --all-features --locked mixed_modes`, `cargo test -p recipe-domain --all-features --locked star_and_index`, and `cargo test -p recipe-web --all-features --locked i18n`.
- [x] 3.5 (recipe-web) [sonnet] A line of a sent limit row names its input by the leaf path and the field (`fields.rs:605`).
  - The line does not use the sent row number.
  - When the grid has no input of that field for the leaf path, the line keeps the id of the sent row number. It then stands only in the summary.
  - Red test: a `STRING` form with `[1][*]` and a bad `max_length` on `[2][1]`. The summary link and the line under the input name the input of `[2][1]`.
  - Today the line names the sent row number 1, which is the input of a copied row `[1][2]` after task 3.4.
  - Guard: a line of `max_length` on an `LDT` row keeps its sent id and stands only in the summary.
  - Check: `cargo test -p recipe-web --all-features --locked limit_line`.
- [x] 3.6 (recipe-web) [sonnet] Add `crates/recipe-web/src/pages/parameters/mixed_budget_tests.rs`. It tests the copy of D2 against `budget.limit_rows_per_version` (32 768).
  - The draft holds `WIDE` of type `ARRAY[1..2] OF ARRAY[1..8192] OF INT` with a limit on `[1][*]` and on each of `[2][1]` to `[2][8192]`.
  - A second parameter of the draft holds 16 384 limit rows. An unchanged save of `WIDE` then gives 32 768 rows and succeeds.
  - On success, the stored limits of `[1][1]` to `[1][8192]` equal the limit of `[1][*]`.
  - With one more limit row in the draft, the save gives 32 769 rows. Assert `422`, `E-RCP-021`, its summary line, and no write.
  - No write means: the limits, the audit entries, and the `row_version` of the draft do not change.
  - Red test: none. At `1551311` the test fails, because the page does not copy and the save answers `303`. After task 3.4, it guards the budget.
  - The test covers the scenario "Copied limits above the limit row budget" of the `web-ui` delta.
  - Check: `cargo test -p recipe-web --all-features --locked mixed_budget`.
- [x] 3.7 (recipe-web) [sonnet] Pin the SHA-256 of two refused route answers after D2 in `error/refused_page_tests.rs`. Tasks 4.3 and 4.4 keep these pins.
  - Use the parameter `LINE` of `DEFINITION_BODY` (`route_tests.rs:18`) with a bad value. Its grid holds a mode row and limit rows.
  - Pin the refused page without `HX-Request` and the HTMX refused answer. The hash input replaces each copy of the request id with a fixed text.
  - Red test: none. The task pins the bytes of the code after group 3, as a guard of group 4.
  - Check: `cargo test -p recipe-web --all-features --locked error_refused_page`.

## 4. One buffer, and no grid without a grid answer (D3)

- [x] 4.1 (recipe-web) [sonnet] Add the counting pass of D3. Add `render_exact` to `pages/render.rs` and `RenderedForm::from_template_exact` to `error/form.rs`.
  - Both render with `render_into` into a buffer of the counted length. `render` and `from_template` keep their code.
  - Use `render_exact` at `page.rs:214`, `reply.rs:343`, and `reply.rs:381`. `refuse` (`reply.rs:424`) gets the exact form through a variant of `refused` (`draft_reply.rs:34`).
  - Add a test builder of a grid of at least 1 MiB. The red tests of group 4 use it, so no bound tests a small form only.
  - Red test: the `String` of a rendered parameter page of at least 1 MiB has a capacity equal to its length. Today `render` grows the `String` by doubling.
  - Check: `cargo test -p recipe-web --all-features --locked render_exact`.
  - Check: `rg -n 'render_exact|from_template_exact' crates/recipe-web/src` lists only the call sites of D3 and the tests.
- [x] 4.2 (recipe-web) [sonnet] `with_form` (`pages/draft_reply.rs:100`) renders the form straight into the buffer of the body. It checks the first tag of the form inside that buffer.
  - Guard: the existing answers of the six callers of `with_form` keep their bytes. The test of `draft_reply/tests.rs:79` stays green.
  - Red test: `peak_in` around `with_form` with a form of at least 1 MiB gives at most the capacity of the body plus 64 KiB. Today the form and its copy exist at the same time.
  - Check: `cargo test -p recipe-web --all-features --locked with_form` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked with_form_peak -- --ignored --test-threads=1`.
- [x] 4.3 (recipe-web) [sonnet] `refused_page` (`error/answer.rs:396`) writes the head, the plain form, and the tail into one buffer of the page length. Remove `page_html()` (`error/form.rs:82`).
  - Guard: the pin of the refused page of task 3.7 stays equal.
  - Red test: `peak_in` around `answer` for a refused form of at least 1 MiB gives at most the page length plus 64 KiB. Today `page_html()` and `format!` make two more copies.
  - Check: `cargo test -p recipe-web --all-features --locked refused_page` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked refused_page_peak -- --ignored --test-threads=1`.
- [x] 4.4 (recipe-web) [sonnet] `body_of` (`error/answer.rs:416`) writes the fragment, the form, and `TOAST_CLEAR` of an HTMX refused answer into one buffer of their exact length.
  - Guard: the pin of the HTMX refused answer of task 3.7 stays equal.
  - Red test: the body of an HTMX refused answer with a form of at least 1 MiB has a capacity equal to its length. Today `push_str` grows it.
  - Red test: `peak_in` around `body_of` gives at most the body length plus 64 KiB. The form exists before the interval.
  - Check: `cargo test -p recipe-web --all-features --locked refused_htmx` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked refused_htmx_peak -- --ignored --test-threads=1`.
- [x] 4.5 (recipe-web) [sonnet] `ready` (`prepare.rs:122`) builds no grid for a valid save. `failed` (`reply.rs:512`) builds the sent grid only after the command refused the save.
  - Add a `#[cfg(test)]` thread-local count of the calls of `GridView::build` (`grid.rs:234`).
  - Red test: the count is 0 for a valid save that answers `303`. Today `ready` calls `sent_shown` for every save, so the count is 1.
  - Red test: for an HTMX success of `/definition`, only the clean form of `saved` builds a grid, so the count is 1. Today it is 2.
  - Red test: the count is 0 for a `409` conflict. Today it is 1.
  - Guard: the count is 1 for a refused field. A save that the command refuses still gets the lines of `input_of` under its inputs.
  - Check: `cargo test -p recipe-web --all-features --locked prepare` and `cargo test -p recipe-web --all-features --locked grid_builds`.
- [x] 4.6 (recipe-web) [sonnet] `saved` (`edit.rs:123`) writes three parts into one buffer of their exact length. The parts are the summary, the clean form, and the base block of the translations.
  - Guard: the HTMX success answer of `/definition` in the existing tests keeps its bytes, with the request id replaced.
  - First move the assembly of the body unchanged into a function `saved_body` that returns the `Html<String>`. `success` then consumes that body.
  - Red test: `saved_body` with a clean form of at least 1 MiB gives a body with a capacity equal to its length. On the moved code, `with_form` and `push_str` grow it (`edit.rs:153-160`).
  - Red test: `peak_in` around `saved_body` gives at most the body length plus 64 KiB. The view exists before the interval.
  - Show in the task report one run where a temporary copy of the whole fragment fails that bound.
  - Check: `cargo test -p recipe-web --all-features --locked definition_saved` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked saved_body_peak -- --ignored --test-threads=1`.
- [x] 4.7 (recipe-web) [sonnet] `values_answer` (`value.rs:92`) writes the fragment and the toast into one buffer of their exact length.
  - Guard: the answer of `SetParameterValue` in the existing tests keeps its bytes, with the request id replaced.
  - First move the assembly of the body unchanged into a function `values_body` that returns the `Html<String>`. `success` then consumes that body.
  - Red test: `values_body` for a grid of at least 1 MiB gives a body with a capacity equal to its length. On the moved code, `push_str` grows it (`value.rs:101`).
  - Red test: `peak_in` around `values_body` gives at most the body length plus 64 KiB. The view exists before the interval.
  - Show in the task report one run where a temporary copy of the whole fragment fails that bound.
  - Check: `cargo test -p recipe-web --all-features --locked values_answer` and `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked values_body_peak -- --ignored --test-threads=1`.

## 5. Measurement after the fix (D6)

Each task of this group adds ignored tests to `route_tests.rs`. They use `nested_form_world` of task 2.4, `peak_of`, and `peak_in`.

- Each save sends the committed `row_version` of the draft, not the `row_version=1` of `form_pairs`.
- Each test checks the status, the mode rows, and the leaf rows before it pins a number.
- A test of a save also checks the `Location` header, or the committed `HX-Trigger` and the clean form.
- It pins the length and the SHA-256 exactly, and each peak as the measured value plus 1 MiB.
- It prints the peak of the request, the peak of the render interval, the request allowance, and the render time.
- A pin records the code after groups 3 and 4, so it is a regression check and not a red test. Tasks 4.1 to 4.7 hold the red tests of D3.

- [x] 5.1 (recipe-web) [sonnet] Measure the `GET` of the edit page in `en` and in `pl`, and the fragment with `HX-Request` and `HX-Target: content`.
  - Red test: none, as the group says. The test covers the scenario "Edit page of the largest valid parameter" of the `operations` delta.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_page -- --ignored --test-threads=1`.
- [x] 5.2 (recipe-web) [sonnet] Measure the read-only page of a reader in `pl`, and the HTMX answer of a value save on `/value`.
  - The body of `/value` holds only the fields of the value route: `row_version` and the value leaves.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_read -- --ignored --test-threads=1`.
- [x] 5.3 (recipe-web) [sonnet] Measure the refused form of round 1 of the paused change in `en` and in `pl`, with and without `HX-Request`.
  - Send it to the scalar `BIG` of `largest_form_world`, with `row_version=1`. The body holds 4 600 000 `x` in `description_en`.
  - Each `LDT` text of a value, a minimum, and a maximum has the prefix `LDATE_AND_TIME#` (ADR-0205).
  - Assert the body length 16 758 074 and the SHA-256 `bef1574257ef4baa65a9f61e4b96f981255100359efeedfee6d3a7b1077dd6bd` of the paused change.
  - The test also prints the sum of "The sum of change A" with the measured render interval. The sum is 201 326 592 plus 4 times that peak.
  - Red test: none, as the group says. The test covers the scenario "Refused form of the largest valid parameter" of the `operations` delta.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_refused -- --ignored --test-threads=1`.
- [x] 5.4 (recipe-web) [sonnet] Measure a refused fixture form with a bad text in each value leaf in `pl`, with and without `HX-Request`.
  - The answer holds one line under each of 16 384 value inputs. The test prints the sum of task 5.3 with this peak.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_refused_lines -- --ignored --test-threads=1`.
- [x] 5.5 (recipe-web) [sonnet] Measure the switch answers of the fixture in `pl`, with and without `HX-Request`.
  - The first answer joins the innermost type level with `limit_mode`. It holds 16 384 rows `[*]` and 16 384 leaf rows.
  - The second answer is the answer of `fields=1`.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_switch -- --ignored --test-threads=1`.
- [x] 5.6 (recipe-web) [sonnet] Measure the notice of a new type and a refused command in `pl`, with and without `HX-Request`.
  - The notice answer sends a `type` that differs from `value_type` (`prepare.rs:51`).
  - The command answer sends the `tag` of a second parameter of the draft. `failed` (`reply.rs:512`) gives the line of `RecipeError::DuplicateTag`.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_again -- --ignored --test-threads=1`.
- [x] 5.7 (recipe-web) [sonnet] Measure a valid save of the edit form on `/definition`: the `303` answer without `HX-Request`, and the HTMX success.
  - Red test: none, as the group says. The test covers the scenarios "Save that answers 303" and "HTMX success of a definition save" of the `operations` delta.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked largest_save -- --ignored --test-threads=1`.
- [x] 5.8 (recipe-web) [sonnet] Measure the shape `ARRAY[1..16384] OF ARRAY[1..1] OF LDT` with a limit per element at its outer type level (owner choice of 2026-11-22).
  - Pin its mode rows (2), its rows `[*]` of the inner type level, its leaf rows, its page length in `pl`, and its peaks.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked wide_shape -- --ignored --test-threads=1`.
- [x] 5.9 (recipe-web) [sonnet] Measure the deep shape. Its type has one type level of 16 384 elements and 30 type levels of one element. Its leaf type is `LDT`, and each leaf has a limit.
  - Pin its mode rows (31), its leaf rows, its page length in `pl`, and its peaks.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked deep_shape -- --ignored --test-threads=1`.
- [x] 5.10 (recipe-web) [sonnet] Measure a refused form of the shape `ARRAY[1..16384] OF STRING(254)` in `pl`, without `HX-Request`.
  - Each value leaf holds the character `"` as often as the body limit of 16 MiB permits, at most 254 times. Record that count.
  - A `"` costs 3 bytes in the body and 5 bytes in the HTML. The answer thus is larger than its body.
  - Red test: none, as the group says.
  - Check: `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked string_shape -- --ignored --test-threads=1`.
- [x] 5.11 (docs/adr, openspec/changes/parameter-page-memory) [mechanical] Copy the pins and the printed sums of tasks 5.1 to 5.10 into ADR-0223. Copy them also into the section "Context" of `design.md`.
  - Replace each estimate of "The sum of change A" with the larger measured peak of tasks 5.3, 5.4, and 5.10.
  - Set the status of ADR-0223 to "Accepted".
  - Red test: none, because the task writes no code.
  - Check: `python -I tools/ste-lint/ste-lint.py openspec/changes/parameter-page-memory/design.md docs/adr/0223-memory-of-the-grid-answers.md` gives 0 hard violations.
- [ ] 5.12 (.github/workflows) [deploy] Add the job `heap` to `.github/workflows/ci.yml`. It sets `RUSTFLAGS="--cfg heap_measure"` and `CARGO_TARGET_DIR=target/heap`. It is a matrix of five shards with `fail-fast: false`. Each shard runs `cargo test -p recipe-web --all-features --locked -- --ignored --test-threads=1` with its own filters.
  - Only the shard `rest` also runs `cargo clippy -p recipe-web --all-targets --all-features --locked -- -D warnings` with the same `RUSTFLAGS`, because the allocator line compiles only there.
  - The shards are `page` (`largest_page largest_read largest_mode_rows`), `refused` (`largest_refused`), `switch` (`largest_switch`), `again` (`largest_again`), and `rest`.
  - The shard `rest` passes `--skip` for each filter of the other four shards, so it runs every other ignored test.
  - The owner chose this split on 2026-11-23, because one job would run for about 4 to 5 hours.
  - The filters are those of tasks 2.2 to 2.4, 4.2 to 4.4, 4.6, 4.7, and 5.1 to 5.10.
  - Each shard gets its own `timeout-minutes`. The provisional value is 150. Set the final value from the first branch run plus a margin. The job `test` keeps its 30 minutes (`ci.yml:718`).
  - The shards share one rust-cache key, `heap`.
  - The job stays in `-p recipe-web`, because a workspace run of `--ignored` also runs the `recipe-db` tests that write fixture files.
  - Change the comment "CI runs no ignored test" of `xss_coverage_tests/compare.rs:130`.
  - Record the time of the job in the task report.
  - Red test: none, because the task changes CI only. A temporary wrong pin in a branch run proves that the job fails.
  - Check: `deploy/ci/check.sh` and a CI run of the branch with the approval of the owner.

## 6. Checks of the whole change

- [ ] 6.1 (workspace, e2e) [mechanical] Run the local checks of `CONTRIBUTING.md`.
  - Red test: none, because the task writes no code.
  - Check: `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, `cargo fmt --all --check`, and `cargo deny check`.
  - Check: the `ste-lint.py` command of `CLAUDE.md` and `openspec validate parameter-page-memory --strict`.
  - Check: `cd e2e && npm ci && npm test`, with `E2E_PASSWORD` exported in the shell.

## Workflow follow-up

- After the merge, the paused change `value-body-memory-load` applies the section "Effect on the paused change" of `design.md`.
- Change B starts after task 5.11 and after the paused change reports a real `memory.peak`.
