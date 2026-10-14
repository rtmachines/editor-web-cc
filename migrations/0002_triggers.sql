-- Triggers and functions of the recipe editor. Copy of docs/architecture/data-model.md section 4.
-- Never edit this file after it ships. A change is a new forward migration.

-- 4.1 enforce_version_mutable (I3)
CREATE FUNCTION enforce_version_mutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_old UUID;
    v_new UUID;
    v     UUID;
BEGIN
    IF TG_TABLE_NAME = 'parameter' THEN
        IF TG_OP <> 'INSERT' THEN v_old := OLD.recipe_version_id; END IF;
        IF TG_OP <> 'DELETE' THEN v_new := NEW.recipe_version_id; END IF;
    ELSE
        -- Child tables carry recipe_version_id. No parent lookup is needed.
        IF TG_OP <> 'INSERT' THEN
            v_old := OLD.recipe_version_id;
            IF TG_OP = 'DELETE'
               AND NOT EXISTS (SELECT 1 FROM parameter p WHERE p.id = OLD.parameter_id) THEN
                RETURN OLD;   -- cascade from a parent DELETE that already passed this check
            END IF;
        END IF;
        IF TG_OP <> 'DELETE' THEN v_new := NEW.recipe_version_id; END IF;
    END IF;

    IF TG_OP = 'UPDATE' AND v_old IS DISTINCT FROM v_new THEN
        RAISE EXCEPTION 'E-RCP-030 row cannot move to another version'
            USING ERRCODE = 'check_violation';
    END IF;

    FOREACH v IN ARRAY ARRAY[v_old, v_new] LOOP
        CONTINUE WHEN v IS NULL;
        PERFORM 1 FROM recipe_version
        WHERE id = v AND status = 'draft' AND deleted_at IS NULL
        FOR SHARE;
        IF NOT FOUND THEN
            RAISE EXCEPTION 'E-RCP-031 version % is not a live draft', v
                USING ERRCODE = 'check_violation';
        END IF;
    END LOOP;
    RETURN COALESCE(NEW, OLD);
END;
$$;

CREATE TRIGGER parameter_mutable
    BEFORE INSERT OR UPDATE OR DELETE ON parameter
    FOR EACH ROW EXECUTE FUNCTION enforce_version_mutable();
CREATE TRIGGER parameter_limit_mutable
    BEFORE INSERT OR UPDATE OR DELETE ON parameter_limit
    FOR EACH ROW EXECUTE FUNCTION enforce_version_mutable();
CREATE TRIGGER parameter_translation_mutable
    BEFORE INSERT OR UPDATE OR DELETE ON parameter_translation
    FOR EACH ROW EXECUTE FUNCTION enforce_version_mutable();

-- 4.2 enforce_type_platform (I6)
CREATE FUNCTION enforce_type_platform() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_platforms      TEXT[];
    v_type_machine   UUID;
    v_recipe_machine UUID;
BEGIN
    SELECT t.platforms, t.machine_id INTO v_platforms, v_type_machine
    FROM plc_type t WHERE t.id = NEW.plc_type_id;

    IF NOT (NEW.plc_platform = ANY (v_platforms)) THEN
        RAISE EXCEPTION 'E-PLT-031 type % does not exist on platform %',
            NEW.plc_type_id, NEW.plc_platform USING ERRCODE = 'check_violation';
    END IF;

    SELECT r.machine_id INTO v_recipe_machine
    FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id
    WHERE v.id = NEW.recipe_version_id;

    IF v_type_machine IS NOT NULL AND v_type_machine <> v_recipe_machine THEN
        RAISE EXCEPTION 'E-PLT-032 type % belongs to another machine', NEW.plc_type_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER parameter_type_platform
    BEFORE INSERT OR UPDATE OF plc_type_id, plc_platform ON parameter
    FOR EACH ROW EXECUTE FUNCTION enforce_type_platform();

