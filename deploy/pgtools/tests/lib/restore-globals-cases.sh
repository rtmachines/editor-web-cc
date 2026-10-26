# Sourced by restore-test-test. It holds the cases of the gate of the globals file.
#
# The globals file is external input and runs as the scratch superuser. pg_dumpall 16 writes the
# line \restrict KEY before the SQL and the line \unrestrict KEY after it (pg_dumpall.c). A key is
# alphanumeric (dumputils.c, valid_restrict_key). The gate accepts no other backslash.

META="the globals file holds a psql meta-command"
PAIR="the globals file has no valid restrict pair"

# A backup writer must not change the SQL that psql receives after the globals check.
case_restore_globals_swap_after_check() {
  new_restore_case globals-swap-after-check
  cp -- "$B/globals_$RESTORE_NAME.sql" "$CASE/globals.checked"
  restore_run STUB_GLOBALS_SWAP=1
  expect_eq "globals swap: exit code" "$RC" 0
  expect_eq "globals swap: the globals file ran" "$(event_count globals)" 1
  expect_ok "globals swap: the backup file changed" \
    grep -qF "COPY (SELECT 1) TO PROGRAM 'touch /tmp/x';" "$B/globals_$RESTORE_NAME.sql"
  expect_ok "globals swap: psql received the checked content" \
    cmp -s -- "$CASE/globals.checked" "$CASE/stub/globals.received"
  expect_no "globals swap: psql received no swapped content" \
    grep -qF 'TO PROGRAM' "$CASE/stub/globals.received"
  expect_ok "globals swap: psql read the private copy" \
    grep -qF "$CASE/pgpass/run." "$CASE/stub/globals.path"
  expect_eq "globals swap: no pgpass file stays" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
  case $(uname -s) in
    MINGW* | MSYS*) ;;
    *) expect_eq "globals swap: the copy has mode 0600" "$(cat "$CASE/stub/globals.mode")" 600 ;;
  esac
}

