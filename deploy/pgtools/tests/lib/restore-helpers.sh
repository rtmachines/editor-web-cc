# Sourced by restore-test-test. It holds the helpers for the cases of restore-test.
# It needs lib/helpers.sh (counters, new_case, run_script, listing, event_sequence).

SEQ_TO_PRE_DATA="EVENT list,EVENT initdb,EVENT pg_ctl start,EVENT globals,EVENT alter-role recipe_app,EVENT alter-role recipe_backup,EVENT alter-role recipe_migrator,EVENT alter-role recipe_replicator,EVENT pg_restore pre-data,EVENT code pre,"
SEQ_TO_POST_DATA="${SEQ_TO_PRE_DATA}EVENT pg_restore data,EVENT pg_restore post-data,EVENT code post,"

# hash_of <text>: a SHA-256 in lower-case hex, for the lines of the synthetic restore-code.expected.
hash_of() { printf '%s' "$1" | sha256sum | cut -d' ' -f1; }

# write_expected <path>: one line of each kind, in byte order (ADR-0067 (i), ADR-0076 (a)).
write_expected() {
  {
    printf 'CHECK|public machine machine_code_format|recipe_migrator|%s\n' "$(hash_of check)"
    printf 'EVENT TRIGGER|- audit_objects_dropped|postgres|%s\n' "$(hash_of event)"
    printf 'FUNCTION|audit_guard audit_objects_dropped()|postgres|%s\n' "$(hash_of guard)"
    printf 'FUNCTION|public set_updated_at()|recipe_migrator|%s\n' "$(hash_of touch)"
    printf 'GENERATED|public unit depth|recipe_migrator|%s\n' "$(hash_of depth)"
    printf 'TRIGGER|public machine machine_set_updated_at|recipe_migrator|%s\n' "$(hash_of $'trigger\nO')"
  } >"$1"
}

# A key of pg_dumpall has 63 characters of [A-Za-z0-9] (dumputils.c, generate_restrict_key).
RESTRICT_KEY=k3Jq9XvT2mZpL8wRb5NdYc7HfG4sAe1uOi6tVn0BxMzQ2rPlKaW9yDjSh8CgE3f

# write_globals <path>: a globals file in the layout of pg_dumpall --globals-only 16
# (src/bin/pg_dump/pg_dumpall.c): header comment, \restrict, one comment line and one blank line,
# the SQL, \unrestrict, footer comment.
write_globals() {
  {
    printf -- '--\n-- PostgreSQL database cluster dump\n--\n\n'
    printf '\\restrict %s\n\n' "$RESTRICT_KEY"
    printf -- '-- Roles\n\n'
    printf 'SET default_transaction_read_only = off;\n\nCREATE ROLE recipe_app;\n\n'
    printf '\\unrestrict %s\n\n' "$RESTRICT_KEY"
    printf -- '--\n-- PostgreSQL database cluster dump complete\n--\n\n'
  } >"$1"
}

# write_listing <path>: the output of pg_restore --list for the synthetic dump.
write_listing() {
  cat >"$1" <<'EOF'
;
; Archive created at 2026-10-23 05:00:01 UTC
;     dbname: recipe
; Selected TOC Entries:
;
5; 2615 16386 SCHEMA - audit_guard recipe_migrator
219; 1255 16390 FUNCTION audit_guard audit_objects_dropped() postgres
220; 1255 16391 FUNCTION public set_updated_at() recipe_migrator
221; 1259 16392 TABLE public machine recipe_migrator
222; 2606 16393 CHECK CONSTRAINT public machine machine_code_format recipe_migrator
223; 0 16392 TABLE DATA public machine recipe_migrator
224; 2606 16394 FK CONSTRAINT public recipe recipe_machine_fk recipe_migrator
230; 2620 16400 TRIGGER public machine machine_set_updated_at recipe_migrator
231; 3466 16401 EVENT TRIGGER - audit_objects_dropped postgres
EOF
}

