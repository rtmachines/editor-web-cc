# Sourced by walarchive-test. Static rules of the scripts and the cases of walarchive-entry
# and walarchive-health (ADR-0068 (a) to (d), design D10).

WAL_SCRIPTS=(walarchive-entry walarchive-health basebackup-once pitr-check)
EXPECTED_PGPASS='db:5432:replication:recipe_replicator:ab\:c\\d9'

# The rules of every script of the image (ADR-0066 (m) to (o)).
case_wal_static_rules() {
  local s f
  for s in "${WAL_SCRIPTS[@]}"; do
    f=$BIN/$s
    expect_ok "static $s: file exists" test -f "$f"
    expect_ok "static $s: bash -n" bash -n "$f"
    expect_eq "static $s: shebang" "$(head -n 1 "$f" 2>/dev/null)" "#!/usr/bin/env bash"
    expect_eq "static $s: set, then umask" \
      "$(grep -vE '^(#|$)' "$f" 2>/dev/null | head -n 2 | tr '\n' ',')" "set -euo pipefail,umask 077,"
    expect_no "static $s: no trace option" grep -qE 'set +-[a-z]*x|[x]trace|bash +-x' "$f"
    expect_no "static $s: no carriage return" grep -q $'\r' "$f"
    expect_no "static $s: no PGPASSWORD" grep -qE 'PGPASSWORD=|export PGPASSWORD' "$f"
    expect_no "static $s: the secret is never read into a variable" \
      grep -qE '\$\(cat[^)]*(SECRET|secret)|read .*(SECRET|secret)' "$f"
    expect_eq "static $s: every rm has the guarded form" \
      "$(grep -E '(^|[^a-z_-])rm ' "$f" 2>/dev/null | grep -vE '^[[:space:]]*#' \
        | grep -vcE 'rm( -r| -f| -rf)? -- "\$\{[a-z_]+:\?\}' || true)" "0"
  done
  expect_ok "static crontab: exists" test -f "$BIN/../crontab.walarchive"
  expect_no "static crontab: no carriage return" grep -q $'\r' "$BIN/../crontab.walarchive"
  expect_eq "static crontab: lines" \
    "$(grep -vE '^(#|$)' "$BIN/../crontab.walarchive" 2>/dev/null | tr '\n' ',')" \
    "CRON_TZ=UTC,30 3 * * 0 basebackup-once,"
}

case_wal_static_entry() {
  local f=$BIN/walarchive-entry
  expect_ok "static entry: --synchronous" grep -qF -- '--synchronous' "$f"
  expect_ok "static entry: application_name" grep -qF -- 'application_name=walarchive' "$f"
  expect_no "static entry: no slot" grep -qE -- '--slot|[[:space:]]-S[[:space:]]|create-slot' "$f"
  expect_ok "static entry: supercronic flags" grep -qF -- 'supercronic -json -passthrough-logs' "$f"
  expect_ok "static entry: wait -n" grep -qE 'wait -n' "$f"
}

# The ADR-0031 rule: the secret file holds one line. A bad file starts no child.
wal_secret_case() {
  local label=$1 before
  wal_new_case "entry-secret-$label"
  case $label in
    empty) : >"$CASE/secret" ;;
    cr) printf 'abc\r\n' >"$CASE/secret" ;;
    two-lines) printf 'abc\ndef\n' >"$CASE/secret" ;;
    nul) printf 'ab\000c\n' >"$CASE/secret" ;;
    missing) rm -f -- "$CASE/secret" ;;
  esac
  before=$(wal_snapshot)
  wal_run -- walarchive-entry
  wal_expect_refused "entry secret $label" "$before"
}

case_wal_missing_marker() {
  local before s
  for s in walarchive-entry basebackup-once; do
    wal_new_case "marker-$s"
    rm -f -- "$W/.walarchive-marker"
    before=$(wal_snapshot)
    wal_run -- "$s"
    wal_expect_refused "$s without marker" "$before"
    expect_eq "$s without marker: message" "$(grep -c 'marker file' "$CASE/out" || true)" "1"
  done
  wal_new_case marker-dir
  rm -f -- "$W/.walarchive-marker"
  mkdir "$W/.walarchive-marker"
  before=$(wal_snapshot)
  wal_run -- walarchive-entry
  wal_expect_refused "entry with a directory as marker" "$before"
  wal_new_case marker-link
  rm -f -- "$W/.walarchive-marker"
  ln -s "$CASE/secret" "$W/.walarchive-marker"
  if [ -L "$W/.walarchive-marker" ]; then
    before=$(wal_snapshot)
    wal_run -- walarchive-entry
    wal_expect_refused "entry with a link as marker" "$before"
  else
    skip "this host does not create a symbolic link"
  fi
}

