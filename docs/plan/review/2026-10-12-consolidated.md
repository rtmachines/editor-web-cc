# Design review — consolidated report

Date: 2026-10-12
Subject: `docs/architecture/*.md`, `docs/adr/*.md`, `docs/plan/implementation-prompts.md` (commit `fe10397`)
Method: four independent reviewers without access to the authoring session (architecture, security, PostgreSQL with live execution on 16.15, traceability and consistency), one PLC domain check by the author, and the STE linter. The four raw reports are in this folder. This document merges them, removes duplicates, states the author's position, and lists the decisions the owner must make.

## 1. Verdict

**Not ready for OpenSpec baseline specs or for `0001_schema.sql`.** The scope is complete and most of it traces to the draft. But the document set contains mechanisms that cannot run as written, several contradictions that would become conflicting SHALL statements, and three integrity gaps on the approval-to-PLC path. All fixes are cheaper now than after the first migration.

Totals after merge: 14 blocking items, 16 important items, 12 owner decisions, about 90 mechanical consistency fixes (listed in the traceability report).

Reviewer agreement is high. Six items were found independently by three reviewers (session revocation, state machine in the database, cascade trigger, migrations and roles, login path, export contract).

## 2. Blocking items (fix before P1.1)

| # | Item | Found by | Author position | Fix |
|---|---|---|---|---|
| K1 | Session revocation and role changes do not work. tower-sessions keeps `user_id` inside opaque `data`, the store bypasses the UnitOfWork, and `CurrentUser` caches the role for up to 12 h | ARCH F1, SEC M4, PG H7 | Agree. Design error | Own `session` table with `user_id` FK and index, written by a `recipe-db` adapter. Reload `role`, `is_active`, `session_epoch` from `app_user` on every request. Delete sessions inside the same transaction. Call `cycle_id()` on login |
| K2 | The database does not enforce the version state machine. `recipe_app` can set Approved back to Draft, edit, re-approve. Table-level UPDATE grants let it move a recipe to another machine or rename a unit path | SEC H1, PG H1, PG H5, TRC B17 | Agree. Verified live | Transition trigger `recipe_version_transition` (tested SQL in the PostgreSQL report). Column-level `GRANT UPDATE (...)`. Freeze triggers on `machine.code`, `unit.path`, `recipe.reference_number` while a non-draft version references the row |
| K3 | Concurrency model is wrong. SERIALIZABLE does not protect Delete vs Approve, and a parameter write can slip into a version being approved | PG H3, PG H4, ARCH F12, ARCH F14 | Agree. Verified live. Simplifies the design | READ COMMITTED everywhere. First statement of every recipe command: `SELECT ... FROM recipe WHERE id = $r FOR UPDATE` and check `deleted_at`. Then lock the version `FOR UPDATE`. Trigger 4.1 takes `FOR SHARE` on the version. SERIALIZABLE only for `SetUserRole` and `DeactivateUser`. Drop the 40001 retry policy |
| K4 | `RemoveParameter` always fails when the parameter has a limit or a translation (cascade runs after the parent is gone) | PG C1, ARCH F13, TRC C5 | Agree. Verified live | New `enforce_version_mutable` (tested SQL). Add `recipe_version_id` to `parameter_limit` and `parameter_translation` with a composite FK so the child check needs no parent lookup |
| K5 | `STRING(n)` and array types have no identity rule. A second `STRING(n)` on a machine violates UNIQUE. Array rows need an invented name | PG C2, ARCH F5, TRC C8 | Agree. Verified live | Three partial unique indexes (tested SQL) or a `shape_key`. Get-or-create with `ON CONFLICT DO NOTHING` then re-select. Expose as Plant Model command `EnsureDerivedType` so Recipe Management does not write `plc_type` directly |
| K6 | A value save deletes the limits and translations of the parameter, because `UpsertParameter` replaces all child rows and each form sends only its part | ARCH F2, TRC B53, TRC B54 | Agree. Data-loss bug | Split into `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`. Identify the parameter by `key` in the form. Test that a value save leaves child rows unchanged |
| K7 | The `Recipe` aggregate (all versions, all parameters) cannot be loaded or saved per request, and the approval rule already lives in SQL as well | ARCH F3 | Agree | `Recipe` root = header + version headers. Live `Draft` = own aggregate with its parameters. Approved and Retired = read models. Domain methods take resolved values (`&TypeTree`, `&PlatformTypes`, `&LiveUnits`), never a port (TRC B66) |
| K8 | Login path: unbounded Argon2 work (OOM), racy failure counter, counter lost on rollback, 5-failure lockout is a denial of service against any known login | SEC H3, SEC H4, ARCH F9, TRC B51, PG L3 | Agree | Global semaphore around Argon2 (permits = memory budget / m_cost), acquired before any DB connection. Hash outside the transaction. Counter as atomic upsert in its own committed transaction under `pg_advisory_xact_lock`. Progressive delay per (login, IP) instead of a hard lockout. Per-IP budget. `UnlockLogin` for Administrator. Same response and timing for unknown, inactive and wrong-password |
| K9 | Migrations and roles cannot run as written: `0001` creates roles as `recipe_migrator`, the SQL seed cannot read env, ADR-0003 and data-model §8 contradict reliability §5 on who migrates | SEC H5, ARCH F16, PG H6, PG M7, TRC B71–B73 | Agree | Roles and database in a superuser `docker-entrypoint-initdb.d` script. One-shot `migrate` service with the migrator secret. `recipe-server create-admin` seeds the first administrator in Rust. `ALTER DEFAULT PRIVILEGES` for future tables. Supersede ADR-0003 |
| K10 | The export contract is defined five different ways (Retired allowed or not, translations in or out, immutable or not, two audit names, missing `types[].revision`) | ARCH F18, TRC B63–B65, SEC M7 | Agree | One JSON Schema file in `recipe-export` is the source. Add `types[].revision`, member string lengths, `content_sha256`. Supersede ADR-0002. See decision D1 and D8 |
| K11 | Drafts can reference archived units or deleted recipes. `CreateVersion` copies without validation. `ApproveVersion` validates values only | ARCH F6, TRC C14 | Agree | Validate the whole snapshot in `CreateVersion` and `ApproveVersion` (unit live, machine live, recipe live, type on platform). One `ArchiveUnit` rule: blocked while the Approved version or the live Draft references the unit, cascade to descendants, `FOR UPDATE` on the unit row |
| K12 | Error codes, HTTP statuses and variant ownership are inconsistent across three documents. About 16 error names have no code | TRC B36–B47, A23 | Agree | One table in reliability §1.2 with every variant, owner crate, code, HTTP status. api.md links to it. Trigger codes listed individually |
| K13 | Unit depth: the author miscounted. `U1.2.4.5.6` has five segments, which matches "up to five levels". The documents say six | TRC A7, ARCH F20 | Author error | Maximum five segments (main unit + four sub-levels). Fix domain-model, data-model CHECK, reliability. See decision D5 |
| K14 | R19 (a UDT change creates a new recipe version) has no mechanism. A new revision is inserted and nothing finds or updates the recipes that use the old one | ARCH F4, TRC A15 | Agree. The R-table overstated coverage | "Where used" query, `outdated_type` marker on affected parameters, `RebindTypeRevision` draft command that maps values and limits by member name and fails instead of dropping a limited leaf. See decision D7 |

