# Sourced by pitr-volume-test. It holds the cases of steps 5 to 7 of ADR-0106 (f): the copy of the base
# backup, the check of the copy, the system identifier of the copy, and the SHA-256 values of the four
# configuration files (ADR-0105 (q)).
# A case with a good copy runs all 12 steps. A case with a bad copy ends at its step with one ERROR line.
# The INFO line "configuration files of the copy hashed" shows the SHA-256 values in the fields
# postgresql_conf, postgresql_auto_conf, pg_hba_conf, and pg_ident_conf.
# A failed step of 5 to 7 writes one ERROR line and starts no server, so the stub of pg_ctl is never called.
# It needs lib/helpers.sh and lib/pitr-volume-stubs.sh (add_pitr_base, the stubs of pg_verifybackup
# and pg_controldata), and lib/pitr-volume-select-cases.sh (clear_pitr_bases, BASE_A).

# new_pitr_copy_case <name>: a case with one base backup that holds the four configuration files
# (add_pitr_base writes them) and a nested file.
# The directory of the base backup has mode 0755, so the case can see the chmod 0700 of the target.
# BACKUP_PGPASS_DIR names an empty private directory. The script must leave it empty.
new_pitr_copy_case() {
  new_pitr_case "$1"
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  mkdir "$CASE/wal/base/$BASE_A/pg_wal"
  printf 'x\n' >"$CASE/wal/base/$BASE_A/pg_wal/nested"
  chmod 0755 "$CASE/wal/base/$BASE_A"
}

# pitr_copy_run [VAR=value ...]: run the script with the stubs and keep the lists of the call.
pitr_copy_run() {
  pitr_run "$@" --
}

# pitr_tool_calls <tool>: the number of calls of a stub tool.
pitr_tool_calls() { grep -c "^EVENT $1\$" "$CASE/stub/log" || true; }

# pitr_hashed <key>: the value of a field of the INFO line of the SHA-256 values.
pitr_hashed() {
  sed -n 's/.*"message":"configuration files of the copy hashed".*"'"$1"'":"\([0-9a-f]*\)".*/\1/p' "$CASE/out"
}

