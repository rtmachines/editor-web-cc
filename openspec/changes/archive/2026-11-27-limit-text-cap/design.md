# Design

## Context

See `proposal.md`, section "Why". This design checks each fact at `origin/main` 0f38841. MiB is 1 048 576 bytes. MB is 1 000 000 bytes. The memo is `~/editor-web-cc-archive/change-b/rebaseline-memo.md`. Review 1 of the plan is in `~/editor-web-cc-archive/change-b/review1-opus.md` and `review1-codex.md`.

### C1: the reader of the limit rows

- `read_pairs` decodes each pair with `decode` (`crates/recipe-web/src/pages/parameters/fields.rs:165-189`). Bytes that are not UTF-8 become U+FFFD, with 3 bytes for each bad byte.
- `place_limit` (`fields.rs:359`) puts each decoded text of `limits[<i>].<field>` into its row as sent.
- `limit_rows` (`fields.rs:458`) builds each `SentLimit` with `min`, `max`, and `max_length` as sent. It checks no length.
- `limit_list` (`fields.rs:562-592`) checks only the digits of `max_length`. `present` (`fields.rs:482-485`) and `is_blank` (`fields.rs:451`) trim each text.
- The reader serves the forms of `AddParameter`, `UpdateParameterDefinition`, and `ReplaceLimits` (`FieldSet::Parameter` and `FieldSet::Limits`).
- `read_form` is the one entry of these forms (`edit.rs:222`, `new.rs:170`, `value.rs:242`). `replace_limits` calls it before `sent_row_version` (`value.rs:242-243`).
- A refused form builds its grid from the sent texts (`refuse_checked`, `prepare.rs:117-124`). A line under a field thus keeps every sent text in the grid.
- The answers of `fields=1` and `limit_mode` also build the grid from the sent texts, with no bound check (`prepare.rs:177-184`).
- The grid copies the texts: `LimitTexts::from_sent` (`grid/limits.rs:101`), `moved_to` (`grid/limits.rs:121-128`), and up to `room` copies (`grid/limits.rs:446-456`).
- The walk writes `from_all` into each element row (`walk.rs:441`, `limit_of_all` at `walk.rs:497-517`). It clones each text into an input cell (`walk.rs:471`).
- askama writes `"` as `&#34;`, so one byte of a text can become 5 bytes of HTML (askama-0.16.1 `src/filters/escape.rs:130`).
- A limit input has no `maxlength`. `InputCell` has no such field (`grid.rs:204-213`), and `limit_cell` writes none (`templates/components/grid.html:11`).
- The stored bound columns are `NUMERIC` and `SMALLINT` (`migrations/0001_schema.sql:447-449`). C1 thus concerns the sent texts of a form answer, not the stored grid.

Worst case of C1 today, an estimate for an ASCII text (memo §5, `A-verify-limits.md:36-42`, checked in the code above):

- The type is `ARRAY[1..16384] OF INT` (A26). One row `[*]` holds a `min` text near `http.body_limit.value` (16 777 216 bytes, `reliability.md` §7.11).
- `from_all` then holds 16 384 × 16 777 216 = 274 877 906 944 bytes (2^38) before any HTML.
- The text shares the body of 16 MiB with the other fields. A text that is not UTF-8 can decode to 3 bytes for each byte of the body, so the body limit is no upper bound.
- `soak.memory_ceiling` (402 653 184 bytes) falls at a text of 402 653 184 / 16 384 = 24 576 bytes.
- At a cap of C bytes, `from_all` holds at most 16 384 × 2 × C bytes. A text of `"` adds up to 16 384 × 2 × C × 5 bytes of HTML in the element notes.
- The two inputs of the row `[*]` add 2 × C × 5 bytes of HTML (`templates/components/grid.html:11`, `:13`). At 256 bytes, the notes and the inputs hold 41 945 600 bytes of escaped bound text.
- After the switch of `limit_mode`, `moved_to` clones all three texts into each element row, also a `max_length` on an `INT` row. The rows then hold up to 16 384 × 3 × C bytes.

### C1: the length of a valid bound text

