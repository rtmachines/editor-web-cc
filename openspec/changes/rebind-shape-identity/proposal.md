# Proposal

## Why

The PLC data type review of 2026-11-06 found a fault in the rebind. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 8 (WP5) holds it:

- #10 (MEDIUM): on `CodesysIec`, `STRING` and `STRING(80)` are two rows with one shape. A UDT revision that only rewrites `STRING(80)` as `STRING` makes the rebind ask for a new value (`E-RCP-028`) and refuse the limit (`E-RCP-029`).

The owner chose option (1) of question Q-H on 2026-11-06. The change satisfies R09, R19, and R31. It uses Assumption A29.

## What Changes

- The new ADR-0146 amends ADR-0018. The rebind keeps a member when the old and the new member type have one type shape. Type identity stays the row (ADR-0017).
- A leaf shape is the primitive and the string length. Each seed row stores the default length of the platform, so the default length counts as an explicit length. An array shape is the dimensions and the element shape. A struct keeps its row identity.
- The domain function `kept_members` compares the shapes and no longer compares the row identifiers. The value mapping, the limit mapping, and the list of new leaves of the rebind form use it.
- `domain-model.md` §5.2 and §7 and `data-model.md` §6 name the rule.
- New tests pin the rule in `recipe-domain`, `recipe-app`, and `recipe-db`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: a rebind keeps the value and the limits of a member of one type shape.

## Impact

- Code: `recipe-domain` (`recipe/rebind/level.rs` and two doc comments in `recipe/rebind/`).
- Tests: `recipe-domain`, `recipe-app`, and `recipe-db`.
- Documents: ADR-0146, `docs/adr/README.md`, the Status line of ADR-0018, `domain-model.md` §5.2 and §7, and `data-model.md` §6.
- Stored data: no row changes. No stored value, limit, or content hash changes.
- No migration, no new error code, no new setting, and no change of the export schema.

## Out of scope

- Option (2) of Q-H: `EnsureDerivedType` keeps one row per `STRING(n)` of a machine.
- A shape rule over the members of a struct. A member of a newer struct revision stays new. The fix plan records it as follow-up F-9.
- A match of member names without case at the rebind (ADR-0143 (b), ADR-0146 (l)). The fix plan records it as follow-up F-8.
- A change of the text of `E-RCP-029`. The fix plan records it as follow-up F-10.
- The findings of the other work packages of the fix plan.