# write_set_files <suffix+timestamp>: the dump, the globals file, and the counts file of a set.
write_set_files() {
  printf 'DUMP-%s\n' "$1" >"$B/recipe_$1.dump"
  write_globals "$B/globals_$1.sql"
  printf 'public.machine|3\npublic.recipe|5\n' >"$B/recipe_$1.counts"
}

# write_manifest <suffix+timestamp>: the manifest of sha256sum, as backup-once writes it.
write_manifest() {
  (cd "$B" && sha256sum -- "recipe_$1.dump" "globals_$1.sql" "recipe_$1.counts") \
    >"$B/recipe_$1.dump.sha256"
}

# make_restore_set <suffix> <days-old>: a complete set with a manifest that verifies.
# It sets RESTORE_NAME to the timestamp and the suffix, and RESTORE_BASE to recipe_<RESTORE_NAME>.
make_restore_set() {
  local ts
  ts=$("$REAL_DATE" -u -d "$2 days ago" +%Y%m%d_%H%M)
  RESTORE_NAME=${ts}$1
  RESTORE_BASE=recipe_$RESTORE_NAME
  write_set_files "$RESTORE_NAME"
  write_manifest "$RESTORE_NAME"
  touch -d "$2 days ago" "$B/$RESTORE_BASE.dump" "$B/globals_$RESTORE_NAME.sql" \
    "$B/$RESTORE_BASE.counts" "$B/$RESTORE_BASE.dump.sha256"
}

# new_restore_case <name>: a backup directory with a complete set from now, the code files, and
# the answers of the stubs. The answers match each other, so a run without a switch passes.
new_restore_case() {
  new_case "$1"
  mkdir -p "$CASE/code" "$CASE/stub/server"
  printf -- '-- stub of restore-code.sql\n' >"$CASE/code/restore-code.sql"
  write_expected "$CASE/code/restore-code.expected"
  write_listing "$CASE/stub/listing"
  printf '%s\n' recipe_app recipe_backup recipe_migrator recipe_replicator >"$CASE/stub/login-roles"
  printf 'public.machine|3\npublic.recipe|5\n' >"$CASE/stub/restored.counts"
  make_restore_set "" 0
}

# restore_run [VAR=value ...]: run restore-test with the stubs and the code files of the case.
restore_run() {
  run_script "RESTORE_CODE_DIR=$CASE/code" "RESTORE_GLOBALS_DIR=$HERE/../lib" \
    PGDATABASE=recipe "$@" -- restore-test
}

# set_listing: the listing of the backup directory without the scratch directory.
set_listing() { listing | grep -v '^\./\.scratch' || true; }

scratch_runs() { find "$B/.scratch" -mindepth 1 -maxdepth 1 -name 'run.*' 2>/dev/null | wc -l; }

# event_count <regex>: the number of EVENT lines that match.
event_count() { grep -c "^EVENT $1" "$CASE/stub/log" || true; }

# restore_failed_checks <name> <listing before the run>: the checks of every failed run.
restore_failed_checks() {
  local name=$1
  expect_no "$name: exit code is not 0" test "$RC" -eq 0
  expect_eq "$name: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "$name: marker, lock, and set files are unchanged" "$(set_listing)" "$2"
  expect_eq "$name: no scratch run directory stays" "$(scratch_runs)" 0
  expect_eq "$name: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_eq "$name: every started server is stopped" \
    "$(event_count 'pg_ctl start')" "$(event_count 'pg_ctl stop')"
  expect_no "$name: no result line" grep -q '"message":"restore test passed"' "$CASE/out"
}

# failed_restore_case <name> [VAR=value ...]: a failing run changes no file of the backup disk.
failed_restore_case() {
  local name=$1 before
  shift
  new_restore_case "$name"
  before=$(set_listing)
  restore_run "$@"
  restore_failed_checks "$name" "$before"
}

# error_has <text>: the one ERROR line holds the text.
error_has() { grep '"level":"ERROR"' "$CASE/out" | grep -qF -- "$1"; }
