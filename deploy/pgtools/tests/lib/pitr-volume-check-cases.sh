# Sourced by pitr-volume-test. It holds the cases of step 9 of ADR-0106 (f): the checks 1 to 6 of the gate of
# ADR-0106 (g) after the promotion.
#   Check 1: the cluster is not in recovery, and its timeline is above the timeline of the base backup.
#   Check 2: without a target time, the switch LSN of the history file lies in or after the newest segment of
#            /wal on the base timeline. A .partial file counts as its segment. PITR_EXPECTED_LSN also bounds it.
#   Check 3: the audit chain check of data-model.md 4.9 gives no break, and the head matches the last row.
#   Check 4: with PITR_MARKER_SEQ and PITR_MARKER_HASH, exactly one audit_log row has these values.
#            Step 1 checks the two grammars of ADR-0077 (b) and refuses one value without the other.
#   Check 5: the four configuration files keep their SHA-256 values of step 7, and no signal file stays.
#   Check 6: the role values equal restore-roles.expected with COMPOSE_SUBNET in the placeholder.
# A case that passes the checks runs on through steps 10 to 12 and ends with the line "pitr volume completed".
# A case that fails a check ends at step 9 with one ERROR line and no last line.
# The stub of pg_ctl writes the history file of the new timeline into the target at the start. The stub of
# psql answers the queries (see lib/pitr-volume-stubs.sh). The base backup of the cases starts on timeline 1,
# and the promoted server runs on timeline 2.
# It needs lib/helpers.sh, lib/pitr-volume-stubs.sh, lib/pitr-volume-copy-cases.sh (new_pitr_copy_case,
# pitr_tool_calls), and lib/pitr-volume-start-cases.sh (START_TARGET_TIME).

CHECK_END_ERROR="the recovery ended before the end of the WAL archive"
CHECK_RECOVERY_ERROR="the recovered cluster is still in recovery"
CHECK_TIMELINE_ERROR="the timeline of the recovered cluster is not above the timeline of the base backup"
CHECK_HISTORY_ERROR="the history file of the new timeline is missing or malformed"
CHECK_CHAIN_ERROR="the audit chain check failed on the recovered cluster"
CHECK_MARKER_ERROR="the marker entry is missing in the recovered cluster"
CHECK_CONFIG_ERROR="a configuration file of the recovered cluster changed"
CHECK_SIGNAL_ERROR="a signal file stays in the recovered cluster"
CHECK_ROLES_ERROR="the roles and the access of the recovered cluster differ from restore-roles.expected"
CHECK_MARKER_SEQ=42
CHECK_MARKER_HASH=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
# The newest segment of /wal on timeline 1 is number 4 (new_pitr_case). The default switch LSN
# PITR_SWITCH_LSN of the stubs lies in it.

# new_pitr_check_case <name>: a copy case. new_pitr_case puts the segments 2 to 4 of timeline 1 in /wal.
new_pitr_check_case() { new_pitr_copy_case "$1"; }

# pitr_check_run [VAR=value ...] -- [argument ...]: run the script. The defaults of pitr_run are a history
# file with the switch LSN PITR_SWITCH_LSN and a promoted server on timeline 2. Later settings win.
pitr_check_run() { pitr_run "$@"; }

# pitr_check_passes <label> [VAR=value ...] -- [argument ...]: the run ends with code 0, no ERROR line, and the last line.
pitr_check_passes() {
  local label=$1
  shift
  pitr_check_run "$@"
  expect_eq "$label: exit code" "$RC" 0
  expect_eq "$label: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_ok "$label: the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
}

