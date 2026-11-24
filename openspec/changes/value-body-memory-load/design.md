# Design

## Context

The code review of change `review-design-fixes-2` (finding F06 of the second reviewer, HIGH) reports this gap. Two route tests pass, but no test measures the memory of four value bodies of 16 MiB at the same time. `docs/plan/review/2026-11-21-fix-plan-closure.md` gives the finding to a separate change.

This design checks the facts at `origin/main` `66cb8e9`. That commit holds change A (`parameter-page-memory`, ADR-0223).

- The Load row of `reliability.md` §6 (line 558) asks for parallel `POST` requests with bodies of 16 MiB on the value routes. The container must stay below `soak.memory_ceiling`. The row also asks for `503 E-WEB-009` after a wait longer than `http.body_read_timeout`.
- ADR-0128 (ac) applies the soak ceiling and the p99 bound to the soak only. Thus ADR-0128 and the §6 row disagree.
- ADR-0128 (i) lets only the musl leg gate the bounds. ADR-0128 (j) fixes the order of the steps, and `deploy/ci/check.sh` checks that order. After the flood, the per-IP budget blocks each login for 15 minutes (ADR-0128 (k)).
- ADR-0211 (p) and `reliability.md` §7.8 (line 923) give an estimate. One body of 16 MiB uses about 48 MiB. Four bodies use about 192 MiB.
- `deploy/ci/load.sh` has three steps: `memory-timings`, `flood`, and `memory-check`. No step sends a value body. `memory_cgroup` of `memory.sh` reads `memory.peak`, `anon`, `file`, and `oom_kill`, but not `memory.current`.
- `read_memory` of `load.sh` (line 35) writes each value of a reading as a string. `flood` calls `recreate_app` before its requests (line 109).
- `memory.peak` is the high-water mark of the cgroup since the start of the container. It includes the file cache. It is the only peak value of cgroup v2.
- The route test `largest_edit_form_fits_the_value_limit` (`crates/recipe-web/src/pages/parameters/route_tests/largest_form_tests.rs:360`) builds a form of 11 617 429 bytes (11.08 MiB). Its type has 32 array levels and 16 384 `LDT` leaves. Each leaf path already has 128 bytes.
- `form_pairs` of that file (line 116) gives the head pairs in this order: `row_version`, `unit_path`, `tag`, `name_en`, `description_en`, `s7_address`, `ordinal`, `type`, and `value_type`.
- `call` of `crates/recipe-web/src/test_support.rs` (line 422) reads an answer of at most 1 MiB (`BODY_LIMIT`). A larger answer panics. `call_bounded` (line 429), `get_bounded`, and `post_fields_bounded` read an answer up to a named limit.
- `refused_pairs` and `refused_body` of `route_tests/largest_refused_tests.rs` (lines 64 and 69) build the refused form of change A with 16 384 leaves. They pin 16 758 074 bytes and the SHA-256 `bef15742...d6bd`. Before D9, this form was the round 1 body of D3.
- The refused page of that form has at most 27 708 942 bytes. This is the pin `REFUSED_PAGE_EN` of `largest_refused_tests.rs` (line 215).
- ADR-0223 (v) gives a heap sum of 894 398 804 bytes for four of these refused forms. The sum is above `soak.memory_ceiling` (402 653 184 bytes) and above `container.app_memory` (536 870 912 bytes).
- `form()` (`test_kit.rs:101`) joins the pairs with `&` and encodes each key and value with `encode()` (`test_helpers.rs:111`). `encode()` keeps only the unreserved characters, so a space is `%20`.
- `Client.post` of `deploy/ci/load_client.py` adds `csrf_token` to the body and encodes a space as `+`.
- `resolve_type` takes the class-2 lock of the `ARRAY` key of the machine until the commit (`definition_steps.rs:72`, `data-model.md` §1.4). Two drafts on one machine thus wait for each other, and `db.lock_timeout` is 2 s.
- The grammar of an `LDT` text uses fixed digit counts (`domain-model.md` §2.1). An index of a `LeafPath` is `-?[0-9]{1,10}`.
- `fifth_value_body_waits_and_gets_e_web_009` (`crates/recipe-web/src/limits/route_tests.rs:467`) checks the `503` after the wait for a permit.
- `http.value_body_permits` is a compiled default (`crates/recipe-web/src/settings.rs:23`). No configuration key of `recipe-server` reads it.
- The machine platform `CodesysIec` has `LDT` (`domain-model.md` §2) and no S7 address (I5).
- The Caddyfile has no `request_buffers`, so Caddy streams the request body to `app`.
- A valid form with `LDT` leaves has at most about 13 206 677 bytes. The alias form of change A has 12 158 101 bytes. The window of D3 starts at 16 711 680 bytes.
- `parameter_value_size` (`migrations/0001_schema.sql` line 431) is `CHECK (pg_column_size(value) <= 1048576)`. It bounds the stored JSONB size, not the JSON text.
- The in-memory store of `recipe-app` has no `parameter_value_size`. A route test can thus save a value that PostgreSQL refuses.
- `ready` of `prepare.rs` builds the leaf texts (line 148) and the limit list (line 149). Then it refuses the form when a line exists or the limit list fails (lines 150 to 152).
- The refusal runs `refuse_checked` (lines 117 to 133). It builds the sent grid (line 123) and, after a bad limit row, the lines of the limit rows with that grid (line 124).
- A save that passes every check builds no grid (ADR-0223 (p), `prepare.rs` line 137).
- A bad `max_length` text gives a line that names the input of its leaf path (`length_line` of `fields.rs`, line 624, ADR-0223 (g)). The reader refuses a bad value key first, with one line of the field `form` (ADR-0138).
- `LeafTexts::parse` refuses only a path with `[*]` and a path twice. The reader refuses both first. Thus no value field gives a line in `leaf_texts`.
- The command parses the `LDT` texts in `stage_definition` (`crates/recipe-app/src/recipe/update_definition.rs`). An over-long `description_en` stops the form in `prepare`. The parse of the value texts then does not run in round 1.
- An over-long `description_en` gives a field line `E-WEB-008` under `description_en`. Without `HX-Request`, the answer is the full page of the refused form (`answer.rs` line 390, ADR-0041 (e)).
- A field line is `<div class="field-error" id="<id>-error">` with its code in `<span class="field-error-code">` (`templates/components/field.html` line 7).
- A changed form body gives one line `E-WEB-008` of the field `form` and no form (ADR-0138).