## 3. Important items (fix in the same reconciliation change)

| # | Item | Found by | Fix |
|---|---|---|---|
| I1 | No size budget per version. 1 000 parameters × `ARRAY[1..200]` with per-element limits = 200k rows. Copy, approve and render exceed the timeouts and the 512 MiB limit | ARCH F7 | Budgets in A19 for leaves and limit rows per parameter and per version, enforced in domain and database. Array-wide limit row `[*]`. Validate in `spawn_blocking` before the transaction. Paginate the editor. Stream the export |
| I2 | Panic policy: `panic = "abort"` makes one bad stored value a crash loop for all users. Docker does not restart an unhealthy container | ARCH F8, TRC A20 | Decision D10. Recommended: unwind + `CatchPanicLayer` (500 + log) + watchdog that exits after N failed self-probes. Remove the `expect` exception in `main` |
| I3 | Low-disk read-only mode cannot work: `pg_stat_file` does not report free space and `recipe_app` cannot call it. `/readyz` with the disk check makes the proxy return 503 for reads | ARCH F17, PG M9, SEC M6, TRC C12, B69, B70 | `statvfs` on a read-only mount of `pgdata` inside `app`. Readiness independent of disk. Read-only mode = commands blocked, inserts into `session`, `login_attempt`, `audit_log` allowed with reserved headroom |
| I4 | Health endpoints: format, checks and routing differ between api.md and reliability.md | SEC M9, TRC B69 | Separate internal listener for `/healthz` and `/readyz`. Caddy probes that port and answers 404 on the public site. Cache readiness 5 s |
| I5 | `audit_log` cannot store required rows (`PasswordReset` without state, `LoginFailed` for an unknown login, `Forbidden` before any transaction). Growth estimate ignores full-row JSON | ARCH F19, TRC A16, C6, SEC M8 | Nullable `entity_id` or a `security_event` table. Always write a small non-null state. Failure-path events in a separate short transaction. Store parameter diffs, not full rows. Add `client_ip`, `user_agent`, `session_ref`, `aggregate_id` |
| I6 | Recipe delete rule differs in five documents. ADR-0011 contradicts the DDL on grants and partial unique keys. `RestoreRecipe` is cited but does not exist | TRC B18–B20, B31, ARCH F12, SEC L3 | Supersede ADR-0011. One rule: ProcessEngineer deletes only a recipe with no Approved and no Retired version. Administrator deletes only when no Approved version exists. Plain UNIQUE stays. Add `RestoreRecipe` for Administrator or remove the sentence |
| I7 | Cookie and session spec: SameSite Lax vs Strict, 128 vs 256 bits, `secure=false` option, no HSTS, no trusted-proxy rule, no fixation handling, no login CSRF, open redirect on `/lang` | SEC M1, M2, TRC B48, A24 | `__Host-recipe_session`, Secure, HttpOnly, SameSite=Strict, 128-bit id. Refuse to start with `secure=false` outside a dev feature. HSTS. Trusted-proxy CIDR. `cycle_id()` on login. Pre-session CSRF token for `/login` and `/lang`. Same-origin redirect only |
| I8 | No CSP and no htmx hardening. A stored XSS from a ProcessEngineer description reaches an Administrator session | SEC M3 | CSP `default-src 'self'; script-src 'self'; ...`. htmx `allowEval=false`, `allowScriptTags=false`, `selfRequestsOnly=true`, `hx-history="false"`. Static script files only. Ban `\|safe` in review. XSS corpus in HTTP tests |
| I9 | Bootstrap admin password stays in env for the life of the install. Admin-chosen reset passwords let an admin act as an engineer | SEC M5 | `create-admin` from stdin or secret file. `must_change_password` on bootstrap and on reset. One-time password shown once |
| I10 | PostgreSQL for 10 years: checksums off, libc collation, floating image tag, no major-upgrade runbook, PUBLIC has TEMP, functions without `search_path` | PG M8, M5, L5 | `--data-checksums --lc-collate=C`. Pin `postgres:16.x-bookworm`. `REVOKE TEMPORARY`. `SET search_path = public, pg_temp` on every function. `pg_dumpall --globals-only` in the backup. Upgrade runbook |
| I11 | S7-300 addresses: no alignment check (`DBW3` is invalid), no overlap check inside a data block, no layout size for structs and arrays, offset not bounded to 65535 | Author, ARCH F15 | Compute S7-300 layout size per type (padding, even starts for W/D). Check alignment and overlap per DB in `UpsertParameter` and `ApproveVersion`. New error codes. Property test |
| I12 | Type matrix: `LTOD`, `LDATE` missing. `TOD`/`DT` names used in reliability. `string_length` limit 254 blocks WSTRING on S7-1500 and STRING(n) > 254 on IEC. BOOL size undefined | TRC A10, A11, PG M1, C7 | Add rows. One name per type. Per-platform string length limits. BOOL `size_bytes = 1` with `X` width |
| I13 | Traceability table overstates coverage in 12 rows. Draft lines 102–104 (patterns, subagents, Polish chat) have no row. 15 of 22 principles have no statement | TRC A1–A5, A27 | Correct the 12 rows. Add process rows that point to the plan. One line per principle: applied for problem X, or not applied |
| I14 | Backup RPO is 24 h on the same disk as `pgdata`. Backup contains sessions and hashes in clear | ARCH F10, SEC H7 | Decision D4. At minimum: exclude `session` and `login_attempt` from the dump, dedicated `recipe_backup` role, manifest with SHA-256, fail the healthcheck when backup path and `pgdata` share a filesystem |
| I15 | Controller replacement (S7-300 → S7-1500) is a destructive manual process because codes and references are globally unique forever | ARCH F11 | Decision D11 |
| I16 | Two documents claim naming authority with different names (`actor` vs `actor_user_id`, `key` vs `parameter_key`) | TRC B23 | Rust names from domain-model, SQL names from data-model, one mapping table in data-model §1 |

