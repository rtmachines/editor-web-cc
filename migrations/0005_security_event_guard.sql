-- Guard of security_event (data-model.md section 5.4, ADR-0035).
-- The file changes an audit object, so it runs the five steps of data-model.md section 5.1.

-- Step 1.
GRANT CREATE ON SCHEMA public TO audit_owner;
-- Step 2.
SET ROLE audit_owner;
-- Step 3. The REVOKE comes first, because it also revokes the column privileges of the table privilege.
REVOKE INSERT ON security_event FROM recipe_app;
GRANT INSERT (kind, login_hmac, user_id, client_ip, user_agent, request_id, detail)
    ON security_event TO recipe_app;
CREATE FUNCTION security_event_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.count IS DISTINCT FROM (OLD.count OPERATOR(pg_catalog.+) 1)
       OR (pg_catalog.to_jsonb(NEW) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text)
          IS DISTINCT FROM
          (pg_catalog.to_jsonb(OLD) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text) THEN
        RAISE EXCEPTION 'E-AUD-003 security event % allows only count + 1', OLD.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
REVOKE EXECUTE ON FUNCTION security_event_guard() FROM PUBLIC;
CREATE TRIGGER security_event_guard BEFORE UPDATE ON security_event
    FOR EACH ROW EXECUTE FUNCTION security_event_guard();
-- Step 4.
RESET ROLE;
-- Step 5.
REVOKE CREATE ON SCHEMA public FROM audit_owner;
