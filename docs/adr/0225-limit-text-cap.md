# ADR-0225: A cap on the bound texts of a limit row, and the memory frame of the value routes

Status: Accepted. Amends ADR-0138 (a), (b), (c), its Context, and its decision driver. Amends ADR-0211 (f). Amends ADR-0222 (a), (c), (f), and its Consequences line. Amends ADR-0223 (w), (x), and its moved lines (Options E and F, and lines 161, 272, 281, 295, 297, and 299)
Date: 2026-11-27
Deciders: project owner

## Context

ADR-0222 and ADR-0223 left four memory decisions to a "change B". On 2026-11-27 the owner split that plan. The change `limit-text-cap` takes four decisions:

- C1 caps the bound texts of a limit row in the reader.
- F3 records the largest `anon` reading of round 2 in the value leg.
- D1 states the memory frame of the value routes in measured readings.
- R1 keeps round 1 as a report without a gate.

A later change takes the other decisions of the old plan. In this ADR, the later change is the next change after `limit-text-cap`. It makes the row model (`GridView`) smaller. Then it plans the budget, the write check, and the bound on parallel `GET` pages.

MiB is 1 048 576 bytes. MB is 1 000 000 bytes.

### Facts of C1

- The reader decodes each pair of a form body. The decoder replaces each invalid UTF-8 sequence with U+FFFD, which has 3 bytes.
- `place_limit` (`crates/recipe-web/src/pages/parameters/fields.rs:359`) puts each decoded text of `limits[<i>].<field>` into its row as sent. No code checks the length of a `min`, `max`, or `max_length` text. `limit_list` checks only the digits of `max_length`.
- A refused form builds its grid from the sent texts (`refuse_checked`, `prepare.rs:117-124`). The answers of `fields=1` and `limit_mode` also build the grid from the sent texts, with no bound check.
- The grid copies each text into the row `[*]` and into each element row. askama writes `"` as `&#34;`, so one byte of a text can become 5 bytes of HTML.
- A limit input has no `maxlength`. The stored bound columns are `NUMERIC` and `SMALLINT`, so C1 concerns the sent texts of a form answer and not the stored grid.
- The type `ARRAY[1..16384] OF INT` (A26) and one row `[*]` with a `min` text of 16 777 216 bytes show the worst case. This size is `http.body_limit.value`.
  - The grid then holds 16 384 × 16 777 216 = 2^38 bytes of bound text before any HTML.
  - `soak.memory_ceiling` (402 653 184 bytes) falls at a text of 24 576 bytes.
  - The body limit is no upper bound, because a text that is not UTF-8 decodes to 3 bytes for each byte of the body.
- At a cap of C bytes, the element rows hold at most 16 384 × 2 × C bytes of notes. The escaped HTML of a text of `"` is 5 times larger.

The table gives the longest valid bound text in decoded bytes. The values come from the grammar in the code. Task 2.5 of the change checks each value with the parser.

| Type | Typed, no leading zeros | Typed, with leading zeros | Canonical text of the server |
|---|---|---|---|
| `LTIME` | 38 | 150 | 35 |
| `TIME` | 23 | 107 | 20 |
| `S5TIME` | 16 | 85 | 14 |
| `LDT` | 44 | 44 | 33 |
| Numeric | no limit | no limit | 31 |
| `max_length` | 5 digits | 5 digits | 5 digits |

- A duration part has 1 to 19 digits, and a leading zero is valid. The longest literal is `LTIME#-` and seven parts of 19 digits, which has 150 bytes.
- An instant field has a fixed width, so an instant literal has no padding.
- A numeric bound has no length limit. Trailing zeros of the fraction, the exponent of a zero mantissa, and spaces around the text all keep the value.
- Two kinds of padding thus exist. The leading zeros of a duration part are finite. The spaces and the over-long spellings of a numeric text have no limit.

### Facts of ADR-0138

- ADR-0138 says that the editor HTML sets each key, each hidden field, and each button value. A user types only the values of the inputs. The answer to a changed body is one line.
- The value form and the limits form read the whole body before they read `row_version` (`value.rs:188-189` and `:242-243`). A fault of ADR-0138 (b) or (c) thus wins over a bad `row_version` today.
- ADR-0138 says the opposite in its Context and in item (c). The code does not follow those two sentences.

### Facts of F3

