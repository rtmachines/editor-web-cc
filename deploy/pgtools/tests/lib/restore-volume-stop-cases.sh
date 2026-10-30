# Sourced by restore-volume-test. It holds the cases of steps 9 and 10 of ADR-0103 (i) and of the
# failure path: the fast stop, the state shut down, sync, the last line, and the trap.
# It needs lib/restore-volume-cases.sh (the helpers and the VOLUME_SEQ_* sequences),
# lib/restore-volume-gate-cases.sh (volume_work_dir), and lib/restore-volume-start-cases.sh (volume_step_fails).

# Step 9: pg_ctl stop gets --mode=fast and the timeout of restore_volume.stop_timeout. pg_controldata
# reads the target. sync has no argument. The server runs once and stops once.
case_volume_stop_arguments() {
  new_volume_case stop-arguments
  volume_run
  local stop
  stop=$(grep '^ARGV pg_ctl .* stop$' "$CASE/stub/log")
  expect_eq "stop: exit code" "$RC" 0
  expect_eq "stop: the sequence" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_eq "stop: one stop call" "$(grep -c '^ARGV pg_ctl .* stop$' "$CASE/stub/log")" 1
  expect_eq "stop: the arguments" "$stop" "ARGV pg_ctl --pgdata=$CASE/pgdata --mode=fast -w -t 600 stop"
  expect_eq "stop: pg_controldata reads the target" \
    "$(grep '^ARGV pg_controldata ' "$CASE/stub/log")" "ARGV pg_controldata --pgdata=$CASE/pgdata"
  expect_eq "stop: sync has no argument" "$(grep '^ARGV sync' "$CASE/stub/log")" "ARGV sync "
  expect_no "stop: the server is stopped" test -e "$CASE/pgdata/postmaster.pid"
  expect_eq "stop: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "stop: no WARN line" "$(grep -c '"level":"WARN"' "$CASE/out" || true)" 0
}

# Step 9: a stop that fails gives one ERROR line. The trap tries the stop once more and stops at once.
# pg_controldata and sync do not run. The failed trap writes a WARN line and no second ERROR line.
case_volume_stop_fails() {
  new_volume_case stop-fails
  volume_step_fails "stop fails" "${VOLUME_SEQ_ACCESS}EVENT pg_ctl stop,EVENT pg_ctl stop," \
    "the temporary server did not stop" STUB_PGCTL_STOP_FAIL=1
  expect_eq "stop fails: pg_controldata did not run" "$(event_count 'pg_controldata')" 0
  expect_eq "stop fails: sync did not run" "$(event_count 'sync')" 0
  expect_eq "stop fails: the trap stops with --mode=immediate" \
    "$(grep -c -- "^ARGV pg_ctl --pgdata=$CASE/pgdata --mode=immediate -w -t 600 stop$" "$CASE/stub/log")" 1
  expect_eq "stop fails: one WARN line for the failed trap" \
    "$(grep -c '"level":"WARN","job":"restore-volume","message":"cannot stop the temporary server of this run"' "$CASE/out")" 1
}

# Step 9: a state other than shut down ends the run. sync does not run, and no last line appears.
case_volume_state_not_shut_down() {
  local state i=0
  for state in "in production" "shut down in recovery" "shutting down" "in crash recovery" "Shut down"; do
    i=$((i + 1))
    new_volume_case "state-$i"
    volume_step_fails "state ($state)" "${VOLUME_SEQ_ACCESS}EVENT pg_ctl stop,EVENT pg_controldata," \
      "the cluster did not shut down cleanly" "STUB_CONTROLDATA_STATE=$state"
    expect_ok "state ($state): the ERROR line holds the state" error_has "\"state\":\"$(printf '%s' "$state")\""
    expect_eq "state ($state): sync did not run" "$(event_count 'sync')" 0
  done
}