## Goals / Non-Goals

**Goals:**

- Measure the memory of four value bodies of nearly 16 MiB that the server holds at the same time, on the real Compose stack.
- Measure the memory of four write forms on the write path.
- Fail the job when the peak of round 2 reaches `soak.memory_ceiling`, and when the four bodies of a round do not overlap.
- Report the peak of round 1 without a gate, so change B starts from a measured value (D8).
- Replace the estimate of ADR-0211 (p) with a measured value after the first green run.

**Non-Goals:**

- No change to the code of the server, to a setting, or to an error code.
- No fix for a peak above the ceiling. The owner decides it after the measurement.
- No check of the permit count by the leg. A route test keeps that check (D2).

## Decisions

### D1. Ceiling and metric (owner, 2026-11-22)

- The value leg reads `memory.peak` of the container `app` of each round. The absolute peak of the round 2 reading must be below `soak.memory_ceiling` (384 MiB, `reliability.md` §7.11).
- The leg also applies the bounds of ADR-0128 (z) to each value reading: no `oom`, `oom_kill`, or `max` event, no `OOMKilled`, and no restart.
- The round 1 reading is the exception of D8. Its ceiling, its bounds of ADR-0128 (z), and its restart count only report.
- As in ADR-0128 (i), these bounds gate the musl leg. The `cc-debian13` leg reports them and has `continue-on-error`. A missing or invalid value fails both legs in the step "Require the values from each leg".
- Each round runs on its own new container `app` (owner, 2026-11-22, after the second plan review). The leg runs `recreate_app` and a baseline reading before each round.
- A new container has a new allocator. Round 2 thus cannot use the freed pages of round 1, and the rise of `anon` of each round stays the proof of the overlap.
- Each reading records `memory.peak`, `memory.current`, `anon`, and the container ID.
- The checker requires a different container ID for each round and the same ID for a round and its baseline. It also requires the same four distinct draft keys in both rounds.
- The value leg takes six readings: `value-seed`, `value-precheck`, `value-baseline-1`, `value-round-1`, `value-baseline-2`, and `value-round-2`. D5 gives their places.
- Before each `recreate_app` of the value leg, the leg reads the retiring container. The `value-precheck` reading comes before the first `recreate_app`. The round 1 reading comes before the second `recreate_app`.
- The run statistic of round n is `(peak of the round n reading − memory.current of the baseline n reading) / 4`. The summary shows it next to 3 × the body length of that round.
- The statistic is not a measured value for each body. It is an upper bound, because it also holds the parse stages and, in round 1, the render of the answers. Round 2 renders no grid (D3).
- The summary shows the container ID, the peak, and the run statistic of each round.
- The job has eight readings: the timings reading, the flood reading, and the six value readings. The peak of the whole run is the largest of the eight peaks.
- `load-memory.json` keeps the flood reading and the value readings separate.
- Rejected: a bound at `container.app_memory` (512 MiB) only. It does not test the claim of ADR-0211 (p).
- Rejected: both rounds on one container. Round 2 can then use the freed pages of round 1, so the rise of `anon` does not prove the overlap.
- ADR-0222 records this decision. It amends ADR-0128 (y), (aa), and (ac). D8 also amends ADR-0128 (z).

### D2. The `503` and the permit count stay with the route tests (owner, 2026-11-22)

