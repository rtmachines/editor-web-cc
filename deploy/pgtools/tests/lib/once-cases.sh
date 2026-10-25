# Sourced by backup-test. It holds the cases of backup-once, backup-now, and backup-entry.

# --- cases -------------------------------------------------------------------------------

case_missing_marker() {
  new_case missing-marker
  rm -- "$B/.backup-marker" "$B/.lock"
  run_script -- backup-once ""
  expect_eq "missing marker: exit code" "$RC" 1
  expect_eq "missing marker: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "missing marker: no file written" "$(find "$B" -mindepth 1 | wc -l)" 0
  expect_eq "missing marker: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "missing marker: no pgpass file" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

case_happy_path() {
  new_case happy
  run_script -- backup-once ""
  expect_eq "happy: exit code" "$RC" 0
  local f
  for f in "recipe_$TS.dump" "globals_$TS.sql" "recipe_$TS.counts" "recipe_$TS.dump.sha256"; do
    expect_ok "happy: $f exists" test -f "$B/$f"
    expect_eq "happy: mode of $f" "$(stat -c %a "$B/$f")" 600
  done
  expect_ok "happy: no temp file stays" no_temp_files
  expect_ok "happy: manifest verifies" bash -c "cd '$B' && sha256sum -c --strict 'recipe_$TS.dump.sha256'"
  expect_eq "happy: manifest covers three files" "$(wc -l <"$B/recipe_$TS.dump.sha256")" 3
  expect_eq "happy: manifest names" \
    "$(awk '{print $2}' "$B/recipe_$TS.dump.sha256" | tr '\n' ',')" \
    "recipe_$TS.dump,globals_$TS.sql,recipe_$TS.counts,"
  expect_eq "happy: counts file" "$(tr '\n' ',' <"$B/recipe_$TS.counts")" \
    "public.machine|3,public.recipe|5,"
  expect_eq "happy: pgpass line" "$(cat "$CASE/stub/pgpass.seen")" \
    'db:5432:recipe:recipe_backup:ab\:c\\d9'
  expect_eq "happy: pgpass mode" "$(cat "$CASE/stub/pgpass.mode")" 600
  expect_eq "happy: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_eq "happy: order of events" "$(event_sequence)" \
    "EVENT psql open,EVENT pg_dump start,EVENT pg_dump done,EVENT psql closed,EVENT pg_dumpall start,EVENT pg_dumpall done,"
  expect_no "happy: session open while pg_dump ran" grep -q SESSION_CLOSED_EARLY "$CASE/stub/log"
  expect_ok "happy: pg_dump arguments" grep -qxF \
    "ARGV pg_dump --format=custom --exclude-table-data=session --exclude-table-data=login_attempt --snapshot=$SNAPSHOT_ID --no-password --file=$B/.tmp-recipe_$TS.dump" \
    "$CASE/stub/log"
  expect_ok "happy: pg_dumpall arguments" grep -qxF \
    "ARGV pg_dumpall --globals-only --no-role-passwords -l recipe --no-password --file=$B/.tmp-globals_$TS.sql" \
    "$CASE/stub/log"
  expect_no "happy: secret in argv" grep -qF -- "$SECRET" "$CASE/stub/log"
  expect_no "happy: secret in output" grep -qF -- "$SECRET" "$CASE/out" "$CASE/err"
  expect_no "happy: non-JSON line on stdout" grep -qvE '^\{.*\}$' "$CASE/out"
  expect_eq "happy: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "happy: completion line" grep -q '"message":"backup completed"' "$CASE/out"
}

# Before the first migration the database has no table. backup-premigrate must still succeed
# and write a complete set with an empty counts file (ADR-0066 (i), ADR-0020).
case_no_tables() {
  new_case no-tables
  run_script STUB_NO_TABLES=1 -- backup-once _premigrate
  expect_eq "no tables: exit code" "$RC" 0
  expect_eq "no tables: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "no tables: set is complete" set_exists "recipe_${TS}_premigrate"
  expect_eq "no tables: counts file is empty" "$(wc -c <"$B/recipe_${TS}_premigrate.counts")" 0
  expect_eq "no tables: mode of counts file" "$(stat -c %a "$B/recipe_${TS}_premigrate.counts")" 600
  expect_ok "no tables: manifest verifies" \
    bash -c "cd '$B' && sha256sum -c --strict 'recipe_${TS}_premigrate.dump.sha256'"
  expect_eq "no tables: manifest covers three files" \
    "$(wc -l <"$B/recipe_${TS}_premigrate.dump.sha256")" 3
  expect_ok "no tables: no temp file stays" no_temp_files
}

case_suffixes() {
  new_case suffixes
  run_script -- backup-once _premigrate
  expect_eq "premigrate: exit code" "$RC" 0
  expect_ok "premigrate: manifest name" test -f "$B/recipe_${TS}_premigrate.dump.sha256"
  TS=20990101_0200
  run_script -- backup-now
  expect_eq "backup-now: exit code" "$RC" 0
  expect_ok "backup-now: manifest name" test -f "$B/recipe_${TS}_now.dump.sha256"
  expect_ok "backup-now: globals name" test -f "$B/globals_${TS}_now.sql"
  TS=20990102_0200
  run_script -- backup-once bogus
  expect_eq "bad suffix: exit code" "$RC" 1
  expect_eq "bad suffix: one ERROR line" "$(error_count "$CASE/out")" 1
}

case_retention_all_old() {
  new_case retention-old
  make_set "" 40; local d_old=$LAST_BASE
  make_set "" 35; local d_new=$LAST_BASE
  make_set _premigrate 60; local p_old=$LAST_BASE
  make_set _premigrate 50; local p_new=$LAST_BASE
  make_set _now 45; local n_old=$LAST_BASE
  make_set _now 31; local n_mid=$LAST_BASE
  run_script -- backup-now
  expect_eq "retention (all old): exit code" "$RC" 0
  expect_ok "retention (all old): new set exists" set_exists "recipe_${TS}_now"
  expect_ok "retention (all old): newest daily stays" set_exists "$d_new"
  expect_ok "retention (all old): newest premigrate stays" set_exists "$p_new"
  expect_ok "retention (all old): old daily goes" set_absent "$d_old"
  expect_ok "retention (all old): old premigrate goes" set_absent "$p_old"
  expect_ok "retention (all old): old now set goes" set_absent "$n_old"
  expect_ok "retention (all old): 31 day now set goes" set_absent "$n_mid"
  expect_ok "retention (all old): marker stays" test -f "$B/.backup-marker"
}

case_retention_mixed() {
  new_case retention-mixed
  make_set "" 40; local d_old=$LAST_BASE
  make_set "" 29; local d_young=$LAST_BASE
  make_set _premigrate 50; local p_old=$LAST_BASE
  make_set _premigrate 3; local p_young=$LAST_BASE
  make_set _now 31; local n_old=$LAST_BASE
  make_set _now 5; local n_young=$LAST_BASE
  mkdir -p "$B/.scratch"
  printf 'keep' >"$B/.scratch/work"
  printf 'keep' >"$B/README.txt"
  printf 'keep' >"$B/recipe_notes.txt"
  # A dump without a manifest is not a set. The retention leaves it alone.
  printf 'orphan' >"$B/recipe_20200101_0100.dump"
  touch -d "900 days ago" "$B/recipe_20200101_0100.dump" "$B/README.txt" "$B/recipe_notes.txt"
  run_script -- backup-once ""
  expect_eq "retention (mixed): exit code" "$RC" 0
  expect_ok "retention (mixed): 29 day daily stays" set_exists "$d_young"
  expect_ok "retention (mixed): young premigrate stays" set_exists "$p_young"
  expect_ok "retention (mixed): young now set stays" set_exists "$n_young"
  expect_ok "retention (mixed): 40 day daily goes" set_absent "$d_old"
  expect_ok "retention (mixed): 50 day premigrate goes" set_absent "$p_old"
  expect_ok "retention (mixed): 31 day now set goes" set_absent "$n_old"
  expect_ok "retention (mixed): scratch directory stays" test -f "$B/.scratch/work"
  expect_ok "retention (mixed): foreign file stays" test -f "$B/README.txt"
  expect_ok "retention (mixed): foreign recipe file stays" test -f "$B/recipe_notes.txt"
  expect_ok "retention (mixed): orphan dump stays" test -f "$B/recipe_20200101_0100.dump"
  expect_ok "retention (mixed): marker stays" test -f "$B/.backup-marker"
  expect_ok "retention (mixed): lock file stays" test -f "$B/.lock"
}

# The temp files of a killed run and a foreign name stay, also when they are older than the
# retention (ADR-0074 (b), (c)).
case_retention_keeps_temp_and_foreign() {
  new_case retention-temp
  make_temp_set 45
  make_foreign_set 45
  run_script -- backup-once ""
  expect_eq "retention (temp): exit code" "$RC" 0
  expect_ok "retention (temp): new set exists" set_exists "recipe_$TS"
  local file
  for file in "${LAST_TEMP[@]}"; do
    expect_eq "retention (temp): ${file##*/} is intact" "$(cat "$file" 2>/dev/null)" partial
  done
  for file in "${FOREIGN_FILES[@]}"; do
    expect_ok "retention (foreign): ${file##*/} stays" test -f "$file"
  done
  expect_eq "retention (foreign): recipe_2026.dump.sha256 is intact" \
    "$(cat "$B/recipe_2026.dump.sha256" 2>/dev/null)" "$FOREIGN_MANIFEST_TEXT"
}

# failed_run_case <name> <VAR=value>: a failing tool changes no existing file.
failed_run_case() {
  new_case "$1"
  make_set "" 40
  make_set _premigrate 60
  make_set _now 45
  mkdir -p "$B/.scratch"
  printf 'keep' >"$B/.scratch/work"
  local before
  before=$(listing)
  run_script "$2" -- backup-once ""
  expect_no "$1: exit code is not 0" test "$RC" -eq 0
  expect_eq "$1: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "$1: listing is unchanged" "$(listing)" "$before"
  expect_ok "$1: no temp file stays" no_temp_files
  expect_eq "$1: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_no "$1: no completion line" grep -q '"message":"backup completed"' "$CASE/out"
}

case_name_exists() {
  local kind file
  for kind in dump globals counts manifest; do
    new_case "exists-$kind"
    make_set "" 40
    case $kind in
      dump) file=recipe_$TS.dump ;;
      globals) file=globals_$TS.sql ;;
      counts) file=recipe_$TS.counts ;;
      manifest) file=recipe_$TS.dump.sha256 ;;
    esac
    printf 'foreign' >"$B/$file"
    local before
    before=$(listing)
    run_script -- backup-once ""
    expect_eq "exists ($kind): exit code" "$RC" 1
    expect_eq "exists ($kind): one ERROR line" "$(error_count "$CASE/out")" 1
    expect_eq "exists ($kind): listing is unchanged" "$(listing)" "$before"
    expect_eq "exists ($kind): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

