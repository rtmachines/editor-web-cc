# Proposal: app-use-cases-and-ports

## Why

`recipe-domain` holds the rules of the Plant Model and of Recipe Management, but no code runs them. The change adds the application layer: ports, permissions, transactions, and audit.

The change satisfies R02, R08, R09, R13, R18, R19, R20, and R23 to R37 of `overview.md` section 7.

It uses these Assumptions of `overview.md` section 9: A2, A4, A5, A7, A8, A9, A13, A20, A24, A26, A28, A29, A33, A34, A40.

## What Changes

- `recipe-domain`: add `Permission`, `Role`, and `Role::permissions` (`api.md` section 3). Add the module `audit` with `AuditAction`, `AuditEntry`, and `SecurityEvent`. Add the module `export` with `ExportError`, the hashed body, and its SHA-256 (ADR-0019).
- `recipe-domain`: `ApprovalReady` owns the checked `Draft`, so the approval check can run in `spawn_blocking`.
- `recipe-app`: add `AppError`, `StorageError`, and `AuditError` (`reliability.md` section 1.1).
- `recipe-app`: add the ports `UnitOfWork`, `RecipeRepository`, `DraftRepository`, `PlantRepository`, `TypeCatalog`, `UnitLookup`, `Clock`, and `SecurityEventSink`, with in-memory adapters.
- `recipe-app`: add `CurrentUser`, the catalog of every operation with its permission, and the permission check.
- `recipe-app`: add the 7 Plant Model commands, `EnsureDerivedType`, the 16 Recipe Management commands, and `ExportVersion` (`domain-model.md` section 5).
- Docs: add ADR-0029 for the decisions of `design.md` that change the documents.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `plant-model`: add the order of the advisory locks of derived types and the archive of an archived unit.
- `audit`: add the child rows of a parameter row and of a struct row.
- `export`: an export timeout writes no audit entry.
- `identity-access`: change the result when the forbidden event cannot be written.
- `operations`: add the storage errors of a known and of an unknown constraint name.
- `recipe-management`: change a definition update with a value but without a type change.

## Impact

- Code: `crates/recipe-domain` and `crates/recipe-app`. New dependencies: `thiserror`, `async-trait`, `tracing`, `tokio`, `serde_json`, `uuid`, `parking_lot`, and `time` in `recipe-app`. `serde`, `serde_json_canonicalizer`, and `sha2` in `recipe-domain`.
- Docs: ADR-0029 and the sections of `docs/architecture/` that it changes.

## Out of scope

- The 9 Identity and Access commands, their ports, `IdentityError`, `User`, `Session`, and the loader of `CurrentUser` (P2.5).
- The query use cases and their read ports (P2.5, P3.2, P3.3, P3.4).
- `MaintenanceStore` and the supervised jobs (P4.1).
- The JSON Schema, the HTTP writer, and the export routes (P3.5).
- The sqlx adapters (P2.4) and the response `E-WEB-006` of the read-only mode (`recipe-web`).