- The route test `fifth_value_body_waits_and_gets_e_web_009` keeps the check of the `503` and of the permit count. The value leg checks memory and the overlap.
- The leg cannot detect extra permits. Four requests also fit with five permits. ADR-0222 (c) states this limit.
- ADR-0222 changes the Load row of §6. The row names the route test for the `503`.
- Rejected: four slow requests that hold the permits for more than 10 s. The body deadline of a holder also ends after 10 s, so the test can be flaky.

### D3. The requests (owner, 2026-11-22)

- Each round sends four bodies in parallel to `POST /recipes/{reference}/versions/1/parameters/{key}/definition`. D6 defines the two bodies.
- The round 2 body is the write form. It is the edit form of the fixture of D6, and every `LDT` text has the alias prefix `LDATE_AND_TIME#` of ADR-0205. Its bounds are the fixture bounds of D6.
- Both bodies use the value shape of D9: 32 array levels and 4 096 leaves. Thus `parameter_value_size` accepts the round 2 value.
- The round 1 body is the write form with an over-long `description_en`. The server refuses only `description_en`. The encoded length of the round 1 body lies in the window from 16 MiB − 64 KiB to 16 MiB.
- The round 1 body makes the server read the whole body, build the pairs, and build the leaf texts and the limit list. These are the three buffers of ADR-0211 (p). Only then does `prepare` refuse it with `422` (`prepare.rs` lines 148 to 152).
- The refusal of a round 1 body builds the sent grid and renders the refused page. The page has 32 mode rows and 4 096 leaf rows (ADR-0223 (a), (k)).
- A round 2 body passes every check of `prepare`, so it builds no grid (ADR-0223 (p)). Its answer `303` renders no page. Thus the peak of round 2 holds no row model and no HTML of a grid.
- The parse of the `LDT` texts in `stage_definition` happens only in round 2.
- The round 1 request writes nothing. Task 2.1 proves both facts with tests.
- The round 1 body has no unknown, repeated, or malformed key. Such a key gives `E-WEB-008` of the field `form` and stops the reader before the texts (ADR-0138).
- The order test of D6 proves that the round 1 body reaches the check of the limit rows. A code reference to `prepare.rs` line 149 pins that `ready` builds the limit list before the refusal.
- If a change of `prepare` makes the refusal come before `leaf_texts` or `limit_list`, the implementer stops and reports the code path.
- The seed creates five machines `M9000002` to `M9000006` with the platform `CodesysIec`. Each machine gets one recipe, `LOAD-V1` to `LOAD-V5`. Thus each draft has its own class-2 `ARRAY` key, and no request waits for the lock of another request.
- Each draft gets one parameter `BIG` of the type `LDT` with the value `LDT#1970-01-01-00:00:00.000000001`. The seed uses the HTTP routes and the client of `deploy/ci/load_client.py`.
- The drafts `LOAD-V1` to `LOAD-V4` get the measured requests of both rounds. The draft `LOAD-V5` gets the pre-check of D5.
- The requests have no `HX-Request` header. A round 1 request must get `422` with the first field line `E-WEB-008` under `description_en`.
- A round 2 request must get `303` with the exact `Location` `/recipes/{reference}/versions/1/parameters/{key}` of its key.
- After round 1, the `row_version` of each draft of `LOAD-V1` to `LOAD-V4` must equal its value in the state file. After round 2, it must be larger.
- The pre-check changes the `row_version` of `LOAD-V5`. After each round, the leg compares `LOAD-V5` with `load-value-versions-precheck.json`.

### D4. Proof of the overlap

- The client runs once for each round. Each round has its own barrier, its own shared deadline, and its own proof of the overlap.
- The client of a round opens four connections through Caddy (`https://localhost:8443`), with the root certificate of `login.sh`. Each connection has its own thread.
- The four threads of a round share one deadline of 8 s from their common start. The value of `http.body_read_timeout` is 10 s, so each server body deadline is still open at the shared deadline.
- Each socket has a timeout, so a blocked `sendall` ends at the shared deadline.
- Each thread sends the headers and the body without its last byte. Then it waits at a barrier until the shared deadline.
- A thread that does not reach the barrier fails the leg. The error is "round <n>: request <reference> did not reach the barrier".
- The client barrier is the trigger only. TCP, `docker-proxy`, and Caddy can buffer a part of each body.
- When all four threads reach the barrier, the client reads `anon` of `memory.stat` of `app` with `sudo -n`, inside the shared deadline. `anon` excludes the socket buffers.
- The reference is the `anon` of the baseline reading of that round.
- The rise of `anon` over the reference must be at least 4 × the body length of that round. The client reads `anon` again until the rise reaches that bound or the shared deadline ends.
- Otherwise the leg fails with "round <n>: the bodies did not overlap".
- Then each thread sends its last byte. The collection of the answers lasts at most `http.request_timeout` (30 s).
- The client reads each answer to its end in chunks. It keeps only the last 4 KiB of the earlier chunk, so a match across two chunks stays possible.
- The client never keeps the HTML or a token after the match.
- For each answer, the client records the status, the `Location`, the code, and the field name of the first field line. It uses the fixed pattern of the section "Interfaces".
- The client also records the body length and the times of the last byte and of the answer. It records the `anon` of the barrier.
- The client reads `MemAvailable` of `/proc/meminfo` of the runner at the barrier and after the last answer. It records the smaller value.
- The leg checks round 1 before it starts round 2 (D5). A failure of round 1 ends the leg before round 2. The leg still takes the round 1 reading.