# A temp name of the new set exists. The run stops before any tool runs (ADR-0074 (b)).
case_temp_name_exists() {
  local kind file before
  for kind in dump globals counts manifest; do
    new_case "temp-exists-$kind"
    make_set "" 40
    case $kind in
      dump) file=.tmp-recipe_$TS.dump ;;
      globals) file=.tmp-globals_$TS.sql ;;
      counts) file=.tmp-recipe_$TS.counts ;;
      manifest) file=.tmp-recipe_$TS.dump.sha256 ;;
    esac
    printf 'foreign' >"$B/$file"
    before=$(listing)
    run_script -- backup-once ""
    expect_eq "temp exists ($kind): exit code" "$RC" 1
    expect_eq "temp exists ($kind): one ERROR line" "$(error_count "$CASE/out")" 1
    expect_eq "temp exists ($kind): listing is unchanged" "$(listing)" "$before"
    expect_eq "temp exists ($kind): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

# A name appears during the run. The rename never replaces it.
case_rename_never_replaces() {
  local kind foreign
  for kind in dump counts; do
    new_case "rename-$kind"
    run_script "STUB_CREATE_FINAL=$kind" -- backup-once ""
    foreign=$B/recipe_$TS.$kind
    expect_eq "rename ($kind): exit code" "$RC" 1
    expect_eq "rename ($kind): one ERROR line" "$(error_count "$CASE/out")" 1
    expect_eq "rename ($kind): foreign file is intact" "$(cat "$foreign")" FOREIGN
    expect_no "rename ($kind): no manifest" test -e "$B/recipe_$TS.dump.sha256"
    expect_ok "rename ($kind): no temp file stays" no_temp_files
    expect_eq "rename ($kind): only the foreign file is new" \
      "$(find "$B" -mindepth 1 -type f | LC_ALL=C sort | tr '\n' ',')" \
      "$B/.backup-marker,$B/.lock,$foreign,"
  done
}

