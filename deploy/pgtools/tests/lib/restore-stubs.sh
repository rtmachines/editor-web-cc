# Sourced by restore-test-test. It writes the stubs of the server tools to the stub directory.
#
# The stubs model a scratch server. They record each call in the file STUB_LOG:
#   EVENT <name>   one step of the script. The order of the EVENT lines is tested.
#   ARGV <tool> .. the arguments of the call. The password tests search this file.
# The stub of psql keeps the passwords that the script sets in STUB_DIR/server, one file for each
# role. A login over 127.0.0.1 passes when the pgpass file holds the stored password.
#
# Switches, set as VAR=value in the environment of one run:
#   STUB_LIST_EXTRA=<line>        one more entry in the output of pg_restore --list.
#   STUB_LIST_FAIL=1              pg_restore --list fails.
#   STUB_LIST_SLEEP=<seconds>     pg_restore --list waits.
#   STUB_INITDB_FAIL=1            initdb fails.
#   STUB_GLOBALS_SWAP=1           initdb replaces the globals file after its check.
#   STUB_PGCTL_START_FAIL=1       pg_ctl start fails.
#   STUB_PGCTL_STOP_FAIL=1        pg_ctl stop fails.
#   STUB_RESTORE_FAIL=<section>   pg_restore fails in that section (pre-data, data, post-data).
#   STUB_RESTORE_SLEEP=<seconds>  pg_restore waits in the section data.
#   STUB_GLOBALS_QUIET=1          the globals file gives no error (the system user is not postgres).
#   STUB_GLOBALS_OTHER_ERROR=1    the globals file gives one more error.
#   STUB_CODE_CHANGE_KIND=<kind>  the first line of that kind gets another hash.
#   STUB_TRIGGER_STATE=<char>     the first TRIGGER line holds the hash of its definition with that
#                                 tgenabled value (the synthetic file holds the state O).
#   STUB_CODE_DROP_KIND=<kind>    the lines of that kind are missing.
#   STUB_CODE_EXTRA=<line>        one more line in the output of restore-code.sql.
#   STUB_NO_DATABASE=1            the restore creates no database of the expected name.
#   STUB_NO_AUDIT_TABLE=1         the table audit_log does not exist.
#   STUB_CHAIN_RESULT=<text>      the answer of the audit chain check, default 0|t|t.
#   STUB_LOGIN_ANY=1              the server accepts each password.
#   STUB_LOGIN_NEVER=1            the server rejects each password.
# The files STUB_DIR/restored.counts and STUB_DIR/login-roles hold the answers of the count query
# and of the role query. STUB_DIR/listing holds the output of pg_restore --list.

make_stub_initdb() {
  cat >"$ROOT/stubs/initdb" <<'EOF'
#!/usr/bin/env bash
pgdata="" pwfile=""
for a in "$@"; do
  case $a in
    --pgdata=*) pgdata=${a#--pgdata=} ;;
    --pwfile=*) pwfile=${a#--pwfile=} ;;
  esac
done
echo "EVENT initdb" >>"$STUB_LOG"
echo "ARGV initdb $*" >>"$STUB_LOG"
if [ -n "${STUB_GLOBALS_SWAP:-}" ]; then
  for file in "$BACKUP_DIR"/globals_*.sql; do
    printf "%s\n" "COPY (SELECT 1) TO PROGRAM 'touch /tmp/x';" >"$file"
  done
fi
if [ -n "${STUB_INITDB_FAIL:-}" ]; then
  echo "initdb: error: could not create directory" >&2
  exit 1
fi
[ -s "$pwfile" ] && echo "PWFILE nonempty mode=$(stat -c %a "$pwfile")" >>"$STUB_LOG"
mkdir -p "$pgdata"
chmod 0700 "$pgdata"
printf '16\n' >"$pgdata/PG_VERSION"
: >"$pgdata/postgresql.conf"
exit 0
EOF
}

make_stub_pg_ctl() {
  cat >"$ROOT/stubs/pg_ctl" <<'EOF'
#!/usr/bin/env bash
pgdata="" action=""
for a in "$@"; do
  case $a in
    --pgdata=*) pgdata=${a#--pgdata=} ;;
    start | stop) action=$a ;;
  esac
