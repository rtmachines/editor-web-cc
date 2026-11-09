# Sourced by restore-test-test. It holds the result file cases of ADR-0151 (a) to (g).
# This file defines functions only, like the other case files.

# restore_result_checks <label> <result>: check the file separately from the set listing.
restore_result_checks() {
  local label=$1 result=$2 file=$B/restore-test.result
  expect_ok "$label: result file exists" test -f "$file"
  [ -f "$file" ] || return 0
  expect_eq "$label: mode is 644" "$(stat -c %a -- "$file")" 644
  expect_eq "$label: one line" "$(wc -l <"$file")" 1
  expect_eq "$label: no second line without a newline" "$(grep -c '^' "$file")" 1
  expect_ok "$label: result and UTC time match the format" \
    grep -qxE "\{\"result\":\"$result\",\"finished_at\":\"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z\"\}" "$file"
}

case_restore_result_passed() {
  new_restore_case result-passed
  restore_run
  expect_eq "result passed: exit code" "$RC" 0
  restore_result_checks "result passed" passed
  expect_no "result passed: no temp result stays" test -e "$B/.tmp-restore-test.result"
}

case_restore_result_counts_failed() {
  new_restore_case result-counts-failed
  printf 'public.machine|4\npublic.recipe|5\n' >"$CASE/stub/restored.counts"
  restore_run
  expect_eq "result counts failed: exit code" "$RC" 1
  expect_eq "result counts failed: count step ran" "$(event_count counts)" 1
  expect_eq "result counts failed: chain step did not run" "$(event_count chain)" 0
  restore_result_checks "result counts failed" failed
  expect_no "result counts failed: no temp result stays" test -e "$B/.tmp-restore-test.result"
}

case_restore_result_term() {
  new_restore_case result-term
  env PATH="$ROOT/stubs:$PATH" BACKUP_DIR="$B" BACKUP_PGPASS_DIR="$CASE/pgpass" \
    RESTORE_CODE_DIR="$CASE/code" RESTORE_GLOBALS_DIR="$HERE/../lib" PGDATABASE=recipe \
    STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" STUB_RESTORE_SLEEP=3 \
    bash "$BIN/restore-test" >"$CASE/out" 2>"$CASE/err" &
  local pid=$! waited=0
  while ! grep -q '^EVENT pg_restore data' "$CASE/stub/log" && [ "$waited" -lt 50 ]; do
    sleep 0.2
    waited=$((waited + 1))
  done
  expect_ok "result TERM: data step started" grep -q '^EVENT pg_restore data' "$CASE/stub/log"
  kill -TERM "$pid"
  wait "$pid"
  RC=$?
  expect_eq "result TERM: exit code" "$RC" 143
  expect_eq "result TERM: the server is stopped" "$(event_count 'pg_ctl stop')" 1
  expect_eq "result TERM: no scratch run directory stays" "$(scratch_runs)" 0
  restore_result_checks "result TERM" failed
  expect_no "result TERM: no temp result stays" test -e "$B/.tmp-restore-test.result"
  case_restore_result_cleanup_term
  case_restore_result_write_term
}

# Each signal during the result write must preserve the passed result and exit code 0.
case_restore_result_write_term() {
  restore_result_write_signal_case TERM
  restore_result_write_signal_case INT
  restore_result_write_signal_case HUP
}

restore_result_write_signal_case() {
  local signal=$1 label="result write $1"
  new_restore_case "result-write-${signal,,}"
  mkdir "$CASE/mvstub"
  cat >"$CASE/mvstub/mv" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "${*: -1}" = "$BACKUP_DIR/restore-test.result" ]; then
  kill -"$STUB_SIGNAL" "$PPID"
  echo "EVENT result write $STUB_SIGNAL" >>"$STUB_LOG"
fi
exec "$REAL_MV" "$@"
EOF
  chmod 0755 "$CASE/mvstub/mv"
  restore_run "STUB_SIGNAL=$signal" "REAL_MV=$(command -v mv)" "PATH=$CASE/mvstub:$ROOT/stubs:$PATH"
  expect_eq "$label: signal sent" "$(event_count "result write $signal")" 1
  expect_eq "$label: exit code" "$RC" 0
  restore_result_checks "$label" passed
  expect_no "$label: no temp result stays" test -e "$B/.tmp-restore-test.result"
}

