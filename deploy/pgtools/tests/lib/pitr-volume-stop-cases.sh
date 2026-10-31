# Sourced by pitr-volume-test. It holds the cases of steps 10 to 12 of ADR-0106 (f).
#   Step 10: one psql query reads each value of ADR-0106 (d). A text value passes the function clean_json_text.
#   Step 11: the fast stop within pitr_volume.stop_timeout, the state shut down in pg_controldata, with --drill
#            the empty file pitr-drill at the top of the target, then sync.
#   Step 12: the last line "pitr volume completed" with the values of (d) and the field drill.
# The cases run the script to the end. The stub of psql answers the value queries
# (see lib/pitr-volume-stubs.sh). The switches STUB_LAST_SEQ, STUB_LAST_AT, STUB_LAST_ACTION, STUB_HEARTBEAT, and
# STUB_VALUE_FAIL change the answers.
# It needs lib/helpers.sh, lib/pitr-volume-stubs.sh, lib/pitr-volume-copy-cases.sh (pitr_tool_calls),
# lib/pitr-volume-check-cases.sh (new_pitr_check_case, pitr_check_run), and lib/pitr-volume-start-cases.sh
# (START_TARGET_TIME).

STOP_DONE_MESSAGE="pitr volume completed"
# The patterns of read_pitr_line in deploy/ci/switch-steps.sh, lines 153 to 156 (PITR_MESSAGE_PATTERN,
# PITR_IDENTIFIER_PATTERN, PITR_TIMELINE_PATTERN, PITR_DRILL_PATTERN). The switch reads the last line with them.
STOP_MESSAGE_PATTERN='^\{.*"message":"pitr volume completed".*\}$'
STOP_IDENTIFIER_PATTERN='"system_identifier":"([0-9]{1,20})"'
STOP_TIMELINE_PATTERN='"timeline":"?([0-9]{1,10})"?[,}]'
STOP_DRILL_PATTERN='"drill":(true|false)[,}]'
# The default answers of the stub of psql.
STOP_BASE=base_20260101_000000
STOP_AT=2026-01-02T03:04:05.123456Z
STOP_HEARTBEAT=2026-01-02T03:04:00.000000Z
STOP_FAIL_STOP="the temporary server did not stop"
STOP_FAIL_STATE="the cluster did not shut down cleanly"
STOP_FAIL_CONTROL="pg_controldata failed"
STOP_FAIL_TIMELINE="the timeline of the stopped cluster differs from the recovered cluster"
STOP_FAIL_SYNC="sync failed"
# One JSON value of the strict check: a string without a quote, a backslash, or a control character, or a number, or a word.
STOP_JSON_VALUE='("[^"\\[:cntrl:]]*"|[0-9]+|true|false)'

# pitr_stop_run [VAR=value ...] -- [argument ...]: run the script to the last step.
pitr_stop_run() { pitr_check_run "$@"; }

# pitr_stop_last: the last line of the output.
pitr_stop_last() { tail -n 1 "$CASE/out"; }

# pitr_stop_body: the last line without its timestamp.
pitr_stop_body() { pitr_stop_last | sed 's/^{"timestamp":"[^"]*",//'; }

# pitr_stop_strict <file>: every line is strict JSON. A string holds no quote, no backslash, and no control
# character. With jq in the image, jq parses each line too.
pitr_stop_strict() {
  local line
  while IFS= read -r line; do
    printf '%s\n' "$line" | grep -qE "^\\{\"[a-z_]+\":$STOP_JSON_VALUE(,\"[a-z_]+\":$STOP_JSON_VALUE)*\\}\$" || return 1
    ! command -v jq >/dev/null 2>&1 || printf '%s\n' "$line" | jq -e . >/dev/null || return 1
  done <"$1"
}