# A child that exits ends the container. The other child stops. A clean exit is also a fault.
case_wal_entry_args() {
  wal_new_case entry-args
  wal_run STUB_SUPERCRONIC_SLEEP=2 -- walarchive-entry
  local log=$CASE/stub/log
  expect_eq "entry: exit code when supercronic ends" "$RC" "1"
  expect_eq "entry: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_ok "entry: JSON lines" json_lines_ok "$CASE/out"
  expect_eq "entry: pg_receivewal started once" "$(count_events 'pg_receivewal start')" "1"
  expect_eq "entry: supercronic started once" "$(count_events 'supercronic')" "1"
  expect_ok "entry: --synchronous" grep -qF -- ' --synchronous' "$log"
  expect_ok "entry: application_name" grep -qF -- 'application_name=walarchive' "$log"
  expect_ok "entry: directory" grep -qF -- "--directory=$W" "$log"
  expect_no "entry: no slot" grep -qE -- '--slot|[[:space:]]-S[[:space:]]|create-slot' "$log"
  expect_ok "entry: supercronic flags and crontab" \
    grep -qF -- "ARGV supercronic -json -passthrough-logs $CASE/crontab" "$log"
  expect_ok "entry: supercronic has no pgpass file" grep -qF -- 'ENV PGPASSFILE=unset' "$log"
  expect_ok "entry: host and user" grep -qF -- 'ENV PGHOST=db PGPORT=5432 PGUSER=recipe_replicator' "$log"
  expect_eq "entry: pgpass line" "$(cat "$CASE/stub/pgpass.seen" 2>/dev/null)" "$EXPECTED_PGPASS"
  expect_eq "entry: pgpass mode" "$(cat "$CASE/stub/pgpass.mode" 2>/dev/null)" "600"
  expect_eq "entry: pgpass directory mode" "$(cat "$CASE/stub/pgpass.dirmode" 2>/dev/null)" "700"
  expect_eq "entry: pgpass directory removed" "$(find "$CASE/pgpass" -mindepth 1 | wc -l | tr -d ' ')" "0"
  expect_no "entry: pg_receivewal stopped" pid_alive "$CASE/stub/receivewal.pid"
  expect_ok "entry: no password in output" no_secret
  expect_eq "entry: the archive stays" "$(ls -A "$W" | LC_ALL=C sort | tr '\n' ' ')" ".lock .walarchive-marker base "
}

case_wal_entry_receiver_exit() {
  wal_new_case entry-receiver-exit
  wal_run STUB_RECEIVEWAL_EXIT=3 STUB_SUPERCRONIC_SLEEP=30 -- walarchive-entry
  expect_eq "entry, receiver exits: exit code" "$RC" "3"
  expect_eq "entry, receiver exits: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_ok "entry, receiver exits: names the child" grep -q 'pg_receivewal' "$CASE/out"
  expect_no "entry, receiver exits: supercronic stopped" pid_alive "$CASE/stub/supercronic.pid"
  expect_eq "entry, receiver exits: pgpass directory removed" \
    "$(find "$CASE/pgpass" -mindepth 1 | wc -l | tr -d ' ')" "0"
}

case_wal_entry_sigterm() {
  wal_new_case entry-term
  WAL_BG=1 wal_run STUB_SUPERCRONIC_SLEEP=30 -- walarchive-entry
  if wait_for_file "$CASE/stub/supercronic.pid" && wait_for_file "$CASE/stub/receivewal.pid"; then
    sleep 0.3
    kill -TERM "$BG_PID" 2>/dev/null
    wait_bg
    expect_eq "entry, SIGTERM: exit code" "$RC" "0"
    expect_no "entry, SIGTERM: pg_receivewal stopped" pid_alive "$CASE/stub/receivewal.pid"
    expect_no "entry, SIGTERM: supercronic stopped" pid_alive "$CASE/stub/supercronic.pid"
    expect_eq "entry, SIGTERM: no ERROR line" "$(error_count "$CASE/out")" "0"
  else
    kill "$BG_PID" 2>/dev/null
    wait_bg
    bad "entry, SIGTERM: the children did not start"
  fi
}

