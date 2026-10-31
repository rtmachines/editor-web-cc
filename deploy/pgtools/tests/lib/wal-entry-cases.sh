# Sourced by walarchive-test. Static rules of the scripts and the cases of walarchive-entry
# and walarchive-health (ADR-0068 (a) to (d), design D10).

WAL_SCRIPTS=(walarchive-entry walarchive-health basebackup-once pitr-check)
EXPECTED_PGPASS='db:5432:replication:recipe_replicator:ab\:c\\d9'

# The rules of every script of the image (ADR-0066 (m) to (o)).
case_wal_static_rules() {
  local s f
  local crontab_dir=${PGTOOLS_CRONTAB_DIR:-$HERE/..}
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
  expect_ok "static crontab: exists" test -f "$crontab_dir/crontab.walarchive"
  expect_no "static crontab: no carriage return" grep -q $'\r' "$crontab_dir/crontab.walarchive"
  expect_eq "static crontab: lines" \
    "$(grep -vE '^(#|$)' "$crontab_dir/crontab.walarchive" 2>/dev/null | tr '\n' ',')" \
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

# The image holds the mount point of the tmpfs /run/pgpass with mode 1777 and the owner root.
# runc gives the tmpfs the mode of an existing mount point. A restart then keeps the mode of the first start.
case_wal_pgpass_mount_point() {
  local dir=/run/pgpass
  expect_ok "image: the mount point $dir is a directory" test -d "$dir"
  expect_no "image: the mount point $dir is no symbolic link" test -L "$dir"
  expect_eq "image: the mode and the owner of $dir" "$(stat -c %a:%u -- "$dir" 2>/dev/null)" "1777:0"
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
  expect_eq "entry: the archive stays" "$(ls -A "$W" | LC_ALL=C sort | tr '\n' ' ')" ".lock .system-identifier .walarchive-marker base "
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

# --- identifier guard (ADR-0103 (s), design D7) ------------------------------------------------

ID_FILE_NAME=.system-identifier
MSG_NO_FILE='segments are in the WAL archive and the file .system-identifier is missing'
MSG_OTHER='the file .system-identifier holds the identifier of another system'
MSG_MALFORMED='the file .system-identifier is malformed'
MSG_UNREADABLE='cannot read the file .system-identifier'
MSG_NO_SERVER='cannot read the system identifier of the server'
ID_LISTING='.lock .system-identifier .walarchive-marker base '

# guard_run [VAR=value ...]: a run of walarchive-entry that ends after two seconds when the guard passes.
guard_run() { wal_run STUB_SUPERCRONIC_SLEEP=2 "$@" -- walarchive-entry; }

id_file_content() { cat "$W/$ID_FILE_NAME" 2>/dev/null; }

top_listing() { ls -A "$W" | LC_ALL=C sort | tr '\n' ' '; }

# log_line <text>: the number of the first line of the log of the stubs that holds the text.
log_line() { grep -nF -- "$1" "$CASE/stub/log" | head -n 1 | cut -d: -f1; }

# expect_guard_passed <name>: pg_receivewal started once, and the guard wrote no message.
expect_guard_passed() {
  expect_eq "$1: pg_receivewal started" "$(count_events 'pg_receivewal start')" "1"
  expect_eq "$1: no guard message" "$(grep -ci 'identifier' "$CASE/out" || true)" "0"
}

# expect_guard_refused <name> <message> <snapshot-before>: one ERROR line with the message,
# no tool started, and no change of /wal.
expect_guard_refused() {
  wal_expect_refused "$1" "$3"
  expect_message "$1" "$2"
}

# make_ln_stub: a stub of ln in $CASE/lnstub that acts on a call which names the identifier file.
#   STUB_LN_WINNER_ID=n  another run publishes the file with the identifier n just before this call
#   STUB_LN_BARRIER=dir  the call waits until two runs have reached it
# It sets LN_ENV for wal_run.
make_ln_stub() {
  mkdir -p "$CASE/lnstub" "$CASE/barrier"
  cat >"$CASE/lnstub/ln" <<'EOF'
#!/usr/bin/env bash
last=${!#}
case $last in
  */.system-identifier)
    if [ -n "${STUB_LN_WINNER_ID:-}" ] && [ ! -e "$last" ]; then
      printf '%s\n' "$STUB_LN_WINNER_ID" >"$last"
    fi
    if [ -n "${STUB_LN_BARRIER:-}" ]; then
      : >"$STUB_LN_BARRIER/arrived.$$"
      n=100
      while [ "$(ls "$STUB_LN_BARRIER" | wc -l)" -lt 2 ] && [ "$n" -gt 0 ]; do
        sleep 0.1
        n=$((n - 1))
      done
    fi
    ;;
esac
exec "$REAL_LN" "$@"
EOF
  chmod 0755 "$CASE/lnstub/ln"
  LN_ENV=("PATH=$CASE/lnstub:$ROOT/stubs:$PATH" "REAL_LN=$(command -v ln)")
}

# make_sync_stub: a stub of sync in $CASE/syncstub. It records "SYNC <name>" and whether the final
# file exists at that call, then it succeeds. It acts only on calls with an argument.
#   STUB_SYNC_FAIL=file  the call on a temp name of the identifier file fails
#   STUB_SYNC_FAIL=dir   the call on the directory /wal fails
# It sets SYNC_ENV for wal_run.
make_sync_stub() {
  mkdir -p "$CASE/syncstub"
  cat >"$CASE/syncstub/sync" <<'EOF'
#!/usr/bin/env bash
[ "$1" = "--" ] && shift
[ "$#" -gt 0 ] || exit 0
final=no
[ -e "$WAL_DIR/.system-identifier" ] && final=yes
case $1 in
  "$WAL_DIR"/.system-identifier.tmp.*) kind=file name=TEMP ;;
  "$WAL_DIR") kind=dir name=WAL_DIR ;;
  *) kind=other name=$1 ;;
