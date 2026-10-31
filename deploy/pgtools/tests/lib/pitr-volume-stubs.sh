# Sourced by pitr-volume-test. It writes the stubs that pitr-volume needs and the case helpers.
#
# The stubs of the PostgreSQL tools write one line "EVENT <name>" and one line "ARGV <name> <arguments>"
# to the file STUB_LOG. A gate case asserts that this file stays empty, so it proves that no tool ran
# before the gate refused the run. The stub of flock writes to the file STUB_DIR/flock.log and not to
# STUB_LOG, so a case can tell the lock from the PostgreSQL tools. The stub of id answers the user of
# the switches below. It writes no line to STUB_LOG.
#
# Switches, set as VAR=value in the environment of one run:
#   STUB_ID_UID=<digits>   the user id that `id -u` prints, default 999.
#   STUB_ID_NAME=<text>    the user name that `id -un` prints, default postgres.
#   STUB_ID_FAIL=1         `id -u` and `id -un` fail.
#   STUB_DF_AVAIL=<digits> the free bytes that df prints, default 1000000000000.
#   STUB_DF_SIZE=<digits>  the disk size that df prints, default 2000000000000.
#   STUB_DF_TEXT=<text>    the one data line that df prints instead of the two numbers.
#   STUB_DF_FAIL=1         df fails.
#   STUB_VERIFY_FAIL=1     pg_verifybackup fails with one line on standard error.
#   STUB_CONTROL_ID=<text> the system identifier that pg_controldata prints, default 7300000000000000001.
#   STUB_CONTROL_FAIL=1    pg_controldata fails.
#   STUB_CONTROL_STATE=<text> the cluster state that pg_controldata prints, default "shut down".
#   STUB_CONTROL_TLI=<digits> the checkpoint timeline that pg_controldata prints, default the decimal value of STUB_TIMELINE.
#   STUB_CONTROL_FAIL_AFTER_STOP=1 pg_controldata fails after a stop of pg_ctl.
#   STUB_PGCTL_START_FAIL=1 `pg_ctl start` fails with one line on standard error.
#   STUB_PGCTL_STOP_FAIL=1 `pg_ctl --mode=fast stop` fails with one line on standard error. The immediate stop works.
#   STUB_SYNC_FAIL=1       sync fails with one line on standard error.
#   STUB_PGCTL_NO_PID=1    `pg_ctl start` succeeds but writes no postmaster.pid, as if the postmaster exited.
#   STUB_PSQL_FAIL=1       psql fails with one line on standard error.
#   STUB_PSQL_RECOVERED=<t|f> the answer to the query pg_is_in_recovery (true means "recovered"), default t.
#   STUB_TIMELINE=<text>   the timeline of the promoted server, 8 hex digits, default 00000002. It names the
#                          file <timeline>.history that a start writes and is the answer to the timeline query.
#   STUB_HISTORY_TEXT=<text> the content of that history file. A start writes no file when the variable is unset.
#   STUB_CHAIN=<text>      the answer to the audit chain query, default 0|t|t. STUB_CHAIN_FAIL=1 makes it fail.
#   STUB_MARKER_COUNT=<n>  the answer to the marker query, default 1. STUB_MARKER_FAIL=1 makes it fail.
#   STUB_KEEP_RECOVERY_SIGNAL=1 the promotion leaves recovery.signal in the target. By default the answer to
#                          the recovery state query removes it, as a promotion does.
#   STUB_LEFTOVER_SIGNAL=<name> the same answer also writes this signal file into the target.
#   STUB_PGCTL_CHANGE_FILE=<name> `pg_ctl start` appends one line to this file of the target.
#   STUB_PGCTL_REMOVE_FILE=<name> `pg_ctl start` deletes this file of the target.
#   STUB_ROLES_SUBNET=<text> the subnet that fills the placeholder of the answer to the roles query,
#                          default 172.28.0.0/16. The answer is restore-roles.expected next to the query file.
#   STUB_ROLES_MISSING=1, STUB_ROLES_EXTRA=<line>, STUB_ROLES_CHANGE=1, STUB_ROLES_SWAP=1 change that answer:
#                          drop the first line, add a line, change the first line, swap the first two lines.
#   STUB_ROLES_FAIL=1      the roles query fails.
#   STUB_LAST_SEQ=<text>, STUB_LAST_AT=<text>, STUB_LAST_ACTION=<text>, STUB_HEARTBEAT=<text> the answers to the four
#                          value queries of step 10, default 57, 2026-01-02T03:04:05.123456Z, RecipeApproved, and
#                          2026-01-02T03:04:00.000000Z. A set variable with an empty value gives an empty answer.
#   STUB_VALUE_FAIL=<seq|at|action|heartbeat> the value query of that name fails.
#   The stub of psql writes one line "QUERY <kind>" for the queries chain, marker, last_seq, last_at, last_action,
#   heartbeat, and roles.
#   STUB_HISTORY_MISSING=1 `pg_ctl start` writes no history file, even when STUB_HISTORY_TEXT is set.
# The helper pitr_run sets the defaults of a complete run: the private directory $CASE/pgpass and a history file
# with the switch LSN PITR_SWITCH_LSN. A case with valid inputs therefore runs all 12 steps of pitr-volume
# and ends with the line "pitr volume completed". A case changes a default with a later VAR=value.
# The stub of pg_ctl logs the lines PGCTL_ACTION, PGCTL_O, PGCTL_TIMEOUT, PGCTL_FD9, and PGCTL_LOCK for each
# call. PGCTL_O is the argument of -o. PGCTL_FD9 tells if the call inherited file descriptor 9 (present or
# absent). PGCTL_LOCK tells if the lock of the WAL archive is held (held or free) at that time. A start writes
# 25 lines to the server log and the file postmaster.pid. Before it writes the file, a start records the state of
# the target at that moment. These records replace a look at the target after the run, because the later steps
# change it (the promotion removes recovery.signal, the history file appears, the stop removes postmaster.pid):
#   STUB_DIR/start-listing  name, type, and size of every entry of the target (find output, sorted)
#   STUB_DIR/start-hashes   the sha256sum lines of the four configuration files of the target
#   START_RECOVERY_SIGNAL   file or absent, START_RECOVERY_SIZE the size in bytes, START_STANDBY_SIGNAL
#                           present or absent (lines of STUB_LOG) The stub of pg_isready logs ISREADY_LOCK too.
# The stub of df replaces `df --output=avail,size -B1 <path>` and writes no line to STUB_LOG.
# The stub of sync writes SYNC_STOPPED (yes when a stop of pg_ctl ran) and SYNC_DRILL (present when the file
# pitr-drill exists in the target) to STUB_LOG.
# The stub of flock needs the variable REAL_FLOCK. It writes the access mode of file descriptor 9
# (0 is read-only) and the arguments of each call, then it runs the real flock.

