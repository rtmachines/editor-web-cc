# Proposal

## Why

The reader of the limit rows keeps each bound text as sent, with no length limit. One request with `ParameterEdit` can send a `[*]` row with a text of almost 16 MiB on a type with 16 384 elements. The grid copies that text into each element row. By estimate for an ASCII-only text, the server then needs up to 274 877 906 944 bytes before any HTML. A lossy decode can triple a text that is not UTF-8.

The job `load` also needs data and a frame:

- Run 45805398657 had a round 2 overlap margin of 588 012 bytes on musl.
- ADR-0222 (a) says that change B turns the gate of round 1 on. The worst measured round 1 margin is 19 980 288 bytes on musl.
- The saved memory frame of change B does not describe the measured `memory.peak` readings.

The change satisfies R30, R31, and R38 and uses A26.

## What Changes

- ADR-0225 records the three parts below and the gate of round 1.
- C1: the reader refuses a limit bound text above `form.limit_text_max` decoded bytes as a changed body. The answer is one line `422 E-WEB-008`. §7.11 holds the cap.
- C1: a `dhat` pin measures the largest answer at the cap.
- F3: in round 2, the value leg records the largest `anon` between the first crossing and the barrier release. The overlap check stays.
- D1: `reliability.md` §7.8 adds the memory frame in measured `memory.peak` readings. It keeps the values of ADR-0222 (f).
- Round 1 stays report only.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: one new requirement on the cap of a limit bound text.
- `operations`: one new requirement on the largest `anon` of round 2.

## Impact

- `recipe-web`: the form reader, the reason texts, the cap, and tests.
- `deploy/ci/` and `.github/workflows/ci.yml`: the value leg, its tests, and the values check.
- `docs/`: ADR-0225, amended items of ADR-0138, ADR-0211, ADR-0222, and ADR-0223, `reliability.md` §7.8 and §7.11, and `api.md` §2.6.
- `CHANGELOG.md`: the kinds of bound text that the cap now refuses.

## Out of scope

- A smaller row model (`GridView`), the next change.
- D4, D5, A54, a bound on parallel `GET` pages, and Options E and F of ADR-0223.
- A new threshold of the overlap check, and a gate on round 1.
