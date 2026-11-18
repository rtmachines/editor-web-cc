--
-- PostgreSQL database dump
--

\restrict G5JEiWoPk4nrEjwnKBqZmszmUJQDf8YS0bdgOCGQi2UWGYoBfTHagC2HlPUzfu2

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
1	schema	2026-11-17 19:17:04.843209+00	t	\\xa8074a1ca24050fa1da4c362335412e0463259ff74591e91e4ccc9bce444e9123df5fb8ffdbc12b4017e32c4aa27002b	946181510
2	triggers	2026-11-17 19:17:05.792448+00	t	\\xf7c4a521f4417fcaa9c106802107b768f121415e5c455aee1391ff77f4d6df7b0caf7a1d242737b3213206f3185afa5d	5391794
3	grants	2026-11-17 19:17:05.799421+00	t	\\x8bef38cd7a68befb34c0b5d5297338bb3ebb6eadeb80120540b766348db5a57078a393e391bf25c47e2cf52abc21025e	11041466
4	seed plc types	2026-11-17 19:17:05.815589+00	t	\\xfdf29fe2bc0474e45ab7f01d6f27c57dcf24fd0f183c52b54cedec869aee7e9bc54afbdf8665102c22c42760e1097bea	6987405
5	security event guard	2026-11-17 19:17:05.828304+00	t	\\x9fa9cabea400c4448d0a03e51d866a43788d41583d207e31ce95225a8614e5779e60e3adc6ac7bcfcdc7e97bf0dc0a6e	7784402
6	operations	2026-11-17 19:17:05.838305+00	t	\\xa2714ec7de0547f74f522e35042c6836e9fc41951f9eac20b3ffbed19f34c88bfc8ba1d395731c20c3a9e3340418cf1e	47707801
7	audit definer operators	2026-11-17 19:17:05.887396+00	t	\\x49cea3ce54ce09314de441ed02cb7451bb58627035341fb5f4bac2bb7dd93cb981abfdeb50b98503309bd98f013f001e	88275190
8	audit log insert columns	2026-11-17 19:17:06.052353+00	t	\\xbc78aee1b04376cd174b9af831832ae3e29635406b019965224525d03d4d9adf3885dd7866041978a60ea1a7da648c98	32477122
9	export client role	2026-11-17 19:17:06.090997+00	t	\\xdfe3418f00ec1b064d135f1a0d56749bbacbbb1f74b9ee9c624f541de37fe9ce3f53e135d0ce752fe0e3213f9b034147	3751390
10	validate export client checks	2026-11-17 19:17:06.111425+00	t	\\xfd20ebf3fc3240a3d1add0c718c7e0802a95f3145270cb82cda1d952fa7f34888c40c4c27dcb6832c0eed47ab6655c6a	3845480
11	plc type rules	2026-11-17 19:17:06.116681+00	t	\\xb41bc227cb81d2085c2c11439274650f828efcff805c7d7b3f34e598d44378521e2dc58c5e040f0eaf172c2e0417f854	7747332
12	plc connection	2026-11-17 19:17:06.125753+00	t	\\x68591a72f3649bfce05f49cadc9168afc85079dfdd60c7387e14689ff59e661407848f49a8a0e85ae4470e87e6045d05	14514385
13	machine db number	2026-11-17 19:17:06.15388+00	t	\\xeaee47777d929a3bb22d06866376a44d5862de9a0c81feee7fa8dc9921f366061404a65912d9532be6e264ce59c8bab7	7357532
14	machine db number copy	2026-11-17 19:17:06.162498+00	t	\\xb55528afa13ee2af130501c45ad8b6887c8bf0b2bbb05e02560e0fa20c1c19e6d0902cab307637e14e789b9f89a02c5e	3506226
15	machine db number validate	2026-11-17 19:17:06.167476+00	t	\\xd86bc489b74a5691046516a2828cc666bd0e9ce8abfae2b405e61b78b7680b3b282ead1e16b2146d6ecb3bf00548bbf7	2715005
16	recipe version insert	2026-11-17 19:17:06.171502+00	t	\\x891fd00812746f078ca445aaf0e18dc90c2cebb4c214e20a8c58672bfeb2459de50c17b9387b82c31ea25d13442ed333	4259815
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
21eba560-26c2-4a2d-876a-d73fc338f994	user1	$argon2id$fixture	administrator	t	0	f	\N	2026-11-17 19:17:06.411447+00	2026-11-17 19:17:06.411447+00
ed1fca19-9125-4d12-8dc9-e87f5db2c543	engineer	$argon2id$fake$Seed-password-2024	process_engineer	t	0	f	\N	2026-11-17 19:17:06.51484+00	2026-11-17 19:17:06.51484+00
\.


--
-- Data for Name: audit_chain_head; Type: TABLE DATA; Schema: public; Owner: audit_owner
--

COPY public.audit_chain_head (singleton, last_seq, last_hash) FROM stdin;
t	43	\\x5dc132d8e039511e3c75b0d8c8875fe0cd6a1643333e8ee89000f6ac4594dea1
\.


--
-- Data for Name: audit_log; Type: TABLE DATA; Schema: public; Owner: audit_owner
--

