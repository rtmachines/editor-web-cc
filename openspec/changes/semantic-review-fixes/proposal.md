# Proposal

## Why

The semantic review of 2026-11-17 (`docs/plan/review/2026-11-17-semantic-review.md`) found 5 MEDIUM findings, 6 LOW findings, and 3 doc conflicts. This change carries the owner decisions of §5 of the record. For RCB-01, it carries the later decision of §7, after plan review R3.

The change satisfies R18, R27, R29, R31, R34, R35, R36, and R38. It uses A4 and A10.

## What Changes

- RCB-01: the rule "Idempotent approve" of `domain-model.md` §4 gets the order of the approval checks of the code (ADR-0191 (i)). The `web-ui` requirement "Order of the approval errors" states the same order. No code changes.
- RCB-02: the second pick of `/approved` on a route with `{code}` applies the machine filter. A reused machine code gives `404 E-WEB-005`.
- PLT-01: the second-line `DuplicateMemberName` names the repeated member.
- CDX-01: the memory adapter mirrors the version guard of ADR-0030. The negative guard cases run on both adapters.
- QRY-02: the memory store refuses a write in a read-only transaction, as PostgreSQL does with `25006`.
- DOM-03: the memory adapter emulates the parameter triggers and the platform `CHECK`.
- QRY-01, DOM-01, DOM-02: new tests pin the isolation of six queries, two frozen-column clauses, and I17 for four leaf classes.
- PLT-02: the memory double no longer claims the lock order of ADR-0144.
- RCA-01, DC-2: the operation rows of `domain-model.md` §5.3, §5.4, and `data-model.md` §6 follow the code (ADR-0191).
- DC-1: ADR-0035 (t) gets the amendment of ADR-0172 (q) and its scope (ADR-0190).
- DC-3: `reliability.md` §7.4 and `CLAUDE.md` name input parsing before the transaction and checks of the locked state under the lock (ADR-0192).
- Three new ADRs, 0190 to 0192, with the status `Proposed`.
- `CHANGELOG.md` gets one line for the changed behavior of RCB-02.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `export`: the requirement "Second pick of the approved version" gets the machine filter and one scenario.

## Impact

- `recipe-app`: `export/flow.rs`, `plant/define_udt.rs`, the memory adapter, and query tests.
- `recipe-db` and `recipe-domain`: tests only.
- `docs/adr/` (0035, 0172, 0190 to 0192, `README.md`), `docs/architecture/` (`domain-model.md`, `data-model.md`, `reliability.md`), `CLAUDE.md`, `CHANGELOG.md`.
- No migration, no route, no setting, and no new error code.

## Out of scope

- A restructure of the draft commands that moves the checks of the locked state before the transaction (ADR-0192, option B).
- The type rules of change `siemens-type-rules` and its ADRs 0177 to 0189.
- The notes of §6 of the review record.