# pitr_stop_expected <timeline> <chain_seq> <occurred_at> <action> <heartbeat> <drill>: the last line without its timestamp.
pitr_stop_expected() {
  printf '"level":"INFO","job":"pitr-volume","message":"%s","base_backup":"%s","system_identifier":"7300000000000000001","timeline":%s,"chain_seq":%s,"occurred_at":"%s","action":"%s","heartbeat_last_at":"%s","drill":%s}' \
    "$STOP_DONE_MESSAGE" "$STOP_BASE" "$@"
}

# pitr_stop_passes <label> [VAR=value ...] -- [argument ...]: the run ends with code 0, no ERROR line, and strict JSON.
pitr_stop_passes() {
  local label=$1
  shift
  pitr_stop_run "$@"
  expect_eq "$label: exit code" "$RC" 0
  expect_eq "$label: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_ok "$label: every line is strict JSON" pitr_stop_strict "$CASE/out"
}

# pitr_stop_fails <label> <ERROR text> [VAR=value ...] -- [argument ...]: one ERROR line, no last line, and no server.
pitr_stop_fails() {
  local label=$1 text=$2
  shift 2
  pitr_stop_run "$@"
  expect_no "$label: exit code is not 0" test "$RC" -eq 0
  expect_eq "$label: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$label: the ERROR line holds [$text]" error_has "$text"
  expect_eq "$label: no last line" "$(grep -c "\"message\":\"$STOP_DONE_MESSAGE\"" "$CASE/out" || true)" 0
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_no "$label: the cleanup stops the server" test -e "$CASE/target/postmaster.pid"
}

# A passed run: the fast stop, the state, sync, and the last line with every value.
case_pitr_stop_pass() {
  local kind
  new_pitr_check_case stop-pass
  pitr_stop_passes "pass" --
  expect_ok "pass: pg_ctl stops the server with --mode=fast within 600 seconds" \
    grep -qE '^ARGV pg_ctl --pgdata=\S+ --mode=fast --wait --timeout=600 stop$' "$CASE/stub/log"
  expect_eq "pass: one stop only" "$(grep -c '^PGCTL_ACTION stop$' "$CASE/stub/log" || true)" 1
  expect_eq "pass: no immediate stop" "$(grep -c -- '--mode=immediate' "$CASE/stub/log" || true)" 0
  expect_eq "pass: sync runs once" "$(pitr_tool_calls sync)" 1
  expect_ok "pass: the order is stop, pg_controldata, sync" bash -c '[[ $1 == *"EVENT pg_ctl,EVENT pg_controldata,EVENT sync," ]]' _ "$(event_sequence)"
  expect_ok "pass: sync runs after the stop" grep -qx 'SYNC_STOPPED yes' "$CASE/stub/log"
  expect_ok "pass: sync runs without the file pitr-drill" grep -qx 'SYNC_DRILL absent' "$CASE/stub/log"
  expect_no "pass: no postmaster file stays" test -e "$CASE/target/postmaster.pid"
  expect_no "pass: no file pitr-drill" test -e "$CASE/target/pitr-drill"
  for kind in last_seq last_at last_action heartbeat; do
    expect_eq "pass: one query $kind" "$(grep -c "^QUERY $kind\$" "$CASE/stub/log" || true)" 1
  done
  expect_eq "pass: the last line is the one line with the message" \
    "$(grep -c "\"message\":\"$STOP_DONE_MESSAGE\"" "$CASE/out" || true)" 1
  expect_eq "pass: the last line" "$(pitr_stop_body)" "$(pitr_stop_expected 2 57 "$STOP_AT" RecipeApproved "$STOP_HEARTBEAT" false)"
}

