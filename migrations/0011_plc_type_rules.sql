-- Identifier rules of PLC types (ADR-0143, ADR-0145, data-model.md sections 3.3 and 3.4).
-- Every array type starts at the subscript 1 in both bound arrays (ADR-0145).
-- A struct name and a member name compare without case (ADR-0143).

-- Step 0: lock both tables against writes before the check (ADR-0143 (k)). The old application
-- keeps running during migrate (reliability.md section 7.10 step 2). Without the lock, a writer
-- that is still open could commit a row after the check and before the index builds.
-- The lock waits for such a writer and then blocks new writes until the file ends.
LOCK TABLE plc_type, plc_type_member IN SHARE MODE;

-- Steps 1 to 4: stop the file when a stored row breaks a new rule. The check runs over every
-- machine, also an archived one, and every revision. The exception rolls back the whole file.
CREATE FUNCTION plc_type_rule_check() RETURNS void
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_arrays  BIGINT;
    v_structs BIGINT;
    v_members BIGINT;
BEGIN
    SELECT count(*) INTO v_arrays FROM plc_type
    WHERE kind = 'array'
      AND (array_lower(dim_lower, 1) <> 1 OR array_lower(dim_upper, 1) <> 1);

    SELECT count(*) INTO v_structs FROM (
        SELECT 1 FROM plc_type
        WHERE kind = 'struct'
        GROUP BY machine_id, lower(name), revision
        HAVING count(DISTINCT name) > 1
    ) AS groups;

    SELECT count(*) INTO v_members FROM (
        SELECT 1 FROM plc_type_member
        GROUP BY struct_type_id, lower(member_name)
        HAVING count(*) > 1
    ) AS groups;

    IF v_arrays <> 0 OR v_structs <> 0 OR v_members <> 0 THEN
        RAISE EXCEPTION 'migration 0011 stopped: % array rows start at another subscript than 1, % groups of struct names and % groups of member names differ only in case. Run deploy/db/check-identifier-rules.sql (docs/ops/runbook.md).',
            v_arrays, v_structs, v_members;
    END IF;
END;
$$;

SELECT plc_type_rule_check();

DROP FUNCTION plc_type_rule_check();

CREATE OR REPLACE FUNCTION dims_valid(lower_b INTEGER[], upper_b INTEGER[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT COALESCE(
        array_ndims(lower_b) = 1 AND array_ndims(upper_b) = 1
        AND cardinality(lower_b) = cardinality(upper_b)
        -- At most 3 dimensions (Assumption A38).
        AND cardinality(lower_b) BETWEEN 1 AND 3
        -- Both arrays start at the subscript 1 (ADR-0145).
        AND array_lower(lower_b, 1) = 1 AND array_lower(upper_b, 1) = 1
        AND array_position(lower_b, NULL) IS NULL
        AND array_position(upper_b, NULL) IS NULL
        AND NOT EXISTS (
            SELECT 1 FROM generate_subscripts(lower_b, 1) AS i
            WHERE lower_b[i] > upper_b[i]
               OR upper_b[i]::bigint - lower_b[i] >= 65536
        ),
        false);
$$;

-- Step 6: a struct name compares without case on one machine (ADR-0143). The index has no
-- CONCURRENTLY (ADR-0143 (k)). The SHARE lock of step 0 keeps writers out until the file ends.
CREATE UNIQUE INDEX plc_type_struct_name_ci_unique
    ON plc_type (machine_id, lower(name), revision) WHERE kind = 'struct';

-- Step 7: a member name compares without case in one struct (ADR-0143). The index has no
-- CONCURRENTLY and the same lock as step 6.
CREATE UNIQUE INDEX plc_type_member_name_ci_unique
    ON plc_type_member (struct_type_id, lower(member_name));