case_parallel_runs() {
  new_case parallel
  local rc_a rc_b
  OUTTAG=a TS=20990201_0200 run_script STUB_PG_DUMP_SLEEP=3 -- backup-once "" &
  sleep 1
  OUTTAG=b TS=20990201_0201 run_script STUB_PG_DUMP_SLEEP=3 -- backup-once "" &
  wait
  rc_a=$(cat "$CASE/rca")
  rc_b=$(cat "$CASE/rcb")
  expect_eq "parallel: one run wins, one run stops" "$((rc_a == 0 ? 1 : 0)),$((rc_b == 0 ? 1 : 0))" "1,0"
  expect_eq "parallel: one manifest" "$(find "$B" -maxdepth 1 -name 'recipe_*.dump.sha256' | wc -l)" 1
  expect_eq "parallel: one ERROR line in total" \
    "$(( $(error_count "$CASE/outa") + $(error_count "$CASE/outb") ))" 1
  expect_ok "parallel: no temp file stays" no_temp_files
}

case_secret_file_rules() {
  local variant
  for variant in carriage-return two-lines empty newline-only missing; do
    new_case "secret-$variant"
    case $variant in
      carriage-return) printf 'abc\r\n' >"$CASE/secret" ;;
      two-lines) printf 'a\nb\n' >"$CASE/secret" ;;
      empty) : >"$CASE/secret" ;;
      newline-only) printf '\n' >"$CASE/secret" ;;
      missing) rm -- "$CASE/secret" ;;
    esac
    local before
    before=$(listing)
    run_script -- backup-once ""
    expect_eq "secret ($variant): exit code" "$RC" 1
    expect_eq "secret ($variant): one ERROR line" "$(error_count "$CASE/out")" 1
    expect_eq "secret ($variant): listing is unchanged" "$(listing)" "$before"
    expect_eq "secret ($variant): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
  new_case secret-no-newline
  printf 'abc' >"$CASE/secret"
  run_script -- backup-once ""
  expect_eq "secret (no final newline): exit code" "$RC" 0
  expect_eq "secret (no final newline): pgpass line" "$(cat "$CASE/stub/pgpass.seen")" \
    'db:5432:recipe:recipe_backup:abc'
}

