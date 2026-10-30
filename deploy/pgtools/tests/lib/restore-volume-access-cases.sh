# Sourced by restore-volume-test. It holds the cases of steps 7 and 8 of ADR-0103 (i): the REVOKE
# statements, the file pg_hba.conf, the comparison with restore-roles.expected, the three
# configuration files, and the top-level names of the target.
# It needs lib/restore-volume-cases.sh (the helpers and the VOLUME_SEQ_* sequences) and
# lib/restore-volume-start-cases.sh (volume_step_fails).

# Step 8 compares the rules of pg_hba.conf in the order of the file, so the stub reads the file that
# the script wrote. A stub cluster whose pg_hba.conf is still the file of initdb has no rule.

# volume_hba_want <subnet> <file>: the file that write_hba_conf of the shared client rules writes.
volume_hba_want() {
  (
    # shellcheck source=/dev/null
    source "$ROOT/etc/cluster-access.sh"
    write_hba_conf "$1" "$2"
  )
}

# Step 7: the script runs the output of print_default_db_revokes in one psql call over the private
# socket, after the audit chain check and before the query of restore-roles.sql.
case_volume_access_revokes() {
  new_volume_case access-revokes
  volume_run
  local work want
  work=$(volume_work_dir)
  want=$(
    # shellcheck source=/dev/null
    source "$ROOT/etc/cluster-access.sh"
    print_default_db_revokes
  )
  expect_eq "revokes: exit code" "$RC" 0
  expect_eq "revokes: the sequence" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_eq "revokes: one call" "$(event_count 'revoke')" 1
  expect_eq "revokes: psql got the output of print_default_db_revokes" "$(cat "$CASE/stub/revoke.received")" "$want"
  expect_ok "revokes: the call uses the private socket" grep -qF -- " --host=$work/sock " "$CASE/stub/revoke.argv"
  expect_ok "revokes: the call runs as the system user" grep -qF -- " --username=$(id -un) " "$CASE/stub/revoke.argv"
  expect_ok "revokes: the call stops at the first error" grep -qF -- ' -v ON_ERROR_STOP=1 ' "$CASE/stub/revoke.argv"
  expect_eq "revokes: the database of the call" "$(cat "$CASE/stub/revoke.dbname")" postgres
  expect_eq "revokes: no ERROR line" "$(error_count "$CASE/out")" 0
}

# Step 7: a failed REVOKE call gives one ERROR line. The script writes no pg_hba.conf and runs no query.
case_volume_access_revoke_fails() {
  new_volume_case access-revoke-fails
  volume_step_fails "revoke fails" "$VOLUME_SEQ_REVOKE" "psql cannot revoke the default access of the databases" \
    STUB_REVOKE_FAIL=1
  expect_ok "revoke fails: the ERROR line holds the first line of the message" error_has "stub failure"
  expect_eq "revoke fails: pg_hba.conf is the file of initdb" \
    "$(cat "$CASE/pgdata/pg_hba.conf")" "# stub of initdb: pg_hba.conf"
  expect_eq "revoke fails: no roles query ran" "$(event_count 'roles')" 0
}

# Step 7: pg_hba.conf is the file of write_hba_conf for COMPOSE_SUBNET. Step 8 passes for each subnet,
# also when the padding of the subnet differs.
case_volume_access_hba() {
  local subnet i=0
  for subnet in 172.28.0.0/16 192.168.100.0/24 10.0.0.0/8; do
    i=$((i + 1))
    new_volume_case "access-hba-$i"
    volume_run "COMPOSE_SUBNET=$subnet"
    volume_hba_want "$subnet" "$CASE/hba.want"
    expect_eq "hba ($subnet): exit code" "$RC" 0
    expect_eq "hba ($subnet): the sequence" "$(event_sequence)" "$VOLUME_SEQ_DONE"
    expect_eq "hba ($subnet): no ERROR line" "$(error_count "$CASE/out")" 0
    expect_ok "hba ($subnet): pg_hba.conf equals the file of write_hba_conf" \
      cmp -s -- "$CASE/pgdata/pg_hba.conf" "$CASE/hba.want"
    expect_eq "hba ($subnet): pg_hba.conf has mode 600" "$(stat -c %a -- "$CASE/pgdata/pg_hba.conf")" 600
    expect_eq "hba ($subnet): seven rules" \
      "$(grep -cvE '^[[:space:]]*(#|$)' "$CASE/pgdata/pg_hba.conf")" 7
    expect_ok "hba ($subnet): the rules hold the subnet" grep -qF -- " $subnet " "$CASE/pgdata/pg_hba.conf"
    expect_ok "hba ($subnet): the query read the file that the script wrote" \
      cmp -s -- "$CASE/stub/hba.at-query" "$CASE/hba.want"
    expect_no "hba ($subnet): no temporary file stays" test -e "$CASE/pgdata/pg_hba.conf.new"
  done
}