The table gives the longest text in decoded bytes. The values come from the grammar in the code below, not from a run of the parser. Task 2.5 checks each value with the parser. The test `nineteen_digits_in_one_part_are_accepted` (`crates/recipe-domain/src/plant/iec_literal/duration/tests.rs:244`) already passes with a part with leading zeros.

| Type | Typed, no leading zeros | Typed, with leading zeros | Canonical text of the server |
|---|---|---|---|
| `LTIME` | 38: `LTIME#-106751d23h47m16s854ms775us808ns` (S7-1500) | 150: `LTIME#-` and seven parts of 19 digits | 35: `LT#-106751d23h47m16s854ms775us808ns` |
| `TIME` | 23: `TIME#-24d20h31m23s648ms` (S7) | 107: `TIME#-` and five parts of 19 digits | 20: `T#-24d20h31m23s648ms` |
| `S5TIME` | 16: `S5T#1h10m10000ms` | 85: `S5T#` and four parts of 19 digits | 14: `S5T#1m39s900ms` |
| `LDT` | 44: `LDATE_AND_TIME#2262-04-11-23:47:16.854775807` | 44, because each field has a fixed width | 33: `LDT#2262-04-11-23:47:16.854775807` |
| Numeric | no limit, see below | no limit | 31: a sign, `0.`, and 28 digits |
| `max_length` | 5 digits | 5 digits, leading zeros included | 5 digits |

The facts behind the table:

- A duration part has 1 to 19 digits, and a leading zero is valid (`iec_literal/duration.rs:8`, `:156-158`). Each zero part passes the part check (`duration.rs:199-212`).
- `TIME` and `LTIME` accept a sign on every platform (`duration.rs:114-121`). `-0` has the count 0, which each range holds (`iec_literal/range.rs:12-18`).
- The longest prefixes are the long aliases of ADR-0205 (a), for example `LTIME` and `LDATE_AND_TIME` (`iec_literal/mod.rs:146-160`).
- An instant field has a fixed width, and a fraction has 1 to 9 digits (`iec_literal/instant.rs:157-159`, `:186`, `:205`). An instant literal thus has no padding.
- The fraction limit depends on the type (`instant.rs:114-120`). `DATE`, `LDATE`, and `DATE_AND_TIME` take no fraction. `TIME_OF_DAY` takes 3 digits. `LTOD`, `LDT`, and `DTL` take 9 digits.
- ADR-0211 (f) gives 39 and 50 bytes. These values count encoded bytes. The decoded values are 33 and 44 bytes.
- A numeric bound is a JSON number (`crates/recipe-domain/src/recipe/value/decimal.rs:95-112`). Trailing zeros of the fraction and a zero mantissa parse at any length (`decimal.rs:35-46`). A zero mantissa also takes an exponent of any length and value, because the parse returns zero before it reads the exponent (`decimal.rs:35-38`).
- An exponent can have any count of leading zeros (`decimal.rs:58`). Zeros at the start of the fraction with a larger exponent also keep the value.
- The canonical numeric text has a scale of at most 28 (`decimal.rs:9`). An integer type has at most 20 bytes, for example `-9223372036854775808`.
- `REAL` and `LREAL` bounds have at most 7 and 15 significant digits (`recipe/value/leaf.rs:31`, `:34`). A stored bound of the earlier rule set can have 28 (`recipe/limit/parse.rs:171-173`).
- `present` trims spaces of any length in each field, also in `max_length`. A blank text of any length is no bound.
- After the trim, `max_length` accepts 1 to 5 digits, leading zeros included (`fields.rs:49`, `:488-492`). The reader refuses a longer digit string on the save today.

Two kinds of padding thus exist. Leading zeros of a duration part are finite, with 150 bytes for `LTIME`. Spaces and the over-long spellings of a numeric text have no limit. Examples are the zeros of a fraction and the exponent of a zero mantissa.

### F3: the overlap check of the value leg

