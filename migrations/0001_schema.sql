-- Schema of the recipe editor. Copy of docs/architecture/data-model.md sections 3.0 to 3.17.
-- Never edit this file after it ships. A change is a new forward migration.

-- 3.0 Shared functions
CREATE FUNCTION set_updated_at() RETURNS trigger
LANGUAGE plpgsql SET search_path = pg_catalog, public, pg_temp AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- 3.1 machine
CREATE TABLE machine (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code          TEXT NOT NULL,
    name          TEXT NOT NULL,
    plc_platform  TEXT NOT NULL,
    archived_at   TIMESTAMPTZ,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT machine_code_format CHECK (code ~ '^M[1-9][0-9]{0,6}$'),
    CONSTRAINT machine_name_length CHECK (length(name) BETWEEN 1 AND 200),
    CONSTRAINT machine_platform_value CHECK (
        plc_platform IN ('CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300')
    ),
    -- Target for the composite FK that propagates the platform (I5).
    CONSTRAINT machine_id_platform_unique UNIQUE (id, plc_platform)
);
-- I11, Assumption A32: a code is unique among live machines. An archived machine frees its code.
CREATE UNIQUE INDEX machine_code_live_unique ON machine (code) WHERE archived_at IS NULL;
CREATE TRIGGER machine_set_updated_at
    BEFORE UPDATE ON machine FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- 3.2 unit
CREATE TABLE unit (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    machine_id   UUID NOT NULL REFERENCES machine (id),
    path         TEXT NOT NULL,
    -- Parent path. NULL for a main unit. Generated, so it can never disagree with path.
    parent_path  TEXT GENERATED ALWAYS AS (substring(path FROM '^(.*)\.[0-9]+$')) STORED,
    -- Number of segments. Generated.
    depth        SMALLINT GENERATED ALWAYS AS
                     (array_length(string_to_array(path, '.'), 1)) STORED,
    name         TEXT NOT NULL,
    archived_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- 1 to 5 segments. Each segment is 1..64 (I9, Assumptions A6 and A30, ADR-0022).
    CONSTRAINT unit_path_format CHECK (
        path ~ '^U([1-9]|[1-5][0-9]|6[0-4])(\.([1-9]|[1-5][0-9]|6[0-4])){0,4}$'
    ),
    CONSTRAINT unit_depth_range CHECK (depth BETWEEN 1 AND 5),
    CONSTRAINT unit_name_length CHECK (length(name) BETWEEN 1 AND 200),
    CONSTRAINT unit_path_unique UNIQUE (machine_id, path),
    -- Target for parameter scope checks.
    CONSTRAINT unit_machine_id_unique UNIQUE (machine_id, id),
    -- The parent exists in the same machine (I10). A unit cannot exist without its parent.
    CONSTRAINT unit_parent_exists
        FOREIGN KEY (machine_id, parent_path) REFERENCES unit (machine_id, path)
);
CREATE INDEX unit_parent_path_idx ON unit (machine_id, parent_path);
CREATE TRIGGER unit_set_updated_at
    BEFORE UPDATE ON unit FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- 3.3 plc_type
-- NULL-safe. A NULL input or a NULL element gives false, not NULL.
CREATE FUNCTION dims_valid(lower_b INTEGER[], upper_b INTEGER[]) RETURNS BOOLEAN
LANGUAGE sql IMMUTABLE SET search_path = pg_catalog, public, pg_temp AS $$
    SELECT COALESCE(
        array_ndims(lower_b) = 1 AND array_ndims(upper_b) = 1
        AND cardinality(lower_b) = cardinality(upper_b)
        -- At most 3 dimensions (Assumption A38).
        AND cardinality(lower_b) BETWEEN 1 AND 3
        AND array_position(lower_b, NULL) IS NULL
        AND array_position(upper_b, NULL) IS NULL
        AND NOT EXISTS (
            SELECT 1 FROM generate_subscripts(lower_b, 1) AS i
            WHERE lower_b[i] > upper_b[i]
               OR upper_b[i]::bigint - lower_b[i] >= 65536
        ),
        false);
$$;

