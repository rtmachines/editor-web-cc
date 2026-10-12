# PostgreSQL data model review — 2026-10-12

Reviewer: independent agent (Opus) with the `postgresql-table-design` skill, read-only on the repository. Method: the SQL from data-model.md §3–§5 was executed on PostgreSQL 16.15 (`postgres:16-bookworm`, throwaway container). Invariant-breaking tests, two-connection race tests and a timed copy of 1 000 parameters were run. Every fix below was applied and re-tested.

Checked, no finding: sections 3–4 execute in order as superuser (13 tables); every composite FK has a matching UNIQUE target; the regexes for machine code, unit path, S7 address and leaf_path match the stated grammars; identity INSERT into `audit_log` by `recipe_app` works without a sequence grant; the machine-platform freeze is race-safe because the composite FK enforces it; two concurrent `CreateVersion` calls → the second gets 40001; per-row triggers on a copy of 1 000 parameters + 3 000 limits + 2 000 translations take about 130 ms (27 ms with triggers off).

## CRITICAL

**C1 — §4.1 (lines 584–588, note on 609): `RemoveParameter` always fails when the parameter has a limit or a translation.**
`ON DELETE CASCADE` runs after the parent row is gone; the lookup finds nothing, `v_version` is NULL, the trigger raises `E-RCP-031 version <NULL> is not a live draft` (verified). Fix: the H2 function (`IF NOT FOUND AND TG_OP = 'DELETE' THEN RETURN OLD; END IF;`).

**C2 — §3.3 line 276: a machine can have only one STRING(n) and needs an invented name for every array type.**
`UNIQUE NULLS NOT DISTINCT (machine_id, name, revision)`: STRING(16) then STRING(20) on M12 gives 23505 (verified). Fix (tested):
```sql
ALTER TABLE plc_type DROP CONSTRAINT plc_type_name_unique;
CREATE UNIQUE INDEX plc_type_named_unique  ON plc_type (machine_id, name, revision) NULLS NOT DISTINCT
  WHERE kind <> 'array' AND string_length IS NULL;
CREATE UNIQUE INDEX plc_type_string_unique ON plc_type (machine_id, name, string_length) NULLS NOT DISTINCT
  WHERE string_length IS NOT NULL;
CREATE UNIQUE INDEX plc_type_array_unique  ON plc_type (machine_id, element_type_id, dim_lower, dim_upper)
  WHERE kind = 'array';   -- on-demand create: INSERT ... ON CONFLICT DO NOTHING, then SELECT
```

## HIGH

**H1 — §3.8 and §4.4: nothing stops illegal changes to `recipe_version` rows (breaks I4 and the state machine).**
Verified: `UPDATE recipe_version SET status='draft', approved_at=NULL, approved_by=NULL` on an approved version passes every CHECK; retired → approved possible; a discarded draft can be undeleted; `number`, `recipe_id`, `based_on_version_id` can be rewritten. Fix (tested):
```sql
CREATE FUNCTION recipe_version_transition() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
  IF OLD.deleted_at IS NOT NULL
     OR (NEW.id,NEW.recipe_id,NEW.plc_platform,NEW.number,NEW.based_on_version_id,NEW.created_by,NEW.created_at)
        IS DISTINCT FROM (OLD.id,OLD.recipe_id,OLD.plc_platform,OLD.number,OLD.based_on_version_id,OLD.created_by,OLD.created_at)
     OR NOT ((OLD.status='draft' AND NEW.status IN ('draft','approved')) OR (OLD.status='approved' AND NEW.status='retired'))
     OR (OLD.status <> 'draft' AND (NEW.approved_by,NEW.approved_at,NEW.change_note)
                                IS DISTINCT FROM (OLD.approved_by,OLD.approved_at,OLD.change_note))
  THEN RAISE EXCEPTION 'E-RCP-033 illegal change of version %', OLD.id USING ERRCODE='check_violation'; END IF;
  RETURN NEW;
END $$;
CREATE TRIGGER recipe_version_transition BEFORE UPDATE ON recipe_version FOR EACH ROW EXECUTE FUNCTION recipe_version_transition();
```

