# Sourced by restore-volume-test. It holds the cases of steps 5 and 6 of ADR-0103 (i): the temporary
# server and the restore of the globals file and the dump with the checks of restore-test.
# It needs lib/restore-volume-cases.sh (the helpers and the VOLUME_SEQ_* sequences) and
# lib/restore-volume-gate-cases.sh (volume_work_dir).

# volume_step_fails <name> <expected sequence> <text in the ERROR line> [VAR=value ...]: the case is
# ready. The run stops at one step with one ERROR line. The later tools do not run.
# The trap stops a server that runs. The helper adds that stop to the sequence, unless the sequence
# holds a stop already. A failure in the stop of step 9 therefore names its stops in the sequence.
# The trap also deletes the work directory and no file of the target.
volume_step_fails() {
  local name=$1 sequence=$2 text=$3 before
  shift 3
  before=$(set_listing)
  volume_run "$@"
  if grep -qx 'STATE server running' "$CASE/stub/log" && [[ $sequence != *"EVENT pg_ctl stop"* ]]; then
    sequence+="EVENT pg_ctl stop,"
    expect_no "$name: the server is stopped" test -e "$CASE/pgdata/postmaster.pid"
  fi
  expect_no "$name: exit code is not 0" test "$RC" -eq 0
  expect_eq "$name: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$name: the output is JSON of the job restore-volume" volume_json_ok "$CASE/out"
  expect_ok "$name: the ERROR line holds [$text]" error_has "$text"
  expect_eq "$name: the sequence" "$(event_sequence)" "$sequence"
  expect_eq "$name: the backup directory is unchanged" "$(set_listing)" "$before"
  expect_eq "$name: the work directory is deleted" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_no "$name: no completed line" grep -q 'restore volume completed' "$CASE/out"
}

# Step 5: pg_ctl start waits 60 s (restore_volume.start_timeout of 7.11). The options of design D4
# go through -o. The server log is in the work directory and not in the target.
case_volume_start_arguments() {
  new_volume_case start-arguments
  volume_run
  local work argv
  work=$(volume_work_dir)
  argv=$(grep '^ARGV pg_ctl .* start$' "$CASE/stub/log")
  expect_eq "start: exit code" "$RC" 0
  expect_eq "start: one pg_ctl start call" "$(grep -c '^ARGV pg_ctl .* start$' "$CASE/stub/log")" 1
  expect_ok "start: the target is the data directory" grep -qF -- " --pgdata=$CASE/pgdata " <<<"$argv "
  expect_ok "start: wait and timeout 60" grep -qF -- ' -w -t 60 ' <<<"$argv"
  expect_ok "start: the log is in the work directory" grep -qF -- " --log=$work/server.log " <<<"$argv "
  expect_ok "start: the action is start" grep -qE -- ' start$' <<<"$argv"
  expect_ok "start: no TCP listener" grep -qF -- "-c listen_addresses='' " <<<"$argv "
  expect_ok "start: the private socket directory is in the work directory" \
    grep -qF -- "-c unix_socket_directories='$work/sock' " <<<"$argv "
  expect_ok "start: shared_buffers" grep -qF -- '-c shared_buffers=64MB ' <<<"$argv "
  expect_ok "start: maintenance_work_mem" grep -qF -- '-c maintenance_work_mem=64MB ' <<<"$argv "
  expect_ok "start: max_connections" grep -qF -- '-c max_connections=10 ' <<<"$argv "
  expect_ok "start: max_parallel_workers_per_gather" grep -qF -- '-c max_parallel_workers_per_gather=0 ' <<<"$argv "
  expect_ok "start: max_parallel_maintenance_workers" grep -qF -- '-c max_parallel_maintenance_workers=0 ' <<<"$argv "
  expect_eq "start: five sizes, the socket, and the listener are the only settings" \
    "$(grep -o -- ' -c ' <<<"$argv " | wc -l)" 7
  expect_eq "start: the socket directory has mode 700" "$(stat -c %a -- "$(volume_work_copy)/sock")" 700
  expect_ok "start: pg_ctl does not inherit the lock descriptor" grep -qx 'FD9 pg_ctl closed' "$CASE/stub/log"
}

