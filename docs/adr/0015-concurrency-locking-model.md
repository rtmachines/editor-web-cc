# ADR-0015: Concurrency and locking model

Status: Accepted. Amended by ADR-0096 (the isolation and the retry of `SetUserRole` and `DeactivateUser`)
Date: 2026-10-12
Deciders: project owner

## Context

`data-model.md` section 6 used SERIALIZABLE for six commands and a retry policy for SQLSTATE 40001. The PostgreSQL review executed the schema on PostgreSQL 16.15 and showed three defects:

- Defect 1: a parameter `INSERT` that runs while `ApproveVersion` holds the version row waits on the foreign-key lock and then commits into the approved version. SERIALIZABLE does not help, because the snapshot is taken before the lock wait.
- Defect 2: `DeleteRecipe` and `ApproveVersion` both commit under SERIALIZABLE. There is no read-write cycle, because `ApproveVersion` never reads `recipe`.
- The retry policy had three different definitions (traceability B41).

The architecture review (F3, F14) added that the `Recipe` aggregate re-implemented the approval rule in SQL, and that the retry wrapper had no defined signature.

## Decision drivers

- Data consistency over performance (R31).
- The lock order plus `ORDER BY` on multi-row locks prevents every deadlock cycle found in review.
- A rule that an implementer can apply without reading the planner documentation.
- `row_version` stays the token that the UI sends (ADR-0008).

## Considered options

### Option A: SERIALIZABLE for every command with retries

- Pros: no explicit locks in code.
- Cons: does not cover the two cases above. SIREAD locks escalate to whole tables when 1 000 parameter rows are read. Every command needs a retry wrapper.

### Option B: READ COMMITTED with explicit row locks in a fixed order

- Pros: the lock on the `recipe` row serializes every command on one recipe, no retries, the trigger can take a share lock and holds.
- Cons: the lock order is a convention that code review must enforce.

### Option C: Advisory lock per recipe

- Pros: no dependency on row existence.
- Cons: the lock is not released by a failed row lookup, and it is invisible to the foreign-key machinery.

## Decision

The project uses **Option B**.

- Every command runs in READ COMMITTED. `SetUserRole` and `DeactivateUser` run in SERIALIZABLE because they count administrators.
- Lock order: `machine` → `recipe FOR UPDATE` → `recipe_version FOR UPDATE` → `unit` → child rows. A command takes only the locks that it needs, always in this order.
- `RenameMachine`, `ArchiveMachine`, `RenameUnit`, and `ArchiveUnit` take `machine FOR NO KEY UPDATE` first. `AddUnit`, `CreateRecipe`, `RestoreRecipe`, `CreateVersion`, `AddParameter`, `UpdateParameterDefinition`, `RebindTypeRevision`, and `DefineUdt` take `machine FOR SHARE` first. `DefineUdt` then takes the advisory lock.
- Every recipe command then runs `SELECT deleted_at, machine_id FROM recipe WHERE id = $r FOR UPDATE` and requires `deleted_at IS NULL`. `CreateRecipe` has no recipe row yet. `RestoreRecipe` takes the same lock and requires `deleted_at IS NOT NULL`. Every draft command then locks the version row and compares `row_version`.
- A statement that locks several rows of one table orders them by `id`. This applies to the units of `CreateVersion` and `ArchiveUnit`, the recipes of `ArchiveMachine`, and the versions of `DeleteRecipe`. Without the order, two such statements can lock the same rows in two orders. `app_user` rows are locked `FOR NO KEY UPDATE`. A `FOR UPDATE` lock on `app_user` blocks the `FOR KEY SHARE` lock of the foreign-key check of an audit insert. That insert already holds the row lock of `audit_chain_head`, which the first command needs next. These two rules close the first two deadlock cycles found in review (`data-model.md` 6).
- The machine lock closes two more cycles:
  - Cycle 3. `EnsureDerivedType` inserts a `plc_type` row inside `AddParameter`, which holds the recipe, the version, and the unit. The foreign key `plc_type.machine_id` then takes `FOR KEY SHARE` on the machine row. Without the machine lock, `ArchiveUnit` holds that machine row and waits for the unit.
  - Cycle 4. `ArchiveMachine` holds the machine row and waits for the recipe rows. Without the machine lock, a recipe command holds its recipe and then waits for its machine row.
  - Now every command that reads or writes rows of one machine locks the machine row first. The key share of the foreign key falls on a row that the command already holds.
