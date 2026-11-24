# ADR-0222: Memory of the value bodies under load

Status: Accepted. Amends ADR-0128 (j), (k), (y), (z), (aa), (ac) and ADR-0211 (p)
Date: 2026-11-22
Deciders: project owner

## Context

The Load row of `reliability.md` §6 asks for parallel `POST` requests with bodies of 16 MiB on the four value routes. The container `app` must stay below `soak.memory_ceiling`. The row also asks for `503 E-WEB-009` after a wait longer than `http.body_read_timeout`.

No step of the job `load` sends a value body. Route tests check the permits of ADR-0211 (m) to (o), but no test measures the memory of four value bodies at the same time.

ADR-0128 (ac) applies the soak ceiling and the p99 bound to the soak only. Thus ADR-0128 and the Load row disagree. ADR-0211 (p) and `reliability.md` §7.8 give an estimate: one body of 16 MiB uses about 48 MiB, and four bodies use about 192 MiB.

These facts limit the test:

- `memory.peak` is the high-water mark of the cgroup since the start of the container. It includes the file cache. It is the only peak value of cgroup v2.
- A valid form with `LDT` leaves has at most about 13 206 677 bytes. Thus a valid `LDT` write form stays below the round 1 window of item (c).
- `ready` of `crates/recipe-web/src/pages/parameters/prepare.rs` builds the leaf texts (line 148) and the limit list (line 149). Then it refuses the form when a field line exists or the limit list fails (lines 150 to 152).
- A save that passes every check builds no grid (ADR-0223 (p)). Thus only a refused round 1 body renders a grid answer.
- `http.value_body_permits` is the compiled default `DEFAULT_VALUE_BODY_PERMITS` of `crates/recipe-web/src/settings.rs`. No configuration key of `recipe-server` reads it.
- After the flood, the per-IP budget blocks each login from the runner for 15 minutes (ADR-0128 (k)).

## Decision drivers

- The Load row of `reliability.md` §6 gets a check that fails CI (R30).
- The check measures the claim of ADR-0211 (p), not only the container limit.
- A round proves that the four bodies of the round overlap in the server.
- The check runs on the real Compose stack through Caddy, as the Load row asks.
- Each numeric setting comes from `reliability.md` §7.11.

## Considered options

### Option A: The soak ceiling on round 2, two rounds on two new containers, the `503` in the route test

- Pros: the check measures the absolute peak of round 2 against `soak.memory_ceiling`. Each round has a new allocator, so the rise of `anon` proves the overlap.
- Cons: each round needs its own `recreate_app` and its own baseline reading. The job takes more time.

### Option B: A bound at `container.app_memory` only

- Pros: ADR-0128 (z) already holds this bound.
- Cons: a peak between 384 MiB and 512 MiB passes. The check does not test the claim of ADR-0211 (p).

### Option C: Both rounds on one container

- Pros: one `recreate_app` and one baseline reading less.
- Cons: round 2 can use the freed pages of round 1. The rise of `anon` then does not prove the overlap.

### Option D: Four slow requests that hold the permits for more than 10 s

- Pros: the job also checks the `503 E-WEB-009` of a fifth request.
- Cons: the body deadline of a holder also ends after `http.body_read_timeout`. The answer of the fifth request then depends on the timing, so the check can fail at random.

### Option E: Round 1 keeps the gate of the ceiling

- Pros: the ceiling gates both rounds before change B.
- Cons: the heap sum of ADR-0223 (v) is above the ceiling.
- This ADR rejects Option E, because the musl leg then stays red until change B and the change cannot archive before change B.

### Option F: A smaller round 1 with a small grid

- Pros: round 1 renders a smaller grid answer.
- Cons: round 1 needs a new body, a new recipe, and new pinned values. The smallest refused shape of ADR-0223 still has a heap sum of 432 290 116 bytes.
- This ADR rejects Option F, because that sum is still above the ceiling and change B gets no measured peak of the grid answers.

## Decision

