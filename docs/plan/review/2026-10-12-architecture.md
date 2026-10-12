# Architecture review — 2026-10-12

Reviewer: independent architect-review agent (Opus), read-only. Sources: general-draft.txt, all of docs/architecture, all ADRs.

## CRITICAL

**F1. Session revocation and role changes do not work**
Where: ADR-0004; data-model.md §3.6 and §6 (DeactivateUser, ResetPassword, SetUserRole); overview.md §6 step 3.
Problem: the tower-sessions table keeps `user_id` inside the opaque `data` BYTEA, so you cannot delete by user; the store writes through its own connection, not the UnitOfWork transaction; `CurrentUser` (including role) comes from the session and nothing re-reads `role` or `is_active` from `app_user`.
Failure: a demoted or deactivated engineer keeps the cached role and can approve versions for up to 12 h.
Fix: own the session table (`session(id, user_id UUID NOT NULL REFERENCES app_user, csrf, expires_at…)`, index on user_id) as a recipe-db adapter; delete sessions inside the UoW transaction; load `role` and `is_active` from `app_user` on every request and use the session for identity only.

## HIGH

**F2. A value save deletes the parameter's process limits**
Where: data-model.md §6 UpsertParameter; api.md §2.6.
Problem: UpsertParameter is "DELETE all limits, INSERT the sent limits; same for translations", but the value, limits and translation forms each send only their own part. A value save with an empty limit list deletes all `parameter_limit` rows, the value passes I7 with no limit, approval does not block, and the export has no limits. Saving a `de` translation deletes `pl`.
Fix: split into `UpdateParameterDefinition`, `ReplaceLimits`, `Set/RemoveTranslation`, each touching only its own rows. Test that a value save leaves limit and translation rows unchanged.

**F3. The `Recipe` aggregate is too big to load, lock or save**
Where: domain-model.md §2.2 and §6; overview.md §6 and §8; data-model.md §6 ApproveVersion.
Problem: `Recipe` holds every version with every snapshot parameter; load/save is O(versions × parameters × leaves); a generic save hits trigger 4.1; data-model §6 already bypasses the aggregate with SQL that re-implements `Recipe::approve()`, so the rule lives in two places.
Fix: make `Recipe` a small root (header + `Vec<VersionHeader{id, number, status, row_version}>`); the live `Draft` is its own aggregate holding its parameters; approved and retired versions are read models; persist through explicit intent methods; lock the `recipe` row `FOR UPDATE` at the start of every recipe command.

**F4. A UDT change does not create, or help create, new recipe versions (R19)**
Where: draft "A change to … UDT structure creates a new recipe version"; overview.md §7 R19; ADR-0006; ADR-0007; ADR-0009.
Problem: a UDT change only inserts a new `plc_type` revision. Nothing finds affected recipes, migrates values and limits, or carries the new revision into UDTs and arrays that embed it. R19 is marked satisfied without a mechanism. A renamed member silently loses its limit on rebind.
Fix: recursive "where used" query and an "outdated type revision" marker; `RebindTypeRevision` draft command that maps values and limits by member name, needs explicit values for new members, and fails instead of dropping a limited leaf; define cascade to embedding UDTs and arrays; add `revision` to export `types[]`.

**F5. Generated STRING(n) and array types have no identity rule**
Where: domain-model.md §2.1; data-model.md §3.3 (`plc_type_name_unique`, `plc_type_name_format`) and §7; overview.md §4 rule 2.
Problem: every machine-specific `STRING(n)` row gets the same key `(machine_id, 'STRING', 1)`; array rows need a unique identifier-shaped name with no rule; UpsertParameter creates these rows, so Recipe Management writes a Plant Model table.
Fix: canonical `shape_key` (e.g. `STRING(20)`, `ARRAY[1..3,0..1] OF <uuid>`) with `UNIQUE(machine_id, shape_key)` and get-or-create via `ON CONFLICT DO NOTHING` + re-select; keep `(machine_id, name, revision)` for UDTs only; expose as a Plant Model command `ensure_derived_type`.

