# Sourced by restore-volume-test. It holds the helpers and the cases of steps 1 and 2 of ADR-0103 (i):
# the target, the stamp, COMPOSE_SUBNET, and the lock. The cases of steps 3 and 4 are in
# lib/restore-volume-gate-cases.sh.
# It needs lib/helpers.sh, lib/restore-stubs.sh, lib/restore-helpers.sh, and lib/restore-volume-stubs.sh.

VOLUME_SUBNET=172.28.0.0/16

# The EVENT lines of a run through steps 1 to 8 (ADR-0103 (i)). The stubs log the tools in this order.
# The script sets no role password, so the sequence has no EVENT alter-role and no EVENT login.
# Steps 7 and 8 add the REVOKE statements and the query of restore-roles.sql. A run that passes both ends at ACCESS.
VOLUME_SEQ_START="EVENT list,EVENT initdb,EVENT pg_ctl start,"
VOLUME_SEQ_GLOBALS="${VOLUME_SEQ_START}EVENT globals,"
VOLUME_SEQ_PRE="${VOLUME_SEQ_GLOBALS}EVENT pg_restore pre-data,EVENT code pre,"
VOLUME_SEQ_POST="${VOLUME_SEQ_PRE}EVENT pg_restore data,EVENT pg_restore post-data,EVENT code post,"
VOLUME_SEQ_END="${VOLUME_SEQ_POST}EVENT counts,EVENT chain,"
VOLUME_SEQ_REVOKE="${VOLUME_SEQ_END}EVENT revoke,"
VOLUME_SEQ_ACCESS="${VOLUME_SEQ_REVOKE}EVENT roles,"
# Steps 9 and 10: the fast stop, pg_controldata, and sync. A run that passes all ten steps ends at DONE.
VOLUME_SEQ_DONE="${VOLUME_SEQ_ACCESS}EVENT pg_ctl stop,EVENT pg_controldata,EVENT sync,"

# new_volume_case <name>: a restore case with an empty target, the existing lock file, and no tool call.
# The target is the directory pgdata of the case. The script reads it from the variable PGDATA.
new_volume_case() {
  new_restore_case "$1"
  rm -- "$CASE/pgdata/PG_VERSION" "$B/.lock"
  cp -- "$HERE/../restore-roles.sql" "$HERE/../restore-roles.expected" "$CASE/code/"
  mkdir -p -- "$B/.scratch"
  : >"$B/.scratch/.lock"
  : >"$CASE/stub/flock.log"
}

# volume_run_stamp <stamp> [VAR=value ...]: run restore-volume with the stubs and the files of the case.
volume_run_stamp() {
  local stamp=$1
  shift
  run_script "RESTORE_CODE_DIR=$CASE/code" "RESTORE_GLOBALS_DIR=$ROOT/etc" PGDATABASE=recipe \
    "COMPOSE_SUBNET=$VOLUME_SUBNET" "REAL_FLOCK=$REAL_FLOCK" LANG=en_US.utf8 STUB_SNAPSHOT_WORK=1 "$@" \
    -- restore-volume "$stamp"
}

# volume_run [VAR=value ...]: the stamp of the set of the case.
volume_run() { volume_run_stamp "$RESTORE_NAME" "$@"; }

# target_listing: the names below the target directory.
target_listing() { find "$CASE/pgdata" -mindepth 1 -printf '%p %y %m\n' | LC_ALL=C sort; }

# volume_json_ok <file>: every line is one JSON object of the log format with the job restore-volume.
volume_json_ok() {
  ! grep -qvE '^\{"timestamp":"[0-9TZ:-]+","level":"(INFO|WARN|ERROR)","job":"restore-volume","message":"[^"]+"(,"[a-z_]+":("[^"]*"|[0-9]+))*\}$' "$1"
}