## 4. Decisions for the owner

Each decision changes scope or cost. The author gives a recommendation. Answer "yes" to accept it.

| # | Question | Options | Recommendation |
|---|---|---|---|
| D1 | Export integrity (SEC C1) | (a) `content_sha256` in the document and in `recipe_version`, checked at export. (b) Also an Ed25519 signature with a key in a Docker secret and a verification duty for the transfer system | **(a) now, in schema v1.** (b) only when the transfer system exists and can verify. Record (b) as the planned path in the ADR |
| D2 | Separation of duties on approval (SEC H2). Stage 1 chose "no four-eyes" | (a) Keep: any Administrator or ProcessEngineer approves. (b) Config flag `approval.separation_of_duties`, default on: the approver cannot be the creator or an editor of the draft. (c) Password re-entry on approve as an e-signature | **(b) + the approve page shows a diff with limit changes highlighted.** (c) only if the plant requires it. (a) stays possible per plant through the audited flag |
| D3 | Audit log tamper evidence (SEC H6) | (a) Hash chain trigger in the database. (b) Also ship entries to an off-host append-only sink | **(a).** (b) is a site decision, documented as an option in the runbook |
| D4 | Backup RPO (ARCH F10) | (a) Daily `pg_dump`, RPO 24 h, documented as Assumption. (b) WAL archiving to a second disk, RPO minutes | **(b) if the host has a second disk or a network share.** Otherwise (a) with the risk written in the runbook |
| D5 | Unit depth (K13) | (a) Five segments, as the draft example. (b) Six segments | **(a)** |
| D6 | Mandatory description and limits (TRC A8). The draft says "must have" | (a) Description required non-empty. At approval, every numeric or string leaf needs a limit. BOOL has none. (b) Keep optional and record as Assumption | **(a).** It is what the draft says and it is cheap to enforce at approval |
| D7 | UDT change and recipe versions (K14, R19) | (a) Mark affected parameters as outdated. The engineer creates a draft and runs `RebindTypeRevision`. (b) `DefineUdt` creates a draft for every affected recipe automatically | **(a).** (b) creates hundreds of unattended drafts and conflicts with "one live draft per recipe" |
| D8 | Export of Retired versions (K10) | (a) Only Approved is exportable. (b) Retired exportable by Administrator and ProcessEngineer with `"loadable": false` and a `RETIRED_` file prefix | **(b)** for rollback and audit use. Operators never export Retired |
| D9 | S7 address on S7-1200 and S7-1500 (TRC A12) | (a) Forbidden (current DDL). (b) Optional, format-checked | **(b).** Non-optimized data blocks use absolute addresses, and the draft does not forbid it |
| D10 | Panic policy (I2) | (a) `panic = "abort"` + Docker restart. (b) Unwind + `CatchPanicLayer` + watchdog | **(b).** The draft says "must not crash or stop responding" |
| D11 | Controller replacement (I15) | (a) Out of scope now. Machine code unique among live machines only, so the code can be reused after archive. Record as Assumption. (b) `ReplaceController` command in scope | **(a).** (b) as a later change |
| D12 | Login throttling UX (K8) | (a) Hard lockout 5/15 min (current). (b) Progressive delay per (login, IP), per-IP budget, Administrator unlock | **(b)** |

