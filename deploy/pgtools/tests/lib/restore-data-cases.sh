# Sourced by restore-test-test. It holds the cases of the checks after the restore:
# row counts (step 6), audit chain (step 7), and logins (step 8).

# set_counts <text>: a new counts file for the set of the case, with a manifest that verifies.
set_counts() {
  printf '%s' "$1" >"$B/$RESTORE_BASE.counts"
  write_manifest "$RESTORE_NAME"
}

# counts_case <name> <counts file text> <restored counts text> <text in the ERROR line> [<text>]
# A difference stops the test after the comparison of the code and before the chain check.
counts_case() {
  local before
  new_restore_case "$1"
  set_counts "$2"
  printf '%s' "$3" >"$CASE/stub/restored.counts"
  before=$(set_listing)
  restore_run
  restore_failed_checks "$1" "$before"
  expect_ok "$1: the ERROR line says what differs" error_has "$4"
  expect_ok "$1: the ERROR line names the table" error_has "${5:-$4}"
  expect_eq "$1: stops before the chain check" "$(event_sequence)" \
    "${SEQ_TO_POST_DATA}EVENT counts,EVENT pg_ctl stop,"
}

case_restore_counts() {
  counts_case counts-table-missing $'public.machine|3\npublic.recipe|5\n' $'public.machine|3\n' \
    "a table of the counts file is missing in the restore" '"table":"public.recipe"'
  counts_case counts-table-extra $'public.machine|3\n' $'public.machine|3\npublic.recipe|5\n' \
    "a restored table is not in the counts file" '"table":"public.recipe"'
  counts_case counts-row-count $'public.machine|3\npublic.recipe|5\n' \
    $'public.machine|3\npublic.recipe|4\n' "the row count of a table differs" \
    '"table":"public.recipe"'
  counts_case counts-no-table-but-rows $'' $'public.machine|0\n' \
    "a restored table is not in the counts file" '"table":"public.machine"'
  counts_case counts-table-but-no-restore $'public.machine|0\n' $'' \
    "a table of the counts file is missing in the restore" '"table":"public.machine"'
}

# The last line of the counts file can lack its newline. The table count is the number of lines
# of the file, and the result line shows it (ADR-0066 (g)).
case_restore_counts_without_final_newline() {
  new_restore_case counts-no-final-newline
  set_counts $'public.machine|3\npublic.recipe|5'
  restore_run
  expect_eq "counts without a final newline: exit code" "$RC" 0
  expect_ok "counts without a final newline: both tables are counted" \
    grep -q '"tables":2,' "$CASE/out"
  new_restore_case counts-one-line-no-final-newline
  set_counts 'public.machine|3'
  printf 'public.machine|3\n' >"$CASE/stub/restored.counts"
  restore_run
  expect_eq "one counts line without a newline: exit code" "$RC" 0
  expect_ok "one counts line without a newline: the table is counted" \
    grep -q '"tables":1,' "$CASE/out"
  expect_eq "one counts line without a newline: the chain check ran" "$(event_count 'chain')" 1
}

# A bad line of the counts file stops the test before the first tool starts.
case_restore_counts_bad_line() {
  local variant=0 text before
  for text in $'public.machine|three\n' $'public.machine\n' $'|3\n' $'public.machine|3\r\n' \
    $'public.machine|3\n\n'; do
    variant=$((variant + 1))
    new_restore_case "counts-bad-line-$variant"
    set_counts "$text"
    before=$(set_listing)
    restore_run
    restore_failed_checks "counts bad line $variant" "$before"
    expect_ok "counts bad line $variant: the ERROR line says so" \
      error_has "the counts file has a bad line"
    expect_eq "counts bad line $variant: no tool ran" "$(wc -c <"$CASE/stub/log")" 0
  done
}

