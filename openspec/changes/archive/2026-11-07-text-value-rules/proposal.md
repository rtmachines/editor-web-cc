# Proposal

## Why

The PLC data type review of 2026-11-06 found two faults in the text rules. `docs/plan/2026-11-06-plc-type-fix-plan.md` section 5 (WP2) groups them:

- #1 (HIGH): the domain accepts `U+0000` in a `CHAR`, `WCHAR`, `STRING`, or `WSTRING` value. PostgreSQL `jsonb` refuses it, so the user gets a storage error and not `E-RCP-005`.
- #12 (MEDIUM): `domain-model.md` §2.1 and `reliability.md` §4.4 give two different units for `StringLength`. The code counts characters for `STRING` and UTF-16 code units for `WSTRING`.

The owner chose option (1) of question Q-E on 2026-11-06. The change satisfies R11, R12, R31, and R38. It uses no Assumption.

## What Changes

- The new ADR-0137 records both rules. It amends ADR-0028 and ADR-0022.
- The leaf conversion of `ParameterValue` refuses `U+0000` in the four text types with `E-RCP-005`. The check runs before the first write of the command, except the insert of a new derived type (ADR-0137 (c)).
- `domain-model.md` §2.1 gives `CHAR` the range `U+0001..=U+00FF` and `WCHAR` the range `U+0001..=U+FFFF`. The `String` row of `LeafClass` names the rule.
- `domain-model.md` §2.1 and `reliability.md` §4.4 state the unit of `StringLength`. The code does not change for this unit.
- The expected forms of a `CHAR`, `WCHAR`, `STRING`, and `WSTRING` fault change in the log text and in the bundles `pl` and `en`.
- New tests pin the rule in `recipe-domain`, `recipe-web`, and `recipe-db`. Other new tests pin the unit of `StringLength`.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `recipe-management`: no `U+0000` in a text leaf, and the unit of a string length.

## Impact

- Code: `recipe-domain` (`recipe/value/leaf.rs`, `recipe/value/expected.rs`, the property generator) and `recipe-web` (`i18n/pl.ftl`, `i18n/en.ftl`).
- Tests: `recipe-domain`, `recipe-web`, and `recipe-db`. Two XSS tests of `recipe-web` send the NUL payload to a `STRING` value and expect a success. They change.
- Documents: ADR-0137, `docs/adr/README.md`, the Status lines of ADR-0028 and ADR-0022, `domain-model.md` §2.1, and `reliability.md` §4.4.
- Stored data: no stored row holds `U+0000`, because PostgreSQL refuses it. No stored value, limit, or content hash changes.
- No migration, no new error code, and no new setting.

## Out of scope

- An escaped form of `U+0000` (option (2) of Q-E).
- A change of other control characters in a text value.
- The string length range per platform (#13, WP4).
- The error lists of `SetParameterValue` and `ReplaceLimits` in `domain-model.md` §5 and `api.md` §2.6. A later change adds the missing codes.
- A change of `reliability.md` §7.4.
- The findings of WP1 and WP3 to WP7 of the fix plan.
