#!/usr/bin/env bash
# Job restore-switch (design.md D9). This file holds the steps prepare, restore, switch, access, wal-line, check, refuse, switch-back, keep, and cleanup.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/switch-steps.sh"

# These tables hold the data of the fixture. The later check compares their counts.
COUNT_TABLES=(app_user machine unit recipe recipe_version parameter parameter_limit audit_log)

record_counts() {
  local table count file=$1
  : >"$file"
  for table in "${COUNT_TABLES[@]}"
  do
    count=$(sql -c "SELECT count(*) FROM public.$table")
    [[ $count =~ ^[0-9]+$ ]] || fail "the row count of $table is invalid"
    printf '%s|%s\n' "$table" "$count" >>"$file"
  done
}

prepare() {
  log "restore-switch prepare: start the stack"
  bash "$GITHUB_WORKSPACE/deploy/ci/start.sh"
  log "restore-switch prepare: log in and change the password"
  bash "$GITHUB_WORKSPACE/deploy/ci/login.sh"
  http_probe password
  log "restore-switch prepare: write the fixture data"
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_smoke.py" seed | sql
  http_probe export export.sha256
  record_counts "$CI_STATE/row-counts"
  log "restore-switch prepare: back up and test the restore"
  wait_backup_minute
  compose exec -T backup backup-now
  compose exec -T backup restore-test | tee "$CI_STATE/restore-prepare.log"
  grep -Eq '"message":"restore test passed".*"set":"recipe_[0-9]{8}_[0-9]{4}_now"' "$CI_STATE/restore-prepare.log" \
    || fail "restore-test did not prove the new now set"
  sed -En 's/.*"set":"(recipe_[0-9]{8}_[0-9]{4}_now)".*/\1/;T;p;q' "$CI_STATE/restore-prepare.log" >"$CI_STATE/backup.set"
  [[ $(<"$CI_STATE/backup.set") =~ ^recipe_[0-9]{8}_[0-9]{4}_now$ ]] || fail "the backup set name is invalid"
  log "restore-switch prepare: done"
}

# A legal image tag has 1 to 128 characters and starts with a letter, a digit, or an underscore.
TAG_PATTERN='^[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}$'

restore() {
  # ADR-0103 (b) steps 2 to 4. db keeps running on the old volume.
  local backup_set stamp volume last pattern
  [[ -s $CI_STATE/backup.set ]] || fail "restore: run prepare first."
  backup_set=$(<"$CI_STATE/backup.set")
  [[ $backup_set =~ ^recipe_([0-9]{8}_[0-9]{4}_now)$ ]] || fail "restore: the backup set name is invalid."
  stamp=${BASH_REMATCH[1]}
  log "restore: stop app and backup"
  compose stop app backup
  # ADR-0103 (e) gives the name. The label lets delete_ci_volumes find the volume.
  volume=recipe-editor_pgdata_$(date -u +%Y%m%d_%H%M%S)
  docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$volume" >/dev/null
  log "restore: run restore-volume for the set $backup_set into the volume $volume"
  compose run --rm --no-deps -T -v "$volume:/var/lib/postgresql/data" restore-volume "$stamp" \
    | tee "$CI_STATE/restore-volume.log"
  last=$(tail -n 1 "$CI_STATE/restore-volume.log")
  pattern='"message":"restore volume completed".*"set":"'$backup_set'","system_identifier":"([0-9]{1,20})"\}$'
  [[ $last =~ $pattern ]] || fail "restore: the last line of restore-volume is wrong."
  printf '%s\n' "$volume" >"$CI_STATE/new.volume"
  printf '%s\n' "${BASH_REMATCH[1]}" >"$CI_STATE/new.system-identifier"
  log "restore: the volume $volume holds the system identifier ${BASH_REMATCH[1]}"
}