# pitr_check_fails <label> <ERROR text> [VAR=value ...] -- [argument ...]: the run ends with one ERROR line
# that holds the text and no last line. The cleanup still stops the server and deletes the work directory.
pitr_check_fails() {
  local label=$1 text=$2
  shift 2
  pitr_check_run "$@"
  expect_no "$label: exit code is not 0" test "$RC" -eq 0
  expect_eq "$label: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$label: the ERROR line holds [$text]" error_has "$text"
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_no "$label: no last line" grep -q 'pitr volume completed' "$CASE/out"
  expect_no "$label: the cleanup stops the server" test -e "$CASE/target/postmaster.pid"
  expect_eq "$label: the work directory is gone" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# pitr_psql_calls: the number of calls of psql that hold the text.
pitr_psql_calls() { grep -c "^ARGV psql .*$1" "$CASE/stub/log" || true; }

# A recovered cluster on a new timeline with a switch LSN in the newest segment passes both checks.
case_pitr_check_pass() {
  new_pitr_check_case check-pass
  pitr_check_passes "pass" --
  expect_eq "pass: the recovery state is read once" "$(pitr_psql_calls pg_is_in_recovery)" 1
  expect_ok "pass: the segment of the switch LSN comes from pg_walfile_name in SQL" \
    grep -qE "^ARGV psql .*pg_walfile_name\('0/4000100'" "$CASE/stub/log"
  expect_eq "pass: no expected position, so no comparison in SQL" "$(pitr_psql_calls '>=')" 0
}

# Check 1: a cluster that is still in recovery.
case_pitr_check_in_recovery() {
  new_pitr_check_case check-in-recovery
  pitr_check_fails "in recovery" "$CHECK_RECOVERY_ERROR" STUB_PSQL_RECOVERED=f --
  expect_eq "in recovery: no more query after the first" "$(pitr_tool_calls psql)" 1
}

# Check 1: the timeline is not above the timeline of the base backup. Equal and lower both fail.
case_pitr_check_timeline() {
  new_pitr_check_case check-timeline-equal
  pitr_check_fails "equal timeline" "$CHECK_TIMELINE_ERROR" STUB_TIMELINE=00000001 --
  new_pitr_check_case check-timeline-lower
  sed -i 's/(file 00000001/(file 00000003/' "$CASE/wal/base/$BASE_A/backup_label"
  pitr_check_fails "lower timeline" "$CHECK_TIMELINE_ERROR" --
  new_pitr_check_case check-timeline-label
  sed -i '/^START WAL LOCATION/d' "$CASE/wal/base/$BASE_A/backup_label"
  pitr_check_fails "no timeline in the label" "cannot read the timeline of the base backup" --
  new_pitr_check_case check-timeline-garbled
  pitr_check_fails "garbled timeline" "cannot read the timeline of the recovered cluster" STUB_TIMELINE=xyz --
}

# Check 2: the switch LSN against the newest segment of /wal on the base timeline.
case_pitr_check_segment() {
  local lsn
  new_pitr_check_case check-segment-early
  pitr_check_fails "switch LSN in segment 3" "$CHECK_END_ERROR" "STUB_HISTORY_TEXT=$(pitr_history 0/3FFFFFF)" --
  new_pitr_check_case check-segment-gap
  pitr_check_fails "switch LSN in segment 2" "$CHECK_END_ERROR" "STUB_HISTORY_TEXT=$(pitr_history 0/2000100)" --
  for lsn in 0/4000001 0/4FFFFFF 0/5000100 1/0 A/12345678; do
    new_pitr_check_case "check-segment-pass-$lsn"
    pitr_check_passes "switch LSN $lsn" "STUB_HISTORY_TEXT=$(pitr_history "$lsn")" --
  done
  new_pitr_check_case check-segment-other-timeline
  add_pitr_segment 2 9
  pitr_check_fails "segment of another timeline does not count" "$CHECK_END_ERROR" \
    "STUB_HISTORY_TEXT=$(pitr_history 0/3000100)" --
  new_pitr_check_case check-segment-none
  rm -f -- "$CASE"/wal/0*
  pitr_check_fails "no segment of the base timeline" "holds no segment of the base timeline" --
}

# new_pitr_second_timeline_case <name>: a check case with a base backup on timeline 2. /wal holds the
# segments 2 to 4 of timeline 2 and no segment of timeline 1. The promoted server runs on timeline 3.
new_pitr_second_timeline_case() {
  local number
  new_pitr_check_case "$1"
  rm -f -- "$CASE"/wal/0*
  sed -i 's/(file 00000001/(file 00000002/' "$CASE/wal/base/$BASE_A/backup_label"
  for number in 2 3 4; do
    add_pitr_segment 2 "$number"
  done
}

# pitr_second_timeline_history <first LSN> <last LSN>: a history file of two lines. Timeline 2 branched
# off timeline 1 at the first LSN. Timeline 3 branched off timeline 2 at the last LSN.
pitr_second_timeline_history() { printf '1	%s	no recovery target specified
2	%s	no recovery target specified
' "$1" "$2"; }

# Check 2 with a base backup on timeline 2 and a new timeline 3: the file is 00000003.history with two lines,
# and the segment names of /wal start with 00000002. The switch LSN is the field 2 of the last line.
case_pitr_check_second_timeline() {
  local lsn
  for lsn in 0/4000100 0/4000001 0/5000100 1/0; do
    new_pitr_second_timeline_case "check-second-pass-$lsn"
    pitr_check_passes "timeline 3: switch LSN $lsn" STUB_TIMELINE=00000003       "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/1000000 "$lsn")" --
    expect_ok "timeline 3: the switch LSN $lsn is the one in the query"       grep -qF "pg_walfile_name('$lsn'" "$CASE/stub/log"
  done
  new_pitr_second_timeline_case check-second-segment-after
  add_pitr_segment 2 5
  pitr_check_fails "timeline 3: segment 5 of timeline 2 lies after the switch LSN" "$CHECK_END_ERROR" STUB_TIMELINE=00000003     "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/1000000 0/4000100)" --
  new_pitr_second_timeline_case check-second-segment-early
  pitr_check_fails "timeline 3: switch LSN in segment 3" "$CHECK_END_ERROR" STUB_TIMELINE=00000003     "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/1000000 0/3FFFFFF)" --
  new_pitr_second_timeline_case check-second-last-line-pass
  pitr_check_passes "timeline 3: an early first line does not matter" STUB_TIMELINE=00000003     "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/1000000 0/4000100)" --
  new_pitr_second_timeline_case check-second-last-line-fail
  pitr_check_fails "timeline 3: a late first line does not help" "$CHECK_END_ERROR" STUB_TIMELINE=00000003     "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/9000000 0/3FFFFFF)" --
  new_pitr_second_timeline_case check-second-other-timeline
  add_pitr_segment 1 9
  add_pitr_segment 3 9
  pitr_check_passes "timeline 3: segments of timeline 1 and 3 do not count" STUB_TIMELINE=00000003     "STUB_HISTORY_TEXT=$(pitr_second_timeline_history 0/1000000 0/4000100)" --
}

# A .partial file counts as its segment.
case_pitr_check_partial() {
  new_pitr_check_case check-partial-fail
  add_pitr_segment 1 5 .partial
  pitr_check_fails "partial: switch LSN in segment 4" "$CHECK_END_ERROR" --
  new_pitr_check_case check-partial-pass
  add_pitr_segment 1 5 .partial
  pitr_check_passes "partial: switch LSN in segment 5" "STUB_HISTORY_TEXT=$(pitr_history 0/5000100)" --
  new_pitr_check_case check-partial-only
  rm -f -- "$CASE"/wal/0*
  add_pitr_segment 1 6 .partial
  pitr_check_passes "partial only: switch LSN in segment 6" "STUB_HISTORY_TEXT=$(pitr_history 0/6000100)" --
  new_pitr_check_case check-partial-lower
  add_pitr_segment 1 3 .partial
  pitr_check_passes "partial below the newest segment" --
}

# PITR_EXPECTED_LSN: the switch LSN must be at or after it. The comparison runs in SQL with pg_lsn.
case_pitr_check_expected_lsn() {
  local value
  new_pitr_check_case check-expected-after
  pitr_check_fails "expected LSN after the switch LSN" "$CHECK_END_ERROR" PITR_EXPECTED_LSN=0/4000101 --
  expect_eq "expected after: one comparison in SQL" "$(pitr_psql_calls 'pg_lsn')" 2
  new_pitr_check_case check-expected-high
  pitr_check_fails "expected LSN in a higher word" "$CHECK_END_ERROR" PITR_EXPECTED_LSN=1/0 --
  for value in 0/4000100 0/4000000 0/1; do
    new_pitr_check_case "check-expected-$value"
    pitr_check_passes "expected LSN $value" "PITR_EXPECTED_LSN=$value" --
    expect_ok "expected $value: the comparison runs in SQL" grep -qE "^ARGV psql .*>=.*pg_lsn" "$CASE/stub/log"
  done
}

# With a target time, check 2 does not run: no history file, no ERROR line for an early LSN or a late expected LSN.
case_pitr_check_target_time() {
  new_pitr_check_case check-target-no-history
  pitr_check_passes "target time without a history file" "STUB_HISTORY_TEXT=$(pitr_history 0/2000100)" \
    PITR_EXPECTED_LSN=1/0 -- --target-time "$START_TARGET_TIME"
  expect_eq "target time: two queries of check 1, the chain query, the roles query, and the four value queries" \
    "$(pitr_tool_calls psql)" 8
  expect_eq "target time: no segment name of a switch LSN" "$(pitr_psql_calls "pg_lsn")" 0
  new_pitr_check_case check-target-missing-history
  pitr_run STUB_HISTORY_MISSING=1 -- --target-time "$START_TARGET_TIME"
  expect_eq "target time: a missing history file is no fault, exit code" "$RC" 0
  expect_eq "target time: a missing history file is no fault, no ERROR line" "$(error_count "$CASE/out")" 0
  new_pitr_check_case check-target-timeline
  pitr_check_fails "target time: check 1 still runs" "$CHECK_TIMELINE_ERROR" STUB_TIMELINE=00000001 -- --target-time "$START_TARGET_TIME"
}

# A missing or malformed history file gives one ERROR line.
case_pitr_check_history() {
  local text label
  new_pitr_check_case check-history-missing
  pitr_run STUB_HISTORY_MISSING=1 --
  expect_no "missing history: exit code is not 0" test "$RC" -eq 0
  expect_eq "missing history: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "missing history: the ERROR line holds the text" error_has "$CHECK_HISTORY_ERROR"
  for label in empty text no-tab lowercase no-slash long-word comment word; do
    case $label in
      empty) text="" ;;
      text) text=$'garbage\n' ;;
      no-tab) text=$'1\n' ;;
      lowercase) text=$'1\t0/4a00100\tx\n' ;;
      no-slash) text=$'1\t4000100\tx\n' ;;
      long-word) text=$'1\t0/123456789\tx\n' ;;
      comment) text=$'1\t0/4000100\tx\n# a comment\n' ;;
      word) text=$'one\t0/4000100\tx\n' ;;
    esac
    new_pitr_check_case "check-history-$label"
    pitr_check_fails "history $label" "$CHECK_HISTORY_ERROR" "STUB_HISTORY_TEXT=$text" --
  done
  new_pitr_check_case check-history-last-line
  pitr_check_passes "history: an earlier line does not matter" \
    "STUB_HISTORY_TEXT=$(printf '1\t0/1000000\tx\n\n2\t0/4000100\ty\n')" --
}

