# Sourced by restore-volume-test. It holds the cases of steps 3 and 4 of ADR-0103 (i): the gates of
# restore-test and the initdb call. A refused gate stops the run before initdb.
# It needs lib/helpers.sh, lib/restore-helpers.sh (RESTRICT_KEY and the set helpers), and
# lib/restore-volume-cases.sh. The globals cases build their file as lib/restore-globals-cases.sh does.

INITDB_NAMES='PG_VERSION|base|global|pg_hba.conf|pg_ident.conf|postgresql.auto.conf|postgresql.conf'

# volume_gate_checks <name> <backup listing>: the checks of a run that a gate of step 3 refused.
# The run created its work directory in the tmpfs, so the pgpass directory is not part of the checks.
volume_gate_checks() {
  local name=$1
  expect_no "$name: exit code is not 0" test "$RC" -eq 0
  expect_eq "$name: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$name: the output is JSON of the job restore-volume" volume_json_ok "$CASE/out"
  expect_eq "$name: initdb did not run" "$(event_count initdb)" 0
  expect_eq "$name: no server tool ran" "$(event_count 'pg_ctl')" 0
  expect_eq "$name: the backup directory is unchanged" "$(set_listing)" "$2"
  expect_eq "$name: the target directory is empty" "$(target_listing)" ""
  expect_eq "$name: no scratch run directory is new" "$(scratch_runs)" 0
}

# volume_work_dir: the path of the work directory of the run in the tmpfs of the secret files. The trap
# of the script deletes the directory, so the path comes from the --log argument of pg_ctl start.
# volume_work_copy: the copy that the stub of pg_ctl made when the server started.
volume_work_dir() { sed -n 's|^ARGV pg_ctl .* --log=\(.*\)/server\.log .* start$|\1|p' "$CASE/stub/log" | sed -n '1p'; }
volume_work_copy() { printf '%s\n' "$CASE/stub/work"; }

# volume_gate_case <name> <text in the ERROR line> [VAR=value ...]: the case is ready. The run is refused.
volume_gate_case() {
  local name=$1 text=$2 before
  shift 2
  before=$(set_listing)
  volume_run "$@"
  volume_gate_checks "$name" "$before"
  expect_ok "$name: the ERROR line holds [$text]" error_has "$text"
}

# The functions that the two scripts share live in the library (design D3).
case_volume_steps_library() {
  local name
  for name in set_paths random_secret; do
    expect_ok "steps library: the library defines $name" \
      bash -c 'source "$1" && declare -F -- "$2" >/dev/null' _ "$ROOT/etc/restore-steps.sh" "$name"
    expect_no "steps library: restore-test does not define $name" grep -Eq "^$name\(\) \{" "$BIN/restore-test"
    expect_no "steps library: restore-volume does not define $name" grep -Eq "^$name\(\) \{" "$BIN/restore-volume"
  done
  expect_ok "steps library: select_set calls set_paths" \
    bash -c 'sed -n "/^select_set() {/,/^}/p" "$1" | grep -q "set_paths"' _ "$ROOT/etc/restore-steps.sh"
}