esac
echo "SYNC $name final=$final" >>"$STUB_LOG"
[ "${STUB_SYNC_FAIL:-}" = "$kind" ] && exit 1
exit 0
EOF
  chmod 0755 "$CASE/syncstub/sync"
  SYNC_ENV=("PATH=$CASE/syncstub:$ROOT/stubs:$PATH")
}

# sync_log: the SYNC lines of the log of the stubs, joined with a semicolon.
sync_log() { grep '^SYNC ' "$CASE/stub/log" 2>/dev/null | paste -sd ';' -; }

# The first start writes the file with two sync calls (ADR-0105 (t)).
# The first call names the temp file before the link. The second names /wal after it.
case_guard_sync_calls() {
  wal_new_case guard-sync-calls
  make_sync_stub
  guard_run "${SYNC_ENV[@]}"
  expect_guard_passed "guard sync calls"
  expect_eq "guard sync calls: order" "$(sync_log)" "SYNC TEMP final=no;SYNC WAL_DIR final=yes"
  expect_eq "guard sync calls: file" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard sync calls: no temp file" "$(top_listing)" "$ID_LISTING"
}

# A failed sync on the temp file leaves no temp name and no final file. A failed sync on /wal
# leaves no temp name. Each run writes one ERROR line.
case_guard_sync_fail() {
  local before
  wal_new_case guard-sync-fail-file
  make_sync_stub
  before=$(wal_snapshot)
  guard_run "${SYNC_ENV[@]}" STUB_SYNC_FAIL=file
  expect_guard_refused "guard sync fail on the file" "cannot write the file .system-identifier" "$before"
  expect_eq "guard sync fail on the file: only the first call" "$(sync_log)" "SYNC TEMP final=no"
  wal_new_case guard-sync-fail-dir
  make_sync_stub
  guard_run "${SYNC_ENV[@]}" STUB_SYNC_FAIL=dir
  expect_eq "guard sync fail on the dir: exit code" "$RC" "1"
  expect_eq "guard sync fail on the dir: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_message "guard sync fail on the dir" "the file .system-identifier exists, but the sync of the directory failed"
  expect_eq "guard sync fail on the dir: the file stays" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard sync fail on the dir: no tool started" "$(count_events '')" "0"
  expect_eq "guard sync fail on the dir: no temp file" "$(top_listing)" "$ID_LISTING"
}

