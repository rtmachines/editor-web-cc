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
| [0005](0005-audit-log-all-mutations.md) | Append-only audit log for every mutating operation | Accepted. Amended by ADR-0022 (hash chain, security_event table, diff state, indexes) and ADR-0029 (child rows and source of a full row) | 2026-10-12 |
| [0006](0006-plc-type-catalog-relational.md) | Relational PLC type catalog with immutable UDT revisions | Accepted. Amended by ADR-0017 (derived types) | 2026-10-12 |
| [0007](0007-process-limits-per-leaf.md) | Process limits per scalar leaf of a parameter | Accepted. Amended by ADR-0022 (limits required at approval, `[*]` leaf path, leaf classes) and ADR-0018 (limits at rebind) | 2026-10-12 |
| [0008](0008-optimistic-concurrency-row-version.md) | Optimistic concurrency with a row version on the draft | Accepted. Amended by ADR-0015 (locking model, idempotent approve) and ADR-0028 (row version check of version commands in the domain) | 2026-10-12 |
| [0009](0009-version-snapshot-immutable.md) | A recipe version is a full immutable snapshot | Accepted | 2026-10-12 |
| [0010](0010-recipe-scoped-to-one-machine.md) | A recipe belongs to exactly one machine | Accepted | 2026-10-12 |
| [0011](0011-soft-delete-everywhere.md) | Soft delete for recipes, versions, machines, and users | Superseded by ADR-0021 | 2026-10-12 |
| [0012](0012-translations-table-and-fluent.md) | Parameter translations in a table and UI text with Fluent | Accepted. Amended by ADR-0019 (translations are not exported) and ADR-0022 (description_en required) | 2026-10-12 |
| [0013](0013-rbac-fixed-permissions.md) | Fixed role-based permissions checked in each use case | Accepted. Amended by ADR-0022 (separation of duties, Forbidden recorded in security_event) and ADR-0029 (failed record of a Forbidden event) | 2026-10-12 |
| [0014](0014-session-identity-model.md) | Session and identity model with an owned session table | Accepted. Amended by ADR-0035 (throttle of ChangeOwnPassword, credential check at the commit, key format, pre-session cookie, create-admin) | 2026-10-12 |
| [0015](0015-concurrency-locking-model.md) | Concurrency and locking model | Accepted | 2026-10-12 |
| [0016](0016-aggregate-boundaries.md) | Aggregate boundaries and command split | Accepted. Amended by ADR-0028 (version header, approval token, version command row check) | 2026-10-12 |
| [0017](0017-derived-type-identity.md) | Identity of derived PLC types and S7-300 layout | Accepted | 2026-10-12 |
| [0018](0018-udt-revision-rebind.md) | UDT revisions and the rebind of recipe drafts | Accepted | 2026-10-12 |
| [0019](0019-export-integrity-and-contract.md) | Export integrity and the export format | Accepted | 2026-10-12 |
| [0020](0020-deployment-migrations-backup.md) | Deployment, migrations, backup and failure policy | Accepted. Amended by ADR-0033 (options of the globals dump) | 2026-10-12 |
| [0021](0021-soft-delete-and-delete-rules.md) | Soft delete, unique keys and delete rules | Accepted | 2026-10-12 |
| [0022](0022-approval-policy.md) | Approval policy, audit tamper evidence and draft clarifications | Accepted. Amended by ADR-0029 (failed record of a Forbidden event) and ADR-0035 (column grant and guard trigger of security_event) | 2026-10-12 |
| [0023](0023-ste-lint-scope-and-location.md) | Scope and location of the STE lint in CI | Accepted | 2026-10-12 |
| [0024](0024-error-codes-member-names-s7-width.md) | Error codes for duplicate member names and the S7 address width | Accepted | 2026-10-13 |
| [0025](0025-parse-reasons-of-value-objects.md) | Parse reasons of value objects without an own error code | Accepted | 2026-10-13 |
| [0026](0026-ranges-of-date-and-date-and-time.md) | Platform ranges of DATE and DATE_AND_TIME | Accepted | 2026-10-13 |
| [0027](0027-error-lists-and-draft-errors.md) | Error lists and the error of a draft command | Accepted | 2026-10-13 |
| [0028](0028-version-header-and-approval-token.md) | Version header, approval token, and draft clarifications | Accepted. Amended by ADR-0029 (the approval token owns the draft) | 2026-10-13 |
| [0029](0029-application-layer-clarifications.md) | Approval token ownership, audit row images, and application clarifications | Accepted. Amended by ADR-0035 (dependencies of recipe-app, row image of LoginUnlocked) | 2026-10-13 |
| [0030](0030-guard-of-a-version-status-write.md) | Guard of a version status write | Accepted | 2026-10-13 |
| [0031](0031-migrator-configuration-and-secret-files.md) | Configuration of the migrator and the secret files of the roles | Accepted | 2026-10-13 |
| [0032](0032-default-databases-closed-to-public.md) | Default databases closed to PUBLIC | Accepted | 2026-10-13 |
| [0033](0033-globals-dump-of-the-backup.md) | Globals dump of the backup | Accepted | 2026-10-14 |
| [0034](0034-hardening-of-the-audit-objects.md) | Hardening of the audit objects | Accepted. Amended by ADR-0035 (security_event_guard in the guard list) | 2026-10-14 |
| [0035](0035-identity-sessions-clarifications.md) | Identity, sessions, and the guard of security events | Accepted | 2026-10-14 |
| [0036](0036-design-system-and-ui-foundation.md) | Design system and UI foundation | Accepted | 2026-10-14 |