CREATE TABLE plc_type (
    id                  UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    kind                TEXT NOT NULL,
    -- Primitive and struct rows have a name. Array rows have no name.
    -- The domain renders an array as 'ARRAY[1..3] OF INT'.
    name                TEXT,
    platforms           TEXT[] NOT NULL,
    -- S7 size of one primitive element in bytes. BOOL is 1, its storage unit. NULL for
    -- arrays and structs: the domain computes their layout size and does not store it.
    size_bytes          INTEGER,
    -- Only for STRING and WSTRING. Number of characters. The range per platform is the
    -- StringLength table of domain-model.md 2.1. This CHECK is the widest range.
    string_length       SMALLINT,
    -- LeafClass of domain-model.md 2.1. Set on every primitive row, NULL on every other row.
    -- Section 7 gives the class of each primitive.
    leaf_class          TEXT,
    element_type_id     UUID REFERENCES plc_type (id),
    dim_lower           INTEGER[],
    dim_upper           INTEGER[],
    -- NULL for the global primitive catalog. Set for a UDT, an array type,
    -- and a machine-specific STRING(n) / WSTRING(n).
    machine_id          UUID REFERENCES machine (id),
    revision            INTEGER NOT NULL DEFAULT 1,
    supersedes_type_id  UUID REFERENCES plc_type (id),
    created_at          TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT plc_type_kind_value CHECK (kind IN ('primitive', 'array', 'struct')),
    CONSTRAINT plc_type_name_present CHECK ((kind = 'array') = (name IS NULL)),
    CONSTRAINT plc_type_name_format CHECK (name IS NULL OR name ~ '^[A-Za-z_][A-Za-z0-9_]{0,63}$'),
    CONSTRAINT plc_type_platforms_value CHECK (
        cardinality(platforms) >= 1
        AND platforms <@ ARRAY['CodesysIec', 'TwinCat3', 'S7_1200', 'S7_1500', 'S7_300']
    ),
    CONSTRAINT plc_type_revision_positive CHECK (revision >= 1),
    -- STRING and WSTRING always carry a length. No other type does.
    CONSTRAINT plc_type_string_length_required CHECK (
        (kind = 'primitive' AND name IN ('STRING', 'WSTRING')) = (string_length IS NOT NULL)
    ),
    CONSTRAINT plc_type_string_length_range CHECK (
        string_length IS NULL OR string_length BETWEEN 1 AND 16382
    ),
    -- A primitive has a leaf class. A struct and an array have none.
    CONSTRAINT plc_type_leaf_class_present CHECK ((kind = 'primitive') = (leaf_class IS NOT NULL)),
    CONSTRAINT plc_type_leaf_class_value CHECK (
        leaf_class IS NULL
        OR leaf_class IN ('numeric', 'string', 'bool', 'char', 'duration', 'instant')
    ),
    -- The class 'string' and a string length always come together.
    CONSTRAINT plc_type_leaf_class_string CHECK (
        COALESCE(leaf_class = 'string', false) = (string_length IS NOT NULL)
    ),
    -- Shape per kind.
    CONSTRAINT plc_type_primitive_shape CHECK (
        kind <> 'primitive'
        OR (element_type_id IS NULL AND dim_lower IS NULL AND dim_upper IS NULL
            AND size_bytes IS NOT NULL AND size_bytes >= 1
            AND (machine_id IS NULL OR name IN ('STRING', 'WSTRING')))
    ),
    CONSTRAINT plc_type_array_shape CHECK (
        kind <> 'array'
        OR (element_type_id IS NOT NULL AND dims_valid(dim_lower, dim_upper)
            AND size_bytes IS NULL AND string_length IS NULL AND machine_id IS NOT NULL)
    ),
    CONSTRAINT plc_type_struct_shape CHECK (
        kind <> 'struct'
        OR (element_type_id IS NULL AND dim_lower IS NULL AND dim_upper IS NULL
            AND size_bytes IS NULL AND string_length IS NULL AND machine_id IS NOT NULL)
    ),
    CONSTRAINT plc_type_supersedes_only_struct CHECK (
        supersedes_type_id IS NULL OR kind = 'struct'
    )
);