# A fault of psql gives one ERROR line.
case_pitr_check_psql_fails() {
  new_pitr_check_case check-psql-fails
  pitr_check_fails "psql fails" "cannot read the recovery state of the recovered cluster" STUB_PSQL_FAIL=1 --
}

# The script and the stub agree on the facts that the cases rely on.
case_pitr_check_static_rules() {
  expect_ok "check: the error text of check 2 is in the script" \
    grep -qF "$CHECK_END_ERROR" "$BIN/pitr-volume"
  expect_no "check: the script uses no bash arithmetic on an LSN" \
    grep -qE 'lsn.*\$\(\(' "$BIN/pitr-volume"
}

# pitr_query_count <kind>: the number of queries of this kind that the psql stub answered.
pitr_query_count() { grep -c "^QUERY $1\$" "$CASE/stub/log" || true; }

# Check 3: a break, a head that differs in the sequence, a head that differs in the hash, a fault of psql.
case_pitr_check_chain() {
  local result number=0
  new_pitr_check_case check-chain-pass
  pitr_check_passes "chain pass" --
  expect_eq "chain pass: one chain query" "$(pitr_query_count chain)" 1
  for result in "1|t|t" "3|t|t" "0|f|t" "0|t|f" "2|f|f" "" "0|t" "x"; do
    number=$((number + 1))
    new_pitr_check_case "check-chain-$number"
    pitr_check_fails "chain result [$result]" "$CHECK_CHAIN_ERROR" "STUB_CHAIN=$result" --
    expect_eq "chain result [$result]: no marker query" "$(pitr_query_count marker)" 0
    expect_eq "chain result [$result]: no roles query" "$(pitr_query_count roles)" 0
  done
  new_pitr_check_case check-chain-fails
  pitr_check_fails "chain query fails" "cannot run the audit chain check on the recovered cluster" STUB_CHAIN_FAIL=1 --
  new_pitr_check_case check-chain-first
  pitr_check_fails "chain runs before the file check" "$CHECK_CHAIN_ERROR" "STUB_CHAIN=3|t|t" \
    STUB_PGCTL_CHANGE_FILE=postgresql.conf --
}