-- 4.3 enforce_unit_machine (I15)
CREATE FUNCTION enforce_unit_machine() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_unit_machine   UUID;
    v_recipe_machine UUID;
BEGIN
    SELECT u.machine_id INTO v_unit_machine FROM unit u WHERE u.id = NEW.unit_id;
    SELECT r.machine_id INTO v_recipe_machine
    FROM recipe_version v JOIN recipe r ON r.id = v.recipe_id
    WHERE v.id = NEW.recipe_version_id;

    IF v_unit_machine IS DISTINCT FROM v_recipe_machine THEN
        RAISE EXCEPTION 'E-RCP-032 unit % is not in the machine of the recipe', NEW.unit_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER parameter_unit_machine
    BEFORE INSERT OR UPDATE OF unit_id, recipe_version_id ON parameter
    FOR EACH ROW EXECUTE FUNCTION enforce_unit_machine();

-- 4.4 bump_row_version (I12)
CREATE FUNCTION bump_row_version() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    NEW.row_version := OLD.row_version + 1;
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

CREATE TRIGGER recipe_version_a_bump
    BEFORE UPDATE ON recipe_version
    FOR EACH ROW EXECUTE FUNCTION bump_row_version();

-- 4.5 machine_platform_frozen
CREATE FUNCTION machine_platform_frozen() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.plc_platform <> OLD.plc_platform
       AND EXISTS (SELECT 1 FROM recipe WHERE machine_id = OLD.id) THEN
        RAISE EXCEPTION 'E-PLT-030 machine % has recipes, platform is frozen', OLD.code
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER machine_platform_frozen
    BEFORE UPDATE OF plc_platform ON machine
    FOR EACH ROW EXECUTE FUNCTION machine_platform_frozen();

-- 4.6 plc_type_immutable (ADR-0006)
CREATE FUNCTION plc_type_immutable() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_kind    TEXT;
    v_created TIMESTAMPTZ;
BEGIN
    IF TG_OP IN ('UPDATE', 'DELETE') THEN
        RAISE EXCEPTION 'E-PLT-033 % rows are immutable', TG_TABLE_NAME
            USING ERRCODE = 'check_violation';
    END IF;
    -- INSERT on plc_type_member only.
    SELECT kind, created_at INTO v_kind, v_created FROM plc_type WHERE id = NEW.struct_type_id;
    IF v_kind <> 'struct' THEN
        RAISE EXCEPTION 'E-PLT-035 type % is not a struct', NEW.struct_type_id
            USING ERRCODE = 'check_violation';
    END IF;
    IF v_created <> now() THEN
        RAISE EXCEPTION 'E-PLT-034 members of type % are closed', NEW.struct_type_id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER plc_type_immutable
    BEFORE UPDATE OR DELETE ON plc_type
    FOR EACH ROW EXECUTE FUNCTION plc_type_immutable();
CREATE TRIGGER plc_type_member_immutable
    BEFORE INSERT OR UPDATE OR DELETE ON plc_type_member
    FOR EACH ROW EXECUTE FUNCTION plc_type_immutable();

-- A struct has at least one member at commit.
CREATE FUNCTION plc_type_struct_has_member() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF NEW.kind = 'struct'
       AND NOT EXISTS (SELECT 1 FROM plc_type_member WHERE struct_type_id = NEW.id) THEN
        RAISE EXCEPTION 'E-PLT-036 struct % has no member', NEW.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NULL;
END;
$$;

CREATE CONSTRAINT TRIGGER plc_type_struct_has_member
    AFTER INSERT ON plc_type
    DEFERRABLE INITIALLY DEFERRED
    FOR EACH ROW EXECUTE FUNCTION plc_type_struct_has_member();

