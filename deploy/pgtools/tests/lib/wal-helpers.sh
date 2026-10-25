# Sourced by walarchive-test. It holds the helpers for the cases of the walarchive scripts.
# The counters and the expect functions come from helpers.sh.

WAL_TIMELINE=00000001
WAL_LOG_ID=00000000

# seg_name <number>: the file name of a WAL segment, 24 upper-case hex characters.
seg_name() { printf '%s%s%08X' "$WAL_TIMELINE" "$WAL_LOG_ID" "$1"; }

# wal_new_case <name>: a fresh WAL directory with the marker, the lock file, and a secret file.
wal_new_case() {
  CASE=$ROOT/$1
  W=$CASE/wal
  mkdir -p "$W/base" "$CASE/pgpass" "$CASE/stub" "$CASE/proc"
  chmod 0700 "$W" "$W/base"
  : >"$W/.walarchive-marker"
  : >"$W/.lock"
  : >"$CASE/crontab"
  printf '%s\n' "$SECRET" >"$CASE/secret"
  : >"$CASE/stub/log"
  # The time stamp of the new base backup. Every backdated base backup is older than it.
  # One fixed instant gives every name of the case, also when the case runs over a second boundary.
  WNOW=$("$REAL_DATE" -u +%s)
  WTS=$("$REAL_DATE" -u -d "@$WNOW" +%Y%m%d_%H%M%S)
}

# base_name_of_age <days>: the name of the base backup that make_base makes for this age.
base_name_of_age() {
  printf 'base_%s' "$("$REAL_DATE" -u -d "@$((WNOW - $1 * 86400))" +%Y%m%d_%H%M%S)"
}

# wal_run [VAR=value ...] -- <script> [args...]
# It writes out and err below $CASE and sets RC. The environment points at the case directory.
# With WAL_BG=1 the script runs in the background. BG_PID holds its process id.
wal_run() {
  local envs=()
  while [ "$1" != "--" ]; do
    envs+=("$1")
    shift
  done
  shift
  local script=$1
  shift
  local cmd=(env PATH="$ROOT/stubs:$PATH" WAL_DIR="$W" WALARCHIVE_PGPASS_DIR="$CASE/pgpass"
    WALARCHIVE_SECRET_FILE="$CASE/secret" WALARCHIVE_CRONTAB="$CASE/crontab"
    WALARCHIVE_PROC_DIR="$CASE/proc" STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub"
    STUB_TS="$WTS" ${envs[@]+"${envs[@]}"} bash "$BIN/$script" "$@")
  if [ -n "${WAL_BG:-}" ]; then
    "${cmd[@]}" >"$CASE/out${OUTTAG:-}" 2>"$CASE/err${OUTTAG:-}" &
    BG_PID=$!
    return 0
  fi
  "${cmd[@]}" >"$CASE/out${OUTTAG:-}" 2>"$CASE/err${OUTTAG:-}"
  RC=$?
  echo "$RC" >"$CASE/rc${OUTTAG:-}"
}

# wait_bg: wait for the background script and set RC.
wait_bg() {
  RC=0
  wait "$BG_PID" || RC=$?
  echo "$RC" >"$CASE/rc${OUTTAG:-}"
}

# wait_for_file <path> [<tenths of a second>]: poll until the file exists. Default 100 (10 s).
wait_for_file() {
  local n=${2:-100}
  while [ ! -e "$1" ] && [ "$n" -gt 0 ]; do
    sleep 0.1
    n=$((n - 1))
  done
  [ -e "$1" ]
}