### D5. Place in the job `load`

- A new step `value-seed` runs after the step `memory-timings` and before the flood. It seeds the five drafts of D3 and logs in once.
- `value-seed` writes `load-value-state.json` with mode 0600 in `$CI_STATE`. The file holds the session cookie, the CSRF token, and the fields of each draft (section "Interfaces").
- At its end, `value-seed` takes the reading `value-seed`. The flood replaces `app` later, so only this reading holds the peak of the seed.
- The value leg does not log in again. The `session` table keeps the session over `recreate_app` (ADR-0014). The steps from `value-seed` to `value` take less than `session.idle_timeout` (30 min).
- The sender writes the pinned bytes directly. It sends the token in the header `X-CSRF-Token` with a matching `Origin` header, not in the body.
- A new step `value` runs after the flood. It does these actions in this order:
  1. It sends the round 1 body and then the round 2 body to `LOAD-V5` in the single-request mode, and records both durations.
  2. It reads the `row_version` of `LOAD-V5` from the database and runs the check of the pre-check.
  3. It takes the reading `value-precheck`, also after a failed pre-check. After a failed pre-check, the leg stops here. Else it runs `recreate_app` and takes the reading `value-baseline-1`.
  4. It runs round 1 with the client of D4 and takes the reading `value-round-1`.
  5. It reads the `row_version` of each draft from the database and runs the check of round 1. A failure ends the leg before round 2.
  6. It runs `recreate_app` and takes the reading `value-baseline-2`.
  7. It runs round 2 with the client of D4 and takes the reading `value-round-2`.
  8. It reads the `row_version` of each draft from the database again.
- A failed pre-check is a failure in action 1 or 2: a failed `single`, a failed read of the `row_version`, or a failed check. The leg then skips the rest of actions 1 and 2.
- After a failed pre-check, the leg returns the status of the first failure, as `value_round` does after a failed round (owner, 2026-11-24, D9).
- The check of the pre-check requires `422` with the first field line `E-WEB-008` under `description_en`. Then it requires `303` with the exact `Location` of `LOAD-V5` and a larger `row_version`.
- The check of round 1 requires four answers `422`, each with the first field line `E-WEB-008` under `description_en`. It also requires the `row_version` of the state file for `LOAD-V1` to `LOAD-V4` and the `row_version` of the pre-check for `LOAD-V5`.
- Only the round 1 requests and the round 1 body of the pre-check render a grid answer. The round 2 saves build no grid (D3, ADR-0223 (p)). Thus the reading `value-round-2` holds no grid answer.
- The step `memory-check` then runs after `value` and checks the readings of D1.
- A new step `value-check` runs the checker of `deploy/ci/load_value.py`. It writes `load-value.json` and the lines of the job summary.
- The steps `value`, the memory check, `value-check`, and "Require the values from each leg" have the condition `${{ !cancelled() && steps.seed.outcome == 'success' }}`. Thus `value-check` also runs after a failed `value`.
- When round 2 did not run, `check` writes the lines of each reading and each answers file that exists. It also writes the OOM counts of round 1 and the cause of the stop. Then it fails.
- After a stop, the line of each value reading also holds its OOM counts and its restart count, also for `value-precheck` (D9).
- The upload step keeps its condition. After a stop in round 1, the summary thus holds the round 1 reading, but the job has no artifact.
- The summary records the elapsed time of the leg, the elapsed time of the job, and the smallest `MemAvailable` of each round.
- The step "Check the runner before the stack" also writes the start time of the job to `$RUNNER_TEMP/load-job-start`. This keeps the step order of ADR-0128 (j).
- `value-seed` writes the start time of the leg to `$CI_STATE/load-value-leg-start`. Each file holds one line of epoch seconds.
- The steps `value` and `value-check` have `continue-on-error` for `cc-debian13`, like the flood (ADR-0128 (i)). The step `value-seed` has none.
- Each new step has `timeout-minutes`. The step "Require the values from each leg" also requires the values of `load-value.json`, and the artifact holds that file.
- ADR-0222 records this order. It amends ADR-0128 (j) and (k).

### D6. Two bodies in two generators

