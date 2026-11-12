-- Data block number of a machine (ADR-0156, ADR-0157, design D2 item 2, data-model.md sections 3.1 and 4.13).
-- This is the copy step. The file copies plc_connection.db_number into machine.db_number.
-- The copy writes no audit entry (ADR-0156 (i)). A machine without a record keeps NULL.
-- An IEC record has no number, so the copy skips it and an IEC machine keeps NULL.
-- The copy does not touch a row that holds no number, so machine.updated_at stays as it is.
-- A machine with a number keeps it.

-- Lock every machine row in the order of id before the copy (ADR-0157 (g)). A SetPlcConnection of
-- the previous release that runs at the same time then ends before the copy reads its record.
-- A later one waits for the end of this file and fires the trigger below.
SELECT id FROM machine ORDER BY id FOR NO KEY UPDATE;

UPDATE machine SET db_number = plc_connection.db_number
FROM plc_connection
WHERE plc_connection.machine_id = machine.id
  AND machine.db_number IS NULL
  AND plc_connection.db_number IS NOT NULL;

-- During the expand period the previous release writes plc_connection.db_number only. The release
-- runs during the upgrade and again after a rollback (ADR-0157). This trigger keeps
-- machine.db_number equal to the record. It raises no error of its own. A number outside the limit
-- of the platform fails a range check, and the whole statement rolls back.
-- A row with an equal number is not written, so machine.updated_at does not move.
CREATE FUNCTION plc_connection_sync_db_number() RETURNS trigger
LANGUAGE plpgsql SECURITY INVOKER
SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.db_number IS NOT NULL THEN
        UPDATE machine SET db_number = NEW.db_number
        WHERE id = NEW.machine_id AND db_number IS DISTINCT FROM NEW.db_number;
    END IF;
    RETURN NULL;
END;
$$;

CREATE TRIGGER plc_connection_sync_db_number
    AFTER INSERT OR UPDATE OF db_number ON plc_connection
    FOR EACH ROW EXECUTE FUNCTION plc_connection_sync_db_number();