# volume_failed_checks <name> <backup listing> <target listing>: the checks of a run that a gate refused.
# The gates of steps 1 and 2 run before any PostgreSQL tool and change no file.
volume_failed_checks() {
  local name=$1
  expect_no "$name: exit code is not 0" test "$RC" -eq 0
  expect_eq "$name: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$name: the output is JSON of the job restore-volume" volume_json_ok "$CASE/out"
  expect_eq "$name: no PostgreSQL tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "$name: the backup directory is unchanged" "$(set_listing)" "$2"
  expect_eq "$name: the target directory is unchanged" "$(target_listing)" "$3"
  expect_eq "$name: no scratch run directory is new" "$(scratch_runs)" 0
  expect_eq "$name: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# failed_volume_case <name> <error text> <stamp> [VAR=value ...]: a refused run with the default case.
failed_volume_case() {
  local name=$1 text=$2 stamp=$3 before target
  shift 3
  new_volume_case "$name"
  before=$(set_listing)
  target=$(target_listing)
  volume_run_stamp "$stamp" "$@"
  volume_failed_checks "$name" "$before" "$target"
  expect_ok "$name: the ERROR line holds [$text]" error_has "$text"
}

# The script is a bash script of the same shape as restore-test.
case_volume_static_rules() {
  local script=$BIN/restore-volume
  expect_ok "static: restore-volume exists" test -f "$script"
  expect_ok "static: restore-volume is executable" test -x "$script"
  expect_ok "static: bash -n restore-volume" bash -n "$script"
  expect_no "static: xtrace" grep -Eq 'set[[:space:]]+-[A-Za-z]*x|xtrace' "$script"
  expect_no "static: carriage return" grep -q $'\r' "$script"
  expect_no "static: password in argv or environment" \
    grep -Eq 'PGPASSWORD=|--password[^-]|[[:space:]]-W([[:space:]]|$)' "$script"
  expect_no "static: rm -rf" grep -Eq 'rm[[:space:]]+-[A-Za-z]*r[A-Za-z]*f|rm[[:space:]]+-[A-Za-z]*f[A-Za-z]*r' "$script"
  expect_no "static: eval" grep -Eq '(^|[[:space:];&(])eval[[:space:]]' "$script"
  expect_no "static: fsync" grep -q 'fsync' "$script"
  expect_no "static: the lock opens for writing" grep -Eq 'exec[[:space:]]+9(>|<>)' "$script"
  expect_no "static: the stale-run cleanup of restore-test" grep -q 'delete_stale_runs' "$script"
  expect_eq "static: set and umask" \
    "$(grep -vE '^[[:space:]]*(#|$)' "$script" | sed -n '1p;2p' | tr '\n' '|')" \
    'set -euo pipefail|umask 077|'
  expect_ok "static: the detail length names its row of 7.11" \
    grep -Eq '^DETAIL_LENGTH=160 #.*7\.11' "$script"
}

# The stubs of pg_controldata and sync answer like the tools do. Later cases use them.
case_volume_stubs() {
  new_volume_case stubs
  local out
  out=$(env PATH="$ROOT/stubs:$PATH" STUB_LOG="$CASE/stub/log" pg_controldata --pgdata=x)
  expect_ok "stubs: pg_controldata prints the state" grep -qx 'Database cluster state: *shut down' <<<"$out"
  expect_ok "stubs: pg_controldata prints the identifier" \
    grep -qx 'Database system identifier: *7300000000000000001' <<<"$out"
  out=$(env PATH="$ROOT/stubs:$PATH" STUB_LOG="$CASE/stub/log" STUB_CONTROLDATA_STATE="in production" pg_controldata)
  expect_ok "stubs: pg_controldata prints another state" grep -qx 'Database cluster state: *in production' <<<"$out"
  expect_ok "stubs: sync succeeds" env PATH="$ROOT/stubs:$PATH" STUB_LOG="$CASE/stub/log" sync
  expect_eq "stubs: the calls are logged" "$(event_sequence)" "EVENT pg_controldata,EVENT pg_controldata,EVENT sync,"
}

# Usage: exactly one argument.
case_volume_usage() {
  local name
  for name in none two; do
    new_volume_case "usage-$name"
    if [ "$name" = none ]; then
      run_script "RESTORE_GLOBALS_DIR=$ROOT/etc" "COMPOSE_SUBNET=$VOLUME_SUBNET" -- restore-volume
    else
      run_script "RESTORE_GLOBALS_DIR=$ROOT/etc" "COMPOSE_SUBNET=$VOLUME_SUBNET" -- restore-volume 20261030_1200 x
    fi
    expect_eq "usage ($name): exit code" "$RC" 1
    expect_eq "usage ($name): one ERROR line" "$(error_count "$CASE/out")" 1
    expect_eq "usage ($name): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
    expect_eq "usage ($name): no lock call" "$(wc -c <"$CASE/stub/flock.log")" 0
  done
}

# Step 1: a stamp outside the grammar of the set names stops the run before any tool.
case_volume_bad_stamp() {
  local stamp i=0
  for stamp in 20261030_1200_old 2026103_1200 20261030_120 20261030_1200_premigrate_now \
    20261030_1200_Now 20261030-1200 x recipe_20261030_1200 ../20261030_1200 $'20261030_1200\n' ''; do
    i=$((i + 1))
    failed_volume_case "bad-stamp-$i" "stamp" "$stamp"
  done
}

# Step 1: a stamp of the grammar passes, with each of the three suffixes.
case_volume_good_stamp() {
  local suffix i=0 before target
  for suffix in "" _premigrate _now; do
    i=$((i + 1))
    new_volume_case "good-stamp-$i"
    make_restore_set "$suffix" 0
    before=$(set_listing)
    target=$(target_listing)
    volume_run
    expect_eq "good stamp (${suffix:-none}): exit code" "$RC" 0
    expect_eq "good stamp (${suffix:-none}): no ERROR line" "$(error_count "$CASE/out")" 0
    expect_ok "good stamp (${suffix:-none}): the output is JSON" volume_json_ok "$CASE/out"
    expect_eq "good stamp (${suffix:-none}): the backup directory is unchanged" "$(set_listing)" "$before"
    expect_no "good stamp (${suffix:-none}): initdb filled the target" test "$(target_listing)" = "$target"
    expect_ok "good stamp (${suffix:-none}): the target holds PG_VERSION" test -f "$CASE/pgdata/PG_VERSION"
  done
}

# Step 1: a target that is not empty stops the run. A live data directory is never empty.
case_volume_target_not_empty() {
  local kind before target
  for kind in version hidden directory; do
    new_volume_case "target-$kind"
    case $kind in
      version) printf '16\n' >"$CASE/pgdata/PG_VERSION" ;;
      hidden) : >"$CASE/pgdata/.hidden" ;;
      directory) mkdir "$CASE/pgdata/lost+found" ;;
    esac
    before=$(set_listing)
    target=$(target_listing)
    volume_run
    volume_failed_checks "target ($kind)" "$before" "$target"
    expect_ok "target ($kind): the ERROR line says not empty" error_has "not empty"
    expect_eq "target ($kind): the script took no lock" "$(wc -c <"$CASE/stub/flock.log")" 0
  done
}

