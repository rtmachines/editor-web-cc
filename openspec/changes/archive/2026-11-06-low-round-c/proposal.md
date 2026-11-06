# Proposal

## Why

The review of 2026-11-04 left 28 LOW findings outside the fix plan. Group C holds the 12 findings in Rust code and tests. It also holds the document gaps about that code.

- D1-4: `UnlockLogin` returns an error after a successful commit when the event `unlock` fails. `api.md` §5 says that an error means an incomplete command.
- D2-2: a stalled mount can hold `statvfs` without a limit. Then the read-only mode does not turn on.
- D4-3: four multi-row deletes do not lock their rows in `id` order.
- D5-4: `E-AUD-003` has no text in the bundles.
- D5-5: the own password form shows one line per answer after step P4.2.
- D1-2, D1-3, D2-3, D3-1, D5-3, D5-6, D6-8: code structure, test strength, and document gaps.

The change satisfies R10, R20, R31, R36, and R38. It uses A22, A24, A33, and A44.

## What Changes

- `UnlockLogin` completes when the event `unlock` fails after the commit. The web layer writes one `WARN` line (ADR-0130).
- `POST /lang` with a session accepts either token. The documents record the rule (ADR-0131).
- The own password form lists every bad field. The login form keeps one answer (ADR-0132).
- The disk measurement stops after `health.disk_check_interval` (ADR-0133).
- The multi-row deletes lock their rows in `id` order.
- `POST /logout` reads its form.
- `E-AUD-003` gets its sentence in both bundles.
- One macro table gives the facts of each operation. `run_admin_change` gets a function for the target checks.
- The reset test asserts the code of the failed Basic check.
- `reliability.md` §1.1 and `design-system.md` §1 match the code.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `identity-access`: unlock with a lost event, token of the language switch.
- `web-ui`: answer of the own password form.
- `operations`: time limit of the disk measurement.

## Impact

- Code: `recipe-app`, `recipe-db`, `recipe-web`, and `recipe-server`.
- Documents: four ADRs, `domain-model.md`, `data-model.md`, `api.md`, `reliability.md`, `design-system.md`, `CHANGELOG.md`, and the project rules.
- No migration, error code, setting, or workspace dependency. `recipe-server` uses `parking_lot` of the workspace.

## Out of scope

- The findings of group B (CI and deploy).
- A multi-field answer of the login form.
- A retry of the event `unlock`.
- A new setting for the time limit of `statvfs`.
- A failed refresh after the commit of a user command (`design.md`, Not in scope).