# The last line parses with the patterns of read_pitr_line in deploy/ci/switch-steps.sh.
case_pitr_stop_line_patterns() {
  local last
  new_pitr_check_case stop-patterns
  pitr_stop_passes "patterns" STUB_TIMELINE=0000000A --
  last=$(pitr_stop_last)
  expect_ok "patterns: message" bash -c '[[ $1 =~ $2 ]]' _ "$last" "$STOP_MESSAGE_PATTERN"
  expect_ok "patterns: identifier" bash -c '[[ $1 =~ $2 && ${BASH_REMATCH[1]} == 7300000000000000001 ]]' _ "$last" "$STOP_IDENTIFIER_PATTERN"
  expect_ok "patterns: the timeline 0000000A is the decimal 10" bash -c '[[ $1 =~ $2 && ${BASH_REMATCH[1]} == 10 ]]' _ "$last" "$STOP_TIMELINE_PATTERN"
  expect_ok "patterns: drill" bash -c '[[ $1 =~ $2 && ${BASH_REMATCH[1]} == false ]]' _ "$last" "$STOP_DRILL_PATTERN"
}

# With --drill: the empty file pitr-drill before sync, and "drill":true. Without it: no file and "drill":false.
case_pitr_stop_drill() {
  new_pitr_check_case stop-drill
  pitr_stop_passes "drill" -- --target-time "$START_TARGET_TIME" --drill
  expect_ok "drill: the file pitr-drill exists" test -f "$CASE/target/pitr-drill"
  expect_no "drill: the file pitr-drill is empty" test -s "$CASE/target/pitr-drill"
  expect_ok "drill: sync runs after the file" grep -qx 'SYNC_DRILL present' "$CASE/stub/log"
  expect_ok "drill: sync runs after the stop" grep -qx 'SYNC_STOPPED yes' "$CASE/stub/log"
  expect_eq "drill: the last line" "$(pitr_stop_body)" "$(pitr_stop_expected 2 57 "$STOP_AT" RecipeApproved "$STOP_HEARTBEAT" true)"
  new_pitr_check_case stop-no-drill
  pitr_stop_passes "no drill" -- --target-time "$START_TARGET_TIME"
  expect_no "no drill: no file pitr-drill" test -e "$CASE/target/pitr-drill"
  expect_eq "no drill: the last line" "$(pitr_stop_body)" "$(pitr_stop_expected 2 57 "$STOP_AT" RecipeApproved "$STOP_HEARTBEAT" false)"
}