# Step 5: no call names fsync, a TCP port, or a log path in the target. The server reads no setting
# from a configuration file that the script wrote.
case_volume_start_no_forbidden_option() {
  new_volume_case start-forbidden
  volume_run
  expect_eq "forbidden: exit code" "$RC" 0
  expect_no "forbidden: fsync in a call" grep -qi 'fsync' "$CASE/stub/log"
  expect_no "forbidden: a TCP port in a call" grep -Eq -- '(^| )(--port|-p)[= ]|[ =]port=' "$CASE/stub/log"
  expect_no "forbidden: a TCP address in a call" grep -Eq -- '127\.0\.0\.1|listen_addresses=[^'"'"' ]' "$CASE/stub/log"
  expect_no "forbidden: a log path in the target" grep -qF -- "--log=$CASE/pgdata" "$CASE/stub/log"
  expect_no "forbidden: a log file in the target" test -e "$CASE/pgdata/server.log"
  expect_eq "forbidden: postgresql.conf is as initdb left it" \
    "$(cat "$CASE/pgdata/postgresql.conf")" "# stub of initdb: postgresql.conf"
  expect_eq "forbidden: the server read the file of initdb" \
    "$(cat "$CASE/stub/server.conf")" "# stub of initdb: postgresql.conf"
  expect_eq "forbidden: postgresql.auto.conf is as initdb left it" \
    "$(cat "$CASE/pgdata/postgresql.auto.conf")" "# stub of initdb: postgresql.auto.conf"
}

# Step 5: a server that does not start gives one ERROR line. No restore tool runs.
case_volume_start_fails() {
  new_volume_case start-fails
  volume_step_fails "start fails" "$VOLUME_SEQ_START" "the temporary server did not start" STUB_PGCTL_START_FAIL=1
  expect_eq "start fails: no restore ran" "$(event_count 'pg_restore')" 0
  expect_eq "start fails: no psql ran" "$(grep -c '^ARGV psql ' "$CASE/stub/log" || true)" 0
}

# Step 6: the order of restore-test. The globals file, pre-data, the code of the first kinds, data,
# post-data, the code of every kind, the counts, and the chain. No call sets a role password.
case_volume_restore_order() {
  new_volume_case restore-order
  volume_run
  local work psql_calls
  work=$(volume_work_dir)
  psql_calls=$(grep -c '^ARGV psql ' "$CASE/stub/log")
  expect_eq "order: exit code" "$RC" 0
  expect_eq "order: the sequence" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_eq "order: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "order: the globals file is the private copy" "$(cat "$CASE/stub/globals.path")" \
    "$work/globals_$RESTORE_NAME.sql"
  expect_eq "order: the globals copy has mode 600" "$(cat "$CASE/stub/globals.mode")" 600
  expect_eq "order: no password is set" "$(event_count 'alter-role')" 0
  expect_no "order: no ALTER ROLE in a call" grep -Eqi 'alter role|rolpassword' "$CASE/stub/log"
  expect_eq "order: no login over TCP" "$(event_count 'login')" 0
  expect_eq "order: no login role list" "$(grep -c 'rolcanlogin' "$CASE/stub/log" || true)" 0
  expect_eq "order: every psql call uses the private socket" \
    "$(grep -c "^ARGV psql .*--host=$work/sock " "$CASE/stub/log")" "$psql_calls"
  expect_eq "order: psql runs as the system user" \
    "$(grep -c -- "^ARGV psql .*--username=$(id -un) " "$CASE/stub/log")" "$psql_calls"
  expect_eq "order: every restore call uses the private socket" \
    "$(grep -c "^ARGV pg_restore .*--host=$work/sock " "$CASE/stub/log")" 3
  expect_ok "order: the pre-data restore creates the database" \
    grep -q -- '--section=pre-data --create --dbname=postgres ' "$CASE/stub/log"
}

# Step 6: the helpers come from the library that both scripts source (design D3).
case_volume_start_library() {
  local name
  for name in scratch_psql restore_globals check_database; do
    expect_ok "start library: the library defines $name" \
      bash -c 'source "$1" && declare -F -- "$2" >/dev/null' _ "$ROOT/etc/restore-steps.sh" "$name"
    expect_no "start library: restore-test does not define $name" grep -Eq "^$name\(\) \{" "$BIN/restore-test"
    expect_no "start library: restore-volume does not define $name" grep -Eq "^$name\(\) \{" "$BIN/restore-volume"
  done
  expect_ok "start static: the start timeout names its row of 7.11" \
    grep -Eq '^START_TIMEOUT_SECONDS=60 #.*restore_volume\.start_timeout' "$BIN/restore-volume"
  expect_no "start static: restore-volume names a port" grep -Eq 'SCRATCH_PORT|--port|port=' "$BIN/restore-volume"
  expect_no "start static: restore-volume sets a password" grep -Eqi 'alter role|set_passwords' "$BIN/restore-volume"
}

