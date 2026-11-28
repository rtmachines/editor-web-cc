# Tasks

Marks: a task without a mark is a Sonnet task. `[mechanical]` is a Haiku task. Run every command from the root of the worktree. Each task takes at most 2 hours.

Terms of this file:

- The heap build is `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked --no-run`.
- The heap suite runs the five shards of the CI job `heap` (`.github/workflows/ci.yml:851-866`) as five parallel processes.
  - Each shard runs `RUSTFLAGS="--cfg heap_measure" CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked -- --ignored --test-threads=1 <args>`.
  - `<args>` are the arguments of the shard in `ci.yml`. Each shard writes its output into `target/heap/<shard>.log`.
- A red run uses the heap build and names each test with `--exact`, for example `pages::parameters::route_tests::largest_measure_tests::largest_page_en`.
- The baseline of a task is the pin values before its red step (`design.md` D7).
- A pin update follows `design.md` D7:
  - Set each `request_peak` and `render_peak` of the seven pin files whose measured value is below its baseline to that measured value.
  - Replace each red value with its measured value. A fall of a request peak of at most 1 864 B is noise, so that pin stays.
  - A pin never rises. Report each measured rise. A failing heap test stops the task, and the task reports it.
  - A task never changes a pinned length or SHA-256. A changed length or SHA-256 stops the task, and the task reports it.
  - A pin is thus a reference value. The latest measured value can differ from it by noise or by a rise inside `PEAK_SLACK` (`design.md` D7).
- The heap suite run after the code change is the heap check of a task. Do not run the suite again after the pin update.
- The code check is `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, `cargo test --workspace --all-features --locked`, and `cargo fmt --all --check`.
- Every task ends with the code check, as the apply guidance of `openspec/config.yaml` requires. A measurement task and a docs task also run it.

## Order and shared files

- Task 1.1 comes first, because ADR-0226 is the authority of the code. Task 1.2 comes next, because it gives the baseline and the time budget.
- The code tasks run in the order of `design.md`, section "Order". They do not run in parallel.
  - Each code task measures the whole heap suite and changes the same seven pin files.
  - Each red pin of a task depends on the peaks that the tasks before it leave.
- Tasks 2.1, 2.3, and 2.4 change `grid/limits.rs`. Task 2.3 changes `ValueTexts::from_stored` there. Tasks 2.3, 2.4, and 3.2 change `form_view.rs`.
- Tasks 2.3 and 2.4 both change `components_tests/grid.rs`, `grid_tests.rs`, and `grid/limit_tests.rs`. Task 3.2 also changes `fields.rs`.
- Tasks 2.2 and 3.5 both change `prepare.rs` and the copy in `largest_refused_tests.rs:127`.
  - Task 2.2 adds the refusal step to `prepare.rs` (`design.md` D2). Task 3.5 changes that step.
- Tasks 3.1 to 3.4 all change `grid.rs`, `grid/walk.rs`, and `grid_tests.rs`. Tasks 3.2 to 3.4 also change `components/grid.html`.
  - Run them in the order 3.1 → 3.2 → 3.3 → 3.4. Tasks 3.1 to 3.4 add their `size_of` tests to `grid/size_tests.rs`.
- A code task that does not fit the budget of task 1.2 splits into a code task and a measurement task (`design.md` D7).
  - The code task ends after the green run of its red tests and the code check.
  - Do not push the branch between the two halves, because the red pins fail the CI job `heap`.
- Task 3.5 runs after task 3.4. The owner accepted C2e (`design.md`, Decided Question 1).
- Task 4.1 comes last. It needs every other task. It writes the measured table into ADR-0226.

## 1. ADR and baseline

- [ ] 1.1 (docs/adr) Write `docs/adr/0226-memory-of-the-grid-build.md` in the shape of ADR-0223. It holds the decisions D1 to D9 of `design.md`.
  - No red test: the task changes no code. The `Check:` commands below check it.
  - First check that 0226 is still the next free number of `docs/adr/README.md`. If it is not free, stop and tell the owner.
  - Set the status to "Proposed". Give each gain as in `design.md`, measured or marked as an estimate.
  - Name each amended item of ADR-0223: Context line 43, and the section "Measured numbers" with its table and lines 266 to 271.
  - State that line 272 of ADR-0223 stays, as ADR-0225 (r) says. ADR-0226 records only new measured values of the save request peaks.
  - Write the sentences of `design.md` D8 on the size of the grid as facts of cdca4d2.
  - State that ADR-0223 (v) stays a record of the old render peaks (ADR-0225 (m)). Give the new peaks in a table of ADR-0226.
  - State that the table is for the route fixture of 16 384 leaves. Round 1 sends a body of 4 096 leaves (ADR-0222 (c)).
  - Name each amended item of ADR-0225: Context line 16 ("the later change" is the change after `grid-build-memory`) and item (t).
  - State that the new meaning of "the later change" also applies to ADR-0225 (r).
  - Name the amended item of ADR-0222: Context line 20. When the owner accepted task 3.5, also name lines 124 to 125 and line 208.
  - State that ADR-0223 (k), (p), (q), (t) and ADR-0039 (l), (n) keep their text.
  - Add the note "Amended by ADR-0226" to the status lines of ADR-0222, ADR-0223, and ADR-0225. Change nothing else in them.
  - Add the index row of ADR-0226. Add the note to the rows of 0222, 0223, and 0225.
  - Add 0226 to the decision map rows "Frontend", "Pages of recipes, versions, and parameters", and "Test gates".
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/0226-memory-of-the-grid-build.md docs/adr/README.md docs/adr/0222-memory-of-the-value-bodies-under-load.md docs/adr/0223-memory-of-the-grid-answers.md docs/adr/0225-limit-text-cap.md` reports 0 hard violations.
  - Check: `grep -c 'ADR-0226' docs/adr/0222-memory-of-the-value-bodies-under-load.md docs/adr/0223-memory-of-the-grid-answers.md docs/adr/0225-limit-text-cap.md` gives 1 for each file. The code check passes.