# Step 7: a temporary file that cannot be written gives one ERROR line before the query.
case_volume_access_hba_fails() {
  new_volume_case access-hba-fails
  volume_step_fails "hba write fails" "$VOLUME_SEQ_REVOKE" "cannot write pg_hba.conf" STUB_HBA_BLOCK=1
  expect_eq "hba write fails: no roles query ran" "$(event_count 'roles')" 0
}

# Step 8: the query runs restore-roles.sql of the code directory in the database of the dump, over
# the private socket. The file restore-roles.expected holds the placeholder only in HBA lines.
case_volume_access_query() {
  new_volume_case access-query
  volume_run
  local work
  work=$(volume_work_dir)
  expect_eq "query: exit code" "$RC" 0
  expect_eq "query: one call" "$(event_count 'roles')" 1
  expect_ok "query: the file is restore-roles.sql of the code directory" \
    grep -qF -- " --file=$CASE/code/restore-roles.sql" "$CASE/stub/log"
  expect_eq "query: the database" "$(cat "$CASE/stub/roles.dbname")" recipe
  expect_eq "query: the call uses the private socket" \
    "$(grep -c "^ARGV psql .*--host=$work/sock .*--dbname=recipe --file=$CASE/code/restore-roles.sql" "$CASE/stub/log")" 1
  expect_ok "query: the call is quiet" grep -qE -- '^ARGV psql --no-psqlrc --quiet .*--file=[^ ]*restore-roles.sql' "$CASE/stub/log"
  expect_eq "query: the placeholder is only in HBA lines" \
    "$(grep -F '@COMPOSE_SUBNET@' "$CASE/code/restore-roles.expected" | grep -vc '^HBA|')" 0
}

# Step 8: a query that fails gives one ERROR line.
case_volume_access_query_fails() {
  new_volume_case access-query-fails
  volume_step_fails "query fails" "$VOLUME_SEQ_ACCESS" "restore-roles.sql failed" STUB_ROLES_FAIL=1
  expect_ok "query fails: the ERROR line holds the first line of the message" error_has "stub failure"
}

# roles_differ <name> <first> <missing> <extra> [VAR=value ...]: the output of the query differs from
# restore-roles.expected. The run ends with one ERROR line, which names the first line by kind and name.
roles_differ() {
  local name=$1 first=$2 missing=$3 extra=$4
  shift 4
  new_volume_case "access-$name"
  volume_step_fails "roles ($name)" "$VOLUME_SEQ_ACCESS" \
    "the roles and the access of the new cluster differ from restore-roles.expected" "$@"
  expect_ok "roles ($name): the ERROR line counts the lines" error_has "\"missing\":$missing,\"extra\":$extra"
  expect_ok "roles ($name): the ERROR line names the first line" error_has "\"first\":\"$first\""
  expect_no "roles ($name): the ERROR line holds no hash" grep -q 'SCRAM-SHA-256' "$CASE/out"
}

# Step 8: each difference of a kind ends the run. The kinds are ACL, GRANT, HBA, MEMBER, ROLE, SETTING.
case_volume_access_roles_differ() {
  roles_differ role-attribute "ROLE|recipe_app" 1 1 \
    'STUB_ROLES_SED=s#^ROLE|recipe_app|f|t|f|f|t|f|f|-1|-#ROLE|recipe_app|t|t|f|f|t|f|f|-1|-#'
  roles_differ role-planted "ROLE|planted_admin" 0 1 'STUB_ROLES_EXTRA=ROLE|planted_admin|t|t|t|t|t|t|t|-1|-'
  roles_differ acl-dropped "ACL|recipe" 1 0 'STUB_ROLES_SED=/^ACL|recipe|recipe_migrator|recipe_app|CONNECT|f/d'
  roles_differ acl-public "ACL|postgres" 0 1 'STUB_ROLES_EXTRA=ACL|postgres|postgres|PUBLIC|CONNECT|f'
  roles_differ grant-dropped "GRANT|session" 1 0 'STUB_ROLES_SED=/^GRANT|session|DELETE/d'
  roles_differ hba-dropped "HBA|host all all all reject" 1 0 'STUB_ROLES_SED=/^HBA|host all all all reject/d'
  roles_differ member-dropped "MEMBER|audit_owner" 1 0 'STUB_ROLES_SED=/^MEMBER|audit_owner|/d'
  roles_differ setting-dropped "SETTING|-" 1 0 \
    'STUB_ROLES_SED=/^SETTING|-|recipe_app|log_error_verbosity/d'
}

# Step 8: the same lines in another order end the run. The first rule that matches wins in pg_hba.conf.
case_volume_access_roles_order() {
  roles_differ order "SETTING|-" 0 0 STUB_ROLES_REVERSE=1
}