case_psql_not_usable() {
  failed_run_case psql-fails STUB_PSQL_FAIL=1
}

# session_end_case <name> <VAR=value>: the snapshot session ends and never answers COUNTS_DONE.
# wait_for must stop the run before pg_dump starts (CR-O2-3). A wait_for that returns 0 at the
# end of the input lets pg_dump run, so the last two checks fail.
session_end_case() {
  failed_run_case "$1" "$2"
  expect_ok "$1: the ERROR line names the end of the session" \
    grep -q 'ended before it answered' "$CASE/out"
  expect_no "$1: pg_dump did not start" grep -q '^EVENT pg_dump' "$CASE/stub/log"
}

case_session_ends() {
  session_end_case session-dies-at-query STUB_PSQL_DIE_GEXEC=3
  session_end_case session-dies-at-end STUB_PSQL_DIE_END=1
}

# The session answers every line and then ends with status 1. backup-once must read that status
# after pg_dump and stop before pg_dumpall (CR-O2-14).
case_session_exit_status() {
  failed_run_case session-exit-status STUB_PSQL_EXIT_END=1
  expect_eq "session-exit-status: exit code" "$RC" 1
  expect_ok "session-exit-status: the ERROR line names the failed session" \
    grep -q 'the snapshot session failed' "$CASE/out"
  expect_ok "session-exit-status: pg_dump ran" grep -q '^EVENT pg_dump start' "$CASE/stub/log"
  expect_no "session-exit-status: pg_dumpall did not start" \
    grep -q '^EVENT pg_dumpall' "$CASE/stub/log"
}