# Check 4: a marker that the recovered cluster holds, a marker that it lacks, and no marker.
case_pitr_check_marker() {
  local seq=$CHECK_MARKER_SEQ hash=$CHECK_MARKER_HASH
  new_pitr_check_case check-marker-pass
  pitr_check_passes "marker pass" "PITR_MARKER_SEQ=$seq" "PITR_MARKER_HASH=$hash" --
  expect_eq "marker pass: one marker query" "$(pitr_query_count marker)" 1
  expect_ok "marker pass: the query holds the sequence and the hash" \
    grep -qF "chain_seq = $seq AND entry_hash = pg_catalog.decode('$hash', 'hex')" "$CASE/stub/log"
  new_pitr_check_case check-marker-none
  pitr_check_passes "no marker" --
  expect_eq "no marker: no marker query" "$(pitr_query_count marker)" 0
  new_pitr_check_case check-marker-missing
  pitr_check_fails "missing marker" "$CHECK_MARKER_ERROR" "PITR_MARKER_SEQ=$seq" "PITR_MARKER_HASH=$hash" STUB_MARKER_COUNT=0 --
  expect_eq "missing marker: no roles query" "$(pitr_query_count roles)" 0
  new_pitr_check_case check-marker-twice
  pitr_check_fails "two marker rows" "$CHECK_MARKER_ERROR" "PITR_MARKER_SEQ=$seq" "PITR_MARKER_HASH=$hash" STUB_MARKER_COUNT=2 --
  new_pitr_check_case check-marker-fails
  pitr_check_fails "marker query fails" "cannot read the marker entry in the recovered cluster" \
    "PITR_MARKER_SEQ=$seq" "PITR_MARKER_HASH=$hash" STUB_MARKER_FAIL=1 --
  new_pitr_check_case check-marker-target-time
  pitr_check_passes "marker with a target time" "PITR_MARKER_SEQ=$seq" "PITR_MARKER_HASH=$hash" -- --target-time "$START_TARGET_TIME"
  expect_eq "marker with a target time: one marker query" "$(pitr_query_count marker)" 1
}

