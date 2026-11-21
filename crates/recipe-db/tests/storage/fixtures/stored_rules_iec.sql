--
-- PostgreSQL database dump
--

\restrict IY1G2fHBf6p9pIbawsn6CiL30GqiS0yL6gaFDEKn1ldpQshphJV4LqgQGGgGDba

-- Dumped from database version 16.15 (Debian 16.15-1.pgdg12+2)
-- Dumped by pg_dump version 16.15 (Debian 16.15-1.pgdg12+2)

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

--
-- Name: audit_guard; Type: SCHEMA; Schema: -; Owner: postgres
--

CREATE SCHEMA audit_guard;


ALTER SCHEMA audit_guard OWNER TO postgres;

--
-- Name: audit_objects_dropped(); Type: FUNCTION; Schema: audit_guard; Owner: postgres
--

CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'pg_temp'
    AS $$
DECLARE
    v_object RECORD;
BEGIN
    FOR v_object IN SELECT * FROM pg_catalog.pg_event_trigger_dropped_objects() LOOP
        IF v_object.object_type IN ('table', 'function')
           AND v_object.address_names[pg_catalog.array_upper(v_object.address_names, 1)] IN (
               'audit_log', 'audit_chain_head', 'security_event',
               'audit_canonical', 'audit_hash_chain',
               'security_event_purge', 'security_event_guard',
               'replication_state')
           AND current_user <> 'audit_owner' THEN
            RAISE EXCEPTION 'audit object % can be dropped only by audit_owner',
                v_object.object_identity USING ERRCODE = 'insufficient_privilege';
        END IF;
    END LOOP;
END;
$$;


ALTER FUNCTION audit_guard.audit_objects_dropped() OWNER TO postgres;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: audit_log; Type: TABLE; Schema: public; Owner: audit_owner
--

CREATE TABLE public.audit_log (
    id bigint NOT NULL,
    chain_seq bigint NOT NULL,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL,
    actor_user_id uuid,
    action text NOT NULL,
    entity_type text NOT NULL,
    entity_id uuid NOT NULL,
    aggregate_type text NOT NULL,
    aggregate_id uuid NOT NULL,
    before_state jsonb,
    after_state jsonb,
    request_id uuid NOT NULL,
    client_ip inet,
    session_ref text,
    prev_hash bytea,
    entry_hash bytea NOT NULL,
    CONSTRAINT audit_log_action_format CHECK ((action ~ '^[A-Z][A-Za-z]{2,63}$'::text)),
    CONSTRAINT audit_log_aggregate_type_value CHECK ((aggregate_type = ANY (ARRAY['machine'::text, 'recipe'::text, 'app_user'::text, 'app_setting'::text, 'plc_type'::text]))),
    CONSTRAINT audit_log_entity_type_format CHECK ((entity_type ~ '^[a-z_]{2,64}$'::text)),
    CONSTRAINT audit_log_has_state CHECK (((before_state IS NOT NULL) OR (after_state IS NOT NULL))),
    CONSTRAINT audit_log_hash_length CHECK (((length(entry_hash) = 32) AND ((prev_hash IS NULL) OR (length(prev_hash) = 32)))),
    CONSTRAINT audit_log_session_ref_format CHECK (((session_ref IS NULL) OR (session_ref ~ '^[0-9a-f]{16}$'::text)))
);
ALTER TABLE ONLY public.audit_log ALTER COLUMN before_state SET COMPRESSION lz4;
ALTER TABLE ONLY public.audit_log ALTER COLUMN after_state SET COMPRESSION lz4;


ALTER TABLE public.audit_log OWNER TO audit_owner;

--
-- Name: audit_canonical(public.audit_log); Type: FUNCTION; Schema: public; Owner: audit_owner
--

CREATE FUNCTION public.audit_canonical(a public.audit_log) RETURNS text
    LANGUAGE sql STABLE
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.audit_canonical(a public.audit_log) OWNER TO audit_owner;

--
-- Name: audit_hash_chain(); Type: FUNCTION; Schema: public; Owner: audit_owner
--

CREATE FUNCTION public.audit_hash_chain() RETURNS trigger
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.audit_hash_chain() OWNER TO audit_owner;