# The PostgreSQL tools that a later step of pitr-volume runs. A stub of each one proves "no tool ran".
PITR_STUB_TOOLS=(initdb pg_ctl pg_controldata pg_isready pg_resetwal pg_verifybackup pg_waldump)

make_pitr_tool_stub() {
  cat >"$ROOT/stubs/$1" <<STUB
#!/usr/bin/env bash
echo "EVENT $1" >>"\$STUB_LOG"
echo "ARGV $1 \$*" >>"\$STUB_LOG"
exit 0
STUB
}

make_pitr_flock_stub() {
  cat >"$ROOT/stubs/flock" <<'STUB'
#!/usr/bin/env bash
flags=$(awk '/^flags:/ { print $2 }' "/proc/$$/fdinfo/9" 2>/dev/null)
echo "ARGV flock $*" >>"$STUB_DIR/flock.log"
echo "FDMODE $((8#${flags:-0} & 3))" >>"$STUB_DIR/flock.log"
exec "$REAL_FLOCK" "$@"
STUB
}

make_pitr_id_stub() {
  cat >"$ROOT/stubs/id" <<'STUB'
#!/usr/bin/env bash
[ -z "${STUB_ID_FAIL:-}" ] || { echo "id: cannot find name for user ID" >&2; exit 1; }
case "$*" in
  -u) echo "${STUB_ID_UID:-999}" ;;
  -un) echo "${STUB_ID_NAME:-postgres}" ;;
  *) exec "$REAL_ID" "$@" ;;
