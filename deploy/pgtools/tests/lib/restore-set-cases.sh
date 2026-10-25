# Sourced by restore-test-test. It holds the cases of the gates and of the choice of the set:
# static rules, usage, marker, lock, manifest, and the grammar of a set name (ADR-0074).

# unsafe_rm_lines <script>: the lines that call rm without the end of options.
unsafe_rm_lines() {
  grep -E '(^|[[:space:];&(])rm[[:space:]]' "$1" | grep -vE '^[[:space:]]*#' \
    | grep -vE 'rm[[:space:]]+(-[A-Za-z]+[[:space:]]+)*--[[:space:]]'
}

case_restore_static_rules() {
  local script=$BIN/restore-test
  expect_ok "static: restore-test exists" test -f "$script"
  expect_ok "static: restore-test is executable" test -x "$script"
  expect_ok "static: bash -n restore-test" bash -n "$script"
  expect_no "static: xtrace" grep -Eq 'set[[:space:]]+-[A-Za-z]*x|xtrace' "$script"
  expect_no "static: carriage return" grep -q $'\r' "$script"
  expect_no "static: password in argv or environment" \
    grep -Eq 'PGPASSWORD=|--password[^-]|[[:space:]]-W([[:space:]]|$)' "$script"
  expect_no "static: password in a shell variable" \
    grep -Eq '(^|[^A-Za-z_])(pw|passwd|password|secret)=' "$script"
  expect_no "static: rm -rf" grep -Eq 'rm[[:space:]]+-[A-Za-z]*r[A-Za-z]*f|rm[[:space:]]+-[A-Za-z]*f[A-Za-z]*r' "$script"
  expect_no "static: rm without the end of options" unsafe_rm_lines "$script"
  expect_no "static: eval" grep -Eq '(^|[[:space:];&(])eval[[:space:]]' "$script"
  expect_ok "static: the port names its row of 7.11" grep -Eq '^SCRATCH_PORT=5433 #.*scratch\.port' "$script"
  expect_ok "static: the start timeout names its row of 7.11" \
    grep -Eq '^START_TIMEOUT_SECONDS=60 #.*scratch\.restore_start_timeout' "$script"
  expect_ok "static: the stop timeout names its row of 7.11" \
    grep -Eq '^STOP_TIMEOUT_SECONDS=60 #.*scratch\.stop_timeout' "$script"
  expect_ok "static: the detail length names its row of 7.11" \
    grep -Eq '^DETAIL_LENGTH=160 #.*restore_test\.detail_length' "$script"
  expect_no "static: a constant is called a script constant" grep -q 'script constant' "$script"
  expect_eq "static: set and umask" \
    "$(grep -vE '^[[:space:]]*(#|$)' "$script" | sed -n '1p;2p' | tr '\n' '|')" \
    'set -euo pipefail|umask 077|'
}

case_restore_usage() {
  new_restore_case usage
  run_script "RESTORE_CODE_DIR=$CASE/code" -- restore-test extra
  expect_eq "usage: exit code" "$RC" 1
  expect_eq "usage: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "usage: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

case_restore_missing_marker() {
  new_restore_case missing-marker
  rm -- "$B/.backup-marker" "$B/.lock"
  rm -- "$B"/recipe_* "$B"/globals_*
  restore_run
  expect_eq "missing marker: exit code" "$RC" 1
  expect_eq "missing marker: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "missing marker: the ERROR line names the marker" error_has "marker"
  expect_eq "missing marker: no file written" "$(find "$B" -mindepth 1 | wc -l)" 0
  expect_eq "missing marker: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "missing marker: no pgpass file" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# Without a set the test has nothing to restore. It stops before it starts a tool.
case_restore_no_set() {
  new_restore_case no-set
  rm -- "$B"/recipe_* "$B"/globals_*
  local before
  before=$(set_listing)
  restore_run
  restore_failed_checks "no set" "$before"
  expect_ok "no set: the ERROR line says so" error_has "no complete backup set"
  expect_eq "no set: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# The manifest is the first check. A changed dump stops the test before any tool starts.
case_restore_manifest_mismatch() {
  local file
  for file in dump globals counts; do
    new_restore_case "manifest-mismatch-$file"
    case $file in
      dump) printf 'changed' >>"$B/$RESTORE_BASE.dump" ;;
      globals) printf 'changed' >>"$B/globals_$RESTORE_NAME.sql" ;;
      counts) printf 'changed' >>"$B/$RESTORE_BASE.counts" ;;
    esac
    local before
    before=$(set_listing)
    restore_run
    restore_failed_checks "manifest mismatch ($file)" "$before"
    expect_ok "manifest mismatch ($file): the ERROR line names the manifest" error_has "manifest"
    expect_eq "manifest mismatch ($file): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

# The check of the manifest must cover the three files of the set and nothing else.
case_restore_manifest_files() {
  local variant
  for variant in no-dump extra-line other-name; do
    new_restore_case "manifest-files-$variant"
    case $variant in
      no-dump) sed -i '1d' "$B/$RESTORE_BASE.dump.sha256" ;;
      extra-line) printf '%s  other.txt\n' "$(hash_of other)" >>"$B/$RESTORE_BASE.dump.sha256" ;;
      other-name)
        sed -i "s/recipe_$RESTORE_NAME.counts/..\/counts/" "$B/$RESTORE_BASE.dump.sha256"
        ;;
    esac
    local before
    before=$(set_listing)
    restore_run
    restore_failed_checks "manifest files ($variant)" "$before"
    expect_eq "manifest files ($variant): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