## Decision map

| Topic | Current ADR |
|---|---|
| Frontend | 0001 + 0015 (`row-version.js`) + 0036 (design system, fonts, icons, layout) |
| Design system, accessibility, UI layout | 0036 |
| Export | 0019 |
| Deployment, migrations, backup, panic policy, health | 0020 + 0031 (migrator configuration, secret files) + 0032 (default databases) + 0033 (globals dump) |
| Sessions, login, passwords, bootstrap | 0014 + 0035 (throttle of ChangeOwnPassword, credential check at the commit, key format, pre-session cookie, Origin, client address, create-admin) |
| Audit log | 0005 + 0022 + 0029 (child rows and source of a full row) + 0034 (audit hardening) + 0035 (security_event guard, LoginUnlocked image) |
| PLC types, UDT revisions, derived types, S7-300 layout | 0006 + 0017 + 0018 + 0024 (member names, S7 width) |
| Process limits | 0007 + 0018 (rebind) + 0022 (completeness at approval, `[*]`, leaf classes, time values) + 0026 (ranges of `DATE`, `DATE_AND_TIME`) |
| Errors of value objects without an own code | 0025 |
| Error lists, errors of draft commands | 0027 |
| Concurrency | 0008 + 0015 + 0028 (row version check of version commands) + 0030 (guard of a version status write) + 0035 (lock failure cause) |
| Versions and aggregates | 0009 + 0016 + 0028 (version header, approval token, draft load, value and text clarifications) + 0029 (token owns the draft, two command clarifications) |
| Recipe scope | 0010 |
| Soft delete and delete rules | 0021 |
| Translations | 0012 |
| Permissions and approval | 0013 + 0022 + 0028 (approval token, `Approver`) + 0029 (failed record of a Forbidden event) |
| Dependencies of the application layer | 0029 + 0035 (Argon2, random bytes, semaphore) |
| Stored settings, security event retention | 0022 |
| STE lint scope and script location in CI | 0023 |

## How to add an ADR

1. Copy the shape of an existing ADR: Status, Date, Deciders, Context, Decision drivers, Considered options, Decision, Consequences, Related.
2. Use the next free number and a short slug: `NNNN-<slug>.md`.
3. An accepted ADR changes only by a Status line and an "Amended by" note. To replace it, write a new ADR that supersedes it and set the old status to "Superseded by ADR-NNNN". For a partial change, set "Accepted. Amended by ADR-NNNN (scope)".
4. Add a row to the index and update the decision map.

## Status values

- Proposed: under discussion.
- Accepted: decision made.
- Accepted. Amended by ADR-NNNN (scope): still valid, one part replaced by a later ADR.
- Superseded by ADR-NNNN: replaced in full. Kept for history.
- Rejected: considered, not adopted.