**Decision record:** the owner accepted every recommendation on 2026-10-12 (D1a, D2b, D3a, D4b, D5a, D6a, D7a, D8b, D9b, D10b, D11a, D12b).

## 5. Items the author does not accept as written

| Item | Reason |
|---|---|
| TRC A2, A3 as requirement rows | "Use subagents" and "chat in Polish" are instructions to the agent, not product requirements. They get a note in overview §7, not an R row with a mechanism |
| SEC M7 API tokens now | No transfer system exists. The token design goes into the superseding ADR-0004 as the defined path, implementation waits for the first consumer |
| PG M4 `JSON` instead of `JSONB` | Keep `JSONB`. The domain canonicalises numbers before storage and documents it. Add the size CHECK and lz4 compression |
| ARCH F3 "approved versions as read models only" | Accepted, with one addition: the read model must still carry `row_version` so that `RetireVersion` keeps its optimistic check |

## 6. Fix plan: change `design-reconciliation` (docs only, before P1.1)

1. Owner answers D1–D12.
2. New ADRs: 0014 session and identity model (supersedes 0004 in part), 0015 concurrency and locking model, 0016 aggregate boundaries (Recipe root + Draft), 0017 derived type identity, 0018 UDT revision rebind, 0019 export integrity (supersedes 0002), 0020 deployment and migrations (supersedes 0003), 0021 soft delete and delete rules (supersedes 0011), 0022 translations in export (amends 0012). Mark the old ones Superseded.
3. `data-model.md`: replace §3–§6 with the tested SQL from the PostgreSQL report (K2–K5, K9, I5, I10, I12). Owned `session` table. `login_attempt` with lock. Composite FK on child tables. Column grants. Default privileges. Five-segment path CHECK. Transactions table complete for all commands, READ COMMITTED with explicit locks.
4. `domain-model.md`: Draft aggregate, split commands, `AuditAction` list, I15, one grammar per value object, five segments, N-notes converted to Assumptions, mapping table Rust name ↔ SQL name.
5. `api.md`: one status table linked to reliability, `/lang` row, login CSRF, command names from domain-model, health on the internal listener, export per D8.
6. `reliability.md`: complete error table (every variant, owner, code, status, including trigger codes and E-WEB-006), one settings table (timeouts, limits, retention), panic policy per D10, Argon2 parameters and semaphore, disk check by `statvfs`, backup per D4, security headers and CSP.
7. `overview.md`: correct the 12 R rows, add R43–R45 process notes, A25–A35, remove N references, C4 container view with `migrate`.
8. `implementation-prompts.md`: update P2.3–P2.5, P3.4, flowchart; add this change as P1.0.
9. Mechanical pass: the ~90 items in the traceability report Pass B and C, then STE lint to zero hard violations.
10. Re-run the traceability review with a fresh agent. Gate for P1.1: zero contradictions in Pass B.