# These patterns match the lines that 01-roles.sh and the image entrypoint write on the first start.
INIT_LOG_PATTERN='01-roles\.sh|docker-entrypoint-initdb\.d|init process complete|CREATE (ROLE|DATABASE|SCHEMA|FUNCTION|EVENT TRIGGER)|^(GRANT|REVOKE)$'

check_db_log() {
  # The restored cluster skips the init of the image, so its log holds no line of 01-roles.sh.
  local count
  compose logs --no-color --no-log-prefix db >"$CI_STATE/db-before-passwords.log"
  grep -Fq 'database system is ready to accept connections' "$CI_STATE/db-before-passwords.log" \
    || fail "access: the log of db has no start line."
  count=$(grep -c -E "$INIT_LOG_PATTERN" "$CI_STATE/db-before-passwords.log" || true)
  [[ $count == 0 ]] || fail "access: the log of db holds $count lines of 01-roles.sh."
}

access() {
  # ADR-0103 (b) step 6. The checks of the client rules run before any password exists.
  [[ -s $CI_STATE/active.volume ]] || fail "access: run switch first."
  log "access: step 1, postgres over TCP from the subnet of backend fails by the reject rule"
  expect_probe reject postgres
  log "access: step 2, password login of each login role before the passwords fails"
  expect_probe password "${LOGIN_ROLES[@]}"
  log "access: step 3, the log of db holds no line of 01-roles.sh"
  check_db_log
  log "access: step 4, set the passwords from the secret files"
  set_role_passwords
  log "access: step 5, password login of each login role after the passwords works"
  expect_probe ok "${LOGIN_ROLES[@]}"
  log "access: done"
}

wal_line() {
  # ADR-0103 (s). The new WAL line starts after the passwords and before app. Nothing is deleted.
  local name
  for name in change.log old.volume new.system-identifier active.volume
  do
    [[ -s $CI_STATE/$name ]] || fail "wal-line: run switch first."
  done
  log "wal-line: step 1, the switch deleted the walarchive container"
  start_wal_line "$(<"$CI_STATE/new.system-identifier")"
  log "wal-line: done"
}

compare_counts() {
  # The first argument is the file for the new counts. The second is the step name.
  # Login and export add audit entries, so audit_log may only grow. Every other table keeps its count.
  local table before after who=${2:-check}
  record_counts "$1"
  while IFS='|' read -r table before
  do
    after=$(sed -n "s/^$table|//p" "$1")
    if [[ $table == audit_log ]]
    then
      (( after >= before )) || fail "$who: the row count of audit_log fell from $before to $after."
    else
      [[ $after == "$before" ]] || fail "$who: the row count of $table is $after. Expected $before."
    fi
  done <"$CI_STATE/row-counts"
}

verify_export() {
  # The first argument is the step name. The second is the file for the export hash.
  # The export hash equals the hash from before the switch (prepare wrote it).
  check_readyz "$1"
  rm -f -- "$CI_STATE/cookies.json"
  http_probe relogin || fail "$1: the login failed."
  http_probe export "$2" || fail "$1: the export request failed."
  [[ $(<"$CI_STATE/$2") == "$(<"$CI_STATE/export.sha256")" ]] \
    || fail "$1: the export hash differs from the hash before the switch."
}

check_chain() {
  # Run the chain query of the restore test, so both use one text.
  local query result
  query=$(sed -n "/^IFS= read -r -d '' CHAIN_SQL <<'SQL'/,/^SQL\$/p" "$GITHUB_WORKSPACE/deploy/pgtools/lib/restore-steps.sh" | sed '1d;$d')
  [[ $query == WITH* ]] || fail "check: cannot read the chain query of restore-steps.sh."
  result=$(sql <<<"$query") || fail "check: the audit chain check could not run."
  [[ $result == "0|t|t" ]] || fail "check: the audit chain check failed."
}