done
echo "ARGV pg_ctl $*" >>"$STUB_LOG"
case $action in
  start)
    echo "EVENT pg_ctl start" >>"$STUB_LOG"
    [ -z "${STUB_PGCTL_START_FAIL:-}" ] || exit 1
    cp -- "$pgdata/postgresql.conf" "$STUB_DIR/server.conf"
    echo "$$" >"$pgdata/postmaster.pid"
    ;;
  stop)
    echo "EVENT pg_ctl stop" >>"$STUB_LOG"
    [ -z "${STUB_PGCTL_STOP_FAIL:-}" ] || exit 1
    "$REAL_RM" -f -- "$pgdata/postmaster.pid"
    ;;
esac
exit 0
EOF
}

make_stub_pg_restore() {
  cat >"$ROOT/stubs/pg_restore" <<'EOF'
#!/usr/bin/env bash
list=0 section="" dump=""
for a in "$@"; do
  case $a in
    --list) list=1 ;;
    --section=*) section=${a#--section=} ;;
  esac
  dump=$a
done
echo "ARGV pg_restore $*" >>"$STUB_LOG"
if [ "$list" -eq 1 ]; then
  echo "EVENT list" >>"$STUB_LOG"
  sleep "${STUB_LIST_SLEEP:-0}"
  [ -z "${STUB_LIST_FAIL:-}" ] || exit 1
  cat "$STUB_DIR/listing"
  [ -z "${STUB_LIST_EXTRA:-}" ] || printf '%s\n' "$STUB_LIST_EXTRA"
  exit 0
fi
echo "EVENT pg_restore $section" >>"$STUB_LOG"
[ -f "$dump" ] || exit 2
case $section in
  pre-data) echo pre >"$STUB_DIR/stage" ;;
  data) echo data >"$STUB_DIR/stage"; sleep "${STUB_RESTORE_SLEEP:-0}" ;;
  post-data) echo post >"$STUB_DIR/stage" ;;
esac
if [ "${STUB_RESTORE_FAIL:-}" = "$section" ]; then
  echo "pg_restore: error: could not execute query: ERROR: stub failure" >&2
  exit 1
fi
exit 0
EOF
}

# The stub of psql. It tells the calls apart by their arguments and by the text of the statement.
make_stub_psql_login() {
  cat >>"$ROOT/stubs/psql" <<'EOF'
if [ "$host" = 127.0.0.1 ]; then
  echo "EVENT login $user" >>"$STUB_LOG"
  echo "PGPASSFILE mode=$(stat -c %a "$PGPASSFILE")" >>"$STUB_LOG"
  given=$(awk -F: -v u="$user" '$4 == u { print $5 }' "$PGPASSFILE")
  stored=$(cat "$STUB_DIR/server/$user" 2>/dev/null)
  if [ -z "${STUB_LOGIN_NEVER:-}" ] && { [ -n "${STUB_LOGIN_ANY:-}" ] || [ "$given" = "$stored" ]; }; then
    if [ "$user" = recipe_replicator ]; then
      echo "psql: error: connection failed: FATAL:  permission denied for database \"$db\"" >&2
      exit 2
    fi
    exit 0
  fi
  echo "psql: error: connection failed: FATAL:  password authentication failed for user \"$user\"" >&2
  exit 2
fi
EOF
}