Estimated size: one session with three parallel subagents (data-model, ADRs, cross-document consistency) plus one review agent.

## 6a. Reconciliation status (2026-10-12, closed)

Steps 1–10 of section 6 were applied in seven rounds. Each round was a fix pass followed by an independent gate review (no access to earlier findings). The gate reports are in this folder.

| Gate | Pass B contradictions | Pass C blocking | Main items found |
|---|---|---|---|
| 1 | 66 | 5 | Names and numbers diverged between parallel rewrites; audit chain false alarms; export hash over mutable fields; login counter lost on rollback |
| 2 | 6 | 2 | Login grant missing; FOR SHARE on a table without UPDATE grant; races on ArchiveUnit and CreateVersion; hash order not reproducible; UI language had no storage; crate dependency cycle |
| 3 | 8 | 5 | Array types unreachable from the API (TypeExpr added); migration 0003 fails after the ownership transfer; DbError cycle; WAL archive without a replication role |
| 4 | 11 | 1 | Chain check without batching; FK key-share deadlock on app_user; outdated check only before the transaction |
| 5 | 7 | 2 | Time types and CHAR had no canonical form or limit semantics (leaf classes, IecLiteral, A43); FK key-share deadlock on machine; login flood unbounded |
| 6 | 3 | 1 | htmx 2 does not swap 4xx/5xx without responseHandling; advisory lock order between callers; JoinError mapping |
| close | — | — | Round 7 applied the gate-6 items. The gate-6 reviewer stated the residual non-blocking risk is acceptable for writing specifications |

Final state: 102 error codes in one registry, every code used; 33 command and 33 query use cases with identical names in four documents; 35 audit actions; 43 Assumptions with identical copies; 790 cross-references resolve; every number equals `reliability.md` 7.11; migrations simulated as `recipe_migrator` on PostgreSQL 16; every data-model 6 statement covered by the grants; `ste-lint.py` 0 hard violations in 28 files (51 800 words).