check_flush() {
  # The marker is the newest audit row. The flush check and the .partial file prove that the archive holds it.
  local marker position segment
  marker=$(sql -c "SELECT chain_seq, encode(entry_hash, 'hex') FROM public.audit_log ORDER BY chain_seq DESC LIMIT 1")
  IFS='|' read -r PITR_MARKER_SEQ PITR_MARKER_HASH <<<"$marker"
  [[ $PITR_MARKER_SEQ =~ ^[1-9][0-9]{0,18}$ && $PITR_MARKER_HASH =~ ^[0-9a-f]{64}$ ]] \
    || fail "check: the audit marker identity is invalid."
  position=$(sql -c 'SELECT pg_current_wal_lsn()')
  [[ $position =~ $LSN_PATTERN ]] || fail "check: the WAL position is invalid."
  retry flush_reached "$position" || fail "check: the flush check did not return t."
  segment=$(sql -c "SELECT pg_walfile_name('$position'::pg_lsn - 1)")
  [[ $segment =~ ^[0-9A-F]{24}$ ]] || fail "check: the WAL segment name is invalid."
  sudo test -f "$RECIPE_WALARCHIVE_DIR/$segment.partial" || fail "check: the marker segment has no .partial file."
  if sudo test -e "$RECIPE_WALARCHIVE_DIR/$segment"
  then
    fail "check: the marker segment is complete, not partial."
  fi
  export PITR_MARKER_SEQ PITR_MARKER_HASH
}

check_db_log_passwords() {
  # Count the log lines that hold the value of a secret. The grep reads the patterns from the secret files.
  local name count files=()
  for name in postgres migrator app backup replicator
  do
    files+=(-f "$RECIPE_SECRETS_DIR/${name}_password")
  done
  compose logs --no-color --no-log-prefix db >"$CI_STATE/db-after-check.log"
  [[ -s $CI_STATE/db-after-check.log ]] || fail "check: the log of db is empty."
  count=$(sudo grep -c -F "${files[@]}" "$CI_STATE/db-after-check.log" || true)
  [[ $count == 0 ]] || fail "check: the log of db holds $count lines with a password."
}

check_base_directory() {
  # ADR-0103 (u). The first argument is the base backup of this check. wal-line made the other one.
  local listing name
  listing=$(archive_bases) || fail "check: cannot list /wal/base."
  [[ -n $listing ]] || fail "check: /wal/base holds no base backup."
  while IFS= read -r name
  do
    if grep -qxF -- "$name" "$CI_STATE/wal-old.bases"
    then
      fail "check: /wal/base holds a base backup of the old line."
    fi
    [[ $name == "$1" || $name == "$(<"$CI_STATE/wal-new.base")" ]] \
      || fail "check: /wal/base holds a base backup that is not of the new line."
  done <<<"$listing"
}

check_kept_files() {
  # ADR-0103 (t). The old volume and every moved file stay until the operator deletes them.
  # The file old-system-identifier holds what write_old_identifier wrote (ADR-0106 (k)).
  local path expected
  docker volume inspect "$(<"$CI_STATE/old.volume")" >/dev/null || fail "check: the old volume is gone."
  while IFS= read -r path
  do
    sudo test -e "$path" || fail "check: a file of the target directory is missing."
  done <"$CI_STATE/wal-target.manifest"
  # expected_old_identifier writes the error line. The exit keeps its status.
  expected=$(expected_old_identifier check) || exit "$?"
  [[ $(sudo cat "$(<"$CI_STATE/wal-target.dir")/old-system-identifier") == "$expected" ]] \
    || fail "check: the file old-system-identifier of the target directory changed."
  require_new_identity_file "$(<"$CI_STATE/new.system-identifier")"
}

