# Proposal

## Why

The review of 2026-11-06 found three faults. Fix plan section 6 (WP3) groups them:

- #6 (HIGH): the class-2 locks follow the order of the key kinds, not of the key values. A stored struct `ARRAY` or a hash collision can end two commands with `40P01`.
- #5 (HIGH): `dims_valid` accepts bounds that start at another subscript than 1.
- #9 (MEDIUM): struct and member names compare in exact case, and a struct name can be `Int` or `END_STRUCT`. A PLC refuses such a type.

The owner chose option (1) of Q-D and Q-I on 2026-11-06. On 2026-11-07 the owner decided the open points of the plan review. They are the index build in `0011`, the rule of a type text, and the keyword sources. The change satisfies R11, R12, R13, R19, R29, R31, R36, and R38. It uses A38.

## What Changes

- ADR-0143 compares struct names without case on one machine and member names without case in one struct. It reserves 182 keywords and the 29 primitive names as struct names. The keywords are the union of the IEC 61131-3 Ed. 3 list and the TwinCAT 3 list. It amends ADR-0024 and ADR-0120 (a).
- A stored struct with a reserved name passes the earlier rule set and stays readable. A stored parameter type keeps it. A new revision and a type text that a command writes follow the current rule.
- ADR-0144 takes the class-2 locks in the ascending order of their key values, each value once. It amends ADR-0017.
- ADR-0145 requires the subscript 1 at the start of both bound arrays in `dims_valid`. It amends ADR-0017.
- Migration `0011` checks the stored rows, replaces `dims_valid`, and builds two unique indexes on `lower` of the name. It runs in one transaction. An `EXEMPT` entry of `migration_rewrite.py` permits the two indexes without `CONCURRENTLY`.
- An ops query lists the stored rows that the new rules affect. The merge needs the sign-off of the owner.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: the name rules of structs and members, and the reserved struct names. The order of the class-2 locks, the subscripts of array bounds, and the stored structs also change.

## Impact

- Code: `recipe-domain` (`plant/`), `recipe-app` (`plant/`, `recipe/`, `error/mapping.rs`, `memory/types/`), `recipe-db` (`transaction/type_rows.rs`, `transaction/audit_reads.rs`), and `recipe-web` (`pages/path_values.rs`, `pages/parameters/`, `pages/versions/`, `error/plant.rs`, `i18n/`).
- Schema: migration `0011` and `deploy/pgtools/restore-code.expected`. `grants.expected` does not change.
- CI: one `EXEMPT` entry in `deploy/ci/migration_rewrite.py`.
- Ops: `deploy/db/check-identifier-rules.sql` and `docs/ops/runbook.md`.
- Documents: ADR-0143 to ADR-0145, `docs/adr/README.md`, three Status lines, `data-model.md`, `domain-model.md`, `reliability.md`, and `.claude/rules/sql.md`.
- No new error code.

## Out of scope

- Keyword rules for member names (ADR-0143 (l)).
- Keywords of Siemens SCL and vendor point R4 of the fix plan.
- A rename of a stored struct or member.
- The other work packages and the follow-ups F-1 to F-7.