Known residual items (not blocking, to close during specification): cosmetic wording in the gate-6 list that the final round applied; the query list may need more entries when screens are designed (P3.1); the runbook file is a P4.1 deliverable.

## 7. Binding contract for the reconciliation

Every rewritten document MUST use these names and values. A document that needs a different value changes this section first.

### 7.1 Aggregates and commands

- `Recipe` (root): id, machine, reference_number, name, deleted_at, `versions: Vec<VersionHeader { id, number, status, row_version }>`.
- `Draft` (aggregate): one `RecipeVersion` with `status = Draft` and its `Vec<Parameter>`. Loaded and saved by the parameter commands.
- `ApprovedVersion`, `RetiredVersion`: read models. They carry `row_version` so `RetireVersion` keeps its optimistic check.
- Domain methods take resolved values only: `&TypeTree`, `&PlatformTypes`, `&LiveUnits`, `&ApprovalPolicy`. No port in `recipe-domain`.

Commands (final list, one use case each, one transaction each):

| Context | Commands |
|---|---|
| Plant Model | `CreateMachine`, `RenameMachine`, `ArchiveMachine`, `AddUnit`, `RenameUnit`, `ArchiveUnit`, `DefineUdt`, `EnsureDerivedType` (internal, get-or-create of `STRING(n)`, `WSTRING(n)`, array types) |
| Recipe Management | `CreateRecipe`, `RenameRecipe`, `DeleteRecipe`, `RestoreRecipe` (Administrator), `CreateVersion`, `DiscardDraft`, `ApproveVersion`, `RetireVersion`, `AddParameter`, `UpdateParameterDefinition`, `SetParameterValue`, `ReplaceLimits`, `UpsertTranslation`, `RemoveTranslation`, `RemoveParameter`, `RebindTypeRevision` |
| Identity and Access | `Login`, `Logout`, `CreateUser`, `SetUserRole`, `DeactivateUser`, `ActivateUser`, `ResetPassword`, `ChangeOwnPassword`, `UnlockLogin` |
| Export | `ExportVersion` (audited) |

Queries are use cases too. Each query use case checks its permission first (`RecipeView`, `AuditView`, ...).

### 7.2 Concurrency and locking

- Isolation: READ COMMITTED for every command. SERIALIZABLE only for `SetUserRole` and `DeactivateUser` (last-administrator count).
- Lock order: `recipe FOR UPDATE` → `recipe_version FOR UPDATE` → children. Every recipe command starts with `SELECT deleted_at FROM recipe WHERE id = $1 FOR UPDATE` and requires `deleted_at IS NULL`. Every draft command then locks the version row `FOR UPDATE` and compares `row_version`.
- Trigger `enforce_version_mutable` takes `FOR SHARE` on the version row (tested SQL in the PostgreSQL report, H2).
- `SetUserRole` and `DeactivateUser` retry once on 40001, then return `E-DB-001`. Every other command has no retry. 40P01 (deadlock) maps to `E-DB-001`, HTTP 503. A deadlock is a bug in the lock order and is logged at ERROR.
- Validation of a whole snapshot runs in `spawn_blocking` before the transaction. Inside the transaction only `row_version` is re-checked.
- Idempotent approve: when the version is already Approved and `row_version == expected + 1`, `ApproveVersion` returns `Ok(ApprovalOutcome::AlreadyApproved)` and writes no audit entry.

### 7.3 Schema changes (data-model.md)

