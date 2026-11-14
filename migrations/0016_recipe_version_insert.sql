-- A new row of recipe_version is a live draft (ADR-0169, data-model.md section 4.14).
-- The file adds one function and one trigger. It scans and rewrites no table.
CREATE FUNCTION recipe_version_insert_draft() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.status IS DISTINCT FROM 'draft'
       OR NEW.row_version IS DISTINCT FROM 1
       OR NEW.deleted_at IS NOT NULL
    THEN
        RAISE EXCEPTION 'E-RCP-037 new version % is not a live draft', NEW.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER recipe_version_insert_draft
    BEFORE INSERT ON recipe_version
    FOR EACH ROW EXECUTE FUNCTION recipe_version_insert_draft();
