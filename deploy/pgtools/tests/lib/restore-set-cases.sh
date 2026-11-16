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

# The shared steps live in /etc/pgtools/restore-steps.sh (change restore-volume-switch, design D3).
# The library defines each function and restore-test defines none of them.
case_restore_steps_library() {
  local library=${PGTOOLS_ETC:-/etc/pgtools}/restore-steps.sh name
  local names=(log fail clean tool_detail select_set check_manifest check_counts check_globals
    check_names restore_section sorted_kinds compare_code fail_first_table compare_counts check_chain)
  expect_ok "steps library: restore-steps.sh exists" test -f "$library"
  expect_ok "steps library: bash -n restore-steps.sh" bash -n "$library"
  expect_no "steps library: carriage return" grep -q $'\r' "$library"
  for name in "${names[@]}"; do
    expect_ok "steps library: the library defines $name" \
      bash -c 'source "$1" && declare -F -- "$2" >/dev/null' _ "$library" "$name"
    expect_no "steps library: restore-test does not define $name" \
      grep -Eq "^$name\(\) \{" "$BIN/restore-test"
  done
  expect_ok "steps library: restore-test sources the library" \
    grep -Eq '^source .*restore-steps\.sh' "$BIN/restore-test"
  case_restore_manifest_argument
}

# A valid disk manifest must not replace the required checked copy.
case_restore_manifest_argument() {
  new_restore_case manifest-argument
  env PATH="$ROOT/stubs:$PATH" STUB_LOG="$CASE/stub/log" STUB_DIR="$CASE/stub" \
    bash -euo pipefail -c '
      source "$1" || { echo "cannot load the restore steps" >&2; exit 1; }
      JOB=restore-test
      BACKUP_DIR=$2
      work_dir=$3
      error_logged=0
      set_paths "$4"
      check_manifest
      psql --no-psqlrc --command "SELECT 1"
    ' _ "$HERE/../lib/restore-steps.sh" "$B" "$CASE/pgpass" "$RESTORE_NAME" \
    >"$CASE/out" 2>"$CASE/err"
  RC=$?
  expect_eq "manifest argument: exit code" "$RC" 1
  expect_eq "manifest argument: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_eq "manifest argument: one output line" "$(wc -l <"$CASE/out")" 1
  expect_ok "manifest argument: fixed message" error_has 'the checked manifest argument is missing'
  expect_eq "manifest argument: no stderr" "$(wc -c <"$CASE/err")" 0
  expect_eq "manifest argument: no SQL step ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_no "manifest argument: no globals copy" test -e "$CASE/pgpass/globals_$RESTORE_NAME.sql"
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

# A bad tag of the newest set must never select a valid older set.
case_restore_bad_tag() {
  local variant tag before newest message
  for variant in missing link malformed unknown-key different-tag changed-dump; do
    new_restore_case "tag-$variant"
    newest=$RESTORE_NAME
    make_restore_set "" 2
    RESTORE_NAME=$newest
    RESTORE_BASE=recipe_$newest
    tag=$B/$RESTORE_BASE.dump.sha256.hmac
    case $variant in
      missing) rm -- "$tag" ;;
      link) mv -- "$tag" "$CASE/tag"; ln -s "$CASE/tag" "$tag" ;;
      malformed) printf 'bad tag line\n' >"$tag" ;;
      unknown-key)
        awk '{$2 = ($2 == "0000000000000000" ? "1111111111111111" : "0000000000000000"); print}' \
          "$tag" >"$CASE/tag"
        cp -- "$CASE/tag" "$tag"
        ;;
      different-tag)
        awk '{z = sprintf("%064d", 0); $3 = ($3 == z ? "1" substr(z, 2) : z); print}' \
          "$tag" >"$CASE/tag"
        cp -- "$CASE/tag" "$tag"
        ;;
      changed-dump)
        cp -- "$tag" "$CASE/tag"
        printf 'changed dump\n' >>"$B/$RESTORE_BASE.dump"
        write_manifest "$RESTORE_NAME"
        cp -- "$CASE/tag" "$tag"
        ;;
    esac
    before=$(set_listing)
    restore_run
    restore_failed_checks "tag $variant" "$before"
    case $variant in
      missing|link) message='the newest set has no tag file' ;;
      malformed) message='the tag file of the set has a bad line' ;;
      unknown-key) message='the tag of the set comes from an unknown key' ;;
      different-tag|changed-dump) message='the tag of the set does not match the manifest' ;;
    esac
    restore_manifest_error "tag $variant" "$message"
    expect_eq "tag $variant: no PostgreSQL tool or older set is read" "$(wc -c <"$CASE/stub/log")" 0
    expect_eq "tag $variant: no read fault" "$(wc -c <"$CASE/err")" 0
  done
  restore_key_refusal_cases
  restore_oversized_copy_cases
}

# Observe the private copies before cleanup without changing the copy or the tag check.
restore_copy_size_stub() {
  mkdir -- "$CASE/copy-stubs"
  cat >"$CASE/copy-stubs/rm" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
umask 077
for path in "$@"; do
  if [[ $path == "$BACKUP_PGPASS_DIR"/run.* ]] && [ -d "$path" ]; then
    for file in "$path"/*.dump.sha256 "$path"/*.dump.sha256.hmac; do
      [ -f "$file" ] || continue
      printf '%s %s\n' "${file##*/}" "$(wc -c <"$file")" >>"$STUB_DIR/copy-sizes"
    done
  fi
done
exec "$REAL_RM" "$@"
EOF
  chmod 0755 "$CASE/copy-stubs/rm"
}