The decision selects Option A and rejects Options B, C, D, E, and F. The owner decided the ceiling, the metric, the place of the `503`, and the two rounds on 2026-11-22. On 2026-11-24 the owner decided that the ceiling applies to round 2 only. Round 1 only reports its peak (item (a)).

In this ADR, a leg is one leg of the matrix of ADR-0128 (b). The value leg is the part of each leg that sends the value bodies.

Ceiling and metric:

- (a) The value leg reads `memory.peak` of the container `app` after each round. The absolute peak of the round 2 reading must be below `soak.memory_ceiling` (384 MiB).
  - Each round runs on its own new container `app`. Before each round, the value leg runs `recreate_app` and takes a baseline reading.
  - This ADR rejects one container for both rounds (Option C), because round 2 can then use the freed pages of round 1.
  - The value leg also applies the cases of ADR-0128 (z) to each value reading.
  - The round 1 reading only reports, with one exception (owner, 2026-11-24):
    - Its ceiling, the cases of ADR-0128 (z), and its restart count only report. The reason is the heap sum of ADR-0223 (v), which is above `container.app_memory`.
    - The exception: a value of the round 1 reading that is `unavailable` or not a number still fails both legs.
    - Round 1 keeps its answer, `row_version`, and overlap checks. An OOM kill that ends a round 1 request thus fails the answer check, and the value leg ends before round 2.
    - Change B of ADR-0223 (w) turns the gate of round 1 on again.
  - As in ADR-0128 (i), these bounds gate the leg `musl` only. The leg `cc-debian13` reports them with `continue-on-error`.
  - A missing or invalid value fails both legs in the step "Require the values from each leg".
  - This item amends ADR-0128 (z) for the round 1 reading and ADR-0128 (ac) for the value leg. For the flood, ADR-0128 (z) and (ac) stay.
  - The Load row of `reliability.md` §6 states this check.
- (b) The route test `fifth_value_body_waits_and_gets_e_web_009` keeps the check of `503 E-WEB-009` and of the permit count.
  - The value leg checks the memory and the overlap only.
  - The Load row of `reliability.md` §6 names the route test for the `503` (ADR-0211).

Rounds and the proof of the overlap:

- (c) Each round sends four bodies in parallel through Caddy to `POST /recipes/{reference}/versions/1/parameters/{key}/definition`.
  - Each request goes to its own draft. Both rounds use the same four drafts.
  - Each draft is on its own machine of the platform `CodesysIec`. Thus each draft has its own class-2 `ARRAY` key, and no request waits for the lock of another request.
  - A round 1 body is the write form with an over-long `description_en`. Its encoded length lies in the window from 16 MiB − 64 KiB to 16 MiB.
  - The server reads the whole round 1 body and builds the pairs, the leaf texts, and the limit list. These are the three buffers of ADR-0211 (p).
  - Then `prepare` refuses the round 1 body with `422` and the first field line `E-WEB-008` under `description_en`. The request writes nothing.
  - A round 1 body has no unknown, repeated, or malformed key. Such a key gives one line of the field `form` and stops the reader before the texts (ADR-0138).
  - A round 2 body is the write form. Each `LDT` text has the alias prefix `LDATE_AND_TIME#` of ADR-0205.
  - A round 2 request gets `303` with the `Location` of the parameter page of its key. The `row_version` of its draft increases.
  - The command parses the `LDT` texts in `stage_definition` only in round 2.
  - The client of a round holds back the last byte of each body until a barrier. The four requests share one deadline of 8 s, below `http.body_read_timeout`.
    - At the barrier, the client reads `anon` of `memory.stat` of `app`. `anon` excludes the socket buffers.
    - The rise of `anon` over the baseline reading of the round must be at least 4 × the body length of that round.
    - Otherwise the value leg fails with `round <n>: the bodies did not overlap`.
    - A request that does not reach the barrier also fails the value leg.
  - Known limit: `app` runs the parse stages on `runtime.worker_threads` (2) Tokio workers and one CPU (`container.app_cpus`). The barrier holds four byte buffers at the same time. The parse stages can overlap only in part.
  - Known limit: the value leg cannot detect extra permits, because four requests also fit with five permits. `fifth_value_body_waits_and_gets_e_web_009` stays the test of the permit count.
  - Known limit: no test proves the place of `leaf_texts` or the order of the limit list in `prepare`.
    - The reader refuses each bad value key first, so no value field gives a line in `leaf_texts`.
    - A code reference to `prepare.rs` line 148 pins the place of `leaf_texts`.
    - A code reference to `prepare.rs` line 149 pins the order of the limit list, as line 148 pins `leaf_texts`. `ready` builds the limit list before the refusal.
    - A route test sends the round 1 body with one bad `max_length` text in one limit row. Its answer lists the line of `description_en` and the line of the limit row.
    - This route test proves only that round 1 reaches the check of the limit rows. `refuse_checked` builds the line of the limit row again, so the answer does not prove the order.

