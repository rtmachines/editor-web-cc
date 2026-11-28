# Design

## Context

ADR-0223 pins the length, the SHA-256, and the peak heap of each grid answer of the largest valid parameter of A26. Item (t) of ADR-0223 gives the method. ADR-0225 (t) adds two pins at the cap of a bound text.

The attribution memo of 2026-11-27 measured each render interval with `dhat` and heap marks. Its logs are outside the repository, in `~/editor-web-cc-archive/change-o4/measure/`. This design checked each number below in those logs.

Terms of this design:

- A phase is a part of a render interval between two heap marks.
- "Measured" means a value of a log of the memo. "Estimate" means a sum of measured sites, not a run.
- The three profiled answers are the `GET` page `en`, the read-only page `pl`, and the refused round 1 page `en`.
- B is bytes and M is 10^6 B.

### Measured phases of the render interval

| Answer and code | Phase | Peak (B) | Source |
|---|---|---|---|
| `GET` `en`, today | build of the grid, in `LimitSet::new` | 106 201 040 | `o4_get_page_en.log` |
| `GET` `en`, with C1 | `GridTexts::stored` | 72 040 843 | `p1_get_marks.log` mark 1 |
| `GET` `en`, with C1 | render: view and HTML held | 54 149 256 (held after the step) | `p1_get_marks.log` mark 7 |
| `GET` `en`, with C1 | build of the grid above the inputs | 35 684 144 | `p1_parts.log` mark B |
| refused `en`, today | build of the grid, in `LimitSet::new` | 173 268 309 | `o4_refused_page_en.log` |
| refused `en`, with C1 | end of `grid_of` | 126 708 733 | `p1_refused_marks.log` |
| refused `en`, with C1 and C2a | end of `grid_of` | 64 784 637 | `p1c3_refused_marks.log` mark 3c |
| refused `en`, with C1 and C2a | leaf texts and limit list held together | 62 245 184 | `p1c3_refused_marks.log` mark 2 |
| refused `en`, with C1 and C2a | page of the error layer | 55 416 520 (held after the step) | `p1c3_refused_marks.log` mark 6 |

- The `GET` inputs hold 24 987 808 B before `grid_of` (`p1_get_marks.log` mark 1). The build with C1 thus peaks at most at about 60 672 000 B (upper estimate: 24 987 808 + 35 684 144).
  - The sum is an upper estimate. `LimitSet::new` consumes its input vector before the peak of mark B, so a part of the inputs is free at that moment.
- The refused phase of mark 2 lies before the grid. C1, C2a to C2d, and C3a do not change it. Decided Question 1 gives the consequence.
- At the peaks of the three profiled answers on cdca4d2, the row model holds 0 B.
- On cdca4d2, the built grid of the `GET` holds 31 574 502 B (`p1_parts.log` mark B). This is more than the 22 569 867 B of its HTML.

### Rules that hold for every part

- The HTML stays byte-identical. ADR-0223 (q) and the route tests pin each length and SHA-256.
- The `id` lookup of the line under each input stays (ADR-0039 (n)). The inputs of a closed group stay in the form (ADR-0039 (l)).
- The lints of `reliability.md` §2 hold. The pin tests copy the handler steps (`largest_measure_tests.rs:101`, `:253`, `largest_refused_tests.rs:127`).
  - After task 2.2, the copy of `largest_refused_tests.rs:127` calls the refusal step of `ready` and copies it no longer (D2).
- A pin never rises. Round 1 of the value leg stays a report (ADR-0225).

## Goals / Non-Goals

**Goals:**

- Measure a `GET` render peak of at most 49 653 707 B. Measure a refused round 1 render peak of at most 60 959 672 B.
- Keep each answer byte for byte.
- Pin each lowered peak at its new measured value, as ADR-0223 (t) requires.

**Non-Goals:**

- The budget `form.grid_memory`, the write check with `E-WEB-018`, A54, and a bound on parallel `GET` pages. The next change plans them on the new peaks.
- A text path in `SentFields` (`fields.rs:118`), and the two page copies of the error layer.
- A change of A26 or ADR-0211, and a gate on round 1 (ADR-0225).

## Decisions

### D1. C1: level sets without a string for each prefix