# Step 1: a target that is no directory stops the run. The script never creates it.
case_volume_target_missing() {
  new_volume_case target-missing
  local before
  rmdir "$CASE/pgdata"
  before=$(set_listing)
  volume_run
  expect_no "target missing: exit code is not 0" test "$RC" -eq 0
  expect_eq "target missing: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "target missing: the ERROR line says missing" error_has "missing"
  expect_eq "target missing: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_no "target missing: the script did not create the target" test -e "$CASE/pgdata"
  expect_eq "target missing: the backup directory is unchanged" "$(set_listing)" "$before"
}

# Step 1: COMPOSE_SUBNET must be an IPv4 subnet (cluster-access.sh, check_compose_subnet).
case_volume_bad_subnet() {
  local subnet i=0
  for subnet in "" 172.28.0.0 172.28.0.0/33 172.28.0.0/0 256.1.1.1/8 172.28.0/16 a/b '172.28.0.0/16 ' \
    '172.28.0.0/16;x' 01.2.3.4/8; do
    i=$((i + 1))
    failed_volume_case "bad-subnet-$i" "COMPOSE_SUBNET" "$(date -u +%Y%m%d_%H%M)" "COMPOSE_SUBNET=$subnet"
  done
}

# Step 1: a missing COMPOSE_SUBNET stops the run, and the lock stays untouched.
case_volume_missing_subnet() {
  new_volume_case missing-subnet
  local before target
  before=$(set_listing)
  target=$(target_listing)
  (
    unset COMPOSE_SUBNET
    run_script "RESTORE_CODE_DIR=$CASE/code" "RESTORE_GLOBALS_DIR=$ROOT/etc" "REAL_FLOCK=$REAL_FLOCK"       -- restore-volume "$RESTORE_NAME"
  )
  RC=$(<"$CASE/rc")
  volume_failed_checks "missing subnet" "$before" "$target"
  expect_ok "missing subnet: the ERROR line names the variable" error_has "COMPOSE_SUBNET"
  expect_eq "missing subnet: the script took no lock" "$(wc -c <"$CASE/stub/flock.log")" 0
}