check() {
  # ADR-0103 (b) step 8 and the checks of the new WAL line. Nothing is deleted.
  local name
  for name in wal-new.base wal-old.bases wal-target.dir wal-target.manifest row-counts export.sha256 old.volume new.system-identifier
  do
    [[ -e $CI_STATE/$name ]] || fail "check: run wal-line first."
  done
  log "check: step 1, truncate session and login_attempt, then start app, proxy, and backup"
  sql -c "TRUNCATE public.session, public.login_attempt" >/dev/null
  compose up -d --no-deps --wait --wait-timeout 180 app
  compose up -d --no-deps --wait --wait-timeout 180 proxy backup
  log "check: step 2, /readyz, the login, and the export hash"
  verify_export check export-after.sha256
  log "check: step 3, the row counts"
  compare_counts "$CI_STATE/row-counts.after" check
  log "check: step 4, the chain query, the flush check, the WAL check, and pitr-check"
  check_chain
  check_flush
  wal_check "$(<"$CI_STATE/wal-new.base")" check
  check_pitr
  log "check: step 5, the log of db holds no password"
  check_db_log_passwords
  log "check: step 6, basebackup-once and backup-now"
  take_base_backup check-base.log check
  wait_backup_minute
  compose exec -T backup backup-now
  log "check: step 7, /wal/base holds only base backups of the new line"
  check_base_directory "$LAST_BASE"
  log "check: step 8, the old volume and the files of the target directory stay"
  check_kept_files
  log "check: done"
}

containers_listing() {
  # Print the sorted IDs of every container of the project.
  compose ps -aq | sort
}

refuse_live_volume() {
  # Case 1. ADR-0103 (i) step 1 refuses a target that is not empty. A live data directory is never empty.
  local backup_set stamp output status=0
  backup_set=$(<"$CI_STATE/backup.set")
  [[ $backup_set =~ ^recipe_([0-9]{8}_[0-9]{4}_now)$ ]] || fail "refuse: the backup set name is invalid."
  stamp=${BASH_REMATCH[1]}
  log "refuse: case 1, restore-volume on the live volume $RECIPE_PGDATA_VOLUME"
  output=$(compose run --rm --no-deps -T -v "$RECIPE_PGDATA_VOLUME:/var/lib/postgresql/data" restore-volume "$stamp" \
    2>"$CI_STATE/refuse-live.err") || status=$?
  printf '%s\n' "$output" >"$CI_STATE/refuse-live.log"
  [[ $status == 1 ]] || fail "refuse: restore-volume on the live volume gave the exit code $status. Expected 1."
  # The refusal is the only line of the output. A later step of restore-volume would write more lines.
  [[ $output == *'"level":"ERROR","job":"restore-volume","message":"the target directory is not empty"}' && $output != *$'\n'* ]] \
    || fail "refuse: restore-volume on the live volume stopped for another reason."
  log "refuse: case 1 stopped as expected: the target directory is not empty."
}

refuse_missing_volume() {
  # Case 2. Compose stops on a missing external volume before it creates or replaces a container.
  local missing output status=0
  missing=${COMPOSE_PROJECT_NAME}_missing_$(date -u +%Y%m%d_%H%M%S)
  if docker volume inspect "$missing" >/dev/null 2>&1
  then
    fail "refuse: the volume for case 2 exists."
  fi
  log "refuse: case 2, up with the missing external volume $missing"
  output=$(export RECIPE_PGDATA_VOLUME=$missing; compose up -d --no-deps db 2>&1) || status=$?
  printf '%s\n' "$output" >"$CI_STATE/refuse-missing.log"
  [[ $status != 0 ]] || fail "refuse: Compose accepted the missing external volume."
  [[ $output == *"external volume \"$missing\" not found"* ]] \
    || fail "refuse: Compose stopped for another reason than the missing external volume."
  if docker volume inspect "$missing" >/dev/null 2>&1
  then
    fail "refuse: Compose created the missing external volume."
  fi
  log "refuse: case 2 stopped as expected: Compose reports the missing external volume."
}