- New route tests in `crates/recipe-web/src/pages/parameters/route_tests/` build the two bodies of D3. The old test `largest_edit_form_fits_the_value_limit` stays.
- The tests use the test support of change A: `post_fields_bounded` of `crates/recipe-web/src/test_support.rs` and the fixture of `largest_form_tests.rs`. The fixture takes the number of wide levels as an argument (D9).
- The limit of an answer is `ANSWER_LIMIT`, 32 MiB (33 554 432 bytes). Task 2A.1 measures the longest round 1 page: `19 478 174` bytes. The margin is `14 076 258` bytes.
- The round 1 test reads its answer with `post_fields_bounded`. It proves `422`, a list of field lines equal to the one line `E-WEB-008` under `description_en`, and no write.
- The round 1 test pins the length and the SHA-256 of the body.
- The order test sends the round 1 body with one bad `max_length` text in one limit row. Its `description_en` is shorter by the length of the extra pair, so the body stays at or below 16 MiB.
- The `422` answer of the order test must list the line of `description_en` and the line of that limit row. This proves that the round 1 body reaches the check of the limit rows.
- `refuse_checked` builds the line of the limit row again with the grid (line 124). Thus the answer does not prove the order. A code reference to `prepare.rs` line 149 pins it, as line 148 pins `leaf_texts`.
- Only a `String` leaf has a `max_length` input (`grid.rs` line 69). The grid of the fixture thus has no `max_length` input, and `max_length_ids` is empty.
- `length_line` then keeps the sent id `limits-0-max_length` (ADR-0223 (g)). The line stands only in the summary, with the link `href="#limits-0-max_length"`. The test `limit_line_of_a_maximum_length_on_a_row_without_that_input_keeps_its_sent_id` pins this rule.
- No value field can give a line in `leaf_texts`, because the reader refuses each bad value key first. Thus no test line proves the place of `leaf_texts`. ADR-0222 (c) records this as a known limit (owner, 2026-11-22).
- The round 2 test proves `303` with the exact `Location` on the in-memory store. It pins the length and the SHA-256 of the body.
- The round 2 test also pins the length and the SHA-256 of the compact JSON text of the stored value. The `recipe-db` test of D9 pins the same two values.
- The three route tests have no `#[ignore]` (owner, 2026-11-24). They pin a length and a SHA-256, so the normal test run must catch a change.
- The port of task 2.1 runs each test alone in the library tests, with its exact name. The time of a test is the value of the libtest line `finished in`, in a run that reports one passed test.
- When one test takes more than 30 s, the agent stops and reports it to the owner.
- A Python function in `deploy/ci/load_value.py` builds the same two bodies from ordered lists of pairs.
- The Python encoder uses `urllib.parse.quote(value, safe="")` on each key and value, as `encode()` does. It joins the pairs with `&`, as `form()` does. A space is `%20`. `urllib.parse.urlencode` with `+` is wrong.
- A unit test of `deploy/ci/` asserts the four pinned values for the two Python bodies. So a change of one generator fails one test.
- All tests use the head of the route test. The leg changes only the head fields `row_version`, `unit_path`, and `ordinal`. The checker requires that each live round 1 body length also lies in the window of D3.

#### Byte recipe

Task 2.2 copied this list from the report of task 2.1 on 2026-11-24. The shape of D9 changed the list on 2026-11-24. Task 2A.3 copies the values of task 2A.1 into it.

- The ordered head pairs, with the value of each pair. The base is `form_pairs()` of `route_tests/largest_form_tests.rs`.
  - `row_version=1`
  - `unit_path=U1`
  - `tag=BIG`
  - `name_en=Big`
  - `description_en`: the padding of round 1 (see the padding item below), or `The largest edit form` in round 2.
  - `s7_address=` (empty)
  - `ordinal=0`
  - Then `type` and `value_type`, in this order. Both hold the type text: 12 times `ARRAY[10..11] OF `, 20 times `ARRAY[10..10] OF `, and `LDT`.
- The fixture bounds:
  - 32 array levels.
  - 4 096 leaves (2^12), numbered 0 to 4 095.
  - Each leaf path has 128 bytes: 32 segments of 4 bytes.
- The order of the leaves and of the pairs of one leaf:
  - The leaves appear in ascending order, from leaf 0 to leaf 4 095.
  - The pairs of leaf `i` appear in this order: `value{path}`, `limits[i].leaf_path`, `limits[i].min`, `limits[i].max`.
  - The path of leaf `i` writes `i` as 12 binary digits, most significant first. Level `l` below 12 gets `[11]` when digit `l` is 1, else `[10]`. Levels 12 to 31 get `[10]`.
- The value text, the minimum text, and the maximum text of each leaf:
  - The value text of leaf `i` is `LDT#2262-04-11-23:47:16.854775`, followed by `i mod 800` with three digits (`value_text`).
  - The minimum text is `LDT#1970-01-01-00:00:00.000000001`.
  - The maximum text is `LDT#2262-04-11-23:47:16.854775807`.
  - The alias of `aliased_pairs` in `largest_refused_tests.rs` replaces the first `LDT#` of each text with `LDATE_AND_TIME#`. Each replacement adds 11 bytes.
