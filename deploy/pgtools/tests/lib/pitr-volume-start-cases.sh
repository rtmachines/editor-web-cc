# Sourced by pitr-volume-test. It holds the cases of step 8 of ADR-0106 (f): the file recovery.signal and
# the start of the temporary server with pg_ctl -o.
# A case with a good start runs all 12 steps. The later steps change the target (the promotion removes
# recovery.signal, the stop removes postmaster.pid). A case that needs the target at the start of the
# server reads the records that the stub of pg_ctl writes at that moment (see lib/pitr-volume-stubs.sh).
# The stub of pg_ctl logs the argument of -o in the line PGCTL_O and the wait in the line PGCTL_TIMEOUT.
# A start that fails or that loses its postmaster must print the tail of the server log and one ERROR line.
# It needs lib/helpers.sh, lib/pitr-volume-stubs.sh, and lib/pitr-volume-copy-cases.sh (new_pitr_copy_case,
# pitr_hashed, pitr_tool_calls, COPY_CONFIG_FILES).

START_TARGET_TIME=2026-01-01T00:20:00Z
START_LOG_TAIL=20

# pitr_start_run [VAR=value ...] -- [argument ...]: run the script with the stubs.
pitr_start_run() {
  pitr_run "$@"
}

# pitr_stub_value <name>: the text after the name in the first line of the tool log that starts with it.
pitr_stub_value() { sed -n "s/^$1 //p" "$CASE/stub/log" | sed -n '1p'; }

# pitr_options_have <text>: the argument of -o of the start holds the text.
pitr_options_have() { pitr_stub_value PGCTL_O | grep -qF -- "$1"; }

# pitr_socket_dir: the socket directory that the argument of -o names.
pitr_socket_dir() { pitr_stub_value PGCTL_O | sed -n 's/.*-c unix_socket_directories=\([^ ]*\).*/\1/p'; }

# A run without a target. The one start passes the sizes, the restore_command of ADR-0068 (l), and no target.
case_pitr_start_options() {
  local setting
  new_pitr_copy_case start-options
  pitr_start_run --
  expect_eq "start: exit code" "$RC" 0
  expect_eq "start: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "start: the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
  expect_ok "start: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_eq "start: one start of the server" "$(grep -c '^PGCTL_ACTION start$' "$CASE/stub/log" || true)" 1
  expect_ok "start: pg_ctl waits and reads the target" \
    grep -qE "^ARGV pg_ctl --pgdata=$CASE/target --log=\S+ --wait --timeout=[0-9]+ -o .* start\$" "$CASE/stub/log"
  expect_ok "start: the restore_command of ADR-0068 (l) for the WAL directory" \
    pitr_options_have "-c \"restore_command=cp $CASE/wal/%f %p || cp $CASE/wal/%f.partial %p\""
  for setting in hot_standby=off archive_mode=off "listen_addresses=''" port=5433 shared_buffers=64MB \
    maintenance_work_mem=64MB max_connections=10 max_parallel_workers_per_gather=0 max_parallel_maintenance_workers=0; do
    expect_ok "start: the options hold -c $setting" pitr_options_have "-c $setting"
  done
  expect_no "start: no recovery target without --target-time" pitr_options_have recovery_target
  expect_no "start: no fsync option" pitr_options_have fsync
  expect_ok "start: the socket directory is a run directory below the private directory" \
    test "$(pitr_socket_dir)" != "" -a "$(dirname "$(pitr_socket_dir)")" = "$CASE/pgpass"
  expect_ok "start: the connection check uses that socket directory" \
    grep -qxF "ARGV pg_isready --quiet --host=$(pitr_socket_dir) --port=5433 --username=postgres --dbname=recipe --timeout=1" "$CASE/stub/log"
}