# A later size rejection cannot compensate for an unbounded copy into the private directory.
restore_oversized_copy_cases() {
  local kind constant limit file message before
  for kind in manifest tag; do
    new_restore_case "oversized-copy-$kind"
    file=$RESTORE_BASE.dump.sha256
    constant=MAX_FILE_BYTES
    message='the manifest exceeds the size limit'
    if [ "$kind" = tag ]; then
      file=$file.hmac
      constant=MAX_TAG_BYTES
      message='the tag file of the set exceeds the size limit'
    fi
    limit=$(sed -n "s/^use constant $constant => \([0-9][0-9]*\);$/\1/p" \
      "$HERE/../lib/manifest-tag.pl")
    expect_ok "$kind copy: the library declares the limit" test "${limit:-0}" -gt 0
    head -c "$((limit * 2 + 1))" /dev/zero >"$B/$file"
    restore_copy_size_stub
    before=$(set_listing)
    restore_run "PATH=$CASE/copy-stubs:$ROOT/stubs:$PATH"
    restore_failed_checks "$kind copy size" "$before"
    restore_manifest_error "$kind copy size" "$message"
    expect_eq "$kind copy: at most limit plus one bytes reach the private directory" \
      "$(grep -F -- "$file " "$CASE/stub/copy-sizes")" "$file $((limit + 1))"
    expect_eq "$kind copy: no PostgreSQL tool ran" "$(wc -c <"$CASE/stub/log")" 0
    expect_eq "$kind copy: no read fault" "$(wc -c <"$CASE/err")" 0
  done
}

# Check key faults before any PostgreSQL tool runs. The second key line is deliberately invalid.
restore_key_refusal_cases() {
  local variant key before message line
  for variant in missing location bad-line-2; do
    new_restore_case "key-$variant"
    key=$CASE/key
    line=''
    case $variant in
      missing) message='the manifest key file is missing' ;;
      location)
        cp -- "$RESTORE_TAG_KEY_FILE" "$key"
        message='the manifest key file lies on the backup disk'
        ;;
      bad-line-2)
        cp -- "$RESTORE_TAG_KEY_FILE" "$key"
        printf '\ninvalid\n' >>"$key"
        message='the manifest key file has a bad line'
        line=2
        ;;
    esac
    before=$(set_listing)
    restore_run "BACKUP_MANIFEST_KEY_FILE=$key"
    restore_failed_checks "key $variant" "$before"
    restore_manifest_error "key $variant" "$message" "$line"
    expect_eq "key $variant: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
    expect_eq "key $variant: no read fault" "$(wc -c <"$CASE/err")" 0
  done
}

# The tag gate precedes each read of the other three files of the set.
case_restore_bad_tag_before_set_reads() {
  new_restore_case tag-before-set-reads
  printf 'bad tag line\n' >"$B/$RESTORE_BASE.dump.sha256.hmac"
  local file before
  for file in "$B/$RESTORE_BASE.dump" "$B/globals_$RESTORE_NAME.sql" "$B/$RESTORE_BASE.counts"; do
    rm -- "$file"
    mkdir -- "$file"
  done
  before=$(set_listing)
  restore_run
  restore_failed_checks "tag before set reads" "$before"
  restore_manifest_error "tag before set reads" 'the tag file of the set has a bad line'
  expect_eq "tag before set reads: no read fault" "$(wc -c <"$CASE/err")" 0
  expect_eq "tag before set reads: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# The wrapper invokes Perl. This PATH stub changes only the disk manifest after a successful check.
case_restore_manifest_swap_after_tag() {
  new_restore_case manifest-swap-after-tag
  mkdir -- "$CASE/tag-stubs"
  sed -E "1s/^[0-9a-f]{64}/$(printf '%064d' 0)/" "$B/$RESTORE_BASE.dump.sha256" \
    >"$CASE/stub/manifest.swapped"
  cat >"$CASE/tag-stubs/perl" <<'EOF'
#!/usr/bin/env bash
umask 077
"$REAL_PERL" "$@" || exit $?
if [ "${1##*/}" = manifest-tag.pl ] && [ "${2:-}" = check ]; then
  printf '%s\n' "$4" "$5" >"$STUB_DIR/tag.paths"
  stat -c %a -- "$4" "$5" >"$STUB_DIR/tag.modes"
  cp -- "$STUB_DIR/manifest.swapped" "$DISK_MANIFEST"
fi
EOF
  chmod 0755 "$CASE/tag-stubs/perl"
  restore_run "REAL_PERL=$(command -v perl)" "PATH=$CASE/tag-stubs:$ROOT/stubs:$PATH" \
    "DISK_MANIFEST=$B/$RESTORE_BASE.dump.sha256"
  if [ "$RC" -ne 0 ]; then cat -- "$CASE/out" >&2; fi
  expect_eq "manifest swap: exit code" "$RC" 0
  expect_eq "manifest swap: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "manifest swap: the disk manifest changed" \
    cmp -s -- "$CASE/stub/manifest.swapped" "$B/$RESTORE_BASE.dump.sha256"
  expect_eq "manifest swap: the tag check reads two private copies" \
    "$(grep -cF "$CASE/pgpass/run." "$CASE/stub/tag.paths")" 2
  expect_eq "manifest swap: private copies have mode 0600" \
    "$(cat "$CASE/stub/tag.modes")" $'600\n600'
  expect_eq "manifest swap: the restore reaches post-data" "$(event_count 'pg_restore post-data')" 1
  restore_result_checks "manifest swap result" passed
  expect_eq "manifest swap: no private copy stays" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# A changed dump stops the test before any PostgreSQL tool starts.
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
    sign_restore_manifest "$RESTORE_NAME"
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