case_guard_static() {
  local f=$HERE/../lib/wal-identity.sh out rc
  expect_ok "static identity: file exists" test -f "$f"
  expect_ok "static identity: bash -n" bash -n "$f"
  expect_no "static identity: no carriage return" grep -q $'\r' "$f"
  expect_no "static identity: no PGPASSWORD" grep -qE 'PGPASSWORD=|export PGPASSWORD' "$f"
  expect_eq "static identity: every rm has the guarded form" \
    "$(grep -E '(^|[^a-z_-])rm ' "$f" 2>/dev/null | grep -vE '^[[:space:]]*#' \
      | grep -vcE 'rm( -r| -f| -rf)? -- "\$\{[a-z_]+:\?\}' || true)" "0"
  # The file runs nothing at source time. A clean environment has none of the names it uses.
  out=$(cd "$ROOT" && env -i PATH="$PATH" bash -c 'set -euo pipefail; source "$1"' _ "$f" 2>&1)
  rc=$?
  expect_eq "static identity: sourcing exit code" "$rc" "0"
  expect_eq "static identity: sourcing prints nothing" "$out" ""
  expect_ok "static entry: sources the guard" grep -qF 'wal-identity.sh' "$BIN/walarchive-entry"
}

# Scenario "First start of the WAL archive".
case_guard_first_start() {
  wal_new_case guard-first
  guard_run
  expect_eq "guard first start: file content" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard first start: file mode" "$(stat -c %a "$W/$ID_FILE_NAME" 2>/dev/null)" "600"
  expect_eq "guard first start: one line" "$(wc -l <"$W/$ID_FILE_NAME" 2>/dev/null | tr -d ' ')" "1"
  expect_guard_passed "guard first start"
  expect_eq "guard first start: only the file is new" "$(top_listing)" "$ID_LISTING"
  expect_ok "guard first start: replication connection" grep -qF 'replication=true' "$CASE/stub/log"
  expect_ok "guard first start: the pgpass file exists" grep -qF 'CALL psql pgpass present' "$CASE/stub/log"
  expect_ok "guard first start: the pgpass line matches the connection" grep -qF 'CALL psql pgpass match' "$CASE/stub/log"
  expect_ok "guard first start: the guard runs before pg_receivewal" \
    test "$(log_line 'CALL psql identify')" -lt "$(log_line 'EVENT pg_receivewal start')"
  expect_no "guard first start: no password in the log" grep -qF -- 'c\d9' "$CASE/stub/log"
}

# Names that are no segment of ADR-0099 (d) do not count. The first start still works.
case_guard_other_names() {
  wal_new_case guard-names
  : >"$W/0000000100000000000000A"
  : >"$W/00000001000000000000000a"
  : >"$W/0000000100000000000000001"
  : >"$W/000000010000000000000001.history"
  : >"$W/000000010000000000000001.partial.tmp"
  : >"$W/00000002.history"
  guard_run
  expect_eq "guard other names: file content" "$(id_file_content)" "$WAL_SYSID"
  expect_guard_passed "guard other names"
}

# Scenario "Segments without the file". A segment or a .partial file counts.
case_guard_segments_without_file() {
  local before label
  for label in segment partial both; do
    wal_new_case "guard-nofile-$label"
    case $label in
      segment) printf 'segment\n' >"$W/$(seg_name 1)" ;;
      partial) printf 'partial\n' >"$W/$(seg_name 2).partial" ;;
      both) make_segments 1 3 ;;
    esac
    before=$(wal_snapshot)
    guard_run
    expect_guard_refused "guard segments without file ($label)" "$MSG_NO_FILE" "$before"
    expect_no "guard segments without file ($label): no file" test -e "$W/$ID_FILE_NAME"
  done
}