Order of the steps:

- (d) This item amends ADR-0128 (j) and (k). Each leg runs these steps in this order:
  1. The runner check of ADR-0128 (c). It also writes the start time of the job.
  2. The first administrator and its password change, as in the job `compose`.
  3. A second user with the role `ProcessEngineer`.
  4. The seed, the count check, and the three timings.
  5. The first memory reading, which is the timings reading of item (e).
  6. The step `value-seed`. It seeds five drafts on five machines and logs in once. It writes the state file with mode 0600 and the start time of the value leg. Then it takes the reading `value-seed`.
  7. A new container `app`, the checks before the flood, the flood, and the flood reading.
  8. The step `value`. It sends a pre-check to the fifth draft and runs round 1 and round 2 on the other four drafts.
  9. The memory check of the eight readings of item (e).
  10. The step `value-check`. It writes the results of the value leg and the lines of the job summary.
  - The flood is no longer the last step. The value leg needs no login after the flood, because the step `value-seed` logs in before the flood.
    - The `session` table keeps the session over `recreate_app` (ADR-0014).
    - The steps from `value-seed` to `value` take less than `session.idle_timeout` (30 min).
  - The step `value` runs these actions in this order:
    1. It sends a round 1 body and then a round 2 body to the fifth draft, one request at a time.
    2. It requires `422` with the first field line `E-WEB-008` under `description_en`. Then it requires `303` with the exact `Location` and a larger `row_version`.
    3. It takes the reading `value-precheck`, runs `recreate_app`, and takes the reading `value-baseline-1`.
    4. It runs round 1 and takes the reading `value-round-1`.
    5. It requires four answers `422` under `description_en` and an unchanged `row_version` of each draft.
    6. It runs `recreate_app` and takes the reading `value-baseline-2`.
    7. It runs round 2 and takes the reading `value-round-2`.
    8. It reads the `row_version` of each draft from the database again.
  - A failure of the pre-check stops the value leg before its first `recreate_app`. A failure of round 1 ends the value leg before round 2. The value leg still takes the round 1 reading.
  - The steps `value` and `value-check` have `continue-on-error` in the leg `cc-debian13`, like the flood (ADR-0128 (i)). The step `value-seed` has none. Each new step has `timeout-minutes`.

Readings and the peak of the whole run:

- (e) This item amends ADR-0128 (y) and (aa). The job reads the cgroup of `app` eight times:
  - the timings reading and the flood reading of ADR-0128 (y)
  - the six value readings `value-seed`, `value-precheck`, `value-baseline-1`, `value-round-1`, `value-baseline-2`, and `value-round-2`
  - Before each `recreate_app`, the value leg reads the container that it replaces.
  - Each value reading records `memory.peak`, `memory.current`, `anon`, and the container ID.
  - The checker requires a different container ID for each round and the same ID for a round and its baseline. It also requires the same four distinct draft keys in both rounds.
  - The `anon` of a baseline reading is the reference of the overlap proof of item (c). For the other readings, the job records `anon` for information only.
  - The run statistic of round n is `(peak of the round n reading − memory.current of the baseline n reading) / 4`. The summary shows it next to 3 × the body length of that round.
    - The statistic is an upper bound, not a measured value for each body. It also holds the parse stages and, in round 1, the render of the answers.
  - The peak of the whole run is the largest of the eight peaks. `load-memory.json` keeps the flood reading and the value readings separate.
    - Thus the flood peak of ADR-0128 (ab) and the line "ADR-0064 (o) memory condition" do not change.
  - The summary shows the container ID, the peak, and the run statistic of each round. It also shows the smallest `MemAvailable` of the runner in each round, the elapsed time of the value leg, and the elapsed time of the job.