esac
STUB
}

make_pitr_df_stub() {
  cat >"$ROOT/stubs/df" <<'STUB'
#!/usr/bin/env bash
[ -z "${STUB_DF_FAIL:-}" ] || { echo "df: cannot read the file system" >&2; exit 1; }
echo "Avail 1B-blocks"
echo "${STUB_DF_TEXT:-${STUB_DF_AVAIL:-1000000000000} ${STUB_DF_SIZE:-2000000000000}}"
STUB
}

# The stub of pg_verifybackup logs its call and fails on request. It checks nothing else.
make_pitr_verify_stub() {
  cat >"$ROOT/stubs/pg_verifybackup" <<'STUB'
#!/usr/bin/env bash
echo "EVENT pg_verifybackup" >>"$STUB_LOG"
echo "ARGV pg_verifybackup $*" >>"$STUB_LOG"
[ -z "${STUB_VERIFY_FAIL:-}" ] || { echo "pg_verifybackup: error: stub failure" >&2; exit 1; }
exit 0
STUB
}

# The stub of pg_controldata logs its call and prints the field of the system identifier.
make_pitr_controldata_stub() {
  cat >"$ROOT/stubs/pg_controldata" <<'STUB'
#!/usr/bin/env bash
echo "EVENT pg_controldata" >>"$STUB_LOG"
echo "ARGV pg_controldata $*" >>"$STUB_LOG"
[ -z "${STUB_CONTROL_FAIL:-}" ] || { echo "pg_controldata: error: stub failure" >&2; exit 1; }
if [ -n "${STUB_CONTROL_FAIL_AFTER_STOP:-}" ] && [ -e "$STUB_DIR/stopped" ]; then
  echo "pg_controldata: error: stub failure after the stop" >&2
  exit 1
fi
echo "pg_control version number:            1300"
echo "Database system identifier:           ${STUB_CONTROL_ID-7300000000000000001}"
echo "Database cluster state:               ${STUB_CONTROL_STATE-shut down}"
tli=${STUB_CONTROL_TLI-}
if [ -z "$tli" ] && [[ ${STUB_TIMELINE:-00000002} =~ ^[0-9A-F]{1,8}$ ]]; then tli=$((16#${STUB_TIMELINE:-00000002})); fi
echo "Latest checkpoint's TimeLineID:       $tli"
STUB
}

make_pitr_pgctl_stub() {
  cat >"$ROOT/stubs/pg_ctl" <<'STUB'
#!/usr/bin/env bash
echo "EVENT pg_ctl" >>"$STUB_LOG"
echo "ARGV pg_ctl $*" >>"$STUB_LOG"
args=("$@")
action="" data="" logfile="" timeout="" opts=""
i=0
while [ "$i" -lt "${#args[@]}" ]; do
  case ${args[$i]} in
    --pgdata=*) data=${args[$i]#--pgdata=} ;;
    --log=*) logfile=${args[$i]#--log=} ;;
    --timeout=*) timeout=${args[$i]#--timeout=} ;;
    -o) i=$((i + 1)); opts=${args[$i]} ;;
    start | stop | status) action=${args[$i]} ;;
  esac
  i=$((i + 1))
done
echo "PGCTL_ACTION $action" >>"$STUB_LOG"
case $action in
  start)
    echo "PGCTL_O $opts" >>"$STUB_LOG"
    echo "PGCTL_TIMEOUT $timeout" >>"$STUB_LOG"
    if [ -e "/proc/$$/fdinfo/9" ]; then echo "PGCTL_FD9 present" >>"$STUB_LOG"; else echo "PGCTL_FD9 absent" >>"$STUB_LOG"; fi
    if "$REAL_FLOCK" -n "$WAL_DIR/.lock" true; then echo "PGCTL_LOCK free" >>"$STUB_LOG"; else echo "PGCTL_LOCK held" >>"$STUB_LOG"; fi
    (cd "$data" && find . -mindepth 1 -printf '%p %y %s\n' | LC_ALL=C sort) >"$STUB_DIR/start-listing"
    (cd "$data" && sha256sum postgresql.conf postgresql.auto.conf pg_hba.conf pg_ident.conf 2>/dev/null) >"$STUB_DIR/start-hashes" || true
    if [ -f "$data/recovery.signal" ]; then echo "START_RECOVERY_SIGNAL file" >>"$STUB_LOG"; else echo "START_RECOVERY_SIGNAL absent" >>"$STUB_LOG"; fi
    echo "START_RECOVERY_SIZE $(stat -c %s "$data/recovery.signal" 2>/dev/null || echo 0)" >>"$STUB_LOG"
    if [ -e "$data/standby.signal" ]; then echo "START_STANDBY_SIGNAL present" >>"$STUB_LOG"; else echo "START_STANDBY_SIGNAL absent" >>"$STUB_LOG"; fi
    for n in $(seq 1 25); do echo "server log line $n"; done >"$logfile"
    if [ -n "${STUB_PGCTL_START_FAIL:-}" ]; then
      echo "pg_ctl: error: stub failure of the start" >&2
      exit 1
    fi
    [ -n "${STUB_PGCTL_NO_PID:-}" ] || : >"$data/postmaster.pid"
    if [ -n "${STUB_HISTORY_TEXT+set}" ] && [ -z "${STUB_HISTORY_MISSING:-}" ]; then
      mkdir -p "$data/pg_wal"
      printf '%s' "$STUB_HISTORY_TEXT" >"$data/pg_wal/${STUB_TIMELINE:-00000002}.history"
    fi
    [ -z "${STUB_PGCTL_CHANGE_FILE:-}" ] || echo "# written during the recovery" >>"$data/$STUB_PGCTL_CHANGE_FILE"
    [ -z "${STUB_PGCTL_REMOVE_FILE:-}" ] || rm -f -- "$data/$STUB_PGCTL_REMOVE_FILE"
    ;;
  stop)
    if [ -n "${STUB_PGCTL_STOP_FAIL:-}" ] && [[ " $* " == *" --mode=fast "* ]]; then
      echo "pg_ctl: error: stub failure of the stop" >&2
      exit 1
    fi
    : >"$STUB_DIR/stopped"
    rm -f -- "$data/postmaster.pid"
    ;;
  status)
    [ -f "$data/postmaster.pid" ] || exit 3
    ;;