# Scenario "Another system".
case_guard_other_system() {
  local before
  wal_new_case guard-other
  printf '%s\n' "$OTHER_SYSID" >"$W/$ID_FILE_NAME"
  make_segments 1 3
  before=$(wal_snapshot)
  guard_run
  expect_guard_refused "guard another system" "$MSG_OTHER" "$before"
  expect_eq "guard another system: file kept" "$(id_file_content)" "$OTHER_SYSID"
  wal_new_case guard-other-nosegment
  printf '%s\n' "$OTHER_SYSID" >"$W/$ID_FILE_NAME"
  before=$(wal_snapshot)
  guard_run
  expect_guard_refused "guard another system, no segment" "$MSG_OTHER" "$before"
}

# Scenario "Same system".
case_guard_same_system() {
  local before
  wal_new_case guard-same
  printf '%s\n' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  make_segments 1 3
  before=$(wal_snapshot)
  guard_run
  expect_guard_passed "guard same system"
  expect_eq "guard same system: archive unchanged" "$(wal_snapshot)" "$before"
  wal_new_case guard-same-nonewline
  printf '%s' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  make_segments 1 3
  guard_run
  expect_guard_passed "guard same system, no line end"
}

# Scenario "Gap of the WAL stream": the operator moved the segments, and the file stays.
case_guard_gap() {
  local before
  wal_new_case guard-gap
  printf '%s\n' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  before=$(wal_snapshot)
  guard_run
  expect_guard_passed "guard gap"
  expect_eq "guard gap: file kept" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard gap: archive unchanged" "$(wal_snapshot)" "$before"
}

# Scenario "Malformed identifier file": empty, CR, a character other than a digit, more than one line.
guard_malformed_case() {
  local label=$1 before
  wal_new_case "guard-malformed-$label"
  case $label in
    empty) : >"$W/$ID_FILE_NAME" ;;
    empty-with-segment)
      : >"$W/$ID_FILE_NAME"
      make_segments 1 3
      ;;
    line-end-only) printf '\n' >"$W/$ID_FILE_NAME" ;;
    cr) printf '%s\r\n' "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
    non-digit) printf '73549x7216845319027\n' >"$W/$ID_FILE_NAME" ;;
    space) printf '%s \n' "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
    minus) printf -- '-%s\n' "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
    nul) printf '7354987216\000845319027\n' >"$W/$ID_FILE_NAME" ;;
    two-lines) printf '%s\n%s\n' "$WAL_SYSID" "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
    blank-line) printf '%s\n\n' "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
    no-end-two-lines) printf '%s\n%s' "$WAL_SYSID" "$WAL_SYSID" >"$W/$ID_FILE_NAME" ;;
  esac
  before=$(wal_snapshot)
  guard_run
  expect_guard_refused "guard malformed ($label)" "$MSG_MALFORMED" "$before"
}

# The file is a directory, or the server gives no identifier.
case_guard_unreadable() {
  local before
  wal_new_case guard-dir
  mkdir "$W/$ID_FILE_NAME"
  before=$(wal_snapshot)
  guard_run
  expect_guard_refused "guard file is a directory" "$MSG_UNREADABLE" "$before"
  wal_new_case guard-server-down
  before=$(wal_snapshot)
  guard_run STUB_IDENTIFY_FAIL=1
  expect_guard_refused "guard server down" "$MSG_NO_SERVER" "$before"
  wal_new_case guard-server-bad-answer
  before=$(wal_snapshot)
  guard_run 'STUB_IDENTIFY_OUT=x|1|0/0|'
  expect_guard_refused "guard bad answer of the server" "$MSG_NO_SERVER" "$before"
}