- The overlap check of round 2 reads `anon` of the container `app` until it reaches `reference_anon` plus the sum of the four body lengths (`deploy/ci/load_value.py:297`). The poll returns at the first such read (`:303-305`).
- The client holds back the last byte of each body until the barrier. The server then holds at most 4 bytes less than the threshold. A pass thus depends on other growth of `anon`.
- Each read starts a `sudo cat`, so the read rate is below 20 reads each second. The barrier deadline is `BARRIER_TIMEOUT` (8 s).
- The first read that reaches the threshold ends the poll. No run records how far `anon` rises after the crossing. A later review of the threshold needs this data.

### Facts of D1

- `reliability.md` §7.8 holds the readings of run 45748785867. One run does not show the spread of the peaks.
- The table gives the `memory.peak` readings of three dispatch runs of the branch of `value-body-memory-load`. The code of `main` at 0f38841 equals the code of these runs, except CI scripts and tests. Sizes are in bytes.

| Run | Leg | Round 1 peak | Margin to the ceiling | Round 2 peak | Margin to the ceiling |
|---|---|---|---|---|---|
| 45748785867 | musl | 352 993 280 | 49 659 904 | 268 808 192 | 133 844 992 |
| 45748785867 | cc-debian13 | 400 224 256 | 2 428 928 | 293 281 792 | 109 371 392 |
| 45776554175 | musl | 365 916 160 | 36 737 024 | 269 119 488 | 133 533 696 |
| 45776554175 | cc-debian13 | 338 546 688 | 64 106 496 | 275 632 128 | 127 021 056 |
| 45805398657 | musl | 372 977 664 | 29 675 520 | 259 321 856 | 143 331 328 |
| 45805398657 | cc-debian13 | 388 915 200 | 13 737 984 | 279 339 008 | 123 314 176 |

- The highest round 1 reading on musl in all runs is 382 672 896 bytes. It comes from run 45768560058, a build with mutation B. Its margin is 19 980 288 bytes.
- The highest round 1 reading on `cc-debian13` is 402 501 632 bytes. It comes from run 45574974451 on the branch `diag/musl-round2`. Its margin is 151 552 bytes.
- Run 45805398657 records `overlap_margin`. Round 1 has 44 334 744 bytes on musl and 4 538 008 bytes on `cc-debian13`. Round 2 has 588 012 bytes and 690 412 bytes.
- No run measures parallel `GET` pages of a large grid.

## Decision drivers

- One request cannot make the grid copy a limit bound text of more than a fixed number of bytes (R30).
- A valid bound text stays valid. A cap refuses only a text that has padding without a limit.
- The answer to a changed body stays one line, and the HTMX browser keeps the typed form (ADR-0138 (g)).
- The value leg gives data for a later review of the threshold of the overlap check. It changes no rule of that check.
- The documents state the memory frame in measured readings and not in estimates.
- Each numeric setting comes from `reliability.md` §7.11.

## Considered options

### C1: the value of the cap

The cap counts the decoded UTF-8 bytes of a bound text before the trim.

#### Option A: 64 bytes

- Rule: the longest valid text without padding, 44 bytes, rounded up to a power of two.
- Pros: `from_all` holds at most 2 097 152 bytes. The added HTML is at most 10 485 760 bytes.
- Cons: it refuses each duration literal with leading zeros above 64 bytes. These literals have up to 150 bytes for `LTIME`, 107 for `TIME`, and 85 for `S5TIME`. One line of `xss-corpus.txt` has 150 bytes, so its `min` and `max` cases then give `E-WEB-008` and not `E-RCP-006`.

#### Option B: 128 bytes

- Pros: `from_all` holds at most 4 194 304 bytes. The added HTML is at most 20 971 520 bytes.
- Cons: no row of the table gives this value. It refuses each `LTIME` literal with leading zeros from 129 to 150 bytes. The same line of `xss-corpus.txt` changes its answer.

#### Option C: 256 bytes

- Rule: the longest text that `IecLiteral::parse` accepts, 150 bytes, rounded up to a power of two.
- Pros: it refuses no valid time literal. `from_all` holds at most 8 388 608 bytes. The added HTML of the element notes is at most 41 943 040 bytes. No line of `xss-corpus.txt` is above the cap.
- Cons: it refuses each numeric text and each text with spaces above 256 bytes. No cap can accept each numeric text, because its length has no limit.