**H2 — §4.1: an UPDATE on a child table checks only the NEW parent.** Moving a limit from an approved parameter to a draft parameter removes it from the approved version (verified: 3 000 → 2 999 limits). Fix (tested; contains C1 and H3 fixes):
```sql
CREATE OR REPLACE FUNCTION enforce_version_mutable() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE v_old UUID; v_new UUID; v UUID;
BEGIN
  IF TG_TABLE_NAME = 'parameter' THEN
    IF TG_OP <> 'INSERT' THEN v_old := OLD.recipe_version_id; END IF;
    IF TG_OP <> 'DELETE' THEN v_new := NEW.recipe_version_id; END IF;
  ELSE
    IF TG_OP <> 'INSERT' THEN
      SELECT p.recipe_version_id INTO v_old FROM parameter p WHERE p.id = OLD.parameter_id;
      IF NOT FOUND AND TG_OP = 'DELETE' THEN RETURN OLD; END IF;   -- cascade from a checked parent
    END IF;
    IF TG_OP <> 'DELETE' THEN SELECT p.recipe_version_id INTO v_new FROM parameter p WHERE p.id = NEW.parameter_id; END IF;
  END IF;
  IF TG_OP = 'UPDATE' AND v_old IS DISTINCT FROM v_new THEN
    RAISE EXCEPTION 'E-RCP-030 row cannot move to another version' USING ERRCODE='check_violation'; END IF;
  FOREACH v IN ARRAY ARRAY[v_old, v_new] LOOP
    CONTINUE WHEN v IS NULL;
    PERFORM 1 FROM recipe_version WHERE id = v AND status='draft' AND deleted_at IS NULL FOR SHARE;
    IF NOT FOUND THEN RAISE EXCEPTION 'E-RCP-031 version % is not a live draft', v USING ERRCODE='check_violation'; END IF;
  END LOOP;
  RETURN COALESCE(NEW, OLD);
END $$;
```

**H3 — §4.1 checks and then writes without a lock.** Verified: a parameter INSERT without the "touch", run while `ApproveVersion` holds `FOR UPDATE`, waits on the FK KEY SHARE lock and then commits into the approved version (`approved | P2_UNVALIDATED`). Fix: the `FOR SHARE` in the H2 function; run `ApproveVersion` in READ COMMITTED with `SELECT … FROM recipe_version WHERE id=$v FOR UPDATE`, then validate. SERIALIZABLE does not help here (its snapshot is taken before the lock wait, and READ COMMITTED writers take no SIREAD locks). Verified: with this fix the writer gets E-RCP-031.

**H4 — §6 lines 814, 817: SERIALIZABLE does not protect `DeleteRecipe` against `ApproveVersion`.** Verified: both committed, leaving recipe R2 deleted with v1 approved (no read/write cycle because ApproveVersion never reads `recipe`). The database also never checks `recipe.deleted_at` for CreateVersion or parameter edits. Fix: READ COMMITTED and the first statement of every Recipe command is `SELECT deleted_at FROM recipe WHERE id = $r FOR UPDATE` (require NULL). One lock order (recipe → version → children), no deadlocks, no 40001 retries. Keep `row_version` as the UI token. Keep SERIALIZABLE (or `SELECT … FROM app_user WHERE role='administrator' AND is_active FOR UPDATE`) only for SetUserRole and DeactivateUser.

**H5 — §5 lines 764–766: table-level UPDATE grants let the app rewrite key columns.** Verified as `recipe_app`: `UPDATE recipe SET machine_id = <other machine, same platform>` and `UPDATE unit SET path='U2'` succeed. Fix (tested):
```sql
REVOKE UPDATE ON machine, unit, recipe, recipe_version, parameter, app_user FROM recipe_app;
GRANT UPDATE (name, plc_platform, archived_at) ON machine TO recipe_app;
GRANT UPDATE (name, archived_at) ON unit TO recipe_app;
GRANT UPDATE (name, deleted_at) ON recipe TO recipe_app;
GRANT UPDATE (status, change_note, updated_at, approved_by, approved_at, retired_at, deleted_at) ON recipe_version TO recipe_app;
GRANT UPDATE (unit_id, tag, name_en, description_en, plc_type_id, s7_address, value, ordinal) ON parameter TO recipe_app;
GRANT UPDATE (role, password_hash, is_active, deactivated_at) ON app_user TO recipe_app;
```

**H6 — §5 lines 751–752 and §8 line 872: migration 0001 cannot run.** `CREATE ROLE` inside 0001 run as `recipe_migrator` → `permission denied to create role` (verified); every GRANT after that fails; roles are cluster-wide and not in `pg_dump`. Fix: a `docker-entrypoint-initdb.d/*.sh` script, run as superuser, that reads the Docker secrets and runs `CREATE ROLE recipe_migrator LOGIN PASSWORD …; CREATE ROLE recipe_app LOGIN PASSWORD …; CREATE DATABASE recipe OWNER recipe_migrator; REVOKE TEMPORARY ON DATABASE recipe FROM PUBLIC;`.