# Step 8: the placeholder takes the value of COMPOSE_SUBNET. An expected file with another subnet
# ends the run. The runs with three other subnets pass (see case_volume_access_hba).
case_volume_access_roles_subnet() {
  new_volume_case access-roles-subnet
  sed -i 's#^HBA|host recipe recipe_app @COMPOSE_SUBNET@#HBA|host recipe recipe_app 10.9.9.0/24#' \
    "$CASE/code/restore-roles.expected"
  volume_step_fails "roles (other subnet)" "$VOLUME_SEQ_ACCESS" \
    "the roles and the access of the new cluster differ from restore-roles.expected"
  expect_ok "roles (other subnet): the ERROR line counts the lines" error_has '"missing":1,"extra":1'
}

# Step 3: the files restore-roles.sql and restore-roles.expected must exist. A missing file stops the run
# before initdb, like a missing file of the code comparison.
case_volume_access_files_missing() {
  local name before target
  for name in restore-roles.sql restore-roles.expected; do
    new_volume_case "access-missing-$name"
    rm -- "$CASE/code/$name"
    before=$(set_listing)
    target=$(target_listing)
    volume_run
    volume_failed_checks "missing $name" "$before" "$target"
    expect_ok "missing $name: the ERROR line names the files" error_has "restore-roles.sql or restore-roles.expected is missing"
  done
}

# Step 8: a changed configuration file ends the run, after the query of the roles.
case_volume_access_config_changed() {
  local file
  for file in postgresql.conf postgresql.auto.conf pg_ident.conf; do
    new_volume_case "access-config-$file"
    volume_step_fails "changed $file" "$VOLUME_SEQ_ACCESS" \
      "a configuration file of the new cluster changed after initdb" "STUB_CONFIG_CHANGE=$file"
    expect_ok "changed $file: the ERROR line names the file" error_has "\"file\":\"$file\""
    new_volume_case "access-config-removed-$file"
    volume_step_fails "removed $file" "$VOLUME_SEQ_ACCESS" \
      "a configuration file of the new cluster changed after initdb" "STUB_CONFIG_REMOVE=$file"
    expect_ok "removed $file: the ERROR line names the file" error_has "\"file\":\"$file\""
  done
}

# Step 8: a new top-level name or a missing top-level name ends the run. The files postmaster.pid
# and postmaster.opts of the running server do not count.
case_volume_access_top_level() {
  new_volume_case access-top-level-added
  volume_step_fails "new name" "$VOLUME_SEQ_ACCESS" \
    "the top-level names of the target differ from the names after initdb" STUB_TOPLEVEL_ADD=server.log
  expect_ok "new name: the ERROR line counts the names" error_has '"added":1,"removed":0'
  expect_ok "new name: the ERROR line names the first name" error_has '"first":"server.log"'
  new_volume_case access-top-level-removed
  volume_step_fails "missing name" "$VOLUME_SEQ_ACCESS" \
    "the top-level names of the target differ from the names after initdb" STUB_TOPLEVEL_REMOVE=global
  expect_ok "missing name: the ERROR line counts the names" error_has '"added":0,"removed":1'
  expect_ok "missing name: the ERROR line names the first name" error_has '"first":"global"'
  new_volume_case access-top-level-server-files
  volume_run STUB_POSTMASTER_OPTS=1
  expect_eq "server files: exit code" "$RC" 0
  expect_eq "server files: the sequence" "$(event_sequence)" "$VOLUME_SEQ_DONE"
  expect_no "server files: the stop deleted postmaster.pid" test -e "$CASE/pgdata/postmaster.pid"
  expect_ok "server files: postmaster.opts exists" test -f "$CASE/pgdata/postmaster.opts"
  expect_eq "server files: no ERROR line" "$(error_count "$CASE/out")" 0
}

# Steps 7 and 8 use the shared file of the client rules, and the script holds no second copy of its text.
case_volume_access_static() {
  local script=$BIN/restore-volume
  expect_ok "access static: the script runs print_default_db_revokes" grep -q 'print_default_db_revokes' "$script"
  expect_ok "access static: the script runs write_hba_conf" grep -q 'write_hba_conf' "$script"
  expect_no "access static: no copy of the REVOKE statements" grep -Eq 'REVOKE[[:space:]]+(CONNECT|ALL)' "$script"
  expect_no "access static: no password column" grep -Eqi 'rolpassword' "$script"
  expect_no "access static: no forbidden word" grep -Eq 'fsync|SCRATCH_PORT|--port|port=|ALTER ROLE' "$script"
  expect_ok "access static: the files of the roles come from the code directory" \
    grep -Eq 'CODE_DIR/restore-roles\.(sql|expected)' "$script"
}