### C1: the answer to a text above the cap

#### Option D: one line `E-WEB-008` from the reader, with no `maxlength`

- Pros: the answer holds no grid, so the server copies no text. The HTMX browser keeps the form and the typed text (ADR-0138 (g)).
- Cons: a user types a bound text, so a line under the field would fit ADR-0041 better.

#### Option E: a line under the field (ADR-0041 (a))

- Pros: it fits the rule of the lists of a form.
- Cons: the refused form then builds its grid from the sent texts (`prepare.rs:117-124`), so the copy of C1 stays. The answer also replaces the form, and the HTMX browser loses the typed text.

#### Option F: a `maxlength` on each limit input

- Pros: the browser refuses the text before it sends it.
- Cons: `maxlength` counts UTF-16 code units and not UTF-8 bytes. A text within the cap in units can exceed it in bytes, so the server check stays necessary. It also changes every grid answer with a limit input. The route tests of ADR-0223 pin the length, the SHA-256, and the peak of each answer. The pins then need new measurements.

#### Option G: the check in `limit_rows`

- Pros: the same refusal.
- Cons: the reader then places every pair first. The check in `place_limit` stops earlier and needs no second pass.

### F3: the window of the record

#### Option H: a window of `RECORD_WINDOW_S` in round 2 only

- Pros: the overlap check does not change. The round holds its bodies for one second more at most.
- Cons: round 1 gets no data.

#### Option I: a read until the barrier deadline

- Pros: the data covers the whole interval before the deadline.
- Cons: each round 2 body stays in the server for up to 8 s. The window gives the same data for a short overshoot.

#### Option J: a window in both rounds

- Pros: round 1 data would show the cause of its larger margin.
- Cons: the owner named round 2. A window in round 1 changes the timing of the round 1 peaks of D1.

#### Option K: a change of the threshold (F1 or F2 of the memo)

- Cons: it changes the rule of ADR-0222 (c) before data exists. The owner rejected it.

### D1: the memory frame

#### Option L: measured readings

- Pros: the frame is the `memory.peak` reading of each round against `soak.memory_ceiling`. The readings include the real process.

#### Option M: the saved frame as a sum of estimates

- Cons: the frame of change B was 4 × 48 MiB, a budget of 64 MiB, and a rest of 128 MiB. The flood reading and the pre-check reading show that the rest is not a constant. The measured readings do not follow the sum.

## Decision

The decision selects Options C, D, H, and L. It rejects Options A, B, E, F, G, I, J, K, and M. The owner decided the cap, the answer, the reason text, the unit, the window, and the round of F3 on 2026-11-27.

C1, the cap in the reader:

- (a) The reader refuses a `min`, `max`, or `max_length` text of a limit row that has more than `form.limit_text_max` bytes.
  - `form.limit_text_max` is 256 bytes. The rule of the value is the longest text that `IecLiteral::parse` accepts, 150 bytes, rounded up to a power of two.
  - The count is the decoded UTF-8 bytes of the text before the trim. This count bounds the text that the grid keeps and copies. A count after the trim lets the spaces through. A count of characters lets the multibyte characters through.
  - `reliability.md` §7.11 holds the value in the new row `form.limit_text_max`, with the unit `bytes`.
  - The value is the code constant `LIMIT_TEXT_MAX` of `crates/recipe-web/src/settings.rs`. It is no field of `WebSettings` and no key of `serve`, because `read_form` has no settings handle.
  - A test compares the constant with the row of §7.11. A second test compares it with the longest time literal and with the stored bounds.
- (b) `place_limit` runs the check when the reader places the pair. The reader thus stops at the first over-long text, as ADR-0138 requires for a changed body.
  - The answer is one line `422 E-WEB-008` with the label `Form`. It has the new reason `LimitText`.
  - The answer holds no out-of-band form and no grid (ADR-0138 (g)). The command does not run and writes nothing (ADR-0138 (h)).
  - The English text of `LimitText` names the cap, for example "a limit field has more than 256 bytes. Make the text shorter". The text in `pl` holds the same number. The reason `LimitRow` stays the reason of a missing leaf path.
  - The check covers every form that carries limit rows: the definition form, the add form, the limits form, and the answers of `fields=1` and `limit_mode`.