# Step 3: the files that restore-test needs before it starts any tool.
case_volume_gate_inputs() {
  new_volume_case gate-marker
  rm -- "$B/.backup-marker"
  volume_gate_case "missing marker" "marker"
  expect_eq "missing marker: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  local file
  for file in restore-code.sql restore-code.expected; do
    new_volume_case "gate-code-$file"
    rm -- "$CASE/code/$file"
    volume_gate_case "missing $file" "restore-code.sql or restore-code.expected is missing"
    expect_eq "missing $file: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
  new_volume_case gate-pgpass
  rmdir "$CASE/pgpass"
  volume_gate_case "missing pgpass directory" "the directory of the secret files is missing"
  expect_eq "missing pgpass directory: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# Step 3: the stamp names the set. A missing set, a manifest that is no regular file, and a set
# that is not the newest one. The script takes the set of the stamp and not the newest set.
case_volume_gate_set() {
  new_volume_case gate-no-set
  rm -- "$B/$RESTORE_BASE.dump.sha256"
  volume_gate_case "no manifest" "no complete backup set"
  expect_eq "no manifest: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  new_volume_case gate-manifest-link
  mv -- "$B/$RESTORE_BASE.dump.sha256" "$B/real.sha256"
  ln -s real.sha256 "$B/$RESTORE_BASE.dump.sha256"
  volume_gate_case "manifest is a symbolic link" "no complete backup set"
  new_volume_case gate-other-stamp
  volume_run_stamp 20200101_0000
  expect_no "other stamp: exit code is not 0" test "$RC" -eq 0
  expect_ok "other stamp: the ERROR line says there is no set" error_has "no complete backup set"
  expect_eq "other stamp: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  local newer
  new_volume_case gate-older-stamp
  newer=$RESTORE_BASE
  make_restore_set "" 3
  printf 'changed' >>"$B/$newer.dump"
  volume_run
  expect_eq "older stamp: exit code" "$RC" 0
  expect_ok "older stamp: the script used the set of the stamp" \
    grep -qF "\"set\":\"$RESTORE_BASE\"" "$CASE/out"
  new_volume_case gate-older-stamp-bad
  make_restore_set "" 3
  printf 'changed' >>"$B/$RESTORE_BASE.dump"
  volume_gate_case "older set with a bad dump" "manifest"
  expect_eq "older set with a bad dump: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# Step 3: the manifest lists the three files of the set, and each hash is right.
case_volume_gate_manifest() {
  local file variant
  for file in dump globals counts; do
    new_volume_case "gate-mismatch-$file"
    case $file in
      dump) printf 'changed' >>"$B/$RESTORE_BASE.dump" ;;
      globals) printf 'changed' >>"$B/globals_$RESTORE_NAME.sql" ;;
      counts) printf 'changed' >>"$B/$RESTORE_BASE.counts" ;;
    esac
    volume_gate_case "manifest mismatch ($file)" "manifest"
    expect_eq "manifest mismatch ($file): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
  for variant in no-dump extra-line other-name; do
    new_volume_case "gate-files-$variant"
    case $variant in
      no-dump) sed -i '1d' "$B/$RESTORE_BASE.dump.sha256" ;;
      extra-line) printf '%s  other.txt\n' "$(hash_of other)" >>"$B/$RESTORE_BASE.dump.sha256" ;;
      other-name) sed -i "s/recipe_$RESTORE_NAME.counts/..\/counts/" "$B/$RESTORE_BASE.dump.sha256" ;;
    esac
    volume_gate_case "manifest files ($variant)" "manifest"
    expect_eq "manifest files ($variant): no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
  new_volume_case gate-set-file-missing
  rm -- "$B/$RESTORE_BASE.counts"
  volume_gate_case "counts file missing" "the newest set lacks a regular file"
}