# The script stops when it cannot load a library. The first run has no library directory.
# The second run has restore-steps.sh and globals-grammar.sh but not cluster-access.sh.
case_volume_missing_library() {
  new_volume_case library-none
  run_script "RESTORE_GLOBALS_DIR=$CASE/none" "COMPOSE_SUBNET=$VOLUME_SUBNET" -- restore-volume "$RESTORE_NAME"
  expect_no "library (none): exit code is not 0" test "$RC" -eq 0
  expect_ok "library (none): stderr names the file" grep -qF restore-steps.sh "$CASE/err"
  expect_eq "library (none): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "library (none): no lock call" "$(wc -c <"$CASE/stub/flock.log")" 0
  new_volume_case library-cluster-access
  mkdir "$CASE/lib"
  cp -- "$ROOT/etc/restore-steps.sh" "$ROOT/etc/globals-grammar.sh" "$CASE/lib/"
  run_script "RESTORE_GLOBALS_DIR=$CASE/lib" "COMPOSE_SUBNET=$VOLUME_SUBNET" -- restore-volume "$RESTORE_NAME"
  expect_no "library (cluster-access.sh): exit code is not 0" test "$RC" -eq 0
  expect_ok "library (cluster-access.sh): stderr names the file" grep -qF cluster-access.sh "$CASE/err"
  expect_eq "library (cluster-access.sh): no lock call" "$(wc -c <"$CASE/stub/flock.log")" 0
}

# Step 2: a missing lock file stops the run. The script never creates the lock file or its directory.
# A directory or a symbolic link in place of the lock file stops the run too.
case_volume_missing_lock() {
  local before target
  new_volume_case missing-lock-file
  rm -- "$B/.scratch/.lock"
  before=$(set_listing)
  target=$(target_listing)
  volume_run
  volume_failed_checks "missing lock file" "$before" "$target"
  expect_ok "missing lock file: the ERROR line names the lock" error_has "lock"
  expect_no "missing lock file: the script did not create the lock file" test -e "$B/.scratch/.lock"
  new_volume_case missing-lock-directory
  rm -r -- "${B:?}/.scratch"
  before=$(set_listing)
  target=$(target_listing)
  volume_run
  volume_failed_checks "missing lock directory" "$before" "$target"
  expect_no "missing lock directory: the script did not create .scratch" test -e "$B/.scratch"
  new_volume_case lock-is-directory
  rm -- "$B/.scratch/.lock"
  mkdir "$B/.scratch/.lock"
  before=$(set_listing)
  target=$(target_listing)
  volume_run
  volume_failed_checks "lock is a directory" "$before" "$target"
  new_volume_case lock-is-symlink
  rm -- "$B/.scratch/.lock"
  : >"$B/.scratch/real-lock"
  ln -s real-lock "$B/.scratch/.lock"
  before=$(set_listing)
  target=$(target_listing)
  volume_run
  volume_failed_checks "lock is a symbolic link" "$before" "$target"
}

