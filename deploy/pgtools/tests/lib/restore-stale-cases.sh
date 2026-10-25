# Sourced by restore-test-test. It holds the cases of the stale scratch directories (ADR-0067 (g)).
#
# A killed run leaves a directory run.* below .scratch. The next run deletes it. It stops the server
# of that directory only when the PID of postmaster.pid is a postgres process of that directory.
# Another process never gets a signal. It reads /proc/<pid>/cmdline, so these cases need Linux
# procfs, as the image has.

STALE_FAKE_PID=""

# start_fake_process <argv0> <data directory>: a process that sleeps, with the argument list of a
# postmaster: <argv0> -c <script> postgres <data directory>. It sets STALE_FAKE_PID.
start_fake_process() {
  (exec -a "$1" bash -c 'sleep 30; :' postgres "$2") &
  STALE_FAKE_PID=$!
  sleep 0.3
}

stop_fake_process() {
  kill "$STALE_FAKE_PID" 2>/dev/null || true
  wait "$STALE_FAKE_PID" 2>/dev/null || true
}

# stale_stop_count: the calls of pg_ctl stop for the stale directory.
stale_stop_count() {
  grep -c '^ARGV pg_ctl .*run\.OLD123/data.* stop$' "$CASE/stub/log" || true
}

# new_stale_case <name>: a restore case with the stale directory run.OLD123 and without a PID file.
# The link run.LINK123 points to a directory that must stay.
new_stale_case() {
  new_restore_case "$1"
  mkdir -p "$B/.scratch/run.OLD123/data" "$CASE/outside"
  printf '16\n' >"$B/.scratch/run.OLD123/data/PG_VERSION"
  printf 'keep' >"$B/.scratch/work"
  printf 'keep' >"$CASE/outside/file"
  ln -s "$CASE/outside" "$B/.scratch/run.LINK123"
}

case_restore_stale_scratch() {
  new_stale_case stale
  start_fake_process postgres "$B/.scratch/run.OLD123/data"
  printf '%s\n' "$STALE_FAKE_PID" >"$B/.scratch/run.OLD123/data/postmaster.pid"
  restore_run
  stop_fake_process
  expect_eq "stale: exit code" "$RC" 0
  expect_no "stale: the old run directory is gone" test -e "$B/.scratch/run.OLD123"
  expect_eq "stale: the server of the old run is stopped" "$(stale_stop_count)" 1
  expect_eq "stale: the server of this run is stopped too" "$(event_count 'pg_ctl stop')" 2
  expect_ok "stale: the other entry of .scratch stays" test -f "$B/.scratch/work"
  expect_ok "stale: the target of a link stays" test -f "$CASE/outside/file"
  expect_ok "stale: the log says so" grep -q '"message":"stale scratch directory deleted"' "$CASE/out"
}

# Another process can have the PID of a stale postmaster.pid after a restart of the container.
# It gets no signal: the script does not call pg_ctl stop, and it still deletes the directory.
case_restore_stale_pid_of_another_process() {
  new_stale_case stale-other-process
  start_fake_process supercronic "$B/.scratch/run.OLD123/data"
  printf '%s\n' "$STALE_FAKE_PID" >"$B/.scratch/run.OLD123/data/postmaster.pid"
  restore_run
  expect_ok "stale other process: the process lives" kill -0 "$STALE_FAKE_PID"
  stop_fake_process
  expect_eq "stale other process: exit code" "$RC" 0
  expect_eq "stale other process: no stop for the stale directory" "$(stale_stop_count)" 0
  expect_no "stale other process: the old run directory is gone" test -e "$B/.scratch/run.OLD123"
  expect_ok "stale other process: the target of a link stays" test -f "$CASE/outside/file"
  new_stale_case stale-postgres-of-another-directory
  start_fake_process postgres "$B/.scratch/run.OTHER1/data"
  printf '%s\n' "$STALE_FAKE_PID" >"$B/.scratch/run.OLD123/data/postmaster.pid"
  restore_run
  expect_ok "stale other directory: the process lives" kill -0 "$STALE_FAKE_PID"
  stop_fake_process
  expect_eq "stale other directory: exit code" "$RC" 0
  expect_eq "stale other directory: no stop for the stale directory" "$(stale_stop_count)" 0
  expect_no "stale other directory: the old run directory is gone" test -e "$B/.scratch/run.OLD123"
}

# A PID that is no number, or no process, gets no call either.
case_restore_stale_pid_not_a_process() {
  local text variant=0
  for text in 'abc' '0' '-1' '99999999' '1 2' ''; do
    variant=$((variant + 1))
    new_stale_case "stale-bad-pid-$variant"
    printf '%s\n' "$text" >"$B/.scratch/run.OLD123/data/postmaster.pid"
    restore_run
    expect_eq "stale bad pid [$text]: exit code" "$RC" 0
    expect_eq "stale bad pid [$text]: no stop for the stale directory" "$(stale_stop_count)" 0
    expect_no "stale bad pid [$text]: the old run directory is gone" test -e "$B/.scratch/run.OLD123"
  done
}
