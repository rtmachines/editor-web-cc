-- Table plc_connection: the connection record of one machine (ADR-0154 (b), design D1).
-- A new table has no rows, so the file rewrites and scans no existing table.
-- Task 4.1 adds the columns, the keys, and the shape check. Tasks 4.2 to 4.4 add the range
-- checks, the credential check, and the trigger plc_connection_a_bump in this file.

CREATE TABLE plc_connection (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    machine_id      UUID NOT NULL,
    -- Copy of machine.plc_platform. Kept correct by the composite FK (I5 pattern).
    plc_platform    TEXT NOT NULL,
    host            TEXT NOT NULL,
    -- S7 fields.
    rack            SMALLINT,
    slot            SMALLINT,
    db_number       INTEGER,
    -- IEC fields.
    port            INTEGER,
    address         TEXT,
    comm_mode       TEXT,
    login           TEXT,
    -- The sealed form of the password (ADR-0154 (d)). No page shows it.
    password_sealed BYTEA,
    row_version     BIGINT NOT NULL DEFAULT 1,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- One row per machine. The constraint also serves as the index of the foreign key.
    CONSTRAINT plc_connection_machine_unique UNIQUE (machine_id),
    -- The record cannot hold another platform than its machine (I5 pattern).
    CONSTRAINT plc_connection_machine_platform
        FOREIGN KEY (machine_id, plc_platform) REFERENCES machine (id, plc_platform),
    CONSTRAINT plc_connection_row_version_positive CHECK (row_version >= 1),
    -- An S7 row has rack, slot, and db_number and no IEC column. An IEC row has port, address,
    -- and comm_mode and no S7 column.
    CONSTRAINT plc_connection_shape CHECK (
        (plc_platform IN ('S7_300', 'S7_1200', 'S7_1500')
            AND rack IS NOT NULL AND slot IS NOT NULL AND db_number IS NOT NULL
            AND port IS NULL AND address IS NULL AND comm_mode IS NULL)
        OR
        (plc_platform IN ('CodesysIec', 'TwinCat3')
            AND port IS NOT NULL AND address IS NOT NULL AND comm_mode IS NOT NULL
            AND rack IS NULL AND slot IS NULL AND db_number IS NULL)
    ),
    -- Task 4.2: range checks. They mirror the value objects of domain-model.md section 2.
    CONSTRAINT plc_connection_rack_range CHECK (rack BETWEEN 0 AND 7),
    CONSTRAINT plc_connection_slot_range CHECK (slot BETWEEN 0 AND 31),
    -- The data block limit of the platform (ADR-0116 (c), ADR-0140 (a)).
    CONSTRAINT plc_connection_db_number_range CHECK (
        db_number IS NULL
        OR (plc_platform = 'S7_300' AND db_number BETWEEN 1 AND 16000)
        OR (plc_platform IN ('S7_1200', 'S7_1500') AND db_number BETWEEN 1 AND 59999)
    ),
    CONSTRAINT plc_connection_port_range CHECK (port BETWEEN 1 AND 65535),
    -- The modes of the platform (ADR-0154 (e)).
    CONSTRAINT plc_connection_mode_value CHECK (
        comm_mode IS NULL
        OR (plc_platform = 'CodesysIec' AND comm_mode IN ('OpcUa', 'CodesysV3', 'ModbusTcp'))
        OR (plc_platform = 'TwinCat3' AND comm_mode IN ('Ads', 'OpcUa', 'ModbusTcp'))
    ),
    -- Task 4.3: login and sealed password come as a pair. The mode decides the pair (ADR-0154 (ea)).
    -- OpcUa and CodesysV3 need it. Ads permits it. ModbusTcp and an S7 row refuse it.
    CONSTRAINT plc_connection_credentials CHECK (
        (login IS NULL) = (password_sealed IS NULL)
        AND CASE
            WHEN comm_mode IN ('OpcUa', 'CodesysV3') THEN login IS NOT NULL
            WHEN comm_mode = 'Ads' THEN true
            ELSE login IS NULL
        END
    ),
    -- Text limits (data-model.md section 1.3). length() counts characters.
    CONSTRAINT plc_connection_host_length CHECK (length(host) BETWEEN 1 AND 253),
    -- IPv4 without a leading zero, or an RFC 1123 host name without a trailing dot.
    -- A text of digits and dots only must be IPv4 (ADR-0154 (f)). PlcHost mirrors this check.
    CONSTRAINT plc_connection_host_format CHECK (
        host ~ '^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){3}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])$'
        OR (host !~ '^[0-9.]+$'
            AND host ~ '^[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?(\.[A-Za-z0-9]([A-Za-z0-9-]{0,61}[A-Za-z0-9])?)*$')
    ),
    CONSTRAINT plc_connection_address_length CHECK (length(address) BETWEEN 1 AND 200),
    CONSTRAINT plc_connection_login_length CHECK (length(login) BETWEEN 1 AND 64),
    -- No control character in address and login. The value objects refuse the same.
    CONSTRAINT plc_connection_text_no_control CHECK (
        address !~ '[[:cntrl:]]' AND login !~ '[[:cntrl:]]'
    ),
    -- The mode Ads takes an AMS Net ID: six numbers 0 to 255 with five dots, no leading zero.
    CONSTRAINT plc_connection_ams_net_id CHECK (
        comm_mode IS DISTINCT FROM 'Ads'
        OR address ~ '^((25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])\.){5}(25[0-5]|2[0-4][0-9]|1[0-9][0-9]|[1-9]?[0-9])$'
    )
);

-- Optimistic lock token (ADR-0008). The existing function of migration 0002 (data-model.md
-- section 4.4) adds 1 to row_version and sets updated_at. The application never sets row_version.
CREATE TRIGGER plc_connection_a_bump
    BEFORE UPDATE ON plc_connection
    FOR EACH ROW EXECUTE FUNCTION bump_row_version();

-- Grants of data-model.md section 5.2 (design D1). The default privilege of migration 0003 already
-- gives SELECT. This file states it again.
GRANT SELECT, INSERT ON plc_connection TO recipe_app;
-- Updates: the value columns and updated_at. id, machine_id, plc_platform, row_version,
-- and created_at get no UPDATE grant. Trigger plc_connection_a_bump (task 4.4) sets row_version.
GRANT UPDATE (host, rack, slot, db_number, port, address, comm_mode, login,
              password_sealed, updated_at) ON plc_connection TO recipe_app;
-- No grant means denied:
--   plc_connection : no DELETE, no TRUNCATE