- [ ] 1.2 (recipe-web) [mechanical] Measure the baseline and the time of each step on the branch of task 1.1.
  - No red test: the task changes no code. The heap suite is its check.
  - Run the heap build. Then run the heap suite, and record the time of each shard.
  - Record one line for each of the 26 answers in `target/heap/baseline.txt`. The line holds the two pin constants and the two measured peaks.
  - Run the red run of `largest_page_en` and `largest_refused_page_en` once, and record the time of the full command.
  - Run the code check once, and record its time.
  - Compute the budget of each code task as `design.md` D7 says. Report each code task that does not fit 120 minutes.
  - When a code task does not fit, stop. `tasks.md` then gets the split of `design.md` D7 before task 2.1 starts.
  - Check: the heap suite passes, and `target/heap/baseline.txt` holds 26 lines.

## 2. One copy of each input

- [ ] 2.1 (recipe-web) C1: `LimitSet::new` keeps no `index_levels` set and buffers no type levels (`design.md` D1).
  - Red: set the `render_peak` of `largest_page_en`, `largest_page_pl`, `largest_page_fragment_pl`, and `largest_read_page_pl` to 72 040 843.
  - Red: set the `render_peak` of `largest_refused_page_en`, `largest_refused_page_pl`, `largest_refused_htmx_en`, and `largest_refused_htmx_pl` to 126 708 733.
  - Red run: `largest_page_en` and `largest_refused_page_en`. Both fail.
  - Guard: add cases to `grid/index_level_tests.rs` on trees that resolve their paths (`design.md` D1). They pass before and after.
  - The cases cover an index `[-1]`, the members `.a` and `.ab`, and an index `[1,2]`. Each case holds a blank row `[*]` next to its index row.
  - Guard: add a case with bound rows `[*]` that repeat one type level at one array level. A bound index row sits at another array level of it.
  - The case asserts that `is_mixed` is true for that type level. It passes before and after.
  - Change `limits.rs` as `design.md` D1 says. Rewrite the doc comments at `limits.rs:153-154`, `:157`, `:301`, and `:617`.
  - `mixed_type_levels` inserts the set `all` with `extend` (`limits.rs:244-249`). This site has no red step (`design.md` D1).
  - Run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 2.2 (recipe-web) C2a: a refusal step keeps the flag of the limit list, then drops `value` and `limits` before it calls `refuse_checked` (`design.md` D2).
  - Red: set the `render_peak` of the four round 1 pins of `largest_refused_tests.rs` to 64 784 637.
  - Red: set the `request_peak` of the same four pins to their baseline minus 30 000 000 (estimate, `design.md` D2).
  - Red run: `largest_refused_page_en`. It fails.
  - Change `prepare.rs:148-152`: add the refusal step of `design.md` D2, a sync function that takes `value` and `limits` by value.
  - The step keeps `limits.is_err()` in `limit_error`, drops `value` and `limits`, and calls `refuse_checked` with `limit_error`. `ready` calls the step.
  - The copy of `largest_refused_tests.rs:127` calls the step in place of `sent_shown`, `refuse`, and its `drop((value, limits))`.
  - Then run the heap suite and do the pin update.
  - A request pin that fails shows that `ready` keeps the texts. Stop and report it.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked value_leg_refusal_order` passes.
  - Check: `cargo test -p recipe-web --all-features --locked grid_builds_one_for_a_bad_maximum_length_in_the_summary` passes. The code check passes.
- [ ] 2.3 (recipe-web) C2b: `GridTexts::stored` drops the `ParameterValue` after its leaf texts, and moves the leaf texts into `ValueTexts` (`design.md` D3).
  - Red: set the `render_peak` of `largest_page_en`, `largest_page_pl`, `largest_page_fragment_pl`, and `largest_read_page_pl` to 66 000 000.
  - Red run: `largest_page_en`. It fails at 72 040 843.
  - Red: add a unit test in `grid/limit_tests.rs`. For each nonempty text, it asserts that `get(path).as_ptr()` equals the pointer of the moved `LeafText.text`.
  - The red pin cannot find a kept copy of the texts, so the unit test checks the move (`design.md` D3). Today the clone fails it.
  - Change `form_view.rs:116-124`. `ValueTexts::from_stored` in `grid/limits.rs:24-32` takes the `Vec<LeafText>` by value.
  - Change its four test callers (`components_tests/grid.rs:68`, `:430`, `grid_tests.rs:43`, `:251`) to pass the vector by value.
  - Then run the heap suite and do the pin update. No pin measures the stored phase, so its gain stays an estimate.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked form_view` passes. The code check passes.