# pitr_copy_refused <label> <ERROR text> [VAR=value ...]: the run ends with one ERROR line that holds
# the text. It starts no server, writes no INFO line of the hashes, and leaves the WAL archive as it was.
pitr_copy_refused() {
  local label=$1 text=$2 wal tool
  shift 2
  wal=$(pitr_listing "$CASE/wal")
  pitr_copy_run "$@"
  expect_no "$label: exit code is not 0" test "$RC" -eq 0
  expect_eq "$label: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$label: the ERROR line holds [$text]" error_has "$text"
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  for tool in pg_ctl initdb psql pg_resetwal; do
    expect_eq "$label: no call of $tool" "$(pitr_tool_calls "$tool")" 0
  done
  expect_eq "$label: no INFO line of the hashes" "$(grep -c 'configuration files of the copy hashed' "$CASE/out" || true)" 0
  expect_eq "$label: the WAL archive is unchanged" "$(pitr_listing "$CASE/wal")" "$wal"
  expect_eq "$label: no file below the private directory" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# A passed copy: the target holds the base backup without backup_end_time, has mode 0700, and the
# script keeps the four SHA-256 values. The script calls pg_verifybackup before pg_controldata, and both
# before the start of the server. The run then completes. The list of the target at the start of the server
# comes from the record of the stub of pg_ctl, because the later steps add files to the target.
case_pitr_copy_pass() {
  local file want base_list target_list calls
  new_pitr_copy_case copy-pass
  pitr_copy_run
  expect_eq "copy: exit code" "$RC" 0
  expect_eq "copy: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "copy: the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
  expect_ok "copy: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_ok "copy: the target holds the data file" test -f "$CASE/target/data"
  expect_ok "copy: the target holds a nested file" test -f "$CASE/target/pg_wal/nested"
  expect_ok "copy: the target holds backup_label" test -f "$CASE/target/backup_label"
  expect_no "copy: the target holds no backup_end_time" test -e "$CASE/target/backup_end_time"
  expect_ok "copy: the base backup keeps backup_end_time" test -f "$CASE/wal/base/$BASE_A/backup_end_time"
  expect_eq "copy: the target has mode 0700" "$(stat -c %a "$CASE/target")" 700
  base_list=$(cd "$CASE/wal/base/$BASE_A" && find . -mindepth 1 ! -name backup_end_time -printf '%p %y %s\n' | LC_ALL=C sort)
  target_list=$(grep -v '^\./recovery\.signal ' "$CASE/stub/start-listing")
  expect_eq "copy: at the start of the server the target holds the files of the base backup and nothing else" \
    "$target_list" "$base_list"
  calls=$(event_sequence)
  expect_ok "copy: the copy tools run first, in this order, and the server starts after them" \
    bash -c '[[ $1 == "EVENT pg_verifybackup,EVENT pg_controldata,EVENT pg_ctl,"* ]]' _ "$calls"
  expect_ok "copy: pg_verifybackup ignores backup_end_time and checks the target" \
    grep -qxF "ARGV pg_verifybackup --quiet --ignore=backup_end_time $CASE/target" "$CASE/stub/log"
  expect_ok "copy: pg_controldata reads the target" \
    grep -qxF "ARGV pg_controldata --pgdata=$CASE/target" "$CASE/stub/log"
  expect_eq "copy: one verification of the copy" "$(pitr_tool_calls pg_verifybackup)" 1
  for file in "${COPY_CONFIG_FILES[@]}"; do
    want=$(sha256sum "$CASE/target/$file" | cut -d' ' -f1)
    expect_eq "copy: the SHA-256 value of $file" "$(pitr_hashed "${file//./_}")" "$want"
  done
  expect_eq "copy: no baseline file below the private directory" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  expect_eq "copy: no baseline file in the target" "$(find "$CASE/target" -iname '*baseline*' -o -iname '*sha256*' | wc -l)" 0
}

# The values follow the content of the copy, so two different files give two different values.
case_pitr_copy_hash_values() {
  local first second
  new_pitr_copy_case copy-hash-a
  pitr_copy_run
  first=$(pitr_hashed postgresql_auto_conf)
  new_pitr_copy_case copy-hash-b
  printf '# changed\n' >"$CASE/wal/base/$BASE_A/postgresql.auto.conf"
  pitr_copy_run
  second=$(pitr_hashed postgresql_auto_conf)
  expect_ok "hash: a value has 64 hex digits" test "${#second}" -eq 64
  expect_no "hash: another content gives another value" test "$first" = "$second"
}

# A check of the copy that fails gives one ERROR line and starts no server. The script then reads no
# identifier, because the copy is not trusted.
case_pitr_copy_verify_fails() {
  new_pitr_copy_case copy-verify-fails
  pitr_copy_refused "failed check of the copy" "the check of the copy" STUB_VERIFY_FAIL=1
  expect_eq "failed check of the copy: pg_controldata not called" "$(pitr_tool_calls pg_controldata)" 0
}

# A copy of another system gives one ERROR line and starts no server (step 6).
case_pitr_copy_identifier() {
  local variant id
  for variant in other empty letters; do
    new_pitr_copy_case "copy-id-$variant"
    case $variant in
      other) id=7300000000000000002 ;;
      empty) id="" ;;
      letters) id=73abc ;;
    esac
    pitr_copy_refused "system identifier of the copy ($variant)" "system identifier" "STUB_CONTROL_ID=$id"
  done
  new_pitr_copy_case copy-id-tool-fails
  pitr_copy_refused "pg_controldata fails" "cannot read the system identifier of the copy" STUB_CONTROL_FAIL=1
}

# A missing configuration file in the copy gives one ERROR line (step 7). A link or a directory
# in its place gives the same line.
case_pitr_copy_config_missing() {
  local file
  for file in "${COPY_CONFIG_FILES[@]}"; do
    new_pitr_copy_case "copy-missing-$file"
    rm -- "$CASE/wal/base/$BASE_A/$file"
    pitr_copy_refused "missing $file" "configuration file of the copy"
    expect_ok "missing $file: the ERROR line names the file" error_has "\"file\":\"$file\""
  done
  new_pitr_copy_case copy-config-directory
  rm -- "$CASE/wal/base/$BASE_A/pg_hba.conf"
  mkdir "$CASE/wal/base/$BASE_A/pg_hba.conf"
  pitr_copy_refused "a directory in place of pg_hba.conf" "configuration file of the copy"
}

# The copy takes the selected base backup only. The older base backup adds nothing to the target.
case_pitr_copy_selected_only() {
  local file
  new_pitr_copy_case copy-selected-only
  printf 'old
' >"$CASE/wal/base/$BASE_A/only-in-a"
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "2026-01-02 00:00:00 UTC"
  for file in "${COPY_CONFIG_FILES[@]}"; do
    printf '# %s of the newer base backup
' "$file" >"$CASE/wal/base/$BASE_B/$file"
  done
  printf 'new
' >"$CASE/wal/base/$BASE_B/only-in-b"
  pitr_copy_run
  expect_eq "selected only: exit code" "$RC" 0
  expect_ok "selected only: the target holds the file of the selected base backup" test -f "$CASE/target/only-in-b"
  expect_no "selected only: the target holds nothing of the older base backup" test -e "$CASE/target/only-in-a"
}
