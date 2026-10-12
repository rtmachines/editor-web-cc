# Architecture Decision Records

This directory holds the Architecture Decision Records (ADRs) of the recipe editor. Each ADR records one decision from the analysis stage, the options that were considered, and the consequences.

Language: English, ASD-STE100 structural rules. Requirements that are not in `general-draft.txt` are marked "Assumption".

## Index

| ADR | Title | Status | Date |
|---|---|---|---|
| [0001](0001-frontend-rust-ssr-htmx.md) | Frontend with Rust server-side rendering and HTMX | Accepted | 2026-10-12 |
| [0002](0002-export-contract-json.md) | Export contract as a versioned JSON document | Accepted | 2026-10-12 |
| [0003](0003-deployment-docker-compose.md) | Deployment with Docker Compose on one on-premises host | Accepted | 2026-10-12 |
| [0004](0004-auth-server-sessions.md) | Authentication with server-side sessions in PostgreSQL | Accepted | 2026-10-12 |
| [0005](0005-audit-log-all-mutations.md) | Append-only audit log for every mutating operation | Accepted | 2026-10-12 |
| [0006](0006-plc-type-catalog-relational.md) | Relational PLC type catalog with immutable UDT revisions | Accepted | 2026-10-12 |
| [0007](0007-process-limits-per-leaf.md) | Process limits per scalar leaf of a parameter | Accepted | 2026-10-12 |
| [0008](0008-optimistic-concurrency-row-version.md) | Optimistic concurrency with a row version on the draft | Accepted | 2026-10-12 |
| [0009](0009-version-snapshot-immutable.md) | A recipe version is a full immutable snapshot | Accepted | 2026-10-12 |
| [0010](0010-recipe-scoped-to-one-machine.md) | A recipe belongs to exactly one machine | Accepted | 2026-10-12 |
| [0011](0011-soft-delete-everywhere.md) | Soft delete for recipes, versions, machines, and users | Accepted | 2026-10-12 |
| [0012](0012-translations-table-and-fluent.md) | Parameter translations in a table and UI text with Fluent | Accepted | 2026-10-12 |
| [0013](0013-rbac-fixed-permissions.md) | Fixed role-based permissions checked in each use case | Accepted | 2026-10-12 |

## How to add an ADR

1. Copy the shape of an existing ADR: Status, Date, Deciders, Context, Decision drivers, Considered options, Decision, Consequences, Related.
2. Use the next free number and a short slug: `NNNN-<slug>.md`.
3. Do not change an accepted ADR. Write a new ADR that supersedes it and set the old status to "Superseded by NNNN".
4. Add a row to the index.

## Status values

- Proposed: under discussion.
- Accepted: decision made.
- Superseded: replaced by a later ADR.
- Rejected: considered and not adopted.