# Only a complete set of the grammar of ADR-0074 (c) counts. A temp manifest, a name outside the
# grammar, and a manifest that is no regular file never win. All three are newer than the set.
case_restore_set_selection() {
  new_restore_case set-selection
  rm -- "$B"/recipe_* "$B"/globals_*
  make_restore_set "" 2
  local good=$RESTORE_BASE
  printf 'partial' >"$B/.tmp-recipe_$TS.dump.sha256"
  printf 'partial' >"$B/.tmp-recipe_$TS.dump"
  printf 'DUMP-x\n' >"$B/recipe_${TS}x.dump"
  printf 'CREATE ROLE evil;\n' >"$B/globals_${TS}x.sql"
  printf 'public.t|1\n' >"$B/recipe_${TS}x.counts"
  (cd "$B" && sha256sum -- "recipe_${TS}x.dump" "globals_${TS}x.sql" "recipe_${TS}x.counts") \
    >"$B/recipe_${TS}x.dump.sha256"
  mkdir "$B/recipe_${TS}_now.dump.sha256"
  restore_run
  expect_eq "selection: exit code" "$RC" 0
  expect_eq "selection: the complete set is restored" \
    "$(grep -c "^ARGV pg_restore --exit-on-error.*$good.dump\$" "$CASE/stub/log")" 3
  expect_eq "selection: no other dump is read" \
    "$(grep -c '^ARGV pg_restore' "$CASE/stub/log")" 4
  expect_ok "selection: the temp manifest stays" test -f "$B/.tmp-recipe_$TS.dump.sha256"
  expect_ok "selection: the foreign manifest stays" test -f "$B/recipe_${TS}x.dump.sha256"
}

# The newest set wins, also when it has a suffix. A _premigrate set is a set.
case_restore_newest_set() {
  new_restore_case newest-set
  make_restore_set "" 3
  make_restore_set _premigrate 1
  local newest=$RESTORE_BASE
  make_restore_set "" 0
  local current=$RESTORE_BASE
  restore_run
  expect_eq "newest: exit code" "$RC" 0
  expect_eq "newest: the set from now is restored" \
    "$(grep -c "^ARGV pg_restore .*$current.dump\$" "$CASE/stub/log")" 4
  expect_eq "newest: the older set is not read" \
    "$(grep -c "$newest" "$CASE/stub/log")" 0
  new_restore_case newest-premigrate
  rm -- "$B"/recipe_* "$B"/globals_*
  make_restore_set "" 3
  make_restore_set _premigrate 1
  newest=$RESTORE_BASE
  restore_run
  expect_eq "newest (premigrate): exit code" "$RC" 0
  expect_eq "newest (premigrate): the newer set is restored" \
    "$(grep -c "^ARGV pg_restore .*$newest.dump\$" "$CASE/stub/log")" 4
}

# Two runs must not share the scratch directory. The second run stops at the lock.
# The fake flock of a host without flock does not lock, so this case fails on such a host.
case_restore_parallel_runs() {
  new_restore_case parallel
  local rc_a rc_b
  OUTTAG=a restore_run STUB_LIST_SLEEP=3 &
  sleep 1
  OUTTAG=b restore_run STUB_LIST_SLEEP=3 &
  wait
  rc_a=$(cat "$CASE/rca")
  rc_b=$(cat "$CASE/rcb")
  expect_eq "parallel: one run wins, one run stops" "$((rc_a == 0 ? 1 : 0)),$((rc_b == 0 ? 1 : 0))" "1,0"
  expect_eq "parallel: one ERROR line in total" \
    "$(( $(error_count "$CASE/outa") + $(error_count "$CASE/outb") ))" 1
  expect_eq "parallel: one cluster started" "$(event_count 'initdb')" 1
  expect_eq "parallel: no scratch run directory stays" "$(scratch_runs)" 0
}

case_restore_code_files() {
  local file
  for file in restore-code.sql restore-code.expected; do
    new_restore_case "code-file-$file"
    rm -- "$CASE/code/$file"
    local before
    before=$(set_listing)
    restore_run
    restore_failed_checks "code file ($file)" "$before"
    expect_ok "code file ($file): the ERROR line names the file" error_has "$file"
    expect_eq "code file ($file): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}