# Step 2: the script opens the lock read-only and takes flock without waiting.
case_volume_lock_read_only() {
  new_volume_case lock-read-only
  chmod 0444 "$B/.scratch/.lock"
  local before
  before=$(set_listing)
  volume_run
  expect_eq "lock read-only: exit code" "$RC" 0
  expect_eq "lock read-only: one flock call" "$(grep -c '^ARGV flock' "$CASE/stub/flock.log")" 1
  expect_ok "lock read-only: flock gets -n and descriptor 9" grep -qx 'ARGV flock -n 9' "$CASE/stub/flock.log"
  expect_ok "lock read-only: descriptor 9 has the access mode O_RDONLY" grep -qx 'FDMODE 0' "$CASE/stub/flock.log"
  expect_eq "lock read-only: the lock file keeps its size, mode, and time" "$(set_listing)" "$before"
  expect_eq "lock read-only: the lock file stays empty" "$(wc -c <"$B/.scratch/.lock")" 0
}

# Step 2: a second holder of the lock stops the run. A running restore-test holds the same lock.
case_volume_lock_held() {
  new_volume_case lock-held
  local before target holder
  "$REAL_FLOCK" -n "$B/.scratch/.lock" sleep 3 &
  holder=$!
  sleep 1
  before=$(set_listing)
  target=$(target_listing)
  volume_run
  volume_failed_checks "lock held" "$before" "$target"
  expect_ok "lock held: the ERROR line names the lock" error_has "holds the lock"
  wait "$holder" 2>/dev/null
}

# The script does not run the cleanup of stale runs. A run directory of a killed run stays as it is.
case_volume_keeps_stale_run() {
  new_volume_case stale-run
  mkdir -p "$B/.scratch/run.stale/data"
  printf '1\n' >"$B/.scratch/run.stale/data/postmaster.pid"
  printf 'x\n' >"$B/.scratch/run.stale/data/file"
  local before scratch_before
  before=$(set_listing)
  scratch_before=$(find "$B/.scratch" -printf '%p %y %s %m\n' | LC_ALL=C sort)
  volume_run
  expect_eq "stale run: exit code" "$RC" 0
  expect_eq "stale run: the backup files are unchanged" "$(set_listing)" "$before"
  expect_eq "stale run: the run directory and its files stay" \
    "$(find "$B/.scratch" -printf '%p %y %s %m\n' | LC_ALL=C sort)" "$scratch_before"
  expect_ok "stale run: the pid file stays" test -f "$B/.scratch/run.stale/data/postmaster.pid"
  expect_eq "stale run: the steps 1 to 10 ran" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_eq "stale run: only the server of this run was stopped" "$(event_count 'pg_ctl stop')" 1
  expect_eq "stale run: both pg_ctl calls name the target" \
    "$(grep -c -- "^ARGV pg_ctl --pgdata=$CASE/pgdata " "$CASE/stub/log")" 2
}

# A run through all steps writes nothing to the backup disk. Its only new directory is its own
# work directory in the tmpfs of the secret files, and the trap deletes that directory.
case_volume_writes_nothing() {
  new_volume_case writes-nothing
  local before
  before=$(set_listing)
  volume_run
  expect_eq "writes nothing: exit code" "$RC" 0
  expect_eq "writes nothing: the backup directory is unchanged" "$(set_listing)" "$before"
  expect_eq "writes nothing: the scratch directory holds the lock file only" \
    "$(find "$B/.scratch" -mindepth 1 | wc -l)" 1
  expect_eq "writes nothing: the trap deleted the work directory" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}