- (c) The limit inputs keep no `maxlength`. A user can type a text above the cap into the editor HTML. That text gets the line of item (b).
  - This departs from the premise of ADR-0138 that the editor HTML cannot send a changed body. Items (e) and (f) record the departure for this one fault.
- (d) The `leaf_path` of a limit row keeps its own bound (ADR-0211). A body above the body limit of its route (`api.md` §7.6) keeps `413 E-WEB-003`, and the reader does not run.
  - `/limits` is no value route, so its limit is `http.body_limit.form`.

The text that the cap refuses:

- Each numeric text above 256 bytes. Examples are `1.` with 300 zeros after it and `0e` with 300 digits after it.
- Each text that has spaces so that the text has more than 256 bytes, also a text with no other character.
- Each text with bytes that are not UTF-8, when the decoded text has more than 256 bytes. The decoder replaces each invalid sequence with U+FFFD, which has 3 bytes.
- The cap refuses no valid time literal and no canonical text of the server. The reader already refused a `max_length` of more than 5 digits after the trim, but only in a save. The 5-digit rule is a check of the save.
- The answers of `fields=1` and `limit_mode` skip that check and showed each `max_length` text again. They now refuse an unpadded `max_length` of 257 digits or more.
- The cap also adds its padding of spaces as a new fault, in a save and in these two answers.
- Each such text has a short form. `CHANGELOG.md` names each kind of refused input.

New text of the amended items of ADR-0138:

- (e) ADR-0138 (a) and (b).
  - ADR-0138 (a) gets two sentences after the text of `reason-form`.
    - New text: "The reader gives the text `reason-limit-text` for a bound text of a limit row above `form.limit_text_max` bytes."
    - New text: "The text names the cap."
  - ADR-0138 (b) gets this item: "a bound text of a limit row (`min`, `max`, or `max_length`) with more than `form.limit_text_max` decoded bytes before the trim".
  - The label of this line is `Form`.
- (f) The premise of ADR-0138 and the order of the reads.
  - Context line 19 becomes: "The HTML of the editor sets each key, each hidden field, and each button value of a form. A user types only the values of the inputs. One fault is the exception: a user can type a bound text of a limit row above `form.limit_text_max` bytes (ADR-0225 (c))."
  - The decision driver of line 29 becomes: "A body that the editor HTML cannot send gets a short answer. The same answer covers a bound text above the cap, which a user can type. The server does not check the rest of a changed request."
  - Context line 21 becomes: "The value form and the limits form read the whole body before `row_version`. A fault of (b) or (c) gives one line first. A bad `row_version` gives one line only for a body without such a fault, and the reader then checks no other field."
  - The sub-item of item (c) on line 64 gets this new text:
    - "A bad `row_version` of the value form or the limits form is not a fault of (c)."
    - "A body with a fault of (b) or (c) gives its own line, because the reader checks the body before `row_version`."
    - "A body without such a fault gives one line for the bad `row_version`, and the reader checks no other field."
- (g) `api.md` §2.6 (line 229) gets the new text of its premise sentence. Task 2.3 copies it.
  - Old: "The HTML of the editor never sends such a body, so the server treats it as a changed request (ADR-0138 (b) to (d))."
  - New: "The HTML of the editor sends such a body only when a user types a bound text above `form.limit_text_max` bytes into a limit input."
  - New: "The server treats each such body as a changed request (ADR-0138 (b) to (d), ADR-0225)."
  - The item also gets a sentence after its first sentence.
  - New: "A bound text of a limit row above `form.limit_text_max` bytes is also `422 E-WEB-008`."
- (h) ADR-0211 (f) gets two sentences after its last sentence.
  - New text: "A typed bound text of a limit row above `form.limit_text_max` bytes gets the line of ADR-0225 (b)."
  - New text: "A typed form above the body limit of its route (`api.md` §7.6) keeps `413 E-WEB-003`."

F3, the largest read of round 2:

- (i) In round 2, the poll keeps the first read at or above the threshold as `barrier_anon`. The overlap check does not change.
  - After that read, the poll reads again for `RECORD_WINDOW_S` and sleeps 50 ms between two completed reads. Then it releases the barrier.
  - `RECORD_WINDOW_S` is 1 s. It is a constant of `deploy/ci/load_value.py` next to `BARRIER_TIMEOUT`. It is no setting of `serve`, so §7.11 gets no row.
  - The barrier deadline (8 s) is the upper end of the window. The barrier thus releases no later than the barrier deadline.
  - After the first crossing, the round passes the overlap check. The barrier deadline then ends the window with a pass and not with a failure.
  - A failed read in the window gives `None`. This includes a read that times out. The poll skips the read while the window has time left, and the window does not end.
  - Each read and each sleep in the window has the budget of the earlier end: the end of the window or the barrier deadline. A read after that end does not count.
  - The poll records the largest read as `barrier_anon_max`. It is at least `barrier_anon`.
- (j) The checker computes `overlap_margin_max` as `barrier_anon_max − reference_anon −` the sum of the four body lengths. The summary and `load-value.json` show both new values for round 2.
  - A round without a crossing writes `null` for both values. Its failure stays as today: "the bodies did not overlap", the missing `barrier_anon`, or the request that missed the barrier.
  - The checker requires a count for `barrier_anon_max` when `overlap_margin` is 0 or more. It also requires `barrier_anon_max` at or above `barrier_anon`.
  - The step "Require the values from each leg" checks the shape only. It requires both keys in each round, `null` in round 1, and `null` or a number in round 2.
  - Round 1 keeps the release at the first read. Its timing then stays equal to the runs of D1.
- (k) This ADR amends ADR-0222 (c). The text "The rise of `anon` over the baseline reading of the round must be at least 4 × the body length of that round" stays. The record of `barrier_anon_max` and `overlap_margin_max` follows the record of `overlap_margin`.

D1, the memory frame:

- (l) The frame of the value routes and of the grid answers is the `memory.peak` reading of each round of the value leg against `soak.memory_ceiling`.
  - `reliability.md` §7.8 keeps the measured values of run 45748785867 and its text, as ADR-0222 (f) requires. It adds the peak table of the three runs of the Context.
  - The text states the worst round 1 margins with the run and the build of each reading. It also states the overlap margins of run 45805398657.
  - The text states the known limit: no run measures parallel `GET` pages of a large grid.
  - The `dhat` pins of ADR-0223 stay bounds for one answer. The text does not add them to a sum against the ceiling.
  - This item amends ADR-0222 (f).
  - The Consequences line "Until change B, the ceiling does not gate round 1 (item (a))." of ADR-0222 changes.
  - New: "Until a later change bounds the grid answers, the ceiling does not gate round 1 (item (a))."
- (m) This ADR does not adopt the saved sum of change B. That sum is 4 × 48 MiB, a budget of 64 MiB, and a rest of 128 MiB.
  - The sum of ADR-0223 (v) stays a record of the render peaks of the old round 1 body.

R1, round 1:

- (n) Round 1 stays a report without a gate until a later change bounds the grid answers. This item replaces the two sentences of ADR-0222 (a), lines 83-84:
  - Old: "Change B of ADR-0223 (w) turns the gate of round 1 on again." and "The owner kept this rule on 2026-11-24. Change B decides the gate of round 1 from its measured peak with the body of item (c)."
  - New: "Round 1 stays a report without a gate. A later change decides its gate from measured peaks, after it bounds the grid answers."
  - The exception of ADR-0222 (a) stays: a round 1 value that is `unavailable` or not a number fails both legs.
- (o) The reason: the worst round 1 margin is 19 980 288 bytes on musl. A gate would fail on a small change of the allocator or of the render.
  - The spec of `operations` already says that the job reports round 1 without a gate. This change thus has no delta for R1.
  - D1 changes documents only, so it has no delta either.

The moved items of ADR-0223:

- (p) ADR-0223 (w) after this ADR: D1 (item (l)) decides the memory frame. The budget `form.grid_memory`, the write check of five commands with the port `GridSize` and the code `E-WEB-018`, and the Assumption A54 move to a later change.
  - ADR-0223 (x) follows (w). `form.grid_memory`, `E-WEB-018`, and A54 have no row yet. The later change proposes them. This change takes no A-number, so A54 stays free.
