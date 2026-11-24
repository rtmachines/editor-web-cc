# Proposal

## Why

The Load row of `reliability.md` §6 asks for parallel `POST` requests of 16 MiB on the value routes below `soak.memory_ceiling`. No CI code checks it (HIGH finding of `review-design-fixes-2`).

ADR-0128 (ac) applies the ceiling to the soak only. A valid `LDT` form has at most about 13 206 677 bytes. The owner decided on 2026-11-22.

It satisfies R30 and R31 and uses A26.

## What Changes

- ADR-0222 records the decisions:
  - On the musl leg, the `memory.peak` of round 2 stays below `soak.memory_ceiling` (384 MiB). Round 1 only reports its peak (owner, 2026-11-24, `design.md` D8). This amends ADR-0128 (ac).
  - The route tests keep the checks of `503 E-WEB-009` and the permit count.
  - The leg sends two rounds of four parallel requests through Caddy, one draft for each machine, each round on a new `app`.
  - Round 1 sends bodies of nearly 16 MiB that get `422` for `description_en` and write nothing. Round 2 sends the write form and gets `303`.
  - The step order, the memory readings, and the run peak amend ADR-0128 (j), (k), (y), (z), and (aa).
  - After the first green run, `reliability.md` §7.8 states the measured peaks. This amends ADR-0211 (p).
- The load job seeds five drafts and logs in before the flood. After the flood, it runs a pre-check.
  - The client holds back the last byte. At the barrier of each round, `anon` of `app` must rise by 4 × the body length.
  - The leg fails on a wrong answer, no overlap, a missing value, or a round 2 OOM kill, restart, or peak of 384 MiB.
- Route tests and the Python generator pin the length and SHA-256 of both bodies.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: two new requirements, one modified requirement.

## Impact

- `deploy/ci/`: five scripts with tests. `.github/workflows/ci.yml`: new steps in job `load`.
- `crates/recipe-web/src/`: new route tests.
- `docs/adr/`: ADR-0222, two Status lines, the index. `reliability.md` §6 and §7.8.
- No change to the server code or an error code.

## Out of scope

- A change of a limit or a setting. The owner decides on a peak at the ceiling.
- A streaming form parser and the soak test of §6.
- The 268 MiB sum of ADR-0211 (p), because the leg runs no Argon2.
- A permit check by the leg.
