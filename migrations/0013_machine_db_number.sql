-- Data block number of a machine (ADR-0156, data-model.md sections 3.1 and 5.2).
-- This is the expand step. Migration 0014 copies the number from plc_connection into machine.
-- Migration 0015 runs VALIDATE CONSTRAINT on both constraints.
-- The ALTER statements add a column and two NOT VALID constraints. They scan no table.

-- Block every session on machine from the first statement to the end of the file. A previous
-- release that still runs cannot insert a live S7 machine without a record after the check reads.
-- The file takes the mode of ALTER TABLE at once. A weaker mode first would let a running command
-- lock a row, then wait for the file, while the file waits for that command. That is a deadlock.
-- The table holds the machines of one production line, so the wait is short.
LOCK TABLE machine IN ACCESS EXCLUSIVE MODE;

-- Stop the file when a live S7 machine has no plc_connection row. Such a machine has no number
-- to copy. An archived machine and an IEC machine pass. The exception rolls back the whole file.
CREATE FUNCTION machine_db_number_check() RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_codes TEXT;
BEGIN
    SELECT string_agg(machine.code, ', ' ORDER BY machine.code) INTO v_codes
    FROM machine
    WHERE machine.archived_at IS NULL
      AND machine.plc_platform IN ('S7_300', 'S7_1200', 'S7_1500')
      AND NOT EXISTS (
          SELECT 1 FROM plc_connection WHERE plc_connection.machine_id = machine.id
      );

    IF v_codes IS NOT NULL THEN
        RAISE EXCEPTION 'migration 0013 stopped: live S7 machines without a PLC connection record: %. Save the PLC connection of each listed machine with the previous release (docs/ops/runbook.md).',
            v_codes;
    END IF;
END;
$$;

SELECT machine_db_number_check();

DROP FUNCTION machine_db_number_check();

ALTER TABLE machine ADD COLUMN db_number INTEGER;

-- A live S7 machine has a data block number. An IEC machine has none (ADR-0156 (a)).
-- An archived S7 machine from before ADR-0156 may have none.
ALTER TABLE machine ADD CONSTRAINT machine_db_number_shape CHECK (
    (plc_platform NOT IN ('CodesysIec', 'TwinCat3') OR db_number IS NULL)
    AND (plc_platform NOT IN ('S7_300', 'S7_1200', 'S7_1500')
        OR db_number IS NOT NULL OR archived_at IS NOT NULL)
) NOT VALID;

-- The data block limit of the platform (ADR-0116 (c), ADR-0140 (a), ADR-0156 (b)).
ALTER TABLE machine ADD CONSTRAINT machine_db_number_range CHECK (
    db_number IS NULL
    OR plc_platform NOT IN ('S7_300', 'S7_1200', 'S7_1500')
    OR (plc_platform = 'S7_300' AND db_number BETWEEN 1 AND 16000)
    OR (plc_platform IN ('S7_1200', 'S7_1500') AND db_number BETWEEN 1 AND 59999)
) NOT VALID;

GRANT UPDATE (db_number) ON machine TO recipe_app;