--
-- Name: bump_row_version(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.bump_row_version() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    NEW.row_version := OLD.row_version + 1;
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.bump_row_version() OWNER TO recipe_migrator;

--
-- Name: dims_valid(integer[], integer[]); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.dims_valid(lower_b integer[], upper_b integer[]) RETURNS boolean
    LANGUAGE sql IMMUTABLE
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.dims_valid(lower_b integer[], upper_b integer[]) OWNER TO recipe_migrator;

--
-- Name: enforce_type_platform(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.enforce_type_platform() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.enforce_type_platform() OWNER TO recipe_migrator;

--
-- Name: enforce_unit_machine(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.enforce_unit_machine() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.enforce_unit_machine() OWNER TO recipe_migrator;

--
-- Name: enforce_version_mutable(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.enforce_version_mutable() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.enforce_version_mutable() OWNER TO recipe_migrator;

--
-- Name: freeze_identifiers(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.freeze_identifiers() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.freeze_identifiers() OWNER TO recipe_migrator;

--
-- Name: machine_platform_frozen(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.machine_platform_frozen() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.plc_platform <> OLD.plc_platform
       AND EXISTS (SELECT 1 FROM recipe WHERE machine_id = OLD.id) THEN
        RAISE EXCEPTION 'E-PLT-030 machine % has recipes, platform is frozen', OLD.code
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.machine_platform_frozen() OWNER TO recipe_migrator;

--
-- Name: plc_connection_sync_db_number(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.plc_connection_sync_db_number() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.db_number IS NOT NULL THEN
        UPDATE machine SET db_number = NEW.db_number
        WHERE id = NEW.machine_id AND db_number IS DISTINCT FROM NEW.db_number;
    END IF;
    RETURN NULL;
END;
$$;


ALTER FUNCTION public.plc_connection_sync_db_number() OWNER TO recipe_migrator;

--
-- Name: plc_type_immutable(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.plc_type_immutable() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.plc_type_immutable() OWNER TO recipe_migrator;

--
-- Name: plc_type_struct_has_member(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.plc_type_struct_has_member() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    IF NEW.kind = 'struct'
       AND NOT EXISTS (SELECT 1 FROM plc_type_member WHERE struct_type_id = NEW.id) THEN
        RAISE EXCEPTION 'E-PLT-036 struct % has no member', NEW.id
            USING ERRCODE = 'check_violation';
    END IF;
    RETURN NULL;
END;
$$;


ALTER FUNCTION public.plc_type_struct_has_member() OWNER TO recipe_migrator;

--
-- Name: recipe_version_insert_draft(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.recipe_version_insert_draft() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.recipe_version_insert_draft() OWNER TO recipe_migrator;

--
-- Name: recipe_version_transition(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.recipe_version_transition() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.recipe_version_transition() OWNER TO recipe_migrator;

--
-- Name: replication_state(); Type: FUNCTION; Schema: public; Owner: audit_owner
--

CREATE FUNCTION public.replication_state(OUT connected boolean, OUT lag_bytes numeric) RETURNS record
    LANGUAGE sql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
    SELECT pg_catalog.count(r.pid) OPERATOR(pg_catalog.>) 0,
           pg_catalog.max(pg_catalog.pg_wal_lsn_diff(pg_catalog.pg_current_wal_lsn(), r.sent_lsn))
    FROM pg_catalog.pg_stat_replication AS r
    WHERE r.usename OPERATOR(pg_catalog.=) 'recipe_replicator'::pg_catalog.name
      AND r.application_name OPERATOR(pg_catalog.=) 'walarchive'::pg_catalog.text;
$$;


ALTER FUNCTION public.replication_state(OUT connected boolean, OUT lag_bytes numeric) OWNER TO audit_owner;

--
-- Name: security_event_guard(); Type: FUNCTION; Schema: public; Owner: audit_owner
--

CREATE FUNCTION public.security_event_guard() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.security_event_guard() OWNER TO audit_owner;

--
-- Name: security_event_purge(); Type: FUNCTION; Schema: public; Owner: audit_owner
--

CREATE FUNCTION public.security_event_purge() RETURNS bigint
    LANGUAGE plpgsql SECURITY DEFINER
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.security_event_purge() OWNER TO audit_owner;

--
-- Name: set_updated_at(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.set_updated_at() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;


ALTER FUNCTION public.set_updated_at() OWNER TO recipe_migrator;

--
-- Name: version_size_budget(); Type: FUNCTION; Schema: public; Owner: recipe_migrator
--

CREATE FUNCTION public.version_size_budget() RETURNS trigger
    LANGUAGE plpgsql
    SET search_path TO 'pg_catalog', 'public', 'pg_temp'
    AS $$
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


ALTER FUNCTION public.version_size_budget() OWNER TO recipe_migrator;

--
-- Name: _sqlx_migrations; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public._sqlx_migrations (
    version bigint NOT NULL,
    description text NOT NULL,
    installed_on timestamp with time zone DEFAULT now() NOT NULL,
    success boolean NOT NULL,
    checksum bytea NOT NULL,
    execution_time bigint NOT NULL
);


ALTER TABLE public._sqlx_migrations OWNER TO recipe_migrator;

--
-- Name: app_setting; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.app_setting (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    key text NOT NULL,
    value text NOT NULL,
    updated_by uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT app_setting_key_format CHECK ((key ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$'::text)),
    CONSTRAINT app_setting_value_length CHECK ((length(value) <= 1000))
);


ALTER TABLE public.app_setting OWNER TO recipe_migrator;

--
-- Name: app_user; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.app_user (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    login text NOT NULL,
    password_hash text NOT NULL,
    role text NOT NULL,
    is_active boolean DEFAULT true NOT NULL,
    session_epoch bigint DEFAULT 0 NOT NULL,
    must_change_password boolean DEFAULT false NOT NULL,
    deactivated_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT app_user_active_consistent CHECK ((is_active = (deactivated_at IS NULL))),
    CONSTRAINT app_user_epoch_nonnegative CHECK ((session_epoch >= 0)),
    CONSTRAINT app_user_export_client_password CHECK (((role <> 'export_client'::text) OR (NOT must_change_password))),
    CONSTRAINT app_user_hash_format CHECK ((password_hash ~~ '$argon2id$%'::text)),
    CONSTRAINT app_user_login_format CHECK ((login ~ '^[a-z0-9._-]{3,64}$'::text)),
    CONSTRAINT app_user_role_value CHECK ((role = ANY (ARRAY['administrator'::text, 'process_engineer'::text, 'operator'::text, 'export_client'::text])))
);


ALTER TABLE public.app_user OWNER TO recipe_migrator;

--
-- Name: audit_chain_head; Type: TABLE; Schema: public; Owner: audit_owner
--

CREATE TABLE public.audit_chain_head (
    singleton boolean DEFAULT true NOT NULL,
    last_seq bigint DEFAULT 0 NOT NULL,
    last_hash bytea,
    CONSTRAINT audit_chain_head_hash_length CHECK (((last_hash IS NULL) OR (length(last_hash) = 32))),
    CONSTRAINT audit_chain_head_one_row CHECK (singleton),
    CONSTRAINT audit_chain_head_seq_nonnegative CHECK ((last_seq >= 0))
);


ALTER TABLE public.audit_chain_head OWNER TO audit_owner;

--
-- Name: audit_log_id_seq; Type: SEQUENCE; Schema: public; Owner: audit_owner
--

ALTER TABLE public.audit_log ALTER COLUMN id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.audit_log_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: draft_editor; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.draft_editor (
    recipe_version_id uuid NOT NULL,
    user_id uuid NOT NULL,
    first_edit_at timestamp with time zone DEFAULT now() NOT NULL
);


ALTER TABLE public.draft_editor OWNER TO recipe_migrator;

--
-- Name: login_attempt; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.login_attempt (
    id bigint NOT NULL,
    login text NOT NULL,
    client_ip inet NOT NULL,
    attempted_at timestamp with time zone DEFAULT now() NOT NULL,
    succeeded boolean NOT NULL,
    CONSTRAINT login_attempt_login_length CHECK (((length(login) >= 1) AND (length(login) <= 64)))
);


ALTER TABLE public.login_attempt OWNER TO recipe_migrator;

--
-- Name: login_attempt_id_seq; Type: SEQUENCE; Schema: public; Owner: recipe_migrator
--

ALTER TABLE public.login_attempt ALTER COLUMN id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.login_attempt_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: machine; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.machine (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    code text NOT NULL,
    name text NOT NULL,
    plc_platform text NOT NULL,
    archived_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    db_number integer,
    CONSTRAINT machine_code_format CHECK ((code ~ '^M[1-9][0-9]{0,6}$'::text)),
    CONSTRAINT machine_db_number_range CHECK (((db_number IS NULL) OR (plc_platform <> ALL (ARRAY['S7_300'::text, 'S7_1200'::text, 'S7_1500'::text])) OR ((plc_platform = 'S7_300'::text) AND ((db_number >= 1) AND (db_number <= 16000))) OR ((plc_platform = ANY (ARRAY['S7_1200'::text, 'S7_1500'::text])) AND ((db_number >= 1) AND (db_number <= 59999))))),
    CONSTRAINT machine_db_number_shape CHECK ((((plc_platform <> ALL (ARRAY['CodesysIec'::text, 'TwinCat3'::text])) OR (db_number IS NULL)) AND ((plc_platform <> ALL (ARRAY['S7_300'::text, 'S7_1200'::text, 'S7_1500'::text])) OR (db_number IS NOT NULL) OR (archived_at IS NOT NULL)))),
    CONSTRAINT machine_name_length CHECK (((length(name) >= 1) AND (length(name) <= 200))),
    CONSTRAINT machine_platform_value CHECK ((plc_platform = ANY (ARRAY['CodesysIec'::text, 'TwinCat3'::text, 'S7_1200'::text, 'S7_1500'::text, 'S7_300'::text])))
);


ALTER TABLE public.machine OWNER TO recipe_migrator;

--
-- Name: parameter; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.parameter (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    recipe_version_id uuid NOT NULL,
    plc_platform text NOT NULL,
    parameter_key uuid DEFAULT gen_random_uuid() NOT NULL,
    unit_id uuid NOT NULL,
    tag text NOT NULL,
    name_en text NOT NULL,
    description_en text NOT NULL,
    plc_type_id uuid NOT NULL,
    s7_address text,
    value jsonb NOT NULL,
    ordinal integer DEFAULT 0 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT parameter_description_length CHECK (((length(description_en) >= 1) AND (length(description_en) <= 4000))),
    CONSTRAINT parameter_name_length CHECK (((length(name_en) >= 1) AND (length(name_en) <= 200))),
    CONSTRAINT parameter_ordinal_range CHECK (((ordinal >= 0) AND (ordinal <= 100000))),
    CONSTRAINT parameter_s7_address_format CHECK (((s7_address IS NULL) OR (s7_address ~ '^DB[1-9][0-9]{0,4}\.DB(X[0-9]{1,5}\.[0-7]|[BWD][0-9]{1,5})$'::text))),
    CONSTRAINT parameter_s7_address_platform CHECK ((((plc_platform = 'S7_300'::text) AND (s7_address IS NOT NULL)) OR (plc_platform = ANY (ARRAY['S7_1200'::text, 'S7_1500'::text])) OR ((plc_platform = ANY (ARRAY['CodesysIec'::text, 'TwinCat3'::text])) AND (s7_address IS NULL)))),
    CONSTRAINT parameter_tag_format CHECK ((tag ~ '^[A-Z][A-Z0-9_]{0,63}$'::text)),
    CONSTRAINT parameter_value_size CHECK ((pg_column_size(value) <= 1048576))
);
ALTER TABLE ONLY public.parameter ALTER COLUMN value SET COMPRESSION lz4;


ALTER TABLE public.parameter OWNER TO recipe_migrator;

--
-- Name: parameter_limit; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.parameter_limit (
    parameter_id uuid NOT NULL,
    recipe_version_id uuid NOT NULL,
    leaf_path text DEFAULT ''::text NOT NULL,
    min_value numeric,
    max_value numeric,
    max_length smallint,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT parameter_limit_max_length_range CHECK (((max_length IS NULL) OR ((max_length >= 1) AND (max_length <= 16382)))),
    CONSTRAINT parameter_limit_min_le_max CHECK (((min_value IS NULL) OR (max_value IS NULL) OR (min_value <= max_value))),
    CONSTRAINT parameter_limit_not_empty CHECK (((min_value IS NOT NULL) OR (max_value IS NOT NULL) OR (max_length IS NOT NULL))),
    CONSTRAINT parameter_limit_one_kind CHECK (((max_length IS NULL) OR ((min_value IS NULL) AND (max_value IS NULL)))),
    CONSTRAINT parameter_limit_path_format CHECK ((leaf_path ~ '^(\.[A-Za-z_][A-Za-z0-9_]{0,63}|\[(\*|-?[0-9]{1,10}(,-?[0-9]{1,10}){0,2})\])*$'::text))
);


ALTER TABLE public.parameter_limit OWNER TO recipe_migrator;

--
-- Name: parameter_translation; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.parameter_translation (
    parameter_id uuid NOT NULL,
    recipe_version_id uuid NOT NULL,
    lang text NOT NULL,
    name text NOT NULL,
    description text DEFAULT ''::text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT parameter_translation_description_length CHECK ((length(description) <= 4000)),
    CONSTRAINT parameter_translation_lang_format CHECK (((lang ~ '^[a-z]{2,3}(-[A-Z]{2})?$'::text) AND (lang <> 'en'::text))),
    CONSTRAINT parameter_translation_name_length CHECK (((length(name) >= 1) AND (length(name) <= 200)))
);


ALTER TABLE public.parameter_translation OWNER TO recipe_migrator;

--
-- Name: plc_connection; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.plc_connection (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    machine_id uuid NOT NULL,
    plc_platform text NOT NULL,
    host text NOT NULL,
    rack smallint,
    slot smallint,
    db_number integer,
    port integer,
    address text,
    comm_mode text,
    login text,
    password_sealed bytea,
    row_version bigint DEFAULT 1 NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT plc_connection_address_length CHECK (((length(address) >= 1) AND (length(address) <= 200))),
    CONSTRAINT plc_connection_ams_net_id CHECK (((comm_mode IS DISTINCT FROM 'Ads'::text) OR (address ~ '^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){5}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])$'::text))),
    CONSTRAINT plc_connection_credentials CHECK ((((login IS NULL) = (password_sealed IS NULL)) AND
CASE
    WHEN (comm_mode = ANY (ARRAY['OpcUa'::text, 'CodesysV3'::text])) THEN (login IS NOT NULL)
    WHEN (comm_mode = 'Ads'::text) THEN true
    ELSE (login IS NULL)
END)),
    CONSTRAINT plc_connection_db_number_range CHECK (((db_number IS NULL) OR ((plc_platform = 'S7_300'::text) AND ((db_number >= 1) AND (db_number <= 16000))) OR ((plc_platform = ANY (ARRAY['S7_1200'::text, 'S7_1500'::text])) AND ((db_number >= 1) AND (db_number <= 59999))))),
    CONSTRAINT plc_connection_host_format CHECK (((host ~ '^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])$'::text) OR ((host !~ '^[0-9.]+$'::text) AND (host ~ '^[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?)*$'::text)))),
    CONSTRAINT plc_connection_host_length CHECK (((length(host) >= 1) AND (length(host) <= 253))),
    CONSTRAINT plc_connection_login_length CHECK (((length(login) >= 1) AND (length(login) <= 64))),
    CONSTRAINT plc_connection_mode_value CHECK (((comm_mode IS NULL) OR ((plc_platform = 'CodesysIec'::text) AND (comm_mode = ANY (ARRAY['OpcUa'::text, 'CodesysV3'::text, 'ModbusTcp'::text]))) OR ((plc_platform = 'TwinCat3'::text) AND (comm_mode = ANY (ARRAY['Ads'::text, 'OpcUa'::text, 'ModbusTcp'::text]))))),
    CONSTRAINT plc_connection_port_range CHECK (((port >= 1) AND (port <= 65535))),
    CONSTRAINT plc_connection_rack_range CHECK (((rack >= 0) AND (rack <= 7))),
    CONSTRAINT plc_connection_row_version_positive CHECK ((row_version >= 1)),
    CONSTRAINT plc_connection_shape CHECK ((((plc_platform = ANY (ARRAY['S7_300'::text, 'S7_1200'::text, 'S7_1500'::text])) AND (rack IS NOT NULL) AND (slot IS NOT NULL) AND (db_number IS NOT NULL) AND (port IS NULL) AND (address IS NULL) AND (comm_mode IS NULL)) OR ((plc_platform = ANY (ARRAY['CodesysIec'::text, 'TwinCat3'::text])) AND (port IS NOT NULL) AND (address IS NOT NULL) AND (comm_mode IS NOT NULL) AND (rack IS NULL) AND (slot IS NULL) AND (db_number IS NULL)))),
    CONSTRAINT plc_connection_slot_range CHECK (((slot >= 0) AND (slot <= 31))),
    CONSTRAINT plc_connection_text_no_control CHECK (((address !~ '[[:cntrl:]]'::text) AND (login !~ '[[:cntrl:]]'::text)))
);


ALTER TABLE public.plc_connection OWNER TO recipe_migrator;

--
-- Name: plc_type; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.plc_type (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    kind text NOT NULL,
    name text,
    platforms text[] NOT NULL,
    size_bytes integer,
    string_length smallint,
    leaf_class text,
    element_type_id uuid,
    dim_lower integer[],
    dim_upper integer[],
    machine_id uuid,
    revision integer DEFAULT 1 NOT NULL,
    supersedes_type_id uuid,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT plc_type_array_shape CHECK (((kind <> 'array'::text) OR ((element_type_id IS NOT NULL) AND public.dims_valid(dim_lower, dim_upper) AND (size_bytes IS NULL) AND (string_length IS NULL) AND (machine_id IS NOT NULL)))),
    CONSTRAINT plc_type_kind_value CHECK ((kind = ANY (ARRAY['primitive'::text, 'array'::text, 'struct'::text]))),
    CONSTRAINT plc_type_leaf_class_present CHECK (((kind = 'primitive'::text) = (leaf_class IS NOT NULL))),
    CONSTRAINT plc_type_leaf_class_string CHECK ((COALESCE((leaf_class = 'string'::text), false) = (string_length IS NOT NULL))),
    CONSTRAINT plc_type_leaf_class_value CHECK (((leaf_class IS NULL) OR (leaf_class = ANY (ARRAY['numeric'::text, 'string'::text, 'bool'::text, 'char'::text, 'duration'::text, 'instant'::text])))),
    CONSTRAINT plc_type_name_format CHECK (((name IS NULL) OR (name ~ '^[A-Za-z_][A-Za-z0-9_]{0,63}$'::text))),
    CONSTRAINT plc_type_name_present CHECK (((kind = 'array'::text) = (name IS NULL))),
    CONSTRAINT plc_type_platforms_value CHECK (((cardinality(platforms) >= 1) AND (platforms <@ ARRAY['CodesysIec'::text, 'TwinCat3'::text, 'S7_1200'::text, 'S7_1500'::text, 'S7_300'::text]))),
    CONSTRAINT plc_type_primitive_shape CHECK (((kind <> 'primitive'::text) OR ((element_type_id IS NULL) AND (dim_lower IS NULL) AND (dim_upper IS NULL) AND (size_bytes IS NOT NULL) AND (size_bytes >= 1) AND ((machine_id IS NULL) OR (name = ANY (ARRAY['STRING'::text, 'WSTRING'::text])))))),
    CONSTRAINT plc_type_revision_positive CHECK ((revision >= 1)),
    CONSTRAINT plc_type_string_length_range CHECK (((string_length IS NULL) OR ((string_length >= 1) AND (string_length <= 16382)))),
    CONSTRAINT plc_type_string_length_required CHECK ((((kind = 'primitive'::text) AND (name = ANY (ARRAY['STRING'::text, 'WSTRING'::text]))) = (string_length IS NOT NULL))),
    CONSTRAINT plc_type_struct_shape CHECK (((kind <> 'struct'::text) OR ((element_type_id IS NULL) AND (dim_lower IS NULL) AND (dim_upper IS NULL) AND (size_bytes IS NULL) AND (string_length IS NULL) AND (machine_id IS NOT NULL)))),
    CONSTRAINT plc_type_supersedes_only_struct CHECK (((supersedes_type_id IS NULL) OR (kind = 'struct'::text)))
);


ALTER TABLE public.plc_type OWNER TO recipe_migrator;

--
-- Name: plc_type_member; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.plc_type_member (
    struct_type_id uuid NOT NULL,
    ordinal smallint NOT NULL,
    member_name text NOT NULL,
    member_type_id uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT plc_type_member_name_format CHECK ((member_name ~ '^[A-Za-z_][A-Za-z0-9_]{0,63}$'::text)),
    CONSTRAINT plc_type_member_no_self CHECK ((struct_type_id <> member_type_id)),
    CONSTRAINT plc_type_member_ordinal_range CHECK (((ordinal >= 1) AND (ordinal <= 1024)))
);


ALTER TABLE public.plc_type_member OWNER TO recipe_migrator;

--
-- Name: recipe; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.recipe (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    machine_id uuid NOT NULL,
    plc_platform text NOT NULL,
    reference_number text NOT NULL,
    name text NOT NULL,
    deleted_at timestamp with time zone,
    created_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT recipe_name_length CHECK (((length(name) >= 1) AND (length(name) <= 200))),
    CONSTRAINT recipe_reference_format CHECK ((reference_number ~ '^[A-Z0-9][A-Z0-9_-]{0,49}$'::text))
);


ALTER TABLE public.recipe OWNER TO recipe_migrator;

--
-- Name: recipe_version; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.recipe_version (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    recipe_id uuid NOT NULL,
    plc_platform text NOT NULL,
    number integer NOT NULL,
    status text DEFAULT 'draft'::text NOT NULL,
    row_version bigint DEFAULT 1 NOT NULL,
    based_on_version_id uuid,
    change_note text NOT NULL,
    content_sha256 text,
    created_by uuid NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    approved_by uuid,
    approved_at timestamp with time zone,
    retired_at timestamp with time zone,
    deleted_at timestamp with time zone,
    CONSTRAINT recipe_version_change_note_length CHECK (((length(change_note) >= 1) AND (length(change_note) <= 4000))),
    CONSTRAINT recipe_version_deleted_only_draft CHECK (((deleted_at IS NULL) OR (status = 'draft'::text))),
    CONSTRAINT recipe_version_hash_format CHECK (((content_sha256 IS NULL) OR (content_sha256 ~ '^[0-9a-f]{64}$'::text))),
    CONSTRAINT recipe_version_number_positive CHECK ((number >= 1)),
    CONSTRAINT recipe_version_row_version_positive CHECK ((row_version >= 1)),
    CONSTRAINT recipe_version_status_fields CHECK ((((status = 'draft'::text) AND (approved_at IS NULL) AND (approved_by IS NULL) AND (retired_at IS NULL) AND (content_sha256 IS NULL)) OR ((status = 'approved'::text) AND (approved_at IS NOT NULL) AND (approved_by IS NOT NULL) AND (retired_at IS NULL) AND (content_sha256 IS NOT NULL)) OR ((status = 'retired'::text) AND (approved_at IS NOT NULL) AND (approved_by IS NOT NULL) AND (retired_at IS NOT NULL) AND (content_sha256 IS NOT NULL)))),
    CONSTRAINT recipe_version_status_value CHECK ((status = ANY (ARRAY['draft'::text, 'approved'::text, 'retired'::text])))
)
WITH (fillfactor='70');


ALTER TABLE public.recipe_version OWNER TO recipe_migrator;

--
-- Name: security_event; Type: TABLE; Schema: public; Owner: audit_owner
--

CREATE TABLE public.security_event (
    id bigint NOT NULL,
    occurred_at timestamp with time zone DEFAULT now() NOT NULL,
    kind text NOT NULL,
    login_hmac bytea,
    user_id uuid,
    client_ip inet,
    user_agent text,
    request_id uuid NOT NULL,
    detail jsonb,
    count integer DEFAULT 1 NOT NULL,
    CONSTRAINT security_event_client_ip_present CHECK (((client_ip IS NOT NULL) OR (kind = 'audit_chain_broken'::text))),
    CONSTRAINT security_event_count_aggregated CHECK (((count = 1) OR (kind = ANY (ARRAY['login_failed'::text, 'login_throttled'::text, 'csrf_rejected'::text])))),
    CONSTRAINT security_event_count_positive CHECK ((count >= 1)),
    CONSTRAINT security_event_kind_value CHECK ((kind = ANY (ARRAY['login_failed'::text, 'login_rejected_inactive'::text, 'login_throttled'::text, 'csrf_rejected'::text, 'forbidden'::text, 'unlock'::text, 'audit_chain_broken'::text]))),
    CONSTRAINT security_event_subject CHECK (((login_hmac IS NOT NULL) OR (user_id IS NOT NULL) OR (kind = ANY (ARRAY['csrf_rejected'::text, 'audit_chain_broken'::text])))),
    CONSTRAINT security_event_user_agent_length CHECK (((user_agent IS NULL) OR (length(user_agent) <= 256)))
);


ALTER TABLE public.security_event OWNER TO audit_owner;

--
-- Name: security_event_id_seq; Type: SEQUENCE; Schema: public; Owner: audit_owner
--

ALTER TABLE public.security_event ALTER COLUMN id ADD GENERATED ALWAYS AS IDENTITY (
    SEQUENCE NAME public.security_event_id_seq
    START WITH 1
    INCREMENT BY 1
    NO MINVALUE
    NO MAXVALUE
    CACHE 1
);


--
-- Name: session; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.session (
    id text NOT NULL,
    user_id uuid NOT NULL,
    epoch bigint NOT NULL,
    csrf_token text NOT NULL,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    last_seen_at timestamp with time zone DEFAULT now() NOT NULL,
    expires_at timestamp with time zone NOT NULL,
    CONSTRAINT session_csrf_format CHECK ((csrf_token ~ '^[0-9a-f]{64}$'::text)),
    CONSTRAINT session_id_format CHECK ((id ~ '^[0-9a-f]{64}$'::text))
);


ALTER TABLE public.session OWNER TO recipe_migrator;

--
-- Name: unit; Type: TABLE; Schema: public; Owner: recipe_migrator
--

CREATE TABLE public.unit (
    id uuid DEFAULT gen_random_uuid() NOT NULL,
    machine_id uuid NOT NULL,
    path text NOT NULL,
    parent_path text GENERATED ALWAYS AS ("substring"(path, '^(.*)\.[0-9]+$'::text)) STORED,
    depth smallint GENERATED ALWAYS AS (array_length(string_to_array(path, '.'::text), 1)) STORED,
    name text NOT NULL,
    archived_at timestamp with time zone,
    created_at timestamp with time zone DEFAULT now() NOT NULL,
    updated_at timestamp with time zone DEFAULT now() NOT NULL,
    CONSTRAINT unit_depth_range CHECK (((depth >= 1) AND (depth <= 5))),
    CONSTRAINT unit_name_length CHECK (((length(name) >= 1) AND (length(name) <= 200))),
    CONSTRAINT unit_path_format CHECK ((path ~ '^U([1-9]|[1-5][0-9]|6[0-4])(\.([1-9]|[1-5][0-9]|6[0-4])){0,4}$'::text))
);


ALTER TABLE public.unit OWNER TO recipe_migrator;

--
-- Data for Name: _sqlx_migrations; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public._sqlx_migrations (version, description, installed_on, success, checksum, execution_time) FROM stdin;
1	schema	2026-11-20 12:02:12.965608+00	t	\\xa8074a1ca24050fa1da4c362335412e0463259ff74591e91e4ccc9bce444e9123df5fb8ffdbc12b4017e32c4aa27002b	161214287
2	triggers	2026-11-20 12:02:13.128282+00	t	\\xf7c4a521f4417fcaa9c106802107b768f121415e5c455aee1391ff77f4d6df7b0caf7a1d242737b3213206f3185afa5d	4384147
3	grants	2026-11-20 12:02:13.133956+00	t	\\x8bef38cd7a68befb34c0b5d5297338bb3ebb6eadeb80120540b766348db5a57078a393e391bf25c47e2cf52abc21025e	5177810
4	seed plc types	2026-11-20 12:02:13.140132+00	t	\\xfdf29fe2bc0474e45ab7f01d6f27c57dcf24fd0f183c52b54cedec869aee7e9bc54afbdf8665102c22c42760e1097bea	3337793
5	security event guard	2026-11-20 12:02:13.144228+00	t	\\x9fa9cabea400c4448d0a03e51d866a43788d41583d207e31ce95225a8614e5779e60e3adc6ac7bcfcdc7e97bf0dc0a6e	1910185
6	operations	2026-11-20 12:02:13.14715+00	t	\\xa2714ec7de0547f74f522e35042c6836e9fc41951f9eac20b3ffbed19f34c88bfc8ba1d395731c20c3a9e3340418cf1e	3989385
7	audit definer operators	2026-11-20 12:02:13.15258+00	t	\\x49cea3ce54ce09314de441ed02cb7451bb58627035341fb5f4bac2bb7dd93cb981abfdeb50b98503309bd98f013f001e	2395000
8	audit log insert columns	2026-11-20 12:02:13.155963+00	t	\\xbc78aee1b04376cd174b9af831832ae3e29635406b019965224525d03d4d9adf3885dd7866041978a60ea1a7da648c98	1677036
9	export client role	2026-11-20 12:02:13.158841+00	t	\\xdfe3418f00ec1b064d135f1a0d56749bbacbbb1f74b9ee9c624f541de37fe9ce3f53e135d0ce752fe0e3213f9b034147	2415841
10	validate export client checks	2026-11-20 12:02:13.162551+00	t	\\xfd20ebf3fc3240a3d1add0c718c7e0802a95f3145270cb82cda1d952fa7f34888c40c4c27dcb6832c0eed47ab6655c6a	2086058
11	plc type rules	2026-11-20 12:02:13.165667+00	t	\\xb41bc227cb81d2085c2c11439274650f828efcff805c7d7b3f34e598d44378521e2dc58c5e040f0eaf172c2e0417f854	5522835
12	plc connection	2026-11-20 12:02:13.172263+00	t	\\x68591a72f3649bfce05f49cadc9168afc85079dfdd60c7387e14689ff59e661407848f49a8a0e85ae4470e87e6045d05	6074849
13	machine db number	2026-11-20 12:02:13.179404+00	t	\\xeaee47777d929a3bb22d06866376a44d5862de9a0c81feee7fa8dc9921f366061404a65912d9532be6e264ce59c8bab7	3390578
14	machine db number copy	2026-11-20 12:02:13.184027+00	t	\\xb55528afa13ee2af130501c45ad8b6887c8bf0b2bbb05e02560e0fa20c1c19e6d0902cab307637e14e789b9f89a02c5e	2001603
15	machine db number validate	2026-11-20 12:02:13.187102+00	t	\\xd86bc489b74a5691046516a2828cc666bd0e9ce8abfae2b405e61b78b7680b3b282ead1e16b2146d6ecb3bf00548bbf7	2236093
16	recipe version insert	2026-11-20 12:02:13.190186+00	t	\\x891fd00812746f078ca445aaf0e18dc90c2cebb4c214e20a8c58672bfeb2459de50c17b9387b82c31ea25d13442ed333	1681424
\.


--
-- Data for Name: app_setting; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.app_setting (id, key, value, updated_by, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: app_user; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.app_user (id, login, password_hash, role, is_active, session_epoch, must_change_password, deactivated_at, created_at, updated_at) FROM stdin;
8b06d67b-b1c2-41a6-870c-941a85183736	user1	$argon2id$fixture	administrator	t	0	f	\N	2026-11-20 12:02:13.332264+00	2026-11-20 12:02:13.332264+00
d9dff170-ad15-42b1-a15f-74b85178c69b	engineer	$argon2id$fake$Seed-password-2024	process_engineer	t	0	f	\N	2026-11-20 12:02:13.389761+00	2026-11-20 12:02:13.389761+00
\.


--
-- Data for Name: audit_chain_head; Type: TABLE DATA; Schema: public; Owner: audit_owner
--

COPY public.audit_chain_head (singleton, last_seq, last_hash) FROM stdin;
t	42	\\xac0b284f999c73b66ea093bd66ceac6ef7d8bbd9c42163c5f14d7a8555c542bc
\.


--
-- Data for Name: audit_log; Type: TABLE DATA; Schema: public; Owner: audit_owner
--

COPY public.audit_log (id, chain_seq, occurred_at, actor_user_id, action, entity_type, entity_id, aggregate_type, aggregate_id, before_state, after_state, request_id, client_ip, session_ref, prev_hash, entry_hash) FROM stdin;
1	1	2026-11-20 12:02:13.424907+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	\N	{"id": "717120e4-b1b0-49d9-8688-4679a3d6d924", "name": "SR-IEC-WSTRING-TC", "created_at": "2026-11-20T12:02:13.424907+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "2ec96487-4430-40f9-bef8-f98034463bf5", "updated_at": "2026-11-20T12:02:13.424907+00:00", "plc_platform": "TwinCat3", "reference_number": "SR-IEC-WSTRING-TC"}	0b86c8c7-66d9-4e06-8de2-ade1741262b2	\N	\N	\N	\\x1e679dfb21a746fb1cca60b8309797ae2a1f667c7bd75a55a18a6f9c879e32a4
2	2	2026-11-20 12:02:13.424907+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	2de45b46-8d15-4943-a011-42f89a1d90eb	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	\N	{"id": "2de45b46-8d15-4943-a011-42f89a1d90eb", "number": 1, "status": "draft", "recipe_id": "717120e4-b1b0-49d9-8688-4679a3d6d924", "created_at": "2026-11-20T12:02:13.424907+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.424907+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "TwinCat3", "content_sha256": null, "based_on_version_id": null}	0b86c8c7-66d9-4e06-8de2-ade1741262b2	\N	\N	\\x1e679dfb21a746fb1cca60b8309797ae2a1f667c7bd75a55a18a6f9c879e32a4	\\x7ef08a39601f43dfc275f85388d39306dccb1db28936e6fe03e641f367ab43d3
3	3	2026-11-20 12:02:13.445046+00	d9dff170-ad15-42b1-a15f-74b85178c69b	DerivedTypeCreated	plc_type	fd56f366-7512-4f11-8285-7cecda5154ff	machine	2ec96487-4430-40f9-bef8-f98034463bf5	\N	{"id": "fd56f366-7512-4f11-8285-7cecda5154ff", "kind": "primitive", "name": "WSTRING", "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["TwinCat3"], "created_at": "2026-11-20T12:02:13.445046+00:00", "leaf_class": "string", "machine_id": "2ec96487-4430-40f9-bef8-f98034463bf5", "size_bytes": 24, "string_length": 10, "element_type_id": null, "supersedes_type_id": null}	0b1afc39-7113-488a-a4ec-e7b550695fda	\N	\N	\\x7ef08a39601f43dfc275f85388d39306dccb1db28936e6fe03e641f367ab43d3	\\x42155fcef29afdaa86ef26f2fd9c14a4c3b730a59d81ded07d69b116783a9a37
4	4	2026-11-20 12:02:13.445046+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	f1b88a4e-f083-4696-94d5-4a3c71a405eb	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	\N	{"id": "f1b88a4e-f083-4696-94d5-4a3c71a405eb", "tag": "VALUE", "value": "A😀", "limits": [{"leaf_path": "", "max_value": null, "min_value": null, "created_at": "2026-11-20T12:02:13.445046+00:00", "max_length": 10, "parameter_id": "f1b88a4e-f083-4696-94d5-4a3c71a405eb", "recipe_version_id": "2de45b46-8d15-4943-a011-42f89a1d90eb"}], "name_en": "Value", "ordinal": 1, "unit_id": "d0d8f101-58b6-4b57-9ede-acbcce9bc5ac", "created_at": "2026-11-20T12:02:13.445046+00:00", "s7_address": null, "plc_type_id": "fd56f366-7512-4f11-8285-7cecda5154ff", "plc_platform": "TwinCat3", "translations": [], "parameter_key": "366f8fd3-3ee9-4b04-8f1f-5ffb923b0017", "description_en": "Value of the matrix row", "recipe_version_id": "2de45b46-8d15-4943-a011-42f89a1d90eb"}	0b1afc39-7113-488a-a4ec-e7b550695fda	\N	\N	\\x42155fcef29afdaa86ef26f2fd9c14a4c3b730a59d81ded07d69b116783a9a37	\\xd25989a0b94c36d67178d6f5bfd71bed7d8677d9d031f7bf3327c2c827de414c
5	5	2026-11-20 12:02:13.473554+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	2de45b46-8d15-4943-a011-42f89a1d90eb	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	{"id": "2de45b46-8d15-4943-a011-42f89a1d90eb", "number": 1, "status": "draft", "recipe_id": "717120e4-b1b0-49d9-8688-4679a3d6d924", "created_at": "2026-11-20T12:02:13.424907+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.445046+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "TwinCat3", "content_sha256": null, "based_on_version_id": null}	{"id": "2de45b46-8d15-4943-a011-42f89a1d90eb", "number": 1, "status": "approved", "recipe_id": "717120e4-b1b0-49d9-8688-4679a3d6d924", "created_at": "2026-11-20T12:02:13.424907+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.473554+00:00", "approved_at": "2026-11-20T12:02:13.473554+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "TwinCat3", "allow_outdated": false, "content_sha256": "b6d9a8f1810cf83c46a84697e88b8c06ae2a8d1f29f1d910d2a93e1a67600862", "based_on_version_id": null}	82b9c78d-119b-41aa-8fb0-751aef39bbea	\N	\N	\\xd25989a0b94c36d67178d6f5bfd71bed7d8677d9d031f7bf3327c2c827de414c	\\x1dc47a1c9e968f98758108bd1fdbf15eaba403f57a6558c3eeea10fd4fd2346b
6	6	2026-11-20 12:02:13.483798+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	\N	{"id": "9696a65e-d566-4376-b3a4-f01f0aaa6fb8", "name": "SR-IEC-WSTRING-CS", "created_at": "2026-11-20T12:02:13.483798+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "6d612460-a630-443f-9277-007c32a239c3", "updated_at": "2026-11-20T12:02:13.483798+00:00", "plc_platform": "CodesysIec", "reference_number": "SR-IEC-WSTRING-CS"}	909e5445-fd8c-481e-9112-9d70544f80a0	\N	\N	\\x1dc47a1c9e968f98758108bd1fdbf15eaba403f57a6558c3eeea10fd4fd2346b	\\x076749d6beef44a9d6f981472c5e78997e539e93a889f3b9414cb1b03c0b9436
7	7	2026-11-20 12:02:13.483798+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	29e00b9b-a944-4f81-b463-d49b88503d58	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	\N	{"id": "29e00b9b-a944-4f81-b463-d49b88503d58", "number": 1, "status": "draft", "recipe_id": "9696a65e-d566-4376-b3a4-f01f0aaa6fb8", "created_at": "2026-11-20T12:02:13.483798+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.483798+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	909e5445-fd8c-481e-9112-9d70544f80a0	\N	\N	\\x076749d6beef44a9d6f981472c5e78997e539e93a889f3b9414cb1b03c0b9436	\\x6e53590d861c4238c0e250483220098f02dcd5358680eeda0a45344f73f94b0d
8	8	2026-11-20 12:02:13.49072+00	d9dff170-ad15-42b1-a15f-74b85178c69b	DerivedTypeCreated	plc_type	f5092aa6-88a5-4bdc-9b87-99911d260d7a	machine	6d612460-a630-443f-9277-007c32a239c3	\N	{"id": "f5092aa6-88a5-4bdc-9b87-99911d260d7a", "kind": "primitive", "name": "WSTRING", "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["CodesysIec"], "created_at": "2026-11-20T12:02:13.49072+00:00", "leaf_class": "string", "machine_id": "6d612460-a630-443f-9277-007c32a239c3", "size_bytes": 24, "string_length": 10, "element_type_id": null, "supersedes_type_id": null}	e324a40c-b938-4459-a6d7-7b18b33dbbe7	\N	\N	\\x6e53590d861c4238c0e250483220098f02dcd5358680eeda0a45344f73f94b0d	\\xcd0a196922c11c3fddbff7bf791817014b90e804de48f5011a8a3a6c20eb5807
9	9	2026-11-20 12:02:13.49072+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	22110336-2f33-46dd-a4a4-7101a8f8cf76	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	\N	{"id": "22110336-2f33-46dd-a4a4-7101a8f8cf76", "tag": "VALUE", "value": "Z𝄞", "limits": [{"leaf_path": "", "max_value": null, "min_value": null, "created_at": "2026-11-20T12:02:13.49072+00:00", "max_length": 10, "parameter_id": "22110336-2f33-46dd-a4a4-7101a8f8cf76", "recipe_version_id": "29e00b9b-a944-4f81-b463-d49b88503d58"}], "name_en": "Value", "ordinal": 1, "unit_id": "66c39e6c-4caf-460c-9d53-aaff74efbe56", "created_at": "2026-11-20T12:02:13.49072+00:00", "s7_address": null, "plc_type_id": "f5092aa6-88a5-4bdc-9b87-99911d260d7a", "plc_platform": "CodesysIec", "translations": [], "parameter_key": "4aa5d1ea-6a10-48b1-8567-81ca2a1faae4", "description_en": "Value of the matrix row", "recipe_version_id": "29e00b9b-a944-4f81-b463-d49b88503d58"}	e324a40c-b938-4459-a6d7-7b18b33dbbe7	\N	\N	\\xcd0a196922c11c3fddbff7bf791817014b90e804de48f5011a8a3a6c20eb5807	\\x2363edea7a43decb73c2cce35bc4e009df4b6587df3e5472164bb2dbfd392078
31	31	2026-11-20 12:02:13.660119+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	af56350c-10e0-41e3-b422-15c45b313de4	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	\N	{"version_id": "af56350c-10e0-41e3-b422-15c45b313de4", "content_sha256": "185954e961df02479b108120c474dedd1739011868f321282de22b95ae49cc2a", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x607c5005e0e2ead10ea9549b5de7fd241a2e86fe82467e1aa23b621a121323d1	\\x0c960976215b2c2f34bf60d520c0b73d9a4048ddcd13ce1834109666ae752bb2
10	10	2026-11-20 12:02:13.505533+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	29e00b9b-a944-4f81-b463-d49b88503d58	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	{"id": "29e00b9b-a944-4f81-b463-d49b88503d58", "number": 1, "status": "draft", "recipe_id": "9696a65e-d566-4376-b3a4-f01f0aaa6fb8", "created_at": "2026-11-20T12:02:13.483798+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.49072+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	{"id": "29e00b9b-a944-4f81-b463-d49b88503d58", "number": 1, "status": "approved", "recipe_id": "9696a65e-d566-4376-b3a4-f01f0aaa6fb8", "created_at": "2026-11-20T12:02:13.483798+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.505533+00:00", "approved_at": "2026-11-20T12:02:13.505533+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "CodesysIec", "allow_outdated": false, "content_sha256": "98ca14c97da1093fef2c961a053155f4c8d3de9861332d743e451d3255bd68f5", "based_on_version_id": null}	115306ff-a802-4a17-986e-763219fa354c	\N	\N	\\x2363edea7a43decb73c2cce35bc4e009df4b6587df3e5472164bb2dbfd392078	\\x5c16b65e6edd41cf57ed60fb488acd4141102062911b9e15ce4b710ea44bd346
11	11	2026-11-20 12:02:13.514341+00	8b06d67b-b1c2-41a6-870c-941a85183736	UdtDefined	plc_type	ce4d1eae-73c4-42ec-b8c7-b02bcb73befd	machine	db087d62-7aa2-4b03-99a0-0294faa178aa	\N	{"id": "ce4d1eae-73c4-42ec-b8c7-b02bcb73befd", "kind": "struct", "name": "VERSION", "members": [{"ordinal": 1, "created_at": "2026-11-20T12:02:13.514341+00:00", "member_name": "major", "member_type_id": "5eed0000-0000-4000-8000-000000000008", "struct_type_id": "ce4d1eae-73c4-42ec-b8c7-b02bcb73befd"}, {"ordinal": 2, "created_at": "2026-11-20T12:02:13.514341+00:00", "member_name": "minor", "member_type_id": "5eed0000-0000-4000-8000-000000000008", "struct_type_id": "ce4d1eae-73c4-42ec-b8c7-b02bcb73befd"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["TwinCat3"], "created_at": "2026-11-20T12:02:13.514341+00:00", "leaf_class": null, "machine_id": "db087d62-7aa2-4b03-99a0-0294faa178aa", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	f75652bc-8430-41e9-9363-5d349c183225	\N	\N	\\x5c16b65e6edd41cf57ed60fb488acd4141102062911b9e15ce4b710ea44bd346	\\x4ca34d18c75e59e820a93f25a76b0e147358bda5b28cbf77e099b5ab436f49bf
12	12	2026-11-20 12:02:13.52297+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	\N	{"id": "e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35", "name": "SR-IEC-RESERVED-STRUCT", "created_at": "2026-11-20T12:02:13.52297+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "db087d62-7aa2-4b03-99a0-0294faa178aa", "updated_at": "2026-11-20T12:02:13.52297+00:00", "plc_platform": "TwinCat3", "reference_number": "SR-IEC-RESERVED-STRUCT"}	b191c0bd-a2f6-4a95-ae2f-064fddcc2cfc	\N	\N	\\x4ca34d18c75e59e820a93f25a76b0e147358bda5b28cbf77e099b5ab436f49bf	\\x662ae7afe95e7c5519ebec8295243d6edb679517a873965a5400b4287c15e538
13	13	2026-11-20 12:02:13.52297+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	4047715c-70d9-451d-8ab2-966f7f1dc497	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	\N	{"id": "4047715c-70d9-451d-8ab2-966f7f1dc497", "number": 1, "status": "draft", "recipe_id": "e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35", "created_at": "2026-11-20T12:02:13.52297+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.52297+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "TwinCat3", "content_sha256": null, "based_on_version_id": null}	b191c0bd-a2f6-4a95-ae2f-064fddcc2cfc	\N	\N	\\x662ae7afe95e7c5519ebec8295243d6edb679517a873965a5400b4287c15e538	\\xead5009d72fd36e3a3262f943d988d23e9eadf796d36ac2e93167e6398538e29
15	15	2026-11-20 12:02:13.548983+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	4047715c-70d9-451d-8ab2-966f7f1dc497	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	{"id": "4047715c-70d9-451d-8ab2-966f7f1dc497", "number": 1, "status": "draft", "recipe_id": "e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35", "created_at": "2026-11-20T12:02:13.52297+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.529622+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "TwinCat3", "content_sha256": null, "based_on_version_id": null}	{"id": "4047715c-70d9-451d-8ab2-966f7f1dc497", "number": 1, "status": "approved", "recipe_id": "e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35", "created_at": "2026-11-20T12:02:13.52297+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.548983+00:00", "approved_at": "2026-11-20T12:02:13.548983+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "TwinCat3", "allow_outdated": false, "content_sha256": "279516853a2b79709954f99dcb149cc020bb4b34186e2768293333390ba6f556", "based_on_version_id": null}	ffe5d0d8-4d0d-4cc9-a025-c6a4ea0004da	\N	\N	\\x1c42f4fc8841b2c4b3fe6882a02af6ba1f1fbf3fe8676e23f1327e11406f6766	\\x1eac5290773ef8f04839122824554a7f0121add2c7c540f7eb1c82199c30adc7
16	16	2026-11-20 12:02:13.558911+00	8b06d67b-b1c2-41a6-870c-941a85183736	UdtDefined	plc_type	b5845c35-8da8-477a-845f-90f2189b1950	machine	c47efd5d-1f71-4ec1-8520-be399b89b134	\N	{"id": "b5845c35-8da8-477a-845f-90f2189b1950", "kind": "struct", "name": "ST_Coil", "members": [{"ordinal": 1, "created_at": "2026-11-20T12:02:13.558911+00:00", "member_name": "END_IF", "member_type_id": "5eed0000-0000-4000-8000-000000000001", "struct_type_id": "b5845c35-8da8-477a-845f-90f2189b1950"}, {"ordinal": 2, "created_at": "2026-11-20T12:02:13.558911+00:00", "member_name": "S", "member_type_id": "5eed0000-0000-4000-8000-000000000001", "struct_type_id": "b5845c35-8da8-477a-845f-90f2189b1950"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["CodesysIec"], "created_at": "2026-11-20T12:02:13.558911+00:00", "leaf_class": null, "machine_id": "c47efd5d-1f71-4ec1-8520-be399b89b134", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	0837741f-0e23-46d3-a903-9b37c1f8ff23	\N	\N	\\x1eac5290773ef8f04839122824554a7f0121add2c7c540f7eb1c82199c30adc7	\\xa01c4494cdbd9a7ffe6ee5d843757f284b2eacfead55e060aae48c2b8a8a8d35
17	17	2026-11-20 12:02:13.565255+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	\N	{"id": "9ec15baf-8247-42e4-9f83-b9ca47b7e436", "name": "SR-IEC-KEYWORD-MEMBER", "created_at": "2026-11-20T12:02:13.565255+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "c47efd5d-1f71-4ec1-8520-be399b89b134", "updated_at": "2026-11-20T12:02:13.565255+00:00", "plc_platform": "CodesysIec", "reference_number": "SR-IEC-KEYWORD-MEMBER"}	d8a5f860-12c1-48f5-8dd7-43d2c2134f18	\N	\N	\\xa01c4494cdbd9a7ffe6ee5d843757f284b2eacfead55e060aae48c2b8a8a8d35	\\x0b7cc673d5b2a8bb6ce00345bb7c42d0f6367b811a6ad34fd01fbe2d7f301b88
18	18	2026-11-20 12:02:13.565255+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	af56350c-10e0-41e3-b422-15c45b313de4	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	\N	{"id": "af56350c-10e0-41e3-b422-15c45b313de4", "number": 1, "status": "draft", "recipe_id": "9ec15baf-8247-42e4-9f83-b9ca47b7e436", "created_at": "2026-11-20T12:02:13.565255+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.565255+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	d8a5f860-12c1-48f5-8dd7-43d2c2134f18	\N	\N	\\x0b7cc673d5b2a8bb6ce00345bb7c42d0f6367b811a6ad34fd01fbe2d7f301b88	\\x3e14558c4901c2f9937b84af85a70f4df0537b98b3c8f5f42731e73d6cf0d93a
14	14	2026-11-20 12:02:13.529622+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	1f7f3491-e46c-4775-b0b4-fa574163ae55	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	\N	{"id": "1f7f3491-e46c-4775-b0b4-fa574163ae55", "tag": "VALUE", "value": {"major": 1, "minor": 2}, "limits": [{"leaf_path": ".major", "max_value": 100, "min_value": 0, "created_at": "2026-11-20T12:02:13.529622+00:00", "max_length": null, "parameter_id": "1f7f3491-e46c-4775-b0b4-fa574163ae55", "recipe_version_id": "4047715c-70d9-451d-8ab2-966f7f1dc497"}, {"leaf_path": ".minor", "max_value": 100, "min_value": 0, "created_at": "2026-11-20T12:02:13.529622+00:00", "max_length": null, "parameter_id": "1f7f3491-e46c-4775-b0b4-fa574163ae55", "recipe_version_id": "4047715c-70d9-451d-8ab2-966f7f1dc497"}], "name_en": "Value", "ordinal": 1, "unit_id": "e0f35cca-6f35-48bf-8487-4a17a7447cd7", "created_at": "2026-11-20T12:02:13.529622+00:00", "s7_address": null, "plc_type_id": "ce4d1eae-73c4-42ec-b8c7-b02bcb73befd", "plc_platform": "TwinCat3", "translations": [], "parameter_key": "11e0cd37-83f3-4c4e-ab9b-1ea8885b478c", "description_en": "Value of the matrix row", "recipe_version_id": "4047715c-70d9-451d-8ab2-966f7f1dc497"}	8c8b0dc0-c98c-4aba-bccd-46d627970e05	\N	\N	\\xead5009d72fd36e3a3262f943d988d23e9eadf796d36ac2e93167e6398538e29	\\x1c42f4fc8841b2c4b3fe6882a02af6ba1f1fbf3fe8676e23f1327e11406f6766
19	19	2026-11-20 12:02:13.5723+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	8dcb09e0-0f10-415d-a0a0-91ab585b7534	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	\N	{"id": "8dcb09e0-0f10-415d-a0a0-91ab585b7534", "tag": "VALUE", "value": {"S": false, "END_IF": true}, "limits": [], "name_en": "Value", "ordinal": 1, "unit_id": "8f542085-69dc-4047-acdf-dd011b2fd579", "created_at": "2026-11-20T12:02:13.5723+00:00", "s7_address": null, "plc_type_id": "b5845c35-8da8-477a-845f-90f2189b1950", "plc_platform": "CodesysIec", "translations": [], "parameter_key": "4f55f0f1-8cb9-463a-9c2b-20b0d8d36772", "description_en": "Value of the matrix row", "recipe_version_id": "af56350c-10e0-41e3-b422-15c45b313de4"}	68af9173-5543-4310-a23c-e20cdd060610	\N	\N	\\x3e14558c4901c2f9937b84af85a70f4df0537b98b3c8f5f42731e73d6cf0d93a	\\x819c00a8378f7b3d8cef2a8ae0b0741fd7282d3bcfcb27a849a422adc2c40824
24	24	2026-11-20 12:02:13.61031+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	527b46a4-493e-4244-9053-338bc33998c0	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	\N	{"id": "527b46a4-493e-4244-9053-338bc33998c0", "tag": "VALUE", "value": {"value": 3}, "limits": [{"leaf_path": ".value", "max_value": 100, "min_value": 0, "created_at": "2026-11-20T12:02:13.61031+00:00", "max_length": null, "parameter_id": "527b46a4-493e-4244-9053-338bc33998c0", "recipe_version_id": "35a29460-305b-4353-97af-ab1bc85450b8"}], "name_en": "Value", "ordinal": 1, "unit_id": "261deddc-3566-4a97-bc1d-3323937dad5f", "created_at": "2026-11-20T12:02:13.61031+00:00", "s7_address": null, "plc_type_id": "444fa16f-c385-47c5-b18a-afc098a6a9f4", "plc_platform": "CodesysIec", "translations": [], "parameter_key": "fd34d17d-a254-43dd-bcab-b7c11c598a63", "description_en": "Value of the matrix row", "recipe_version_id": "35a29460-305b-4353-97af-ab1bc85450b8"}	33187fa2-e2de-4617-a898-e6270cf49d6b	\N	\N	\\x607fbf3473b43c8036965c6bc6c753e7b88ea099cb8260dfe127fae47703f605	\\x38ff78b19ac0f71a10784efb9f1319e787e616ad2d77573b1b59ca83f0027a27
28	28	2026-11-20 12:02:13.638465+00	d9dff170-ad15-42b1-a15f-74b85178c69b	DerivedTypeCreated	plc_type	80f884f5-8adc-4c6e-be02-915d6e203bb7	machine	00cd26a4-4172-4fee-ba63-104be5cca762	\N	{"id": "80f884f5-8adc-4c6e-be02-915d6e203bb7", "kind": "primitive", "name": "STRING", "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["CodesysIec"], "created_at": "2026-11-20T12:02:13.638465+00:00", "leaf_class": "string", "machine_id": "00cd26a4-4172-4fee-ba63-104be5cca762", "size_bytes": 6, "string_length": 4, "element_type_id": null, "supersedes_type_id": null}	aca22560-d811-4347-8e7f-37604eaf2cdf	\N	\N	\\x0b5b60315c4aecacf82adbfae8ec2c6d48e8b9e73a6c5e36de507471baf4adf9	\\x094d6870504ce4d114610acce149d7e08a7800ba3a23ae15ce44e7f27d972a13
29	29	2026-11-20 12:02:13.638465+00	d9dff170-ad15-42b1-a15f-74b85178c69b	ParameterAdded	parameter	fe6e4900-42da-4323-a88d-fa6e3d792e44	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	\N	{"id": "fe6e4900-42da-4323-a88d-fa6e3d792e44", "tag": "VALUE", "value": "é", "limits": [{"leaf_path": "", "max_value": null, "min_value": null, "created_at": "2026-11-20T12:02:13.638465+00:00", "max_length": 4, "parameter_id": "fe6e4900-42da-4323-a88d-fa6e3d792e44", "recipe_version_id": "6826910b-c423-40b3-bfbd-df605f528968"}], "name_en": "Value", "ordinal": 1, "unit_id": "7fda2c8f-4a14-4211-8dd8-f175e4b8462b", "created_at": "2026-11-20T12:02:13.638465+00:00", "s7_address": null, "plc_type_id": "80f884f5-8adc-4c6e-be02-915d6e203bb7", "plc_platform": "CodesysIec", "translations": [], "parameter_key": "ce416b86-ee76-4ef0-b6d7-14e926d1a20f", "description_en": "Value of the matrix row", "recipe_version_id": "6826910b-c423-40b3-bfbd-df605f528968"}	aca22560-d811-4347-8e7f-37604eaf2cdf	\N	\N	\\x094d6870504ce4d114610acce149d7e08a7800ba3a23ae15ce44e7f27d972a13	\\xb44e60809d47f19b944defc038d621939ddeab2bc3a57aec605ed88050515631
20	20	2026-11-20 12:02:13.588004+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	af56350c-10e0-41e3-b422-15c45b313de4	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	{"id": "af56350c-10e0-41e3-b422-15c45b313de4", "number": 1, "status": "draft", "recipe_id": "9ec15baf-8247-42e4-9f83-b9ca47b7e436", "created_at": "2026-11-20T12:02:13.565255+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.5723+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	{"id": "af56350c-10e0-41e3-b422-15c45b313de4", "number": 1, "status": "approved", "recipe_id": "9ec15baf-8247-42e4-9f83-b9ca47b7e436", "created_at": "2026-11-20T12:02:13.565255+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.588004+00:00", "approved_at": "2026-11-20T12:02:13.588004+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "CodesysIec", "allow_outdated": false, "content_sha256": "185954e961df02479b108120c474dedd1739011868f321282de22b95ae49cc2a", "based_on_version_id": null}	a9317273-1cb1-414d-bd3b-e451fd1c81da	\N	\N	\\x819c00a8378f7b3d8cef2a8ae0b0741fd7282d3bcfcb27a849a422adc2c40824	\\xbde71c1da13ec248f70b31e0ad0188caa10358dffe55249752df445f91bb12f0
21	21	2026-11-20 12:02:13.596792+00	8b06d67b-b1c2-41a6-870c-941a85183736	UdtDefined	plc_type	444fa16f-c385-47c5-b18a-afc098a6a9f4	machine	c76b49f9-f028-4d00-be95-6b5bb89fe526	\N	{"id": "444fa16f-c385-47c5-b18a-afc098a6a9f4", "kind": "struct", "name": "ST_One", "members": [{"ordinal": 1, "created_at": "2026-11-20T12:02:13.596792+00:00", "member_name": "value", "member_type_id": "5eed0000-0000-4000-8000-000000000008", "struct_type_id": "444fa16f-c385-47c5-b18a-afc098a6a9f4"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["CodesysIec"], "created_at": "2026-11-20T12:02:13.596792+00:00", "leaf_class": null, "machine_id": "c76b49f9-f028-4d00-be95-6b5bb89fe526", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	30534471-f125-43f1-b5cc-9ea9e4728f2f	\N	\N	\\xbde71c1da13ec248f70b31e0ad0188caa10358dffe55249752df445f91bb12f0	\\xbba0dbb7f6d8d315fc160a489dd1033159bf4ea629ead2ab85def9f0ad4586da
22	22	2026-11-20 12:02:13.603195+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	\N	{"id": "a58ace5b-3ed9-4f31-a601-f760cb525724", "name": "SR-IEC-ONE-MEMBER", "created_at": "2026-11-20T12:02:13.603195+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "c76b49f9-f028-4d00-be95-6b5bb89fe526", "updated_at": "2026-11-20T12:02:13.603195+00:00", "plc_platform": "CodesysIec", "reference_number": "SR-IEC-ONE-MEMBER"}	67c8951f-ffcf-43a4-ab4e-b423aa60726e	\N	\N	\\xbba0dbb7f6d8d315fc160a489dd1033159bf4ea629ead2ab85def9f0ad4586da	\\x04b258cca707d741a7a44b0ff5f3c2a79439acde354437a9cb7a34c2c9c7f031
23	23	2026-11-20 12:02:13.603195+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	35a29460-305b-4353-97af-ab1bc85450b8	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	\N	{"id": "35a29460-305b-4353-97af-ab1bc85450b8", "number": 1, "status": "draft", "recipe_id": "a58ace5b-3ed9-4f31-a601-f760cb525724", "created_at": "2026-11-20T12:02:13.603195+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.603195+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	67c8951f-ffcf-43a4-ab4e-b423aa60726e	\N	\N	\\x04b258cca707d741a7a44b0ff5f3c2a79439acde354437a9cb7a34c2c9c7f031	\\x607fbf3473b43c8036965c6bc6c753e7b88ea099cb8260dfe127fae47703f605
25	25	2026-11-20 12:02:13.624037+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	35a29460-305b-4353-97af-ab1bc85450b8	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	{"id": "35a29460-305b-4353-97af-ab1bc85450b8", "number": 1, "status": "draft", "recipe_id": "a58ace5b-3ed9-4f31-a601-f760cb525724", "created_at": "2026-11-20T12:02:13.603195+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.61031+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	{"id": "35a29460-305b-4353-97af-ab1bc85450b8", "number": 1, "status": "approved", "recipe_id": "a58ace5b-3ed9-4f31-a601-f760cb525724", "created_at": "2026-11-20T12:02:13.603195+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.624037+00:00", "approved_at": "2026-11-20T12:02:13.624037+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "CodesysIec", "allow_outdated": false, "content_sha256": "2facdcf5a391ce911375c5bc1468a19f217ff1956458fddae05d968d922ac1cd", "based_on_version_id": null}	e1d4512c-b077-4797-b897-b6be397049cf	\N	\N	\\x38ff78b19ac0f71a10784efb9f1319e787e616ad2d77573b1b59ca83f0027a27	\\xdbb3fbc9273c5805e74191d1b40711bc6dba5a778792f4ebcc423916ebf8397f
26	26	2026-11-20 12:02:13.631568+00	d9dff170-ad15-42b1-a15f-74b85178c69b	RecipeCreated	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	\N	{"id": "63db2f60-9e7b-45db-a526-6088a23b166f", "name": "SR-IEC-STRING-LATIN1", "created_at": "2026-11-20T12:02:13.631568+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "machine_id": "00cd26a4-4172-4fee-ba63-104be5cca762", "updated_at": "2026-11-20T12:02:13.631568+00:00", "plc_platform": "CodesysIec", "reference_number": "SR-IEC-STRING-LATIN1"}	558ceb80-33a3-4898-ab9d-59dfa2e84e3c	\N	\N	\\xdbb3fbc9273c5805e74191d1b40711bc6dba5a778792f4ebcc423916ebf8397f	\\x693f773a233815a1380ab21739443f0549f7539efe7cca4bbebca678912fe565
27	27	2026-11-20 12:02:13.631568+00	d9dff170-ad15-42b1-a15f-74b85178c69b	VersionCreated	recipe_version	6826910b-c423-40b3-bfbd-df605f528968	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	\N	{"id": "6826910b-c423-40b3-bfbd-df605f528968", "number": 1, "status": "draft", "recipe_id": "63db2f60-9e7b-45db-a526-6088a23b166f", "created_at": "2026-11-20T12:02:13.631568+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.631568+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	558ceb80-33a3-4898-ab9d-59dfa2e84e3c	\N	\N	\\x693f773a233815a1380ab21739443f0549f7539efe7cca4bbebca678912fe565	\\x0b5b60315c4aecacf82adbfae8ec2c6d48e8b9e73a6c5e36de507471baf4adf9
30	30	2026-11-20 12:02:13.655034+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionApproved	recipe_version	6826910b-c423-40b3-bfbd-df605f528968	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	{"id": "6826910b-c423-40b3-bfbd-df605f528968", "number": 1, "status": "draft", "recipe_id": "63db2f60-9e7b-45db-a526-6088a23b166f", "created_at": "2026-11-20T12:02:13.631568+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.638465+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "CodesysIec", "content_sha256": null, "based_on_version_id": null}	{"id": "6826910b-c423-40b3-bfbd-df605f528968", "number": 1, "status": "approved", "recipe_id": "63db2f60-9e7b-45db-a526-6088a23b166f", "created_at": "2026-11-20T12:02:13.631568+00:00", "created_by": "d9dff170-ad15-42b1-a15f-74b85178c69b", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-20T12:02:13.655034+00:00", "approved_at": "2026-11-20T12:02:13.655034+00:00", "approved_by": "8b06d67b-b1c2-41a6-870c-941a85183736", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "CodesysIec", "allow_outdated": false, "content_sha256": "ed72ebd29f5551d76020017dcb6a915cef4b861ff4606011c6768fcc6695d03f", "based_on_version_id": null}	273f7211-ad6d-4241-9bc8-ce9808b0ced8	\N	\N	\\xb44e60809d47f19b944defc038d621939ddeab2bc3a57aec605ed88050515631	\\x607c5005e0e2ead10ea9549b5de7fd241a2e86fe82467e1aa23b621a121323d1
32	32	2026-11-20 12:02:13.667116+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	af56350c-10e0-41e3-b422-15c45b313de4	recipe	9ec15baf-8247-42e4-9f83-b9ca47b7e436	\N	{"version_id": "af56350c-10e0-41e3-b422-15c45b313de4", "scope_sha256": "e995ef7064e3500d355319dd2fa5e36353b8079b7c1b22d864ebf48c28ba7b67", "content_sha256": "185954e961df02479b108120c474dedd1739011868f321282de22b95ae49cc2a", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x0c960976215b2c2f34bf60d520c0b73d9a4048ddcd13ce1834109666ae752bb2	\\x312db2f82569d96f70156f4750440ab823d754730b2789c821773e88dd525678
33	33	2026-11-20 12:02:13.673334+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	35a29460-305b-4353-97af-ab1bc85450b8	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	\N	{"version_id": "35a29460-305b-4353-97af-ab1bc85450b8", "content_sha256": "2facdcf5a391ce911375c5bc1468a19f217ff1956458fddae05d968d922ac1cd", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x312db2f82569d96f70156f4750440ab823d754730b2789c821773e88dd525678	\\x9a816225046c99f530dc26773f19498bee83cc586da1d61a4acefff4fa94d1ff
34	34	2026-11-20 12:02:13.679666+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	35a29460-305b-4353-97af-ab1bc85450b8	recipe	a58ace5b-3ed9-4f31-a601-f760cb525724	\N	{"version_id": "35a29460-305b-4353-97af-ab1bc85450b8", "scope_sha256": "cc15777a257516d7679cfe6961340d1d12f81dd1387fffce94e889a7c6e49f5e", "content_sha256": "2facdcf5a391ce911375c5bc1468a19f217ff1956458fddae05d968d922ac1cd", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x9a816225046c99f530dc26773f19498bee83cc586da1d61a4acefff4fa94d1ff	\\xa490cd70d1f5051e15359103064a5bd1d17279439335aa8648b4587181c683db
35	35	2026-11-20 12:02:13.686162+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	4047715c-70d9-451d-8ab2-966f7f1dc497	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	\N	{"version_id": "4047715c-70d9-451d-8ab2-966f7f1dc497", "content_sha256": "279516853a2b79709954f99dcb149cc020bb4b34186e2768293333390ba6f556", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\xa490cd70d1f5051e15359103064a5bd1d17279439335aa8648b4587181c683db	\\x39f3a102d589a953c928150f5d6efa33d7a1a1c4391df1aa9ced2f50ef5c53d9
36	36	2026-11-20 12:02:13.691594+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	4047715c-70d9-451d-8ab2-966f7f1dc497	recipe	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	\N	{"version_id": "4047715c-70d9-451d-8ab2-966f7f1dc497", "scope_sha256": "e80bc82caa45c250fdee35c994ac6dc110a64e38910618b74416a620a3af94d6", "content_sha256": "279516853a2b79709954f99dcb149cc020bb4b34186e2768293333390ba6f556", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x39f3a102d589a953c928150f5d6efa33d7a1a1c4391df1aa9ced2f50ef5c53d9	\\xf85f407d88988e0fc9309ba551581199bf4d92ab303ed1b6aea56c50b2117b45
37	37	2026-11-20 12:02:13.698526+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	6826910b-c423-40b3-bfbd-df605f528968	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	\N	{"version_id": "6826910b-c423-40b3-bfbd-df605f528968", "content_sha256": "ed72ebd29f5551d76020017dcb6a915cef4b861ff4606011c6768fcc6695d03f", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\xf85f407d88988e0fc9309ba551581199bf4d92ab303ed1b6aea56c50b2117b45	\\xf4cefbe3d04fe6e623bcb6559bf4d0ddeaf0ef530ee0092520ac58a37e1049dc
38	38	2026-11-20 12:02:13.704064+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	6826910b-c423-40b3-bfbd-df605f528968	recipe	63db2f60-9e7b-45db-a526-6088a23b166f	\N	{"version_id": "6826910b-c423-40b3-bfbd-df605f528968", "scope_sha256": "1554590b706476780470c49589c931e1f4ca2fa906ac5cb36eaa279ba8404324", "content_sha256": "ed72ebd29f5551d76020017dcb6a915cef4b861ff4606011c6768fcc6695d03f", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\xf4cefbe3d04fe6e623bcb6559bf4d0ddeaf0ef530ee0092520ac58a37e1049dc	\\x14b69ab23d5a534e23ed90cba747ecf94af8e4d589b12fa97ec96c8aecde1dfe
39	39	2026-11-20 12:02:13.709489+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	29e00b9b-a944-4f81-b463-d49b88503d58	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	\N	{"version_id": "29e00b9b-a944-4f81-b463-d49b88503d58", "content_sha256": "98ca14c97da1093fef2c961a053155f4c8d3de9861332d743e451d3255bd68f5", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x14b69ab23d5a534e23ed90cba747ecf94af8e4d589b12fa97ec96c8aecde1dfe	\\x1729d145101196fd9a25d64e6a2121ca898af92d58efdf2d07fb356eb69f0787
40	40	2026-11-20 12:02:13.714161+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	29e00b9b-a944-4f81-b463-d49b88503d58	recipe	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	\N	{"version_id": "29e00b9b-a944-4f81-b463-d49b88503d58", "scope_sha256": "24694bd9669164c0271d1015fe4057c02bcf175dfd1684dbd9e595112d1de4cb", "content_sha256": "98ca14c97da1093fef2c961a053155f4c8d3de9861332d743e451d3255bd68f5", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x1729d145101196fd9a25d64e6a2121ca898af92d58efdf2d07fb356eb69f0787	\\x85f97a10772b61cee8c5c22bf42b76efd276440f795ac7581656615f53fc179b
41	41	2026-11-20 12:02:13.71926+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	2de45b46-8d15-4943-a011-42f89a1d90eb	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	\N	{"version_id": "2de45b46-8d15-4943-a011-42f89a1d90eb", "content_sha256": "b6d9a8f1810cf83c46a84697e88b8c06ae2a8d1f29f1d910d2a93e1a67600862", "schema_version": "1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\x85f97a10772b61cee8c5c22bf42b76efd276440f795ac7581656615f53fc179b	\\xe0dcdd554f98fe1deb936e705405bfa7fe832adaf088e9898c0576b59eb873ad
42	42	2026-11-20 12:02:13.724413+00	8b06d67b-b1c2-41a6-870c-941a85183736	VersionExported	recipe_version	2de45b46-8d15-4943-a011-42f89a1d90eb	recipe	717120e4-b1b0-49d9-8688-4679a3d6d924	\N	{"version_id": "2de45b46-8d15-4943-a011-42f89a1d90eb", "scope_sha256": "8793948aa83f52fff6697228446c749f918d4075c6e128dcf9e97bdc2d1737db", "content_sha256": "b6d9a8f1810cf83c46a84697e88b8c06ae2a8d1f29f1d910d2a93e1a67600862", "schema_version": "2", "scope_unit_path": "U1"}	09056827-3a4e-459d-a83a-24665ee4203b	10.0.0.7	0123456789abcdef	\\xe0dcdd554f98fe1deb936e705405bfa7fe832adaf088e9898c0576b59eb873ad	\\xac0b284f999c73b66ea093bd66ceac6ef7d8bbd9c42163c5f14d7a8555c542bc
\.


--
-- Data for Name: draft_editor; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.draft_editor (recipe_version_id, user_id, first_edit_at) FROM stdin;
2de45b46-8d15-4943-a011-42f89a1d90eb	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.445046+00
29e00b9b-a944-4f81-b463-d49b88503d58	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.49072+00
4047715c-70d9-451d-8ab2-966f7f1dc497	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.529622+00
af56350c-10e0-41e3-b422-15c45b313de4	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.5723+00
35a29460-305b-4353-97af-ab1bc85450b8	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.61031+00
6826910b-c423-40b3-bfbd-df605f528968	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.638465+00
\.


--
-- Data for Name: login_attempt; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.login_attempt (id, login, client_ip, attempted_at, succeeded) FROM stdin;
\.


--
-- Data for Name: machine; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.machine (id, code, name, plc_platform, archived_at, created_at, updated_at, db_number) FROM stdin;
2ec96487-4430-40f9-bef8-f98034463bf5	M2	machine	TwinCat3	\N	2026-11-20 12:02:13.392094+00	2026-11-20 12:02:13.392094+00	\N
6d612460-a630-443f-9277-007c32a239c3	M3	machine	CodesysIec	\N	2026-11-20 12:02:13.479506+00	2026-11-20 12:02:13.479506+00	\N
db087d62-7aa2-4b03-99a0-0294faa178aa	M4	machine	TwinCat3	\N	2026-11-20 12:02:13.51082+00	2026-11-20 12:02:13.51082+00	\N
c47efd5d-1f71-4ec1-8520-be399b89b134	M5	machine	CodesysIec	\N	2026-11-20 12:02:13.554877+00	2026-11-20 12:02:13.554877+00	\N
c76b49f9-f028-4d00-be95-6b5bb89fe526	M6	machine	CodesysIec	\N	2026-11-20 12:02:13.593095+00	2026-11-20 12:02:13.593095+00	\N
00cd26a4-4172-4fee-ba63-104be5cca762	M7	machine	CodesysIec	\N	2026-11-20 12:02:13.628518+00	2026-11-20 12:02:13.628518+00	\N
\.


--
-- Data for Name: parameter; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.parameter (id, recipe_version_id, plc_platform, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, value, ordinal, created_at) FROM stdin;
f1b88a4e-f083-4696-94d5-4a3c71a405eb	2de45b46-8d15-4943-a011-42f89a1d90eb	TwinCat3	366f8fd3-3ee9-4b04-8f1f-5ffb923b0017	d0d8f101-58b6-4b57-9ede-acbcce9bc5ac	VALUE	Value	Value of the matrix row	fd56f366-7512-4f11-8285-7cecda5154ff	\N	"A😀"	1	2026-11-20 12:02:13.445046+00
22110336-2f33-46dd-a4a4-7101a8f8cf76	29e00b9b-a944-4f81-b463-d49b88503d58	CodesysIec	4aa5d1ea-6a10-48b1-8567-81ca2a1faae4	66c39e6c-4caf-460c-9d53-aaff74efbe56	VALUE	Value	Value of the matrix row	f5092aa6-88a5-4bdc-9b87-99911d260d7a	\N	"Z𝄞"	1	2026-11-20 12:02:13.49072+00
1f7f3491-e46c-4775-b0b4-fa574163ae55	4047715c-70d9-451d-8ab2-966f7f1dc497	TwinCat3	11e0cd37-83f3-4c4e-ab9b-1ea8885b478c	e0f35cca-6f35-48bf-8487-4a17a7447cd7	VALUE	Value	Value of the matrix row	ce4d1eae-73c4-42ec-b8c7-b02bcb73befd	\N	{"major": 1, "minor": 2}	1	2026-11-20 12:02:13.529622+00
8dcb09e0-0f10-415d-a0a0-91ab585b7534	af56350c-10e0-41e3-b422-15c45b313de4	CodesysIec	4f55f0f1-8cb9-463a-9c2b-20b0d8d36772	8f542085-69dc-4047-acdf-dd011b2fd579	VALUE	Value	Value of the matrix row	b5845c35-8da8-477a-845f-90f2189b1950	\N	{"S": false, "END_IF": true}	1	2026-11-20 12:02:13.5723+00
527b46a4-493e-4244-9053-338bc33998c0	35a29460-305b-4353-97af-ab1bc85450b8	CodesysIec	fd34d17d-a254-43dd-bcab-b7c11c598a63	261deddc-3566-4a97-bc1d-3323937dad5f	VALUE	Value	Value of the matrix row	444fa16f-c385-47c5-b18a-afc098a6a9f4	\N	{"value": 3}	1	2026-11-20 12:02:13.61031+00
fe6e4900-42da-4323-a88d-fa6e3d792e44	6826910b-c423-40b3-bfbd-df605f528968	CodesysIec	ce416b86-ee76-4ef0-b6d7-14e926d1a20f	7fda2c8f-4a14-4211-8dd8-f175e4b8462b	VALUE	Value	Value of the matrix row	80f884f5-8adc-4c6e-be02-915d6e203bb7	\N	"é"	1	2026-11-20 12:02:13.638465+00
\.


--
-- Data for Name: parameter_limit; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value, max_length, created_at) FROM stdin;
f1b88a4e-f083-4696-94d5-4a3c71a405eb	2de45b46-8d15-4943-a011-42f89a1d90eb		\N	\N	10	2026-11-20 12:02:13.445046+00
22110336-2f33-46dd-a4a4-7101a8f8cf76	29e00b9b-a944-4f81-b463-d49b88503d58		\N	\N	10	2026-11-20 12:02:13.49072+00
1f7f3491-e46c-4775-b0b4-fa574163ae55	4047715c-70d9-451d-8ab2-966f7f1dc497	.major	0	100	\N	2026-11-20 12:02:13.529622+00
1f7f3491-e46c-4775-b0b4-fa574163ae55	4047715c-70d9-451d-8ab2-966f7f1dc497	.minor	0	100	\N	2026-11-20 12:02:13.529622+00
527b46a4-493e-4244-9053-338bc33998c0	35a29460-305b-4353-97af-ab1bc85450b8	.value	0	100	\N	2026-11-20 12:02:13.61031+00
fe6e4900-42da-4323-a88d-fa6e3d792e44	6826910b-c423-40b3-bfbd-df605f528968		\N	\N	4	2026-11-20 12:02:13.638465+00
\.


--
-- Data for Name: parameter_translation; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.parameter_translation (parameter_id, recipe_version_id, lang, name, description, created_at) FROM stdin;
\.


--
-- Data for Name: plc_connection; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.plc_connection (id, machine_id, plc_platform, host, rack, slot, db_number, port, address, comm_mode, login, password_sealed, row_version, created_at, updated_at) FROM stdin;
\.


--
-- Data for Name: plc_type; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.plc_type (id, kind, name, platforms, size_bytes, string_length, leaf_class, element_type_id, dim_lower, dim_upper, machine_id, revision, supersedes_type_id, created_at) FROM stdin;
5eed0000-0000-4000-8000-000000000001	primitive	BOOL	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	1	\N	bool	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000002	primitive	BYTE	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000003	primitive	WORD	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000004	primitive	DWORD	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000005	primitive	LWORD	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000006	primitive	SINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000007	primitive	USINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000008	primitive	INT	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000009	primitive	UINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000a	primitive	DINT	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000b	primitive	UDINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000c	primitive	LINT	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000d	primitive	ULINT	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000e	primitive	REAL	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000000f	primitive	LREAL	{CodesysIec,TwinCat3,S7_1200,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000010	primitive	STRING	{S7_1200,S7_1500,S7_300}	256	254	string	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000011	primitive	WSTRING	{S7_1200,S7_1500}	512	254	string	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000012	primitive	CHAR	{S7_1200,S7_1500,S7_300}	1	\N	char	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000013	primitive	WCHAR	{S7_1200,S7_1500}	2	\N	char	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000014	primitive	TIME	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	duration	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000015	primitive	LTIME	{CodesysIec,TwinCat3,S7_1500}	8	\N	duration	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000016	primitive	DATE	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000017	primitive	LDATE	{CodesysIec,TwinCat3}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000018	primitive	TIME_OF_DAY	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-000000000019	primitive	LTOD	{CodesysIec,TwinCat3,S7_1500}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001a	primitive	DATE_AND_TIME	{CodesysIec,TwinCat3,S7_1500,S7_300}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001b	primitive	LDT	{CodesysIec,TwinCat3,S7_1500}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001c	primitive	DTL	{S7_1200,S7_1500}	12	\N	instant	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001d	primitive	S5TIME	{S7_1500,S7_300}	2	\N	duration	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001e	primitive	STRING	{CodesysIec,TwinCat3}	82	80	string	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
5eed0000-0000-4000-8000-00000000001f	primitive	WSTRING	{CodesysIec,TwinCat3}	164	80	string	\N	\N	\N	\N	1	\N	2026-11-20 12:02:13.140132+00
fd56f366-7512-4f11-8285-7cecda5154ff	primitive	WSTRING	{TwinCat3}	24	10	string	\N	\N	\N	2ec96487-4430-40f9-bef8-f98034463bf5	1	\N	2026-11-20 12:02:13.445046+00
f5092aa6-88a5-4bdc-9b87-99911d260d7a	primitive	WSTRING	{CodesysIec}	24	10	string	\N	\N	\N	6d612460-a630-443f-9277-007c32a239c3	1	\N	2026-11-20 12:02:13.49072+00
ce4d1eae-73c4-42ec-b8c7-b02bcb73befd	struct	VERSION	{TwinCat3}	\N	\N	\N	\N	\N	\N	db087d62-7aa2-4b03-99a0-0294faa178aa	1	\N	2026-11-20 12:02:13.514341+00
b5845c35-8da8-477a-845f-90f2189b1950	struct	ST_Coil	{CodesysIec}	\N	\N	\N	\N	\N	\N	c47efd5d-1f71-4ec1-8520-be399b89b134	1	\N	2026-11-20 12:02:13.558911+00
444fa16f-c385-47c5-b18a-afc098a6a9f4	struct	ST_One	{CodesysIec}	\N	\N	\N	\N	\N	\N	c76b49f9-f028-4d00-be95-6b5bb89fe526	1	\N	2026-11-20 12:02:13.596792+00
80f884f5-8adc-4c6e-be02-915d6e203bb7	primitive	STRING	{CodesysIec}	6	4	string	\N	\N	\N	00cd26a4-4172-4fee-ba63-104be5cca762	1	\N	2026-11-20 12:02:13.638465+00
\.


--
-- Data for Name: plc_type_member; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.plc_type_member (struct_type_id, ordinal, member_name, member_type_id, created_at) FROM stdin;
ce4d1eae-73c4-42ec-b8c7-b02bcb73befd	1	major	5eed0000-0000-4000-8000-000000000008	2026-11-20 12:02:13.514341+00
ce4d1eae-73c4-42ec-b8c7-b02bcb73befd	2	minor	5eed0000-0000-4000-8000-000000000008	2026-11-20 12:02:13.514341+00
b5845c35-8da8-477a-845f-90f2189b1950	1	END_IF	5eed0000-0000-4000-8000-000000000001	2026-11-20 12:02:13.558911+00
b5845c35-8da8-477a-845f-90f2189b1950	2	S	5eed0000-0000-4000-8000-000000000001	2026-11-20 12:02:13.558911+00
444fa16f-c385-47c5-b18a-afc098a6a9f4	1	value	5eed0000-0000-4000-8000-000000000008	2026-11-20 12:02:13.596792+00
\.


--
-- Data for Name: recipe; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.recipe (id, machine_id, plc_platform, reference_number, name, deleted_at, created_by, created_at, updated_at) FROM stdin;
717120e4-b1b0-49d9-8688-4679a3d6d924	2ec96487-4430-40f9-bef8-f98034463bf5	TwinCat3	SR-IEC-WSTRING-TC	SR-IEC-WSTRING-TC	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.424907+00	2026-11-20 12:02:13.424907+00
9696a65e-d566-4376-b3a4-f01f0aaa6fb8	6d612460-a630-443f-9277-007c32a239c3	CodesysIec	SR-IEC-WSTRING-CS	SR-IEC-WSTRING-CS	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.483798+00	2026-11-20 12:02:13.483798+00
e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	db087d62-7aa2-4b03-99a0-0294faa178aa	TwinCat3	SR-IEC-RESERVED-STRUCT	SR-IEC-RESERVED-STRUCT	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.52297+00	2026-11-20 12:02:13.52297+00
9ec15baf-8247-42e4-9f83-b9ca47b7e436	c47efd5d-1f71-4ec1-8520-be399b89b134	CodesysIec	SR-IEC-KEYWORD-MEMBER	SR-IEC-KEYWORD-MEMBER	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.565255+00	2026-11-20 12:02:13.565255+00
a58ace5b-3ed9-4f31-a601-f760cb525724	c76b49f9-f028-4d00-be95-6b5bb89fe526	CodesysIec	SR-IEC-ONE-MEMBER	SR-IEC-ONE-MEMBER	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.603195+00	2026-11-20 12:02:13.603195+00
63db2f60-9e7b-45db-a526-6088a23b166f	00cd26a4-4172-4fee-ba63-104be5cca762	CodesysIec	SR-IEC-STRING-LATIN1	SR-IEC-STRING-LATIN1	\N	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.631568+00	2026-11-20 12:02:13.631568+00
\.


--
-- Data for Name: recipe_version; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.recipe_version (id, recipe_id, plc_platform, number, status, row_version, based_on_version_id, change_note, content_sha256, created_by, created_at, updated_at, approved_by, approved_at, retired_at, deleted_at) FROM stdin;
2de45b46-8d15-4943-a011-42f89a1d90eb	717120e4-b1b0-49d9-8688-4679a3d6d924	TwinCat3	1	approved	3	\N	Stored row of the matrix	b6d9a8f1810cf83c46a84697e88b8c06ae2a8d1f29f1d910d2a93e1a67600862	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.424907+00	2026-11-20 12:02:13.473554+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.473554+00	\N	\N
29e00b9b-a944-4f81-b463-d49b88503d58	9696a65e-d566-4376-b3a4-f01f0aaa6fb8	CodesysIec	1	approved	3	\N	Stored row of the matrix	98ca14c97da1093fef2c961a053155f4c8d3de9861332d743e451d3255bd68f5	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.483798+00	2026-11-20 12:02:13.505533+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.505533+00	\N	\N
4047715c-70d9-451d-8ab2-966f7f1dc497	e8f28a2e-c0ae-483c-a9ac-4e57ae75ae35	TwinCat3	1	approved	3	\N	Stored row of the matrix	279516853a2b79709954f99dcb149cc020bb4b34186e2768293333390ba6f556	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.52297+00	2026-11-20 12:02:13.548983+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.548983+00	\N	\N
af56350c-10e0-41e3-b422-15c45b313de4	9ec15baf-8247-42e4-9f83-b9ca47b7e436	CodesysIec	1	approved	3	\N	Stored row of the matrix	185954e961df02479b108120c474dedd1739011868f321282de22b95ae49cc2a	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.565255+00	2026-11-20 12:02:13.588004+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.588004+00	\N	\N
35a29460-305b-4353-97af-ab1bc85450b8	a58ace5b-3ed9-4f31-a601-f760cb525724	CodesysIec	1	approved	3	\N	Stored row of the matrix	2facdcf5a391ce911375c5bc1468a19f217ff1956458fddae05d968d922ac1cd	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.603195+00	2026-11-20 12:02:13.624037+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.624037+00	\N	\N
6826910b-c423-40b3-bfbd-df605f528968	63db2f60-9e7b-45db-a526-6088a23b166f	CodesysIec	1	approved	3	\N	Stored row of the matrix	ed72ebd29f5551d76020017dcb6a915cef4b861ff4606011c6768fcc6695d03f	d9dff170-ad15-42b1-a15f-74b85178c69b	2026-11-20 12:02:13.631568+00	2026-11-20 12:02:13.655034+00	8b06d67b-b1c2-41a6-870c-941a85183736	2026-11-20 12:02:13.655034+00	\N	\N
\.


--
-- Data for Name: security_event; Type: TABLE DATA; Schema: public; Owner: audit_owner
--

COPY public.security_event (id, occurred_at, kind, login_hmac, user_id, client_ip, user_agent, request_id, detail, count) FROM stdin;
\.


--
-- Data for Name: session; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.session (id, user_id, epoch, csrf_token, created_at, last_seen_at, expires_at) FROM stdin;
\.


--
-- Data for Name: unit; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.unit (id, machine_id, path, name, archived_at, created_at, updated_at) FROM stdin;
d0d8f101-58b6-4b57-9ede-acbcce9bc5ac	2ec96487-4430-40f9-bef8-f98034463bf5	U1	unit	\N	2026-11-20 12:02:13.393868+00	2026-11-20 12:02:13.393868+00
66c39e6c-4caf-460c-9d53-aaff74efbe56	6d612460-a630-443f-9277-007c32a239c3	U1	unit	\N	2026-11-20 12:02:13.481045+00	2026-11-20 12:02:13.481045+00
e0f35cca-6f35-48bf-8487-4a17a7447cd7	db087d62-7aa2-4b03-99a0-0294faa178aa	U1	unit	\N	2026-11-20 12:02:13.512346+00	2026-11-20 12:02:13.512346+00
8f542085-69dc-4047-acdf-dd011b2fd579	c47efd5d-1f71-4ec1-8520-be399b89b134	U1	unit	\N	2026-11-20 12:02:13.556426+00	2026-11-20 12:02:13.556426+00
261deddc-3566-4a97-bc1d-3323937dad5f	c76b49f9-f028-4d00-be95-6b5bb89fe526	U1	unit	\N	2026-11-20 12:02:13.594375+00	2026-11-20 12:02:13.594375+00
7fda2c8f-4a14-4211-8dd8-f175e4b8462b	00cd26a4-4172-4fee-ba63-104be5cca762	U1	unit	\N	2026-11-20 12:02:13.629621+00	2026-11-20 12:02:13.629621+00
\.


--
-- Name: audit_log_id_seq; Type: SEQUENCE SET; Schema: public; Owner: audit_owner
--

SELECT pg_catalog.setval('public.audit_log_id_seq', 42, true);


--
-- Name: login_attempt_id_seq; Type: SEQUENCE SET; Schema: public; Owner: recipe_migrator
--

SELECT pg_catalog.setval('public.login_attempt_id_seq', 1, false);


--
-- Name: security_event_id_seq; Type: SEQUENCE SET; Schema: public; Owner: audit_owner
--

SELECT pg_catalog.setval('public.security_event_id_seq', 1, false);


--
-- Name: _sqlx_migrations _sqlx_migrations_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public._sqlx_migrations
    ADD CONSTRAINT _sqlx_migrations_pkey PRIMARY KEY (version);


--
-- Name: app_setting app_setting_key_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.app_setting
    ADD CONSTRAINT app_setting_key_unique UNIQUE (key);


--
-- Name: app_setting app_setting_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.app_setting
    ADD CONSTRAINT app_setting_pkey PRIMARY KEY (id);


--
-- Name: app_user app_user_login_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.app_user
    ADD CONSTRAINT app_user_login_unique UNIQUE (login);


--
-- Name: app_user app_user_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.app_user
    ADD CONSTRAINT app_user_pkey PRIMARY KEY (id);


--
-- Name: audit_chain_head audit_chain_head_pkey; Type: CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.audit_chain_head
    ADD CONSTRAINT audit_chain_head_pkey PRIMARY KEY (singleton);


--
-- Name: audit_log audit_log_chain_seq_unique; Type: CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_chain_seq_unique UNIQUE (chain_seq);


--
-- Name: audit_log audit_log_pkey; Type: CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_pkey PRIMARY KEY (id);


--
-- Name: draft_editor draft_editor_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.draft_editor
    ADD CONSTRAINT draft_editor_pkey PRIMARY KEY (recipe_version_id, user_id);


--
-- Name: login_attempt login_attempt_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.login_attempt
    ADD CONSTRAINT login_attempt_pkey PRIMARY KEY (id);


--
-- Name: machine machine_id_platform_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.machine
    ADD CONSTRAINT machine_id_platform_unique UNIQUE (id, plc_platform);


--
-- Name: machine machine_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.machine
    ADD CONSTRAINT machine_pkey PRIMARY KEY (id);


--
-- Name: parameter parameter_id_version_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_id_version_unique UNIQUE (id, recipe_version_id);


--
-- Name: parameter parameter_key_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_key_unique UNIQUE (recipe_version_id, parameter_key);


--
-- Name: parameter_limit parameter_limit_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter_limit
    ADD CONSTRAINT parameter_limit_pkey PRIMARY KEY (parameter_id, leaf_path);


--
-- Name: parameter parameter_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_pkey PRIMARY KEY (id);


--
-- Name: parameter parameter_tag_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_tag_unique UNIQUE (recipe_version_id, tag);


--
-- Name: parameter_translation parameter_translation_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter_translation
    ADD CONSTRAINT parameter_translation_pkey PRIMARY KEY (parameter_id, lang);


--
-- Name: plc_connection plc_connection_machine_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_connection
    ADD CONSTRAINT plc_connection_machine_unique UNIQUE (machine_id);


--
-- Name: plc_connection plc_connection_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_connection
    ADD CONSTRAINT plc_connection_pkey PRIMARY KEY (id);


--
-- Name: plc_type_member plc_type_member_name_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type_member
    ADD CONSTRAINT plc_type_member_name_unique UNIQUE (struct_type_id, member_name);


--
-- Name: plc_type_member plc_type_member_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type_member
    ADD CONSTRAINT plc_type_member_pkey PRIMARY KEY (struct_type_id, ordinal);


--
-- Name: plc_type plc_type_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type
    ADD CONSTRAINT plc_type_pkey PRIMARY KEY (id);


--
-- Name: recipe recipe_id_platform_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe
    ADD CONSTRAINT recipe_id_platform_unique UNIQUE (id, plc_platform);


--
-- Name: recipe recipe_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe
    ADD CONSTRAINT recipe_pkey PRIMARY KEY (id);


--
-- Name: recipe recipe_reference_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe
    ADD CONSTRAINT recipe_reference_unique UNIQUE (reference_number);


--
-- Name: recipe_version recipe_version_id_platform_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_id_platform_unique UNIQUE (id, plc_platform);


--
-- Name: recipe_version recipe_version_number_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_number_unique UNIQUE (recipe_id, number);


--
-- Name: recipe_version recipe_version_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_pkey PRIMARY KEY (id);


--
-- Name: recipe_version recipe_version_recipe_id_id_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_recipe_id_id_unique UNIQUE (recipe_id, id);


--
-- Name: security_event security_event_pkey; Type: CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.security_event
    ADD CONSTRAINT security_event_pkey PRIMARY KEY (id);


--
-- Name: session session_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.session
    ADD CONSTRAINT session_pkey PRIMARY KEY (id);


--
-- Name: unit unit_machine_id_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.unit
    ADD CONSTRAINT unit_machine_id_unique UNIQUE (machine_id, id);


--
-- Name: unit unit_path_unique; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.unit
    ADD CONSTRAINT unit_path_unique UNIQUE (machine_id, path);


--
-- Name: unit unit_pkey; Type: CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.unit
    ADD CONSTRAINT unit_pkey PRIMARY KEY (id);


--
-- Name: app_setting_updated_by_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX app_setting_updated_by_idx ON public.app_setting USING btree (updated_by);


--
-- Name: audit_log_action_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX audit_log_action_idx ON public.audit_log USING btree (action, id);


--
-- Name: audit_log_actor_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX audit_log_actor_idx ON public.audit_log USING btree (actor_user_id, id);


--
-- Name: audit_log_aggregate_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX audit_log_aggregate_idx ON public.audit_log USING btree (aggregate_id, id);


--
-- Name: audit_log_entity_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX audit_log_entity_idx ON public.audit_log USING btree (entity_type, entity_id, id);


--
-- Name: audit_log_occurred_at_brin; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX audit_log_occurred_at_brin ON public.audit_log USING brin (occurred_at);


--
-- Name: draft_editor_user_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX draft_editor_user_id_idx ON public.draft_editor USING btree (user_id);


--
-- Name: login_attempt_attempted_at_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX login_attempt_attempted_at_idx ON public.login_attempt USING btree (attempted_at);


--
-- Name: login_attempt_ip_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX login_attempt_ip_idx ON public.login_attempt USING btree (client_ip, attempted_at);


--
-- Name: login_attempt_login_ip_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX login_attempt_login_ip_idx ON public.login_attempt USING btree (login, client_ip, attempted_at);


--
-- Name: machine_code_live_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX machine_code_live_unique ON public.machine USING btree (code) WHERE (archived_at IS NULL);


--
-- Name: parameter_key_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_key_idx ON public.parameter USING btree (parameter_key);


--
-- Name: parameter_limit_version_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_limit_version_idx ON public.parameter_limit USING btree (recipe_version_id);


--
-- Name: parameter_plc_type_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_plc_type_id_idx ON public.parameter USING btree (plc_type_id);


--
-- Name: parameter_translation_version_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_translation_version_idx ON public.parameter_translation USING btree (recipe_version_id);


--
-- Name: parameter_unit_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_unit_id_idx ON public.parameter USING btree (unit_id);


--
-- Name: parameter_version_unit_ordinal_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX parameter_version_unit_ordinal_idx ON public.parameter USING btree (recipe_version_id, unit_id, ordinal);


--
-- Name: plc_type_array_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX plc_type_array_unique ON public.plc_type USING btree (machine_id, element_type_id, dim_lower, dim_upper) WHERE (kind = 'array'::text);


--
-- Name: plc_type_element_type_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX plc_type_element_type_id_idx ON public.plc_type USING btree (element_type_id);


--
-- Name: plc_type_machine_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX plc_type_machine_id_idx ON public.plc_type USING btree (machine_id);


--
-- Name: plc_type_member_member_type_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX plc_type_member_member_type_id_idx ON public.plc_type_member USING btree (member_type_id);


--
-- Name: plc_type_member_name_ci_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX plc_type_member_name_ci_unique ON public.plc_type_member USING btree (struct_type_id, lower(member_name));


--
-- Name: plc_type_named_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX plc_type_named_unique ON public.plc_type USING btree (machine_id, name, revision) NULLS NOT DISTINCT WHERE ((kind <> 'array'::text) AND (string_length IS NULL));


--
-- Name: plc_type_string_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX plc_type_string_unique ON public.plc_type USING btree (machine_id, name, string_length) NULLS NOT DISTINCT WHERE (string_length IS NOT NULL);


--
-- Name: plc_type_struct_name_ci_unique; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX plc_type_struct_name_ci_unique ON public.plc_type USING btree (machine_id, lower(name), revision) WHERE (kind = 'struct'::text);


--
-- Name: plc_type_supersedes_type_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX plc_type_supersedes_type_id_idx ON public.plc_type USING btree (supersedes_type_id);


--
-- Name: recipe_created_by_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_created_by_idx ON public.recipe USING btree (created_by);


--
-- Name: recipe_machine_id_all_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_machine_id_all_idx ON public.recipe USING btree (machine_id);


--
-- Name: recipe_machine_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_machine_id_idx ON public.recipe USING btree (machine_id, name) WHERE (deleted_at IS NULL);


--
-- Name: recipe_version_approved_by_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_version_approved_by_idx ON public.recipe_version USING btree (approved_by);


--
-- Name: recipe_version_based_on_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_version_based_on_idx ON public.recipe_version USING btree (based_on_version_id);


--
-- Name: recipe_version_created_by_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX recipe_version_created_by_idx ON public.recipe_version USING btree (created_by);


--
-- Name: recipe_version_one_approved; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX recipe_version_one_approved ON public.recipe_version USING btree (recipe_id) WHERE (status = 'approved'::text);


--
-- Name: recipe_version_one_live_draft; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE UNIQUE INDEX recipe_version_one_live_draft ON public.recipe_version USING btree (recipe_id) WHERE ((status = 'draft'::text) AND (deleted_at IS NULL));


--
-- Name: security_event_aggregate_unique; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE UNIQUE INDEX security_event_aggregate_unique ON public.security_event USING btree (client_ip, kind, date_trunc('minute'::text, (occurred_at AT TIME ZONE 'UTC'::text))) WHERE (kind = ANY (ARRAY['login_failed'::text, 'login_throttled'::text, 'csrf_rejected'::text]));


--
-- Name: security_event_client_ip_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX security_event_client_ip_idx ON public.security_event USING btree (client_ip, id);


--
-- Name: security_event_kind_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX security_event_kind_idx ON public.security_event USING btree (kind, id);


--
-- Name: security_event_occurred_at_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX security_event_occurred_at_idx ON public.security_event USING btree (occurred_at);


--
-- Name: security_event_user_id_idx; Type: INDEX; Schema: public; Owner: audit_owner
--

CREATE INDEX security_event_user_id_idx ON public.security_event USING btree (user_id, id);


--
-- Name: session_expires_at_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX session_expires_at_idx ON public.session USING btree (expires_at);


--
-- Name: session_user_id_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX session_user_id_idx ON public.session USING btree (user_id);


--
-- Name: unit_parent_path_idx; Type: INDEX; Schema: public; Owner: recipe_migrator
--

CREATE INDEX unit_parent_path_idx ON public.unit USING btree (machine_id, parent_path);


--
-- Name: app_setting app_setting_set_updated_at; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER app_setting_set_updated_at BEFORE UPDATE ON public.app_setting FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();


--
-- Name: app_user app_user_set_updated_at; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER app_user_set_updated_at BEFORE UPDATE ON public.app_user FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();


--
-- Name: audit_log audit_log_hash_chain; Type: TRIGGER; Schema: public; Owner: audit_owner
--

CREATE TRIGGER audit_log_hash_chain BEFORE INSERT ON public.audit_log FOR EACH ROW EXECUTE FUNCTION public.audit_hash_chain();


--
-- Name: machine machine_freeze_identifiers; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER machine_freeze_identifiers BEFORE UPDATE OF code ON public.machine FOR EACH ROW EXECUTE FUNCTION public.freeze_identifiers();


--
-- Name: machine machine_platform_frozen; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER machine_platform_frozen BEFORE UPDATE OF plc_platform ON public.machine FOR EACH ROW EXECUTE FUNCTION public.machine_platform_frozen();


--
-- Name: machine machine_set_updated_at; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER machine_set_updated_at BEFORE UPDATE ON public.machine FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();


--
-- Name: parameter_limit parameter_limit_mutable; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_limit_mutable BEFORE INSERT OR DELETE OR UPDATE ON public.parameter_limit FOR EACH ROW EXECUTE FUNCTION public.enforce_version_mutable();


--
-- Name: parameter_limit parameter_limit_size_budget; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_limit_size_budget AFTER INSERT ON public.parameter_limit REFERENCING NEW TABLE AS inserted FOR EACH STATEMENT EXECUTE FUNCTION public.version_size_budget();


--
-- Name: parameter parameter_mutable; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_mutable BEFORE INSERT OR DELETE OR UPDATE ON public.parameter FOR EACH ROW EXECUTE FUNCTION public.enforce_version_mutable();


--
-- Name: parameter parameter_size_budget; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_size_budget AFTER INSERT ON public.parameter REFERENCING NEW TABLE AS inserted FOR EACH STATEMENT EXECUTE FUNCTION public.version_size_budget();


--
-- Name: parameter_translation parameter_translation_mutable; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_translation_mutable BEFORE INSERT OR DELETE OR UPDATE ON public.parameter_translation FOR EACH ROW EXECUTE FUNCTION public.enforce_version_mutable();


--
-- Name: parameter parameter_type_platform; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_type_platform BEFORE INSERT OR UPDATE OF plc_type_id, plc_platform ON public.parameter FOR EACH ROW EXECUTE FUNCTION public.enforce_type_platform();


--
-- Name: parameter parameter_unit_machine; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER parameter_unit_machine BEFORE INSERT OR UPDATE OF unit_id, recipe_version_id ON public.parameter FOR EACH ROW EXECUTE FUNCTION public.enforce_unit_machine();


--
-- Name: plc_connection plc_connection_a_bump; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER plc_connection_a_bump BEFORE UPDATE ON public.plc_connection FOR EACH ROW EXECUTE FUNCTION public.bump_row_version();


--
-- Name: plc_connection plc_connection_sync_db_number; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER plc_connection_sync_db_number AFTER INSERT OR UPDATE OF db_number ON public.plc_connection FOR EACH ROW EXECUTE FUNCTION public.plc_connection_sync_db_number();


--
-- Name: plc_type plc_type_immutable; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER plc_type_immutable BEFORE DELETE OR UPDATE ON public.plc_type FOR EACH ROW EXECUTE FUNCTION public.plc_type_immutable();


--
-- Name: plc_type_member plc_type_member_immutable; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER plc_type_member_immutable BEFORE INSERT OR DELETE OR UPDATE ON public.plc_type_member FOR EACH ROW EXECUTE FUNCTION public.plc_type_immutable();


--
-- Name: plc_type plc_type_struct_has_member; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE CONSTRAINT TRIGGER plc_type_struct_has_member AFTER INSERT ON public.plc_type DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION public.plc_type_struct_has_member();


--
-- Name: recipe recipe_freeze_identifiers; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER recipe_freeze_identifiers BEFORE UPDATE OF reference_number, machine_id ON public.recipe FOR EACH ROW EXECUTE FUNCTION public.freeze_identifiers();


--
-- Name: recipe recipe_set_updated_at; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER recipe_set_updated_at BEFORE UPDATE ON public.recipe FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();


--
-- Name: recipe_version recipe_version_a_bump; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER recipe_version_a_bump BEFORE UPDATE ON public.recipe_version FOR EACH ROW EXECUTE FUNCTION public.bump_row_version();


--
-- Name: recipe_version recipe_version_b_transition; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER recipe_version_b_transition BEFORE UPDATE ON public.recipe_version FOR EACH ROW EXECUTE FUNCTION public.recipe_version_transition();


--
-- Name: recipe_version recipe_version_insert_draft; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER recipe_version_insert_draft BEFORE INSERT ON public.recipe_version FOR EACH ROW EXECUTE FUNCTION public.recipe_version_insert_draft();


--
-- Name: security_event security_event_guard; Type: TRIGGER; Schema: public; Owner: audit_owner
--

CREATE TRIGGER security_event_guard BEFORE UPDATE ON public.security_event FOR EACH ROW EXECUTE FUNCTION public.security_event_guard();


--
-- Name: unit unit_freeze_identifiers; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER unit_freeze_identifiers BEFORE UPDATE OF path, machine_id ON public.unit FOR EACH ROW EXECUTE FUNCTION public.freeze_identifiers();


--
-- Name: unit unit_set_updated_at; Type: TRIGGER; Schema: public; Owner: recipe_migrator
--

CREATE TRIGGER unit_set_updated_at BEFORE UPDATE ON public.unit FOR EACH ROW EXECUTE FUNCTION public.set_updated_at();


--
-- Name: app_setting app_setting_updated_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.app_setting
    ADD CONSTRAINT app_setting_updated_by_fkey FOREIGN KEY (updated_by) REFERENCES public.app_user(id);


--
-- Name: audit_log audit_log_actor_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.audit_log
    ADD CONSTRAINT audit_log_actor_user_id_fkey FOREIGN KEY (actor_user_id) REFERENCES public.app_user(id);


--
-- Name: draft_editor draft_editor_recipe_version_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.draft_editor
    ADD CONSTRAINT draft_editor_recipe_version_id_fkey FOREIGN KEY (recipe_version_id) REFERENCES public.recipe_version(id);


--
-- Name: draft_editor draft_editor_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.draft_editor
    ADD CONSTRAINT draft_editor_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.app_user(id);


--
-- Name: parameter_limit parameter_limit_parameter; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter_limit
    ADD CONSTRAINT parameter_limit_parameter FOREIGN KEY (parameter_id, recipe_version_id) REFERENCES public.parameter(id, recipe_version_id) ON DELETE CASCADE;


--
-- Name: parameter parameter_plc_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_plc_type_id_fkey FOREIGN KEY (plc_type_id) REFERENCES public.plc_type(id);


--
-- Name: parameter_translation parameter_translation_parameter; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter_translation
    ADD CONSTRAINT parameter_translation_parameter FOREIGN KEY (parameter_id, recipe_version_id) REFERENCES public.parameter(id, recipe_version_id) ON DELETE CASCADE;


--
-- Name: parameter parameter_unit_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_unit_id_fkey FOREIGN KEY (unit_id) REFERENCES public.unit(id);


--
-- Name: parameter parameter_version_platform; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.parameter
    ADD CONSTRAINT parameter_version_platform FOREIGN KEY (recipe_version_id, plc_platform) REFERENCES public.recipe_version(id, plc_platform);


--
-- Name: plc_connection plc_connection_machine_platform; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_connection
    ADD CONSTRAINT plc_connection_machine_platform FOREIGN KEY (machine_id, plc_platform) REFERENCES public.machine(id, plc_platform);


--
-- Name: plc_type plc_type_element_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type
    ADD CONSTRAINT plc_type_element_type_id_fkey FOREIGN KEY (element_type_id) REFERENCES public.plc_type(id);


--
-- Name: plc_type plc_type_machine_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type
    ADD CONSTRAINT plc_type_machine_id_fkey FOREIGN KEY (machine_id) REFERENCES public.machine(id);


--
-- Name: plc_type_member plc_type_member_member_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type_member
    ADD CONSTRAINT plc_type_member_member_type_id_fkey FOREIGN KEY (member_type_id) REFERENCES public.plc_type(id);


--
-- Name: plc_type_member plc_type_member_struct_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type_member
    ADD CONSTRAINT plc_type_member_struct_type_id_fkey FOREIGN KEY (struct_type_id) REFERENCES public.plc_type(id);


--
-- Name: plc_type plc_type_supersedes_type_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.plc_type
    ADD CONSTRAINT plc_type_supersedes_type_id_fkey FOREIGN KEY (supersedes_type_id) REFERENCES public.plc_type(id);


--
-- Name: recipe recipe_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe
    ADD CONSTRAINT recipe_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.app_user(id);


--
-- Name: recipe recipe_machine_platform; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe
    ADD CONSTRAINT recipe_machine_platform FOREIGN KEY (machine_id, plc_platform) REFERENCES public.machine(id, plc_platform);


--
-- Name: recipe_version recipe_version_approved_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_approved_by_fkey FOREIGN KEY (approved_by) REFERENCES public.app_user(id);


--
-- Name: recipe_version recipe_version_based_on_same_recipe; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_based_on_same_recipe FOREIGN KEY (recipe_id, based_on_version_id) REFERENCES public.recipe_version(recipe_id, id);


--
-- Name: recipe_version recipe_version_created_by_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_created_by_fkey FOREIGN KEY (created_by) REFERENCES public.app_user(id);


--
-- Name: recipe_version recipe_version_recipe_platform; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.recipe_version
    ADD CONSTRAINT recipe_version_recipe_platform FOREIGN KEY (recipe_id, plc_platform) REFERENCES public.recipe(id, plc_platform);


--
-- Name: security_event security_event_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: audit_owner
--

ALTER TABLE ONLY public.security_event
    ADD CONSTRAINT security_event_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.app_user(id);


--
-- Name: session session_user_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.session
    ADD CONSTRAINT session_user_id_fkey FOREIGN KEY (user_id) REFERENCES public.app_user(id);


--
-- Name: unit unit_machine_id_fkey; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.unit
    ADD CONSTRAINT unit_machine_id_fkey FOREIGN KEY (machine_id) REFERENCES public.machine(id);


--
-- Name: unit unit_parent_exists; Type: FK CONSTRAINT; Schema: public; Owner: recipe_migrator
--

ALTER TABLE ONLY public.unit
    ADD CONSTRAINT unit_parent_exists FOREIGN KEY (machine_id, parent_path) REFERENCES public.unit(machine_id, path);


--
-- Name: SCHEMA public; Type: ACL; Schema: -; Owner: pg_database_owner
--

REVOKE USAGE ON SCHEMA public FROM PUBLIC;
GRANT USAGE ON SCHEMA public TO recipe_app;
GRANT USAGE ON SCHEMA public TO recipe_backup;
GRANT USAGE ON SCHEMA public TO audit_owner;


--
-- Name: FUNCTION audit_objects_dropped(); Type: ACL; Schema: audit_guard; Owner: postgres
--

REVOKE ALL ON FUNCTION audit_guard.audit_objects_dropped() FROM PUBLIC;


--
-- Name: TABLE audit_log; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT SELECT ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.actor_user_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(actor_user_id) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.action; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(action) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.entity_type; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(entity_type) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.entity_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(entity_id) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.aggregate_type; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(aggregate_type) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.aggregate_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(aggregate_id) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.before_state; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(before_state) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.after_state; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(after_state) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.request_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(request_id) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.client_ip; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(client_ip) ON TABLE public.audit_log TO recipe_app;


--
-- Name: COLUMN audit_log.session_ref; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(session_ref) ON TABLE public.audit_log TO recipe_app;


--
-- Name: FUNCTION audit_hash_chain(); Type: ACL; Schema: public; Owner: audit_owner
--

REVOKE ALL ON FUNCTION public.audit_hash_chain() FROM PUBLIC;


--
-- Name: FUNCTION replication_state(OUT connected boolean, OUT lag_bytes numeric); Type: ACL; Schema: public; Owner: audit_owner
--

REVOKE ALL ON FUNCTION public.replication_state(OUT connected boolean, OUT lag_bytes numeric) FROM PUBLIC;
GRANT ALL ON FUNCTION public.replication_state(OUT connected boolean, OUT lag_bytes numeric) TO recipe_app;


--
-- Name: FUNCTION security_event_guard(); Type: ACL; Schema: public; Owner: audit_owner
--

REVOKE ALL ON FUNCTION public.security_event_guard() FROM PUBLIC;


--
-- Name: FUNCTION security_event_purge(); Type: ACL; Schema: public; Owner: audit_owner
--

REVOKE ALL ON FUNCTION public.security_event_purge() FROM PUBLIC;
GRANT ALL ON FUNCTION public.security_event_purge() TO recipe_app;


--
-- Name: TABLE _sqlx_migrations; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT ON TABLE public._sqlx_migrations TO recipe_app;


--
-- Name: TABLE app_setting; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.app_setting TO recipe_app;


--
-- Name: COLUMN app_setting.value; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(value) ON TABLE public.app_setting TO recipe_app;


--
-- Name: COLUMN app_setting.updated_by; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_by) ON TABLE public.app_setting TO recipe_app;


--
-- Name: COLUMN app_setting.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.app_setting TO recipe_app;


--
-- Name: TABLE app_user; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.password_hash; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(password_hash) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.role; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(role) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.is_active; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(is_active) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.session_epoch; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(session_epoch) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.must_change_password; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(must_change_password) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.deactivated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(deactivated_at) ON TABLE public.app_user TO recipe_app;


--
-- Name: COLUMN app_user.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.app_user TO recipe_app;


--
-- Name: TABLE audit_chain_head; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT SELECT ON TABLE public.audit_chain_head TO recipe_app;


--
-- Name: TABLE draft_editor; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.draft_editor TO recipe_app;


--
-- Name: TABLE login_attempt; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.login_attempt TO recipe_app;


--
-- Name: COLUMN login_attempt.succeeded; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(succeeded) ON TABLE public.login_attempt TO recipe_app;


--
-- Name: TABLE machine; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.machine TO recipe_app;


--
-- Name: COLUMN machine.name; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(name) ON TABLE public.machine TO recipe_app;


--
-- Name: COLUMN machine.archived_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(archived_at) ON TABLE public.machine TO recipe_app;


--
-- Name: COLUMN machine.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.machine TO recipe_app;


--
-- Name: COLUMN machine.db_number; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(db_number) ON TABLE public.machine TO recipe_app;


--
-- Name: TABLE parameter; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.unit_id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(unit_id) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.tag; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(tag) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.name_en; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(name_en) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.description_en; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(description_en) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.plc_type_id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(plc_type_id) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.s7_address; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(s7_address) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.value; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(value) ON TABLE public.parameter TO recipe_app;


--
-- Name: COLUMN parameter.ordinal; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(ordinal) ON TABLE public.parameter TO recipe_app;


--
-- Name: TABLE parameter_limit; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.parameter_limit TO recipe_app;


--
-- Name: COLUMN parameter_limit.min_value; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(min_value) ON TABLE public.parameter_limit TO recipe_app;


--
-- Name: COLUMN parameter_limit.max_value; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(max_value) ON TABLE public.parameter_limit TO recipe_app;


--
-- Name: COLUMN parameter_limit.max_length; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(max_length) ON TABLE public.parameter_limit TO recipe_app;


--
-- Name: TABLE parameter_translation; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.parameter_translation TO recipe_app;


--
-- Name: COLUMN parameter_translation.name; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(name) ON TABLE public.parameter_translation TO recipe_app;


--
-- Name: COLUMN parameter_translation.description; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(description) ON TABLE public.parameter_translation TO recipe_app;


--
-- Name: TABLE plc_connection; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.host; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(host) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.rack; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(rack) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.slot; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(slot) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.db_number; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(db_number) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.port; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(port) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.address; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(address) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.comm_mode; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(comm_mode) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.login; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(login) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.password_sealed; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(password_sealed) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: COLUMN plc_connection.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.plc_connection TO recipe_app;


--
-- Name: TABLE plc_type; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(id) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.kind; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(kind) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.name; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(name) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.platforms; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(platforms) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.size_bytes; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(size_bytes) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.string_length; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(string_length) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.leaf_class; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(leaf_class) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.element_type_id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(element_type_id) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.dim_lower; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(dim_lower) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.dim_upper; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(dim_upper) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.machine_id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(machine_id) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.revision; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(revision) ON TABLE public.plc_type TO recipe_app;


--
-- Name: COLUMN plc_type.supersedes_type_id; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT INSERT(supersedes_type_id) ON TABLE public.plc_type TO recipe_app;


--
-- Name: TABLE plc_type_member; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.plc_type_member TO recipe_app;


--
-- Name: TABLE recipe; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.recipe TO recipe_app;


--
-- Name: COLUMN recipe.name; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(name) ON TABLE public.recipe TO recipe_app;


--
-- Name: COLUMN recipe.deleted_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(deleted_at) ON TABLE public.recipe TO recipe_app;


--
-- Name: COLUMN recipe.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.recipe TO recipe_app;


--
-- Name: TABLE recipe_version; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.status; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(status) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.change_note; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(change_note) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.content_sha256; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(content_sha256) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.approved_by; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(approved_by) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.approved_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(approved_at) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.retired_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(retired_at) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: COLUMN recipe_version.deleted_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(deleted_at) ON TABLE public.recipe_version TO recipe_app;


--
-- Name: TABLE security_event; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT SELECT ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.kind; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(kind) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.login_hmac; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(login_hmac) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.user_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(user_id) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.client_ip; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(client_ip) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.user_agent; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(user_agent) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.request_id; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(request_id) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.detail; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT INSERT(detail) ON TABLE public.security_event TO recipe_app;


--
-- Name: COLUMN security_event.count; Type: ACL; Schema: public; Owner: audit_owner
--

GRANT UPDATE(count) ON TABLE public.security_event TO recipe_app;


--
-- Name: TABLE session; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT,DELETE ON TABLE public.session TO recipe_app;


--
-- Name: COLUMN session.csrf_token; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(csrf_token) ON TABLE public.session TO recipe_app;


--
-- Name: COLUMN session.last_seen_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(last_seen_at) ON TABLE public.session TO recipe_app;


--
-- Name: COLUMN session.expires_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(expires_at) ON TABLE public.session TO recipe_app;


--
-- Name: TABLE unit; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT SELECT,INSERT ON TABLE public.unit TO recipe_app;


--
-- Name: COLUMN unit.name; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(name) ON TABLE public.unit TO recipe_app;


--
-- Name: COLUMN unit.archived_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(archived_at) ON TABLE public.unit TO recipe_app;


--
-- Name: COLUMN unit.updated_at; Type: ACL; Schema: public; Owner: recipe_migrator
--

GRANT UPDATE(updated_at) ON TABLE public.unit TO recipe_app;


--
-- Name: DEFAULT PRIVILEGES FOR TABLES; Type: DEFAULT ACL; Schema: public; Owner: recipe_migrator
--

ALTER DEFAULT PRIVILEGES FOR ROLE recipe_migrator IN SCHEMA public GRANT SELECT ON TABLES TO recipe_app;


--
-- Name: audit_objects_dropped; Type: EVENT TRIGGER; Schema: -; Owner: postgres
--

CREATE EVENT TRIGGER audit_objects_dropped ON sql_drop
   EXECUTE FUNCTION audit_guard.audit_objects_dropped();


ALTER EVENT TRIGGER audit_objects_dropped OWNER TO postgres;

--
-- PostgreSQL database dump complete
--

\unrestrict IY1G2fHBf6p9pIbawsn6CiL30GqiS0yL6gaFDEKn1ldpQshphJV4LqgQGGgGDba