- The threshold is `reference_anon` + the sum of the four body lengths (`deploy/ci/load_value.py:297`). ADR-0222 (c) requires this rise.
- The poll returns at the first read at or above the threshold (`load_value.py:303-305`). It sleeps 50 ms between two completed reads (`load_value.py:307`). Each read starts a `sudo cat`, so the read rate is below 20 reads each second.
- `remaining` raises `TimeoutError` at the deadline (`load_value.py:83-87`). The poll then returns the last read with the failure "the bodies did not overlap" (`load_value.py:308-311`).
- The client holds back the last byte of each body until the barrier (`load_value.py:237`, `:243`).
- The fixture bodies have 3 033 541 bytes. The server then holds at most 4 × (3 033 541 − 1) = 12 134 160 bytes of round 2 body.
- With these bodies, the threshold needs 12 134 164 bytes.
- A round 2 body can have up to 3 033 559 bytes (`load_value.py:358-359`, `:497-498`). In general, the threshold needs the sum of the four lengths, and the server holds 4 bytes less.
- A pass thus depends on other growth of `anon`.
- The checker computes `overlap_margin` (`load_value.py:530-548`). The summary writes it (`load_value.py:667-669`).
- The step "Require the values from each leg" checks the shape of `barrier_anon` and `overlap_margin`: a count or `null` (`.github/workflows/ci.yml:408-416`).
- That step has no `continue-on-error`, so it gates both legs. The step `value-check` only reports on `cc-debian13` (`ci.yml:335`, `:337`).
- The barrier deadline is `BARRIER_TIMEOUT` (8 s, `load_value.py:75`). ADR-0222 (c) keeps it below `http.body_read_timeout`.

### D1: the measured runs

The three runs below are dispatch runs of the branch of `value-body-memory-load`. `git diff 19c5ccc 0f38841 -- crates deploy .github Cargo.lock` is empty. The runs on bcb3824 and cae5371 differ from `main` only in CI scripts and tests. The values come from `load-value.json` of each leg in `~/editor-web-cc-archive/value-body-memory-load/run-*/`.

| Run | Leg | Round 1 peak | Margin to the ceiling | Round 2 peak | Margin to the ceiling |
|---|---|---|---|---|---|
| 45748785867 | musl | 352 993 280 | 49 659 904 | 268 808 192 | 133 844 992 |
| 45748785867 | cc-debian13 | 400 224 256 | 2 428 928 | 293 281 792 | 109 371 392 |
| 45776554175 | musl | 365 916 160 | 36 737 024 | 269 119 488 | 133 533 696 |
| 45776554175 | cc-debian13 | 338 546 688 | 64 106 496 | 275 632 128 | 127 021 056 |
| 45805398657 | musl | 372 977 664 | 29 675 520 | 259 321 856 | 143 331 328 |
| 45805398657 | cc-debian13 | 388 915 200 | 13 737 984 | 279 339 008 | 123 314 176 |

- The highest round 1 reading on musl in all runs of the memo is 382 672 896 bytes. It comes from run 45768560058, a build with mutation B. Its margin is 19 980 288 bytes.
- Mutation B copies only bodies below 16 711 680 bytes, so its round 1 reading is real.
- The highest round 1 reading on `cc-debian13` is 402 501 632 bytes. It comes from run 45574974451 on the branch `diag/musl-round2`. Its margin is 151 552 bytes.
- Run 45805398657 records `overlap_margin`. Round 1: 44 334 744 bytes on musl and 4 538 008 bytes on `cc-debian13`. Round 2: 588 012 bytes and 690 412 bytes.
- The baseline `memory.current` of round 1 in run 45805398657 is 31 129 600 bytes on musl (`load-memory.json:61`).
- The run statistic of run 45805398657 in round 1 is 85 462 016 bytes on musl and 95 715 328 bytes on `cc-debian13`.
- In round 2, it is 57 695 232 bytes on musl and 68 393 984 bytes on `cc-debian13`.
- No run measures parallel `GET` pages of a large grid.

## Goals / Non-Goals

**Goals:**

- One request cannot make the grid copy a limit bound text of more than `form.limit_text_max` bytes.
- The value leg gives data for a later review of the threshold of the overlap check.
- The documents state the memory frame of the value routes in measured readings.