- Concurrency tests run `ArchiveUnit` against `AddParameter` with a new derived type, and `ArchiveMachine` against `CreateVersion`. No run gives `40P01` (`reliability.md` 6).
- `AddParameter` and `UpdateParameterDefinition` take `FOR SHARE` on the unit row after the version row. `CreateVersion` takes `FOR SHARE` on the unit rows of the base version after the recipe lock and checks `archived_at` again. `ArchiveUnit` takes `machine FOR NO KEY UPDATE` first, then `FOR UPDATE` on the unit rows. Every command that adds a reference to a unit takes `machine FOR SHARE` first. Two archives on one machine therefore run one after the other. An archive and a new reference to the unit cannot interleave (I19).
- `DefineUdt` and `EnsureDerivedType` take `pg_advisory_xact_lock(2, hashtext(machine_id::text || ':' || name))` instead of a row lock on `plc_type` (`domain-model.md` 5.1). Class 2 is the advisory lock class of derived types (`data-model.md` 1.4). `recipe_app` has no `UPDATE` grant on `plc_type`, so it cannot take a row lock there. The unique indexes stay as the second line. A command takes all its class-2 keys in one order before its first `plc_type` insert (`data-model.md` 1.4). Two commands therefore never take two keys in opposite orders.
- The trigger function `enforce_version_mutable` takes `FOR SHARE` on the version row. A writer that bypasses the application waits for the approval and then fails with `E-RCP-031`.
- `SetUserRole` and `DeactivateUser` retry once on 40001, then return `E-DB-001`. Every other command has no retry. A deadlock (40P01) maps to `E-DB-001`, returns 503 and is logged at ERROR. It is a bug in the lock order.
- Validation of a whole snapshot (I7, limits, units, types) runs in `spawn_blocking` before the transaction opens. Inside the transaction the use case re-checks `row_version`. After the locks it also repeats the unit check of `CreateVersion`, and the limit count and the outdated check of `ApproveVersion` (`data-model.md` 6).
- Idempotent approve: when the version is already Approved and `row_version == expected + 1`, `ApproveVersion` returns `Ok(ApprovalOutcome::AlreadyApproved)` and writes no audit entry. Any other mismatch returns `ConcurrentEdit`.
- Commit paths. `recipe-app` has four write ports. `domain-model.md` section 5 gives the path of each operation:
  - `UnitOfWork` exposes `begin()` and `UnitOfWork::commit(entries: NonEmpty<AuditEntry>)`. Every command, `Logout`, `Login` T2 on success, `ExportVersion`, and the startup settings comparison use it. There is no closure runner.
  - `SecurityEventSink::record(SecurityEvent)` writes one row in its own short transaction: failed and throttled logins, `Forbidden`, CSRF rejections, the `unlock` event, and audit chain breaks. An aggregated kind increments `count` of the row of its minute (`data-model.md` 3.8).
  - `LoginAttemptStore` counts the failures and inserts the attempt in `Login` T1 under the advisory lock. T1 inserts no row after the per-IP budget is reached. Read-only mode does not change T1. The store marks the attempt as a success in T2.
  - `MaintenanceStore` serves the job `cleanup`. It deletes old sessions and `login_attempt` rows and calls `security_event_purge()` once per batch. It also records the two progress keys of the job `audit_chain_check` and the key `heartbeat.last_at` of the job `heartbeat` in `app_setting`.
- The response to a successful draft command carries `HX-Trigger: {"rowVersion": <new>}`. The static script `row-version.js` copies the value into every hidden `row_version` field of the page (`api.md` section 4). It holds no application state. This amends ADR-0001.

## Consequences

### Positive

- One rule covers delete, approve, archive and every parameter edit.
- The two race conditions and the four deadlock cycles from the review are closed and have tests.
- No retry logic in recipe use cases.

### Negative

- Two users who edit the same recipe wait on each other for the length of one transaction. With validation outside the transaction this is milliseconds.
- The convention needs a test: every command that touches a recipe must lock it first. A storage test inspects `pg_locks` for each command.

### Risks and mitigations

- Risk: a new command forgets the first lock. Mitigation: `recipe-db` exposes one `lock_recipe(tx, id)` helper and the review checklist requires it.
- Risk: SERIALIZABLE commands on `app_user` fail with 40001 under load. Mitigation: they are rare and administrative. They retry once, then return `E-DB-001`, and the HTTP layer shows "try again".

## Related

- Amends ADR-0008 and ADR-0001 (`row-version.js`). ADR-0016 (aggregate boundaries) depends on this ADR.
- `data-model.md` sections 4 and 6, `reliability.md` section 1.1.
- Review findings: PostgreSQL, defects 1 and 2 above. Architecture F3, F12, F14, F20. Traceability B41, B59, B78.