case_wal_entry_crontab_missing() {
  local before
  wal_new_case entry-no-crontab
  rm -f -- "$CASE/crontab"
  before=$(wal_snapshot)
  wal_run -- walarchive-entry
  wal_expect_refused "entry without crontab" "$before"
}

case_wal_entry_secret_escape() {
  wal_new_case entry-escape
  printf '%s' "$SECRET" >"$CASE/secret"
  wal_run STUB_SUPERCRONIC_SLEEP=2 -- walarchive-entry
  expect_eq "entry: pgpass line of a secret without a newline" \
    "$(cat "$CASE/stub/pgpass.seen" 2>/dev/null)" "$EXPECTED_PGPASS"
  expect_eq "entry: the pgpass line ends with a newline" "$(cat "$CASE/stub/pgpass.last" 2>/dev/null)" "0a"
  wal_new_case entry-escape-newline
  wal_run STUB_SUPERCRONIC_SLEEP=2 -- walarchive-entry
  expect_eq "entry: a secret with a newline gives one line end" "$(cat "$CASE/stub/pgpass.last" 2>/dev/null)" "0a"
  expect_eq "entry: a secret with a newline gives one line" \
    "$(wc -l <"$CASE/stub/pgpass.seen" 2>/dev/null | tr -d ' ')" "1"
}

# --- walarchive-health ------------------------------------------------------------------------

# make_proc <pid> <comm> [<state>]: a fake process entry below the proc directory of the case.
make_proc() {
  mkdir -p "$CASE/proc/$1"
  printf '%s\n' "$2" >"$CASE/proc/$1/comm"
  printf '%s (%s) %s 1 1 1 0\n' "$1" "$2" "${3:-S}" >"$CASE/proc/$1/stat"
}

# health_case <name> <expected rc> -- the run happens before the call.
health_expect() {
  expect_eq "health $1: exit code" "$RC" "$2"
  expect_eq "health $1: ERROR lines" "$(error_count "$CASE/out")" "$3"
  expect_ok "health $1: JSON lines" json_lines_ok "$CASE/out"
}

case_wal_health() {
  local before
  wal_new_case health-running
  make_proc 7 sleep
  make_proc 42 pg_receivewal
  make_proc 90 bash
  before=$(wal_snapshot)
  wal_run -- walarchive-health
  health_expect running 0 0
  expect_eq "health running: nothing changed" "$(wal_snapshot)" "$before"
  wal_new_case health-absent
  make_proc 7 sleep
  make_proc 8 supercronic
  wal_run -- walarchive-health
  health_expect absent 1 1
  wal_new_case health-zombie
  make_proc 42 pg_receivewal Z
  wal_run -- walarchive-health
  health_expect zombie 1 1
  wal_new_case health-zombie-and-live
  make_proc 42 pg_receivewal Z
  make_proc 43 pg_receivewal S
  wal_run -- walarchive-health
  health_expect "zombie and live" 0 0
  wal_new_case health-other-names
  make_proc 42 pg_receivewal2
  make_proc 43 xpg_receivewal
  make_proc 44 pg_receivewa
  wal_run -- walarchive-health
  health_expect "other names" 1 1
  wal_new_case health-odd-entries
  mkdir -p "$CASE/proc/self" "$CASE/proc/1a"
  printf 'pg_receivewal\n' >"$CASE/proc/self/comm"
  printf 'pg_receivewal\n' >"$CASE/proc/1a/comm"
  mkdir -p "$CASE/proc/55"
  wal_run -- walarchive-health
  health_expect "odd entries" 1 1
  wal_new_case health-no-proc
  rm -rf -- "${CASE:?}/proc"
  wal_run -- walarchive-health
  health_expect "no proc directory" 1 1
}