- Sites: `limits.rs:155-174` (`LimitSet`), `:184-187` (`new`), `:244-249` (`mixed_type_levels`), `:302-305` (`refresh_levels`), `:318-324` (`has_index_at`), `:618-623` (`index_levels`), `:642-647` (`each_type_levels`).
- `LimitSet` drops the field `index_levels`. `has_index_at` asks the sorted `rows` for the first key at or after `<prefix>[+`.
- It returns true when that key starts with `<prefix>`, then an index segment.
- An index segment starts with `-` or a digit. Both sort after `*` and `+`, so the query skips the keys of `[*]`.
- The query is one `BTreeMap::range` lookup, with no scan. A forged form thus keeps the rule of the doc comment of `LimitSet` (`limits.rs:153-154`).
- The query relies on canonical keys. `LimitSet::new` keys each row by the `Display` text of a parsed path, so each key is canonical. D4 keeps this rule.
- `each_type_levels` inserts each type level into its set at once. It buffers no `Vec`.
- `mixed_type_levels` collects the `[*]` type levels of each bound row into the set `all` with `collect` (`limits.rs:244-249`).
  - `collect` into a `BTreeSet` buffers every item before it removes the duplicates, as the comment of `limits.rs:234-235` says.
  - C1 inserts these levels one at a time with `extend`, as `limits.rs:236-243` does for the set `each`.
  - None of the four peak profiles shows this site. Its gain on the fixture is thus 0 B (estimate). The measured gains below stay valid.
- Four doc comments name the dropped set: `limits.rs:153-154`, `:157`, `:301`, and `:617`. Task 2.1 rewrites them for the range query.
- Gain, measured: `GET` 106 201 040 → 72 040 843 B. Refused 173 268 309 → 126 708 733 B. The patch is `measure/temp-P1-limits.diff`.
- Risk: low to medium. A wrong query changes a mode row, and the SHA-256 pins find it.
- Guard cases: `LimitSet::new` drops each row that its tree does not resolve. `assert_modes` (`index_level_tests.rs:88`) uses `nested_tree`, with positive one-dimensional indexes and the members `x`, `w`, and `v`.
  - The guard cases thus build their own trees with `Kit` of `tree_kit.rs`. The trees hold an array with a negative lower bound, a two-dimensional array, and the members `a` and `ab`.
  - Each guard case also holds a blank row `[*]` next to its index row. The query must then skip the key of `[*]`.
  - One more guard case holds bound rows `[*]` that repeat one type level, and a bound index row at another array level of it. `is_mixed` stays true.
  - That case has no red step. The buffer of `all` is private to `limits.rs`, and a test of `grid` measures only all of `LimitSet::new`, where the row map is larger.

### D2. C2a: the refused save drops its texts before the refused form

- Site: `ready` in `prepare.rs:148-152`.
- Task 2.2 moves the refusal of `ready` into one sync function of `prepare.rs`, the refusal step. It takes `value` and `limits` by value.
- The refusal step keeps `limits.is_err()` in a local `limit_error`. Then it drops `value` and `limits` and calls `refuse_checked` with `limit_error`.
- `refuse_checked` reads neither value. It reads only the flag, and builds the lines of the bad limit rows again.
- Without the flag, the refused form loses the lines of the bad limit rows. `value_leg_refusal_order` and `grid_builds_one_for_a_bad_maximum_length_in_the_summary` find that loss, because each summary must link `#limits-0-max_length`.
- `ready` and the copy in `largest_refused_tests.rs:127` both call the refusal step. The render pins thus measure the drops of `ready`, not a second copy of them.
  - `shape_tests.rs:23` uses the same copy, so its refused `STRING(254)` form also runs the refusal step.
- The request pins run `ready` through the router. They stay an end-to-end check. Task 2.2 lowers the request pins of round 1 by 30 000 000 B in its red step.
  - That fall is an estimate: half of the measured render gain of C2a. A smaller fall fails the heap test, so the task stops.
  - A fall of the request pins alone does not prove both drops. The leaf texts alone hold 40 297 344 B (`p1c3_refused_marks.log` mark 1). The shared refusal step gives that proof.