- [ ] 2.4 (recipe-web) C2c: `LimitTexts` holds the canonical text of its path, not a `LeafPath` (`design.md` D4).
  - Red: add an ignored `peak_in` test in `grid/limit_tests.rs` in the shape of `values_body_tests.rs:116`.
  - The test builds 1 024 `SentLimit` rows with empty bounds and paths of 32 segments and 128 B before `peak_in`.
  - Inside `peak_in`, it calls `LimitTexts::from_sent`. It asserts a peak of at most 1 024 × 256 B.
  - It also asserts 1 024 result rows, and that the path text of each row equals the `Display` of its sent path.
  - Today each row holds a `LeafPath` of 1 152 B, so the test fails with the heap build.
  - Change the sites of `design.md` D4, with the nine test files. `LimitSet::new` keys each row by the `Display` text of its parsed path.
  - Guard: add a case in `grid/limit_tests.rs`. A literal row with the path text `[01]` goes to `LimitSet::new` on a tree that resolves `[1]`.
  - The case asserts that `get("[1]")` returns the row, with the path text `[1]`. It does not use `from_sent`.
  - Then run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 2.5 (recipe-domain) C2d: `extended` in `plant/leaf_path.rs:23` allocates the exact length (`design.md` D5).
  - Red: add a unit test in `leaf_path.rs`. It extends a path of 31 segments and asserts `capacity() == len()`. Today the capacity is 62.
  - The comment of the test names its dependency on the exact allocation of `collect` (`design.md` D5).
  - Change `extended`. Then run the heap suite and do the pin update.
  - Check: `cargo test -p recipe-domain --all-features --locked` passes. The heap check passes. The code check passes.

## 3. Compact rows