# Step 1: a bad name of the database stops the run before any tool. DB_NAME enters SQL.
case_volume_bad_database_name() {
  local value i=0
  for value in "bad-name" "Recipe" "x;y" "a b" "1abc"; do
    i=$((i + 1))
    failed_volume_case "bad-database-$i" "PGDATABASE" "$(date -u +%Y%m%d_%H%M)" "PGDATABASE=$value"
  done
}

# Step 6: each failing tool gives one ERROR line, and the later tools do not run.
case_volume_restore_failures() {
  local section sequence
  new_volume_case restore-globals-error
  volume_step_fails "globals other error" "$VOLUME_SEQ_GLOBALS" \
    "the globals file gave an error that is not known" STUB_GLOBALS_OTHER_ERROR=1
  new_volume_case restore-globals-quiet
  volume_run STUB_GLOBALS_QUIET=1
  expect_eq "globals quiet: exit code" "$RC" 0
  for section in pre-data data post-data; do
    new_volume_case "restore-fails-$section"
    case $section in
      pre-data) sequence="${VOLUME_SEQ_GLOBALS}EVENT pg_restore pre-data," ;;
      data) sequence="${VOLUME_SEQ_PRE}EVENT pg_restore data," ;;
      post-data) sequence="${VOLUME_SEQ_PRE}EVENT pg_restore data,EVENT pg_restore post-data," ;;
    esac
    volume_step_fails "pg_restore fails in $section" "$sequence" "pg_restore failed" "STUB_RESTORE_FAIL=$section"
    expect_ok "pg_restore fails in $section: the ERROR line holds the section" error_has "\"section\":\"$section\""
  done
  new_volume_case restore-no-database
  volume_step_fails "no database" "${VOLUME_SEQ_GLOBALS}EVENT pg_restore pre-data," \
    "the dump did not create the database of the expected name" STUB_NO_DATABASE=1
}

# Step 6: the code comparisons run after pre-data and after post-data, as in restore-test.
case_volume_code_comparison() {
  local text="differs from restore-code.expected"
  new_volume_case code-function-changed
  volume_step_fails "changed function" "$VOLUME_SEQ_PRE" "the code after the restore of pre-data $text" \
    STUB_CODE_CHANGE_KIND=FUNCTION
  new_volume_case code-trigger-changed
  volume_step_fails "changed trigger" "$VOLUME_SEQ_POST" "the code after the restore of post-data $text" \
    STUB_CODE_CHANGE_KIND=TRIGGER
  new_volume_case code-trigger-disabled
  volume_step_fails "disabled trigger" "$VOLUME_SEQ_POST" "the code after the restore of post-data $text" \
    STUB_TRIGGER_STATE=D
  new_volume_case code-extra-line
  volume_step_fails "extra line" "$VOLUME_SEQ_POST" "the code after the restore of post-data $text" \
    "STUB_CODE_EXTRA=TRIGGER|public machine planted|recipe_migrator|$(hash_of planted)"
  new_volume_case code-dropped-kind
  volume_step_fails "dropped kind" "$VOLUME_SEQ_PRE" "the code after the restore of pre-data $text" \
    STUB_CODE_DROP_KIND=CHECK
}

# Step 6: the counts equal the counts file in both directions. The chain check must pass.
case_volume_counts_and_chain() {
  new_volume_case counts-differ
  printf 'public.machine|4\npublic.recipe|5\n' >"$CASE/stub/restored.counts"
  volume_step_fails "count differs" "${VOLUME_SEQ_POST}EVENT counts," \
    "the row count of a table differs from the counts file"
  expect_ok "count differs: the ERROR line names the table" error_has '"table":"public.machine"'
  new_volume_case counts-missing-table
  printf 'public.machine|3\n' >"$CASE/stub/restored.counts"
  volume_step_fails "missing table" "${VOLUME_SEQ_POST}EVENT counts," \
    "a table of the counts file is missing in the restore"
  new_volume_case counts-extra-table
  printf 'public.machine|3\npublic.recipe|5\npublic.planted|1\n' >"$CASE/stub/restored.counts"
  volume_step_fails "extra table" "${VOLUME_SEQ_POST}EVENT counts," "a restored table is not in the counts file"
  new_volume_case chain-broken
  volume_step_fails "chain broken" "$VOLUME_SEQ_END" "the audit chain check failed" STUB_CHAIN_RESULT='2|t|t'
  new_volume_case chain-head
  volume_step_fails "chain head" "$VOLUME_SEQ_END" "the audit chain check failed" STUB_CHAIN_RESULT='0|f|t'
  new_volume_case chain-no-table
  volume_step_fails "no audit table" "${VOLUME_SEQ_POST}EVENT counts," \
    "the restored database has tables but no audit_log table" STUB_NO_AUDIT_TABLE=1
}