make_foreign_volume() {
  # Make a labelled volume with a cluster of another system. The cluster never starts and has no network.
  FOREIGN_VOLUME=recipe-editor_pgdata_foreign_$(date -u +%Y%m%d_%H%M%S)
  docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$FOREIGN_VOLUME" >/dev/null
  printf '%s\n' "$FOREIGN_VOLUME" >"$CI_STATE/foreign.volume"
  docker run --rm --network none --security-opt no-new-privileges:true --cap-drop ALL \
    -v "$FOREIGN_VOLUME:/var/lib/postgresql/data" --entrypoint initdb "recipe-pgtools:${RECIPE_VERSION:?}" \
    --pgdata=/var/lib/postgresql/data --username=postgres --auth-local=reject --auth-host=reject \
    --locale=C --encoding=UTF8 >/dev/null || fail "refuse: initdb of the foreign volume failed."
}

refuse_foreign_volume() {
  # Case 3. The step 4 check of ADR-0103 (f) is the function that switch calls. It must stop on the identifier.
  local expected output
  expected=$(<"$CI_STATE/new.system-identifier")
  make_foreign_volume
  log "refuse: case 3, the step 4 check on the volume $FOREIGN_VOLUME of another system"
  if output=$( (check_new_volume "$FOREIGN_VOLUME" "$expected") 2>&1 )
  then
    fail "refuse: the check accepted the volume of another system."
  fi
  [[ $output == "CI: switch: the system identifier of the new volume differs from the line of restore-volume." ]] \
    || fail "refuse: the check of the foreign volume stopped for another reason."
  log "refuse: case 3 stopped as expected: the system identifier differs."
}

refuse() {
  # D9 step 5. Each negative case must fail for its reason. The live cluster must stay as check left it.
  local name containers_before db_before
  for name in backup.set active.volume new.system-identifier row-counts.after
  do
    [[ -s $CI_STATE/$name ]] || fail "refuse: run check first."
  done
  containers_before=$(containers_listing)
  db_before=$(container db)
  refuse_live_volume
  refuse_missing_volume
  [[ $(containers_listing) == "$containers_before" ]] || fail "refuse: the list of containers changed."
  [[ $(container db) == "$db_before" ]] || fail "refuse: the db container changed."
  refuse_foreign_volume
  record_counts "$CI_STATE/row-counts.refuse"
  cmp -s -- "$CI_STATE/row-counts.after" "$CI_STATE/row-counts.refuse" \
    || fail "refuse: the row counts of the live cluster changed."
  [[ $(db_volume) == "$RECIPE_PGDATA_VOLUME" ]] || fail "refuse: db does not use the live volume."
  [[ $(container db) == "$db_before" ]] || fail "refuse: the db container changed."
  log "refuse: the live cluster keeps its row counts and db keeps its volume."
  log "refuse: done"
}

# The configuration files that the comparison reads whole (ADR-0103 (m), ADR-0105 (m), design.md D9).
CONFIG_FILES=(postgresql.conf postgresql.auto.conf)

read_config_files() {
  # The first argument is the volume of a stopped cluster. The second is its label, old or new.
  # The container has no network and mounts the volume read-only. The output files hold the whole files.
  local file
  for file in "${CONFIG_FILES[@]}"
  do
    docker run --rm --network none --security-opt no-new-privileges:true \
      -v "$1:/var/lib/postgresql/data:ro" --entrypoint cat \
      "recipe-pgtools:${RECIPE_VERSION:?}" "/var/lib/postgresql/data/$file" >"$CI_STATE/config.$2.$file" \
      || fail "switch-back: cannot read $file of the $2 cluster."
  done
}

capture_roles() {
  # ADR-0103 (m). The first argument is the label, old or new. The query runs as the superuser while the cluster runs.
  # The option -q hides the command tag of SET, so the output holds only the lines of the query.
  sql -q <"$GITHUB_WORKSPACE/deploy/pgtools/restore-roles.sql" >"$CI_STATE/roles.$1" \
    || fail "switch-back: restore-roles.sql failed on the $1 cluster."
  [[ -s $CI_STATE/roles.$1 ]] || fail "switch-back: restore-roles.sql printed no line on the $1 cluster."
}