esac
exit 0
STUB
}

make_pitr_isready_stub() {
  cat >"$ROOT/stubs/pg_isready" <<'STUB'
#!/usr/bin/env bash
echo "EVENT pg_isready" >>"$STUB_LOG"
echo "ARGV pg_isready $*" >>"$STUB_LOG"
if "$REAL_FLOCK" -n "$WAL_DIR/.lock" true; then echo "ISREADY_LOCK free" >>"$STUB_LOG"; else echo "ISREADY_LOCK held" >>"$STUB_LOG"; fi
exit 0
STUB
}

# The stub of psql logs its call and answers the queries of the checks after the promotion (ADR-0106 (g)).
# It matches the text of the query. The name of a segment follows pg_walfile_name: the timeline, then the
# segment that holds the byte before the position (XLByteToPrevSeg), for segments of 16 MiB.
# The query file of the roles check (--file) gets restore-roles.expected as its answer.
make_pitr_psql_stub() {
  cat >"$ROOT/stubs/psql" <<'STUB'
#!/usr/bin/env bash
echo "EVENT psql" >>"$STUB_LOG"
echo "ARGV psql $*" >>"$STUB_LOG"
[ -z "${STUB_PSQL_FAIL:-}" ] || { echo "psql: error: stub failure" >&2; exit 1; }
sql=""
file=""
for arg in "$@"; do
  case $arg in
    --command=*) sql=${arg#--command=} ;;
    --file=*) file=${arg#--file=} ;;
  esac
done
if [ -n "$file" ]; then
  echo "QUERY roles" >>"$STUB_LOG"
  [ -z "${STUB_ROLES_FAIL:-}" ] || { echo "psql: error: stub failure of the roles query" >&2; exit 1; }
  roles=$(sed "s|@COMPOSE_SUBNET@|${STUB_ROLES_SUBNET:-172.28.0.0/16}|g" "${file%.sql}.expected")
  [ -z "${STUB_ROLES_MISSING:-}" ] || roles=$(printf '%s\n' "$roles" | sed '1d')
  [ -z "${STUB_ROLES_CHANGE:-}" ] || roles=$(printf '%s\n' "$roles" | sed '1s/$/x/')
  [ -z "${STUB_ROLES_SWAP:-}" ] || roles=$(printf '%s\n' "$roles" | awk 'NR == 1 { first = $0; next } NR == 2 { print; print first; next } { print }')
  printf '%s\n' "$roles"
  [ -z "${STUB_ROLES_EXTRA:-}" ] || printf '%s\n' "$STUB_ROLES_EXTRA"
  exit 0
fi
lsn_value() { echo $(((0x${1%%/*} << 32) | 0x${1##*/})); }
mapfile -t lsns < <(grep -oE '[0-9A-F]+/[0-9A-F]+' <<<"$sql")
case $sql in
  *audit_chain_head*)
    echo "QUERY chain" >>"$STUB_LOG"
    [ -z "${STUB_CHAIN_FAIL:-}" ] || { echo "psql: error: stub failure of the chain query" >&2; exit 1; }
    echo "${STUB_CHAIN-0|t|t}"
    ;;
  *"FROM public.audit_log WHERE chain_seq"*)
    echo "QUERY marker" >>"$STUB_LOG"
    [ -z "${STUB_MARKER_FAIL:-}" ] || { echo "psql: error: stub failure of the marker query" >&2; exit 1; }
    echo "${STUB_MARKER_COUNT:-1}"
    ;;
  "SELECT chain_seq FROM"*)
    echo "QUERY last_seq" >>"$STUB_LOG"
    [ "${STUB_VALUE_FAIL:-}" != seq ] || { echo "psql: error: stub failure of the chain_seq query" >&2; exit 1; }
    printf '%s