COPY public.audit_log (id, chain_seq, occurred_at, actor_user_id, action, entity_type, entity_id, aggregate_type, aggregate_id, before_state, after_state, request_id, client_ip, session_ref, prev_hash, entry_hash) FROM stdin;
1	1	2026-11-17 19:17:06.588135+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	7746c280-7895-45e4-95fc-146aaba747a9	recipe	7746c280-7895-45e4-95fc-146aaba747a9	\N	{"id": "7746c280-7895-45e4-95fc-146aaba747a9", "name": "SR-NESTED-ARRAY", "created_at": "2026-11-17T19:17:06.588135+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "b7e0f969-50c8-437e-bc5c-1d37008663b1", "updated_at": "2026-11-17T19:17:06.588135+00:00", "plc_platform": "S7_300", "reference_number": "SR-NESTED-ARRAY"}	1c5d3bb5-5997-4096-b4df-8274214bd284	\N	\N	\N	\\x33fe9c8c1f3969afeb89a502f0fa17da54b5358def4bd25854b7726a08f68c6b
2	2	2026-11-17 19:17:06.588135+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	recipe	7746c280-7895-45e4-95fc-146aaba747a9	\N	{"id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d", "number": 1, "status": "draft", "recipe_id": "7746c280-7895-45e4-95fc-146aaba747a9", "created_at": "2026-11-17T19:17:06.588135+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:06.588135+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	1c5d3bb5-5997-4096-b4df-8274214bd284	\N	\N	\\x33fe9c8c1f3969afeb89a502f0fa17da54b5358def4bd25854b7726a08f68c6b	\\x973524623acf2cde43e754c7e60a82076c1a547a09156b2d1acc91ae6ce7e7e6
3	3	2026-11-17 19:17:06.844889+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	DerivedTypeCreated	plc_type	c4803844-dec9-4ea4-8cb0-f9d9bbf41ff5	machine	b7e0f969-50c8-437e-bc5c-1d37008663b1	\N	{"id": "c4803844-dec9-4ea4-8cb0-f9d9bbf41ff5", "kind": "array", "name": null, "revision": 1, "dim_lower": [1], "dim_upper": [3], "platforms": ["S7_300"], "created_at": "2026-11-17T19:17:06.844889+00:00", "leaf_class": null, "machine_id": "b7e0f969-50c8-437e-bc5c-1d37008663b1", "size_bytes": null, "string_length": null, "element_type_id": "5eed0000-0000-4000-8000-000000000002", "supersedes_type_id": null}	b5a109cf-b989-4cc5-8b5f-889b12f0f39c	\N	\N	\\x973524623acf2cde43e754c7e60a82076c1a547a09156b2d1acc91ae6ce7e7e6	\\x959d553e08a0263660d6a879f49a90b6c675914ac6e294ddf0d56032b42a38dc
4	4	2026-11-17 19:17:06.844889+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	DerivedTypeCreated	plc_type	651a353d-3248-4a31-920d-15520d32b746	machine	b7e0f969-50c8-437e-bc5c-1d37008663b1	\N	{"id": "651a353d-3248-4a31-920d-15520d32b746", "kind": "array", "name": null, "revision": 1, "dim_lower": [1], "dim_upper": [2], "platforms": ["S7_300"], "created_at": "2026-11-17T19:17:06.844889+00:00", "leaf_class": null, "machine_id": "b7e0f969-50c8-437e-bc5c-1d37008663b1", "size_bytes": null, "string_length": null, "element_type_id": "c4803844-dec9-4ea4-8cb0-f9d9bbf41ff5", "supersedes_type_id": null}	b5a109cf-b989-4cc5-8b5f-889b12f0f39c	\N	\N	\\x959d553e08a0263660d6a879f49a90b6c675914ac6e294ddf0d56032b42a38dc	\\xf8161d88fbd1580b42fa6288fc8381d108d6144f6ec04e8b4dcbe00161ccf5db
5	5	2026-11-17 19:17:06.844889+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	d418c4e5-3798-407a-8be8-2e1d5459f5a7	recipe	7746c280-7895-45e4-95fc-146aaba747a9	\N	{"id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "tag": "VALUE", "value": [[1, 2, 3], [4, 5, 6]], "limits": [{"leaf_path": "[1][1]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}, {"leaf_path": "[1][2]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}, {"leaf_path": "[1][3]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}, {"leaf_path": "[2][1]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}, {"leaf_path": "[2][2]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}, {"leaf_path": "[2][3]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:06.844889+00:00", "max_length": null, "parameter_id": "d418c4e5-3798-407a-8be8-2e1d5459f5a7", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}], "name_en": "Value", "ordinal": 1, "unit_id": "7778262a-7aa6-4be9-9a34-91664d595a34", "created_at": "2026-11-17T19:17:06.844889+00:00", "s7_address": "DB1.DBB0", "plc_type_id": "651a353d-3248-4a31-920d-15520d32b746", "plc_platform": "S7_300", "translations": [], "parameter_key": "3d77b7a2-5842-4ec1-a0e5-cceb6df84395", "description_en": "Value of the matrix row", "recipe_version_id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d"}	b5a109cf-b989-4cc5-8b5f-889b12f0f39c	\N	\N	\\xf8161d88fbd1580b42fa6288fc8381d108d6144f6ec04e8b4dcbe00161ccf5db	\\xa5cb22b75d2eedcf7f7749fde47ce534e37e6786314f54b7ce4a5c85c18ea995
6	6	2026-11-17 19:17:06.9606+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	recipe	7746c280-7895-45e4-95fc-146aaba747a9	{"id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d", "number": 1, "status": "draft", "recipe_id": "7746c280-7895-45e4-95fc-146aaba747a9", "created_at": "2026-11-17T19:17:06.588135+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:06.844889+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	{"id": "3d0c0d2d-de32-4fec-bd35-3473c428cf2d", "number": 1, "status": "approved", "recipe_id": "7746c280-7895-45e4-95fc-146aaba747a9", "created_at": "2026-11-17T19:17:06.588135+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:06.9606+00:00", "approved_at": "2026-11-17T19:17:06.9606+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_300", "allow_outdated": false, "content_sha256": "ad753cf21ea496f7bfb18668aafb84111b1469f3b43df32d803ee73e748e0826", "based_on_version_id": null}	8845f1de-d245-44fb-9b3a-91e2bf322a90	\N	\N	\\xa5cb22b75d2eedcf7f7749fde47ce534e37e6786314f54b7ce4a5c85c18ea995	\\xfe26a4024b8c5bc09a25bcc72dac0cbf6e850c88ec5176dfb2554da556b486bf
7	7	2026-11-17 19:17:06.984401+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	b13d25f6-8634-481a-a235-35bd825b2f35	recipe	b13d25f6-8634-481a-a235-35bd825b2f35	\N	{"id": "b13d25f6-8634-481a-a235-35bd825b2f35", "name": "SR-ARRAY-BOUND", "created_at": "2026-11-17T19:17:06.984401+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "64f392bb-9857-41a1-8048-3ff29152bc6c", "updated_at": "2026-11-17T19:17:06.984401+00:00", "plc_platform": "S7_300", "reference_number": "SR-ARRAY-BOUND"}	8445bcef-420d-4e49-b7eb-7fee78c01b51	\N	\N	\\xfe26a4024b8c5bc09a25bcc72dac0cbf6e850c88ec5176dfb2554da556b486bf	\\xeb68d95d211c034390a6a46341450bf79035cbed6d8a481af1350a67e10c821a
8	8	2026-11-17 19:17:06.984401+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	recipe	b13d25f6-8634-481a-a235-35bd825b2f35	\N	{"id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe", "number": 1, "status": "draft", "recipe_id": "b13d25f6-8634-481a-a235-35bd825b2f35", "created_at": "2026-11-17T19:17:06.984401+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:06.984401+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	8445bcef-420d-4e49-b7eb-7fee78c01b51	\N	\N	\\xeb68d95d211c034390a6a46341450bf79035cbed6d8a481af1350a67e10c821a	\\x6cae3486e47e167a465d1b1b7859d22d9d3f2f7a504e08de5ea2de26f34b61a3
9	9	2026-11-17 19:17:07.015927+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	DerivedTypeCreated	plc_type	f8c6a2f7-fb7f-4489-91af-9c22dd564762	machine	64f392bb-9857-41a1-8048-3ff29152bc6c	\N	{"id": "f8c6a2f7-fb7f-4489-91af-9c22dd564762", "kind": "array", "name": null, "revision": 1, "dim_lower": [32760], "dim_upper": [32770], "platforms": ["S7_300"], "created_at": "2026-11-17T19:17:07.015927+00:00", "leaf_class": null, "machine_id": "64f392bb-9857-41a1-8048-3ff29152bc6c", "size_bytes": null, "string_length": null, "element_type_id": "5eed0000-0000-4000-8000-000000000002", "supersedes_type_id": null}	349de13c-880a-4404-87cb-495644fb5f09	\N	\N	\\x6cae3486e47e167a465d1b1b7859d22d9d3f2f7a504e08de5ea2de26f34b61a3	\\xf70ede7861a311ad44e1adbcc75879e698ea54b8d45ce30d9a4e54656c2f634f
10	10	2026-11-17 19:17:07.015927+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	06e7cac4-71fc-41e4-a316-af0f3c361fd0	recipe	b13d25f6-8634-481a-a235-35bd825b2f35	\N	{"id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "tag": "VALUE", "value": [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10], "limits": [{"leaf_path": "[32760]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32761]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32762]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32763]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32764]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32765]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32766]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32767]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32768]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32769]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}, {"leaf_path": "[32770]", "max_value": 255, "min_value": 0, "created_at": "2026-11-17T19:17:07.015927+00:00", "max_length": null, "parameter_id": "06e7cac4-71fc-41e4-a316-af0f3c361fd0", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}], "name_en": "Value", "ordinal": 1, "unit_id": "efed59ba-3d26-4bc1-b380-3d6d641cb408", "created_at": "2026-11-17T19:17:07.015927+00:00", "s7_address": "DB1.DBB0", "plc_type_id": "f8c6a2f7-fb7f-4489-91af-9c22dd564762", "plc_platform": "S7_300", "translations": [], "parameter_key": "8bbfe9e9-e2a6-4605-acdc-34b3f7bece34", "description_en": "Value of the matrix row", "recipe_version_id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe"}	349de13c-880a-4404-87cb-495644fb5f09	\N	\N	\\xf70ede7861a311ad44e1adbcc75879e698ea54b8d45ce30d9a4e54656c2f634f	\\x01ad870b9ae7acb4c55d1ccb7915a914a0e111f6976c4287260299efac060d57
11	11	2026-11-17 19:17:07.066072+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	recipe	b13d25f6-8634-481a-a235-35bd825b2f35	{"id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe", "number": 1, "status": "draft", "recipe_id": "b13d25f6-8634-481a-a235-35bd825b2f35", "created_at": "2026-11-17T19:17:06.984401+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.015927+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	{"id": "c9b2ee6e-4bd9-43ac-83de-319fd2a776fe", "number": 1, "status": "approved", "recipe_id": "b13d25f6-8634-481a-a235-35bd825b2f35", "created_at": "2026-11-17T19:17:06.984401+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.066072+00:00", "approved_at": "2026-11-17T19:17:07.066072+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_300", "allow_outdated": false, "content_sha256": "882552ae17279c6c1eed8a658bac4113d47bcd5d2a4b6261dd13537a3d195896", "based_on_version_id": null}	75ccd9ab-241e-4880-95f6-97b5f902a198	\N	\N	\\x01ad870b9ae7acb4c55d1ccb7915a914a0e111f6976c4287260299efac060d57	\\xbe88561e2ee2b6df27cc3321f52000de2ccb3d6a80c500afd6b3cb25e7fd5ac7
14	14	2026-11-17 19:17:07.128319+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	99643d9d-4022-4520-87ee-5d3fe9c79db3	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "99643d9d-4022-4520-87ee-5d3fe9c79db3", "kind": "struct", "name": "ST_L7", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.128319+00:00", "member_name": "next", "member_type_id": "2679212d-b408-4a22-88fe-1021e3a021f1", "struct_type_id": "99643d9d-4022-4520-87ee-5d3fe9c79db3"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.128319+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	e27312b5-bf31-4f18-afb6-538bbe74b68e	\N	\N	\\x05529b7802f9abe9d50b413f7358d9fff29a46cfeb3e5451e890bb55a797d0d4	\\x009de9d5ffd3428ae3365982af611dd2a8782efe642a80220b658da94d0e8376
17	17	2026-11-17 19:17:07.174297+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	c6784913-77af-4bf7-bf11-2a4fa6ad5abe	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "c6784913-77af-4bf7-bf11-2a4fa6ad5abe", "kind": "struct", "name": "ST_L4", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.174297+00:00", "member_name": "next", "member_type_id": "3d44448a-f4fa-42f7-8a48-45449d71e190", "struct_type_id": "c6784913-77af-4bf7-bf11-2a4fa6ad5abe"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.174297+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	8bd8237e-b06a-4b5f-98d2-790dda00e610	\N	\N	\\x0f04232d92f1cbaa9fd69bfe5a02370e89bb78bdb11e0102cf6d6689fcd8f949	\\x28e1f34dda9da4622dd4f8b97c90268c377a6dd032c6ef6f758ea444f4f0814d
20	20	2026-11-17 19:17:07.223878+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	a407158f-1cb0-4a58-bd05-909cbc642986	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "a407158f-1cb0-4a58-bd05-909cbc642986", "kind": "struct", "name": "ST_Top", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.223878+00:00", "member_name": "next", "member_type_id": "b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea", "struct_type_id": "a407158f-1cb0-4a58-bd05-909cbc642986"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.223878+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	1ce2a489-08c7-43f0-b3fd-e91e74d44464	\N	\N	\\xaeda4751cb32d7b50d640fe7d3ffa3348f56918b40bc69b453801dbf1bcd419a	\\x62a9bcfdaeac13d99babaf7d5b19b13f48be450f3f2f1c4bbb74d2b1d15c84c0
29	29	2026-11-17 19:17:07.381188+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	12951f8d-047f-4080-97f7-3f8d32cb2562	recipe	12951f8d-047f-4080-97f7-3f8d32cb2562	\N	{"id": "12951f8d-047f-4080-97f7-3f8d32cb2562", "name": "SR-WSTRING", "created_at": "2026-11-17T19:17:07.381188+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0", "updated_at": "2026-11-17T19:17:07.381188+00:00", "plc_platform": "S7_1200", "reference_number": "SR-WSTRING"}	f6397d93-20d9-4026-a672-b01c4bbc130e	\N	\N	\\x8eee1ab2bb119edfd05c06c36d03920287d7b15a0a3d00adfbe0941dd28c5882	\\x48adedae354d0e6ec087013ade613037b2db677f46ddf8e4e4f5ffd6c8a57fe3
30	30	2026-11-17 19:17:07.381188+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	4a1f7a87-1618-4036-988f-4b789cc93269	recipe	12951f8d-047f-4080-97f7-3f8d32cb2562	\N	{"id": "4a1f7a87-1618-4036-988f-4b789cc93269", "number": 1, "status": "draft", "recipe_id": "12951f8d-047f-4080-97f7-3f8d32cb2562", "created_at": "2026-11-17T19:17:07.381188+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.381188+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_1200", "content_sha256": null, "based_on_version_id": null}	f6397d93-20d9-4026-a672-b01c4bbc130e	\N	\N	\\x48adedae354d0e6ec087013ade613037b2db677f46ddf8e4e4f5ffd6c8a57fe3	\\xb424d742da64f409734d1ad612850c6e10becb3e1b92d6a07293e8e9c2d74aad
31	31	2026-11-17 19:17:07.406198+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	DerivedTypeCreated	plc_type	37af5eff-d6c5-4669-866b-daeea731d8a8	machine	562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0	\N	{"id": "37af5eff-d6c5-4669-866b-daeea731d8a8", "kind": "primitive", "name": "WSTRING", "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1200"], "created_at": "2026-11-17T19:17:07.406198+00:00", "leaf_class": "string", "machine_id": "562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0", "size_bytes": 24, "string_length": 10, "element_type_id": null, "supersedes_type_id": null}	e455647a-6113-4b98-84fc-77b281cf9a34	\N	\N	\\xb424d742da64f409734d1ad612850c6e10becb3e1b92d6a07293e8e9c2d74aad	\\xf4f4342a4725b1c65ee99dbb96ca69b4c78d4840a9d139acdb3af3a49adac0e2
32	32	2026-11-17 19:17:07.406198+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	b94ac947-65b7-4e99-bcbb-15ca00332b1b	recipe	12951f8d-047f-4080-97f7-3f8d32cb2562	\N	{"id": "b94ac947-65b7-4e99-bcbb-15ca00332b1b", "tag": "VALUE", "value": "A퀀", "limits": [{"leaf_path": "", "max_value": null, "min_value": null, "created_at": "2026-11-17T19:17:07.406198+00:00", "max_length": 10, "parameter_id": "b94ac947-65b7-4e99-bcbb-15ca00332b1b", "recipe_version_id": "4a1f7a87-1618-4036-988f-4b789cc93269"}], "name_en": "Value", "ordinal": 1, "unit_id": "a04b42cd-a556-4ad9-853d-f9e5843abf1f", "created_at": "2026-11-17T19:17:07.406198+00:00", "s7_address": null, "plc_type_id": "37af5eff-d6c5-4669-866b-daeea731d8a8", "plc_platform": "S7_1200", "translations": [], "parameter_key": "92a35f46-dd39-44e1-829d-68498c868005", "description_en": "Value of the matrix row", "recipe_version_id": "4a1f7a87-1618-4036-988f-4b789cc93269"}	e455647a-6113-4b98-84fc-77b281cf9a34	\N	\N	\\xf4f4342a4725b1c65ee99dbb96ca69b4c78d4840a9d139acdb3af3a49adac0e2	\\x4de5213ab99420e525c9c982e78d255c05fb79e20c68b98d625df1b519767669
37	37	2026-11-17 19:17:07.481158+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	300612e6-82d9-487f-a8da-e9c58bf72a2b	recipe	a756d4fa-b224-42b6-b52c-2a5081ba487c	\N	{"id": "300612e6-82d9-487f-a8da-e9c58bf72a2b", "tag": "VALUE", "value": {"setpoint": 1.5}, "limits": [{"leaf_path": ".setpoint", "max_value": 10, "min_value": 0, "created_at": "2026-11-17T19:17:07.481158+00:00", "max_length": null, "parameter_id": "300612e6-82d9-487f-a8da-e9c58bf72a2b", "recipe_version_id": "4fba22de-81b4-4bd1-b253-5bd3e2fce25c"}], "name_en": "Value", "ordinal": 1, "unit_id": "bfe75b2f-34a4-47bd-80d1-75cb959ed356", "created_at": "2026-11-17T19:17:07.481158+00:00", "s7_address": null, "plc_type_id": "b897d462-1b9f-4169-99fb-d27b8c7c6d94", "plc_platform": "S7_1500", "translations": [], "parameter_key": "528ae44e-2470-4d6d-a66a-d55dafe1b6af", "description_en": "Value of the matrix row", "recipe_version_id": "4fba22de-81b4-4bd1-b253-5bd3e2fce25c"}	13bf530e-4427-4c79-9017-92e1d7cb3236	\N	\N	\\xbb1802ad89fc94010e35860a00b3a678dfd58bf7a3353f3600abd232122919ff	\\x037def02efeb6103a51d49317ca980fdf74242f8dabda091d087230dcb051748
12	12	2026-11-17 19:17:07.090733+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	f3a0725a-8c43-4d08-84c0-5123017f3c99	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "f3a0725a-8c43-4d08-84c0-5123017f3c99", "kind": "struct", "name": "ST_L9", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.090733+00:00", "member_name": "leaf", "member_type_id": "5eed0000-0000-4000-8000-00000000000a", "struct_type_id": "f3a0725a-8c43-4d08-84c0-5123017f3c99"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.090733+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	30c2076d-ced9-41ed-9022-19351bd8106b	\N	\N	\\xbe88561e2ee2b6df27cc3321f52000de2ccb3d6a80c500afd6b3cb25e7fd5ac7	\\x7d964c255780ce00001889fcd562da5b1ae08b6c3476124670ff919ea315cd81
15	15	2026-11-17 19:17:07.146117+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	7cf94785-7829-469a-b3d9-566a6f928362	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "7cf94785-7829-469a-b3d9-566a6f928362", "kind": "struct", "name": "ST_L6", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.146117+00:00", "member_name": "next", "member_type_id": "99643d9d-4022-4520-87ee-5d3fe9c79db3", "struct_type_id": "7cf94785-7829-469a-b3d9-566a6f928362"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.146117+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	a80d481f-c699-4fd3-a631-711081161dec	\N	\N	\\x009de9d5ffd3428ae3365982af611dd2a8782efe642a80220b658da94d0e8376	\\x61efb490935c952b2124867a41ad39c2beda6f8b5c56aa808d3b2003ce9755e7
18	18	2026-11-17 19:17:07.188094+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	12aadb7c-5967-41f2-9694-a90ed88e9162	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "12aadb7c-5967-41f2-9694-a90ed88e9162", "kind": "struct", "name": "ST_L3", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.188094+00:00", "member_name": "next", "member_type_id": "c6784913-77af-4bf7-bf11-2a4fa6ad5abe", "struct_type_id": "12aadb7c-5967-41f2-9694-a90ed88e9162"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.188094+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	255b9efc-5386-49ce-8be8-dd3e80c81538	\N	\N	\\x28e1f34dda9da4622dd4f8b97c90268c377a6dd032c6ef6f758ea444f4f0814d	\\x7d6c87fc3f344de838d68ce3e9a8f6a992af6145f59d5a0840f3da387841ed37
21	21	2026-11-17 19:17:07.239144+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	3b5fc113-6899-4c11-bf64-e1f71e385c4e	recipe	3b5fc113-6899-4c11-bf64-e1f71e385c4e	\N	{"id": "3b5fc113-6899-4c11-bf64-e1f71e385c4e", "name": "SR-NESTING", "created_at": "2026-11-17T19:17:07.239144+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "updated_at": "2026-11-17T19:17:07.239144+00:00", "plc_platform": "S7_1500", "reference_number": "SR-NESTING"}	f7d9e3f9-3b52-4039-8438-134327594f2a	\N	\N	\\x62a9bcfdaeac13d99babaf7d5b19b13f48be450f3f2f1c4bbb74d2b1d15c84c0	\\xe50b0ce352763b7ea4c7fd255ca6d2c73689838de5cdd748c1532baf2c84b113
22	22	2026-11-17 19:17:07.239144+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	7c96e26f-c416-4ba7-baf9-a279186b6e9a	recipe	3b5fc113-6899-4c11-bf64-e1f71e385c4e	\N	{"id": "7c96e26f-c416-4ba7-baf9-a279186b6e9a", "number": 1, "status": "draft", "recipe_id": "3b5fc113-6899-4c11-bf64-e1f71e385c4e", "created_at": "2026-11-17T19:17:07.239144+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.239144+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	f7d9e3f9-3b52-4039-8438-134327594f2a	\N	\N	\\xe50b0ce352763b7ea4c7fd255ca6d2c73689838de5cdd748c1532baf2c84b113	\\x715512adda8a0a25c8146dc7fb98e230f530e39c53054a7af6cb596b2c78c4d8
23	23	2026-11-17 19:17:07.256563+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	b04f38cd-b399-4084-9555-a3d9e7099c9f	recipe	3b5fc113-6899-4c11-bf64-e1f71e385c4e	\N	{"id": "b04f38cd-b399-4084-9555-a3d9e7099c9f", "tag": "VALUE", "value": {"next": {"next": {"next": {"next": {"next": {"next": {"next": {"next": {"leaf": 7}}}}}}}}}, "limits": [{"leaf_path": ".next.next.next.next.next.next.next.next.leaf", "max_value": 100, "min_value": 0, "created_at": "2026-11-17T19:17:07.256563+00:00", "max_length": null, "parameter_id": "b04f38cd-b399-4084-9555-a3d9e7099c9f", "recipe_version_id": "7c96e26f-c416-4ba7-baf9-a279186b6e9a"}], "name_en": "Value", "ordinal": 1, "unit_id": "0073eb38-c322-4c57-826c-54ead67481fd", "created_at": "2026-11-17T19:17:07.256563+00:00", "s7_address": null, "plc_type_id": "a407158f-1cb0-4a58-bd05-909cbc642986", "plc_platform": "S7_1500", "translations": [], "parameter_key": "d0a96797-8e3e-4857-b8c8-9cddeb3bc704", "description_en": "Value of the matrix row", "recipe_version_id": "7c96e26f-c416-4ba7-baf9-a279186b6e9a"}	8148c33c-9db1-43b7-8dce-0aa10b17499a	\N	\N	\\x715512adda8a0a25c8146dc7fb98e230f530e39c53054a7af6cb596b2c78c4d8	\\xe13aef8d5d28bc87d2fe92bc81036eee6d32f27492af231760a9a51a18c8ec1c
24	24	2026-11-17 19:17:07.302116+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	7c96e26f-c416-4ba7-baf9-a279186b6e9a	recipe	3b5fc113-6899-4c11-bf64-e1f71e385c4e	{"id": "7c96e26f-c416-4ba7-baf9-a279186b6e9a", "number": 1, "status": "draft", "recipe_id": "3b5fc113-6899-4c11-bf64-e1f71e385c4e", "created_at": "2026-11-17T19:17:07.239144+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.256563+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	{"id": "7c96e26f-c416-4ba7-baf9-a279186b6e9a", "number": 1, "status": "approved", "recipe_id": "3b5fc113-6899-4c11-bf64-e1f71e385c4e", "created_at": "2026-11-17T19:17:07.239144+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.302116+00:00", "approved_at": "2026-11-17T19:17:07.302116+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_1500", "allow_outdated": false, "content_sha256": "f7ee5004c0fb529fb4ac92be7492ef2a9ca20c6145f9b1d269e3d0b59d71995b", "based_on_version_id": null}	e0b87377-5e70-4c50-a81d-9a593a85d21b	\N	\N	\\xe13aef8d5d28bc87d2fe92bc81036eee6d32f27492af231760a9a51a18c8ec1c	\\x02f333f777f768365f5fb95484dc53ec6a40926be39e7b2802e8ee1fd1f64246
34	34	2026-11-17 19:17:07.458193+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	b897d462-1b9f-4169-99fb-d27b8c7c6d94	machine	0993bcf2-aa52-461d-b73f-a4dfc65016c3	\N	{"id": "b897d462-1b9f-4169-99fb-d27b8c7c6d94", "kind": "struct", "name": "PID_Compact", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.458193+00:00", "member_name": "setpoint", "member_type_id": "5eed0000-0000-4000-8000-00000000000e", "struct_type_id": "b897d462-1b9f-4169-99fb-d27b8c7c6d94"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.458193+00:00", "leaf_class": null, "machine_id": "0993bcf2-aa52-461d-b73f-a4dfc65016c3", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	7daaeee3-2ff6-4286-abc0-7409ddf6e02d	\N	\N	\\x71b5f7bdce5f1a0d7a76729a696b33fc01d50bb8581b3a59ee7bcdb6fcc869d7	\\xc930cfba2150bc64973d263f869d92e335be4f37fe43056b90632af12da33bd2
13	13	2026-11-17 19:17:07.113958+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	2679212d-b408-4a22-88fe-1021e3a021f1	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "2679212d-b408-4a22-88fe-1021e3a021f1", "kind": "struct", "name": "ST_L8", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.113958+00:00", "member_name": "next", "member_type_id": "f3a0725a-8c43-4d08-84c0-5123017f3c99", "struct_type_id": "2679212d-b408-4a22-88fe-1021e3a021f1"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.113958+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	1db015fa-a4de-40a1-9cf9-d739a36a77a2	\N	\N	\\x7d964c255780ce00001889fcd562da5b1ae08b6c3476124670ff919ea315cd81	\\x05529b7802f9abe9d50b413f7358d9fff29a46cfeb3e5451e890bb55a797d0d4
16	16	2026-11-17 19:17:07.162097+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	3d44448a-f4fa-42f7-8a48-45449d71e190	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "3d44448a-f4fa-42f7-8a48-45449d71e190", "kind": "struct", "name": "ST_L5", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.162097+00:00", "member_name": "next", "member_type_id": "7cf94785-7829-469a-b3d9-566a6f928362", "struct_type_id": "3d44448a-f4fa-42f7-8a48-45449d71e190"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.162097+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	52222ca4-8f80-4c85-a567-506dc9762790	\N	\N	\\x61efb490935c952b2124867a41ad39c2beda6f8b5c56aa808d3b2003ce9755e7	\\x0f04232d92f1cbaa9fd69bfe5a02370e89bb78bdb11e0102cf6d6689fcd8f949
19	19	2026-11-17 19:17:07.203369+00	21eba560-26c2-4a2d-876a-d73fc338f994	UdtDefined	plc_type	b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea	machine	c2496c3a-d89e-4d77-92ea-c83075a49d02	\N	{"id": "b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea", "kind": "struct", "name": "ST_L2", "members": [{"ordinal": 1, "created_at": "2026-11-17T19:17:07.203369+00:00", "member_name": "next", "member_type_id": "12aadb7c-5967-41f2-9694-a90ed88e9162", "struct_type_id": "b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea"}], "revision": 1, "dim_lower": null, "dim_upper": null, "platforms": ["S7_1500"], "created_at": "2026-11-17T19:17:07.203369+00:00", "leaf_class": null, "machine_id": "c2496c3a-d89e-4d77-92ea-c83075a49d02", "size_bytes": null, "string_length": null, "element_type_id": null, "supersedes_type_id": null}	8d5d5bb2-e7fa-4a35-9ee7-3d67dcce51fc	\N	\N	\\x7d6c87fc3f344de838d68ce3e9a8f6a992af6145f59d5a0840f3da387841ed37	\\xaeda4751cb32d7b50d640fe7d3ffa3348f56918b40bc69b453801dbf1bcd419a
25	25	2026-11-17 19:17:07.31769+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	d1e40cc3-76f4-441f-a957-6a0a447e9f66	recipe	d1e40cc3-76f4-441f-a957-6a0a447e9f66	\N	{"id": "d1e40cc3-76f4-441f-a957-6a0a447e9f66", "name": "SR-WCHAR", "created_at": "2026-11-17T19:17:07.31769+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "859d6b1e-0dbc-49f5-bf5e-ecc956638666", "updated_at": "2026-11-17T19:17:07.31769+00:00", "plc_platform": "S7_1500", "reference_number": "SR-WCHAR"}	aadf293e-2a40-466e-9780-98e611035d68	\N	\N	\\x02f333f777f768365f5fb95484dc53ec6a40926be39e7b2802e8ee1fd1f64246	\\x40fd4efc90b7f6bb8d3ea8f3a50954550f2f8df1aa8865b39cb8bb1561e71b5f
26	26	2026-11-17 19:17:07.31769+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	bfd86327-8954-4a67-ad61-3f5b6e3f7f0e	recipe	d1e40cc3-76f4-441f-a957-6a0a447e9f66	\N	{"id": "bfd86327-8954-4a67-ad61-3f5b6e3f7f0e", "number": 1, "status": "draft", "recipe_id": "d1e40cc3-76f4-441f-a957-6a0a447e9f66", "created_at": "2026-11-17T19:17:07.31769+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.31769+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	aadf293e-2a40-466e-9780-98e611035d68	\N	\N	\\x40fd4efc90b7f6bb8d3ea8f3a50954550f2f8df1aa8865b39cb8bb1561e71b5f	\\x84efb767482f7799c6de8181346397a10664984e8319ab38bb473697a7687fa6
27	27	2026-11-17 19:17:07.333042+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	1e1d7d3e-e024-4b16-af79-0bb2f737f294	recipe	d1e40cc3-76f4-441f-a957-6a0a447e9f66	\N	{"id": "1e1d7d3e-e024-4b16-af79-0bb2f737f294", "tag": "VALUE", "value": "", "limits": [], "name_en": "Value", "ordinal": 1, "unit_id": "b6198aa7-086c-4429-8a4b-f2802a5f56bb", "created_at": "2026-11-17T19:17:07.333042+00:00", "s7_address": null, "plc_type_id": "5eed0000-0000-4000-8000-000000000013", "plc_platform": "S7_1500", "translations": [], "parameter_key": "543cb7a1-b00e-4677-bb8d-468ff29766d9", "description_en": "Value of the matrix row", "recipe_version_id": "bfd86327-8954-4a67-ad61-3f5b6e3f7f0e"}	2d5066bd-cb03-438d-affa-bf6862400d84	\N	\N	\\x84efb767482f7799c6de8181346397a10664984e8319ab38bb473697a7687fa6	\\xeb732b1f62d9f2f441711c76c8fde0574ef9e2a408c6ffe571a22b1e156c1ed4
28	28	2026-11-17 19:17:07.365902+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	bfd86327-8954-4a67-ad61-3f5b6e3f7f0e	recipe	d1e40cc3-76f4-441f-a957-6a0a447e9f66	{"id": "bfd86327-8954-4a67-ad61-3f5b6e3f7f0e", "number": 1, "status": "draft", "recipe_id": "d1e40cc3-76f4-441f-a957-6a0a447e9f66", "created_at": "2026-11-17T19:17:07.31769+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.333042+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	{"id": "bfd86327-8954-4a67-ad61-3f5b6e3f7f0e", "number": 1, "status": "approved", "recipe_id": "d1e40cc3-76f4-441f-a957-6a0a447e9f66", "created_at": "2026-11-17T19:17:07.31769+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.365902+00:00", "approved_at": "2026-11-17T19:17:07.365902+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_1500", "allow_outdated": false, "content_sha256": "258c318dd19a04ac06bbc6e9fef2a43acfaf30117f6701dc12cfe0a777b06ed7", "based_on_version_id": null}	0deda1bb-79a5-4e3a-a5b7-24288d2aef5d	\N	\N	\\xeb732b1f62d9f2f441711c76c8fde0574ef9e2a408c6ffe571a22b1e156c1ed4	\\x8eee1ab2bb119edfd05c06c36d03920287d7b15a0a3d00adfbe0941dd28c5882
35	35	2026-11-17 19:17:07.466984+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	a756d4fa-b224-42b6-b52c-2a5081ba487c	recipe	a756d4fa-b224-42b6-b52c-2a5081ba487c	\N	{"id": "a756d4fa-b224-42b6-b52c-2a5081ba487c", "name": "SR-RESERVED-NAME", "created_at": "2026-11-17T19:17:07.466984+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "0993bcf2-aa52-461d-b73f-a4dfc65016c3", "updated_at": "2026-11-17T19:17:07.466984+00:00", "plc_platform": "S7_1500", "reference_number": "SR-RESERVED-NAME"}	3aebb714-bc4b-4668-b253-79902c4ee5a6	\N	\N	\\xc930cfba2150bc64973d263f869d92e335be4f37fe43056b90632af12da33bd2	\\xb032870db662691ca664939047b0b89a982c9e63fea20fa9c6c5eaf02fe1e643
36	36	2026-11-17 19:17:07.466984+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	4fba22de-81b4-4bd1-b253-5bd3e2fce25c	recipe	a756d4fa-b224-42b6-b52c-2a5081ba487c	\N	{"id": "4fba22de-81b4-4bd1-b253-5bd3e2fce25c", "number": 1, "status": "draft", "recipe_id": "a756d4fa-b224-42b6-b52c-2a5081ba487c", "created_at": "2026-11-17T19:17:07.466984+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.466984+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	3aebb714-bc4b-4668-b253-79902c4ee5a6	\N	\N	\\xb032870db662691ca664939047b0b89a982c9e63fea20fa9c6c5eaf02fe1e643	\\xbb1802ad89fc94010e35860a00b3a678dfd58bf7a3353f3600abd232122919ff
33	33	2026-11-17 19:17:07.443443+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	4a1f7a87-1618-4036-988f-4b789cc93269	recipe	12951f8d-047f-4080-97f7-3f8d32cb2562	{"id": "4a1f7a87-1618-4036-988f-4b789cc93269", "number": 1, "status": "draft", "recipe_id": "12951f8d-047f-4080-97f7-3f8d32cb2562", "created_at": "2026-11-17T19:17:07.381188+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.406198+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_1200", "content_sha256": null, "based_on_version_id": null}	{"id": "4a1f7a87-1618-4036-988f-4b789cc93269", "number": 1, "status": "approved", "recipe_id": "12951f8d-047f-4080-97f7-3f8d32cb2562", "created_at": "2026-11-17T19:17:07.381188+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.443443+00:00", "approved_at": "2026-11-17T19:17:07.443443+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_1200", "allow_outdated": false, "content_sha256": "332f30c899a01c7879d219b8d9b1e7e5c88e0636d61526bc5852f58938a41301", "based_on_version_id": null}	20a79bab-77ec-40f9-9328-912812855681	\N	\N	\\x4de5213ab99420e525c9c982e78d255c05fb79e20c68b98d625df1b519767669	\\x71b5f7bdce5f1a0d7a76729a696b33fc01d50bb8581b3a59ee7bcdb6fcc869d7
39	39	2026-11-17 19:17:07.532941+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	RecipeCreated	recipe	60cd87ab-ef7c-47a8-b9a7-21d88b373175	recipe	60cd87ab-ef7c-47a8-b9a7-21d88b373175	\N	{"id": "60cd87ab-ef7c-47a8-b9a7-21d88b373175", "name": "SR-BOOL-3D", "created_at": "2026-11-17T19:17:07.532941+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "machine_id": "6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1", "updated_at": "2026-11-17T19:17:07.532941+00:00", "plc_platform": "S7_300", "reference_number": "SR-BOOL-3D"}	c4b0c620-110f-4d64-8488-33838039b1e3	\N	\N	\\x0ff84dadbdec4c7ef1e71bdc93b55c50a75035ee0c6de8c98914abad3d64265f	\\xb1edd93120b9110164698a119048287872091d7a13feb35b6f4a014050975ab4
40	40	2026-11-17 19:17:07.532941+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	VersionCreated	recipe_version	3537b054-3917-4cb3-bcfa-f5f84177f4af	recipe	60cd87ab-ef7c-47a8-b9a7-21d88b373175	\N	{"id": "3537b054-3917-4cb3-bcfa-f5f84177f4af", "number": 1, "status": "draft", "recipe_id": "60cd87ab-ef7c-47a8-b9a7-21d88b373175", "created_at": "2026-11-17T19:17:07.532941+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.532941+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 1, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	c4b0c620-110f-4d64-8488-33838039b1e3	\N	\N	\\xb1edd93120b9110164698a119048287872091d7a13feb35b6f4a014050975ab4	\\x768101a1a5916a2398dca6c11883121281fe32a5c08bc4cacbf307ce84739b36
41	41	2026-11-17 19:17:07.545795+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	DerivedTypeCreated	plc_type	89014950-9fab-4031-a2d4-f9ebd137fe3e	machine	6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1	\N	{"id": "89014950-9fab-4031-a2d4-f9ebd137fe3e", "kind": "array", "name": null, "revision": 1, "dim_lower": [1, 1, 1], "dim_upper": [2, 2, 2], "platforms": ["S7_300"], "created_at": "2026-11-17T19:17:07.545795+00:00", "leaf_class": null, "machine_id": "6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1", "size_bytes": null, "string_length": null, "element_type_id": "5eed0000-0000-4000-8000-000000000001", "supersedes_type_id": null}	780053e0-4989-4b74-b41e-26b70b9df376	\N	\N	\\x768101a1a5916a2398dca6c11883121281fe32a5c08bc4cacbf307ce84739b36	\\x8931b1d6f28fd1dd21f152b467f2a091cf8b37b061a57bddc8c0cc3164c3de57
42	42	2026-11-17 19:17:07.545795+00	ed1fca19-9125-4d12-8dc9-e87f5db2c543	ParameterAdded	parameter	f4b60f0d-c1a7-403c-ab10-f054d34aaab7	recipe	60cd87ab-ef7c-47a8-b9a7-21d88b373175	\N	{"id": "f4b60f0d-c1a7-403c-ab10-f054d34aaab7", "tag": "VALUE", "value": [[[true, false], [false, true]], [[true, true], [false, false]]], "limits": [], "name_en": "Value", "ordinal": 1, "unit_id": "91d9bbb2-9515-4128-a251-af5449a7a641", "created_at": "2026-11-17T19:17:07.545795+00:00", "s7_address": "DB1.DBB0", "plc_type_id": "89014950-9fab-4031-a2d4-f9ebd137fe3e", "plc_platform": "S7_300", "translations": [], "parameter_key": "29f51f73-d787-4a18-8546-7c5368dda76c", "description_en": "Value of the matrix row", "recipe_version_id": "3537b054-3917-4cb3-bcfa-f5f84177f4af"}	780053e0-4989-4b74-b41e-26b70b9df376	\N	\N	\\x8931b1d6f28fd1dd21f152b467f2a091cf8b37b061a57bddc8c0cc3164c3de57	\\x62df9e86908742680d6fb0fc391ab1dc1b5044bcc41443e3ffabf19416a73fcf
43	43	2026-11-17 19:17:07.58851+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	3537b054-3917-4cb3-bcfa-f5f84177f4af	recipe	60cd87ab-ef7c-47a8-b9a7-21d88b373175	{"id": "3537b054-3917-4cb3-bcfa-f5f84177f4af", "number": 1, "status": "draft", "recipe_id": "60cd87ab-ef7c-47a8-b9a7-21d88b373175", "created_at": "2026-11-17T19:17:07.532941+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.545795+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_300", "content_sha256": null, "based_on_version_id": null}	{"id": "3537b054-3917-4cb3-bcfa-f5f84177f4af", "number": 1, "status": "approved", "recipe_id": "60cd87ab-ef7c-47a8-b9a7-21d88b373175", "created_at": "2026-11-17T19:17:07.532941+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.58851+00:00", "approved_at": "2026-11-17T19:17:07.58851+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_300", "allow_outdated": false, "content_sha256": "9bc683b7ebf7f51d60e296ce820addf96cf3b22f293ecc846ef5e36362cc02ed", "based_on_version_id": null}	c4ababb9-736b-4458-b0fa-577ae6d877c0	\N	\N	\\x62df9e86908742680d6fb0fc391ab1dc1b5044bcc41443e3ffabf19416a73fcf	\\x5dc132d8e039511e3c75b0d8c8875fe0cd6a1643333e8ee89000f6ac4594dea1
38	38	2026-11-17 19:17:07.519391+00	21eba560-26c2-4a2d-876a-d73fc338f994	VersionApproved	recipe_version	4fba22de-81b4-4bd1-b253-5bd3e2fce25c	recipe	a756d4fa-b224-42b6-b52c-2a5081ba487c	{"id": "4fba22de-81b4-4bd1-b253-5bd3e2fce25c", "number": 1, "status": "draft", "recipe_id": "a756d4fa-b224-42b6-b52c-2a5081ba487c", "created_at": "2026-11-17T19:17:07.466984+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.481158+00:00", "approved_at": null, "approved_by": null, "change_note": "Stored row of the matrix", "row_version": 2, "plc_platform": "S7_1500", "content_sha256": null, "based_on_version_id": null}	{"id": "4fba22de-81b4-4bd1-b253-5bd3e2fce25c", "number": 1, "status": "approved", "recipe_id": "a756d4fa-b224-42b6-b52c-2a5081ba487c", "created_at": "2026-11-17T19:17:07.466984+00:00", "created_by": "ed1fca19-9125-4d12-8dc9-e87f5db2c543", "deleted_at": null, "retired_at": null, "updated_at": "2026-11-17T19:17:07.519391+00:00", "approved_at": "2026-11-17T19:17:07.519391+00:00", "approved_by": "21eba560-26c2-4a2d-876a-d73fc338f994", "change_note": "Stored row of the matrix", "row_version": 3, "plc_platform": "S7_1500", "allow_outdated": false, "content_sha256": "e4885a52bb79bd0dd775bc66daedb96406256658c3d8142aedf5eca5833a80fc", "based_on_version_id": null}	0ca45fe5-08e1-4ac9-b06b-a5a3f76b62ed	\N	\N	\\x037def02efeb6103a51d49317ca980fdf74242f8dabda091d087230dcb051748	\\x0ff84dadbdec4c7ef1e71bdc93b55c50a75035ee0c6de8c98914abad3d64265f
\.


--
-- Data for Name: draft_editor; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.draft_editor (recipe_version_id, user_id, first_edit_at) FROM stdin;
3d0c0d2d-de32-4fec-bd35-3473c428cf2d	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:06.844889+00
c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.015927+00
7c96e26f-c416-4ba7-baf9-a279186b6e9a	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.256563+00
bfd86327-8954-4a67-ad61-3f5b6e3f7f0e	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.333042+00
4a1f7a87-1618-4036-988f-4b789cc93269	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.406198+00
4fba22de-81b4-4bd1-b253-5bd3e2fce25c	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.481158+00
3537b054-3917-4cb3-bcfa-f5f84177f4af	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.545795+00
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
b7e0f969-50c8-437e-bc5c-1d37008663b1	M2	machine	S7_300	\N	2026-11-17 19:17:06.519235+00	2026-11-17 19:17:06.519235+00	1
64f392bb-9857-41a1-8048-3ff29152bc6c	M3	machine	S7_300	\N	2026-11-17 19:17:06.973919+00	2026-11-17 19:17:06.973919+00	1
c2496c3a-d89e-4d77-92ea-c83075a49d02	M4	machine	S7_1500	\N	2026-11-17 19:17:07.083035+00	2026-11-17 19:17:07.083035+00	1
859d6b1e-0dbc-49f5-bf5e-ecc956638666	M5	machine	S7_1500	\N	2026-11-17 19:17:07.312819+00	2026-11-17 19:17:07.312819+00	1
562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0	M6	machine	S7_1200	\N	2026-11-17 19:17:07.373672+00	2026-11-17 19:17:07.373672+00	1
0993bcf2-aa52-461d-b73f-a4dfc65016c3	M7	machine	S7_1500	\N	2026-11-17 19:17:07.451173+00	2026-11-17 19:17:07.451173+00	1
6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1	M8	machine	S7_300	\N	2026-11-17 19:17:07.527298+00	2026-11-17 19:17:07.527298+00	1
\.


--
-- Data for Name: parameter; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.parameter (id, recipe_version_id, plc_platform, parameter_key, unit_id, tag, name_en, description_en, plc_type_id, s7_address, value, ordinal, created_at) FROM stdin;
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	S7_300	3d77b7a2-5842-4ec1-a0e5-cceb6df84395	7778262a-7aa6-4be9-9a34-91664d595a34	VALUE	Value	Value of the matrix row	651a353d-3248-4a31-920d-15520d32b746	DB1.DBB0	[[1, 2, 3], [4, 5, 6]]	1	2026-11-17 19:17:06.844889+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	S7_300	8bbfe9e9-e2a6-4605-acdc-34b3f7bece34	efed59ba-3d26-4bc1-b380-3d6d641cb408	VALUE	Value	Value of the matrix row	f8c6a2f7-fb7f-4489-91af-9c22dd564762	DB1.DBB0	[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]	1	2026-11-17 19:17:07.015927+00
b04f38cd-b399-4084-9555-a3d9e7099c9f	7c96e26f-c416-4ba7-baf9-a279186b6e9a	S7_1500	d0a96797-8e3e-4857-b8c8-9cddeb3bc704	0073eb38-c322-4c57-826c-54ead67481fd	VALUE	Value	Value of the matrix row	a407158f-1cb0-4a58-bd05-909cbc642986	\N	{"next": {"next": {"next": {"next": {"next": {"next": {"next": {"next": {"leaf": 7}}}}}}}}}	1	2026-11-17 19:17:07.256563+00
1e1d7d3e-e024-4b16-af79-0bb2f737f294	bfd86327-8954-4a67-ad61-3f5b6e3f7f0e	S7_1500	543cb7a1-b00e-4677-bb8d-468ff29766d9	b6198aa7-086c-4429-8a4b-f2802a5f56bb	VALUE	Value	Value of the matrix row	5eed0000-0000-4000-8000-000000000013	\N	""	1	2026-11-17 19:17:07.333042+00
b94ac947-65b7-4e99-bcbb-15ca00332b1b	4a1f7a87-1618-4036-988f-4b789cc93269	S7_1200	92a35f46-dd39-44e1-829d-68498c868005	a04b42cd-a556-4ad9-853d-f9e5843abf1f	VALUE	Value	Value of the matrix row	37af5eff-d6c5-4669-866b-daeea731d8a8	\N	"A퀀"	1	2026-11-17 19:17:07.406198+00
300612e6-82d9-487f-a8da-e9c58bf72a2b	4fba22de-81b4-4bd1-b253-5bd3e2fce25c	S7_1500	528ae44e-2470-4d6d-a66a-d55dafe1b6af	bfe75b2f-34a4-47bd-80d1-75cb959ed356	VALUE	Value	Value of the matrix row	b897d462-1b9f-4169-99fb-d27b8c7c6d94	\N	{"setpoint": 1.5}	1	2026-11-17 19:17:07.481158+00
f4b60f0d-c1a7-403c-ab10-f054d34aaab7	3537b054-3917-4cb3-bcfa-f5f84177f4af	S7_300	29f51f73-d787-4a18-8546-7c5368dda76c	91d9bbb2-9515-4128-a251-af5449a7a641	VALUE	Value	Value of the matrix row	89014950-9fab-4031-a2d4-f9ebd137fe3e	DB1.DBB0	[[[true, false], [false, true]], [[true, true], [false, false]]]	1	2026-11-17 19:17:07.545795+00
\.


--
-- Data for Name: parameter_limit; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.parameter_limit (parameter_id, recipe_version_id, leaf_path, min_value, max_value, max_length, created_at) FROM stdin;
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[1][1]	0	255	\N	2026-11-17 19:17:06.844889+00
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[1][2]	0	255	\N	2026-11-17 19:17:06.844889+00
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[1][3]	0	255	\N	2026-11-17 19:17:06.844889+00
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[2][1]	0	255	\N	2026-11-17 19:17:06.844889+00
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[2][2]	0	255	\N	2026-11-17 19:17:06.844889+00
d418c4e5-3798-407a-8be8-2e1d5459f5a7	3d0c0d2d-de32-4fec-bd35-3473c428cf2d	[2][3]	0	255	\N	2026-11-17 19:17:06.844889+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32760]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32761]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32762]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32763]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32764]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32765]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32766]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32767]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32768]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32769]	0	255	\N	2026-11-17 19:17:07.015927+00
06e7cac4-71fc-41e4-a316-af0f3c361fd0	c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	[32770]	0	255	\N	2026-11-17 19:17:07.015927+00
b04f38cd-b399-4084-9555-a3d9e7099c9f	7c96e26f-c416-4ba7-baf9-a279186b6e9a	.next.next.next.next.next.next.next.next.leaf	0	100	\N	2026-11-17 19:17:07.256563+00
b94ac947-65b7-4e99-bcbb-15ca00332b1b	4a1f7a87-1618-4036-988f-4b789cc93269		\N	\N	10	2026-11-17 19:17:07.406198+00
300612e6-82d9-487f-a8da-e9c58bf72a2b	4fba22de-81b4-4bd1-b253-5bd3e2fce25c	.setpoint	0	10	\N	2026-11-17 19:17:07.481158+00
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
5eed0000-0000-4000-8000-000000000001	primitive	BOOL	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	1	\N	bool	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000002	primitive	BYTE	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000003	primitive	WORD	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000004	primitive	DWORD	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000005	primitive	LWORD	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000006	primitive	SINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000007	primitive	USINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	1	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000008	primitive	INT	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000009	primitive	UINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	2	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000a	primitive	DINT	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000b	primitive	UDINT	{CodesysIec,TwinCat3,S7_1200,S7_1500}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000c	primitive	LINT	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000d	primitive	ULINT	{CodesysIec,TwinCat3,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000e	primitive	REAL	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000000f	primitive	LREAL	{CodesysIec,TwinCat3,S7_1200,S7_1500}	8	\N	numeric	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000010	primitive	STRING	{S7_1200,S7_1500,S7_300}	256	254	string	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000011	primitive	WSTRING	{S7_1200,S7_1500}	512	254	string	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000012	primitive	CHAR	{S7_1200,S7_1500,S7_300}	1	\N	char	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000013	primitive	WCHAR	{S7_1200,S7_1500}	2	\N	char	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000014	primitive	TIME	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	duration	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000015	primitive	LTIME	{CodesysIec,TwinCat3,S7_1500}	8	\N	duration	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000016	primitive	DATE	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	2	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000017	primitive	LDATE	{CodesysIec,TwinCat3}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000018	primitive	TIME_OF_DAY	{CodesysIec,TwinCat3,S7_1200,S7_1500,S7_300}	4	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-000000000019	primitive	LTOD	{CodesysIec,TwinCat3,S7_1500}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001a	primitive	DATE_AND_TIME	{CodesysIec,TwinCat3,S7_1500,S7_300}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001b	primitive	LDT	{CodesysIec,TwinCat3,S7_1500}	8	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001c	primitive	DTL	{S7_1200,S7_1500}	12	\N	instant	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001d	primitive	S5TIME	{S7_1500,S7_300}	2	\N	duration	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001e	primitive	STRING	{CodesysIec,TwinCat3}	82	80	string	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
5eed0000-0000-4000-8000-00000000001f	primitive	WSTRING	{CodesysIec,TwinCat3}	164	80	string	\N	\N	\N	\N	1	\N	2026-11-17 19:17:05.815589+00
c4803844-dec9-4ea4-8cb0-f9d9bbf41ff5	array	\N	{S7_300}	\N	\N	\N	5eed0000-0000-4000-8000-000000000002	{1}	{3}	b7e0f969-50c8-437e-bc5c-1d37008663b1	1	\N	2026-11-17 19:17:06.844889+00
651a353d-3248-4a31-920d-15520d32b746	array	\N	{S7_300}	\N	\N	\N	c4803844-dec9-4ea4-8cb0-f9d9bbf41ff5	{1}	{2}	b7e0f969-50c8-437e-bc5c-1d37008663b1	1	\N	2026-11-17 19:17:06.844889+00
f8c6a2f7-fb7f-4489-91af-9c22dd564762	array	\N	{S7_300}	\N	\N	\N	5eed0000-0000-4000-8000-000000000002	{32760}	{32770}	64f392bb-9857-41a1-8048-3ff29152bc6c	1	\N	2026-11-17 19:17:07.015927+00
f3a0725a-8c43-4d08-84c0-5123017f3c99	struct	ST_L9	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.090733+00
2679212d-b408-4a22-88fe-1021e3a021f1	struct	ST_L8	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.113958+00
99643d9d-4022-4520-87ee-5d3fe9c79db3	struct	ST_L7	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.128319+00
7cf94785-7829-469a-b3d9-566a6f928362	struct	ST_L6	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.146117+00
3d44448a-f4fa-42f7-8a48-45449d71e190	struct	ST_L5	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.162097+00
c6784913-77af-4bf7-bf11-2a4fa6ad5abe	struct	ST_L4	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.174297+00
12aadb7c-5967-41f2-9694-a90ed88e9162	struct	ST_L3	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.188094+00
b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea	struct	ST_L2	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.203369+00
a407158f-1cb0-4a58-bd05-909cbc642986	struct	ST_Top	{S7_1500}	\N	\N	\N	\N	\N	\N	c2496c3a-d89e-4d77-92ea-c83075a49d02	1	\N	2026-11-17 19:17:07.223878+00
37af5eff-d6c5-4669-866b-daeea731d8a8	primitive	WSTRING	{S7_1200}	24	10	string	\N	\N	\N	562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0	1	\N	2026-11-17 19:17:07.406198+00
b897d462-1b9f-4169-99fb-d27b8c7c6d94	struct	PID_Compact	{S7_1500}	\N	\N	\N	\N	\N	\N	0993bcf2-aa52-461d-b73f-a4dfc65016c3	1	\N	2026-11-17 19:17:07.458193+00
89014950-9fab-4031-a2d4-f9ebd137fe3e	array	\N	{S7_300}	\N	\N	\N	5eed0000-0000-4000-8000-000000000001	{1,1,1}	{2,2,2}	6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1	1	\N	2026-11-17 19:17:07.545795+00
\.


--
-- Data for Name: plc_type_member; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.plc_type_member (struct_type_id, ordinal, member_name, member_type_id, created_at) FROM stdin;
f3a0725a-8c43-4d08-84c0-5123017f3c99	1	leaf	5eed0000-0000-4000-8000-00000000000a	2026-11-17 19:17:07.090733+00
2679212d-b408-4a22-88fe-1021e3a021f1	1	next	f3a0725a-8c43-4d08-84c0-5123017f3c99	2026-11-17 19:17:07.113958+00
99643d9d-4022-4520-87ee-5d3fe9c79db3	1	next	2679212d-b408-4a22-88fe-1021e3a021f1	2026-11-17 19:17:07.128319+00
7cf94785-7829-469a-b3d9-566a6f928362	1	next	99643d9d-4022-4520-87ee-5d3fe9c79db3	2026-11-17 19:17:07.146117+00
3d44448a-f4fa-42f7-8a48-45449d71e190	1	next	7cf94785-7829-469a-b3d9-566a6f928362	2026-11-17 19:17:07.162097+00
c6784913-77af-4bf7-bf11-2a4fa6ad5abe	1	next	3d44448a-f4fa-42f7-8a48-45449d71e190	2026-11-17 19:17:07.174297+00
12aadb7c-5967-41f2-9694-a90ed88e9162	1	next	c6784913-77af-4bf7-bf11-2a4fa6ad5abe	2026-11-17 19:17:07.188094+00
b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea	1	next	12aadb7c-5967-41f2-9694-a90ed88e9162	2026-11-17 19:17:07.203369+00
a407158f-1cb0-4a58-bd05-909cbc642986	1	next	b5c4b20a-ae2e-4f84-96d5-6f49be32a1ea	2026-11-17 19:17:07.223878+00
b897d462-1b9f-4169-99fb-d27b8c7c6d94	1	setpoint	5eed0000-0000-4000-8000-00000000000e	2026-11-17 19:17:07.458193+00
\.


--
-- Data for Name: recipe; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.recipe (id, machine_id, plc_platform, reference_number, name, deleted_at, created_by, created_at, updated_at) FROM stdin;
7746c280-7895-45e4-95fc-146aaba747a9	b7e0f969-50c8-437e-bc5c-1d37008663b1	S7_300	SR-NESTED-ARRAY	SR-NESTED-ARRAY	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:06.588135+00	2026-11-17 19:17:06.588135+00
b13d25f6-8634-481a-a235-35bd825b2f35	64f392bb-9857-41a1-8048-3ff29152bc6c	S7_300	SR-ARRAY-BOUND	SR-ARRAY-BOUND	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:06.984401+00	2026-11-17 19:17:06.984401+00
3b5fc113-6899-4c11-bf64-e1f71e385c4e	c2496c3a-d89e-4d77-92ea-c83075a49d02	S7_1500	SR-NESTING	SR-NESTING	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.239144+00	2026-11-17 19:17:07.239144+00
d1e40cc3-76f4-441f-a957-6a0a447e9f66	859d6b1e-0dbc-49f5-bf5e-ecc956638666	S7_1500	SR-WCHAR	SR-WCHAR	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.31769+00	2026-11-17 19:17:07.31769+00
12951f8d-047f-4080-97f7-3f8d32cb2562	562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0	S7_1200	SR-WSTRING	SR-WSTRING	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.381188+00	2026-11-17 19:17:07.381188+00
a756d4fa-b224-42b6-b52c-2a5081ba487c	0993bcf2-aa52-461d-b73f-a4dfc65016c3	S7_1500	SR-RESERVED-NAME	SR-RESERVED-NAME	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.466984+00	2026-11-17 19:17:07.466984+00
60cd87ab-ef7c-47a8-b9a7-21d88b373175	6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1	S7_300	SR-BOOL-3D	SR-BOOL-3D	\N	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.532941+00	2026-11-17 19:17:07.532941+00
\.


--
-- Data for Name: recipe_version; Type: TABLE DATA; Schema: public; Owner: recipe_migrator
--

COPY public.recipe_version (id, recipe_id, plc_platform, number, status, row_version, based_on_version_id, change_note, content_sha256, created_by, created_at, updated_at, approved_by, approved_at, retired_at, deleted_at) FROM stdin;
3d0c0d2d-de32-4fec-bd35-3473c428cf2d	7746c280-7895-45e4-95fc-146aaba747a9	S7_300	1	approved	3	\N	Stored row of the matrix	ad753cf21ea496f7bfb18668aafb84111b1469f3b43df32d803ee73e748e0826	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:06.588135+00	2026-11-17 19:17:06.9606+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:06.9606+00	\N	\N
c9b2ee6e-4bd9-43ac-83de-319fd2a776fe	b13d25f6-8634-481a-a235-35bd825b2f35	S7_300	1	approved	3	\N	Stored row of the matrix	882552ae17279c6c1eed8a658bac4113d47bcd5d2a4b6261dd13537a3d195896	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:06.984401+00	2026-11-17 19:17:07.066072+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.066072+00	\N	\N
7c96e26f-c416-4ba7-baf9-a279186b6e9a	3b5fc113-6899-4c11-bf64-e1f71e385c4e	S7_1500	1	approved	3	\N	Stored row of the matrix	f7ee5004c0fb529fb4ac92be7492ef2a9ca20c6145f9b1d269e3d0b59d71995b	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.239144+00	2026-11-17 19:17:07.302116+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.302116+00	\N	\N
bfd86327-8954-4a67-ad61-3f5b6e3f7f0e	d1e40cc3-76f4-441f-a957-6a0a447e9f66	S7_1500	1	approved	3	\N	Stored row of the matrix	258c318dd19a04ac06bbc6e9fef2a43acfaf30117f6701dc12cfe0a777b06ed7	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.31769+00	2026-11-17 19:17:07.365902+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.365902+00	\N	\N
4a1f7a87-1618-4036-988f-4b789cc93269	12951f8d-047f-4080-97f7-3f8d32cb2562	S7_1200	1	approved	3	\N	Stored row of the matrix	332f30c899a01c7879d219b8d9b1e7e5c88e0636d61526bc5852f58938a41301	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.381188+00	2026-11-17 19:17:07.443443+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.443443+00	\N	\N
4fba22de-81b4-4bd1-b253-5bd3e2fce25c	a756d4fa-b224-42b6-b52c-2a5081ba487c	S7_1500	1	approved	3	\N	Stored row of the matrix	e4885a52bb79bd0dd775bc66daedb96406256658c3d8142aedf5eca5833a80fc	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.466984+00	2026-11-17 19:17:07.519391+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.519391+00	\N	\N
3537b054-3917-4cb3-bcfa-f5f84177f4af	60cd87ab-ef7c-47a8-b9a7-21d88b373175	S7_300	1	approved	3	\N	Stored row of the matrix	9bc683b7ebf7f51d60e296ce820addf96cf3b22f293ecc846ef5e36362cc02ed	ed1fca19-9125-4d12-8dc9-e87f5db2c543	2026-11-17 19:17:07.532941+00	2026-11-17 19:17:07.58851+00	21eba560-26c2-4a2d-876a-d73fc338f994	2026-11-17 19:17:07.58851+00	\N	\N
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
7778262a-7aa6-4be9-9a34-91664d595a34	b7e0f969-50c8-437e-bc5c-1d37008663b1	U1	unit	\N	2026-11-17 19:17:06.526203+00	2026-11-17 19:17:06.526203+00
efed59ba-3d26-4bc1-b380-3d6d641cb408	64f392bb-9857-41a1-8048-3ff29152bc6c	U1	unit	\N	2026-11-17 19:17:06.976331+00	2026-11-17 19:17:06.976331+00
0073eb38-c322-4c57-826c-54ead67481fd	c2496c3a-d89e-4d77-92ea-c83075a49d02	U1	unit	\N	2026-11-17 19:17:07.084965+00	2026-11-17 19:17:07.084965+00
b6198aa7-086c-4429-8a4b-f2802a5f56bb	859d6b1e-0dbc-49f5-bf5e-ecc956638666	U1	unit	\N	2026-11-17 19:17:07.314996+00	2026-11-17 19:17:07.314996+00
a04b42cd-a556-4ad9-853d-f9e5843abf1f	562cb3f3-250a-44d0-8e9e-efdbb9c3d8c0	U1	unit	\N	2026-11-17 19:17:07.375146+00	2026-11-17 19:17:07.375146+00
bfe75b2f-34a4-47bd-80d1-75cb959ed356	0993bcf2-aa52-461d-b73f-a4dfc65016c3	U1	unit	\N	2026-11-17 19:17:07.454079+00	2026-11-17 19:17:07.454079+00
91d9bbb2-9515-4128-a251-af5449a7a641	6d4aa7b8-5247-45f0-8c50-be1c5df2f8f1	U1	unit	\N	2026-11-17 19:17:07.529633+00	2026-11-17 19:17:07.529633+00
\.


--
-- Name: audit_log_id_seq; Type: SEQUENCE SET; Schema: public; Owner: audit_owner
--

SELECT pg_catalog.setval('public.audit_log_id_seq', 43, true);


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

\unrestrict G5JEiWoPk4nrEjwnKBqZmszmUJQDf8YS0bdgOCGQi2UWGYoBfTHagC2HlPUzfu2