- Gain, measured with C1: refused 126 708 733 → 64 784 637 B.
- C2a moves the refusal of `ready` from lines 150 to 152 to later lines. Lines 148 and 149 stay. ADR-0222 Context line 20 cites the moved lines (D8).
- Risk: low.

### D3. C2b: `GridTexts::stored` drops its inputs before the limit texts

- Sites: `form_view.rs:116-124`, and `ValueTexts::from_stored` in `grid/limits.rs:24-32`.
- The function builds the leaf texts in its own statement. It drops the `ParameterValue` after `value.texts(tree)?` and before the value texts exist.
- `ValueTexts::from_stored` then takes the `Vec<LeafText>` by value. It moves each text, as `StoredParts::of` does (`form_view.rs:206-211`). Then `LimitTexts::from_stored` runs.
- The four test callers (`components_tests/grid.rs:68`, `:430`, `grid_tests.rs:43`, `:251`) pass the vector by value. No function stays without a caller, so the clippy gate finds no dead code.
- Gain, estimate: the stored phase falls from 72 040 843 to about 47 052 960 B.
  - Method: the sites of `measure/p1_parts_A.json` at the end of `value.texts(tree)`.
  - The `ParameterValue` holds 10 796 992 B: 10 485 696 B at `stored.rs:58` and 311 296 B of time numbers at `leaf.rs:315`.
  - The leaf paths hold 32 505 856 B. Their index blocks hold 2 031 616 B, and the index vectors of `leaves.rs:99` hold 262 144 B.
  - The `Vec<LeafText>` holds 917 504 B, and the leaf texts hold 538 848 B (`texts.rs:337`).
- That phase needs the `ParameterValue`, so the early drop does not lower it.
- The early drop frees 10 796 992 B (sum of the measured sites) before the limit texts. It lowers only the phase of the limit texts.
  - That phase falls from about 35.8 M to about 25.0 M (estimate). Mark A holds 24 987 808 B after the step (`p1_parts.log`).
  - That phase is below the build, so the early drop does not lower a peak. No pin measures it.
- No pin measures the stored phase after C2b, because the build and the render peak higher. Its value and the gain thus stay estimates.
- The memo gives about 49 973 419 B. Its number keeps the copy into the value texts. The by-value move frees each `LeafPath` when it writes its key.
- The red pin of 66 000 000 B cannot find a kept copy of the texts, because the memo value is also below it.
  - A unit test thus checks the move. For each nonempty text, `get(path).as_ptr()` of the `ValueTexts` equals the pointer of the moved `LeafText.text`.
- After C2b, the `GET` and the read-only page peak in the build at most at about 60 672 000 B (upper estimate). The build of both answers runs the same `grid_of`.
- Risk: low.

### D4. C2c: a limit row holds the text of its path

- Sites: `LimitTexts` in `limits.rs:52-61`, `from_stored` at `:89`, `from_sent` at `:105`, `moved_to` at `:121-128`, `blank_row_at` at `:132-141`, `new` at `:181-182`.
- Other sites: `StoredParts::text_of` (`form_view.rs:229`) and `GridLimitsFragment` (`grid.rs:412`, `grid-limits.html`).
- Test sites: nine files build a `LimitTexts` literal. They are `components_tests/grid.rs`, `big_grid_kit.rs`, `nested_group_tests.rs:135`, and `values_body_tests.rs:66`.
  - The other five are `grid_tests.rs`, `grid/fault_tests.rs`, `grid/index_level_tests.rs`, `grid/limit_tests.rs`, and `grid/type_level_tests.rs`.
- `path` becomes a `String`. `LimitSet::new` parses each text for its check against the tree.
  - It keys the row by the `Display` text of the parsed path and writes that text into the row. Then it drops the parsed path.
  - A literal of a test or a later constructor thus cannot put a key that is not canonical into the set.
  - A guard case gives a literal with the text `[01]` to `LimitSet::new` on a tree that resolves it. `get("[1]")` then returns the row, with the path text `[1]`.
  - `from_sent` alone cannot show this rule, because its typed input already writes canonical text.
- Gain, estimate: 16 777 216 B on each grid input, `GET` and refused. Method: the clone sites at `limits.rs:89` and `:105`.
- After C2c, the `GET` peaks in the render at about 54 149 256 B (view and HTML).
- Risk: low to medium. The texts of `grid-limits.html` stay the same, because the canonical text equals the `Display` of the path.