# Step 9: a failed pg_controldata ends the run. A system identifier that is no number ends the run.
case_volume_controldata_fails() {
  local value i=0
  new_volume_case controldata-fails
  volume_step_fails "pg_controldata fails" "${VOLUME_SEQ_ACCESS}EVENT pg_ctl stop,EVENT pg_controldata," \
    "pg_controldata failed" STUB_CONTROLDATA_FAIL=1
  expect_eq "pg_controldata fails: sync did not run" "$(event_count 'sync')" 0
  for value in abc "12 34" 7300000000000000001x 123456789012345678901 -5; do
    i=$((i + 1))
    new_volume_case "identifier-$i"
    volume_step_fails "identifier ($value)" "${VOLUME_SEQ_ACCESS}EVENT pg_ctl stop,EVENT pg_controldata," \
      "pg_controldata gave no system identifier" "STUB_SYSTEM_IDENTIFIER=$value"
    expect_eq "identifier ($value): sync did not run" "$(event_count 'sync')" 0
  done
}

# Step 10 needs sync first. A failed sync ends the run, and no last line appears.
case_volume_sync_fails() {
  new_volume_case sync-fails
  volume_step_fails "sync fails" "$VOLUME_SEQ_DONE" "sync failed" STUB_SYNC_FAIL=1
}

# Step 10: the last line is INFO restore volume completed with the set name and the system identifier.
case_volume_completed_line() {
  new_volume_case completed-line
  volume_run STUB_SYSTEM_IDENTIFIER=7300000000000000042
  local last
  last=$(tail -n 1 "$CASE/out")
  expect_eq "completed: exit code" "$RC" 0
  expect_eq "completed: one line" "$(grep -c 'restore volume completed' "$CASE/out")" 1
  expect_ok "completed: the line is the last line" grep -qF '"message":"restore volume completed"' <<<"$last"
  expect_ok "completed: the level is INFO" grep -qF '"level":"INFO","job":"restore-volume"' <<<"$last"
  expect_ok "completed: the line holds the set and the system identifier" \
    grep -qF "\"message\":\"restore volume completed\",\"set\":\"$RESTORE_BASE\",\"system_identifier\":\"7300000000000000042\"}" <<<"$last"
  expect_ok "completed: the output is JSON of the job restore-volume" volume_json_ok "$CASE/out"
  expect_eq "completed: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "completed: one line for the start" "$(grep -c 'restore volume started' "$CASE/out")" 1
  expect_eq "completed: the work directory is deleted" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  new_volume_case completed-default-id
  volume_run
  expect_ok "completed: the default system identifier" grep -qF '"system_identifier":"7300000000000000001"}' "$CASE/out"
}

# The failure path: a step that fails after the server started stops the server and deletes the work
# directory. It deletes no file of the target, also not a file that the check refused.
case_volume_trap_keeps_target() {
  new_volume_case trap-keeps-target
  local name
  volume_step_fails "failure keeps the target" "$VOLUME_SEQ_ACCESS" \
    "the top-level names of the target differ from the names after initdb" STUB_TOPLEVEL_ADD=planted
  expect_ok "failure keeps the target: the refused file stays" test -f "$CASE/pgdata/planted"
  for name in $(tr '|' ' ' <<<"$INITDB_NAMES"); do
    expect_ok "failure keeps the target: $name stays" test -e "$CASE/pgdata/$name"
  done
  expect_eq "failure keeps the target: the trap uses --mode=immediate" \
    "$(grep -c -- "^ARGV pg_ctl --pgdata=$CASE/pgdata --mode=immediate -w -t 600 stop$" "$CASE/stub/log")" 1
  new_volume_case trap-roles-differ
  volume_step_fails "roles differ" "$VOLUME_SEQ_ACCESS" \
    "the roles and the access of the new cluster differ from restore-roles.expected" \
    "STUB_ROLES_EXTRA=ROLE|planted|f"
  expect_ok "roles differ: PG_VERSION stays" test -f "$CASE/pgdata/PG_VERSION"
  expect_ok "roles differ: the cluster directories stay" test -d "$CASE/pgdata/base"
}