The measured value:

- (f) This item amends ADR-0211 (p). After the first green dispatch run of the change, `reliability.md` §7.8 states the measured values of each round.
  - These values are the peak, the body length, and the run statistic. The text also says that the job `load` checks the ceiling on round 2 of the musl leg and reports round 1 only.
  - The text compares each run statistic with 3 × the body length of its round, not with a fixed 48 MiB.
  - The text replaces "about 4 × 48 MiB".
  - The sum of about 268 MiB of ADR-0211 (p) stays an estimate, because the value leg runs no Argon2 at the same time.

## Consequences

### Positive

- The memory bound of the value routes in the Load row of `reliability.md` §6 gets a check that fails CI.
- ADR-0128 and the Load row agree.
- A measured value replaces the estimate of ADR-0211 (p) for the value bodies.
- No change to the server code, to a setting, or to an error code is necessary.

### Negative

- The job `load` takes more time. Each new step has a timeout, and the summary records the elapsed time of the job.
- The job `load` does not run on a pull request (ADR-0128 (a)). The check needs a dispatched run of the branch.
- The peak includes the file cache and the start of the server. The ceiling is thus stricter than a bound on the heap.
- The value leg does not check the permit count. No test proves the place of `leaf_texts` or the order of the limit list (item (c)).
- Until change B, the ceiling does not gate round 1 (item (a)).

### Risks and mitigations

- Risk: the peak of round 2 reaches 384 MiB.
  - Mitigation: the job fails, and the owner decides the next step. This ADR changes no limit.
- Risk: four refused forms of round 1 need a heap sum of 894 398 804 bytes (ADR-0223 (v)). An OOM kill in round 1 can end the value leg before round 2.
  - Mitigation: the round 1 reading only reports (item (a)). The answers of round 1 then fail, and the summary still shows the round 1 reading.
  - Stop rule: the owner then decides the next step, because round 2 gives no proof.
- Risk: a later change of `prepare` refuses the form before the limit list. The round 1 body then holds fewer buffers.
  - Mitigation: the route test with the bad `max_length` text fails when the round 1 body skips the check of the limit rows. The code reference to `prepare.rs` line 149 pins the order.
- Risk: the `422` answer renders the refused form with every sent text.
  - Mitigation: the peak of round 1 includes this render, and the summary shows the peak of each round.
- Risk: the value leg does not detect extra permits.
  - Mitigation: the route test keeps the permit count. A mutation that keeps extra copies of each round 2 body proves that the ceiling detects extra memory.
- Risk: four large answers use the memory of the runner.
  - Mitigation: the client records the smallest `MemAvailable` of each round.

## Related

- ADR-0128 (a), (b), (c), (i), (j), (k), (y), (z), (aa), (ab), (ac), amended by this ADR in (j), (k), (y), (z), (aa), and (ac)
- ADR-0211 (m) to (p), amended by this ADR in (p)
- ADR-0014 (session table), ADR-0061 (h) (Tokio workers), ADR-0064 (o), ADR-0138 (changed form body), ADR-0205 (long prefixes)
- ADR-0223 (p) (no grid for a valid save), (v) (the heap sum of round 1), (w) (change B)
- `reliability.md` §6 row "Load", §7.8, §7.11
- `api.md` §7.6
- `overview.md` §7: R30, R31. `overview.md` §9: A26