-- Identity of named types: UDT (machine, name, revision) and global primitives
-- (NULL machine counts as one value). STRING(n) rows are excluded.
CREATE UNIQUE INDEX plc_type_named_unique
    ON plc_type (machine_id, name, revision) NULLS NOT DISTINCT
    WHERE kind <> 'array' AND string_length IS NULL;
-- Identity of STRING(n) and WSTRING(n): one row per machine, name, and length.
CREATE UNIQUE INDEX plc_type_string_unique
    ON plc_type (machine_id, name, string_length) NULLS NOT DISTINCT
    WHERE string_length IS NOT NULL;
-- Identity of array types: one row per machine, element type, and bounds.
CREATE UNIQUE INDEX plc_type_array_unique
    ON plc_type (machine_id, element_type_id, dim_lower, dim_upper)
    WHERE kind = 'array';

CREATE INDEX plc_type_element_type_id_idx ON plc_type (element_type_id);
CREATE INDEX plc_type_machine_id_idx ON plc_type (machine_id);
CREATE INDEX plc_type_supersedes_type_id_idx ON plc_type (supersedes_type_id);

-- 3.4 plc_type_member
CREATE TABLE plc_type_member (
    struct_type_id  UUID NOT NULL REFERENCES plc_type (id),
    ordinal         SMALLINT NOT NULL,
    member_name     TEXT NOT NULL,
    member_type_id  UUID NOT NULL REFERENCES plc_type (id),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (struct_type_id, ordinal),
    -- At most 1024 members (type.max_members, reliability.md 7.11).
    CONSTRAINT plc_type_member_ordinal_range CHECK (ordinal BETWEEN 1 AND 1024),
    CONSTRAINT plc_type_member_name_format CHECK (member_name ~ '^[A-Za-z_][A-Za-z0-9_]{0,63}$'),
    CONSTRAINT plc_type_member_name_unique UNIQUE (struct_type_id, member_name),
    CONSTRAINT plc_type_member_no_self CHECK (struct_type_id <> member_type_id)
);
CREATE INDEX plc_type_member_member_type_id_idx ON plc_type_member (member_type_id);

-- 3.5 app_user
CREATE TABLE app_user (
    id                    UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    login                 TEXT NOT NULL,
    -- Argon2id PHC string. Never returned to the UI. Never logged.
    password_hash         TEXT NOT NULL,
    role                  TEXT NOT NULL,
    is_active             BOOLEAN NOT NULL DEFAULT true,
    -- Incremented on deactivate, password reset, own password change, and role change.
    -- A session with an older epoch is invalid (ADR-0014).
    session_epoch         BIGINT NOT NULL DEFAULT 0,
    -- Set by create-admin, CreateUser, and ResetPassword. Until cleared, the
    -- session can reach only the routes listed in api.md 7.4.
    must_change_password  BOOLEAN NOT NULL DEFAULT false,
    deactivated_at        TIMESTAMPTZ,
    created_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at            TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT app_user_login_format CHECK (login ~ '^[a-z0-9._-]{3,64}$'),
    CONSTRAINT app_user_login_unique UNIQUE (login),
    CONSTRAINT app_user_role_value CHECK (
        role IN ('administrator', 'process_engineer', 'operator')
    ),
    CONSTRAINT app_user_hash_format CHECK (password_hash LIKE '$argon2id$%'),
    CONSTRAINT app_user_active_consistent CHECK (is_active = (deactivated_at IS NULL)),
    CONSTRAINT app_user_epoch_nonnegative CHECK (session_epoch >= 0)
);
CREATE TRIGGER app_user_set_updated_at
    BEFORE UPDATE ON app_user FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- 3.6 session
CREATE TABLE session (
    -- SHA-256 of the cookie token, lower-case hex. The raw token exists only in the cookie.
    id             TEXT PRIMARY KEY,
    user_id        UUID NOT NULL REFERENCES app_user (id),
    -- Copy of app_user.session_epoch at login. Compared on every request.
    epoch          BIGINT NOT NULL,
    csrf_token     TEXT NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Idle deadline. The absolute deadline is created_at + session.absolute_timeout
    -- (reliability.md 7.11), computed by the application.
    expires_at     TIMESTAMPTZ NOT NULL,
    CONSTRAINT session_id_format CHECK (id ~ '^[0-9a-f]{64}$'),
    CONSTRAINT session_csrf_format CHECK (csrf_token ~ '^[0-9a-f]{64}$')
);
CREATE INDEX session_user_id_idx ON session (user_id);
CREATE INDEX session_expires_at_idx ON session (expires_at);