# The server is down and psql prints a two-line error with a quote and a control character.
# The ERROR line keeps the cleaned first line in the field detail (ADR-0105 (s)).
case_guard_error_detail() {
  local before err
  wal_new_case guard-error-detail
  before=$(wal_snapshot)
  err=$'psql: error: bad "host" (db)\001end\nSECOND LINE'
  guard_run STUB_IDENTIFY_FAIL=1 "STUB_IDENTIFY_ERR=$err"
  expect_guard_refused "guard error detail" "$MSG_NO_SERVER" "$before"
  expect_eq "guard error detail: field detail" \
    "$(grep -cF -- '"message":"cannot read the system identifier of the server","detail":"psql: error: bad ?host? (db)?end"}' "$CASE/out" || true)" "1"
  expect_eq "guard error detail: no second line" "$(grep -c 'SECOND' "$CASE/out" || true)" "0"
}

# Two first starts: the loser of the ln reads the file again and compares (design D7).
# The stub of ln makes the loss certain. A real race follows in case_guard_two_first_starts.
case_guard_lost_race() {
  wal_new_case guard-lost-same
  make_ln_stub
  guard_run "${LN_ENV[@]}" "STUB_LN_WINNER_ID=$WAL_SYSID"
  expect_guard_passed "guard lost race, same system"
  expect_eq "guard lost race, same system: file" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard lost race, same system: no temp file" "$(top_listing)" "$ID_LISTING"
  wal_new_case guard-lost-other
  make_ln_stub
  guard_run "${LN_ENV[@]}" "STUB_LN_WINNER_ID=$OTHER_SYSID"
  expect_eq "guard lost race, other system: exit code" "$RC" "1"
  expect_eq "guard lost race, other system: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_message "guard lost race, other system" "$MSG_OTHER"
  expect_eq "guard lost race, other system: no tool started" "$(count_events '')" "0"
  expect_eq "guard lost race, other system: file of the winner" "$(id_file_content)" "$OTHER_SYSID"
  expect_eq "guard lost race, other system: no temp file" "$(top_listing)" "$ID_LISTING"
}

# Two real runs reach the publish at the same time. Both must start pg_receivewal.
case_guard_two_first_starts() {
  local pid_a pid_b
  wal_new_case guard-race
  make_ln_stub
  OUTTAG=a WAL_BG=1 wal_run "${LN_ENV[@]}" "STUB_LN_BARRIER=$CASE/barrier" STUB_SUPERCRONIC_SLEEP=2 -- walarchive-entry
  pid_a=$BG_PID
  OUTTAG=b WAL_BG=1 wal_run "${LN_ENV[@]}" "STUB_LN_BARRIER=$CASE/barrier" STUB_SUPERCRONIC_SLEEP=2 -- walarchive-entry
  pid_b=$BG_PID
  BG_PID=$pid_a
  OUTTAG=a wait_bg
  BG_PID=$pid_b
  OUTTAG=b wait_bg
  expect_eq "guard race: both runs reached the publish" "$(find "$CASE/barrier" -type f | wc -l | tr -d ' ')" "2"
  expect_eq "guard race: pg_receivewal started twice" "$(count_events 'pg_receivewal start')" "2"
  expect_eq "guard race: no guard message in run a" "$(grep -ci 'identifier' "$CASE/outa" || true)" "0"
  expect_eq "guard race: no guard message in run b" "$(grep -ci 'identifier' "$CASE/outb" || true)" "0"
  expect_eq "guard race: file content" "$(id_file_content)" "$WAL_SYSID"
  expect_eq "guard race: no temp file" "$(top_listing)" "$ID_LISTING"
}

# --- timeline check (ADR-0106 (m), design D5 of the change pitr-recovery) ---------------------

MSG_TIMELINE='segments of another timeline are in the WAL archive'

# tl_seg <timeline> <number>: the name of a segment with the given 8-digit timeline.
tl_seg() { printf '%s%s%08X' "$1" "$WAL_LOG_ID" "$2"; }

# timeline_refused_case <label> <file name ...>: the stub answers timeline 2, the identifier file
# matches, and the archive holds the files. The run stops with the message and changes nothing.
timeline_refused_case() {
  local label=$1 before name
  shift
  wal_new_case "guard-timeline-$label"
  printf '%s
' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  for name in "$@"; do printf 'wal
' >"$W/$name"; done
  before=$(wal_snapshot)
  guard_run STUB_TIMELINE=2
  expect_guard_refused "guard timeline ($label)" "$MSG_TIMELINE" "$before"
}