-- 4.7 recipe_version_transition (I4, state machine of domain-model.md section 4)
CREATE FUNCTION recipe_version_transition() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    IF OLD.deleted_at IS NOT NULL
       OR OLD.status = 'retired'
       OR (NEW.id, NEW.recipe_id, NEW.plc_platform, NEW.number, NEW.based_on_version_id,
           NEW.created_by, NEW.created_at)
          IS DISTINCT FROM
          (OLD.id, OLD.recipe_id, OLD.plc_platform, OLD.number, OLD.based_on_version_id,
           OLD.created_by, OLD.created_at)
       OR NOT (   (OLD.status = 'draft'    AND NEW.status IN ('draft', 'approved'))
               OR (OLD.status = 'approved' AND NEW.status = 'retired'))
       OR (OLD.status <> 'draft'
           AND (NEW.approved_by, NEW.approved_at, NEW.change_note, NEW.content_sha256)
               IS DISTINCT FROM
               (OLD.approved_by, OLD.approved_at, OLD.change_note, OLD.content_sha256))
       OR (OLD.status = 'approved' AND NEW.deleted_at IS NOT NULL)
    THEN
        RAISE EXCEPTION 'E-RCP-033 illegal change of version %', OLD.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER recipe_version_b_transition
    BEFORE UPDATE ON recipe_version
    FOR EACH ROW EXECUTE FUNCTION recipe_version_transition();

-- 4.8 freeze_identifiers
CREATE FUNCTION freeze_identifiers() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_referenced BOOLEAN;
BEGIN
    IF TG_TABLE_NAME = 'machine' THEN
        IF NEW.code <> OLD.code THEN
            SELECT EXISTS (
                SELECT 1 FROM recipe r JOIN recipe_version v ON v.recipe_id = r.id
                WHERE r.machine_id = OLD.id AND v.status <> 'draft'
            ) INTO v_referenced;
            IF v_referenced THEN
                RAISE EXCEPTION 'E-PLT-037 code of machine % is frozen', OLD.code
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'unit' THEN
        IF NEW.path <> OLD.path OR NEW.machine_id <> OLD.machine_id THEN
            SELECT EXISTS (
                SELECT 1 FROM parameter p JOIN recipe_version v ON v.id = p.recipe_version_id
                WHERE p.unit_id = OLD.id AND v.status <> 'draft'
            ) INTO v_referenced;
            IF v_referenced THEN
                RAISE EXCEPTION 'E-PLT-038 path of unit % is frozen', OLD.path
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
    ELSIF TG_TABLE_NAME = 'recipe' THEN
        IF NEW.reference_number <> OLD.reference_number OR NEW.machine_id <> OLD.machine_id THEN
            SELECT EXISTS (
                SELECT 1 FROM recipe_version v WHERE v.recipe_id = OLD.id AND v.status <> 'draft'
            ) INTO v_referenced;
            IF v_referenced THEN
                RAISE EXCEPTION 'E-RCP-034 reference of recipe % is frozen', OLD.reference_number
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER machine_freeze_identifiers
    BEFORE UPDATE OF code ON machine FOR EACH ROW EXECUTE FUNCTION freeze_identifiers();
CREATE TRIGGER unit_freeze_identifiers
    BEFORE UPDATE OF path, machine_id ON unit FOR EACH ROW EXECUTE FUNCTION freeze_identifiers();
CREATE TRIGGER recipe_freeze_identifiers
    BEFORE UPDATE OF reference_number, machine_id ON recipe FOR EACH ROW EXECUTE FUNCTION freeze_identifiers();

-- 4.9 audit_hash_chain (ADR-0022)
-- Canonical text of one audit row. The trigger and the check call the same function.
-- Timestamps are epoch microseconds. The text does not depend on TimeZone or DateStyle.
CREATE FUNCTION audit_canonical(a public.audit_log) RETURNS TEXT
LANGUAGE sql STABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT pg_catalog.jsonb_build_object(
        'chain_seq',      a.chain_seq,
        'occurred_at_us', (pg_catalog.extract('epoch', a.occurred_at)
                           OPERATOR(pg_catalog.*) 1000000::pg_catalog.numeric)::bigint,
        'actor_user_id',  a.actor_user_id,
        'action',         a.action,
        'aggregate_type', a.aggregate_type,
        'aggregate_id',   a.aggregate_id,
        'entity_type',    a.entity_type,
        'entity_id',      a.entity_id,
        'before_state',   a.before_state,
        'after_state',    a.after_state,
        'request_id',     a.request_id,
        'client_ip',      a.client_ip,
        'session_ref',    a.session_ref
    )::text;