### D5. C2d: `extended` allocates the exact length

- Site: `recipe-domain`, `plant/leaf_path.rs:23-27`.
- `extended` collects `path.iter().cloned().chain(once(segment))`. The vector then has the capacity of its length, with no doubling.
- The callers are the walks of `leaf_path/expand.rs`, `type_tree/leaves.rs`, `rebind/build.rs`, `rebind/value.rs`, `value/leaves.rs`, and `value/walk.rs`.
- The red test asserts `capacity() == len()`. Its comment names the dependency: `collect` of a `TrustedLen` chain allocates the exact length.
  - `std` documents an exact capacity only for `Vec::with_capacity`. A later `std` can thus fail the test without a change of `extended`.
- Gain, estimate: 15 728 640 B in the stored phase, 960 B of spare capacity for each of 16 384 paths. The stored phase then holds about 31.3 M (estimate).
- The gain reaches the `GET` peak only after C3a. Before C3a, the render phase is larger than the stored phase.
- Risk: low. The order and the content of each path stay the same.

### D6. C3a: compact leaf rows

The row model changes in four field groups. Askama compiles each template against the fields, so each group changes its struct, its template lines, and its tests in one task.

1. The mode row and the toggle row: `GridRow::Mode` holds a `Box<ModeRow>`, and `GridRow::Toggle` holds a `Box<ToggleRow>` (`grid.rs:109-116`, `:238-245`).
   - Both kinds are rare. A row then has 16 B. The `Vec<GridRow>` buffer falls from 3 932 160 B to about 524 288 B (estimate).
   - The memo boxes the mode row only. A `ToggleRow` of about 56 B then stays inline, so a row keeps about 56 B.
   - `GridRow::leaf`, `mode`, and `toggle` keep their signatures. The templates read the rows through them, so no template changes in this group.
2. The input ids: a `Copy` type `InputId` names the value input `value-<n>` and the limit input `limits-<i>-<field>`. Its `Display` writes the text of today.
   - `ValueCell.id`, `InputCell.id`, and `LeafIds` hold `InputId` (`grid.rs:75-80`, `:188-213`).
   - `limit_input_id` (`grid.rs:86`) writes through `InputId`, so a line and its input still cannot differ.
   - `input_id` and `input_of` keep their signatures. They write the id when a caller asks.
   - `field_input` (`grid.rs:427-435`), `input_of_field`, and `inputs_of_field` (`grid.rs:300-332`) read `LeafIds`. They keep the fallback to the first limit input.
   - `inputs_of_field` keeps its sort on the length of the id text, then the id text (`grid.rs:329`). A derived `Ord` of `InputId` would change the order of the fault lines.
   - `limit_cells` (`walk.rs:440-458`) gives an element leaf the inputs of its `[*]` row. `note` (`walk.rs:520-529`) keeps the first id of each kind. Both rules stay.
   - `LeafRow::input_ids` and `with_errors` (`grid.rs:170`, `:364`) write each id once for the lookup.
   - `input_texts` (`grid.rs:346`) returns owned ids for `SentEcho` (`form_view.rs:243-252`).
   - `max_length_ids` (`grid.rs:335`) returns `InputId` values for `length_line` (`fields.rs:633-650`). The collision check of `fields.rs:645` stays.
   - The templates write each id through `Display`. For `FormErrors::lines` (`error/form.rs:242`), a template writes the id once into a short `String`.
   - `test/grid.html` and `test/grid-form.html` only call the macros of `components/grid.html`. They do not change, and they stay as render checks.
