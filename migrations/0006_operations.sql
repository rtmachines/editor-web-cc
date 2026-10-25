-- Operations objects (data-model.md section 5.5, ADR-0069).
-- The file creates two objects that audit_owner owns, so it runs the five steps of
-- data-model.md section 5.1.

-- Step 1.
GRANT CREATE ON SCHEMA public TO audit_owner;
-- Step 2.
SET ROLE audit_owner;
-- Step 3.
CREATE INDEX security_event_occurred_at_idx ON security_event (occurred_at);
-- One row. connected is false and lag_bytes is NULL when no row of walarchive exists.
-- The lag is pg_current_wal_lsn() - sent_lsn (reliability.md 3.7, wal.max_lag_bytes).
CREATE FUNCTION replication_state(OUT connected BOOLEAN, OUT lag_bytes NUMERIC)
LANGUAGE sql SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT pg_catalog.count(r.pid) OPERATOR(pg_catalog.>) 0,
           pg_catalog.max(pg_catalog.pg_wal_lsn_diff(pg_catalog.pg_current_wal_lsn(), r.sent_lsn))
    FROM pg_catalog.pg_stat_replication AS r
    WHERE r.usename OPERATOR(pg_catalog.=) 'recipe_replicator'::pg_catalog.name
      AND r.application_name OPERATOR(pg_catalog.=) 'walarchive'::pg_catalog.text;
$$;
REVOKE EXECUTE ON FUNCTION replication_state() FROM PUBLIC;
GRANT EXECUTE ON FUNCTION replication_state() TO recipe_app;
-- Step 4.
RESET ROLE;
-- Step 5.
REVOKE CREATE ON SCHEMA public FROM audit_owner;
