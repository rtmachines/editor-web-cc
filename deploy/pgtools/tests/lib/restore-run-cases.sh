# Sourced by restore-test-test. It holds the cases of a whole run: the happy path, the arguments of
# the tools, the passwords, the failures of the tools, and the cleanup (ADR-0067 (c) to (g)).

# json_shape_ok <file>: every line is one JSON object of the log format.
json_shape_ok() {
  ! grep -qvE '^\{"timestamp":"[0-9TZ:-]+","level":"(INFO|WARN|ERROR)","job":"restore-test","message":"[^"]+"(,"[a-z_]+":("[^"]*"|[0-9]+))*\}$' "$1"
}

case_restore_happy_path() {
  new_restore_case happy
  local before
  before=$(set_listing)
  restore_run
  expect_eq "happy: exit code" "$RC" 0
  expect_eq "happy: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "happy: no output on stderr" "$(wc -c <"$CASE/err")" 0
  expect_ok "happy: every output line is a JSON object" json_shape_ok "$CASE/out"
  expect_eq "happy: last line is the result" \
    "$(tail -n 1 "$CASE/out" | sed 's/^{"timestamp":"[^"]*",//')" \
    "\"level\":\"INFO\",\"job\":\"restore-test\",\"message\":\"restore test passed\",\"set\":\"$RESTORE_BASE\",\"tables\":2,\"roles\":4}"
  expect_eq "happy: order of events" "$(event_sequence)" \
    "${SEQ_TO_POST_DATA}EVENT counts,EVENT chain,EVENT login recipe_app,EVENT login recipe_app,EVENT login recipe_backup,EVENT login recipe_backup,EVENT login recipe_migrator,EVENT login recipe_migrator,EVENT login recipe_replicator,EVENT login recipe_replicator,EVENT pg_ctl stop,"
  expect_eq "happy: marker, lock, and set files are unchanged" "$(set_listing)" "$before"
  expect_ok "happy: the scratch directory root stays" test -d "$B/.scratch"
  expect_eq "happy: no scratch run directory stays" "$(scratch_runs)" 0
  expect_eq "happy: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_eq "happy: the server is stopped" "$(event_count 'pg_ctl start')" "$(event_count 'pg_ctl stop')"
}

# The initdb call has the arguments of reliability.md 7.1 and ADR-0067 (c).
case_restore_initdb_arguments() {
  new_restore_case initdb-arguments
  restore_run
  local argv flag
  argv=$(grep '^ARGV initdb' "$CASE/stub/log")
  for flag in --data-checksums --lc-collate=C --lc-ctype=C.UTF-8 --auth-local=peer \
    --auth-host=scram-sha-256 "--username=$(id -un)"; do
    expect_ok "initdb: argument $flag" grep -qF -- " $flag" <<<"$argv"
  done
  expect_ok "initdb: the data directory is below the scratch directory" \
    grep -qE -- " --pgdata=$B/\.scratch/run\.[A-Za-z0-9]+/data( |\$)" <<<"$argv"
  expect_ok "initdb: the password file is a file of the pgpass directory" \
    grep -qE -- " --pwfile=$CASE/pgpass/run\.[A-Za-z0-9]+/[A-Za-z0-9._-]+( |\$)" <<<"$argv"
  expect_ok "initdb: the password file is not empty and has mode 600" \
    grep -qx 'PWFILE nonempty mode=600' "$CASE/stub/log"
}

# The scratch server uses the settings scratch.* of reliability.md 7.11 and listens on 127.0.0.1 only.
case_restore_server_settings() {
  new_restore_case server-settings
  restore_run
  local setting
  for setting in shared_buffers=64MB maintenance_work_mem=64MB max_connections=10 \
    max_parallel_workers_per_gather=0 max_parallel_maintenance_workers=0 fsync=off \
    wal_level=minimal max_wal_senders=0 "listen_addresses='127.0.0.1'" port=5433 \
    unix_socket_permissions=0700; do
    expect_ok "settings: $setting" grep -qxF -- "$setting" "$CASE/stub/server.conf"
  done
  expect_ok "settings: the socket directory is below the scratch directory" \
    grep -qE "^unix_socket_directories='$B/\.scratch/run\.[A-Za-z0-9]+/sock'\$" "$CASE/stub/server.conf"
  expect_eq "settings: one listen_addresses line" \
    "$(grep -c '^listen_addresses' "$CASE/stub/server.conf")" 1
}

# No call reaches another server than the scratch cluster. The environment of the production
# connection is gone, and every call names its host and port.
case_restore_no_production_connection() {
  new_restore_case no-production
  run_script "RESTORE_CODE_DIR=$CASE/code" PGDATABASE=recipe PGHOST=db PGPORT=5432 PGUSER=recipe_backup \
    PGPASSWORD=production PGPASSFILE=/nonexistent -- restore-test
  expect_eq "no production: exit code" "$RC" 0
  expect_eq "no production: no call sees the production environment" \
    "$(grep -c '^ENV psql PGHOST=unset PGPORT=unset PGUSER=unset PGPASSWORD=$' "$CASE/stub/log")" \
    "$(grep -c '^ENV psql' "$CASE/stub/log")"
  expect_eq "no production: every psql call names its host" \
    "$(grep '^ARGV psql' "$CASE/stub/log" | grep -vc -- '--host=')" 0
  expect_eq "no production: every pg_restore call except the list names its host" \
    "$(grep '^ARGV pg_restore' "$CASE/stub/log" | grep -v -- '--list' | grep -vc -- '--host=')" 0
  expect_eq "no production: no call names the host db" "$(grep -c -- '--host=db' "$CASE/stub/log")" 0
  expect_eq "no production: no call uses port 5432" "$(grep -c -- '--port=5432' "$CASE/stub/log")" 0
}

# The passwords are random, differ for each role, and never reach argv, stdout, or stderr.
case_restore_passwords() {
  new_restore_case passwords
  restore_run
  local file pw count=0
  for file in "$CASE"/stub/server/*; do
    pw=$(cat "$file")
    count=$((count + 1))
    expect_ok "passwords: ${file##*/} is long" test "${#pw}" -ge 32
    expect_no "passwords: ${file##*/} is in the log of the calls" grep -qF -- "$pw" "$CASE/stub/log"
    expect_no "passwords: ${file##*/} is on stdout" grep -qF -- "$pw" "$CASE/out"
    expect_no "passwords: ${file##*/} is on stderr" grep -qF -- "$pw" "$CASE/err"
  done
  expect_eq "passwords: one password for each role" "$count" 4
  expect_eq "passwords: four different values" \
    "$(for file in "$CASE"/stub/server/*; do cat "$file"; echo; done | sort -u | wc -l)" 4
  local first
  first=$(cat "$CASE/stub/server/recipe_app")
  restore_run
  expect_no "passwords: a second run sets another password" \
    test "$(cat "$CASE/stub/server/recipe_app")" = "$first"
}

# Only the error "role <system user> already exists" is known (ADR-0033). Another error stops the
# test. The gate of the globals file has its own cases (restore-globals-cases.sh).
case_restore_globals() {
  failed_restore_case globals-other-error STUB_GLOBALS_OTHER_ERROR=1
  expect_ok "globals other error: the ERROR line names the globals file" error_has "globals"
  expect_eq "globals other error: stops before the passwords" \
    "$(event_sequence)" "EVENT list,EVENT initdb,EVENT pg_ctl start,EVENT globals,EVENT pg_ctl stop,"
  new_restore_case globals-quiet
  restore_run STUB_GLOBALS_QUIET=1
  expect_eq "globals quiet: exit code" "$RC" 0
}

# Each failing tool gives one ERROR line, stops the server, and deletes the scratch directory.
case_restore_tool_failures() {
  failed_restore_case initdb-fails STUB_INITDB_FAIL=1
  expect_eq "initdb fails: no server started" "$(event_count 'pg_ctl')" 0
  failed_restore_case server-start-fails STUB_PGCTL_START_FAIL=1
  expect_eq "server start fails: nothing was restored" "$(event_count 'pg_restore pre-data')" 0
  local section
  for section in pre-data data post-data; do
    failed_restore_case "restore-fails-$section" "STUB_RESTORE_FAIL=$section"
    expect_ok "restore fails ($section): the ERROR line names the section" error_has "$section"
  done
  failed_restore_case no-database STUB_NO_DATABASE=1
  expect_eq "no database: the data load did not start" "$(event_count 'pg_restore data')" 0
}

# The trap deletes the scratch directory. A failed delete is an error, and the run exits with 1.
case_restore_cleanup_fails() {
  new_restore_case cleanup-fails
  local before
  before=$(set_listing)
  restore_run STUB_RM_REFUSE_SCRATCH=1 "PATH=$ROOT/rmstub2:$ROOT/stubs:$PATH"
  expect_eq "cleanup fails: exit code" "$RC" 1
  expect_eq "cleanup fails: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "cleanup fails: the ERROR line names the scratch directory" \
    error_has "cannot delete the scratch directory"
  expect_eq "cleanup fails: marker, lock, and set files are unchanged" "$(set_listing)" "$before"
  expect_eq "cleanup fails: the server is stopped" "$(event_count 'pg_ctl stop')" 1
}

# A signal runs the trap. The server stops and the scratch directory goes.
case_restore_signal() {
  new_restore_case signal
  env PATH="$ROOT/stubs:$PATH" BACKUP_DIR="$B" BACKUP_PGPASS_DIR="$CASE/pgpass" \
    RESTORE_CODE_DIR="$CASE/code" PGDATABASE=recipe STUB_LOG="$CASE/stub/log" \
    STUB_DIR="$CASE/stub" STUB_RESTORE_SLEEP=3 \
    bash "$BIN/restore-test" >"$CASE/out" 2>"$CASE/err" &
  local pid=$! waited=0
  while ! grep -q '^EVENT pg_restore data' "$CASE/stub/log" && [ "$waited" -lt 50 ]; do
    sleep 0.2
    waited=$((waited + 1))
  done
  kill -TERM "$pid"
  wait "$pid"
  RC=$?
  expect_no "signal: exit code is not 0" test "$RC" -eq 0
  expect_eq "signal: the server is stopped" "$(event_count 'pg_ctl stop')" 1
  expect_eq "signal: no scratch run directory stays" "$(scratch_runs)" 0
  expect_eq "signal: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}