**Non-Goals:**

- The next change is O4 of the memo. It first makes the row model (`GridView`) smaller.
- After that, it plans the budget D4, the write check D5, A54, a bound on parallel `GET` pages, and Options E and F of ADR-0223.
- No change to the threshold of the overlap check, to the gate of round 1, or to a setting other than the new cap.
- No change to the row model, to the existing `dhat` pins of ADR-0223, or to the parse of a value body.
- No change to the stored limits, to the grammar of a stored bound, or to the export.

## Decisions

ADR-0225 records the decisions C1, F3, D1, and R1 below. Task 1.1 writes it. It amends these items:

- ADR-0138 (a) and (b): an over-long bound text is a fault of a changed body, with its own reason text.
- ADR-0138, Context line 19 and the decision driver of line 29: a user can now type one fault of (b). ADR-0225 gives the new text of this premise.
- ADR-0138, Context line 21 and item (c), line 64: the value form and the limits form read the body before `row_version`. A fault of (b) or (c) thus comes first. ADR-0225 gives the new text of both sentences.
- ADR-0211 (f): a typed bound text above the cap gets the line of C1. A body above the body limit of its route (`api.md` §7.6) keeps `413 E-WEB-003`.
- ADR-0222 (a), lines 83-84: round 1 stays report only (R1). ADR-0222 (c): the record of the largest read of round 2 (F3).
- ADR-0222 (f): §7.8 keeps its measured values and adds the frame of D1. The Consequences line "Until change B" of ADR-0222 follows (a).
- ADR-0223 (w) and (x): D1 decides the memory frame. The budget, the write check, `E-WEB-018`, and A54 move to a later change.
- ADR-0223, lines 73-74 and 210: Options E and F move to a later change. The mitigations of lines 295 and 297 (the sum and the parallel `GET` pages) move too.
- ADR-0223, lines 161, 272, 281, and 299: ADR-0225 moves each item of change B to a later change or closes it. These items are the allowance estimate, the request peaks of the saves, the start from measured numbers, and the use of the measured render numbers.

### C1. A cap on the bound texts in the reader

- `place_limit` refuses a `min`, `max`, or `max_length` text above `form.limit_text_max` bytes.
- It counts the decoded UTF-8 bytes of the text before the trim. This count bounds the text that the grid keeps and copies.
- The cap is the code constant `LIMIT_TEXT_MAX` of `crates/recipe-web/src/settings.rs`. It is no field of `WebSettings`, because `read_form` has no settings handle.
- The check runs when the reader places the pair. So the reader stops at the first over-long text, as ADR-0138 requires for a changed body.
- The answer is one line `422 E-WEB-008` of the field `form` with the new reason `LimitText`. The answer holds no out-of-band form and no grid (ADR-0138 (g)).
- An over-long text with a bad `row_version` on `/limits` gets this one line. `replace_limits` and `set_value` read the body before `row_version` (`value.rs:188-189`, `:242-243`).
- So every fault of ADR-0138 (b) and (c) wins over a bad `row_version` today. ADR-0138 line 21 and item (c), line 64, say the opposite. ADR-0225 amends both to match the code.
- A body above the body limit of its route gets `413 E-WEB-003` before the reader runs (`api.md` §7.6). C1 does not change this answer.
- `/limits` is no value route, so its limit is `http.body_limit.form` (`crates/recipe-web/src/limits.rs:23-28`, `:49-57`). The test `limits_every_post_row_of_the_route_table_has_the_limit_of_its_path_shape` pins this limit.
- The limit inputs keep no `maxlength`. A user can thus type a text above the cap into the editor HTML. That text then gets the line of a changed body.
- This departs from the premise of ADR-0138 that the editor HTML cannot send a changed body. ADR-0225 records the departure for this one fault.
- A new row of `reliability.md` §7.11 holds `form.limit_text_max`. `api.md` §2.6 names the cap in the rule of the limit fields.
- The `leaf_path` of a limit row keeps its own bound (ADR-0211).
- A new `dhat` pin measures the largest answer at the cap in `[*]` mode and after the switch of `limit_mode` (task 2.4). Its `[*]` row holds a `min`, a `max`, and a `max_length` of C bytes each. ADR-0225 records its numbers as measurements of this fixture.