- [ ] 3.1 (recipe-web) C3a part 1: `GridRow::Mode` and `GridRow::Toggle` hold a `Box` (`design.md` D6, group 1).
  - Red: add `grid/size_tests.rs`, a child test module of `grid`. It asserts `size_of::<GridRow>() <= 16`. Today it is 120.
  - Box the toggle row too. Change `grid.rs:109-141`, `walk.rs:297`, `walk.rs:337`, `opened` in `grid.rs:449-457`, and the tests.
  - `leaf`, `mode`, and `toggle` keep their signatures, so no template changes. Then run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 3.2 (recipe-web) C3a part 2: the input ids become a `Copy` type `InputId` with a `Display` view (`design.md` D6, group 2).
  - Red: add unit tests for the `Display` text of each id kind. Assert `size_of::<LeafIds>() <= 64` in `grid/size_tests.rs`. Today it is 96.
  - Guard: add a case for `inputs_of_field` on a `[*]` fault over at least eleven element inputs. The limit index starts at 0 (`walk.rs:82-86`).
  - The case asserts that `limits-2-min` comes before `limits-10-min`, so a plain sort on the text fails. It passes before and after.
  - Change the sites of `design.md` D6, group 2, with `fields.rs:633-650` and `form_view.rs:248-252`.
  - Keep the sort of `inputs_of_field` on the id text, the first id of `note`, and the collision check of `fields.rs:645`.
  - Change `components/grid.html` lines 9 to 23. `test/grid.html` and `test/grid-form.html` stay. Then run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 3.3 (recipe-web) C3a part 3: the template writes the names and row ids from numbers and the path. A row holds one path (`design.md` D6, group 3).
  - Red: assert `size_of::<LeafRow>() <= 400` in `grid/size_tests.rs`. Today it is 584.
  - The bound is an estimate. If the compact row misses it by at most 32 B, record the real size in the test and report it.
  - Add `root_label` to `GridView`. `LeafRow.path` holds the shown path, and the row header writes the root label for the empty path.
  - A row that owns limit inputs keeps its limit index, also a row `[*]` without a value path (`design.md` D6, group 3).
  - `grid_component_a_row_all_names_the_cell_every_element_in_its_labels` (`components_tests/grid.rs:180`) passes without a change.
  - Change `walk.rs:368-376`, `:402`, `:470`, `:490-493`, the structs of `grid.rs`, and the templates. Then run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 3.4 (recipe-web) C3a part 4: the hints and the type become `Copy` views (`design.md` D6, group 4).
  - Red: change the bound of task 3.3 to `size_of::<LeafRow>() <= 288`. The rule of task 3.3 for a miss of at most 32 B applies.
  - Change `grid/texts.rs:9-75`, `walk.rs:415-431`, the structs of `grid.rs`, and the templates. Then run the heap suite and do the pin update.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked grid` passes. The code check passes.
- [ ] 3.5 (recipe-web) C2e: `ready` drops `value` before it calls `limit_list` when a line exists.
  - Red: set the `render_peak` of the four round 1 pins of `largest_refused_tests.rs` to the target, 60 959 672.
  - Red run: `largest_refused_page_en`. It fails at 62 245 184 or more, by about 236 936 B (`design.md`, Decided Question 1).
  - A red run that passes shows that the target is met already. Then stop and report.
  - Change the refusal step of task 2.2. It takes `value` by value. When a line exists, it drops `value` and then calls `limit_list` itself.
  - `ready` and the copy of `largest_refused_tests.rs:127` still call the step. The copy stops calling `limit_list` (`largest_refused_tests.rs:156`).
  - Then run the heap suite and do the pin update.
  - The request peak of `largest_refused_page_en` must fall by more than 1 864 B. Otherwise stop and report, because the router path then keeps the texts.
  - Check: the heap check passes. `cargo test -p recipe-web --all-features --locked value_leg_refusal_order` passes. The code check passes.

## 4. Targets

- [ ] 4.1 (docs/adr) Measure both targets and record the result in ADR-0226.
  - No red test: the task changes no code. The heap suite and the comparisons below are its checks.
  - Run the heap suite on the branch with every other task. Take each raw measured render peak from the shard logs, without `PEAK_SLACK`.
  - Compare the largest render peak of `largest_page_en`, `largest_page_pl`, and `largest_page_fragment_pl` with 49 653 707.
  - Compare the largest render peak of the four round 1 answers of `largest_refused_tests.rs` with 60 959 672.
  - Write the table of all 26 answers into ADR-0226, section "Measured numbers".
  - The columns are answer, status, length, request peak, and render peak. The table has no column "Sum of change A" (`design.md` D8).
  - Compare each of the 51 pin constants with the shard logs under the reference rule of `design.md` D7. Record each difference.
  - Give the upper bound of the held grid: the render peak minus the page length of `largest_page_en`. State whether the risk of ADR-0223 line 298 still holds.
  - Write the final line numbers of `ready` in `prepare.rs` into ADR-0226, in the items that amend ADR-0222.
  - Add one line under "Changed" in `CHANGELOG.md`, in both cases below (`design.md`, Decided Question 6).
  - Both targets met: set the status of ADR-0226 to "Accepted".
  - A target missed: keep the status "Proposed". Write the miss into ADR-0226. Stop the plan before the code review.
  - The owner then decides whether the code merges, and decides option (c) of A26 (`design.md`, "Risks / Trade-offs").
  - On a merge after a miss, set the status to "Accepted". ADR-0226 keeps the recorded miss.
  - Without a merge, ADR-0226 stays "Proposed". Remove the three notes "Amended by ADR-0226" from the branch before a later merge (`design.md` D8).
  - Check: the heap suite passes. `python -I tools/ste-lint/ste-lint.py docs/adr/0226-memory-of-the-grid-build.md CHANGELOG.md` reports 0 hard violations. The code check passes.

## Workflow follow-up

- Code review with `code-review`, then `superpowers:verification-before-completion`, then `openspec-archive-change`.
- The plan `machine-link` takes ADR-0227 at its apply (`design.md`, section "Risks / Trade-offs").
