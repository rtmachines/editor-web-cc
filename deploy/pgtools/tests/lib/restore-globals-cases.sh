# Sourced by restore-test-test. It holds the cases of the gate of the globals file.
#
# The globals file is external input and runs as the scratch superuser. pg_dumpall 16 writes the
# line \restrict KEY before the SQL and the line \unrestrict KEY after it (pg_dumpall.c). A key is
# alphanumeric (dumputils.c, valid_restrict_key). The gate accepts no other backslash.

META="the globals file holds a psql meta-command"
PAIR="the globals file has no valid restrict pair"

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

# A comment or blank line outside the pair holds no control character, except a tab. psql could
# read such a character as more than a comment. The lines inside the pair are plain SQL.
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