| Table | Change |
|---|---|
| `unit` | Path CHECK: 1 to 5 segments. `parent_path` generated column with composite FK (PostgreSQL L2) |
| `plc_type` | Drop `plc_type_name_unique`. Three partial unique indexes (PostgreSQL C2). `dims_valid` NULL-safe. `plc_type_string_length_required`. `string_length` limit per platform is checked in the domain, the CHECK is `BETWEEN 1 AND 16382`. BOOL `size_bytes = 1`, width `X`. Column-level INSERT grant without `created_at` |
| `recipe_version` | `content_sha256 TEXT` (set at approval, frozen). `recipe_version_transition` trigger (PostgreSQL H1). `UNIQUE (recipe_id, id)` and composite FK for `based_on_version_id`. `fillfactor = 70` |
| `draft_editor` | New: `(recipe_version_id, user_id, first_edit_at)`. Written by every draft command. Used by the separation-of-duties rule |
| `parameter` | Index `(recipe_version_id, unit_id, ordinal)`. `CHECK (pg_column_size(value) <= 1048576)`. lz4 compression. S7 address CHECK allows a value on every platform, requires one only on `S7_300` |
| `parameter_limit`, `parameter_translation` | Add `recipe_version_id` with composite FK `(parameter_id, recipe_version_id) → parameter (id, recipe_version_id)`. `leaf_path = '[*]'` means every element of an array |
| `session` | Owned table: `id TEXT PRIMARY KEY` (SHA-256 of the cookie token), `user_id UUID NOT NULL REFERENCES app_user`, `csrf_token TEXT NOT NULL`, `created_at`, `last_seen_at`, `expires_at`. Index on `user_id`. Never written by tower-sessions |
| `app_user` | `session_epoch BIGINT NOT NULL DEFAULT 0`, `must_change_password BOOLEAN NOT NULL DEFAULT false`, `created_at`, `updated_at` |
| `login_attempt` | `login TEXT NOT NULL CHECK (length(login) <= 64)`, `client_ip INET NOT NULL`, `attempted_at`, `succeeded`. Counted under `pg_advisory_xact_lock(hashtextextended(login, 0))` in its own committed transaction |
| `audit_log` | `entity_id` stays NOT NULL. Add `aggregate_id UUID NOT NULL`, `client_ip INET`, `session_ref TEXT`, `prev_hash BYTEA`, `entry_hash BYTEA NOT NULL`. Hash chain trigger owned by `audit_owner` (NOLOGIN). Parameter entries store a diff, not full rows |
| `security_event` | New: `id`, `occurred_at`, `kind` (`login_failed`, `login_rejected_inactive`, `login_throttled`, `csrf_rejected`, `forbidden`, `unlock`), `login_hmac BYTEA`, `user_id UUID`, `client_ip INET NOT NULL`, `user_agent TEXT`, `request_id UUID NOT NULL`, `detail JSONB`. Written in its own short transaction, also on the failure path |
| grants | Column-level `GRANT UPDATE` (PostgreSQL H5). `ALTER DEFAULT PRIVILEGES`. `REVOKE TEMPORARY ON DATABASE`. `SET search_path = public, pg_temp` on every function. `audit_owner` owns `audit_log` and `security_event`. `recipe_backup` role with `pg_read_all_data` |
| roles and database | Created by `deploy/db/initdb/01-roles.sh` as superuser from Docker secrets. Migrations never create roles |

Size budgets (Assumption A26): at most 16 384 leaves per parameter, 262 144 leaves per version, 32 768 limit rows per version, 4 000 parameters per version. Enforced in the domain and by a statement-level trigger on `parameter`.

### 7.4 Migrations and deployment

- Folder `migrations/` at the repository root. Files `NNNN_description.sql`: `0001_schema`, `0002_triggers`, `0003_grants`, `0004_seed_plc_types`. A file that needs `CREATE INDEX CONCURRENTLY` starts with `-- no-transaction`.
- Compose services: `proxy` (Caddy, mandatory), `migrate` (one-shot, `restart: no`, holds the migrator secret), `app`, `db` (`postgres:16.x-bookworm` pinned, `--data-checksums --lc-collate=C --lc-ctype=C.UTF-8`), `backup` (daily `pg_dump` with `--exclude-table-data=session --exclude-table-data=login_attempt`, plus `pg_dumpall --globals-only`, `backup-now` one-shot command), `walarchive` (`pg_receivewal` to a bind mount on a second disk, D4b).
- First administrator: `recipe-server create-admin` reads the password from stdin or a secret file, sets `must_change_password = true`, does nothing when an active administrator exists.
- Secrets: Docker secrets (`*_FILE`) only. `app` uses `expose`, never `ports`. Port 5432 is not published.

### 7.5 Settings (single source: reliability.md section 7.11)