# Step 1: a marker value outside the grammars of ADR-0077 (b) stops the run before any tool and any copy.
case_pitr_check_marker_input() {
  local value number=0 hash=$CHECK_MARKER_HASH
  for value in 0 01 abc -1 " 1" "1 " 1.5 12345678901234567890 ""; do
    number=$((number + 1))
    new_pitr_case "marker-seq-$number"
    pitr_refused_case "bad PITR_MARKER_SEQ [$value]" "PITR_MARKER_SEQ" "PITR_MARKER_SEQ=$value" "PITR_MARKER_HASH=$hash" --
    pitr_refused_no_lock "bad PITR_MARKER_SEQ [$value]"
  done
  for value in "${hash^^}" "${hash%?}" "${hash}0" "${hash%?}g" "${hash%?} " ""; do
    number=$((number + 1))
    new_pitr_case "marker-hash-$number"
    pitr_refused_case "bad PITR_MARKER_HASH [$value]" "PITR_MARKER_HASH" "PITR_MARKER_SEQ=$CHECK_MARKER_SEQ" "PITR_MARKER_HASH=$value" --
    pitr_refused_no_lock "bad PITR_MARKER_HASH [$value]"
  done
  new_pitr_case marker-only-seq
  pitr_refused_case "only PITR_MARKER_SEQ" "PITR_MARKER_SEQ and PITR_MARKER_HASH" "PITR_MARKER_SEQ=$CHECK_MARKER_SEQ" --
  new_pitr_case marker-only-hash
  pitr_refused_case "only PITR_MARKER_HASH" "PITR_MARKER_SEQ and PITR_MARKER_HASH" "PITR_MARKER_HASH=$hash" --
  new_pitr_case marker-edge
  pitr_run "PITR_MARKER_SEQ=9223372036854775807" "PITR_MARKER_HASH=$hash" --
  expect_eq "marker edge (19 digits): exit code" "$RC" 0
  expect_eq "marker edge (19 digits): no ERROR line" "$(error_count "$CASE/out")" 0
}