-- 3.7 login_attempt
CREATE TABLE login_attempt (
    id            BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- Normalized login value as typed. It may not match an account.
    login         TEXT NOT NULL,
    client_ip     INET NOT NULL,
    attempted_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    succeeded     BOOLEAN NOT NULL,
    CONSTRAINT login_attempt_login_length CHECK (length(login) BETWEEN 1 AND 64)
);
CREATE INDEX login_attempt_login_ip_idx ON login_attempt (login, client_ip, attempted_at);
CREATE INDEX login_attempt_ip_idx ON login_attempt (client_ip, attempted_at);
-- The batched delete of the job cleanup reads the oldest rows first.
CREATE INDEX login_attempt_attempted_at_idx ON login_attempt (attempted_at);

-- 3.8 security_event
CREATE TABLE security_event (
    id           BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    occurred_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    kind         TEXT NOT NULL,
    -- HMAC-SHA256 of the login value with the key of the Docker secret login_hmac_key
    -- (security.login_hmac_key_file, reliability.md 7.11). Set when no account matches,
    -- and always for login_throttled.
    login_hmac   BYTEA,
    -- Set when an account matches.
    user_id      UUID REFERENCES app_user (id),
    -- NULL only for an event without a request: audit_chain_broken.
    client_ip    INET,
    -- First characters of the User-Agent header (http.user_agent_max, reliability.md 7.11).
    user_agent   TEXT,
    request_id   UUID NOT NULL,
    detail       JSONB,
    -- Number of events in the row. Rows of login_failed, login_throttled, and csrf_rejected
    -- aggregate per (client_ip, kind, minute). Every other row holds one event.
    count        INTEGER NOT NULL DEFAULT 1,
    CONSTRAINT security_event_kind_value CHECK (kind IN (
        'login_failed', 'login_rejected_inactive', 'login_throttled',
        'csrf_rejected', 'forbidden', 'unlock', 'audit_chain_broken'
    )),
    CONSTRAINT security_event_subject CHECK (login_hmac IS NOT NULL OR user_id IS NOT NULL
        OR kind IN ('csrf_rejected', 'audit_chain_broken')),
    CONSTRAINT security_event_client_ip_present CHECK (client_ip IS NOT NULL
        OR kind = 'audit_chain_broken'),
    CONSTRAINT security_event_user_agent_length CHECK (user_agent IS NULL OR length(user_agent) <= 256),
    CONSTRAINT security_event_count_positive CHECK (count >= 1),
    CONSTRAINT security_event_count_aggregated CHECK (
        count = 1 OR kind IN ('login_failed', 'login_throttled', 'csrf_rejected')
    )
);
CREATE INDEX security_event_user_id_idx ON security_event (user_id, id);
CREATE INDEX security_event_kind_idx ON security_event (kind, id);
CREATE INDEX security_event_client_ip_idx ON security_event (client_ip, id);
-- One aggregated row per (client_ip, kind, minute in UTC). The writer upserts on this index.
CREATE UNIQUE INDEX security_event_aggregate_unique
    ON security_event (client_ip, kind, date_trunc('minute', occurred_at AT TIME ZONE 'UTC'))
    WHERE kind IN ('login_failed', 'login_throttled', 'csrf_rejected');