| Setting | Value |
|---|---|
| Session idle / absolute | 30 min / 12 h. Cleanup every 10 min |
| Cookie | `__Host-recipe_session`, Secure, HttpOnly, SameSite=Strict, Path=/, 128-bit token. `cycle_id()` on login |
| Login delay | After 3 failures per (login, client_ip) in 15 min: delay `2^(n-3)` s, cap 60 s. Per-IP budget 100 failures per 15 min, then 429. No permanent lockout. `UnlockLogin` clears the counters |
| Argon2id | m = 19 MiB, t = 2, p = 1. Semaphore 4 permits, acquire timeout 2 s, then 503 |
| Password | 12 to 128 characters, checked against a local breached-password list |
| Timeouts | Request 30 s, `statement_timeout` 10 s, pool acquire 5 s, idle-in-transaction 15 s |
| Pool | 10 connections for the app, 2 for health |
| Text limits | name 200, description 4 000, change_note 4 000, unit name 200, tag 64 |
| Body limits | Form 256 KiB, parameter value field 1 MiB |
| Health | Internal listener on port 9000, `/healthz` and `/readyz`, body `ok` or `not ready`, readiness cached 5 s, no disk check in readiness |
| Disk | `statvfs` on a read-only mount of `pgdata`. Below 5 % free: commands return 503 `E-WEB-006`, inserts into `session`, `login_attempt`, `security_event`, `audit_log` continue. Above 10 %: normal |
| Backup | `pg_dump` daily 02:00 UTC, retention 30 days. WAL archive continuous. Restore test weekly and before each release |
| Soak | 24 h, memory ceiling 384 MiB with a 512 MiB container limit |
| Panic | Unwind. `CatchPanicLayer` returns 500 with `request_id`. Watchdog thread exits the process after 3 failed self-probes in 30 s. `overflow-checks = true` |

### 7.6 Error codes and HTTP statuses

Ranges: `E-PLT-001..029` domain, `E-PLT-030..049` triggers. `E-RCP-001..029` and `E-RCP-050..069` domain, `E-RCP-030..049` triggers. `E-IDN-001..019`. `E-AUD-001..009`. `E-EXP-001..009`. `E-DB-001..009`. `E-WEB-001..019`. The registry is `reliability.md` section 1.2. A new code is added there first.

Status rules: value or shape error 422. State conflict (wrong status, duplicate, last administrator, has approved version) 409. Missing resource in the URL 404. Operation on a deleted recipe 409 `E-RCP-012`. Wrong current password 422. No approved version on `/export/.../approved` 404 `E-EXP-001` (`ExportError::NoApprovedVersion`). Concurrent edit 409 `E-WEB-001`. Forbidden 403 `E-WEB-002`. Throttled 429 `E-IDN-005`. CSRF 403 `E-WEB-007`. Read-only mode 503 `E-WEB-006`. Every variant has exactly one owner enum and one row in reliability section 1.2.

### 7.7 Export v1

- One JSON Schema file: `crates/recipe-export/schema/export-v1.schema.json`. Golden-file tests.
- Fields: `schema_version`, `exported_at`, `exported_by`, `loadable` (true only for Approved), `content_sha256` (SHA-256 over the RFC 8785 canonical form of the document without `exported_at`, `exported_by`, `loadable`, `content_sha256`), `machine`, `recipe`, `version` (with `status`), `types[]` (with `revision`, member `string_length`), `parameters[]`. Translations are not exported.
- `GET /export/v1/recipes/{ref}/approved`: every role with `VersionExport`. `GET /export/v1/recipes/{ref}/versions/{n}`: Approved or Retired, Administrator and ProcessEngineer only, file name prefix `RETIRED_` for Retired.
- `ApproveVersion` computes and stores `content_sha256`. `ExportVersion` recomputes and refuses with `E-EXP-002` on mismatch.

### 7.8 Approval policy

- `approval.separation_of_duties = true` (default). The approver cannot be `created_by` of the draft or any user in `draft_editor`. Error `E-RCP-015 ApproverEditedDraft`, 409. The flag is a config value and a change of it is written to `security_event`.
- At approval every numeric and string leaf MUST have a limit (D6). Error `E-RCP-016 LimitMissing`, 422. BOOL leaves have no limit.
- `description_en` MUST be non-empty.
- The approve page shows the diff against the current Approved version. Limit changes are highlighted.

### 7.9 Naming authority

Rust names come from domain-model.md. SQL names come from data-model.md. data-model.md section 1 holds the mapping table (`actor` ↔ `actor_user_id`, `before`/`after` ↔ `before_state`/`after_state`, `key` ↔ `parameter_key`, `supersedes` ↔ `supersedes_type_id`, and every other difference). N-notes become Assumptions A25–A35 in overview.md section 9. No document cites an N-number.