# TERM during cleanup must replace an old passed result after the server stops.
case_restore_result_cleanup_term() {
  new_restore_case result-cleanup-term
  local old='{"result":"passed","finished_at":"2026-01-01T00:00:00Z"}'
  printf '%s\n' "$old" >"$B/restore-test.result"
  mkdir "$CASE/termstub"
  cat >"$CASE/termstub/pg_ctl" <<'EOF'
#!/usr/bin/env bash
if [ "${*: -1}" = stop ]; then
  : >"$STUB_DIR/cleanup.ready"
  n=100
  while [ ! -e "$STUB_DIR/cleanup.release" ] && [ "$n" -gt 0 ]; do
    sleep 0.1
    n=$((n - 1))
  done
fi
exec "$REAL_PG_CTL" "$@"
EOF
  chmod 0755 "$CASE/termstub/pg_ctl"
  env PATH="$CASE/termstub:$ROOT/stubs:$PATH" BACKUP_DIR="$B" BACKUP_PGPASS_DIR="$CASE/pgpass" \
    RESTORE_CODE_DIR="$CASE/code" RESTORE_GLOBALS_DIR="$HERE/../lib" PGDATABASE=recipe \
    STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" STUB_RESTORE_FAIL=data \
    REAL_PG_CTL="$ROOT/stubs/pg_ctl" bash "$BIN/restore-test" >"$CASE/out" 2>"$CASE/err" &
  local pid=$! waited=0
  while [ ! -e "$CASE/stub/cleanup.ready" ] && [ "$waited" -lt 50 ]; do
    sleep 0.2
    waited=$((waited + 1))
  done
  expect_ok "result cleanup TERM: server stop started" test -e "$CASE/stub/cleanup.ready"
  kill -TERM "$pid"
  : >"$CASE/stub/cleanup.release"
  wait "$pid"
  RC=$?
  expect_eq "result cleanup TERM: exit code" "$RC" 143
  expect_eq "result cleanup TERM: the server is stopped" "$(event_count 'pg_ctl stop')" 1
  expect_eq "result cleanup TERM: no scratch run directory stays" "$(scratch_runs)" 0
  restore_result_checks "result cleanup TERM" failed
  expect_no "result cleanup TERM: old passed result is replaced" test "$(cat "$B/restore-test.result")" = "$old"
  expect_no "result cleanup TERM: no temp result stays" test -e "$B/.tmp-restore-test.result"
}

case_restore_result_replaces_failed() {
  new_restore_case result-replaces-failed
  printf '{"result":"failed","finished_at":"2026-01-01T00:00:00Z"}\n' >"$B/restore-test.result"
  chmod 0600 "$B/restore-test.result"
  restore_run
  expect_eq "result replacement: exit code" "$RC" 0
  restore_result_checks "result replacement" passed
  expect_no "result replacement: no temp result stays" test -e "$B/.tmp-restore-test.result"
}

# Neither rejected run may create a file or replace an old result (ADR-0151 (e)).
case_restore_result_missing_marker() {
  local state before
  for state in missing old; do
    new_restore_case "result-missing-marker-$state"
    rm -- "$B/.backup-marker"
    if [ "$state" = old ]; then
      printf '{"result":"passed","finished_at":"2026-01-01T00:00:00Z"}\n' >"$B/restore-test.result"
    fi
    before=$(listing)
    restore_run
    expect_eq "result missing marker ($state): exit code" "$RC" 1
    expect_eq "result missing marker ($state): no file changes" "$(listing)" "$before"
    expect_eq "result missing marker ($state): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

case_restore_result_lock_held() {
  local state before
  for state in missing old; do
    new_restore_case "result-lock-held-$state"
    if [ "$state" = old ]; then
      printf '{"result":"passed","finished_at":"2026-01-01T00:00:00Z"}\n' >"$B/restore-test.result"
    fi
    mkdir -p -- "$B/.scratch"
    exec 8>"$B/.scratch/.lock"
    flock -n 8
    before=$(listing)
    restore_run
    flock -u 8
    exec 8>&-
    expect_eq "result lock held ($state): exit code" "$RC" 1
    expect_eq "result lock held ($state): no file changes" "$(listing)" "$before"
    expect_eq "result lock held ($state): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

case_restore_result_write_fails() {
  new_restore_case result-write-fails
  local old='{"result":"failed","finished_at":"2026-01-01T00:00:00Z"}'
  printf '%s\n' "$old" >"$B/restore-test.result"
  mkdir -p "$CASE/mvstub"
  cat >"$CASE/mvstub/mv" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
umask 077
if [ "${*: -1}" = "$BACKUP_DIR/restore-test.result" ]; then
  echo 'EVENT result mv refused' >>"$STUB_LOG"
  exit 1
fi
exec "$REAL_MV" "$@"
EOF
  chmod 0755 "$CASE/mvstub/mv"
  restore_run "REAL_MV=$(command -v mv)" "PATH=$CASE/mvstub:$ROOT/stubs:$PATH"
  expect_eq "result write fails: rename was attempted" "$(event_count 'result mv refused')" 1
  expect_eq "result write fails: exit code" "$RC" 1
  expect_eq "result write fails: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "result write fails: old result stays" "$(cat "$B/restore-test.result")" "$old"
  expect_no "result write fails: no temp result stays" test -e "$B/.tmp-restore-test.result"
  expect_eq "result write fails: the server is stopped" "$(event_count 'pg_ctl stop')" 1
  expect_eq "result write fails: no scratch run directory stays" "$(scratch_runs)" 0
}

# This run uses the installed library directory, without RESTORE_GLOBALS_DIR (design D3).
case_restore_result_image_library() {
  new_restore_case result-image-library
  expect_ok "result image library: check-result.sh is installed" test -f /etc/pgtools/check-result.sh
  env -u RESTORE_GLOBALS_DIR PATH="$ROOT/stubs:$PATH" BACKUP_DIR="$B" \
    BACKUP_PGPASS_DIR="$CASE/pgpass" RESTORE_CODE_DIR="$CASE/code" PGDATABASE=recipe \
    STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" \
    bash "$BIN/restore-test" >"$CASE/out" 2>"$CASE/err"
  RC=$?
  expect_eq "result image library: exit code" "$RC" 0
  expect_eq "result image library: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "result image library: no output on stderr" "$(wc -c <"$CASE/err")" 0
  restore_result_checks "result image library" passed
  expect_no "result image library: no temp result stays" test -e "$B/.tmp-restore-test.result"
}