$$;

CREATE FUNCTION audit_hash_chain() RETURNS trigger
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_seq  BIGINT;
    v_prev BYTEA;
BEGIN
    -- The row lock serializes all audit inserts. It waits for the commit of the previous writer.
    SELECT h.last_seq, h.last_hash INTO STRICT v_seq, v_prev
    FROM public.audit_chain_head AS h
    WHERE h.singleton
    FOR UPDATE;

    NEW.chain_seq  := v_seq + 1;
    NEW.prev_hash  := v_prev;
    NEW.entry_hash := pg_catalog.sha256(
        COALESCE(v_prev, '\x'::bytea)
        || pg_catalog.convert_to(public.audit_canonical(NEW), 'UTF8'));

    UPDATE public.audit_chain_head
    SET last_seq = NEW.chain_seq, last_hash = NEW.entry_hash
    WHERE singleton;
    RETURN NEW;
END;
$$;
REVOKE EXECUTE ON FUNCTION audit_hash_chain() FROM PUBLIC;

CREATE TRIGGER audit_log_hash_chain
    BEFORE INSERT ON audit_log
    FOR EACH ROW EXECUTE FUNCTION audit_hash_chain();

-- 4.10 version_size_budget (Assumption A26)
CREATE FUNCTION version_size_budget() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_version UUID;
    v_count   BIGINT;
BEGIN
    FOR v_version IN SELECT DISTINCT recipe_version_id FROM inserted LOOP
        IF TG_TABLE_NAME = 'parameter' THEN
            SELECT count(*) INTO v_count FROM parameter WHERE recipe_version_id = v_version;
            IF v_count > 4000 THEN
                RAISE EXCEPTION 'E-RCP-035 version % has more than 4000 parameters', v_version
                    USING ERRCODE = 'check_violation';
            END IF;
        ELSE
            SELECT count(*) INTO v_count FROM parameter_limit WHERE recipe_version_id = v_version;
            IF v_count > 32768 THEN
                RAISE EXCEPTION 'E-RCP-036 version % has more than 32768 limit rows', v_version
                    USING ERRCODE = 'check_violation';
            END IF;
        END IF;
    END LOOP;
    RETURN NULL;
END;
$$;

CREATE TRIGGER parameter_size_budget
    AFTER INSERT ON parameter
    REFERENCING NEW TABLE AS inserted
    FOR EACH STATEMENT EXECUTE FUNCTION version_size_budget();
CREATE TRIGGER parameter_limit_size_budget
    AFTER INSERT ON parameter_limit
    REFERENCING NEW TABLE AS inserted
    FOR EACH STATEMENT EXECUTE FUNCTION version_size_budget();

-- 4.12 security_event_purge (ADR-0022)
CREATE FUNCTION security_event_purge() RETURNS BIGINT
LANGUAGE plpgsql SECURITY DEFINER
SET search_path = pg_catalog, public, pg_temp AS $$
DECLARE
    v_deleted BIGINT;
BEGIN
    -- One batch: security_event.retention 400 days, security_event.purge_batch 5000 rows.
    -- The primary key gives the oldest rows first.
    DELETE FROM public.security_event
    WHERE id IN (
        SELECT e.id FROM public.security_event AS e
        WHERE e.occurred_at < pg_catalog.now() - INTERVAL '400 days'
        ORDER BY e.id
        LIMIT 5000);
    GET DIAGNOSTICS v_deleted = ROW_COUNT;
    RETURN v_deleted;
END;
$$;
REVOKE EXECUTE ON FUNCTION security_event_purge() FROM PUBLIC;
