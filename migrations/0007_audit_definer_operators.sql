-- Qualified operators and types of three audit functions (data-model.md section 5.6, ADR-0098 (a)).
-- The file changes three audit objects, so it runs the five steps of data-model.md section 5.1.

-- Step 1.
GRANT CREATE ON SCHEMA public TO audit_owner;
-- Step 2.
SET ROLE audit_owner;
-- Step 3. CREATE OR REPLACE keeps the owner and the ACL. SECURITY DEFINER and the search_path repeat.
CREATE OR REPLACE FUNCTION audit_hash_chain() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_seq  BIGINT;
    v_prev pg_catalog.bytea;
BEGIN
    -- The row lock serializes all audit inserts. It waits for the commit of the previous writer.
    SELECT h.last_seq, h.last_hash INTO STRICT v_seq, v_prev
    FROM public.audit_chain_head AS h
    WHERE h.singleton
    FOR UPDATE;

    NEW.chain_seq  := v_seq OPERATOR(pg_catalog.+) 1;
    NEW.prev_hash  := v_prev;
    NEW.entry_hash := pg_catalog.sha256(
        COALESCE(v_prev, '\x'::pg_catalog.bytea)
        OPERATOR(pg_catalog.||) pg_catalog.convert_to(public.audit_canonical(NEW), 'UTF8'));

    UPDATE public.audit_chain_head
    SET last_seq = NEW.chain_seq, last_hash = NEW.entry_hash
    WHERE singleton;
    RETURN NEW;
END;
$$;
CREATE OR REPLACE FUNCTION security_event_purge() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_deleted BIGINT;
BEGIN
    -- One batch: security_event.retention 400 days, security_event.purge_batch 5000 rows.
    -- The primary key gives the oldest rows first.
    -- Every OPERATOR form groups from left to right, so the parentheses keep the old grouping.
    DELETE FROM public.security_event
    WHERE id OPERATOR(pg_catalog.=) ANY (
        SELECT e.id FROM public.security_event AS e
        WHERE e.occurred_at OPERATOR(pg_catalog.<)
              (pg_catalog.now() OPERATOR(pg_catalog.-) '400 days'::pg_catalog.interval)
        ORDER BY e.id
        LIMIT 5000);
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END;
$$;
CREATE OR REPLACE FUNCTION security_event_guard() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    -- The old text used IS DISTINCT FROM, which looks up the operator = in the search_path.
    -- The form (a IS NULL) <> (b IS NULL) OR COALESCE(a <> b, false) gives the same result,
    -- also for NULL. It uses the operator <> of pg_catalog only.
    IF ((NEW.count IS NULL) OPERATOR(pg_catalog.<>) ((OLD.count OPERATOR(pg_catalog.+) 1) IS NULL))
       OR COALESCE(NEW.count OPERATOR(pg_catalog.<>) (OLD.count OPERATOR(pg_catalog.+) 1), false)
       OR (((pg_catalog.to_jsonb(NEW) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text) IS NULL)
           OPERATOR(pg_catalog.<>)
           ((pg_catalog.to_jsonb(OLD) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text) IS NULL))
       OR COALESCE((pg_catalog.to_jsonb(NEW) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text)
                   OPERATOR(pg_catalog.<>)
                   (pg_catalog.to_jsonb(OLD) OPERATOR(pg_catalog.-) 'count'::pg_catalog.text),
                   false) THEN
        RAISE EXCEPTION 'E-AUD-003 security event % allows only count + 1', OLD.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
-- Step 4.
RESET ROLE;
-- Step 5.
REVOKE CREATE ON SCHEMA public FROM audit_owner;