# pid_alive <pid-file>: the process named in the file still runs.
pid_alive() {
  local pid
  pid=$(cat "$1" 2>/dev/null) || return 1
  [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null
}

# wal_listing prints name, type, size, mode, and mtime of every entry below the WAL directory.
# The directory base itself is left out, because its mtime changes with every temp name.
wal_listing() {
  (cd "$W" && find . -mindepth 1 ! -path ./base ! -path './base/*' -printf '%p %y %s %m %T@\n' | LC_ALL=C sort)
}

# json_lines_ok <file>: every line is one JSON object, and the file has a line.
json_lines_ok() {
  [ -s "$1" ] && ! grep -qv '^{.*}$' "$1"
}

# no_secret: the password appears in no output, no log of the stubs, and no argument.
no_secret() {
  ! grep -qF -- 'c\d9' "$CASE/out" "$CASE/err" "$CASE/stub/log"
}

# make_base <days-old> <start-segment>: a verified base backup that is <days-old> days old.
# It sets LAST_BASE to the directory name. The label names the start segment.
make_base() {
  local dir
  LAST_BASE=$(base_name_of_age "$1")
  dir=$W/base/$LAST_BASE
  mkdir -p "$dir/pg_wal"
  printf '16\n' >"$dir/PG_VERSION"
  printf 'START WAL LOCATION: 0/%X000028 (file %s)\nCHECKPOINT LOCATION: 0/%X000060\n' \
    "$2" "$(seg_name "$2")" "$2" >"$dir/backup_label"
  printf '{"PostgreSQL-Backup-Manifest-Version": 1}\n' >"$dir/backup_manifest"
  touch -d "$1 days ago" "$dir/PG_VERSION" "$dir/backup_label" "$dir/backup_manifest" \
    "$dir/pg_wal" "$dir"
}

# make_segments <first> <last>: WAL segment files in the archive. The last one also has a partial.
make_segments() {
  local n
  for ((n = $1; n <= $2; n++)); do
    printf 'segment %d\n' "$n" >"$W/$(seg_name "$n")"
  done
  printf 'partial %d\n' "$(($2 + 1))" >"$W/$(seg_name $(($2 + 1))).partial"
}

# seg_exists <number>: the segment file is in the archive.
seg_exists() { [ -f "$W/$(seg_name "$1")" ]; }

# base_exists <name>: the base backup directory is in the base directory.
base_exists() { [ -d "$W/base/$1" ]; }

# count_events <text>: the number of lines "EVENT <text>" in the log of the stubs.
count_events() { grep -c "^EVENT $1" "$CASE/stub/log" || true; }

# base_tree: name, type, size, mode, and mtime of every entry below the directory base.
base_tree() {
  (cd "$W/base" && find . -mindepth 1 -printf '%p %y %s %m %T@\n' | LC_ALL=C sort)
}

# wal_snapshot: the state that a refused run must not change.
wal_snapshot() {
  wal_listing
  base_tree
  find "$CASE/pgpass" -mindepth 1 | LC_ALL=C sort
}

# wal_expect_refused <name> <snapshot-before>: exit code 1, one ERROR line, no tool started,
# and no change of the archive or of the pgpass directory.
wal_expect_refused() {
  expect_eq "$1: exit code" "$RC" "1"
  expect_eq "$1: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_ok "$1: JSON lines" json_lines_ok "$CASE/out"
  expect_eq "$1: no tool started" "$(count_events '')" "0"
  expect_eq "$1: nothing changed" "$(wal_snapshot)" "$2"
  expect_ok "$1: no password in output" no_secret
}

# expect_message <label> <text>: the stdout of the last run holds the text in exactly one line.
expect_message() {
  expect_eq "$1: message" "$(grep -cF -- "$2" "$CASE/out" || true)" "1"
}

# flock_is_real: flock on this host refuses a second lock of the same file. The fake flock of some
# hosts grants every lock. A case that needs a busy lock calls this first.
flock_is_real() {
  local lock=$ROOT/flock-probe
  : >"$lock"
  (
    exec 8>>"$lock"
    flock -n 8 || exit 1
    (
      exec 7>>"$lock"
      flock -n 7
    ) && exit 1
    exit 0
  )
}

# base_dirs lists the names of the base backups that match the grammar.
base_dirs() {
  (cd "$W/base" && LC_ALL=C find . -mindepth 1 -maxdepth 1 -type d -name 'base_*' -printf '%f\n' \
    | LC_ALL=C sort | tr '\n' ' ')
}