' "${STUB_LAST_SEQ-57}"
    ;;
  *"AT TIME ZONE"*)
    echo "QUERY last_at" >>"$STUB_LOG"
    [ "${STUB_VALUE_FAIL:-}" != at ] || { echo "psql: error: stub failure of the occurred_at query" >&2; exit 1; }
    printf '%s
' "${STUB_LAST_AT-2026-01-02T03:04:05.123456Z}"
    ;;
  "SELECT action FROM"*)
    echo "QUERY last_action" >>"$STUB_LOG"
    [ "${STUB_VALUE_FAIL:-}" != action ] || { echo "psql: error: stub failure of the action query" >&2; exit 1; }
    printf '%s
' "${STUB_LAST_ACTION-RecipeApproved}"
    ;;
  *heartbeat.last_at*)
    echo "QUERY heartbeat" >>"$STUB_LOG"
    [ "${STUB_VALUE_FAIL:-}" != heartbeat ] || { echo "psql: error: stub failure of the heartbeat query" >&2; exit 1; }
    printf '%s
' "${STUB_HEARTBEAT-2026-01-02T03:04:00.000000Z}"
    ;;
  *pg_is_in_recovery*)
    [ -n "${STUB_KEEP_RECOVERY_SIGNAL:-}" ] || rm -f -- "$PGDATA/recovery.signal"
    [ -z "${STUB_LEFTOVER_SIGNAL:-}" ] || : >"$PGDATA/$STUB_LEFTOVER_SIGNAL"
    echo "${STUB_PSQL_RECOVERED:-t}"
    ;;
  *pg_current_wal_lsn*) echo "${STUB_TIMELINE:-00000002}" ;;
  *">="*)
    if [ "$(lsn_value "${lsns[0]}")" -ge "$(lsn_value "${lsns[1]}")" ]; then echo t; else echo f; fi
    ;;
  *pg_walfile_name*)
    segment=$((($(lsn_value "${lsns[0]}") - 1) >> 24))
    printf '%s%08X%08X\n' "${STUB_TIMELINE:-00000002}" $((segment >> 8)) $((segment & 255))
    ;;
