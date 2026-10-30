# Sourced by restore-volume-test. It writes the stubs that restore-volume needs and restore-test does not.
#
# The stubs of initdb, pg_ctl, pg_restore, and psql come from lib/restore-stubs.sh. This file adds
# the stubs of pg_controldata, sync, and flock. They record each call in the file STUB_LOG, as the
# other stubs do. The stub of flock writes to the file STUB_DIR/flock.log and not to STUB_LOG,
# so a case can tell the lock from the PostgreSQL tools.
#
# Switches, set as VAR=value in the environment of one run:
#   STUB_CONTROLDATA_STATE=<text>        the cluster state that pg_controldata prints, default "shut down".
#   STUB_SYSTEM_IDENTIFIER=<digits>      the system identifier that pg_controldata prints.
#   STUB_CONTROLDATA_FAIL=1              pg_controldata fails.
#   STUB_SYNC_FAIL=1                     sync fails.
#   STUB_ID_UID=<digits>                 the user id that `id -u` prints, default 999.
#   STUB_ID_NAME=<text>                  the user name that `id -un` prints, default postgres.
#   STUB_ID_FAIL=1                       `id -u` and `id -un` fail.
# The stub of flock needs the variable REAL_FLOCK. It writes the access mode of file descriptor 9
# (0 is read-only) and the arguments of each call, then it runs the real flock.

make_stub_pg_controldata() {
  cat >"$ROOT/stubs/pg_controldata" <<'STUB'
#!/usr/bin/env bash
echo "EVENT pg_controldata" >>"$STUB_LOG"
echo "ARGV pg_controldata $*" >>"$STUB_LOG"
[ -z "${STUB_CONTROLDATA_FAIL:-}" ] || { echo "pg_controldata: error: could not open file" >&2; exit 1; }
printf 'pg_control version number:            1300\n'
printf 'Database system identifier:           %s\n' "${STUB_SYSTEM_IDENTIFIER:-7300000000000000001}"
printf 'Database cluster state:               %s\n' "${STUB_CONTROLDATA_STATE:-shut down}"
exit 0
STUB
}

make_stub_sync() {
  cat >"$ROOT/stubs/sync" <<'STUB'
#!/usr/bin/env bash
echo "EVENT sync" >>"$STUB_LOG"
echo "ARGV sync $*" >>"$STUB_LOG"
[ -z "${STUB_SYNC_FAIL:-}" ] || exit 1
exit 0
STUB
}

make_stub_flock() {
  cat >"$ROOT/stubs/flock" <<'STUB'
#!/usr/bin/env bash
flags=$(awk '/^flags:/ { print $2 }' "/proc/$$/fdinfo/9" 2>/dev/null)
echo "ARGV flock $*" >>"$STUB_DIR/flock.log"
echo "FDMODE $((8#${flags:-0} & 3))" >>"$STUB_DIR/flock.log"
exec "$REAL_FLOCK" "$@"
STUB
}

# The stub of id answers 999 and postgres by default, because the suite runs as one fixed user (ADR-0105 (r)).
# It writes no line to STUB_LOG, so a refused run still shows an empty log. Other arguments go to the real id.
make_stub_id() {
  cat >"$ROOT/stubs/id" <<'STUB'
#!/usr/bin/env bash
[ -z "${STUB_ID_FAIL:-}" ] || { echo "id: cannot find name for user ID" >&2; exit 1; }
case "$*" in
  -u) echo "${STUB_ID_UID:-999}" ;;
  -un) echo "${STUB_ID_NAME:-postgres}" ;;
  *) exec "$REAL_ID" "$@" ;;
esac
STUB
}

# The stub of comm records the files below BACKUP_PGPASS_DIR in the file STUB_LOG.snapshot when
# STUB_COMM_SNAPSHOT=1. Then it runs the real comm. It writes no line to STUB_LOG.
make_stub_comm() {
  cat >"$ROOT/stubs/comm" <<'STUB'
#!/usr/bin/env bash
[ -z "${STUB_COMM_SNAPSHOT:-}" ] || find "$BACKUP_PGPASS_DIR" -type f -printf 'FILE %f\n' >>"$STUB_LOG.snapshot"
exec "$REAL_COMM" "$@"
STUB
}

make_volume_stubs() {
  make_restore_stubs
  make_stub_pg_controldata
  make_stub_sync
  make_stub_flock
  make_stub_id
  make_stub_comm
  chmod 0755 "$ROOT/stubs/pg_controldata" "$ROOT/stubs/sync" "$ROOT/stubs/flock" "$ROOT/stubs/id" "$ROOT/stubs/comm"
}
