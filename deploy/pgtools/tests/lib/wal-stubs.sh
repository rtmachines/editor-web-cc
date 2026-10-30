# Sourced by walarchive-test. It writes the stubs of the external tools to $ROOT/stubs.
# Every stub appends "EVENT ...", "ARGV ...", and "ENV ..." lines to $STUB_LOG.

make_wal_stub_date() {
  cat >"$ROOT/stubs/date" <<'EOF'
#!/usr/bin/env bash
if [ "$*" = "-u +%Y%m%d_%H%M%S" ] && [ -n "${STUB_TS:-}" ]; then
  echo "$STUB_TS"
else
  exec "$REAL_DATE" "$@"
fi
EOF
}

# The stub of pg_basebackup creates a base backup directory with a label, as -Fp does.
#   STUB_START_SEG=n         the start segment number in the label, default 14
#   STUB_BASEBACKUP_FAIL=1   it leaves a partial directory and fails
#   STUB_BASEBACKUP_SLEEP=n  it waits n seconds before it writes
make_wal_stub_basebackup() {
  cat >"$ROOT/stubs/pg_basebackup" <<'EOF'
#!/usr/bin/env bash
dir=""
for a in "$@"; do
  case $a in --pgdata=*) dir=${a#--pgdata=} ;; esac
done
echo "EVENT pg_basebackup start" >>"$STUB_LOG"
echo "ARGV pg_basebackup $*" >>"$STUB_LOG"
echo "ENV PGHOST=${PGHOST:-} PGPORT=${PGPORT:-} PGUSER=${PGUSER:-}" >>"$STUB_LOG"
if [ -n "${PGPASSFILE:-}" ] && [ -f "$PGPASSFILE" ]; then
  cat "$PGPASSFILE" >"$STUB_DIR/pgpass.seen"
  tail -c 1 "$PGPASSFILE" | od -An -tx1 | tr -d ' \n' >"$STUB_DIR/pgpass.last"
  stat -c %a "$PGPASSFILE" >"$STUB_DIR/pgpass.mode"
  stat -c %a "$(dirname "$PGPASSFILE")" >"$STUB_DIR/pgpass.dirmode"
fi
sleep "${STUB_BASEBACKUP_SLEEP:-0}"
mkdir "$dir" || exit 1
mkdir "$dir/pg_wal"
if [ -n "${STUB_BASEBACKUP_FAIL:-}" ]; then
  printf 'partial' >"$dir/partial"
  echo "EVENT pg_basebackup failed" >>"$STUB_LOG"
  exit 1
fi
seg=${STUB_START_SEG:-14}
printf '16\n' >"$dir/PG_VERSION"
printf 'START WAL LOCATION: 0/%X000028 (file 0000000100000000%08X)\n' "$seg" "$seg" \
  >"$dir/backup_label"
printf '{"PostgreSQL-Backup-Manifest-Version": 1}\n' >"$dir/backup_manifest"
echo "EVENT pg_basebackup done" >>"$STUB_LOG"
EOF
}

#   STUB_VERIFY_FAIL=1  pg_verifybackup fails
# Like the real tool, it prints "backup successfully verified" to stdout on success unless --quiet is set.
make_wal_stub_verify() {
  cat >"$ROOT/stubs/pg_verifybackup" <<'EOF'
#!/usr/bin/env bash
dir=${!#}
quiet=""
for a in "$@"; do
  case $a in --quiet | -q) quiet=1 ;; esac
done
echo "EVENT pg_verifybackup $(basename "$dir")" >>"$STUB_LOG"
echo "ARGV pg_verifybackup $*" >>"$STUB_LOG"
if [ ! -f "$dir/backup_manifest" ]; then
  echo "no manifest" >&2
  exit 2
fi
if [ -n "${STUB_VERIFY_FAIL:-}" ]; then
  echo "pg_verifybackup: error: checksum mismatch" >&2
  exit 1
fi
[ -n "$quiet" ] || echo "backup successfully verified"
exit 0
EOF
}

# The stub of pg_archivecleanup deletes the segments before the named file, like the real tool.
# A name is compared without its timeline. A partial segment counts as its segment.
#   STUB_CLEANUP_FAIL=1  it deletes nothing and fails
make_wal_stub_cleanup() {
  cat >"$ROOT/stubs/pg_archivecleanup" <<'EOF'
#!/usr/bin/env bash
export LC_ALL=C
echo "EVENT pg_archivecleanup $2" >>"$STUB_LOG"
echo "ARGV pg_archivecleanup $*" >>"$STUB_LOG"
[ -z "${STUB_CLEANUP_FAIL:-}" ] || exit 1
for f in "$1"/*; do
  n=${f##*/}
  [[ $n =~ ^[0-9A-F]{24}(\.partial)?$ ]] || continue
  if [[ ${n:8:16} < ${2:8:16} ]]; then rm -- "$f"; fi
done
EOF
}

