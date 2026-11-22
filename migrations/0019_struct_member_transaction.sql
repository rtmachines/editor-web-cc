-- A member insert needs the creating transaction of its struct (ADR-0212, data-model.md 4.6).
-- The file replaces one function. It scans and rewrites no table.
CREATE OR REPLACE FUNCTION plc_type_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_kind    TEXT;
    v_created TIMESTAMPTZ;
    v_xmin    xid;
BEGIN
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        RAISE EXCEPTION 'E-PLT-033 % rows are immutable', TG_TABLE_NAME
            USING ERRCODE = 'check_violation';
    END IF;
    -- INSERT on plc_type_member only.
    SELECT kind, created_at, xmin INTO v_kind, v_created, v_xmin
    FROM plc_type WHERE id = NEW.struct_type_id;
    -- E-PLT-035 fits a missing row: a type that is not visible is not a struct.
    IF NOT FOUND OR v_kind IS DISTINCT FROM 'struct' THEN
        RAISE EXCEPTION 'E-PLT-035 type % is not a struct', NEW.struct_type_id
            USING ERRCODE = 'check_violation';
    END IF;
    -- The xmin test refuses a second transaction with the same start time.
    -- The created_at test refuses an old struct whose 32-bit xmin matches after a wraparound.
    IF v_xmin IS DISTINCT FROM pg_current_xact_id()::xid
       OR v_created IS DISTINCT FROM now()
    THEN
        RAISE EXCEPTION 'E-PLT-034 members of type % are closed', NEW.struct_type_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;