# The service backup refuses to start without the marker file (ADR-0066 (b)).
case_entry() {
  new_case entry-no-marker
  printf '# crontab\n' >"$CASE/crontab"
  rm -- "$B/.backup-marker"
  run_script "BACKUP_CRONTAB=$CASE/crontab" -- backup-entry
  expect_eq "entry (no marker): exit code" "$RC" 1
  expect_eq "entry (no marker): one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "entry (no marker): supercronic did not run" "$(wc -c <"$CASE/stub/log")" 0
  new_case entry-marker
  printf '# crontab\n' >"$CASE/crontab"
  run_script "BACKUP_CRONTAB=$CASE/crontab" -- backup-entry
  expect_eq "entry: exit code" "$RC" 0
  expect_eq "entry: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_eq "entry: supercronic ran once" "$(grep -c '^EVENT supercronic' "$CASE/stub/log")" 1
  expect_ok "entry: supercronic arguments" grep -qxF \
    "ARGV supercronic -json -passthrough-logs $CASE/crontab" "$CASE/stub/log"
}

# A failed delete of a temp file must not keep the pgpass file (CR-O2-4).
case_pgpass_removed_first() {
  new_case pgpass-first
  run_script "PATH=$ROOT/rmstub:$ROOT/stubs:$PATH" STUB_PG_DUMP_FAIL=1 -- backup-once ""
  expect_no "pgpass first: exit code is not 0" test "$RC" -eq 0
  expect_eq "pgpass first: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# The run calls sync three times (CR-O2-6, VW2-8). The old manifest shows the retention has not run.
#   1. after pg_dumpall and before the first publish: the dump has its temp name only.
#   2. after the write of the temp manifest and before its publish.
#   3. after the publish of the manifest and before the retention.
case_sync_after_manifest() {
  new_case sync-order
  make_set "" 40
  local old_manifest=$B/$LAST_BASE.dump.sha256
  run_script "STUB_OLD_MANIFEST=$old_manifest" -- backup-once ""
  expect_eq "sync: exit code" "$RC" 0
  expect_no "sync: the retention deleted the old set" test -e "$old_manifest"
  expect_eq "sync: three calls" "$(grep -c '^SYNC ' "$CASE/stub/log")" 3
  expect_eq "sync: first call is before the first publish" \
    "$(grep '^SYNC ' "$CASE/stub/log" | sed -n 1p)" \
    "SYNC tmp_dump=yes dump=no tmp_manifest=no manifest=no old=yes"
  expect_eq "sync: second call is after the temp manifest and before its publish" \
    "$(grep '^SYNC ' "$CASE/stub/log" | sed -n 2p)" \
    "SYNC tmp_dump=no dump=yes tmp_manifest=yes manifest=no old=yes"
  expect_eq "sync: third call is after the manifest and before the retention" \
    "$(grep '^SYNC ' "$CASE/stub/log" | sed -n 3p)" \
    "SYNC tmp_dump=no dump=yes tmp_manifest=no manifest=yes old=yes"
}

case_static_rules() {
  local script name
  for script in "$BIN"/backup-*; do
    name=$(basename "$script")
    expect_ok "static: $name is executable" test -x "$script"
    expect_ok "static: bash -n $name" bash -n "$script"
    expect_no "static: xtrace in $name" grep -Eq 'set[[:space:]]+-[A-Za-z]*x|xtrace' "$script"
    expect_no "static: carriage return in $name" grep -q $'\r' "$script"
    expect_no "static: password in argv or environment in $name" \
      grep -Eq 'PGPASSWORD|--password[^-]|[[:space:]]-W([[:space:]]|$)' "$script"
    expect_no "static: mv -n in $name" grep -Eq 'mv[[:space:]]+(-[A-Za-z]*n|--no-clobber)' "$script"
    expect_eq "static: set and umask of $name" \
      "$(grep -vE '^[[:space:]]*(#|$)' "$script" | sed -n '1p;2p' | tr '\n' '|')" \
      'set -euo pipefail|umask 077|'
  done
}