# timeline_passed_case <label> <file name ...>: the same setup, and the guard passes.
timeline_passed_case() {
  local label=$1 before name
  shift
  wal_new_case "guard-timeline-$label"
  printf '%s
' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  for name in "$@"; do printf 'wal
' >"$W/$name"; done
  before=$(wal_snapshot)
  guard_run STUB_TIMELINE=2
  expect_guard_passed "guard timeline ($label)"
  expect_eq "guard timeline ($label): archive unchanged" "$(wal_snapshot)" "$before"
}

# Scenarios "Segment of another timeline" and "Partial file of another timeline".
case_guard_timeline_refused() {
  timeline_refused_case segment "$(tl_seg 00000001 5)"
  timeline_refused_case partial "$(tl_seg 00000001 6).partial"
  timeline_refused_case mixed "$(tl_seg 00000002 4)" "$(tl_seg 00000002 5)" "$(tl_seg 00000001 6)"
  timeline_refused_case newer "$(tl_seg 00000003 4)"
}

# A history file is no segment. A segment of the timeline of the server passes.
case_guard_timeline_passed() {
  timeline_passed_case history "00000002.history" "00000001.history"
  timeline_passed_case history-with-segments "00000001.history" "$(tl_seg 00000002 4)"
  timeline_passed_case segments "$(tl_seg 00000002 4)" "$(tl_seg 00000002 5)" "$(tl_seg 00000002 6).partial"
}

# The decimal timeline of the server is compared as 8-digit upper-case hex.
case_guard_timeline_hex() {
  local before
  wal_new_case guard-timeline-hex
  printf '%s
' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  printf 'wal
' >"$W/$(tl_seg 0000000A 4)"
  guard_run STUB_TIMELINE=10
  expect_guard_passed "guard timeline hex"
  wal_new_case guard-timeline-hex-other
  printf '%s
' "$WAL_SYSID" >"$W/$ID_FILE_NAME"
  printf 'wal
' >"$W/$(tl_seg 00000010 4)"
  before=$(wal_snapshot)
  guard_run STUB_TIMELINE=10
  expect_guard_refused "guard timeline hex, decimal text" "$MSG_TIMELINE" "$before"
}

# The identifier check comes first. A file of another system gives its own message.
case_guard_timeline_after_identifier() {
  local before
  wal_new_case guard-timeline-order
  printf '%s
' "$OTHER_SYSID" >"$W/$ID_FILE_NAME"
  printf 'wal
' >"$W/$(tl_seg 00000001 5)"
  before=$(wal_snapshot)
  guard_run STUB_TIMELINE=2
  expect_guard_refused "guard timeline after identifier" "$MSG_OTHER" "$before"
  expect_eq "guard timeline after identifier: no timeline message" "$(grep -cF -- "$MSG_TIMELINE" "$CASE/out" || true)" "0"
}

# A first start with no segment passes. The file is written and no timeline matters.
case_guard_timeline_first_start() {
  wal_new_case guard-timeline-first
  guard_run STUB_TIMELINE=2
  expect_guard_passed "guard timeline first start"
  expect_eq "guard timeline first start: file content" "$(id_file_content)" "$WAL_SYSID"
}

case_wal_guard() {
  local label
  case_guard_static
  case_guard_first_start
  case_guard_other_names
  case_guard_segments_without_file
  case_guard_other_system
  case_guard_same_system
  case_guard_gap
  for label in empty empty-with-segment line-end-only cr non-digit space minus nul two-lines blank-line \
    no-end-two-lines; do
    guard_malformed_case "$label"
  done
  case_guard_unreadable
  case_guard_error_detail
  case_guard_lost_race
  case_guard_sync_calls
  case_guard_sync_fail
  case_guard_two_first_starts
  case_guard_timeline_refused
  case_guard_timeline_passed
  case_guard_timeline_hex
  case_guard_timeline_after_identifier
  case_guard_timeline_first_start
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