**F6. Drafts can reference archived units, and the docs disagree on archive rules**
Where: data-model.md §6 CreateVersion and ArchiveUnit; domain-model.md §2.1 and §5; api.md §2.2; ADR-0010.
Problem: CreateVersion copies with `INSERT … SELECT` and no validation; ApproveVersion validates only values; ArchiveUnit checks drafts without a lock; domain cascades archive to descendants and blocks only on drafts, api.md blocks on sub-units or any parameter.
Fix: validate the whole snapshot in CreateVersion and ApproveVersion (unit live, machine not archived, recipe not deleted, type on platform) and return the list of parameters that need a new unit; one ArchiveUnit rule (block while the Approved version or the live Draft references the unit; cascade to descendants); `FOR UPDATE` on the unit row in ArchiveUnit and `FOR SHARE` in UpsertParameter.

**F7. No size limit per version, so large legal recipes hit timeouts and memory limits**
Where: ADR-0007; ADR-0009; reliability.md §4.4, §7.4, §7.8; data-model.md §6; api.md §2.6.
Failure: 1 000 parameters of `ARRAY[1..200] OF REAL` with per-element limits = 200k limit rows; CreateVersion exceeds the 10 s statement timeout every time; ApproveVersion validates in Rust while its transaction is idle > 15 s and PostgreSQL kills it; the version page renders ~200k inputs; the diff view can OOM the 512 MiB container.
Fix: hard budgets in A19 (leaves and limit rows per parameter and per version), enforced in domain and database; allow one array-wide limit row (`[*]`); validate in `spawn_blocking` before the transaction and only re-check `row_version` inside it; paginate the editor, stream the export; load-test CreateVersion, Approve and page render at the budget in CI.

**F8. Any panic crashes the whole server, and a hung server is never restarted**
Where: reliability.md §2.4 (panic hook calls abort, `panic="abort"`), §7.1–7.2, §7.9.
Problem: every panic aborts the process; Docker `restart: always` acts only on exit, never on unhealthy; nothing produces the §7.9 alerts.
Fix: keep unwinding, add `CatchPanicLayer` (500 + ERROR log with request_id), supervise background tasks; watchdog (autoheal sidecar or OS-thread self-probe that exits after N seconds of failure); name the alert producer in the Compose design.

**F9. The login endpoint can take down the server, and its rate limit is unreliable**
Where: data-model.md §3.6a and §6 Login; api.md §7.4; reliability.md §7.4, §7.5, §7.8; ADR-0004.
Problem: Argon2id with no concurrency limit inside an open transaction holding one of 10 pooled connections; the failure counter is read-then-insert (racy) and a normal `Err`/rollback discards it; five bad attempts lock the real administrator out indefinitely.
Fix: semaphore (2–4 permits) acquired before any DB connection, hash outside the transaction; atomic upsert of the failure counter in its own committed transaction before the password check; per-IP limit; progressive delay instead of a hard lockout.

**F10. Up to 24 hours of approvals and audit rows can be lost, or everything**
Where: reliability.md §7.7, §7.8; ADR-0003.
Problem: the only backup is a daily logical dump on a named Docker volume, by default on the same disk as `pgdata`.
Fix: continuous WAL archiving (pgBackRest or `pg_receivewal`) plus a weekly base backup to a bind mount on a separate disk (RPO minutes); keep `pg_dump` as secondary; fail the backup healthcheck when the backup path is on the same filesystem as `pgdata`.

**F11. Replacing a machine's PLC (ADR-0010) is a destructive manual process**
Where: ADR-0010; domain-model.md §2.1; data-model.md §3.1, §3.2, §3.7.
Problem: a controller change requires a new machine, but machine codes, unit paths and reference numbers are permanently unique, and no command moves recipes.
Fix: machine codes unique among live machines only; `ReplaceController` command (new machine with the same code and units, recipes copied as drafts with a type-mapping report); reference numbers unique per `(machine_id, reference_number)` and routed as `/machines/{code}/recipes/{ref}` (or unique among live recipes only); `UnarchiveUnit`.

## MEDIUM

**F12. Deleted recipes can still be edited and approved, and ADR-0011 contradicts the schema** — lock the recipe row `FOR UPDATE` in every recipe command and check deletion and archive flags there; DeleteRecipe and ArchiveMachine discard live drafts in the same transaction; supersede ADR-0011 with one decision covering partial vs full UNIQUE, the DELETE grant, the ProcessEngineer delete condition, and whether `RestoreRecipe` exists.