# A run with a target: the three target settings of ADR-0106 (a), each once.
case_pitr_start_target() {
  local setting
  new_pitr_copy_case start-target
  pitr_start_run -- --target-time "$START_TARGET_TIME"
  expect_eq "target start: exit code" "$RC" 0
  for setting in "recovery_target_time=$START_TARGET_TIME" recovery_target_inclusive=off recovery_target_action=promote; do
    expect_ok "target start: the options hold -c $setting" pitr_options_have "-c $setting"
  done
  expect_eq "target start: three target settings only" "$(pitr_stub_value PGCTL_O | grep -o 'recovery_target' | wc -l)" 3
  expect_ok "target start: the restore_command stays" pitr_options_have "-c \"restore_command=cp $CASE/wal/%f %p"
  new_pitr_copy_case start-drill
  pitr_start_run -- --target-time "$START_TARGET_TIME" --drill
  expect_eq "drill start: exit code" "$RC" 0
  expect_ok "drill start: the target time is set" pitr_options_have "-c recovery_target_time=$START_TARGET_TIME"
}

# No configuration file of the target changes. At the start of the server the target holds recovery.signal
# and nothing else beyond the base backup. The record of the stub of pg_ctl gives that state. The server log and
# the socket stay below the private directory.
case_pitr_start_files() {
  local file base_list target_list
  new_pitr_copy_case start-files
  pitr_start_run -- --target-time "$START_TARGET_TIME"
  expect_eq "files: exit code" "$RC" 0
  expect_ok "files: the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
  expect_eq "files: recovery.signal is a regular file at the start" "$(pitr_stub_value START_RECOVERY_SIGNAL)" file
  expect_eq "files: recovery.signal is empty at the start" "$(pitr_stub_value START_RECOVERY_SIZE)" 0
  expect_eq "files: no standby.signal at the start" "$(pitr_stub_value START_STANDBY_SIGNAL)" absent
  expect_no "files: no standby.signal" test -e "$CASE/target/standby.signal"
  for file in "${COPY_CONFIG_FILES[@]}"; do
    expect_eq "files: $file keeps its SHA-256 value at the start" \
      "$(grep " $file\$" "$CASE/stub/start-hashes" | cut -d' ' -f1)" "$(pitr_hashed "${file//./_}")"
    expect_eq "files: $file keeps its SHA-256 value at the end" \
      "$(sha256sum "$CASE/target/$file" | cut -d' ' -f1)" "$(pitr_hashed "${file//./_}")"
  done
  expect_eq "files: no setting in a file of the target" "$(grep -rlE 'restore_command|recovery_target|hot_standby|listen_addresses' "$CASE/target" | wc -l)" 0
  base_list=$(cd "$CASE/wal/base/$BASE_A" && find . -mindepth 1 ! -name backup_end_time -printf '%p %y %s\n' | LC_ALL=C sort)
  target_list=$(grep -v '^\./recovery\.signal ' "$CASE/stub/start-listing")
  expect_eq "files: at the start the target holds the base backup and recovery.signal only" "$target_list" "$base_list"
  expect_eq "files: at the start the listing holds recovery.signal" "$(grep -c '^\./recovery\.signal f 0$' "$CASE/stub/start-listing")" 1
  expect_eq "files: the work directory is gone" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# The wait of pg_ctl is pitr_volume.start_timeout. The constants name their rows of reliability.md 7.11.
case_pitr_start_timeout() {
  local timeout
  new_pitr_copy_case start-timeout
  pitr_start_run --
  timeout=$(pitr_stub_value PGCTL_TIMEOUT)
  expect_ok "timeout: the wait is 1800 s less the seconds that passed" test "${timeout:-0}" -ge 1795 -a "${timeout:-0}" -le 1800
  expect_ok "timeout: the constant of the start names its row" \
    grep -qE '^START_TIMEOUT_SECONDS=1800 # pitr_volume\.start_timeout of reliability\.md' "$BIN/pitr-volume"
  expect_ok "timeout: the constant of the poll names its row" \
    grep -qE '^READY_POLL_SECONDS=1 # pitr_check\.ready_poll of reliability\.md' "$BIN/pitr-volume"
  expect_ok "timeout: the constant of the log tail names its row" \
    grep -qE '^LOG_TAIL_LINES=20 # pitr_check\.log_tail of reliability\.md' "$BIN/pitr-volume"
}

# The server does not hold the lock of the WAL archive, and this script keeps it. pg_ctl runs without
# descriptor 9. The lock is still held while pg_ctl runs and while pg_isready runs after the start.
case_pitr_start_lock() {
  new_pitr_copy_case start-lock
  pitr_start_run --
  expect_eq "lock: pg_ctl start has no descriptor 9" "$(pitr_stub_value PGCTL_FD9)" absent
  expect_eq "lock: another process cannot take the lock while pg_ctl runs" "$(pitr_stub_value PGCTL_LOCK)" held
  expect_eq "lock: another process cannot take the lock after the start" "$(pitr_stub_value ISREADY_LOCK)" held
}

# A failed start or a lost postmaster: the tail of the server log, one ERROR line, a code other than 0.
case_pitr_start_fails() {
  local variant
  for variant in STUB_PGCTL_START_FAIL STUB_PGCTL_NO_PID; do
    new_pitr_copy_case "start-fails-$variant"
    pitr_start_run "$variant=1" --
    expect_no "$variant: exit code is not 0" test "$RC" -eq 0
    expect_eq "$variant: one ERROR line" "$(error_count "$CASE/out")" 1
    expect_ok "$variant: the ERROR line names the server" error_has "the temporary server did not finish the recovery"
    expect_ok "$variant: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
    expect_eq "$variant: the log tail has $START_LOG_TAIL lines" "$(grep -c '^server log line ' "$CASE/err" || true)" "$START_LOG_TAIL"
    expect_ok "$variant: the log tail ends with the last line" grep -qxF 'server log line 25' "$CASE/err"
    expect_no "$variant: the log tail leaves the older lines out" grep -qxF 'server log line 5' "$CASE/err"
    expect_eq "$variant: the work directory is gone" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  done
  expect_eq "lost postmaster: no connection check" "$(pitr_tool_calls pg_isready)" 0
}

# The cleanup stops a server that this run started and that is still up when a later check fails.
# A completed run stops the server in step 11 with --mode=fast, so the cleanup stops nothing more.
case_pitr_start_cleanup() {
  new_pitr_copy_case start-cleanup
  pitr_start_run STUB_PSQL_RECOVERED=f --
  expect_no "cleanup: exit code is not 0" test "$RC" -eq 0
  expect_eq "cleanup: one ERROR line, from the failed check" "$(error_count "$CASE/out")" 1
  expect_ok "cleanup: pg_ctl stops the server with --mode=immediate" \
    grep -qE '^ARGV pg_ctl --pgdata=\S+ --mode=immediate --wait --timeout=600 stop$' "$CASE/stub/log"
  expect_no "cleanup: no postmaster file stays" test -e "$CASE/target/postmaster.pid"
  reset_pitr_run
  pitr_start_run --
  expect_eq "cleanup: a completed run exit code" "$RC" 0
  expect_eq "cleanup: a completed run stops the server once" "$(grep -c '^PGCTL_ACTION stop$' "$CASE/stub/log" || true)" 1
  expect_eq "cleanup: a completed run has no immediate stop" "$(grep -c -- '--mode=immediate' "$CASE/stub/log" || true)" 0
  expect_ok "cleanup: the stop timeout names its row" \
    grep -qE '^STOP_TIMEOUT_SECONDS=600 # pitr_volume\.stop_timeout of reliability\.md' "$BIN/pitr-volume"
}

# The directory of the private socket is missing: one ERROR line and no server.
case_pitr_start_no_private_directory() {
  new_pitr_copy_case start-no-private-directory
  rmdir "$CASE/pgpass"
  pitr_start_run --
  expect_no "no private directory: exit code is not 0" test "$RC" -eq 0
  expect_eq "no private directory: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "no private directory: the ERROR line names it" error_has "the directory of the secret files is missing"
  expect_eq "no private directory: no start" "$(grep -c '^PGCTL_ACTION start$' "$CASE/stub/log" || true)" 0
  expect_no "no private directory: no recovery.signal" test -e "$CASE/target/recovery.signal"
}