Alternatives:

- A line under the field (ADR-0041 (a)). Rejected, because the refused form builds its grid from the sent texts (`prepare.rs:117-124`).
  - A line under the field also replaces the form. The HTMX browser then loses the typed text. With `E-WEB-008`, the browser keeps the form and the typed text (ADR-0138 (g)).
- The check in `limit_rows`. Rejected, because the reader then places every pair first. The check in `place_limit` stops earlier and needs no second pass.
- A `maxlength` of the cap on each limit input. Not selected, because it changes every grid answer with a limit input.
  - The route tests of ADR-0223 pin the length, the SHA-256, and the peak of each answer (`route_tests/measure_kit.rs`). The ignored measurement tests run in CI (`ci.yml:883`).
  - `maxlength` counts UTF-16 code units, not UTF-8 bytes. So the server check stays necessary.

### F3. The largest `anon` of round 2

- In round 2, the poll does not return at the first read at or above the threshold. It keeps that read as `barrier_anon`, so the overlap check does not change.
- It then reads again for `RECORD_WINDOW_S` after that read, with a sleep of 50 ms between two completed reads. Then it releases the barrier.
- The barrier deadline (8 s) ends the window earlier. The barrier thus releases no later than the barrier deadline.
- After the first crossing, the round passes the overlap check. The barrier deadline then ends the window with a pass, not with a failure.
- A failed read in the window gives `None`, also a read that times out (`load_value.py:279-288`). The poll skips it, as before the crossing (`load_value.py:303`).
- Each read and each sleep in the window has the budget of the earlier end: the end of the window or the barrier deadline. The poll releases the barrier at that end, and a read after it does not count.
- The poll records the largest read as `barrier_anon_max`. This value is at least `barrier_anon`.
- The checker computes `overlap_margin_max` as `barrier_anon_max − reference_anon −` the sum of the four body lengths. The summary shows both values for round 2.
- A round with no crossing writes `null` for both new fields. Its failure stays as today. Three outcomes exist (`load_value.py:308-311`, `:330-335`, `:539-547`):
  - Valid reads below the threshold give "the bodies did not overlap".
  - No valid read gives the failure that `barrier_anon` is missing.
  - A request that missed the barrier gives the failure of the barrier.
- The checker in `value-check` requires a count for `barrier_anon_max` when `overlap_margin` is 0 or more. It also requires `barrier_anon_max` at or above `barrier_anon`.
- The step "Require the values from each leg" checks the shape only. It requires both keys in each round, `null` in round 1, and `null` or a number in round 2.
- Round 1 keeps the release at the first read. Its timing then stays equal to the runs of D1.
- `RECORD_WINDOW_S` is a constant of `load_value.py`, like `BARRIER_TIMEOUT`. ADR-0225 names its value. It is no setting of `serve`, so §7.11 gets no row.

Alternatives:

- Read until the barrier deadline (memo F3). Rejected as the default, because it holds every round 2 body for up to 8 s. The window gives the same data for a short overshoot.
- Both rounds. Not selected, because the owner named round 2. Decided Questions holds this point.
- F1 or F2 of the memo. Rejected by the owner, because they change the rule of ADR-0222 (c) before data exists.

### D1. The memory frame in `memory.peak` readings

- The frame of the value routes and the grid answers is the `memory.peak` reading of each round of the value leg against `soak.memory_ceiling`.
- `reliability.md` §7.8 keeps the table of run 45748785867 and its text, as ADR-0222 (f) requires. That table holds the body length, the run statistic, and 3 × the body length.
- §7.8 adds the peak table of the three runs of the Context.
- The text states the worst round 1 margins of the memo with the run and the build of each reading. It also states the overlap margins of run 45805398657.
- The `dhat` pins of ADR-0223 stay bounds for one answer. The text does not add them to a sum against the ceiling.
- The saved sum of change B (4 × 48 MiB, a budget of 64 MiB, and a rest of 128 MiB) is not adopted. The flood reading and the pre-check reading show that the rest is not a constant.
- Known limit: no run measures parallel `GET` pages. §7.8 says so.