**H7 — §3.6 line 343 and §6 lines 820–821: "DELETE session rows of the user" cannot be written in SQL** (user id is inside `data BYTEA`). Fix: `ALTER TABLE app_user ADD COLUMN session_epoch BIGINT NOT NULL DEFAULT 0;` the session stores `(user_id, epoch)`; each request does `SELECT is_active, role, session_epoch FROM app_user WHERE id=$uid`; Deactivate/ResetPassword increment the epoch.

## MEDIUM

**M1 — §3.3: the type-shape CHECKs pass when values are NULL** (`dims_valid(NULL, NULL)` returns NULL). Verified as accepted: array row with NULL dimensions; `dim_lower = ARRAY[1,NULL]`; STRING row without `string_length`; member attached to a primitive type. `upper - lower < 65536` is not enforced. `string_length BETWEEN 1 AND 254` is the S7 STRING limit only (WSTRING on S7-1500 up to 16382, CODESYS/TwinCAT STRING(n) above 254). BOOL "1 bit" but `size_bytes` NOT NULL in bytes. Fix (tested):
```sql
CREATE OR REPLACE FUNCTION dims_valid(lower_b INTEGER[], upper_b INTEGER[]) RETURNS BOOLEAN LANGUAGE sql IMMUTABLE AS $$
  SELECT COALESCE(array_ndims(lower_b)=1 AND array_ndims(upper_b)=1 AND cardinality(lower_b)=cardinality(upper_b)
    AND array_position(lower_b,NULL) IS NULL AND array_position(upper_b,NULL) IS NULL
    AND NOT EXISTS (SELECT 1 FROM generate_subscripts(lower_b,1) i
                    WHERE lower_b[i] > upper_b[i] OR upper_b[i]::bigint - lower_b[i] >= 65536), false) $$;
ALTER TABLE plc_type ADD CONSTRAINT plc_type_string_length_required
  CHECK ((kind='primitive' AND name IN ('STRING','WSTRING')) = (string_length IS NOT NULL));
-- plc_type_immutable INSERT branch: also SELECT kind; IF kind <> 'struct' THEN RAISE 'E-PLT-024 ...'
```

**M2 — §4.6 line 730 (`IF v_struct_created <> now()`): sound only if `created_at` is never set explicitly.** An explicit `created_at` from Rust makes every member insert fail with E-PLT-023 (verified). It also does not guarantee at least one member. Fix: `REVOKE INSERT ON plc_type FROM recipe_app; GRANT INSERT (id,kind,name,platforms,size_bytes,string_length,element_type_id,dim_lower,dim_upper,machine_id,revision,supersedes_type_id) ON plc_type TO recipe_app;` and a `CONSTRAINT TRIGGER … DEFERRABLE INITIALLY DEFERRED` requiring at least one member at commit.

**M3 — text length limits disagree with the domain:** DDL 2000 (change_note, description_en, translation description) vs reliability §4.3 4000. Use one limit.

**M4 — §3.9 `value JSONB` contradicts reliability §4.4 ("stored text is the decimal the user typed").** Verified: `1.5e2` reads back as `150`. No size cap; every edit writes before/after copies into audit_log forever. Fix: canonicalise numbers in the domain and document it, or use `JSON`; then `CHECK (pg_column_size(value) <= 1048576)`, `SET COMPRESSION lz4` on value and on audit before/after.

**M5 — §5 `GRANT SELECT ON ALL TABLES IN SCHEMA public` covers only existing tables.** Later migrations' tables get no grant → 42501 after upgrade. `GRANT EXECUTE` does nothing (PUBLIC already has it). Fix: `ALTER DEFAULT PRIVILEGES FOR ROLE recipe_migrator IN SCHEMA public GRANT SELECT ON TABLES TO recipe_app;` plus a storage test comparing `role_table_grants`/`role_column_grants` with a checked-in list.

**M6 — §3.6 "The store creates this table when it is missing" will not work.** tower-sessions-sqlx-store `migrate()` runs `CREATE SCHEMA IF NOT EXISTS` → `permission denied for database` as recipe_app (verified); default schema is `tower_sessions`. Fix: `PostgresStore::new(pool).with_schema_name("public")?.with_table_name("session")?` and never call `migrate()`.