make_stub_psql_files() {
  cat >>"$ROOT/stubs/psql" <<'EOF'
case $file in
  *globals_*)
    echo "EVENT globals" >>"$STUB_LOG"
    cat -- "$file" >"$STUB_DIR/globals.received"
    printf '%s\n' "$file" >"$STUB_DIR/globals.path"
    stat -c %a -- "$file" >"$STUB_DIR/globals.mode"
    if [ -z "${STUB_GLOBALS_QUIET:-}" ]; then
      echo "psql:$file:3: ERROR:  role \"$(id -un)\" already exists" >&2
    fi
    if [ -n "${STUB_GLOBALS_OTHER_ERROR:-}" ]; then
      echo "psql:$file:9: ERROR:  syntax error at or near \"x\"" >&2
    fi
    exit 0
    ;;
  *restore-code.sql)
    stage=$(cat "$STUB_DIR/stage" 2>/dev/null)
    echo "EVENT code $stage" >>"$STUB_LOG"
    {
      changed="" disabled=""
      while IFS= read -r line; do
        kind=${line%%|*}
        [ "$stage" = post ] || case $kind in FUNCTION | CHECK | GENERATED) ;; *) continue ;; esac
        [ "$kind" != "${STUB_CODE_DROP_KIND:-}" ] || continue
        if [ "$kind" = "${STUB_CODE_CHANGE_KIND:-}" ] && [ -z "$changed" ]; then
          changed=1
          line=${line%|*}'|ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff'
        fi
        if [ "$kind" = TRIGGER ] && [ -n "${STUB_TRIGGER_STATE:-}" ] && [ -z "$disabled" ]; then
          disabled=1
          hash=$(printf 'trigger\n%s' "$STUB_TRIGGER_STATE" | sha256sum | cut -d' ' -f1)
          line=${line%|*}"|$hash"
        fi
        printf '%s\n' "$line"
      done < <(cat "$RESTORE_CODE_DIR/restore-code.expected"
        [ -z "${STUB_CODE_EXTRA:-}" ] || printf '%s\n' "$STUB_CODE_EXTRA")
    } | LC_ALL=C sort
    exit 0
    ;;
esac
EOF
}

make_stub_psql_statements() {
  cat >>"$ROOT/stubs/psql" <<'EOF'
input=""
if [ -z "$cmd" ] && [ -z "$file" ]; then
  input=$(cat)
fi
case "$cmd$input" in
  *'\gexec'*)
    echo "EVENT counts" >>"$STUB_LOG"
    cat "$STUB_DIR/restored.counts"
    ;;
  *'ALTER ROLE'*)
    role=$(printf '%s' "$input" | sed -n "s/^ALTER ROLE \([a-z_0-9]*\) PASSWORD '.*/\1/p")
    printf '%s' "$input" | sed -n "s/^ALTER ROLE [a-z_0-9]* PASSWORD '\(.*\)';\$/\1/p" \
      | tr -d '\n' >"$STUB_DIR/server/$role"
    echo "EVENT alter-role $role" >>"$STUB_LOG"
    ;;
  *rolcanlogin*) cat "$STUB_DIR/login-roles" ;;
  *pg_database*) if [ -n "${STUB_NO_DATABASE:-}" ]; then echo 0; else echo 1; fi ;;
  *to_regclass*) if [ -n "${STUB_NO_AUDIT_TABLE:-}" ]; then echo f; else echo t; fi ;;
  *audit_chain_head*)
    echo "EVENT chain" >>"$STUB_LOG"
    echo "${STUB_CHAIN_RESULT-0|t|t}"
    ;;
esac
exit 0
EOF
}

make_stub_psql() {
  cat >"$ROOT/stubs/psql" <<'EOF'
#!/usr/bin/env bash
host="" user="" cmd="" file="" db=""
for a in "$@"; do
  case $a in
    --host=*) host=${a#--host=} ;;
    --username=*) user=${a#--username=} ;;
    --command=*) cmd=${a#--command=} ;;
    --file=*) file=${a#--file=} ;;
    --dbname=*) db=${a#--dbname=} ;;
  esac
done
echo "ARGV psql $*" >>"$STUB_LOG"
echo "ENV psql PGHOST=${PGHOST-unset} PGPORT=${PGPORT-unset} PGUSER=${PGUSER-unset} PGPASSWORD=${PGPASSWORD+set}" >>"$STUB_LOG"
EOF
  make_stub_psql_login
  make_stub_psql_files
  make_stub_psql_statements
}

make_restore_stubs() {
  mkdir -p "$ROOT/stubs" "$ROOT/rmstub2"
  make_stub_initdb
  make_stub_pg_ctl
  make_stub_pg_restore
  make_stub_psql
  # The stub of rm refuses to delete a scratch run directory when STUB_RM_REFUSE_SCRATCH=1.
  cat >"$ROOT/rmstub2/rm" <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do
  case $a in */.scratch/run.*) [ -z "${STUB_RM_REFUSE_SCRATCH:-}" ] || exit 1 ;; esac
done
exec "$REAL_RM" "$@"
EOF
  chmod 0755 "$ROOT/stubs/initdb" "$ROOT/stubs/pg_ctl" "$ROOT/stubs/pg_restore" \
    "$ROOT/stubs/psql" "$ROOT/rmstub2/rm"
}