Alternative: the saved frame as a sum of estimates. Rejected, because the measured readings do not follow it (memo §4).

### R1. Round 1 stays report only

- ADR-0222 (a) says that change B turns the gate of round 1 on (ADR-0222 lines 83-84). ADR-0225 replaces this sentence: round 1 stays report only until a later change bounds the grid answers.
- The reason: the worst round 1 margin is 19 980 288 bytes on musl. A gate would fail on a small change of the allocator or of the render.
- The spec already says that the job reports round 1 without a gate (`openspec/specs/operations/spec.md:2943`). This change thus has no delta for R1.
- D1 changes documents only. Its frame is no behavior of the system, so this change has no delta for D1.

## Risks / Trade-offs

- Risk: The cap refuses a text that the reader or the domain accepts today.
  - These texts are each over-long numeric spelling and each text with many spaces. Examples are `1.` and 300 zeros, and `0e` with 300 digits. A blank text of many spaces is also refused.
  - The reader already refuses a `max_length` of more than 5 digits after the trim on the save. The cap refuses only its space padding as a new fault.
  - At a cap below 150 bytes, a duration literal with many leading zeros is also refused.
  - Mitigation: Each such text has a short form. ADR-0225 and `CHANGELOG.md` name each kind of refused input.
- Risk: A user types a text above the cap and gets the line of a changed body, not a line under the field.
  - Mitigation: The cap is above each canonical text and each valid text without padding. The reason text `LimitText` names the fault and the cap.
- Risk: The cap is too small for a valid time literal.
  - Mitigation: Task 2.5 builds the longest literal of the rule of each time type and platform. Its test fails when the cap differs from the value of the rule.
- Risk: The worst answer at the cap uses more memory than the pins of ADR-0223 show.
  - Mitigation: Task 2.4 measures it. At 256 bytes, the added HTML is 41 945 600 bytes, about 1.9 times the `GET` edit page of 22 569 867 bytes (ADR-0223).
- Risk: The record window holds the round 2 bodies longer.
  - Mitigation: The barrier deadline ends the window, so the barrier releases no later than the barrier deadline. Round 2 bodies have about 3 MB.
- Risk: The record window turns a pass into a failure.
  - Mitigation: After the crossing, the deadline ends the window with a pass, and the poll skips a failed read. Task 3.1 tests both paths with a fake clock.
- Risk: The record window can change the peak of round 2.
  - Mitigation: The value leg records the peak as before. Task 4.1 compares the new peak with the table of D1.
- Risk: A dispatch run of the job `load` is necessary to see F3 data.
  - Mitigation: Task 4.1 dispatches the job on the branch. The job does not run on a pull request (ADR-0128 (a)).
- Risk: One run gives no data for three bodies and no spread of the overhead of four bodies.
  - Mitigation: The data of task 4.1 is evidence for a later review of the threshold. This change selects no threshold.
- Risk: The parallel `GET` exposure of memo §4 stays open.
  - Mitigation: The next change, a smaller row model, takes it.
- Risk: a conflict for `machine-link`. Its `design.md` cites ADR-0224 at lines 26, 62, 77, 181, 418, 505, 506, 513, and 529. Its `tasks.md` cites it at line 40.
  - Mitigation: `main` holds ADR-0224 for `mimalloc`, and this change takes ADR-0225. `machine-link` takes the next free number at apply (`machine-link/design.md:529`). This change does not edit `machine-link`.
- Risk: `machine-link/design.md:529` leaves A54 to change B.
  - Mitigation: This change takes no A-number. A54 stays free for the next change.

## Migration Plan

- No migration, no setting of `serve` other than the code constant `form.limit_text_max`, and no change of a stored row.
- Rollback: revert the commits of the change. A stored limit does not depend on the cap.

## Decided Questions

