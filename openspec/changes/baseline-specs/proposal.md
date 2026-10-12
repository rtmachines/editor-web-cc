# Proposal: baseline-specs

## Why

Before phase 2 starts the implementation, this change freezes the requirements of the architecture documents as OpenSpec specs. Every later change then cites a requirement and adds a delta.

The specs satisfy these requirements of `overview.md` section 7:

- R01, R02, R03, R04, R05, R06, R07, R08, R09, R10.
- R11, R12, R13, R14, R15, R16, R17, R18, R19, R20.
- R21, R22, R23, R24, R25, R26, R27, R28, R29, R30.
- R31, R34, R35, R36, R37, R38, R39, R40.

The specs use these Assumptions of `overview.md` section 9:

- A1, A2, A3, A4, A5, A6, A7, A8, A9, A10.
- A11, A12, A13, A14, A15, A16, A17, A18, A20, A21.
- A22, A23, A24, A25, A26, A27, A28, A29, A30, A31.
- A32, A33, A34, A35, A36, A38, A39, A40, A41, A42, A43.

## What Changes

- Add one delta spec for each capability.
- Each requirement copies a rule of `docs/architecture/` or `docs/adr/` and cites its R-number or A-number.
- Each scenario comes from an invariant I1 to I23 of `domain-model.md` section 3 or a row of `data-model.md` section 6.
- Scenarios of `web-ui` can also come from `api.md` sections 4, 5, and 8.
- Scenarios of `operations` can also come from `reliability.md` sections 1 to 7.

## Capabilities

### New Capabilities

- `plant-model`: machines, units, PLC platforms, type catalog, UDT revisions, derived types.
- `recipe-management`: recipes, drafts, versions, parameters, limits, translations, approval, rebind.
- `identity-access`: users, roles, sessions, login throttle, passwords, permission matrix.
- `audit`: audit log, hash chain, security events.
- `export`: export document v1, content hash, export routes.
- `web-ui`: concurrency protocol, error responses, internationalization, English UI text.
- `operations`: Compose, migrations, backup, health, panic containment, logs, settings, supervised jobs.

### Modified Capabilities

None. `openspec/specs/` is empty.

## Impact

- Only `openspec/changes/baseline-specs/` changes. No crate, migration, or deployment file changes.
- P1.2 reviews the change and moves the specs to `openspec/specs/`.

## Out of scope

- R32, R33, R41, R42, R43, and A37. They are rules for code and process, not product behavior. Clippy, CI, and code review check them.
- R44 and R45. They are process requirements of the build.
- A19. It gives the expected scale and no behavior.
- New design decisions, ADRs, and changes of `docs/architecture/` or `docs/adr/`.
- Code, migrations, and tests. Phase 2 adds them.
- The move of the specs to `openspec/specs/` (P1.2).