3. The names, the row ids, and one path: the template writes `value<path>`, `limits[<i>].<field>`, `limits[<i>].leaf_path`, `grid-row-<n>`, and `grid-row-<n>-all` from numbers and the row path.
   - `ValueCell.name`, `InputCell.name`, `HiddenField`, `LeafRow.id`, and `AllCell.id` go (`walk.rs:368-376`, `:402`, `:470`, `:490-493`).
   - `LeafRow.path` holds the shown path only. `walk.rs:370-374` writes the root label into it today for the empty path.
   - A `root_label` on `GridView` holds that label. The row header writes it for the empty path. The names of a primitive row thus stay `value` and an empty `leaf_path`.
   - `limit_cells` writes the limit inputs and the hidden field for two kinds of row (`walk.rs:440`, `:461-493`).
   - One kind is a row `[*]`, which has no value path. The other kind is a row whose value path equals its limit path.
   - In both kinds, the shown path is the limit path. The hidden limit path thus equals the shown path.
   - A row that owns limit inputs keeps its limit index `<i>`. The template writes `limits[<i>].<field>` and `limits[<i>].leaf_path` from it.
   - `components_tests/grid.rs:180` pins the HTML of a row `[*]` with its hidden field. It stays unchanged.
4. The hints and the type: `ValueCell.hint` and `InputCell.hint` hold a `Copy` hint key. `type_text` becomes a `Display` view of the primitive and its length (`texts.rs:22-75`).

- The `ids` map stays. It maps each path text to its `LeafIds` (ADR-0039 (n)). Closed groups keep their rows (ADR-0039 (l)).
- Sites of the templates: `components/grid.html` lines 9 to 25. Its callers are `parameter-form.html`, `parameter.html`, `grid-values.html`, `test/grid.html`, and `test/grid-form.html`.
- Gain, estimate: the held grid of the `GET` falls from 31 574 502 B to about 13.7 M.
  - Method: the sites of `measure/p1_parts_B.json` (`eb`), with a `LeafRow` of at most 288 B and a `LeafIds` of at most 64 B.
  - The rows hold 4.72 M, the `ids` map 4.77 M (keys 2.10 M and 2 728 nodes of about 980 B), and the `Vec<GridRow>` 0.52 M.
  - The one path holds 2.10 M. The cell texts hold 1 620 192 B: 1 081 344 B of bound texts (`walk.rs:471`) and 538 848 B of value texts (`walk.rs:403`).
  - The memo gives about 9 M, because it maps each path to a row number and not to a `LeafIds`.
- After C3a, the `GET` peaks in the render at about 36.3 M (estimate). The view holds about 13.7 M, and the HTML holds 22 569 867 B.
  - The build then holds about 8.5 M of inputs and at most about 13.7 M of grid. The stored phase holds about 31.3 M.
- After C3a, the refused form render holds about 46.5 M (estimate). The phase of mark 2 (62 245 184 B) then decides the refused peak.
- Risk: medium. A wrong `Display` view changes the HTML, and the SHA-256 pins find it. `grid_tests.rs` has 27 lines that read `id`, `name`, `hint`, `type_text`, or `limit_path`.
- No test checks the order of `inputs_of_field` for ids of two lengths today. The limit index starts at 0 (`next`, `walk.rs:82-86`).
  - Task 3.2 thus adds a guard case with at least eleven element inputs. It asserts that `limits-2-min` comes before `limits-10-min`, so a plain sort on the text fails.

### D7. The pins and the measurement of a task

- Seven files in `route_tests/` hold the pins of 26 answers. Each answer has a request peak, and 25 answers have a render peak.
  - The files are `largest_measure_tests.rs`, `largest_refused_tests.rs`, `largest_switch_tests.rs`, and `largest_again_tests.rs`.
  - The other files are `largest_save_tests.rs`, `shape_tests.rs`, and `largest_cap_tests.rs`.
- Each part builds the grid of each grid answer, so it can move each pin, the request peaks too. Each code task thus measures the whole suite.
- The suite runs the five shards of the CI job `heap` (`.github/workflows/ci.yml:851-866`) as five parallel processes on one build with `--cfg heap_measure`.
- The baseline of a task is the pin values before its red step. Task 1.2 records the measured values of all 26 answers on the base.
- The pin update sets each pin whose measured value is below its baseline to that measured value. It also replaces each red value with its measured value.
- The check adds `PEAK_SLACK` (`measure_kit.rs:27`), so each bound is the pin plus 1 MiB (ADR-0223 (t)).
- A pin never rises. A rise inside `PEAK_SLACK` keeps the old pin, and the task reports the rise. A rise above it fails the heap test.
- A request peak varies by up to 1 864 B between runs (ADR-0223, "Measured numbers"). A fall of at most 1 864 B is noise, so the pin stays.
- A pin is thus a reference value. It is the measured value of the last pin update that changed it. The latest measured value can differ from it by noise or by a rise inside `PEAK_SLACK`.
  - The suite has 51 peaks: 26 request peaks and 25 render peaks. Task 1.2 and task 4.1 record the reference and the latest measured value of each apart.
  - Task 4.1 compares each of the 51 pin constants with the shard logs under this rule.