**M7 — data-model §8 contradicts reliability §5:** file names `NNNN_` vs timestamp; "server runs `migrate!()` at start" vs one-shot `migrate` service; `0003_seed_admin.sql` cannot read env (static file). Fix: `recipe-server migrate` connects with `options=-c recipe.admin_login=… -c recipe.admin_hash=…` and the seed is `INSERT INTO app_user (login, password_hash, role) SELECT current_setting('recipe.admin_login'), current_setting('recipe.admin_hash'), 'administrator' WHERE NOT EXISTS (SELECT 1 FROM app_user WHERE role='administrator');`. Document that `CREATE INDEX CONCURRENTLY` needs a `-- no-transaction` migration.

**M8 — 10-year operation.** Image has `data_checksums = off` and libc collation `en_US.utf8`; a floating tag can move to a new glibc and silently corrupt TEXT unique indexes; PG16 support ends November 2028; `pg_dump` has no roles. Fix: `POSTGRES_INITDB_ARGS="--data-checksums --lc-collate=C --lc-ctype=C.UTF-8"` (UI sorting via `COLLATE "und-x-icu"`); pin `postgres:16.x-bookworm`; runbook for `pg_dumpall --globals-only` + `pg_dump -Fc` or `pg_upgrade`; `log_lock_waits = on`.

**M9 — reliability §7.8 `pg_stat_file` cannot measure free disk space** (size, timestamps, isdir only; recipe_app lacks EXECUTE). Fix: mount the pgdata volume read-only into `app` and call `statvfs`.

## LOW

**L1** — redundant indexes (`unit_machine_id_idx`, `recipe_version_recipe_id_idx`, `plc_type_platforms_gin`); `parameter (recipe_version_id, ordinal)` should be `(recipe_version_id, unit_id, ordinal)`; `ALTER TABLE recipe_version SET (fillfactor = 70)` for HOT updates.
**L2** — §3.2: the parent path rule is not enforced (`U1.2.4` can have parent `U3`). Fix (tested): `parent_path TEXT GENERATED ALWAYS AS (substring(path FROM '^(.*)\.[0-9]+$')) STORED, CONSTRAINT unit_parent_path_fk FOREIGN KEY (machine_id, parent_path) REFERENCES unit (machine_id, path)`.
**L3** — `login_attempt.login` has no length CHECK; count-then-insert race. Fix: `CHECK (length(login) <= 64)` and `SELECT pg_advisory_xact_lock(hashtextextended(login,0))` before the count.
**L4** — audit log: BRIN cannot serve "latest first" (page by PK `WHERE id < $c ORDER BY id DESC`); "history of recipe R" needs `aggregate_id UUID NOT NULL` + index `(aggregate_id, id)`.
**L5** — functions use unqualified names and PUBLIC keeps TEMP on the database (verified: recipe_app can create `TEMP TABLE recipe_version`, which would shadow the real table inside `version_is_live_draft()`). Fix: `REVOKE TEMPORARY ON DATABASE recipe FROM PUBLIC;` and `ALTER FUNCTION … SET search_path = public, pg_temp;` on every function.
**L6** — `based_on_version_id REFERENCES recipe_version (id)` can point to another recipe's version. Fix: `UNIQUE (recipe_id, id)` and `FOREIGN KEY (recipe_id, based_on_version_id) REFERENCES recipe_version (recipe_id, id)`.
**L7** — document contradictions: retry policy (3 attempts vs 3 retries; 50/100/150 vs 50/100/200 ms; 409 vs 503); SERIALIZABLE set (1 vs 6 commands); 23505 said to be retried but absent from the policy; I14 says no DELETE grant on `parameter` but §5 grants it; I5 CHECK text differs from the DDL; `updated_at` exists only on `recipe_version`.

## Verdict
1. The DDL runs on PostgreSQL 16.15 as written, except the role statements. But two core operations always fail: removing a parameter that has limits or translations (C1), and adding a second STRING(n) or array type to a machine (C2).
2. The database does not hold up as the backstop the docs claim. Approved versions can be reopened or stripped of limits, parameters can slip into a version being approved, recipes can move between machines, and SERIALIZABLE misses Delete against Approve. The fix is a recipe-row `FOR UPDATE` under READ COMMITTED, a version transition trigger, a locking 4.1 trigger and column-level grants.
3. Fix C1–C2, H1–H7 and M7 before writing `0001_schema.sql`. M8 and M9 must be decided before production, because they are hard to change after the data directory exists.