# The failure path before the server starts: no server runs, so the trap calls no pg_ctl.
# The work directory goes. A failed initdb leaves the target as initdb left it.
case_volume_trap_before_server() {
  new_volume_case trap-initdb-fails
  volume_step_fails "initdb fails" "EVENT list,EVENT initdb," "initdb failed" STUB_INITDB_FAIL=1
  expect_eq "initdb fails: no pg_ctl call" "$(grep -c '^ARGV pg_ctl ' "$CASE/stub/log" || true)" 0
  new_volume_case trap-start-fails
  volume_step_fails "start fails" "$VOLUME_SEQ_START" "the temporary server did not start" STUB_PGCTL_START_FAIL=1
  expect_eq "start fails: the trap calls no stop" "$(event_count 'pg_ctl stop')" 0
}

# The trap stops only a server of this run. A target with a live cluster stops the run in step 1,
# before the script owns anything. The trap then calls no pg_ctl and deletes no file.
case_volume_trap_live_cluster() {
  new_volume_case trap-live-cluster
  local before
  printf '16\n' >"$CASE/pgdata/PG_VERSION"
  printf '4242\n' >"$CASE/pgdata/postmaster.pid"
  before=$(target_listing)
  volume_run
  expect_no "live cluster: exit code is not 0" test "$RC" -eq 0
  expect_eq "live cluster: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "live cluster: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "live cluster: the target is unchanged" "$(target_listing)" "$before"
}

# A signal runs the trap. The server stops, the work directory goes, and the target keeps its files.
case_volume_trap_signal() {
  new_volume_case trap-signal
  env PATH="$ROOT/stubs:$PATH" BACKUP_DIR="$B" BACKUP_PGPASS_DIR="$CASE/pgpass" PGDATA="$CASE/pgdata" \
    RESTORE_CODE_DIR="$CASE/code" RESTORE_GLOBALS_DIR="$ROOT/etc" PGDATABASE=recipe \
    COMPOSE_SUBNET="$VOLUME_SUBNET" REAL_FLOCK="$REAL_FLOCK" LANG=en_US.utf8 STUB_LOG="$CASE/stub/log" \
    STUB_DIR="$CASE/stub" STUB_RESTORE_SLEEP=3 \
    bash "$BIN/restore-volume" "$RESTORE_NAME" >"$CASE/out" 2>"$CASE/err" &
  local pid=$! waited=0
  while ! grep -q '^EVENT pg_restore data' "$CASE/stub/log" && [ "$waited" -lt 50 ]; do
    sleep 0.2
    waited=$((waited + 1))
  done
  kill -TERM "$pid"
  wait "$pid"
  RC=$?
  expect_no "signal: exit code is not 0" test "$RC" -eq 0
  expect_eq "signal: the server is stopped once" "$(event_count 'pg_ctl stop')" 1
  expect_no "signal: no postmaster.pid stays" test -e "$CASE/pgdata/postmaster.pid"
  expect_eq "signal: the work directory is deleted" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_eq "signal: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "signal: the target keeps PG_VERSION" test -f "$CASE/pgdata/PG_VERSION"
}

# The static rules of steps 9 and 10 and of the trap.
case_volume_stop_static() {
  local script=$BIN/restore-volume
  expect_ok "stop static: the stop timeout names its row of 7.11" \
    grep -Eq '^STOP_TIMEOUT_SECONDS=600 #.*restore_volume\.stop_timeout' "$script"
  expect_ok "stop static: the script sets a trap on EXIT" grep -Eq '^trap [a-z_]+ EXIT$' "$script"
  expect_ok "stop static: a signal runs the trap" grep -Eq "^trap 'exit 143' TERM$" "$script"
  expect_no "stop static: no delete names the target" grep -Eq '(^|[[:space:];&(])rm[[:space:]].*(TARGET_DIR|PGDATA)' "$script"
  expect_no "stop static: no find -delete" grep -Eq 'find .*-delete' "$script"
  expect_no "stop static: no forbidden word" grep -Eq 'fsync|SCRATCH_PORT|--port|port=|ALTER ROLE' "$script"
  expect_ok "stop static: the last step logs the completed line" grep -qF 'log INFO "restore volume completed"' "$script"
}