# Step 3: a line of the counts file outside the grammar stops the run.
case_volume_gate_counts() {
  new_volume_case gate-counts
  printf 'not a count line\n' >>"$B/$RESTORE_BASE.counts"
  write_manifest "$RESTORE_NAME"
  volume_gate_case "counts file" "the counts file has a bad line"
  expect_eq "counts file: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# volume_globals_case <name> <text in the ERROR line> <line>...: the globals file holds exactly these lines.
volume_globals_case() {
  local name=$1 text=$2
  shift 2
  new_volume_case "$name"
  printf '%s\n' "$@" >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  volume_gate_case "$name" "$text"
  expect_eq "$name: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# Step 3: the role grammar of ADR-0095. The manifest is right, but the globals file is not.
case_volume_gate_globals() {
  local open="\\restrict $RESTRICT_KEY" close="\\unrestrict $RESTRICT_KEY" sql='CREATE ROLE recipe_app;'
  volume_globals_case globals-meta "the globals file holds a psql meta-command" \
    "$open" "$sql" '\! echo planted' "$close"
  volume_globals_case globals-no-pair "the globals file has no valid restrict pair" "$sql"
  volume_globals_case globals-copy "the globals file holds a statement outside the role grammar" \
    "$open" "COPY (SELECT 1) TO PROGRAM 'id';" "$close"
  volume_globals_case globals-password "the globals file holds a statement outside the role grammar" \
    "$open" "ALTER ROLE recipe_app PASSWORD 'x';" "$close"
  volume_globals_case globals-control "the globals file holds a control character" \
    "$open" $'-- note\001' "$close"
}

# volume_list_case <name> <entry of pg_restore --list> <text in the ERROR line>: the dump holds one more
# entry. The name check of step 3 stops the run after the list and before initdb.
volume_list_case() {
  new_volume_case "$1"
  volume_gate_case "$1" "$3" "STUB_LIST_EXTRA=$2"
  expect_eq "$1: only the list ran" "$(event_sequence)" "EVENT list,"
}

# Step 3: an entry that restore-code.expected does not hold, and a kind that the release refuses.
case_volume_gate_names() {
  volume_list_case unknown-function "240; 1255 16499 FUNCTION public planted() recipe_migrator" \
    '"entry":"FUNCTION|public planted()"'
  volume_list_case unknown-trigger "241; 2620 16498 TRIGGER public machine planted recipe_migrator" \
    '"entry":"TRIGGER|public machine planted"'
  volume_list_case unknown-event-trigger "242; 3466 16497 EVENT TRIGGER - planted postgres" \
    '"entry":"EVENT TRIGGER|- planted"'
  volume_list_case refused-procedure "250; 1255 16500 PROCEDURE public planted() recipe_migrator" \
    '"kind":"PROCEDURE"'
  volume_list_case refused-aggregate "251; 1255 16501 AGGREGATE public planted(integer) recipe_migrator" \
    '"kind":"AGGREGATE"'
  volume_list_case refused-materialized-view "252; 1259 16502 MATERIALIZED VIEW public planted recipe_migrator" \
    '"kind":"MATERIALIZED VIEW"'
  volume_list_case refused-operator "253; 2617 16503 OPERATOR public + (integer, integer) recipe_migrator" \
    '"kind":"OPERATOR"'
  volume_list_case name-grammar "256; 1255 16505 FUNCTION public planted() " \
    "an entry of pg_restore --list does not match the grammar"
  new_volume_case list-fails
  volume_gate_case "list fails" "pg_restore cannot list the dump" STUB_LIST_FAIL=1
  new_volume_case table-with-a-refused-word
  volume_run "STUB_LIST_EXTRA=255; 1259 16504 TABLE public operator_log recipe_migrator"
  expect_eq "a table with a refused word in its name passes: exit code" "$RC" 0
}

# Step 4: initdb gets the arguments of reliability.md 7.1 and of design D4. The password file is in
# the work directory. The password is in no argument.
case_volume_initdb_arguments() {
  new_volume_case initdb-arguments
  volume_run
  local work argv pwfile copy
  work=$(volume_work_dir)
  copy=$(volume_work_copy)
  expect_eq "initdb: exit code" "$RC" 0
  expect_eq "initdb: the list runs before initdb and the server starts after it" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_eq "initdb: one work directory" "$(volume_work_dir | wc -l)" 1
  argv=$(grep '^ARGV initdb ' "$CASE/stub/log")
  expect_eq "initdb: the arguments" "$argv" \
    "ARGV initdb --pgdata=$CASE/pgdata --username=$(id -un) --auth-local=peer --auth-host=scram-sha-256 --pwfile=$work/superuser.pw --data-checksums --lc-collate=C --lc-ctype=C.UTF-8"
  pwfile=$copy/superuser.pw
  expect_ok "initdb: the password file holds 48 hex characters" grep -Eqx '[0-9a-f]{48}' "$pwfile"
  expect_eq "initdb: the password file has mode 600" "$(stat -c %a -- "$pwfile")" 600
  expect_ok "initdb: the stub read a non-empty password file with mode 600" \
    grep -qx 'PWFILE nonempty mode=600' "$CASE/stub/log"
  expect_no "initdb: the password is in no argument" grep -qF -- "$(cat "$pwfile")" "$CASE/stub/log"
  expect_eq "initdb: the target holds no password file" "$(find "$CASE/pgdata" -name '*.pw')" ""
  expect_ok "initdb: the output is JSON of the job restore-volume" volume_json_ok "$CASE/out"
  expect_eq "initdb: one start line with the set" \
    "$(grep -c "\"message\":\"restore volume started\",\"set\":\"$RESTORE_BASE\"" "$CASE/out")" 1
  expect_eq "initdb: no ERROR line" "$(error_count "$CASE/out")" 0
}

# Step 4: initdb runs without LC_ALL and with the LANG of the image (design D4). The script itself
# sets LC_ALL=C. The locale settings of the new cluster then follow LANG, as in the entrypoint of db.
case_volume_initdb_environment() {
  new_volume_case initdb-environment
  volume_run
  expect_eq "environment: exit code" "$RC" 0
  expect_ok "environment: initdb has no LC_ALL and the LANG of the image" \
    grep -qx 'ENV initdb LC_ALL=unset LANG=en_US.utf8' "$CASE/stub/log"
  new_volume_case initdb-environment-lc-all
  volume_run LC_ALL=pl_PL.UTF-8 LANG=de_DE.utf8
  expect_eq "environment (LC_ALL set): exit code" "$RC" 0
  expect_ok "environment (LC_ALL set): initdb has no LC_ALL and keeps the LANG" \
    grep -qx 'ENV initdb LC_ALL=unset LANG=de_DE.utf8' "$CASE/stub/log"
  expect_ok "static: initdb runs with env -u LC_ALL" grep -Eq '^[[:space:]]+env -u LC_ALL initdb ' "$BIN/restore-volume"
}

# Step 4: the script keeps no copy of the configuration files and no list of the top-level names
# in the work directory (ADR-0105 (q)). The target keeps the files of initdb.
case_volume_initdb_copies() {
  new_volume_case initdb-copies
  volume_run
  local work file
  work=$(volume_work_copy)
  expect_eq "copies: exit code" "$RC" 0
  for file in postgresql.conf postgresql.auto.conf pg_ident.conf pg_hba.conf; do
    expect_no "copies: no $file.initdb in the work directory" test -e "$work/$file.initdb"
  done
  expect_no "copies: no top-level.initdb in the work directory" test -e "$work/top-level.initdb"
  expect_eq "copies: the files of the target keep the text of initdb"     "$(cat "$CASE/pgdata/postgresql.conf")" "# stub of initdb: postgresql.conf"
}

# Step 4: a failed initdb writes one ERROR line. The target stays empty and no server starts.
case_volume_initdb_fails() {
  new_volume_case initdb-fails
  local before
  before=$(set_listing)
  volume_run STUB_INITDB_FAIL=1
  expect_no "initdb fails: exit code is not 0" test "$RC" -eq 0
  expect_eq "initdb fails: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "initdb fails: the ERROR line says so" error_has "initdb failed"
  expect_ok "initdb fails: the ERROR line holds the first line of the message" error_has "could not create directory"
  expect_eq "initdb fails: the sequence" "$(event_sequence)" "EVENT list,EVENT initdb,"
  expect_eq "initdb fails: the target is empty" "$(target_listing)" ""
  expect_eq "initdb fails: the backup directory is unchanged" "$(set_listing)" "$before"
  new_volume_case initdb-omits-file
  volume_run STUB_INITDB_OMIT=pg_ident.conf
  expect_no "initdb omits a file: exit code is not 0" test "$RC" -eq 0
  expect_eq "initdb omits a file: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "initdb omits a file: the ERROR line says so" error_has "cannot keep a copy of a configuration file"
  expect_eq "initdb omits a file: the sequence" "$(event_sequence)" "EVENT list,EVENT initdb,"
}
