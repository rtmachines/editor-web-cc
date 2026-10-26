-- Insert columns of audit_log (data-model.md section 5.7, ADR-0098 (b), (c)).
-- The file changes an audit object, so it runs the five steps of data-model.md section 5.1.

-- Step 1.
GRANT CREATE ON SCHEMA public TO audit_owner;
-- Step 2.
SET ROLE audit_owner;
-- Step 3. The REVOKE comes first, because it also revokes the column privileges of the table privilege.
-- The grant holds no id, occurred_at, chain_seq, prev_hash, or entry_hash.
REVOKE INSERT ON audit_log FROM recipe_app;
GRANT INSERT (actor_user_id, action, entity_type, entity_id, aggregate_type, aggregate_id,
              before_state, after_state, request_id, client_ip, session_ref)
    ON audit_log TO recipe_app;
-- Step 4.
RESET ROLE;
-- Step 5.
REVOKE CREATE ON SCHEMA public FROM audit_owner;