# A text value with a quote, a backslash, or a control character is cleaned. The line stays valid JSON.
case_pitr_stop_clean() {
  local long
  new_pitr_check_case stop-clean
  pitr_stop_passes "clean" "STUB_LAST_ACTION=Bad\"Act\\ion"$'\t'"X"$'\001' "STUB_LAST_AT=2026-01-02T03:04:05\"Z"$'\n'"x" \
    "STUB_HEARTBEAT=beat\\"$'\r'"end" --
  expect_eq "clean: the cleaned values" "$(pitr_stop_body)" \
    "$(pitr_stop_expected 2 57 2026-01-02T03:04:05Zx BadActionX beatend false)"
  new_pitr_check_case stop-long
  long=$(printf 'a%.0s' $(seq 1 200))
  pitr_stop_passes "long value" "STUB_HEARTBEAT=$long" --
  expect_eq "long value: cut to 160 characters" "$(pitr_stop_last | sed -n 's/.*"heartbeat_last_at":"\([^"]*\)".*/\1/p' | tr -d '\n' | wc -c)" 160
}

# A failed stop, a state other than shut down, a failed pg_controldata, a timeline that differs, and a failed sync.
case_pitr_stop_fails() {
  new_pitr_check_case stop-fail-stop
  pitr_stop_fails "failed stop" "$STOP_FAIL_STOP" STUB_PGCTL_STOP_FAIL=1 -- --target-time "$START_TARGET_TIME" --drill
  expect_eq "failed stop: no sync" "$(pitr_tool_calls sync)" 0
  expect_no "failed stop: no file pitr-drill" test -e "$CASE/target/pitr-drill"
  expect_ok "failed stop: the cleanup stops the server with --mode=immediate" \
    grep -qE '^ARGV pg_ctl --pgdata=\S+ --mode=immediate --wait --timeout=600 stop$' "$CASE/stub/log"
  new_pitr_check_case stop-fail-state
  pitr_stop_fails "state in production" "$STOP_FAIL_STATE" "STUB_CONTROL_STATE=in production" -- --target-time "$START_TARGET_TIME" --drill
  expect_ok "state in production: the ERROR line names the state" error_has '"state":"in production"'
  expect_eq "state in production: no sync" "$(pitr_tool_calls sync)" 0
  expect_no "state in production: no file pitr-drill" test -e "$CASE/target/pitr-drill"
  new_pitr_check_case stop-fail-control
  pitr_stop_fails "pg_controldata fails" "$STOP_FAIL_CONTROL" STUB_CONTROL_FAIL_AFTER_STOP=1 --
  expect_eq "pg_controldata fails: no sync" "$(pitr_tool_calls sync)" 0
  new_pitr_check_case stop-fail-timeline
  pitr_stop_fails "timeline differs" "$STOP_FAIL_TIMELINE" STUB_CONTROL_TLI=3 --
  expect_eq "timeline differs: no sync" "$(pitr_tool_calls sync)" 0
  new_pitr_check_case stop-fail-sync
  pitr_stop_fails "sync fails" "$STOP_FAIL_SYNC" STUB_SYNC_FAIL=1 --
}

# A value query that fails, or that gives an empty value, gives one ERROR line. The server stops only in the cleanup.
case_pitr_stop_values() {
  local kind text
  for kind in seq at action heartbeat; do
    case $kind in
      seq) text="cannot read chain_seq of the newest audit entry" ;;
      at) text="cannot read occurred_at of the newest audit entry" ;;
      action) text="cannot read action of the newest audit entry" ;;
      heartbeat) text="cannot read heartbeat.last_at of the recovered cluster" ;;
    esac
    new_pitr_check_case "stop-value-$kind"
    pitr_stop_fails "query $kind fails" "$text" "STUB_VALUE_FAIL=$kind" --
    expect_eq "query $kind fails: no fast stop" "$(grep -c -- '--mode=fast' "$CASE/stub/log" || true)" 0
    expect_eq "query $kind fails: no sync" "$(pitr_tool_calls sync)" 0
  done
  new_pitr_check_case stop-value-empty-action
  pitr_stop_fails "empty action" "cannot read action of the newest audit entry" STUB_LAST_ACTION= --
  new_pitr_check_case stop-value-quotes-only
  pitr_stop_fails "heartbeat of quotes only" "cannot read heartbeat.last_at of the recovered cluster" 'STUB_HEARTBEAT=""' --
  new_pitr_check_case stop-value-seq-grammar
  pitr_stop_fails "chain_seq outside its grammar" "chain_seq of the newest audit entry is outside its grammar" STUB_LAST_SEQ=0 --
  expect_eq "chain_seq outside its grammar: no fast stop" "$(grep -c -- '--mode=fast' "$CASE/stub/log" || true)" 0
}

# The script has no test hook. ADR-0106 (f) says a run completes all 12 steps or writes one ERROR line and
# ends with a code other than 0. A variable PITR_STOP_AFTER_STEP in the environment changes nothing.
case_pitr_no_stop_hook() {
  local value
  expect_no "no hook: the script text holds PITR_STOP_AFTER_STEP" grep -q 'PITR_STOP_AFTER_STEP' "$BIN/pitr-volume"
  expect_no "no hook: the script text holds stop_after" grep -q 'stop_after' "$BIN/pitr-volume"
  for value in 4 9 bogus; do
    new_pitr_check_case "stop-hook-$value"
    pitr_stop_passes "hook variable [$value] is ignored" "PITR_STOP_AFTER_STEP=$value" --
    expect_eq "hook variable [$value]: the last line is written once" \
      "$(grep -c "\"message\":\"$STOP_DONE_MESSAGE\"" "$CASE/out" || true)" 1
    expect_eq "hook variable [$value]: the fast stop ran once" "$(grep -c -- '--mode=fast' "$CASE/stub/log" || true)" 1
    expect_eq "hook variable [$value]: sync ran once" "$(pitr_tool_calls sync)" 1
  done
}