The owner accepted each recommended answer on 2026-11-27. The specs and tasks use these answers. The cap is option C, 256 bytes.

1. The value of `form.limit_text_max`. The Context gives the facts. Each option counts decoded bytes before the trim.
   - Option A, 64 bytes. Rule: the longest valid text without padding, 44 bytes, rounded up to a power of two. Opus review 1 selects it.
     - It refuses each duration literal with leading zeros above 64 bytes, for example `LTIME` up to 150 bytes, `TIME` up to 107, and `S5TIME` up to 85.
     - It refuses each numeric text and each text with spaces above 64 bytes.
     - `from_all` holds at most 2 097 152 bytes. The added HTML is at most 10 485 760 bytes.
     - One line of `xss-corpus.txt` has 150 bytes. Its `min` and `max` cases then give `E-WEB-008`, not `E-RCP-006` (`recipe_xss_tests.rs:351-352`, `:367-378`).
     - The XSS harness holds one fate for each field, so this option also needs a fate for each payload.
   - Option B, 128 bytes. No row of the table gives this value. It lies between `TIME` with leading zeros (107) and `LTIME` with leading zeros (150).
     - It refuses each `LTIME` literal with leading zeros from 129 to 150 bytes. It refuses each numeric text and each text with spaces above 128 bytes.
     - `from_all` holds at most 4 194 304 bytes. The added HTML is at most 20 971 520 bytes. The same line of `xss-corpus.txt` changes its answer, with the same change of the harness.
   - Option C, 256 bytes. Rule: the longest text that `IecLiteral::parse` accepts, 150 bytes, rounded up to a power of two. Codex review 1 selects it.
     - It refuses no valid time literal. It refuses only a numeric text and a text with spaces above 256 bytes.
     - `from_all` holds at most 8 388 608 bytes. The added HTML of the element notes is at most 41 943 040 bytes. No line of `xss-corpus.txt` is above the cap.
   - Recommended: Option C. The parser bounds the length of a time literal, so the cap can accept each valid literal. No cap can accept each numeric text, because its length has no limit.
   - No run measures the peak of the worst answer at a cap yet. Task 2.4 measures it at the chosen cap after this decision.
2. The answer to a text above the cap.
   - Recommended: one line `E-WEB-008` (ADR-0138) from the reader, with no `maxlength` (C1). The HTMX browser keeps the form and the typed text (ADR-0138 (g)). Both reviews select it.
   - A line under the field fits ADR-0041 better, because a user types the bound text. It keeps the copy path of `prepare.rs:117-124`, so C1 fails with it.
   - A `maxlength` on each limit input keeps the premise of ADR-0138 only in part. It costs a new measurement of each pin of ADR-0223 and one more task.
   - A `maxlength` counts UTF-16 code units, not UTF-8 bytes. A text within 256 units can exceed 256 bytes, so it does not keep the premise and does not replace the server check.
3. The reason text of the line. Recommended: a new reason `LimitText`, because the reason `LimitRow` names a missing path.
   - The line has no field, so the text names the cap. Example: "a limit field has more than 256 bytes. Make the text shorter". Both reviews select a text with the cap.
4. The unit of the cap. Recommended: the decoded UTF-8 bytes of the text before the trim, because the cap bounds memory. Both reviews select it.
   - A character can have 4 bytes. A byte that is not UTF-8 decodes to 3 bytes. A count after the trim lets spaces through.
5. The record window of F3. Recommended: `RECORD_WINDOW_S` = 1 s. The memo reads until the barrier deadline. Both reviews select 1 s.
   - The read rate is below 20 reads each second, because each read starts a `sudo cat`.
   - Opus review 1 also proposes to record the index of the largest read. A largest last read then shows that `anon` still rises at the end of the window. The tasks do not add this field. F3 records only the maximum. Whether the window captured a plateau is out of scope.
6. F3 in round 1 too. Recommended: round 2 only, as the owner decided. Both reviews select it.
   - Round 1 data would show the cause of its larger margin (memo §6). A window in round 1 changes the timing of the round 1 peaks of D1.
