# Proposal

## Why

The paused change `value-body-memory-load` measured one answer. By its estimate, four parallel value requests pass `soak.memory_ceiling` (384 MiB). The edit page of the largest valid parameter has 176 653 007 bytes. A refused form holds up to three copies of its page.

This is change A of the split of 2026-11-22. It makes each grid answer smaller and measures it. It satisfies R30, R31, and R39 and uses A26.

Change A does not bound the sum of parallel grid answers. By the estimates of `design.md`, four value bodies and four refused forms need about 423.0 MB in `en` and 422.6 MB in `pl`. Both sums are above the ceiling, so change A alone does not close the finding. Change B decides that bound.

## What Changes

- ADR-0223 records D2, D3, D6, and D8 of `design.md` and the decisions that change B keeps.
- The grid shows one mode row for each type level. This amends ADR-0039 (k), ADR-0045 (b), and ADR-0138 (c).
- The largest valid page shrinks from about 176.7 MB to about 22.6 MB in `en`, by estimate.
- Each grid answer holds at most two of its parts at the same time. A counting pass gives only the grid answers buffers of exact length.
- The HTMX successes of `/definition` and `/value` copy no form and no fragment.
- A valid save builds no grid of the sent fields.
- Route tests with `dhat` pin the length and the peak heap of each grid answer of the largest valid parameter.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: two requirements on the mode rows.
- `operations`: one requirement on the peak memory of a grid answer.

## Impact

- `recipe-web`: the grid walk, the mode switch, the `limit_mode` reader, the grid answers, the error answer, test support, and route tests.
- `e2e/`: the leaf grid tests and the seed.
- `docs/`: ADR-0223, the Status lines of ADR-0039, ADR-0045, and ADR-0138, the ADR index, and `design-system.md` §10.4.
- `.github/workflows/ci.yml`: a job that runs the ignored measurement tests.
- The paused change then updates its pins and its answer reader.

## Out of scope

- Change B: `form.grid_memory`, the write check with `E-WEB-018`, and A54.
- Streaming of the answer body.
- The rebind forms of the version page.
- The value body permits and `http.body_limit.value`.