-- 3.9 recipe
CREATE TABLE recipe (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    machine_id        UUID NOT NULL,
    -- Copy of machine.plc_platform. Kept correct by the composite FK.
    plc_platform      TEXT NOT NULL,
    reference_number  TEXT NOT NULL,
    name              TEXT NOT NULL,
    deleted_at        TIMESTAMPTZ,
    created_by        UUID NOT NULL REFERENCES app_user (id),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT recipe_machine_platform
        FOREIGN KEY (machine_id, plc_platform) REFERENCES machine (id, plc_platform),
    CONSTRAINT recipe_reference_format CHECK (reference_number ~ '^[A-Z0-9][A-Z0-9_-]{0,49}$'),
    -- Plain unique. A reference number is never reused (ADR-0021).
    CONSTRAINT recipe_reference_unique UNIQUE (reference_number),
    CONSTRAINT recipe_name_length CHECK (length(name) BETWEEN 1 AND 200),
    CONSTRAINT recipe_id_platform_unique UNIQUE (id, plc_platform)
);
CREATE INDEX recipe_machine_id_idx ON recipe (machine_id, name) WHERE deleted_at IS NULL;
CREATE INDEX recipe_machine_id_all_idx ON recipe (machine_id);
CREATE INDEX recipe_created_by_idx ON recipe (created_by);
CREATE TRIGGER recipe_set_updated_at
    BEFORE UPDATE ON recipe FOR EACH ROW EXECUTE FUNCTION set_updated_at();

-- 3.10 recipe_version
CREATE TABLE recipe_version (
    id                   UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_id            UUID NOT NULL,
    plc_platform         TEXT NOT NULL,
    number               INTEGER NOT NULL,
    status               TEXT NOT NULL DEFAULT 'draft',
    -- Optimistic lock token (I12, ADR-0008). Bumped by trigger 4.4 on every UPDATE.
    row_version          BIGINT NOT NULL DEFAULT 1,
    based_on_version_id  UUID,
    change_note          TEXT NOT NULL,
    -- SHA-256 (hex) of the canonical export body. Set at approval. Frozen (ADR-0019).
    content_sha256       TEXT,
    created_by           UUID NOT NULL REFERENCES app_user (id),
    created_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT now(),
    approved_by          UUID REFERENCES app_user (id),
    approved_at          TIMESTAMPTZ,
    retired_at           TIMESTAMPTZ,
    deleted_at           TIMESTAMPTZ,
    CONSTRAINT recipe_version_recipe_platform
        FOREIGN KEY (recipe_id, plc_platform) REFERENCES recipe (id, plc_platform),
    CONSTRAINT recipe_version_number_positive CHECK (number >= 1),
    CONSTRAINT recipe_version_number_unique UNIQUE (recipe_id, number),
    -- Target for based_on: a version can be based only on a version of the same recipe.
    CONSTRAINT recipe_version_recipe_id_id_unique UNIQUE (recipe_id, id),
    CONSTRAINT recipe_version_based_on_same_recipe
        FOREIGN KEY (recipe_id, based_on_version_id) REFERENCES recipe_version (recipe_id, id),
    CONSTRAINT recipe_version_status_value CHECK (status IN ('draft', 'approved', 'retired')),
    CONSTRAINT recipe_version_change_note_length CHECK (length(change_note) BETWEEN 1 AND 4000),
    CONSTRAINT recipe_version_row_version_positive CHECK (row_version >= 1),
    CONSTRAINT recipe_version_hash_format CHECK (content_sha256 IS NULL OR content_sha256 ~ '^[0-9a-f]{64}$'),
    -- Status, timestamps, and hash agree.
    CONSTRAINT recipe_version_status_fields CHECK (
        (status = 'draft' AND approved_at IS NULL AND approved_by IS NULL
            AND retired_at IS NULL AND content_sha256 IS NULL)
        OR (status = 'approved' AND approved_at IS NOT NULL AND approved_by IS NOT NULL
            AND retired_at IS NULL AND content_sha256 IS NOT NULL)
        OR (status = 'retired' AND approved_at IS NOT NULL AND approved_by IS NOT NULL
            AND retired_at IS NOT NULL AND content_sha256 IS NOT NULL)
    ),
    -- Only a draft can be discarded.
    CONSTRAINT recipe_version_deleted_only_draft CHECK (deleted_at IS NULL OR status = 'draft'),
    CONSTRAINT recipe_version_id_platform_unique UNIQUE (id, plc_platform)
) WITH (fillfactor = 70);
CREATE INDEX recipe_version_based_on_idx ON recipe_version (based_on_version_id);
CREATE INDEX recipe_version_created_by_idx ON recipe_version (created_by);
CREATE INDEX recipe_version_approved_by_idx ON recipe_version (approved_by);
-- I1: at most one approved version per recipe.
CREATE UNIQUE INDEX recipe_version_one_approved
    ON recipe_version (recipe_id) WHERE status = 'approved';