require_volumes_stay() {
  # ADR-0103 (g). The switch back deletes no volume. Both data volumes of the run must exist.
  local name
  for name in "$(<"$CI_STATE/old.volume")" "$(<"$CI_STATE/new.volume")"
  do
    docker volume inspect "$name" >/dev/null 2>&1 || fail "switch-back: a data volume is gone."
  done
}

first_identifier() {
  # Print the identifier of the first entry of a change log.
  sed -En 's/^Database system identifier: ([0-9]{1,20})$/\1/;T;p;q' "$1"
}

first_timeline() {
  # Print the timeline of the first entry of a change log (ADR-0106 (l)). An entry from before ADR-0106 has none, so nothing prints.
  # A line with the label and a bad value stops the run. A later entry never decides the check.
  local line value
  line=$(awk -v label="$TIMELINE_LABEL:" '/^volume: / { entries++ } entries == 1 && index($0, label) == 1 { print; exit }' "$1")
  [[ -n $line ]] || return 0
  value=${line#"$TIMELINE_LABEL: "}
  [[ $value =~ $TIMELINE_PATTERN ]] || fail "switch-back: the timeline in the change log is not valid."
  printf '%s\n' "$value"
}

old_tag() {
  # Print the tag of the first entry of a change log. A missing or illegal tag stops the run (ADR-0105 (d)).
  local tag
  tag=$(awk '/^volume: / { entries++ } entries == 1 && /^tag: / { sub(/^tag: /, ""); print; exit }' "$1")
  [[ -n $tag ]] || fail "switch-back: the change log holds no tag line for the old volume."
  [[ $tag =~ $TAG_PATTERN ]] || fail "switch-back: the tag in the change log is not a legal tag."
  printf '%s\n' "$tag"
}

switch_back_core() {
  # ADR-0103 (g). These are the steps of (f) with the old name. The first argument is the change log.
  # The second argument is capture or plain. Only capture saves the two configuration files of each cluster.
  local change_log=$1 old leaving leaving_tag expected expected_timeline output state="" identifier="" timeline="" old_image_tag
  old=$(<"$CI_STATE/old.volume")
  leaving=$RECIPE_PGDATA_VOLUME
  leaving_tag=${RECIPE_VERSION:?}
  expected=$(first_identifier "$change_log")
  [[ $expected =~ $IDENTIFIER_PATTERN ]] || fail "switch-back: the change log holds no identifier line."
  expected_timeline=$(first_timeline "$change_log")
  old_image_tag=$(old_tag "$change_log")
  log "switch-back: step 1, stop the services and delete the containers"
  stop_services stop
  log "switch-back: step 2, write the volume $leaving into the change log"
  # The leaving volume needs no state. The run never starts db on it and has no else branch.
  if output=$(controldata "$leaving" 2>/dev/null)
  then
    state=$(sed -En 's/^Database cluster state: +//p' <<<"$output")
    identifier=$(sed -En 's/^Database system identifier: +//p' <<<"$output")
    timeline=$(sed -En "s/^$TIMELINE_LABEL: +//p" <<<"$output")
  fi
  [[ $identifier =~ $IDENTIFIER_PATTERN ]] \
    || fail "switch-back: pg_controldata shows no system identifier of the volume that the run leaves."
  [[ $timeline =~ $TIMELINE_PATTERN ]] \
    || fail "switch-back: pg_controldata shows no timeline of the volume that the run leaves."
  change_log_entry "$leaving" "$leaving_tag" "$identifier" "$timeline" >>"$change_log"
  if [[ $2 == capture ]]
  then
    printf '%s\n' "$state" >"$CI_STATE/state.new"
    read_config_files "$leaving" new
  fi
  export RECIPE_PGDATA_VOLUME=$old
  export RECIPE_VERSION=$old_image_tag
  log "switch-back: step 3, Compose shows the volume $old"
  require_compose_volume switch-back "$old"
  log "switch-back: step 4, read the state of the volume $old and compare its identifier and timeline with the change log"
  check_new_volume "$old" "$expected" switch-back "the change log" "$expected_timeline"
  if [[ $2 == capture ]]
  then
    controldata_field "$old" 'Database cluster state' >"$CI_STATE/state.old"
    read_config_files "$old" old
  fi
  log "switch-back: step 5, start db on the old volume"
  start_db switch-back "$old"
}

run_wrong_switch_back() {
  # ADR-0103 (g), last item. A copy of the change log holds a wrong identifier. Step 4 of (f) must stop the run.
  # The run starts no db and deletes no volume. It stops in a subshell, so this process goes on.
  local wrong_log=$CI_STATE/change.log.wrong expected wrong=1 output status last
  expected=$(first_identifier "$CI_STATE/change.log")
  [[ $expected != 1 ]] || wrong=2
  sed -E 's/^(Database system identifier: )[0-9]+$/\1'"$wrong"'/' "$CI_STATE/change.log" >"$wrong_log"
  log "switch-back: part 2, run with a wrong identifier in a copy of the change log"
  set +e
  output=$( ( set -e; switch_back_core "$wrong_log" plain ) 2>&1 )
  status=$?
  set -e
  printf '%s\n' "$output" >"$CI_STATE/switch-back-wrong.log"
  printf '%s\n' "$output"
  [[ $status != 0 ]] || fail "switch-back: the run with the wrong identifier did not stop."
  last=$(tail -n 1 <<<"$output")
  [[ $last == "CI: switch-back: the system identifier of the new volume differs from the change log." ]] \
    || fail "switch-back: the run with the wrong identifier stopped for another reason."
  [[ -z $(compose ps -aq db) ]] || fail "switch-back: the run with the wrong identifier left a db container."
  require_volumes_stay
  log "switch-back: part 2 stopped as expected: the system identifier differs."
}

check_first_target_stays() {
  # ADR-0103 (g). The first target directory stays with every file that check listed.
  local path
  [[ $(<"$CI_STATE/wal-target-back.dir") != "$(<"$CI_STATE/wal-target.dir")" ]] \
    || fail "switch-back: the second target directory is the first target directory."
  while IFS= read -r path
  do
    sudo test -e "$path" || fail "switch-back: a file of the first target directory is missing."
  done <"$CI_STATE/wal-target.manifest"
}

compare_whole() {
  # ADR-0103 (m) and ADR-0105 (m). The first argument names the value. The other two are the files of the old and the new cluster.
  # The files are whole. No line is filtered. The text of the first difference is only for the log.
  local difference
  if cmp -s -- "$2" "$3"
  then
    return 0
  fi
  difference=$(diff -- "$2" "$3" || true)
  printf 'CI: switch-back: first difference in %s (< old cluster, > new cluster):\n' "$1" >&2
  awk '/^[0-9]/ { if (seen++) exit } { print }' <<<"$difference" >&2
  fail "switch-back: $1 of the old cluster and the new cluster differ."
}

compare_cluster_values() {
  # D9. Each cluster stopped clean before its files were read. The query outputs came from the running clusters.
  local kind file
  for kind in old new
  do
    [[ $(<"$CI_STATE/state.$kind") == "shut down" ]] \
      || fail "switch-back: the $kind cluster was not in the state 'shut down' when its files were read."
  done
  for file in "${CONFIG_FILES[@]}"
  do
    compare_whole "$file" "$CI_STATE/config.old.$file" "$CI_STATE/config.new.$file"
  done
  compare_whole "the output of restore-roles.sql" "$CI_STATE/roles.old" "$CI_STATE/roles.new"
}

switch_back() {
  # D9 step 6 and ADR-0103 (g). The wrong run comes first. The right run goes back to the old volume.
  local name old expected
  for name in change.log old.volume new.volume active.volume export.sha256 row-counts wal-target.dir \
    wal-target.manifest new.system-identifier row-counts.refuse
  do
    [[ -s $CI_STATE/$name ]] || fail "switch-back: run check and refuse first."
  done
  old=$(<"$CI_STATE/old.volume")
  [[ $old =~ ^[A-Za-z0-9_.-]+$ && $RECIPE_PGDATA_VOLUME != "$old" ]] \
    || fail "switch-back: the active volume is the old volume."
  expected=$(first_identifier "$CI_STATE/change.log")
  old_tag "$CI_STATE/change.log" >/dev/null
  first_timeline "$CI_STATE/change.log" >/dev/null
  log "switch-back: part 1, read the roles of the new cluster while it runs"
  capture_roles new
  run_wrong_switch_back
  log "switch-back: part 3, the switch back with the right identifier"
  switch_back_core "$CI_STATE/change.log" capture
  log "switch-back: part 3, read the roles of the old cluster while it runs"
  capture_roles old
  log "switch-back: part 4, start the new WAL line with a second target directory"
  WAL_SUFFIX=-back
  WAL_LABEL="switch-back wal-line"
  log "$WAL_LABEL: step 1, the switch back deleted the walarchive container"
  start_wal_line "$expected"
  check_first_target_stays
  log "switch-back: part 5, start app, then proxy and backup"
  compose up -d --no-deps --wait --wait-timeout 180 app
  compose up -d --no-deps --wait --wait-timeout 180 proxy backup
  log "switch-back: part 6, the data of the time before the switch"
  verify_export switch-back export-back.sha256
  # Login and export add audit entries on the old cluster, so audit_log may only grow. Every other table keeps its count.
  compare_counts "$CI_STATE/row-counts.back" switch-back
  log "switch-back: part 7, compare the configuration files and the roles of both clusters"
  compare_cluster_values
  require_volumes_stay
  printf '%s\n' "$old" >"$CI_STATE/active.volume"
  log "switch-back: done"
}

require_labelled_volume() {
  # The argument is a data volume. It must exist and must carry the label of this job, so that cleanup finds it.
  local label
  label=$(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$1" 2>/dev/null) \
    || fail "keep: the data volume $1 is gone."
  [[ $label == "$COMPOSE_PROJECT_NAME" ]] || fail "keep: the data volume $1 has no label of this job."
}

keep() {
  # D9 step 7 and ADR-0103 (a). The option -v of the project stop must leave both external data volumes.
  # This step deletes no volume. Only cleanup deletes volumes, through the helper of common.sh.
  local name old new left
  for name in old.volume new.volume active.volume
  do
    [[ -s $CI_STATE/$name ]] || fail "keep: run switch-back first."
  done
  old=$(<"$CI_STATE/old.volume")
  new=$(<"$CI_STATE/new.volume")
  [[ $RECIPE_PGDATA_VOLUME == "$old" && $new != "$old" ]] || fail "keep: run switch-back first."
  log "keep: stop the project and remove every container"
  compose --profile restore --profile pitr down -v --remove-orphans || fail "keep: the project stop failed."
  left=$(compose --profile restore --profile pitr ps -aq) || fail "keep: cannot list the containers of the project."
  [[ -z $left ]] || fail "keep: a container of the project still exists."
  log "keep: the active volume $RECIPE_PGDATA_VOLUME and the other data volume $new must exist"
  require_labelled_volume "$RECIPE_PGDATA_VOLUME"
  require_labelled_volume "$new"
  log "keep: done"
}

load_active_volume
case ${1:-} in
  prepare) prepare ;;
  restore) restore ;;
  switch) switch "restore volume completed" ;;
  access) access ;;
  wal-line) wal_line ;;
  check) check ;;
  refuse) refuse ;;
  switch-back) switch_back ;;
  keep) keep ;;
  cleanup) cleanup ;;
  *) fail "usage: restore-switch.sh prepare|restore|switch|access|wal-line|check|refuse|switch-back|keep|cleanup" ;;
esac
