#!/usr/bin/env bash
# Create the roles and the database of the recipe editor.
#
# The postgres image runs this script once, as the superuser, on an empty data directory.
# Source: docs/architecture/data-model.md section 5.1, ADR-0031, ADR-0032, ADR-0034, ADR-0035,
# and ADR-0069. The script writes the whole pg_hba.conf (ADR-0069 (r) to (t)). It holds no test
# switch. The storage tests mount their own second init script (ADR-0069 (u)).
# The subnet check, the pg_hba.conf lines, and the REVOKE statements of the default databases
# live in the shared file /etc/recipe-db/cluster-access.sh (ADR-0103 (l), design D1).
# Each password comes from a secret file (reliability.md section 7.11). psql reads the file,
# so a password never appears in the argument list of a process.
#
# Environment:
#   POSTGRES_USER   the superuser (set by the image)
#   PGDATA          the data directory (set by the image)
#   COMPOSE_SUBNET  the IPv4 CIDR of the Compose network, for example 172.28.0.0/16
#   SECRET_DIR      the directory of the four secret files. The default is /run/secrets.
#                   Production never sets it. The script tests set it.
#
# A secret file holds exactly one line, without a carriage return (ADR-0031).
# The script checks every input before the first change. The backquote commands of psql run
# in a shell, and the shell expands $SECRET_DIR from the exported environment.
set -euo pipefail

SECRET_DIR=${SECRET_DIR:-/run/secrets}
export SECRET_DIR

# Write one message and stop. No change has happened yet.
fail() {
  echo "01-roles.sh: $1" >&2
  exit 1
}

# The shared client rules: check_compose_subnet, write_hba_conf, and print_default_db_revokes.
# The database image mounts the file at this fixed path.
source /etc/recipe-db/cluster-access.sh

# Check that a secret file is readable and holds exactly one non-empty line without "\r".
check_secret_file() {
  local path=$1 content
  [ -r "$path" ] || fail "cannot read $path"
  # The trailing "x" keeps the final newline through the command substitution.
  content=$(cat "$path"; printf x)
  content=${content%x}
  content=${content%$'\n'}
  [ -n "$content" ] || fail "$path is empty"
  [[ $content != *$'\r'* ]] || fail "$path holds a carriage return"
  [[ $content != *$'\n'* ]] || fail "$path holds more than one line"
}

# Fail before any change when an input is wrong.
subnet=${COMPOSE_SUBNET:-}
check_compose_subnet "$subnet" \
  || fail "COMPOSE_SUBNET must be an IPv4 CIDR, for example 172.28.0.0/16"
[ -n "${PGDATA:-}" ] || fail "PGDATA is not set"
for name in migrator app backup replicator; do
  check_secret_file "${SECRET_DIR}/${name}_password"
done

# One psql run reads the statements of both heredocs and the REVOKE statements of the shared
# file, in this order.
{
cat <<'SQL'
\set migrator_password `cat "$SECRET_DIR/migrator_password"`
\set app_password `cat "$SECRET_DIR/app_password"`
\set backup_password `cat "$SECRET_DIR/backup_password"`
\set replicator_password `cat "$SECRET_DIR/replicator_password"`

-- ADR-0031: a failed statement writes no password into the server log.
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
CREATE ROLE recipe_migrator   LOGIN PASSWORD :'migrator_password';     -- owner of the schema
CREATE ROLE recipe_app        LOGIN PASSWORD :'app_password';          -- runtime, least privilege
CREATE ROLE recipe_backup     LOGIN PASSWORD :'backup_password';       -- pg_dump only
CREATE ROLE recipe_replicator LOGIN REPLICATION
                              PASSWORD :'replicator_password';         -- walarchive only
CREATE ROLE audit_owner       NOLOGIN;                                  -- owner of audit tables
GRANT pg_read_all_data TO recipe_backup;
-- ADR-0069: the function replication_state() of audit_owner reads pg_stat_replication
-- (section 5.5). recipe_app holds no pg_read_all_stats and reads no statement text.
GRANT pg_read_all_stats TO audit_owner WITH INHERIT TRUE;
-- ADR-0035: the server log holds no DETAIL with row values of a failed statement of recipe_app,
-- and no bind value of a slow statement.
ALTER ROLE recipe_app SET log_error_verbosity = 'terse';
ALTER ROLE recipe_app SET log_parameter_max_length = 0;
-- 0003 transfers the audit objects to audit_owner. ALTER ... OWNER TO needs membership
-- in the new owner. INHERIT FALSE: the migrator inherits no privilege of audit_owner.
GRANT audit_owner TO recipe_migrator WITH INHERIT FALSE;
CREATE DATABASE recipe OWNER recipe_migrator;
REVOKE TEMPORARY ON DATABASE recipe FROM PUBLIC;
REVOKE CONNECT ON DATABASE recipe FROM PUBLIC;
GRANT CONNECT ON DATABASE recipe TO recipe_migrator, recipe_app, recipe_backup;
-- ADR-0032: a normal session of a login role reaches no default database.
-- The shared file prints the two REVOKE statements here.
SQL
print_default_db_revokes
cat <<'SQL'
-- ADR-0034: only audit_owner can drop an audit object, also through the owner of public.
\connect recipe
CREATE SCHEMA audit_guard;
REVOKE ALL ON SCHEMA audit_guard FROM PUBLIC;
CREATE FUNCTION audit_guard.audit_objects_dropped() RETURNS event_trigger
LANGUAGE plpgsql SET search_path = pg_catalog, pg_temp AS $$
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
REVOKE EXECUTE ON FUNCTION audit_guard.audit_objects_dropped() FROM PUBLIC;
CREATE EVENT TRIGGER audit_objects_dropped ON sql_drop
    EXECUTE FUNCTION audit_guard.audit_objects_dropped();
SQL
} | psql -v ON_ERROR_STOP=1 --username "$POSTGRES_USER" --no-password --no-psqlrc \
  --dbname postgres

# The client rules of data-model.md section 5.1, in place of every line of the image.
write_hba_conf "$subnet" "${PGDATA}/pg_hba.conf"