-- Assumption A7: at most one live draft per recipe.
CREATE UNIQUE INDEX recipe_version_one_live_draft
    ON recipe_version (recipe_id) WHERE status = 'draft' AND deleted_at IS NULL;

-- 3.11 draft_editor
CREATE TABLE draft_editor (
    recipe_version_id  UUID NOT NULL REFERENCES recipe_version (id),
    user_id            UUID NOT NULL REFERENCES app_user (id),
    first_edit_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (recipe_version_id, user_id)
);
CREATE INDEX draft_editor_user_id_idx ON draft_editor (user_id);

-- 3.12 parameter
CREATE TABLE parameter (
    id                 UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recipe_version_id  UUID NOT NULL,
    plc_platform       TEXT NOT NULL,
    -- Stable identity across versions (Assumption A40). AddParameter generates it on the server.
    -- CreateVersion copies it.
    parameter_key      UUID NOT NULL DEFAULT gen_random_uuid(),
    unit_id            UUID NOT NULL REFERENCES unit (id),
    tag                TEXT NOT NULL,
    name_en            TEXT NOT NULL,
    description_en     TEXT NOT NULL,
    plc_type_id        UUID NOT NULL REFERENCES plc_type (id),
    s7_address         TEXT,
    -- JSON tree with the shape of the type tree. Numeric, duration, and instant leaves are
    -- canonical decimals (domain-model.md 2.2). See section 9.
    value              JSONB COMPRESSION lz4 NOT NULL,
    -- Sort order of the UI inside the unit. Not unique. UI only, not exported
    -- (api.md 6.3). At most parameter.ordinal_max (reliability.md 7.11).
    ordinal            INTEGER NOT NULL DEFAULT 0,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT parameter_version_platform
        FOREIGN KEY (recipe_version_id, plc_platform)
        REFERENCES recipe_version (id, plc_platform),
    -- Target for the child tables. A child row always names its version.
    CONSTRAINT parameter_id_version_unique UNIQUE (id, recipe_version_id),
    CONSTRAINT parameter_tag_format CHECK (tag ~ '^[A-Z][A-Z0-9_]{0,63}$'),
    CONSTRAINT parameter_tag_unique UNIQUE (recipe_version_id, tag),
    CONSTRAINT parameter_key_unique UNIQUE (recipe_version_id, parameter_key),
    CONSTRAINT parameter_name_length CHECK (length(name_en) BETWEEN 1 AND 200),
    -- The description is required (Assumption A36, ADR-0022).
    CONSTRAINT parameter_description_length CHECK (length(description_en) BETWEEN 1 AND 4000),
    -- DB<n>.DBX<off>.<bit> or DB<n>.DB{B,W,D}<off>. Bit only with X.
    CONSTRAINT parameter_s7_address_format CHECK (
        s7_address IS NULL
        OR s7_address ~ '^DB[1-9][0-9]{0,4}\.DB(X[0-9]{1,5}\.[0-7]|[BWD][0-9]{1,5})$'
    ),
    -- I5, Assumption A31: required on S7_300, optional on S7_1200 and S7_1500,
    -- not allowed on CodesysIec and TwinCat3.
    CONSTRAINT parameter_s7_address_platform CHECK (
        (plc_platform = 'S7_300' AND s7_address IS NOT NULL)
        OR plc_platform IN ('S7_1200', 'S7_1500')
        OR (plc_platform IN ('CodesysIec', 'TwinCat3') AND s7_address IS NULL)
    ),
    CONSTRAINT parameter_ordinal_range CHECK (ordinal BETWEEN 0 AND 100000),
    CONSTRAINT parameter_value_size CHECK (pg_column_size(value) <= 1048576)
);
CREATE INDEX parameter_version_unit_ordinal_idx ON parameter (recipe_version_id, unit_id, ordinal);
CREATE INDEX parameter_unit_id_idx ON parameter (unit_id);
CREATE INDEX parameter_plc_type_id_idx ON parameter (plc_type_id);
CREATE INDEX parameter_key_idx ON parameter (parameter_key);

