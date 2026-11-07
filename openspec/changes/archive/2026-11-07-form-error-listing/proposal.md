# Proposal

## Why

The PLC data type review of 2026-11-06 found a conflict in the rules of a form answer. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 9 (WP6) holds it:

- #11 (MEDIUM): `api.md` §2.6 and ADR-0041 (a) say that a form answer lists every bad field at once. ADR-0039 (b) gives `422 E-WEB-008` for a bad key. The reader stops at the first bad key, so the body `tag=bad tag&bogus=1&value[*]=2` gives one line.

The owner chose option (1) of question Q-F on 2026-11-06. A body with such a key is a changed request and gives one line. The change satisfies R36, R38, and R39. It uses the Assumption A44.

## What Changes

- The new ADR-0138 records the rule. It amends ADR-0041 (a) and ADR-0039 (b).
- `api.md` §2.6 states which answer applies to a body with a bad key and a bad field.
- The `web-ui` requirement "Every field error at once" gets the rule and five new scenarios.
- New tests in `recipe-web` pin the rule for each form reader. The code does not change.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `web-ui`: a changed form body gives one line `E-WEB-008`.

## Impact

- Code: none.
- Tests: `recipe-web`, in the tests of the parameter routes, the rebind route, the UDT form, and the machine form.
- Documents: ADR-0138, `docs/adr/README.md`, the Status lines of ADR-0041 and ADR-0039, and `api.md` §2.6.
- No migration, no new error code, no new setting, and no change of stored data.

## Out of scope

- Option (2) of Q-F: a reader that collects every bad key with the other faults.
- The answer to a missing field of a `serde` form (`reliability.md` §4.2).
- A conflict: `reliability.md` §4.2 names the field of a missing `serde` field, but the code gives the field `form`. The fix plan records it for a later change.
- The rules of a query string (ADR-0050 (e), ADR-0051).
- The findings of WP1 to WP5 and WP7 of the fix plan.
