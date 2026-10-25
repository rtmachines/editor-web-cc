# Sourced by backup-test. It holds the counters, the expect functions, and the helpers for
# cases and sets.

cleanup() {
  local dir
  for dir in ${SHM_DIRS[@]+"${SHM_DIRS[@]}"}; do
    rm -rf -- "${dir:?}"
  done
  rm -rf -- "${ROOT:?}"
}

ok() { PASSED=$((PASSED + 1)); }
bad() { FAILED=$((FAILED + 1)); printf 'FAIL: %s\n' "$1" >&2; }

# skip <reason>: a skipped case prints SKIP. When the variable CI is set, a skip is a failure.
skip() {
  SKIPPED=$((SKIPPED + 1))
  printf 'SKIP: %s\n' "$1" >&2
  if [ -n "${CI:-}" ]; then bad "skipped under CI: $1"; fi
}

# expect_eq <name> <actual> <expected>
expect_eq() {
  if [ "$2" = "$3" ]; then ok; else bad "$1: got [$2], expected [$3]"; fi
}

# expect_ok <name> <command...>: the command succeeds.
expect_ok() {
  local name=$1
  shift
  if "$@"; then ok; else bad "$name"; fi
}

# expect_no <name> <command...>: the command fails.
expect_no() {
  local name=$1
  shift
  if "$@"; then bad "$name"; else ok; fi
}

# new_case <name>: a fresh backup directory with marker and lock file, a secret file, and a log.
new_case() {
  CASE=$ROOT/$1
  B=$CASE/backups
  mkdir -p "$B" "$CASE/pgpass" "$CASE/pgdata" "$CASE/stub"
  chmod 0755 "$B"
  : >"$B/.backup-marker"
  : >"$B/.lock"
  printf '16\n' >"$CASE/pgdata/PG_VERSION"
  printf '%s\n' "$SECRET" >"$CASE/secret"
  : >"$CASE/stub/log"
  TS=$("$REAL_DATE" -u +%Y%m%d_%H%M)
}

# run_script [VAR=value ...] -- <script> [args...]
# It writes out, err, and rc with the suffix $OUTTAG and sets RC.
run_script() {
  local envs=()
  while [ "$1" != "--" ]; do
    envs+=("$1")
    shift
  done
  shift
  local script=$1
  shift
  env PATH="$ROOT/stubs:$PATH" BACKUP_DIR="$B" BACKUP_PGPASS_DIR="$CASE/pgpass" \
    BACKUP_SECRET_FILE="$CASE/secret" PGDATA="${PGDATA_TEST:-$CASE/pgdata}" \
    STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" STUB_TS="$TS" \
    "${envs[@]}" bash "$BIN/$script" "$@" >"$CASE/out${OUTTAG:-}" 2>"$CASE/err${OUTTAG:-}"
  RC=$?
  echo "$RC" >"$CASE/rc${OUTTAG:-}"
}

error_count() { grep -c '"level":"ERROR"' "$1" || true; }

# listing prints name, type, size, mode, and mtime of every entry below the backup directory.
listing() { (cd "$B" && find . -mindepth 1 -printf '%p %y %s %m %T@\n' | LC_ALL=C sort); }

event_sequence() { grep '^EVENT ' "$CASE/stub/log" | tr '\n' ',' || true; }

# make_set <suffix> <days-old>: a complete set whose files are <days-old> days old.
make_set() {
  local suffix=$1 days=$2 ts
  ts=$("$REAL_DATE" -u -d "$days days ago" +%Y%m%d_%H%M)
  LAST_BASE=recipe_${ts}${suffix}
  printf 'dump\n' >"$B/$LAST_BASE.dump"
  printf 'globals\n' >"$B/globals_${ts}${suffix}.sql"
  printf 'public.t|1\n' >"$B/$LAST_BASE.counts"
  printf 'sum  %s\n' "$LAST_BASE.dump" >"$B/$LAST_BASE.dump.sha256"
  touch -d "$days days ago" "$B/$LAST_BASE.dump" "$B/globals_${ts}${suffix}.sql" \
    "$B/$LAST_BASE.counts" "$B/$LAST_BASE.dump.sha256"
}

set_exists() {
  local base=$1 ts=${1#recipe_}
  [ -f "$B/$base.dump" ] && [ -f "$B/globals_$ts.sql" ] && [ -f "$B/$base.counts" ] \
    && [ -f "$B/$base.dump.sha256" ]
}

set_absent() {
  local base=$1 ts=${1#recipe_}
  [ ! -e "$B/$base.dump" ] && [ ! -e "$B/globals_$ts.sql" ] && [ ! -e "$B/$base.counts" ] \
    && [ ! -e "$B/$base.dump.sha256" ]
}

no_temp_files() { [ -z "$(find "$B" -maxdepth 1 -name '.tmp-*' -print -quit)" ]; }

# make_temp_set <days-old>: the four temp files of a killed run, <days-old> days old.
# The temp manifest is among them. It sets LAST_TEMP to the four paths.
make_temp_set() {
  local ts file
  ts=$("$REAL_DATE" -u -d "$1 days ago" +%Y%m%d_%H%M)
  LAST_TEMP=("$B/.tmp-recipe_$ts.dump" "$B/.tmp-globals_$ts.sql" "$B/.tmp-recipe_$ts.counts" \
    "$B/.tmp-recipe_$ts.dump.sha256")
  for file in "${LAST_TEMP[@]}"; do
    printf 'partial' >"$file"
    touch -d "$1 days ago" "$file"
  done
}

# make_foreign_set <days-old>: four files of the name recipe_2026. The name is no name of the grammar.
# The files have the shape of a set and a valid manifest. So only the grammar keeps them out.
# It sets FOREIGN_FILES to the four paths and FOREIGN_MANIFEST_TEXT to the manifest text.
make_foreign_set() {
  local file
  printf 'dump\n' >"$B/recipe_2026.dump"
  printf 'globals\n' >"$B/globals_2026.sql"
  printf 'public.t|1\n' >"$B/recipe_2026.counts"
  (cd "$B" && sha256sum -- recipe_2026.dump globals_2026.sql recipe_2026.counts) \
    >"$B/recipe_2026.dump.sha256"
  FOREIGN_FILES=("$B/recipe_2026.dump" "$B/globals_2026.sql" "$B/recipe_2026.counts" \
    "$B/recipe_2026.dump.sha256")
  FOREIGN_MANIFEST_TEXT=$(cat "$B/recipe_2026.dump.sha256")
  for file in "${FOREIGN_FILES[@]}"; do
    touch -d "$1 days ago" "$file"
  done
}