**F13. Removing a parameter that has limits or translations always fails** — the child-table trigger finds no parent during cascade and raises E-RCP-031. Fix: add `recipe_version_id` to child tables with composite FK `(parameter_id, recipe_version_id) → parameter(id, recipe_version_id)` and check that column directly (also removes one lookup per row from CreateVersion).

**F14. Ports leak infrastructure, and the use-case tests miss the rules that run in SQL** — domain methods take resolved values (`&TypeTree`, `&PlatformTypes`, `&LiveUnits`); define the UoW as a closure runner re-invoked on 40001/40P01, or with the root lock use READ COMMITTED everywhere and drop retries; run the use-case suite against PostgreSQL as well.

**F15. S7-300 addresses can overlap or be misaligned** — compute and store the S7-300 layout size of every type (padding, even-byte starts for non-byte elements, structs, arrays); check overlap and alignment per data block in UpsertParameter and ApproveVersion, with a new error code and a property test.

**F16. The migration process conflicts across documents and is unsafe on a running system** — keep the one-shot `migrate` service and supersede the conflicting text; create roles in a superuser init script; seed the first administrator from the `migrate` subcommand in Rust; `lock_timeout` 2 s with retry; `CREATE INDEX CONCURRENTLY` with `-- no-transaction`; reject table-rewriting DDL in CI.

**F17. The low-disk read-only mode cannot work as designed** — `pg_stat_file` reports file metadata, not free space, and recipe_app lacks the privilege; `/readyz` includes the disk check so the proxy returns 503 for reads too; "read-only" still writes sessions, login_attempt and export audit rows. Fix: statvfs on a read-only mount of `pgdata` (in `app` or a sidecar); keep readiness independent of disk space; reserve headroom for the allowed writes.

**F18. The export contract is defined five different ways** — Retired both forbidden and allowed; translations both included and excluded; "immutable" but `recipe.name` and `version.status` change; two audit action names; `types[]` lacks revision and string length of struct members. Fix: one JSON Schema file in `recipe-export` as source of truth with golden-file tests; add `types[].revision`, string lengths on members, SHA-256 `content_hash`; drop the immutability claim or snapshot `recipe.name` into the version.

**F19. The audit table cannot store some required entries, and its growth is underestimated** — `audit_log_has_state` rejects ResetPassword "no state"; `LoginFailed` for an unknown login has no `entity_id`; `Forbidden` happens before any transaction; each value save stores two full parameter rows (up to 1 MiB each) forever. Fix: security events in their own table with a nullable subject (or nullable `entity_id`); always write a small non-null state object; failure-path events in a separate short transaction; store parameter diffs instead of full rows; add the growth estimate to A19.

## LOW

**F20. Settings and behaviours have different values in different documents** — approve idempotency (overview §8 vs ADR-0008 vs trigger 4.4: `Ok(AlreadyApproved)` cannot happen because the trigger bumps row_version on approval); retry timing/error/status; session cleanup 10 vs 5 minutes; SameSite Lax vs Strict; 128- vs 256-bit session id; last-admin trigger cited but not defined; `/healthz` through the proxy or not; which commands run SERIALIZABLE; unit depth 6 segments vs the draft example `U1.2.4.5.6`. Fix: one settings table in reliability.md linked from the others; idempotency key + `hx-sync` for approve and save; confirm the unit depth with the product owner.

## Verdict
1. Not ready to implement: the document set contradicts itself on the export, soft delete, migrations and retries, and several mechanisms cannot run as written (F1, F13, F5, F17).
2. ADR-0009 (version snapshot) and ADR-0010 (recipe per machine) can stay, but their consequences (copy cost, UDT revision migration, archived units, controller replacement) have no mechanisms yet (F4, F6, F7, F11).
3. Fix first: (1) identity — owned session table with `user_id`, re-read role and active status on every request (F1); (2) the write model — small `Recipe` root plus a `Draft` aggregate, part-specific commands, lock on the recipe row, validation on copy and approve (F2, F3, F6, F12); (3) size and type identity — hard budgets with array-wide limits, `shape_key`, UDT rebind workflow (F4, F5, F7).
4. Then one reconciliation pass so each rule lives in exactly one document, before the first migration file is written.