# Step 1: COMPOSE_SUBNET fills the placeholder of restore-roles.expected, so it must be an IPv4 subnet.
case_pitr_check_subnet() {
  local value number=0
  for value in "" 172.28.0.0 172.28.0.0/33 172.28.0.0/0 256.1.1.1/16 172.28.0/16 "172.28.0.0/16 " 'a|b/16'; do
    number=$((number + 1))
    new_pitr_case "subnet-$number"
    pitr_refused_case "bad COMPOSE_SUBNET [$value]" "COMPOSE_SUBNET is not an IPv4 subnet" "COMPOSE_SUBNET=$value" --
    pitr_refused_no_lock "bad COMPOSE_SUBNET [$value]"
  done
}

# Check 5: a configuration file of step 7 that changed or vanished, and a signal file that stays.
case_pitr_check_files() {
  local file
  for file in "${COPY_CONFIG_FILES[@]}"; do
    new_pitr_check_case "check-files-$file"
    pitr_check_fails "changed $file" "$CHECK_CONFIG_ERROR" "STUB_PGCTL_CHANGE_FILE=$file" --
    expect_ok "changed $file: the ERROR line names the file" error_has "\"file\":\"$file\""
    expect_eq "changed $file: no roles query" "$(pitr_query_count roles)" 0
    new_pitr_check_case "check-files-gone-$file"
    pitr_check_fails "removed $file" "$CHECK_CONFIG_ERROR" "STUB_PGCTL_REMOVE_FILE=$file" --
  done
  new_pitr_check_case check-files-recovery-signal
  pitr_check_fails "recovery.signal stays" "$CHECK_SIGNAL_ERROR" STUB_KEEP_RECOVERY_SIGNAL=1 --
  expect_ok "recovery.signal stays: the ERROR line names the file" error_has '"file":"recovery.signal"'
  new_pitr_check_case check-files-standby-signal
  pitr_check_fails "standby.signal stays" "$CHECK_SIGNAL_ERROR" STUB_LEFTOVER_SIGNAL=standby.signal --
  expect_ok "standby.signal stays: the ERROR line names the file" error_has '"file":"standby.signal"'
  expect_eq "standby.signal stays: no roles query" "$(pitr_query_count roles)" 0
  new_pitr_check_case check-files-unlisted
  pitr_check_passes "a file outside the four may change" STUB_LEFTOVER_SIGNAL=other.signal STUB_PGCTL_CHANGE_FILE=other.conf --
}