# The disk checksum can pass after a writer replaces the file that supplied the private copy.
case_restore_globals_swap_before_manifest_check() {
  new_restore_case globals-swap-before-manifest-check
  cp -- "$B/globals_$RESTORE_NAME.sql" "$CASE/stub/manifest-globals.sql"
  printf 'CREATE ROLE swapped;\n' >>"$B/globals_$RESTORE_NAME.sql"
  mkdir "$CASE/hash-stubs"
  cat >"$CASE/hash-stubs/sha256sum" <<'EOF'
#!/usr/bin/env bash
if [ "${1:-}" = -c ]; then
  for file in "$BACKUP_DIR"/globals_*.sql; do
    cp -- "$STUB_DIR/manifest-globals.sql" "$file"
  done
fi
exec "$REAL_SHA256SUM" "$@"
EOF
  chmod 0755 "$CASE/hash-stubs/sha256sum"
  restore_run "REAL_SHA256SUM=$(command -v sha256sum)" "PATH=$CASE/hash-stubs:$ROOT/stubs:$PATH"
  expect_no "globals copy mismatch: exit code is not 0" test "$RC" -eq 0
  expect_eq "globals copy mismatch: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "globals copy mismatch: the ERROR line says so" \
    error_has 'the globals copy does not match the manifest'
  expect_eq "globals copy mismatch: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "globals copy mismatch: no pgpass file stays" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# globals_case <name> <text in the ERROR line> <line>...: the globals file of the set holds exactly
# these lines. The manifest is new. The gate stops the test before any tool runs.
globals_case() {
  local name=$1 text=$2 before
  shift 2
  new_restore_case "$name"
  printf '%s\n' "$@" >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  before=$(set_listing)
  restore_run
  restore_failed_checks "$name" "$before"
  expect_ok "$name: the ERROR line says so" error_has "$text"
  expect_eq "$name: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

case_restore_globals_layout() {
  new_restore_case globals-real-layout
  restore_run
  expect_eq "globals real layout: exit code" "$RC" 0
  expect_ok "globals real layout: the file has the restrict pair" \
    grep -qx "\\\\restrict $RESTRICT_KEY" "$B/globals_$RESTORE_NAME.sql"
  new_restore_case globals-no-final-newline
  printf '\\restrict %s\n\nCREATE ROLE recipe_app;\n\n\\unrestrict %s' "$RESTRICT_KEY" "$RESTRICT_KEY" \
    >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  restore_run
  expect_eq "globals without a final newline: exit code" "$RC" 0
}

# A backslash line that is not the pair, or a backslash in a line, is a meta-command.
case_restore_globals_meta_commands() {
  local open="\\restrict $RESTRICT_KEY" close="\\unrestrict $RESTRICT_KEY" sql='CREATE ROLE recipe_app;'
  globals_case globals-meta-after "$META" "$open" "$sql" "$close" '\! echo planted'
  globals_case globals-meta-between "$META" "$open" '\! echo planted' "$sql" "$close"
  globals_case globals-meta-before "$META" '\! echo planted' "$open" "$sql" "$close"
  globals_case globals-meta-in-line "$META" "$open" "$sql"' \! echo planted' "$close"
  globals_case globals-meta-gexec "$META" "$open" 'SELECT 1 \gexec' "$close"
  globals_case globals-meta-after-key "$META" "$open"';\! echo planted' "$sql" "$close"
  globals_case globals-meta-indented "$META" "$open" "$sql" '  \! echo planted' "$close"
  globals_case globals-meta-in-comment "$META" "$open" '-- \! echo planted' "$sql" "$close"
  globals_case globals-carriage-return "$META" "$open"$'\r' "$sql" "$close"
}

# A key outside the grammar [A-Za-z0-9]+ is no restrict line, so it is a meta-command.
case_restore_globals_keys() {
  local key sql='CREATE ROLE recipe_app;' variant=0
  for key in 'ab-cd' '' 'abc def' 'ab_cd' 'ab.cd' 'ab;cd' 'ab$cd' "ab'cd" 'abc"'; do
    variant=$((variant + 1))
    globals_case "globals-key-$variant" "$META" "\\restrict $key" "$sql" "\\unrestrict $key"
  done
  globals_case globals-different-keys "$PAIR" \
    "\\restrict $RESTRICT_KEY" "$sql" "\\unrestrict ${RESTRICT_KEY}1"
  globals_case globals-key-case "$PAIR" \
    "\\restrict $RESTRICT_KEY" "$sql" "\\unrestrict ${RESTRICT_KEY,,}"
}

# Every line holds no control character except a tab. PostgreSQL can end a comment at a CR byte.
case_restore_globals_control_characters() {
  local open="\\restrict $RESTRICT_KEY" close="\\unrestrict $RESTRICT_KEY" sql='CREATE ROLE recipe_app;'
  local text="the globals file holds a control character"
  globals_case globals-control-before "$text" $'-- note\001' "$open" "$sql" "$close"
  globals_case globals-control-after "$text" "$open" "$sql" "$close" $'-- note\001'
  globals_case globals-control-blank-line "$text" "$open" "$sql" "$close" $'\001'
  globals_case globals-control-cr-comment "$text" "$open" "$sql" "$close" $'-- note\r'
  globals_case globals-control-escape "$text" $'--\033[2J' "$open" "$sql" "$close"
  globals_case globals-control-del "$text" "$open" "$sql" "$close" $'-- note\177'
  new_restore_case globals-comment-with-tab
  printf '%s\n' $'--\tnote' "$open" "$sql" "$close" $'-- done\t' >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  restore_run
  expect_eq "globals comment with a tab: exit code" "$RC" 0
  globals_nul_case globals-control-nul-between $'\\restrict key\n-- note' $'\n\\unrestrict key\n' 2
  globals_nul_case globals-control-nul-at-end $'\\restrict key\n\\unrestrict key\n' '' 3
}

# Bash cannot store a NUL byte in a variable. Write that byte directly into the file.
globals_nul_case() {
  local name=$1 before
  new_restore_case "$name"
  printf '%s\0%s' "$2" "$3" >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  before=$(set_listing)
  restore_run
  restore_failed_checks "$name" "$before"
  expect_ok "$name: the ERROR line names the control character" \
    error_has 'the globals file holds a control character'
  expect_ok "$name: the ERROR line names the line" error_has "line $4"
  expect_eq "$name: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# Each form of ADR-0095 (b) must reach the restore steps.
globals_pass_case() {
  local name=$1
  shift
  new_restore_case "$name"
  printf '%s\n' "\\restrict $RESTRICT_KEY" "$@" "\\unrestrict $RESTRICT_KEY" \
    >"$B/globals_$RESTORE_NAME.sql"
  write_manifest "$RESTORE_NAME"
  restore_run
  expect_eq "$name: exit code" "$RC" 0
  expect_eq "$name: the globals file ran" "$(event_count globals)" 1
}

case_restore_globals_grammar_accepts() {
  globals_pass_case globals-form-set 'SET default_transaction_read_only = off;' \
    "SET client_encoding = 'UTF8';" 'SET standard_conforming_strings = on;'
  globals_pass_case globals-form-create 'CREATE ROLE recipe_app;'
  globals_pass_case globals-form-attributes \
    'ALTER ROLE postgres WITH SUPERUSER INHERIT CREATEROLE CREATEDB LOGIN REPLICATION BYPASSRLS;' \
    'ALTER ROLE audit_owner WITH NOSUPERUSER NOINHERIT NOCREATEROLE NOCREATEDB NOLOGIN NOREPLICATION NOBYPASSRLS;'
  globals_pass_case globals-form-setting "ALTER ROLE recipe_app SET log_error_verbosity TO 'terse';" \
    "ALTER ROLE recipe_app SET log_parameter_max_length TO '0';"
  globals_pass_case globals-form-grant 'GRANT audit_owner TO recipe_migrator;' \
    'GRANT audit_owner TO recipe_migrator WITH INHERIT FALSE GRANTED BY postgres;' \
    'GRANT pg_read_all_data TO recipe_backup GRANTED BY postgres;' \
    'GRANT pg_read_all_stats TO audit_owner WITH ADMIN OPTION, INHERIT TRUE, SET FALSE;' \
    'GRANT audit_owner TO recipe_migrator WITH SET TRUE;'
}

# Every bad statement occupies line 3. The error must identify that line without its text.
globals_grammar_case() {
  local name=$1
  shift
  globals_case "$name" 'the globals file holds a statement outside the role grammar' \
    "\\restrict $RESTRICT_KEY" '-- Roles' "$@" "\\unrestrict $RESTRICT_KEY"
  expect_ok "$name: the ERROR line names line 3" error_has 'line 3'
}

case_restore_globals_grammar_rejects() {
  globals_grammar_case globals-copy "COPY (SELECT 1) TO PROGRAM 'id';"
  globals_grammar_case globals-do 'DO $$ BEGIN NULL; END $$;'
  globals_grammar_case globals-function 'CREATE FUNCTION evil() RETURNS int LANGUAGE sql AS $$ SELECT 1 $$;'
  globals_grammar_case globals-preload "SET session_preload_libraries = 'x';"
  globals_grammar_case globals-role-preload "ALTER ROLE recipe_app SET session_preload_libraries TO 'x';"
  globals_grammar_case globals-password "ALTER ROLE recipe_app PASSWORD 'x';"
  globals_grammar_case globals-grant-option 'GRANT audit_owner TO recipe_migrator WITH UNKNOWN TRUE;'
  globals_grammar_case globals-setting-quote "ALTER ROLE recipe_app SET log_error_verbosity TO 'ter''se';"
  globals_grammar_case globals-two-statements 'CREATE ROLE recipe_app; CREATE ROLE evil;'
  globals_grammar_case globals-trailing-text 'CREATE ROLE recipe_app; note'
  globals_grammar_case globals-split-statement 'CREATE ROLE' 'recipe_app;'
  globals_grammar_case globals-quoted-role 'CREATE ROLE "recipe_app";'
  globals_grammar_case globals-lower-case 'create ROLE recipe_app;'
  globals_grammar_case globals-login-password "ALTER ROLE recipe_app WITH LOGIN PASSWORD 'x';"
  globals_grammar_case globals-hyphen-role 'CREATE ROLE recipe-app;'
  globals_grammar_case globals-program-grant 'GRANT pg_execute_server_program TO recipe_app;'
  globals_case globals-control-cr-between 'the globals file holds a control character' \
    "\\restrict $RESTRICT_KEY" "-- note$(printf '\r')COPY (SELECT 1) TO PROGRAM 'id';" \
    "\\unrestrict $RESTRICT_KEY"
  globals_case globals-control-between 'the globals file holds a control character' \
    "\\restrict $RESTRICT_KEY" $'-- note\001' "\\unrestrict $RESTRICT_KEY"
}

# One pair, in the right order, with the SQL between the two lines and nowhere else.
case_restore_globals_pair() {
  local open="\\restrict $RESTRICT_KEY" close="\\unrestrict $RESTRICT_KEY" sql='CREATE ROLE recipe_app;'
  globals_case globals-no-restrict "$PAIR" "$sql" "$close"
  globals_case globals-no-unrestrict "$PAIR" "$open" "$sql"
  globals_case globals-two-pairs "$PAIR" "$open" "$sql" "$close" "$open" "$close"
  globals_case globals-pair-reversed "$PAIR" "$close" "$sql" "$open"
  globals_case globals-sql-after "$PAIR" "$open" "$sql" "$close" 'DROP ROLE recipe_app;'
  globals_case globals-sql-before "$PAIR" 'DROP ROLE recipe_app;' "$open" "$sql" "$close"
  globals_case globals-empty-file "$PAIR"
  globals_case globals-only-comments "$PAIR" '--' '-- nothing'
}