- The suite run after the code change is the check of the task. The task does not run the suite again after the pin update, because each new pin equals its measured value.
- A failing heap test stops the task, and the task reports it. A heap test fails when a peak is above its pin plus `PEAK_SLACK`.
- The red test of a code task is a pin that the task lowers first, a `size_of` bound, or a small ignored `peak_in` test.
- A red run names its tests with `--exact`. The filter `largest_refused` also runs `largest_refused_lines_page_pl` and `largest_refused_lines_htmx_pl`.

Time of a task (Decided Question 3):

- `pins-run.log` gives 353 s for `largest_page_en` and 153 s for `largest_refused_page_en` before C1. These are the render time plus the request time of each test.
  - The fixture setup and the checks after the measurement are outside these times. The run of three tests took 893.91 s (`pins-run.log` line 14).
  - A red run of both tests thus takes about 10 minutes (estimate).
- The local time of the heap suite is unknown, because no log holds a local run of the suite. The local time of the code check is also unknown.
- The CI shards took 63, 46, 44, 30, and 28 minutes (`ci.yml:844`). By estimate, the local suite takes 30 to 60 minutes.
- Task 1.2 measures the time of each full command: the red run of both tests, each shard, and the code check. It then computes the budget of each code task.
- The budget of a code task is an estimate. It holds the red run, about 30 minutes of code, the green run, the code check, and one suite run.
  - `openspec/config.yaml` requires the code check after each task, so the budget holds it. A code task fits when its budget is at most 120 minutes.
- A code task that does not fit splits into two tasks.
  - The code task ends after the green run of its red tests and the code check.
  - A measurement task then runs the suite, the pin update, and the code check.

### D8. ADR-0226

- ADR-0226 records C1, C2a to C2d, and C3a. It also records C2e. Task 1.1 writes it with the status "Proposed" and the estimates.
- Task 4.1 sets the final status:
  - "Accepted" when both targets are met. ADR-0226 then records the measured peaks.
  - "Accepted" also when the code misses a target and the owner merges the code. ADR-0226 then records the miss. D1 to D9 do not depend on the targets.
  - When the owner does not merge the code, ADR-0226 stays "Proposed". The three notes "Amended by ADR-0226" then go out of the branch before a later merge.
- It amends ADR-0223 in two parts:
  - the Context line 43, which puts the row model at about the size of the HTML. On cdca4d2, the built grid holds 31 574 502 B against an HTML of 22 569 867 B.
  - the section "Measured numbers": its table and lines 266 to 271. Lines 266 to 268 give the refused render peak. Lines 269 to 271 give the request peaks of the saves.
- Line 272 of ADR-0223 stays. ADR-0225 (r) gives the request peaks of the saves to the later change, and ADR-0226 keeps that item.
  - ADR-0226 records only new measured values of those peaks. The later change still decides their memory budget.
- ADR-0223 (v) stays a record of the old render peaks, as ADR-0225 (m) says. ADR-0226 gives its own table of the new peaks.
  - The table is for the route fixture of 16 384 leaves (`largest_form_tests.rs:29-33`). Round 1 of the value leg sends the body of 4 096 leaves (ADR-0222 (c)).
  - ADR-0226 thus gives no new sum for round 1.
- It amends ADR-0225 in two parts:
  - the Context line 16: "the later change" becomes the next change after `grid-build-memory`. This also applies to item (r).
  - The budget `form.grid_memory`, the write check with `E-WEB-018`, A54, and the bound on parallel `GET` pages stay with the later change.
  - item (t): the two pins at the cap get their new measured values.
- It amends ADR-0222 Context line 20, because C2a moves the lines of the refusal in `ready`.
  - When task 3.5 runs, C2e also moves `limit_list` from line 149. ADR-0226 then also amends ADR-0222 lines 124 to 125 and line 208.
  - C2a and C2e lower the memory of each round 1 body. ADR-0222 line 207 names this risk. Round 1 stays a report (ADR-0225 (n)).
  - C2e keeps the limit list before the refusal. The route test of ADR-0222 with the bad `max_length` text thus keeps its proof.
