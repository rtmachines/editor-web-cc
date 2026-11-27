# Tasks

Marks: a task without a mark is a Sonnet task. `[mechanical]` is a Haiku task. `[codex]` is a Codex task. Run every command from the root of the worktree `/home/rtm/editor-web-cc-limit-text-cap`. Each task takes at most 2 hours.

The tasks use the answers of the Decided Questions of `design.md` (owner, 2026-11-27). The cap is C below. C is 256 bytes.

## Order and shared files

- Task 1.1 comes first, because ADR-0225 is the authority of the code and of the other documents.
- Task 1.2 comes before tasks 2.2 and 2.3, because they use the row `form.limit_text_max` of §7.11.
- Task 2.1 comes before task 2.2, because task 2.2 uses the reason `LimitText` and adds a test to `reason_tests.rs`.
- Task 2.2 adds the constant `LIMIT_TEXT_MAX` with its first use in `place_limit`, so no build has an unused item. Tasks 2.4 and 2.5 use the constant and come after task 2.2.
- Tasks 2.2 and 2.3 change different files. They can run in parallel after tasks 1.2 and 2.1.
- Task 2.4 comes after task 2.2, because both add test files to `route_tests/`. Task 2.4 also writes its numbers into ADR-0225.
- Task 2.5 comes after task 2.2, because both change `settings/tests.rs`. Tasks 2.4 and 2.5 change different files and can run in parallel.
- Tasks 1.2 and 3.4 both change `docs/architecture/reliability.md`. Run them one after the other.
- Tasks 3.1, 3.2, and 3.3 run in the order 3.1 → 3.2 → 3.3.
  - Tasks 3.1 and 3.2 both change `deploy/ci/load_value.py` and `deploy/ci/test_load_value.py`.
  - The step of `ci.yml` in task 3.3 checks the fields of task 3.2.
- Group 3 does not depend on group 2. It can run in parallel with group 2 after task 1.1.
- Before task 4.1, the orchestrator pushes the branch and opens the pull request. Task 4.1 writes into its description and checks its job `heap`.
- Task 4.1 comes last. It needs every other task on the pushed branch.

## 1. ADR and the cap value