# An empty counts file means that the database has no table (ADR-0066 (g)). The restore holds
# no table, so the test passes. Without an audit_log table the chain check has nothing to check.
case_restore_empty_counts() {
  new_restore_case counts-empty
  set_counts ''
  : >"$CASE/stub/restored.counts"
  restore_run STUB_NO_AUDIT_TABLE=1
  expect_eq "empty counts: exit code" "$RC" 0
  expect_eq "empty counts: the chain check did not run" "$(event_count 'chain')" 0
  expect_eq "empty counts: the logins ran" "$(event_count 'login')" 8
  expect_ok "empty counts: the result line holds no table" grep -q '"tables":0' "$CASE/out"
  new_restore_case counts-empty-but-table
  set_counts ''
  restore_run
  expect_no "empty counts but a restored table: exit code is not 0" test "$RC" -eq 0
}

# A restored database with a table but no audit_log table is broken. The chain check must not pass.
case_restore_no_audit_table() {
  failed_restore_case no-audit-table STUB_NO_AUDIT_TABLE=1
  expect_ok "no audit table: the ERROR line names the table" error_has "audit_log"
  expect_eq "no audit table: no chain check ran" "$(event_count 'chain')" 0
}

case_restore_chain_check() {
  local result index=0
  for result in '1|t|t' '0|f|t' '0|t|f' '' 'x'; do
    index=$((index + 1))
    failed_restore_case "chain-$index" "STUB_CHAIN_RESULT=$result"
    expect_ok "chain [$result]: the ERROR line names the chain" error_has "audit chain"
    expect_eq "chain [$result]: stops before the logins" "$(event_count 'login')" 0
    expect_eq "chain [$result]: the chain check ran" "$(event_count 'chain')" 1
  done
}

# Each login role logs in with its random password over 127.0.0.1. A wrong password must fail.
case_restore_logins() {
  new_restore_case logins
  restore_run
  expect_eq "logins: exit code" "$RC" 0
  expect_eq "logins: one good and one wrong login for each role" \
    "$(grep -E '^EVENT login' "$CASE/stub/log" | tr '\n' ',')" \
    "EVENT login recipe_app,EVENT login recipe_app,EVENT login recipe_backup,EVENT login recipe_backup,EVENT login recipe_migrator,EVENT login recipe_migrator,EVENT login recipe_replicator,EVENT login recipe_replicator,"
  expect_eq "logins: every pgpass file has mode 600" \
    "$(grep -c '^PGPASSFILE mode=600$' "$CASE/stub/log")" 8
  expect_eq "logins: every connection uses the port of the scratch cluster" \
    "$(grep -E '^ARGV psql .*--host=127.0.0.1' "$CASE/stub/log" | grep -c -- '--port=5433')" 8
}

# A wrong password that the server accepts means that the authentication does not work.
case_restore_login_wrong_password_succeeds() {
  failed_restore_case login-wrong-succeeds STUB_LOGIN_ANY=1
  expect_ok "wrong password accepted: the ERROR line says so" error_has "wrong password succeeded"
  expect_eq "wrong password accepted: stops at the first role" "$(event_count 'login')" 2
}

case_restore_login_fails() {
  failed_restore_case login-fails STUB_LOGIN_NEVER=1
  expect_ok "login fails: the ERROR line names the role" error_has "recipe_app"
  expect_eq "login fails: stops at the first login" "$(event_count 'login')" 1
}

# The roles come from the restored globals. No role and a bad name stop the test.
case_restore_login_roles() {
  local variant before
  for variant in none bad-name; do
    new_restore_case "login-roles-$variant"
    case $variant in
      none) : >"$CASE/stub/login-roles" ;;
      bad-name) printf 'recipe_app\nbad name\n' >"$CASE/stub/login-roles" ;;
    esac
    before=$(set_listing)
    restore_run
    restore_failed_checks "login roles ($variant)" "$before"
    expect_eq "login roles ($variant): no login ran" "$(event_count 'login')" 0
    expect_eq "login roles ($variant): the ERROR line names the roles" \
      "$(grep -c '"level":"ERROR".*login role' "$CASE/out")" 1
  done
}
