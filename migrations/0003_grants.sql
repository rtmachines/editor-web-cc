-- Grants of data-model.md section 5.2. Runs as recipe_migrator.
REVOKE ALL ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO recipe_app, recipe_backup, audit_owner;

-- Step 1. Every GRANT on the audit objects runs first, while recipe_migrator owns them.
-- Read everything, now and for tables added by later migrations.
-- audit_chain_head gets SELECT only, through this statement.
GRANT SELECT ON ALL TABLES IN SCHEMA public TO recipe_app;
ALTER DEFAULT PRIVILEGES FOR ROLE recipe_migrator IN SCHEMA public
    GRANT SELECT ON TABLES TO recipe_app;
-- Audit tables: INSERT only. The identity columns need no sequence grant.
GRANT INSERT ON audit_log, security_event TO recipe_app;
-- The only path that deletes security_event rows (4.12).
GRANT EXECUTE ON FUNCTION security_event_purge() TO recipe_app;
-- The aggregated security_event rows increment count (3.8). No other column.
GRANT UPDATE (count) ON security_event TO recipe_app;

-- Step 2. Ownership transfer to audit_owner. No SET ROLE. ALTER ... OWNER TO needs
-- CREATE on the schema for the new owner. audit_owner holds CREATE only for these
-- statements. The grants of step 1 stay, and audit_owner becomes their grantor.
GRANT CREATE ON SCHEMA public TO audit_owner;
ALTER TABLE audit_log        OWNER TO audit_owner;
ALTER TABLE audit_chain_head OWNER TO audit_owner;
ALTER TABLE security_event   OWNER TO audit_owner;
ALTER FUNCTION audit_canonical(audit_log) OWNER TO audit_owner;
ALTER FUNCTION audit_hash_chain()         OWNER TO audit_owner;
ALTER FUNCTION security_event_purge()     OWNER TO audit_owner;
REVOKE CREATE ON SCHEMA public FROM audit_owner;
-- No REVOKE from recipe_migrator follows. The implicit privileges of the old owner
-- moved to audit_owner with the ownership.

-- Step 3. Grants on the tables that recipe_migrator keeps.
-- Inserts. Identity columns need no sequence grant.
GRANT INSERT ON machine, unit, plc_type_member, app_user, session, login_attempt,
                recipe, recipe_version, draft_editor, parameter, parameter_limit,
                parameter_translation, app_setting
    TO recipe_app;
-- plc_type: created_at is excluded so that trigger 4.6 can trust it.
GRANT INSERT (id, kind, name, platforms, size_bytes, string_length, leaf_class,
              element_type_id, dim_lower, dim_upper, machine_id, revision, supersedes_type_id)
    ON plc_type TO recipe_app;

-- Updates: column-level. Identifier and structure columns are not updatable.
GRANT UPDATE (name, archived_at, updated_at) ON machine TO recipe_app;
GRANT UPDATE (name, archived_at, updated_at) ON unit TO recipe_app;
GRANT UPDATE (name, deleted_at, updated_at) ON recipe TO recipe_app;
GRANT UPDATE (status, change_note, content_sha256, updated_at, approved_by, approved_at,
              retired_at, deleted_at) ON recipe_version TO recipe_app;
GRANT UPDATE (unit_id, tag, name_en, description_en, plc_type_id, s7_address, value, ordinal)
    ON parameter TO recipe_app;
GRANT UPDATE (min_value, max_value, max_length) ON parameter_limit TO recipe_app;
GRANT UPDATE (name, description) ON parameter_translation TO recipe_app;
GRANT UPDATE (role, password_hash, is_active, deactivated_at, session_epoch,
              must_change_password, updated_at) ON app_user TO recipe_app;
GRANT UPDATE (last_seen_at, expires_at, csrf_token) ON session TO recipe_app;
-- Login T2 marks its attempt as a success (section 6). No other column.
GRANT UPDATE (succeeded) ON login_attempt TO recipe_app;
GRANT UPDATE (value, updated_by, updated_at) ON app_setting TO recipe_app;

-- Deletes: only rows that are not business history.
GRANT DELETE ON session, login_attempt, draft_editor, parameter, parameter_limit,
                parameter_translation
    TO recipe_app;