- The padding of `description_en` in the round 1 body:
  - The padding character is `x`. Its length is `13 710 000` characters (`FULL_FORM_PADDING` of `value_leg_tests.rs`).
  - `FULL_FORM_PADDING` is the multiple of 10 000 that gives the body length nearest to 16 744 448 bytes, the middle of the window of D3.
  - The order body is the round 1 body with the `x` count shorter by `form(&[BAD_LENGTH_PAIR]).len() + 1`. The pair `limits[0].max_length=x` comes last.
- `FULL_FORM_BYTES` is `16 743 514`. `FULL_FORM_SHA256` is `354b1a3e4c5ff56400dc92f6f3ddf84fc104390e3334269d852b3ebae599ccc9`. The code names these pins in `value_leg_tests.rs`.
- `WRITE_FORM_BYTES` is `3 033 541`. `WRITE_FORM_SHA256` is `631700fb82002bb379b37cc2cc9a7d6d9ef2dd628769e4ac3dd9a517f3076022`. The code names these pins in `value_leg_tests.rs`.
- `VALUE_JSON_BYTES` is `253 949`. `VALUE_JSON_SHA256` is `3f6aacb71efb84742bbb8fc0ccdc435e9869550da22c89f1b74fec934d3d3527`. They pin `serde_json::to_string` of the stored round 2 value.
- The form of change A keeps 14 wide levels, 16 384 leaves, `PADDING` (4 600 000), `BODY_BYTES`, and `BODY_SHA256` of `largest_refused_tests.rs`.

### D7. The measured value

- `reliability.md` §7.8 then states the measured peak, the body length, and the run statistic of each round of D1. This edit follows the first green dispatch run of the branch. It also says that the job `load` checks the ceiling on round 2 of the musl leg and reports round 1 only (D8).
- The text compares each run statistic with 3 × the body length of its round, not with a fixed 48 MiB.
- The text replaces "about 4 × 48 MiB". ADR-0222 item (f) amends ADR-0211 (p) and permits this edit.

### D8. The gate of round 1 (owner, 2026-11-24)

Change A measured the refused page of round 1. ADR-0223 (v) gives a heap sum of 894 398 804 bytes for four refused forms. The ceiling is 402 653 184 bytes.

- The sum is also above `container.app_memory` (536 870 912 bytes). Round 1 may thus end in an OOM kill. Then the answers of round 1 can fail.
- Round 1 only measures and reports its `memory.peak`. Its ceiling, the bounds of ADR-0128 (z), and the restart count do not gate.
- A missing or invalid value of the round 1 reading still fails both legs in the step "Require the values from each leg".
- The answer checks, the `row_version` checks, and the overlap checks of round 1 still gate. An OOM kill that ends a round 1 request thus fails the answers, and the leg ends before round 2. An OOM count of round 1 without a failed check only reports.
- Round 2 keeps the gate of the ceiling. The pre-check keeps its checks of the answers.
- Mutation B moves to round 2. N comes from the round 2 peak and the round 2 body length.
- The mutation keeps its copies only for a value body below 16 711 680 bytes, the start of the window of D3. Thus a round 1 body gets no copies. The round 2 body of the pre-check also gets copies.
- Change B turns the gate of round 1 on again (design of `parameter-page-memory`, section "Deferred to change B").
- ADR-0222 (a), the Load row of `reliability.md` §6, and the requirement "Memory of the value bodies under load" state this decision. The modified requirement "Memory of the container app under load" also states it. ADR-0222 (a) thus also amends ADR-0128 (z).
- Rejected: round 1 keeps the gate. The musl leg then stays red until change B, and the change cannot archive before change B.
- Rejected: a smaller round 1 with a small grid. Its smallest refused shape still has a sum of 432 290 116 bytes, and change B then gets no measured peak of the grid answers.
- On 2026-11-24 the owner kept `continue-on-error` on the step `value-check` of the `cc-debian13` leg, as in ADR-0128 (i). Tasks 3.1 and 3.8 read `failures` of that leg by hand.
- On 2026-11-24 the owner decided that a stop in round 1 gives no artifact. The job summary holds the result of round 1 (task 2.10).

### D9. The value shape after CI run 45444532064 (owner, 2026-11-24)

The first dispatch run of the branch failed in the pre-check of both legs.

- musl: the round 2 body to `LOAD-V5` got `409 E-DB-003`. PostgreSQL logged a violation of `parameter_value_size`.
- The round 2 value of 16 384 leaves has about 0.94 MiB of JSON text. A model of the JSONB format gives about 2.7 MiB. This value is an estimate, not a measurement.
- The route tests passed, because the in-memory store of `recipe-app` has no `parameter_value_size`.
- `cc-debian13`: the app restarted inside the round 2 transaction of the pre-check. The cause is unproven, and an OOM kill is likely.
- No memory reading of the pre-check exists, because `run_value` stopped before the reading `value-precheck`.

The decisions:

- Both bodies keep 32 array levels. 12 levels are `ARRAY[10..11] OF`, and 20 levels are `ARRAY[10..10] OF`. The value thus has 4 096 leaves (2^12).
- Round 1 keeps the window of D3. The padding of `description_en` grows and fills the window.
- The fixture of `largest_form_tests.rs` takes the number of wide levels as an argument. The tests of change A keep 14 wide levels, their pins, and the behavior of `main`.
- New pins replace the pins of D6. Task 2A.1 measures them. Task 2A.3 copies them into D6 and ADR-0222 (c).
- A new `recipe-db` test writes the round 2 value into PostgreSQL with the statement of the adapter. It proves that `parameter_value_size` accepts the value.
- The `recipe-db` test measures `pg_column_size` of the round 2 value: `26 964` bytes. It needs Docker (`reliability.md` §6).
- The leg takes the reading `value-precheck` also after a failed pre-check, as `value_round` does after a failed round. This changes actions 2 and 3 of D5.
- After a stop, the summary shows the OOM counts and the restart count of each value reading. A stop gives no artifact, so only the summary holds these values.
- The owner chose option (a) of the diagnosis. Rejected: (b) 16 384 leaves with fewer array levels.
- Rejected: (c) a larger `parameter_value_size` through a migration and an ADR. The proposal puts a change of a limit out of scope.
- The heap sum of ADR-0223 (v) describes the old round 1 body of 16 384 leaves. The sum of the rejected smaller round 1 of D8 also uses the old fixture. No measured heap sum exists for the new round 1 body.

#### Deferred to change B

- The app does not refuse a value before the database when its JSONB is larger than `parameter_value_size`. PostgreSQL then refuses the save with `409 E-DB-003`.
- Change A assumed that PostgreSQL can store the largest valid parameter. The value of 16 384 leaves of change A passes the route tests, but PostgreSQL refused it in CI run 45444532064.
- The round 2 gate of this change thus covers the largest value of the fixture of D9 that PostgreSQL stores. It does not cover the largest form that the app accepts.
- This change does not fix the finding.

## Interfaces

Tasks 2.3 to 2.9 and tasks 2A.4 to 2A.6 use these names. Each file is in `$CI_STATE` and holds one JSON object.

### Commands

- `python3 -I -B deploy/ci/load_seed.py value` seeds the drafts of D3 and writes the state file. Without the argument `value`, the old seed runs.
- `python3 -I -B deploy/ci/load_value.py single --ci-state DIR --round N --reference REF --url URL --cacert PATH` sends one body and writes `load-value-precheck-<N>.json`.
- `python3 -I -B deploy/ci/load_value.py round --ci-state DIR --round N --cgroup PATH --reference-anon BYTES --url URL --cacert PATH [--deadline SECONDS]` runs one round of D4.
- The round command uses the drafts `LOAD-V1` to `LOAD-V4`, writes `load-value-answers-<N>.json`, and has a default deadline of 8 s.
- `python3 -I -B deploy/ci/load_value.py check-precheck --ci-state DIR` runs the check of the pre-check of D5.
- `python3 -I -B deploy/ci/load_value.py check-round --ci-state DIR --round 1` runs the check of round 1 of D5.
- `python3 -I -B deploy/ci/load_value.py check --ci-state DIR --job-start PATH --summary PATH` writes `load-value.json` and appends the summary lines to `PATH`.
- `check` reads the start time of the job from `--job-start` and the start time of the leg from `load-value-leg-start` (D5).
- `python3 -I -B deploy/ci/load_memory.py TIMINGS FLOOD --value PHASE=PATH ... --output PATH` checks the eight readings of D1. With `--value`, it requires each of the six value phases.
- Without `--value`, `load_memory.py` checks only the timings reading and the flood reading. `bash deploy/ci/load.sh` without an argument (`all`) runs the timings reading, the flood, and this check of two readings.
- `bash deploy/ci/load.sh value-seed`, `bash deploy/ci/load.sh value`, and `bash deploy/ci/load.sh value-check` run the steps of D5.

### Exit statuses

- Each Python command exits with 0 on a pass, 1 on a failed check or a transport error, and 2 on a usage error.
- Each failure writes one line to standard error. No line holds a cookie, a token, or HTML.
- Each new step of `load.sh` exits with the status of its first failed command. `fail` of `common.sh` exits with 1.

### State file `load-value-state.json`

- Mode 0600. Writer: `load_seed.py value`.
- `cookie` (string): the value of the `Cookie` header. `csrf_token` (string).
- `drafts` (array of five objects, in the order `LOAD-V1` to `LOAD-V5`). Each object has these keys:
  - `reference` (string), `machine` (string), and `key` (string, a UUID).
  - `row_version` (integer), `unit_path` (string), and `ordinal` (integer).

### Answer object

- `round` (integer, 1 or 2), `reference` (string), `key` (string), and `body_bytes` (integer).
- `status` (integer, or null without an answer) and `location` (string or null).
- `code` (string or null) and `field` (string or null): the code and the field name of the first field line.
- `last_byte_s` and `answer_s` (number or null): the seconds from the common start.
- `error` (string or null): the error text of D4.
- The field pattern is `<div class="field-error" id="([a-z0-9_.\[\]-]+)-error">`. The code pattern is `<span class="field-error-code">(E-[A-Z]{3}-[0-9]{3})</span>`. Task 2.1 checks both patterns on the round 1 answer.

