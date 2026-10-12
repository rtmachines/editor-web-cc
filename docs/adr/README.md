# Architecture Decision Records

This directory holds the Architecture Decision Records (ADRs) of the recipe editor. Each ADR records one decision, the options that were considered, and the consequences.

Language: English, ASD-STE100 structural rules. Requirements that are not in `general-draft.txt` are marked "Assumption".

ADR-0014 to ADR-0022 come from the design review of 2026-10-12. They replace or amend earlier decisions. A superseded ADR stays in the directory for history. Read the superseding ADR first.

## Index

| ADR | Title | Status | Date |
|---|---|---|---|
| [0001](0001-frontend-rust-ssr-htmx.md) | Frontend with Rust server-side rendering and HTMX | Accepted. Amended by ADR-0015 (row-version.js) | 2026-10-12 |
| [0002](0002-export-contract-json.md) | Export contract as a versioned JSON document | Superseded by ADR-0019 | 2026-10-12 |
| [0003](0003-deployment-docker-compose.md) | Deployment with Docker Compose on one on-premises host | Superseded by ADR-0020 | 2026-10-12 |
| [0004](0004-auth-server-sessions.md) | Authentication with server-side sessions in PostgreSQL | Superseded by ADR-0014 | 2026-10-12 |
| [0005](0005-audit-log-all-mutations.md) | Append-only audit log for every mutating operation | Accepted. Amended by ADR-0022 (hash chain, security_event table, diff state, indexes) | 2026-10-12 |
| [0006](0006-plc-type-catalog-relational.md) | Relational PLC type catalog with immutable UDT revisions | Accepted. Amended by ADR-0017 (derived types) | 2026-10-12 |
| [0007](0007-process-limits-per-leaf.md) | Process limits per scalar leaf of a parameter | Accepted. Amended by ADR-0022 (limits required at approval, `[*]` leaf path) and ADR-0018 (limits at rebind) | 2026-10-12 |
| [0008](0008-optimistic-concurrency-row-version.md) | Optimistic concurrency with a row version on the draft | Accepted. Amended by ADR-0015 (locking model, idempotent approve) | 2026-10-12 |
| [0009](0009-version-snapshot-immutable.md) | A recipe version is a full immutable snapshot | Accepted | 2026-10-12 |
| [0010](0010-recipe-scoped-to-one-machine.md) | A recipe belongs to exactly one machine | Accepted | 2026-10-12 |
| [0011](0011-soft-delete-everywhere.md) | Soft delete for recipes, versions, machines, and users | Superseded by ADR-0021 | 2026-10-12 |
| [0012](0012-translations-table-and-fluent.md) | Parameter translations in a table and UI text with Fluent | Accepted. Amended by ADR-0019 (translations are not exported) and ADR-0022 (description_en required) | 2026-10-12 |
| [0013](0013-rbac-fixed-permissions.md) | Fixed role-based permissions checked in each use case | Accepted. Amended by ADR-0022 (separation of duties, Forbidden recorded in security_event) | 2026-10-12 |
| [0014](0014-session-identity-model.md) | Session and identity model with an owned session table | Accepted | 2026-10-12 |
| [0015](0015-concurrency-locking-model.md) | Concurrency and locking model | Accepted | 2026-10-12 |
| [0016](0016-aggregate-boundaries.md) | Aggregate boundaries and command split | Accepted | 2026-10-12 |
| [0017](0017-derived-type-identity.md) | Identity of derived PLC types and S7-300 layout | Accepted | 2026-10-12 |
| [0018](0018-udt-revision-rebind.md) | UDT revisions and the rebind of recipe drafts | Accepted | 2026-10-12 |
| [0019](0019-export-integrity-and-contract.md) | Export integrity and the export format | Accepted | 2026-10-12 |
| [0020](0020-deployment-migrations-backup.md) | Deployment, migrations, backup and failure policy | Accepted | 2026-10-12 |
| [0021](0021-soft-delete-and-delete-rules.md) | Soft delete, unique keys and delete rules | Accepted | 2026-10-12 |
| [0022](0022-approval-policy.md) | Approval policy, audit tamper evidence and draft clarifications | Accepted | 2026-10-12 |

## Decision map

| Topic | Current ADR |
|---|---|
| Frontend | 0001 + 0015 (`row-version.js`) |
| Export | 0019 |
| Deployment, migrations, backup, panic policy, health | 0020 |
| Sessions, login, passwords, bootstrap | 0014 |
| Audit log | 0005 + 0022 |
| PLC types, UDT revisions, derived types, S7-300 layout | 0006 + 0017 + 0018 |
| Process limits | 0007 + 0018 (rebind) + 0022 (completeness at approval, `[*]`) |
| Concurrency | 0008 + 0015 |
| Versions and aggregates | 0009 + 0016 |
| Recipe scope | 0010 |
| Soft delete and delete rules | 0021 |
| Translations | 0012 |
| Permissions and approval | 0013 + 0022 |
| Stored settings, security event retention | 0022 |

## How to add an ADR

1. Copy the shape of an existing ADR: Status, Date, Deciders, Context, Decision drivers, Considered options, Decision, Consequences, Related.
2. Use the next free number and a short slug: `NNNN-<slug>.md`.
3. Do not change an accepted ADR. Write a new ADR that supersedes it and set the old status to "Superseded by ADR-NNNN". For a partial change, set "Accepted. Amended by ADR-NNNN (scope)".
4. Add a row to the index and update the decision map.

## Status values

- Proposed: under discussion.
- Accepted: decision made.
- Accepted. Amended by ADR-NNNN (scope): still valid, one part replaced by a later ADR.
- Superseded by ADR-NNNN: replaced in full. Kept for history.
- Rejected: considered, not adopted.