- ADR-0226 records that the row model holds 0 B of the peaks of the three profiled answers on cdca4d2.
  - On cdca4d2, the built grid is larger than the HTML of the `GET`. The risk of ADR-0223 line 298 thus holds for the size of the grid on cdca4d2, not for its peaks.
  - After C3a, the grid is about 13.7 M (estimate, D6), so it is smaller than the HTML. Task 1.1 thus writes these sentences as facts of cdca4d2.
  - Task 4.1 gives an upper bound of the held grid after C3a: the render peak minus the page length of `largest_page_en`. It states whether the risk still holds.
- Task 4.1 writes the final line numbers of `ready`, the measured table, and the status, as ADR-0223 (t) does.
  - The table has the columns answer, status, length, request peak, and render peak. It has no column "Sum of change A", because ADR-0226 gives no new sum.
- `reliability.md` §7.8 cites the pins of ADR-0223. ADR-0226 amends ADR-0223, so `docs/architecture/` needs no change.

### D9. The delta spec

- No requirement changes behavior. The HTML stays the same, so `web-ui` has no delta.
- The requirement "Peak memory of a grid answer" of `operations` now names the SHA-256 pin. The route tests already pin it (ADR-0223 (q)), and this change depends on it.
- The refused scenario gets the line of the length and the SHA-256, as the edit page scenario has.
- The Codex review proposes `skip_specs: true`. The plan keeps the MODIFIED block, because the change depends on the SHA-256 pins (Decided Question 2).

### Order

| Task | Part | Red test | `GET` `en` after (B) | Refused `en` after (B) |
|---|---|---|---|---|
| 2.1 | C1 | measured pins | 72 040 843, measured | 126 708 733, measured |
| 2.2 | C2a | measured pins, request pins | 72 040 843 | 64 784 637, measured |
| 2.3 | C2b | `GET` pins at 66 000 000 | at most about 60.67 M, build | about 64.78 M |
| 2.4 | C2c | `peak_in` test of `LimitTexts` | about 54.15 M, render | about 64.43 M, form render |
| 2.5 | C2d | capacity test | about 54.15 M | about 64.43 M |
| 3.1 to 3.4 | C3a | `size_of` bounds | about 36.3 M, render | at least 62 245 184, mark 2 |
| 3.5 | C2e, accepted (Decided Question 1) | refused pins at 60 959 672 | unchanged | about 55.42 M, page |

- The order puts the measured parts first. C2b comes before C2d, so the red pin of C2b fails before its change.
- Each value after task 2.2 is an estimate. The memo gives about 38.2 M for the `GET` build after C3a, without C2c.

## Risks / Trade-offs

- Risk: without C2e, the refused target is likely missed. The phase of mark 2 holds 62 245 184 B, 1 285 512 B above the target.
  - No scoped part changes that phase. Mitigation: task 3.5 runs C2e, which the owner accepted (Decided Question 1).
  - Without C2e, task 4.1 records the miss, and the owner decides option (c) of A26.
- Risk: a `GET` estimate is wrong, and the `GET` target fails.
  - Mitigation: task 4.1 measures it. A miss stops the plan, and the owner decides option (c).
- Risk: a target misses after the last task. ADR-0226 then records the miss and keeps the status "Proposed". The plan stops before the code review.
  - The owner then decides whether the code merges, and decides option (c). Recommended: merge, because each part keeps the HTML and lowers measured peaks.
  - On a merge, ADR-0226 becomes "Accepted" with the recorded miss. Without a merge, the three status notes go out of the branch (D8).
  - The `CHANGELOG.md` line comes with the merge (Decided Question 6).
- Risk: a code task takes more than 2 hours. The CI shards took 211 minutes in sum, and the local time is unknown.
  - Mitigation: the five shards run in parallel. Task 1.2 measures the local times, and D7 gives the split rule.