# The stubs of the two children of walarchive-entry.
#   STUB_RECEIVEWAL_EXIT=n   pg_receivewal exits with n at once. Without it, it sleeps.
#   STUB_SUPERCRONIC_SLEEP=n supercronic sleeps n seconds and exits 0. Without it, it exits at once.
make_wal_stub_children() {
  cat >"$ROOT/stubs/pg_receivewal" <<'EOF'
#!/usr/bin/env bash
echo "EVENT pg_receivewal start" >>"$STUB_LOG"
echo "ARGV pg_receivewal $*" >>"$STUB_LOG"
echo "ENV PGHOST=${PGHOST:-} PGPORT=${PGPORT:-} PGUSER=${PGUSER:-}" >>"$STUB_LOG"
if [ -n "${PGPASSFILE:-}" ] && [ -f "$PGPASSFILE" ]; then
  cat "$PGPASSFILE" >"$STUB_DIR/pgpass.seen"
  tail -c 1 "$PGPASSFILE" | od -An -tx1 | tr -d ' \n' >"$STUB_DIR/pgpass.last"
  stat -c %a "$PGPASSFILE" >"$STUB_DIR/pgpass.mode"
  stat -c %a "$(dirname "$PGPASSFILE")" >"$STUB_DIR/pgpass.dirmode"
fi
echo $$ >"$STUB_DIR/receivewal.pid"
[ -z "${STUB_RECEIVEWAL_EXIT:-}" ] || exit "$STUB_RECEIVEWAL_EXIT"
exec sleep 30
EOF
  cat >"$ROOT/stubs/supercronic" <<'EOF'
#!/usr/bin/env bash
echo "EVENT supercronic" >>"$STUB_LOG"
echo "ARGV supercronic $*" >>"$STUB_LOG"
echo "ENV PGPASSFILE=${PGPASSFILE:-unset}" >>"$STUB_LOG"
echo $$ >"$STUB_DIR/supercronic.pid"
[ -z "${STUB_SUPERCRONIC_SLEEP:-}" ] || exec sleep "$STUB_SUPERCRONIC_SLEEP"
exit 0
EOF
}

# The stubs of the scratch cluster of pitr-check.
#   psql: the command IDENTIFY_SYSTEM answers the identifier guard of walarchive-entry.
#     STUB_SYSID=n          the system identifier, default 7354987216845319027
#     STUB_IDENTIFY_FAIL=1  the connection fails
#     STUB_IDENTIFY_OUT=t   the whole answer line
#   psql: without STUB_PSQL_OK=1 every other command fails. With it, it answers the three checks of the script.
#     STUB_PSQL_RECOVERY=f  the cluster is still in recovery
#     STUB_PSQL_COUNT=n     the number of rows with the marker identity
#     STUB_PSQL_CHAIN=text  the result of the chain check
#   pg_ctl: start writes postmaster.pid and keeps the settings and the label for the test.
#     STUB_CTL_FAIL=1       start fails after it writes a line to the server log
make_wal_stub_scratch() {
  cat >"$ROOT/stubs/psql" <<'EOF'
#!/usr/bin/env bash
cmd=""
for a in "$@"; do
  case $a in --command=*) cmd=${a#--command=} ;; esac
done
case $cmd in
  IDENTIFY_SYSTEM)
    # The replication connection of the identifier guard. The log lines start with CALL and ARGV,
    # so that count_events does not count them as a started tool.
    echo "CALL psql identify" >>"$STUB_LOG"
    echo "ARGV psql-identify $*" >>"$STUB_LOG"
    if [ -n "${PGPASSFILE:-}" ] && [ -f "$PGPASSFILE" ]; then echo "CALL psql pgpass present" >>"$STUB_LOG"; fi
    [ -z "${STUB_IDENTIFY_FAIL:-}" ] || exit 2
    echo "${STUB_IDENTIFY_OUT:-${STUB_SYSID:-7354987216845319027}|1|0/3000060|}"
    exit 0
    ;;
esac
echo "EVENT psql" >>"$STUB_LOG"
echo "ARGV psql $*" >>"$STUB_LOG"
echo "ENV PGHOST=${PGHOST:-} PGPORT=${PGPORT:-} PGUSER=${PGUSER:-} PGPASSWORD=${PGPASSWORD:-} PGPASSFILE=${PGPASSFILE:-}" >>"$STUB_LOG"
[ -n "${STUB_PSQL_OK:-}" ] || exit 1
case $cmd in
  *pg_is_in_recovery*) if [ "${STUB_PSQL_RECOVERY:-t}" = t ]; then echo t; else echo f; fi ;;
  *audit_chain_head*) echo "${STUB_PSQL_CHAIN:-0|t|t}" ;;
  *audit_log*) echo "${STUB_PSQL_COUNT:-1}" ;;
  *) exit 1 ;;
esac
EOF
  cat >"$ROOT/stubs/pg_ctl" <<'EOF'
#!/usr/bin/env bash
action=${!#}
pgdata=""
log=""
for a in "$@"; do
  case $a in --pgdata=*) pgdata=${a#--pgdata=} ;; --log=*) log=${a#--log=} ;; esac
done
if [ "$action" = status ]; then
  [ -f "$pgdata/postmaster.pid" ]
  exit $?
fi
echo "EVENT pg_ctl $action" >>"$STUB_LOG"
echo "ARGV pg_ctl $*" >>"$STUB_LOG"
if [ "$action" = start ]; then
  cp "$pgdata/postgresql.auto.conf" "$STUB_DIR/auto.conf"
  cp "$pgdata/backup_label" "$STUB_DIR/label.seen"
  [ -f "$pgdata/recovery.signal" ] && echo "recovery.signal present" >>"$STUB_LOG"
  stat -c %a "$pgdata" >"$STUB_DIR/data.mode"
  echo "server log line of the recovery" >"$log"
  [ -z "${STUB_CTL_FAIL:-}" ] || exit 1
  echo 1 >"$pgdata/postmaster.pid"
else
  rm -f -- "$pgdata/postmaster.pid"
fi
EOF
}

make_wal_stubs() {
  mkdir -p "$ROOT/stubs"
  make_wal_stub_date
  make_wal_stub_basebackup
  make_wal_stub_verify
  make_wal_stub_cleanup
  make_wal_stub_children
  make_wal_stub_scratch
  # The startup regression tests cover delayed readiness and postmaster exits.
  printf '#!/usr/bin/env bash\nexit 0\n' >"$ROOT/stubs/pg_isready"
  chmod 0755 "$ROOT"/stubs/*
}