### Answers files

- `load-value-precheck-<N>.json`: one answer object.
- `load-value-answers-<N>.json`: `round` (integer), `reference_anon` (integer), `barrier_anon` (integer or null), `mem_available_min_kib` (integer), and `answers` (array of four answer objects).

### Version files

- `load-value-versions-<PHASE>.json` with `PHASE` equal to `precheck`, `1`, or `2`. Writer: `load.sh`.
- `row_versions` (object): each reference of the five drafts maps to its `row_version` (integer).

### Readings

- `load-memory-<PHASE>.json`. Writer: `read_memory` of `load.sh`. `PHASE` is `timings`, `flood`, or one of the six value phases of D1.
- Keys: `container_id`, `peak`, `current`, `anon`, `file`, `limit`, `oom_kill`, `oom`, `max`, `OOMKilled`, and `RestartCount`.
- Each value is a string. `container_id` has 64 lowercase hexadecimal digits. `OOMKilled` is `true` or `false`. Each other value is decimal digits. Each value can also be `unavailable`.
- After a failed pre-check, `load-memory-value-precheck.json` exists, and no later value reading exists (D5).

### Results

- `load-value.json`: `passed` (boolean), `failures` (array of strings), `job_elapsed_s` (integer), and `leg_elapsed_s` (integer).
- `check` collects every failure into `failures`. It does not stop at the first failure.
- It also holds `rounds` (array with one object for each round that has a reading). Each object has `round`, `container_id`, `peak`, `body_bytes`, `run_statistic`, and `mem_available_min_kib`.
- `load-memory.json` keeps its keys. It adds `value` (object): each value phase maps to its reading. Its `peak` is the peak of the whole run.

## Risks / Trade-offs

- [The peak includes the file cache and the start of the server.] → The ceiling is then stricter than a bound on the heap. The baseline readings show the share of the start.
- [The four parse stages run on two Tokio workers and one CPU.] → The barrier holds four byte buffers at the same time. The parse stages can overlap only in part. ADR-0222 names this limit.
- [A later change of `prepare` can refuse before the limit list.] → The order test of task 2.1 fails on that change. Only code references to `prepare.rs` lines 148 and 149 pin the order (D6).
- [The `422` answer renders the refused form with every sent text.] → The peak of round 1 includes this render. The summary shows the peak of each round.
- [The allocator can keep the freed pages of round 1.] → Each round runs on its own new container. This risk thus does not apply.
- [The leg does not detect extra permits.] → The route test keeps the permit count. A mutation that keeps extra copies of each body proves that the ceiling detects extra memory.
- [The leg does not run Argon2 at the same time.] → The sum of 268 MiB of ADR-0211 (p) stays an estimate. The proposal names it out of scope.
- [The job `load` does not run on a pull request (ADR-0128 (a)).] → The check and the mutation proof need a dispatched run. The owner approves each push.
- [The new steps extend a job of 60 minutes.] → Each step has a timeout. The summary records the elapsed time of the job. Task 3.1 checks a reserve of at least 5 minutes.
- [Four large answers use the memory of the runner.] → The client records the smallest `MemAvailable` of each round.
- [The round 2 peak reaches 384 MiB.] → The job fails. The orchestrator stops and asks the owner. No task changes a limit.
- [The old round 1 body needs about 894 MB by the heap sum of ADR-0223 (v).] → The round 1 reading only reports (D8). The sum assumes that the four renders overlap fully, so the real peak may be lower.
- [An OOM kill in round 1 can end the leg before round 2.] → Then the answers of round 1 fail. The summary still shows the round 1 reading. The orchestrator stops and asks the owner, because round 2 then gives no proof.
- [Mutation B can add copies to the round 1 bodies.] → The mutation copies only a body below the window of D3. Round 1 thus gets no copies (D8).
- [The in-memory store has no `parameter_value_size`.] → A route test can pass with a value that PostgreSQL refuses. The `recipe-db` test of D9 writes the round 2 value into PostgreSQL.
- [One valid save has a large request peak in a route test.] → ADR-0223 gives 345 978 140 bytes for the old value. That peak includes the copy of the in-memory store. Round 2 may also reach the ceiling. The reading `value-round-2` shows it.

## Migration Plan

No migration. The change adds CI steps and documents only.

## Open Questions

The owner decided D1, D2, D3, and the two rounds of D3 on 2026-11-22. After the second plan review, the owner decided one container for each round and the known limit of `leaf_texts` (2026-11-22). The owner decided D8 and the test runs of D6 on 2026-11-24. The owner decided D9 on 2026-11-24.

The owner kept D8 on 2026-11-24, and change B decides the gate of round 1 from its measured peak with the D9 body.