# pitr_lib_without <file>: a copy of the library directory without one file. The path is below the case.
pitr_lib_without() {
  mkdir "$CASE/lib"
  cp -- "$ROOT/etc"/* "$CASE/lib/"
  rm -- "$CASE/lib/$1"
  printf '%s' "$CASE/lib"
}

# Check 6: the role values against restore-roles.expected with the subnet of COMPOSE_SUBNET.
case_pitr_check_roles() {
  local label
  new_pitr_check_case check-roles-pass
  pitr_check_passes "roles pass" --
  expect_eq "roles pass: one roles query" "$(pitr_query_count roles)" 1
  expect_ok "roles pass: the query file is restore-roles.sql" grep -qE '^ARGV psql .*--file=[^ ]*/restore-roles\.sql' "$CASE/stub/log"
  expect_ok "roles pass: the query runs on the database recipe" grep -qE '^ARGV psql .*--dbname=recipe' "$CASE/stub/log"
  new_pitr_check_case check-roles-subnet
  pitr_check_passes "another subnet" COMPOSE_SUBNET=10.1.0.0/16 STUB_ROLES_SUBNET=10.1.0.0/16 --
  new_pitr_check_case check-roles-wrong-subnet
  pitr_check_fails "the subnet differs" "$CHECK_ROLES_ERROR" COMPOSE_SUBNET=10.2.0.0/16 STUB_ROLES_SUBNET=10.1.0.0/16 --
  new_pitr_check_case check-roles-missing
  pitr_check_fails "missing line" "$CHECK_ROLES_ERROR" STUB_ROLES_MISSING=1 --
  expect_ok "missing line: the ERROR line counts it" error_has '"missing":1,"extra":0'
  new_pitr_check_case check-roles-extra
  pitr_check_fails "extra line" "$CHECK_ROLES_ERROR" "STUB_ROLES_EXTRA=GRANT|recipe_app|DELETE ON evil" --
  expect_ok "extra line: the ERROR line counts it" error_has '"missing":0,"extra":1'
  new_pitr_check_case check-roles-change
  pitr_check_fails "changed line" "$CHECK_ROLES_ERROR" STUB_ROLES_CHANGE=1 --
  expect_ok "changed line: the ERROR line counts it" error_has '"missing":1,"extra":1'
  new_pitr_check_case check-roles-swap
  pitr_check_fails "swapped lines" "$CHECK_ROLES_ERROR" STUB_ROLES_SWAP=1 --
  expect_ok "swapped lines: the ERROR line counts none" error_has '"missing":0,"extra":0'
  new_pitr_check_case check-roles-fails
  pitr_check_fails "roles query fails" "restore-roles.sql failed on the recovered cluster" STUB_ROLES_FAIL=1 --
  for label in sql expected; do
    new_pitr_case "check-roles-file-$label"
    pitr_refused_case "missing restore-roles.$label" "restore-roles.sql or restore-roles.expected is missing" \
      "WALARCHIVE_LIB_DIR=$(pitr_lib_without "restore-roles.$label")" --
  done
}

# The script loads cluster-access.sh for the check of COMPOSE_SUBNET.
case_pitr_check_library() {
  new_pitr_case lib-cluster-access.sh
  pitr_refused_case "missing cluster-access.sh" "cannot load cluster-access.sh" \
    "WALARCHIVE_LIB_DIR=$(pitr_lib_without cluster-access.sh)" --
}