- Risk: a render pin measures a copy of `ready` that differs from `ready`.
  - Mitigation: after task 2.2, the copy calls the refusal step of `ready` (D2). Task 3.5 keeps this rule.
  - Tasks 2.2 and 3.5 also require a fall of the request pins, which run `ready` through the router.
- Risk: `has_index_at` depends on the sort order of the keys.
  - Mitigation: D1 states the order. Task 2.1 adds guard cases on trees that resolve their paths.
- Risk: C3a writes an id or a name wrong, or changes the order of the fault lines.
  - Mitigation: the route tests pin each SHA-256. The unit tests of `components_tests/grid.rs` compare the HTML of small grids. Task 3.2 adds a guard case for the order.
- Risk: C2d changes every walk of `recipe-domain`.
  - Mitigation: the walks keep their order and their content. `cargo test -p recipe-domain` runs each walk.
- Conflict for `machine-link`: `openspec/changes/machine-link/` cites ADR-0224 14 times. `main` holds ADR-0224 for `mimalloc`, ADR-0225 for the cap, and this change takes ADR-0226.
  - Action for `machine-link`: it takes the next free number, ADR-0227, at its apply. This change does not edit `machine-link`.
- Trade-off: the template writes each id at render time. The render thus does more work for each input, and it holds no id string between two inputs.

## Decided Questions

1. C2e (task 3.5). It waits for the owner at the approval of `tasks.md`. C2e is outside the scope decision of 2026-11-27.
   - Without C2e, the refused target is likely missed: mark 2 peaks at 62 245 184 B, above 60 959 672 B.
   - With C2e, `ready` drops `value` before it calls `limit_list` when a line exists. The phase then peaks at about 40 297 344 B (estimate, mark 1).
   - The refusal step of D2 then takes `value` only. It drops `value` and calls `limit_list` itself, so the copy of the pin test calls no `limit_list`.
   - With C2e and C3a, the page phase of 55 416 520 B is the refused peak. The margin to the target is then 5 543 152 B (estimate).
   - Task 3.5 sets the red pin to the target, 60 959 672 B. With `PEAK_SLACK`, mark 2 fails it by 236 936 B, far above the noise of 1 864 B.
   - A lower red pin would also stop a result that meets the target. A red run that passes shows that the target is met already, so the task stops and reports.
   - C2e moves `limit_list` from `prepare.rs` line 149. ADR-0222 pins that line in lines 124 to 125 and line 208 (D8).
   - Recommended by the Opus review: accept. Recommended by the Codex review: exclude, and let task 4.1 record the measured miss.
   - Decided 2026-11-28 (owner): accept C2e. Task 3.5 runs after task 3.4.
2. The delta spec. Decided: the MODIFIED requirement of D9 stays, and the plan uses no `skip_specs: true`.
   - Decided 2026-11-28 (owner): keep the MODIFIED block of D9. The plan uses no `skip_specs: true`.
3. The time of a task. Recommended default: split the measurement into a task of its own when the budget of D7 needs it. Task 1.2 gives the budget.
   - Decided 2026-11-28 (owner): split the measurement into a task of its own when the budget of D7 is over 120 minutes.
4. A target gate in the tests. Recommended default: no CI gate. ADR-0226 records the targets.
   - Task 4.1 compares the raw measured render peak of each variant with its target, without `PEAK_SLACK`.
   - The `GET` target applies to `largest_page_en`, `largest_page_pl`, and `largest_page_fragment_pl`. The refused target applies to the four answers of round 1.
   - The rebaseline memo defines each target for the `en` page only. The plan applies the same number to each variant of the group.
   - Decided 2026-11-28 (owner): no CI gate for the targets. ADR-0226 records the targets.
5. C2b drops the `ParameterValue` after `value.texts(tree)?` and before `ValueTexts` exists. Recommended default: yes.
   - It frees 10 796 992 B before the limit texts (D3). The stored peak stays at the end of `value.texts(tree)`, so no peak falls.
   - The phase of the limit texts falls by the same amount (estimate). No pin measures that gain.
   - Decided 2026-11-28 (owner): yes, drop the `ParameterValue` early.
6. `CHANGELOG.md`. Recommended default: one line under "Changed" when the code merges, also after a missed target.
   - Decided 2026-11-28 (owner): one line under "Changed" in `CHANGELOG.md` when the code merges.