- [ ] 1.1 (docs/adr) Write `docs/adr/0225-limit-text-cap.md` in the shape of ADR-0222, with the decisions C1, F3, D1, and R1 of `design.md`.
  - First check that 0225 is still the next free number of `docs/adr/README.md`. If it is not free, stop and tell the owner. Do not take another number.
  - Name each amended item of `design.md`, section "Decisions", and give the new text of each item.
  - ADR-0138 (a) gets the reason text of the line. ADR-0138 (b) gets the over-long bound text as a fault of a changed body.
  - ADR-0138, Context line 19 and the decision driver of line 29: a user can type this one fault.
  - ADR-0138, Context line 21 and item (c), line 64: the value form and the limits form read the body before `row_version`. A fault of (b) or (c) comes first (`value.rs:188-189`, `:242-243`).
  - ADR-0211 (f): a typed bound text above the cap gets the line of C1. A body above the body limit of its route (`api.md` §7.6) keeps `413 E-WEB-003`.
  - ADR-0222 (a), lines 83-84: round 1 stays report only. ADR-0222 (c): the record of the largest read of round 2.
  - ADR-0222 (f): §7.8 keeps the measured values of run 45748785867 and adds the frame of D1. The Consequences line "Until change B" follows item (a).
  - ADR-0223 (w): D1 decides the memory frame, and the other items move to a later change. ADR-0223 (x) follows (w).
  - ADR-0223 lines 73-74 and 210 (Options E and F) move to a later change. Lines 295 and 297 (the sum and the parallel `GET` pages) move too.
  - ADR-0223 lines 161, 272, 281, and 299: move each item of change B to a later change or close it.
  - State the rule of the cap value of Decided Question 1 and its value C. State that §7.11 holds the value.
  - Name each kind of input that the cap refuses, from `design.md`, section "Risks / Trade-offs". Give the exact new text of the premise sentence of `api.md` §2.6 (line 229).
  - State the record window `RECORD_WINDOW_S` of 1 s and the barrier deadline as its upper end.
  - State that the barrier deadline after the first crossing ends the window with a pass.
  - State that a failed read in the window does not end it. The poll skips the read while the window has time left.
  - Add an empty table for the numbers of task 2.4.
  - Add the index row and the decision map entries to `docs/adr/README.md`. Add "Amended by ADR-0225" to the Status lines of ADR-0138, ADR-0211, ADR-0222, and ADR-0223.
  - Red test: none, because the task writes no code.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/0225-limit-text-cap.md docs/adr/README.md docs/adr/0138-*.md docs/adr/0211-*.md docs/adr/0222-*.md docs/adr/0223-*.md` gives 0 hard violations.
  - Check: `grep -c 'ADR-0225' docs/adr/0138-*.md docs/adr/0211-*.md docs/adr/0222-*.md docs/adr/0223-*.md` gives at least 1 for each file.
- [ ] 1.2 (docs/architecture, recipe-server) Add the row `form.limit_text_max` to `reliability.md` §7.11, with the value C of ADR-0225 and the unit `bytes`.
  - Red test: add the row first. The test `deploy_values` then fails, because `EXPLAINED` of `crates/recipe-server/tests/deploy_values/units.rs` has no word `bytes` for the key.
  - Add the pair `("form.limit_text_max", "bytes")` to `EXPLAINED`.
  - Do not add the code constant. Task 2.2 adds it with its first use.
  - Check: `cargo test -p recipe-server --all-features --locked --test deploy_values` passes.

## 2. C1: the cap in the reader and in the grid

- [ ] 2.1 (recipe-web) Add the reason `LimitText` in both bundles. Its text names the cap, for example "a limit field has more than 256 bytes. Make the text shorter".
  - Red test: insert `(FieldReason::LimitText, "reason-limit-text")` into `RECIPE_REASON_KEYS` of `crates/recipe-web/src/error/reason_tests.rs` directly after the pair of `LimitRow`. Change the array length from 14 to 15. The test fails, because the variant does not exist.
  - Do not append the pair at the end. The order test then fails, because `LimitRow` is at index 30 of 49.
  - Rename three tests that hold the old numbers. `reason_all_holds_the_14_recipe_reasons_at_20_to_33_in_the_order_of_design_d6` becomes the 15 reasons at 20 to 34.
  - `reason_all_holds_the_7_filter_reasons_at_34_to_40_in_the_order_of_design_d6` becomes 35 to 41. `values_field_reason_all_lists_49_reasons_with_the_plant_reasons_at_11_to_19` in `value_tests.rs` becomes 50 reasons.
  - Put `LimitText` directly after `LimitRow` in `FieldReason::ALL`. Add it to the English text of `reason.rs` and to `crates/recipe-web/i18n/en.ftl` and `pl.ftl`.
  - Update the count 49 → 50 in `crates/recipe-web/src/error/value_tests.rs:126-128`.
  - Update the ranges of `reason_tests.rs`. The recipe reasons change from 20..34 to 20..35 and from 14 to 15 (line 70).
  - The filter reasons change from 34..41 to 35..42 (line 91). The tail changes from 41.. to 42.. (line 157).
  - Check: `cargo test -p recipe-web --all-features --locked reason` passes.
- [ ] 2.2 (recipe-web) Refuse a bound text above the cap in `place_limit` (`fields.rs:359`). Count the decoded UTF-8 bytes before the trim.
  - Add `pub(crate) const LIMIT_TEXT_MAX: usize` to `crates/recipe-web/src/settings.rs`, with the value C of ADR-0225. It is no field of `WebSettings`. `place_limit` is its first use.
  - Red test: `limit_text_max_equals_the_table` in `crates/recipe-web/src/settings/tests.rs` reads the row `form.limit_text_max` of §7.11 and compares it with the constant. It fails, because the constant does not exist.
  - Red test: `limit_text_reason_names_the_cap` in `crates/recipe-web/src/error/reason_tests.rs`. The English text and the `pl` text of `LimitText` each hold the number `LIMIT_TEXT_MAX`.
  - Red tests in `crates/recipe-web/src/pages/parameters/fields_tests.rs`, each with a name that holds `limit_text`:
    - A `limits[0].min`, `max`, or `max_length` of C + 1 bytes gives `BadRequest` of `form` with `LimitText`. A text of exactly C bytes passes the reader.
    - A valid text such as `T#1s` or the `max_length` `5`, padded with spaces to C + 1 bytes, gives `LimitText`. A count after the trim lets it through.
    - A text of 2-byte characters with more than C bytes and fewer than C characters gives `LimitText`. A count of characters lets it through.
    - A run of `%FF` that decodes to more than C bytes gives `LimitText`.
  - Pin test, which passes before and after the task: `read_body` with C × `%31` passes, because it decodes to C bytes.
  - Red tests in a new file `crates/recipe-web/src/pages/parameters/route_tests/limit_text_tests.rs`, each with a name that holds `limit_text`:
    - An HTMX `/definition` save with a `[*]` row and a `min` of C + 1 bytes gives one line `422 E-WEB-008` with the label `Form` and the text of `LimitText`. The answer has no out-of-band form, `builds()` is 0, and the parameter does not change.
    - The same save without `HX-Request` gives `422` with one line, and `builds()` is 0.
    - The same body with `fields=1`, and the same body with a `limit_mode` button, give the same line, and `builds()` is 0.
    - An HTMX add request (`POST .../parameters`, `new.rs:170`) of `ARRAY[1..8] OF INT` with a `[*]` row and a `min` of C + 1 bytes gives one line of `LimitText`. `builds()` is 0, and the draft gets no parameter.
    - A `/limits` save with a `max_length` of C + 1 bytes gives the line of `LimitText`, not the line of `MaxLength`. The limits do not change.
    - A `/limits` save with a `min` of C + 1 bytes and a bad `row_version` gives one line of `LimitText` and no line of `row_version`.
  - Compatibility test, which passes before and after the task: a `/definition` save with a `min` of `1.` and zeros, C bytes long, stores the minimum 1.
  - Check: `cargo test -p recipe-web --all-features --locked limit_text` passes.
  - Check: `cargo test -p recipe-web --all-features --locked xss` and `cargo test -p recipe-web --all-features --locked value_leg` pass, because these tests send limit fields.