- (q) Options E and F of ADR-0223 (lines 73-74 and 210) move to the later change. The later change decides them.
- (r) These items of ADR-0223 move or close:
  - Line 161, the allowance as an estimate: the allowance stays an estimate. D1 does not use it. The later change may use it.
  - Line 272, the request peaks of the saves: they stay for the later change.
  - Line 281, the start from measured numbers: the later change starts from them.
  - Line 295, the sum of the four refused forms: it moves to the later change. D1 states the frame in readings, but it does not bound the four forms.
  - Line 297, the bound of parallel `GET` pages: it moves to the later change.
  - Line 299, the use of the measured render numbers: the later change uses them.
- (s) "Change B" in ADR-0222 and ADR-0223 means the old plan. After this ADR, each open item of that plan belongs to the later change, except C1, F3, D1, and R1.

The numbers of task 2.4:

- (t) Task 2.4 of the change measures the largest answer at the cap with two ignored `dhat` pins. The table gives the measurements of this fixture. Task 2.4 measured them in three runs, and the lengths, the SHA-256 values, and the peaks stayed equal.
  - The type is `ARRAY[1..16384] OF INT`. The row `[*]` holds a `min`, a `max`, and a `max_length` of 256 bytes of `"` each.

| Pin | Length | SHA-256 | Request peak | Render peak | Time |
|---|---|---|---|---|---|
| `largest_cap_all_htmx_pl` | 48 004 340 | `be872cd5a8119186b1685f538bfac7ccbb890a2a64a1ffd0a33067399cd7f3d9` | 85 535 890 | 82 273 386 | 9.7 s |
| `largest_cap_switch_htmx_pl` | 56 677 591 | `052f398015a6865c57fd72d77c208993f90fd079e02d48aeaea35a301e2356e2` | 88 295 509 | 85 032 983 | 12.5 s |

## Consequences

### Positive

- One request cannot make the grid copy a bound text of more than 256 bytes. The grid of a refused form and the grid of `fields=1` and `limit_mode` have a bound.
- The answer to a text above the cap has one line, and the HTMX browser keeps the typed form.
- The value leg records the largest `anon` of round 2. A later review of the threshold of the overlap check then has data.
- `reliability.md` §7.8 states the memory frame in measured readings.
- ADR-0138 matches the code in the order of the reads and in the premise of the editor HTML.

### Negative

- A user who types a text above the cap gets the line of a changed body and not a line under the field.
- The cap refuses a numeric text or a text with spaces above 256 bytes. The reader accepted such a text before.
- The input has no `maxlength`, so the browser does not warn the user before the send.
- Round 2 holds its bodies for up to 1 s more.
- The parallel `GET` pages of a large grid stay without a bound until the later change.

### Risks and mitigations

- Risk: the cap refuses a valid bound text.
  - Mitigation: the cap is above each canonical text and each valid time literal. Task 2.5 builds the longest literal of each time type and platform. Its test fails when the cap differs from the rule.
- Risk: the worst answer at the cap uses more memory than the pins of ADR-0223 show.
  - Mitigation: task 2.4 measures it. At 256 bytes, the added HTML is 41 945 600 bytes, about 1.9 times the `GET` edit page of 22 569 867 bytes.
- Risk: the record window turns a pass into a failure.
  - Mitigation: after the crossing, the deadline ends the window with a pass, and the poll skips a failed read. Task 3.1 tests both paths with a fake clock.
- Risk: the record window changes the peak of round 2.
  - Mitigation: task 4.1 compares the new peak with the table of D1.
- Risk: one dispatch run gives no spread of the overhead of four bodies.
  - Mitigation: the data is evidence for a later review. This ADR selects no threshold.
- Risk: the plan `machine-link` cites ADR-0224 for the next free number.
  - Mitigation: `main` holds ADR-0224 for `mimalloc`, and this ADR takes 0225. `machine-link` takes the next free number at apply. This change does not edit `machine-link`.

## Related

- ADR-0138 (a), (b), (c), (g), (h): amended by this ADR in (a), (b), (c), and the Context
- ADR-0211 (f), (p): (f) amended by this ADR
- ADR-0222 (a), (c), (f): amended by this ADR
- ADR-0223 (v), (w), (x): (w) and (x) amended by this ADR. Options E and F, and lines 161, 272, 281, 295, 297, and 299, moved by items (q) and (r).
- ADR-0041 (a) (the list of bad fields), ADR-0128 (round gate of the load job), ADR-0224 (`mimalloc`)
- `reliability.md` §7.8, §7.11
- `api.md` §2.6, §7.6
- `overview.md` §7: R30. `overview.md` §9: A26