esac
exit 0
STUB
}

# The stub of sync logs its call, the state of the stop, and the file pitr-drill. It fails on request.
make_pitr_sync_stub() {
  cat >"$ROOT/stubs/sync" <<'STUB'
#!/usr/bin/env bash
echo "EVENT sync" >>"$STUB_LOG"
echo "ARGV sync $*" >>"$STUB_LOG"
if [ -e "$STUB_DIR/stopped" ]; then echo "SYNC_STOPPED yes" >>"$STUB_LOG"; else echo "SYNC_STOPPED no" >>"$STUB_LOG"; fi
if [ -e "$PGDATA/pitr-drill" ]; then echo "SYNC_DRILL present" >>"$STUB_LOG"; else echo "SYNC_DRILL absent" >>"$STUB_LOG"; fi
[ -z "${STUB_SYNC_FAIL:-}" ] || { echo "sync: error: stub failure" >&2; exit 1; }
exit 0
STUB
}

make_pitr_stubs() {
  local tool
  mkdir -p "$ROOT/stubs"
  for tool in "${PITR_STUB_TOOLS[@]}"; do
    make_pitr_tool_stub "$tool"
  done
  make_pitr_sync_stub
  make_pitr_psql_stub
  make_pitr_pgctl_stub
  make_pitr_isready_stub
  make_pitr_verify_stub
  make_pitr_controldata_stub
  make_pitr_flock_stub
  make_pitr_id_stub
  make_pitr_df_stub
  chmod 0755 "$ROOT/stubs"/*
}

# The four configuration files of a base backup (ADR-0105 (q)). Step 7 hashes them.
COPY_CONFIG_FILES=(postgresql.conf postgresql.auto.conf pg_hba.conf pg_ident.conf)
# The default switch LSN of the history file. The newest segment of the default WAL archive is number 4.
PITR_SWITCH_LSN=0/4000100

# add_pitr_base <name> <backup_end_time content> <START TIME text> [<size of the data file>]: one directory
# below wal/base. The content "-" writes no file backup_end_time. The text "-" writes no line START TIME.
# The directory also holds the four configuration files and the file origin, which names the directory.
# A case reads origin in the target to see which base backup the copy took.
# A time text is one line, for example 2026-01-01T00:10:00Z and "2026-01-01 00:00:00 UTC".
add_pitr_base() {
  local dir=$CASE/wal/base/$1 file
  mkdir -p "$dir"
  head -c "${4:-100}" /dev/zero >"$dir/data"
  printf '%s\n' "$1" >"$dir/origin"
  for file in "${COPY_CONFIG_FILES[@]}"; do
    printf '# %s of the base backup\n' "$file" >"$dir/$file"
  done
  {
    printf 'START WAL LOCATION: 0/2000028 (file 000000010000000000000002)
'
    [ "$3" = - ] || printf 'START TIME: %s
' "$3"
    printf 'LABEL: stub
'
  } >"$dir/backup_label"
  [ "$2" = - ] || printf '%s\n' "$2" >"$dir/backup_end_time"
}

# add_pitr_segment <timeline> <segment number> [<suffix>]: an empty segment file at the top of /wal.
add_pitr_segment() {
  local name
  name=$(printf '%08X%08X%08X' "$1" 0 "$2")
  : >"$CASE/wal/$name${3:-}"
}

# new_pitr_case <name>: a case that runs all 12 steps. It has an empty target, an empty private directory,
# a WAL archive with the marker file, the lock file, a valid .system-identifier, the segments 2 to 4 of
# timeline 1, and one base backup that ends long before every target time of the cases,
# an empty tool log, and an empty lock log.
new_pitr_case() {
  local number
  CASE=$ROOT/$1
  mkdir -p "$CASE/target" "$CASE/pgpass" "$CASE/wal/base" "$CASE/stub"
  for number in 2 3 4; do
    add_pitr_segment 1 "$number"
  done
  : >"$CASE/wal/.walarchive-marker"
  : >"$CASE/wal/.lock"
  printf '7300000000000000001\n' >"$CASE/wal/.system-identifier"
  add_pitr_base base_20260101_000000 2026-01-01T00:01:00Z "2026-01-01 00:00:00 UTC"
  : >"$CASE/stub/log"
  : >"$CASE/stub/flock.log"
}

# reset_pitr_run: empty the target, the private directory, and the stub records. A case that runs the script
# several times calls it before each later run, because a completed run fills the target.
reset_pitr_run() {
  find "${CASE:?}/target" -mindepth 1 -delete
  find "$CASE/pgpass" -mindepth 1 -delete
  rm -f -- "$CASE/stub/stopped" "$CASE/stub/start-listing" "$CASE/stub/start-hashes"
  : >"$CASE/stub/log"
  : >"$CASE/stub/flock.log"
}

# pitr_run [VAR=value ...] -- [argument ...]: run pitr-volume with the stubs and the files of the case.
# It writes out and err in the case directory and sets RC. A later VAR=value wins over a default.
pitr_run() {
  local envs=()
  while [ "$1" != "--" ]; do
    envs+=("$1")
    shift
  done
  shift
  env PATH="$ROOT/stubs:$PATH" COMPOSE_SUBNET=172.28.0.0/16 PGDATA="$CASE/target" WAL_DIR="$CASE/wal" WALARCHIVE_LIB_DIR="$ROOT/etc" \
    BACKUP_PGPASS_DIR="$CASE/pgpass" STUB_HISTORY_TEXT="$(pitr_history "$PITR_SWITCH_LSN")" STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" REAL_FLOCK="$REAL_FLOCK" REAL_ID="$REAL_ID" \
    ${envs[@]+"${envs[@]}"} bash "$BIN/pitr-volume" "$@" >"$CASE/out" 2>"$CASE/err"
  RC=$?
}

# pitr_history <switch LSN>: the text of a history file of timeline 1 with this switch LSN in its last line.
pitr_history() { printf '1\t%s\tno recovery target specified\n' "$1"; }

# pitr_json_ok <file>: every line is one JSON object of the log format with the job pitr-volume.
pitr_json_ok() {
  ! grep -qvE '^\{"timestamp":"[0-9TZ:-]+","level":"(INFO|WARN|ERROR)","job":"pitr-volume","message":"[^"]+"(,"[a-z_]+":("[^"]*"|[0-9]+|true|false))*\}$' "$1"
}

# error_has <text>: the one ERROR line holds the text.
error_has() { grep '"level":"ERROR"' "$CASE/out" | grep -qF -- "$1"; }

# pitr_listing <directory>: name, type, size, and mode of every entry below the directory.
pitr_listing() { (cd "$1" && find . -mindepth 1 -printf '%p %y %s %m\n' | LC_ALL=C sort); }