- [ ] 2.3 (docs/architecture) [mechanical] Copy the cap of ADR-0225 into `api.md` §2.6 and `CHANGELOG.md`.
  - `api.md` §2.6, line 229: replace the sentence "The HTML of the editor never sends such a body" with the exact text of ADR-0225. Add the limit bound text above `form.limit_text_max` bytes to the item.
  - `CHANGELOG.md`, section "Changed": the cap, and each kind of input of ADR-0225 that the server accepted before and now refuses.
  - Red test: none, because the task writes no code.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/architecture/api.md CHANGELOG.md` gives 0 hard violations.
- [ ] 2.4 (recipe-web) Measure the largest answer at the cap with two ignored `dhat` pins.
  - Add a new file `crates/recipe-web/src/pages/parameters/route_tests/largest_cap_tests.rs`, in the style of `largest_switch_tests.rs` and `measure_kit.rs`.
  - The type is `ARRAY[1..16384] OF INT`. A `[*]` row holds a `min`, a `max`, and a `max_length` of C bytes of `"` each. `moved_to` clones all three texts after the switch (`grid/limits.rs:121-128`).
  - Pin `largest_cap_all_htmx_pl`: the answer to `fields=1` in `[*]` mode. Pin `largest_cap_switch_htmx_pl`: the answer after the switch of `limit_mode` to the element rows.
  - Each pin records the length, the SHA-256, and the peaks. Write the numbers into the table of ADR-0225 as measurements of this fixture.
  - The names do not match a `--skip` filter of the job `heap`, so the shard `rest` runs the pins (`ci.yml:846-863`).
  - Red test: none, because a pin records a measurement. It cannot compile before task 2.2.
  - Check: `RUSTFLAGS='--cfg heap_measure' CARGO_TARGET_DIR=target/heap cargo test -p recipe-web --all-features --locked -- --ignored --test-threads=1 largest_cap` passes.
  - Record the time of the two pins. Tell the owner when they take more than 20 minutes, because the shard `rest` then nears its timeout of 70 minutes.
  - Tell the owner when the request peak of a pin is above the largest request peak of the table of ADR-0223.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/0225-limit-text-cap.md` gives 0 hard violations.

- [ ] 2.5 (recipe-web) Test the value C against the longest time literal and against the stored bounds.
  - Test `limit_text_max_follows_the_longest_time_literal` in `crates/recipe-web/src/settings/tests.rs`. For each time type and each platform of the type, it builds the longest literal of the rule of ADR-0225.
    - A duration literal has the long prefix and a sign where the type takes one. Each part of the type has 19 zeros.
    - An instant literal has the long prefix and the longest fraction of its type (`instant.rs:114-120`).
    - `DATE`, `LDATE`, and `DATE_AND_TIME` get no fraction. `TIME_OF_DAY` gets 3 digits. `LTOD`, `LDT`, and `DTL` get 9 digits.
    - `IecLiteral::parse` must accept each literal. A duration part with 20 digits must fail.
    - The test asserts `LIMIT_TEXT_MAX` equals the length of the longest literal, the `LTIME` duration of 150 bytes, rounded up to a power of two.
  - Test `stored_bound_text_fits_the_cap` in `crates/recipe-web/src/pages/parameters/grid/limit_tests.rs` renders stored bounds with `LimitTexts::from_stored`.
    - It covers both ends of each integer type, and `REAL` and `LREAL` bounds with their current digit limits and a scale of 28.
    - It also covers a stored `REAL` and `LREAL` bound with 28 significant digits.
    - It also covers both ends of each time type on each platform under both rule sets.
    - Each text must have at most `LIMIT_TEXT_MAX` bytes.
  - Red test: none can fail before the task, because task 2.2 sets the constant to C. Set the constant to 128 once, see the literal test fail, and restore C.
  - Check: `cargo test -p recipe-web --all-features --locked limit_text_max` and `cargo test -p recipe-web --all-features --locked fits_the_cap` pass.

## 3. F3 and D1: the value leg and the memory frame

- [ ] 3.1 (deploy/ci) [codex] Add the record window to the overlap poll of round 2 in `deploy/ci/load_value.py`.
  - Red tests in `deploy/ci/test_load_value.py`, with fake reads and a fake clock:
    - Round 2 reads the threshold, then the threshold + 4 096, then the threshold. The report holds `barrier_anon` at the threshold and `barrier_anon_max` at the threshold + 4 096.
    - With the barrier deadline 8 s away, the window ends 1 s after the first crossing. The barrier releases at that time. A read after the window end does not count in `barrier_anon_max`.
    - In that window, each read timeout and each sleep is at most the time to the window end.
    - A first read at the threshold 0.5 s before the barrier deadline ends the window at the deadline. The round passes with `barrier_anon` at the first read, and the barrier releases.
    - After the first crossing, a read that gives `None` does not end the window. A later, larger valid read sets `barrier_anon_max`. The round passes with no failure.
    - A read that stalls until the window end gives a pass with the largest earlier read.
    - A round 2 with valid reads below the threshold keeps the failure "the bodies did not overlap".
    - A round 2 with no valid read keeps the failure "round 2 barrier_anon is missing or invalid".
    - A round 2 with a request that missed the barrier keeps the failure "round 2: request LOAD-V1 did not reach the barrier".
    - In each of these three rounds, `barrier_anon_max` is `null`.
    - Round 1 releases the barrier at the first read, and its report holds `barrier_anon_max` as `null`.
  - Add the constant `RECORD_WINDOW_S` of 1.0 next to `BARRIER_TIMEOUT`. `overlap_anon` keeps the first read as `barrier_anon`.
  - The sender batch gets `barrier_anon_max` only. Task 3.2 computes `overlap_margin_max` in the checker.
  - Each read in the window gets the timeout of the earlier end: the window end or the barrier deadline.
  - Check: `python3 -I -B deploy/ci/test_load_value.py` passes.
- [ ] 3.2 (deploy/ci) [codex] Add `barrier_anon_max` and `overlap_margin_max` to the checker, `load-value.json`, and the job summary.
  - Red tests in `deploy/ci/test_load_value.py`:
    - With four unequal body lengths, the summary of round 2 shows `barrier_anon_max` and `overlap_margin_max`. `load-value.json` holds the exact margin. Round 1 shows `unavailable`.
    - A round 2 batch `load-value-answers-2.json` has a `barrier_anon` with an `overlap_margin` of 0 or more and no `barrier_anon_max`.
    - Then the written `load-value.json` and the summary hold a failure that names the missing value.
    - A round 2 batch with no crossing gives `null` for `overlap_margin_max` in `load-value.json`. Round 1 also gives `null`.
    - A round 2 batch with `barrier_anon_max` below `barrier_anon` gives a failure. The field does not exist before the task, so this test fails first.
  - Pin test, which passes before and after the task: a round 2 batch with no crossing gives one failure, "the bodies did not overlap".
  - Update the synthetic batch of `deploy/ci/test_load_replay.py`, so the archived runs stay green.
  - Check: `python3 -I -B deploy/ci/test_load_value.py` and `python3 -I -B deploy/ci/test_load_replay.py` pass.
- [ ] 3.3 (.github/workflows/ci.yml) [codex] Check the shape of the new fields in the step "Require the values from each leg" (`ci.yml:408-416`).
  - The step requires both keys in each round. Round 1 has `null`. Round 2 has `null` or a count for `barrier_anon_max`, and `null` or an integer for `overlap_margin_max`.
  - Red tests in `deploy/ci/test_load_workflow.py`:
    - A missing key fails the step. A count in round 1 fails the step.
  - Pin test: a round 2 with `null` for both fields passes the step, because the checker of task 3.2 owns the rule of the crossing.
  - Check: `python3 -I -B deploy/ci/test_load_workflow.py` and `bash deploy/ci/check.sh` pass.
- [ ] 3.4 (docs/architecture) State the memory frame D1 in `reliability.md` §7.8.
  - Keep the table of run 45748785867 and its text with the body length, the run statistic, and 3 × the body length (ADR-0222 (f)).
  - Add the peak table of the three runs of `design.md`, section "D1: the measured runs".
  - State the frame: the `memory.peak` reading of each round against `soak.memory_ceiling`. The `dhat` pins of ADR-0223 bound one answer each, and no sum adds them.
  - State the worst round 1 margins with the run and the build of each reading.
    - The musl margin is 19 980 288 bytes, from run 45768560058 with mutation B.
    - The `cc-debian13` margin is 151 552 bytes, from run 45574974451 on the branch `diag/musl-round2`.
  - State the overlap margins of run 45805398657 and the known limit: no run measures parallel `GET` pages.
  - Check each number against `load-value.json` of its run before you write it.
  - Red test: none, because the task writes no code.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/architecture/reliability.md` gives 0 hard violations.