-- 3.13 parameter_limit
CREATE TABLE parameter_limit (
    parameter_id       UUID NOT NULL,
    recipe_version_id  UUID NOT NULL,
    -- '' means the parameter itself (primitive type). '[*]' means every element
    -- of an array at that level. The LeafPath grammar of domain-model.md 2.1 is the
    -- definition. An index segment has 1 to 3 indices and never contains '*'. An index
    -- has at most 10 digits. The domain bounds its value to the INTEGER range.
    leaf_path          TEXT NOT NULL DEFAULT '',
    min_value          NUMERIC,
    max_value          NUMERIC,
    max_length         SMALLINT,
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (parameter_id, leaf_path),
    CONSTRAINT parameter_limit_parameter
        FOREIGN KEY (parameter_id, recipe_version_id)
        REFERENCES parameter (id, recipe_version_id) ON DELETE CASCADE,
    CONSTRAINT parameter_limit_path_format CHECK (
        leaf_path ~ '^(\.[A-Za-z_][A-Za-z0-9_]{0,63}|\[(\*|-?[0-9]{1,10}(,-?[0-9]{1,10}){0,2})\])*$'
    ),
    CONSTRAINT parameter_limit_min_le_max CHECK (
        min_value IS NULL OR max_value IS NULL OR min_value <= max_value
    ),
    CONSTRAINT parameter_limit_not_empty CHECK (
        min_value IS NOT NULL OR max_value IS NOT NULL OR max_length IS NOT NULL
    ),
    -- The leaf class sets the kind: min and max for numeric, duration, and instant leaves,
    -- max_length for string leaves. Never both. Duration and instant limits are counts of
    -- the base unit (domain-model.md 2.1).
    CONSTRAINT parameter_limit_one_kind CHECK (
        max_length IS NULL OR (min_value IS NULL AND max_value IS NULL)
    ),
    CONSTRAINT parameter_limit_max_length_range CHECK (
        max_length IS NULL OR max_length BETWEEN 1 AND 16382
    )
);
CREATE INDEX parameter_limit_version_idx ON parameter_limit (recipe_version_id);

-- 3.14 parameter_translation
CREATE TABLE parameter_translation (
    parameter_id       UUID NOT NULL,
    recipe_version_id  UUID NOT NULL,
    lang               TEXT NOT NULL,
    name               TEXT NOT NULL,
    description        TEXT NOT NULL DEFAULT '',
    created_at         TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (parameter_id, lang),
    CONSTRAINT parameter_translation_parameter
        FOREIGN KEY (parameter_id, recipe_version_id)
        REFERENCES parameter (id, recipe_version_id) ON DELETE CASCADE,
    -- One LangCode grammar for the whole system: language subtag, optional region.
    -- English is the base text in parameter.name_en, so 'en' is not a translation.
    CONSTRAINT parameter_translation_lang_format CHECK (
        lang ~ '^[a-z]{2,3}(-[A-Z]{2})?$' AND lang <> 'en'
    ),
    CONSTRAINT parameter_translation_name_length CHECK (length(name) BETWEEN 1 AND 200),
    CONSTRAINT parameter_translation_description_length CHECK (length(description) <= 4000)
);
CREATE INDEX parameter_translation_version_idx ON parameter_translation (recipe_version_id);