## 4. Evidence

- [ ] 4.1 (deploy/ci) [mechanical] Dispatch the job `load` on the pushed branch and record the result.
  - Red test: none, because the task writes no code. The jobs themselves are the test.
  - Record `passed`, `failures`, the peak of each round, `barrier_anon_max`, and `overlap_margin_max` of round 2 of both legs in the PR description.
  - The step `value-check` only reports on `cc-debian13` (`ci.yml:335`), so a green job does not show a pass. Record the verdict of each artifact.
  - Tell the owner about each leg with `passed: false` or a failure.
  - A peak that is `null` gives no comparison. Record it as a missing measurement.
  - Compare the round 2 peak of each leg with the table of task 3.4. Tell the owner when it is above the largest round 2 peak of its leg in that table.
  - These peaks are 269 119 488 bytes on musl and 293 281 792 bytes on `cc-debian13`.
  - Check: `gh run view <run-id> --json jobs` gives `success` for both matrix jobs of `load`.
  - Check: `gh run download <run-id>` gives `load-value.json` of each leg. Each file holds `passed`, `failures`, the peaks, and the two new values of round 2.
  - Check: in each leg with an `overlap_margin` of 0 or more in round 2, both new values of round 2 are numbers.
  - Check: every shard of the job `heap` of the pull request is green. This proves that the cap changes no pin of ADR-0223 and that the pins of task 2.4 pass.

## Workflow follow-up

- The owner decided the questions of `design.md` on 2026-11-27.
- A later change reviews the threshold of the overlap check. The data of task 4.1 is evidence for it. That change also needs data for three bodies and more runs.
- The next change makes the row model smaller (`GridView`). Then it plans the budget (D4), the write check (D5), A54, a bound on parallel `GET` pages, and Options E and F of ADR-0223.
- `machine-link` takes the next free ADR number at apply, because `main` holds ADR-0224 and this change takes ADR-0225.