-- 3.15 audit_log
CREATE TABLE audit_log (
    id             BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    -- Position in the hash chain. Set by trigger 4.9 from audit_chain_head. No gaps.
    chain_seq      BIGINT NOT NULL,
    occurred_at    TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- NULL only for create-admin, the startup settings comparison, and a data migration
    -- that changes catalog rows (section 10).
    actor_user_id  UUID REFERENCES app_user (id),
    action         TEXT NOT NULL,
    entity_type    TEXT NOT NULL,
    entity_id      UUID NOT NULL,
    -- Root of the aggregate (section 10): recipe for parameters and versions, machine for
    -- units and types, app_user for users, app_setting for settings, plc_type for a global
    -- primitive row that a data migration changes.
    aggregate_type TEXT NOT NULL,
    -- Gives "history of recipe R" in one index scan.
    aggregate_id   UUID NOT NULL,
    -- A diff object or the full row, by action (state rule of section 10).
    before_state   JSONB COMPRESSION lz4,
    after_state    JSONB COMPRESSION lz4,
    request_id     UUID NOT NULL,
    client_ip      INET,
    -- First 16 hex characters of session.id. Never the full id.
    session_ref    TEXT,
    -- Hash chain (ADR-0022). Set by trigger 4.9. prev_hash is NULL for the first row.
    prev_hash      BYTEA,
    entry_hash     BYTEA NOT NULL,
    CONSTRAINT audit_log_action_format CHECK (action ~ '^[A-Z][A-Za-z]{2,63}$'),
    CONSTRAINT audit_log_entity_type_format CHECK (entity_type ~ '^[a-z_]{2,64}$'),
    CONSTRAINT audit_log_aggregate_type_value CHECK (
        aggregate_type IN ('machine', 'recipe', 'app_user', 'app_setting', 'plc_type')
    ),
    CONSTRAINT audit_log_chain_seq_unique UNIQUE (chain_seq),
    CONSTRAINT audit_log_has_state CHECK (before_state IS NOT NULL OR after_state IS NOT NULL),
    CONSTRAINT audit_log_session_ref_format CHECK (session_ref IS NULL OR session_ref ~ '^[0-9a-f]{16}$'),
    CONSTRAINT audit_log_hash_length CHECK (length(entry_hash) = 32 AND (prev_hash IS NULL OR length(prev_hash) = 32))
);
CREATE INDEX audit_log_aggregate_idx ON audit_log (aggregate_id, id);
CREATE INDEX audit_log_entity_idx ON audit_log (entity_type, entity_id, id);
CREATE INDEX audit_log_actor_idx ON audit_log (actor_user_id, id);
-- The /audit filters action, from, and to (api.md 2.9).
CREATE INDEX audit_log_action_idx ON audit_log (action, id);
CREATE INDEX audit_log_occurred_at_brin ON audit_log USING brin (occurred_at);

-- 3.16 audit_chain_head
CREATE TABLE audit_chain_head (
    -- Always true. With the primary key, the table can hold one row only.
    singleton  BOOLEAN PRIMARY KEY DEFAULT true,
    -- chain_seq of the last audit_log row. 0 before the first row.
    last_seq   BIGINT NOT NULL DEFAULT 0,
    -- entry_hash of the last audit_log row. NULL before the first row.
    last_hash  BYTEA,
    CONSTRAINT audit_chain_head_one_row CHECK (singleton),
    CONSTRAINT audit_chain_head_seq_nonnegative CHECK (last_seq >= 0),
    CONSTRAINT audit_chain_head_hash_length CHECK (last_hash IS NULL OR length(last_hash) = 32)
);
INSERT INTO audit_chain_head (singleton, last_seq, last_hash) VALUES (true, 0, NULL);

-- 3.17 app_setting
CREATE TABLE app_setting (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    -- Key of reliability.md section 7.11, for example 'approval.separation_of_duties'.
    key         TEXT NOT NULL,
    -- Canonical text of the effective value. At most setting.value_max characters
    -- (reliability.md 7.11).
    value       TEXT NOT NULL,
    -- NULL when the startup comparison writes the row.
    updated_by  UUID REFERENCES app_user (id),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT now(),
    CONSTRAINT app_setting_key_unique UNIQUE (key),
    CONSTRAINT app_setting_key_format CHECK (key ~ '^[a-z][a-z0-9_]*(\.[a-z][a-z0-9_]*)+$'),
    CONSTRAINT app_setting_value_length CHECK (length(value) <= 1000)
);
CREATE INDEX app_setting_updated_by_idx ON app_setting (updated_by);
CREATE TRIGGER app_setting_set_updated_at
    BEFORE UPDATE ON app_setting FOR EACH ROW EXECUTE FUNCTION set_updated_at();
