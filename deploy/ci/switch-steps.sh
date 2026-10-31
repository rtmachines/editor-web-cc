#!/usr/bin/env bash
# Steps that the CI job scripts share (pitr-recovery design.md D7).
# A job script sources this file after common.sh. The file defines functions and defaults only.
# The file needs only the functions of common.sh: fail, compose, sql, container, and the others there.
# The caller can set WAL_LABEL and WAL_SUFFIX before it calls a step. The defaults suit the first WAL line.

# The line of restore-volume and the id of the system are numbers of at most 20 digits.
IDENTIFIER_PATTERN='^[0-9]{1,20}$'
# The label of the timeline in pg_controldata and in the change log (ADR-0106 (l)). A timeline is a number of at most 10 digits.
TIMELINE_LABEL="Latest checkpoint's TimeLineID"
TIMELINE_PATTERN='^[0-9]{1,10}$'
# The grammar of a segment file (ADR-0099 (d)), a base backup directory (ADR-0077 (d)), and a WAL position.
SEGMENT_REGEX='.*/[0-9A-F]{24}(\.partial)?'
BASE_REGEX='.*/base_[0-9]{8}_[0-9]{6}'
# The grammar of a timeline history file at the top of /wal (ADR-0106 (k), design.md D6). The name has 8 uppercase hex digits.
HISTORY_REGEX='.*/[0-9A-F]{8}\.history'
BASE_NAME_PATTERN='^base_[0-9]{8}_[0-9]{6}$'
LSN_PATTERN='^[0-9A-F]+/[0-9A-F]+$'
STREAM_ROW_PATTERN='^streaming\|[0-9A-F]+/[0-9A-F]+$'
# The state files of a WAL line end in this suffix. The first line has none. A switch back uses -back.
WAL_SUFFIX=${WAL_SUFFIX-}
# The step label of the log lines of a WAL line.
WAL_LABEL=${WAL_LABEL:-wal-line}
# A wait runs this many attempts with one second between them. The tests lower the number.
WAIT_ATTEMPTS=${CI_WAIT_ATTEMPTS:-60}

log() {
  printf 'CI: %s\n' "$1"
}

retry() {
  # Run a command until it succeeds or the attempts end.
  local attempt
  for ((attempt = 0; attempt < WAIT_ATTEMPTS; attempt++))
  do
    "$@" && return 0
    sleep 1
  done
  return 1
}

set_role_passwords() {
  # ADR-0103 (b) step 6 and ADR-0033. Psql reads each secret file inside db, so no password reaches an argument or a log.
  compose exec -T -u postgres db psql -v ON_ERROR_STOP=1 --no-password --no-psqlrc --dbname postgres <<'SQL'
\set postgres_password `cat /run/secrets/postgres_password`
\set migrator_password `cat /run/secrets/migrator_password`
\set app_password `cat /run/secrets/app_password`
\set backup_password `cat /run/secrets/backup_password`
\set replicator_password `cat /run/secrets/replicator_password`
SET log_statement = 'none';
SET log_min_error_statement = 'panic';
SET log_min_duration_statement = -1;
ALTER ROLE postgres PASSWORD :'postgres_password';
ALTER ROLE recipe_migrator PASSWORD :'migrator_password';
ALTER ROLE recipe_app PASSWORD :'app_password';
ALTER ROLE recipe_backup PASSWORD :'backup_password';
ALTER ROLE recipe_replicator PASSWORD :'replicator_password';
SQL
}

# The login roles of 01-roles.sh. The superuser postgres has its own check.
LOGIN_ROLES=(recipe_migrator recipe_app recipe_backup recipe_replicator)

# The step label of the error lines of expect_probe. The step access of restore-switch.sh keeps the default.
PROBE_LABEL=${PROBE_LABEL:-access}

# This script runs in a one-shot pgtools container on the network backend. The secret files are mounted in
# /ci-secrets. For each role it writes a pgpass file in the tmpfs, so no password reaches an argument.
# It prints one line "role|result" for each role. The result is ok, password, reject, or other.
read -r -d '' PROBE_SCRIPT <<'PROBE' || true
set -u
export PGCONNECT_TIMEOUT=5 PGSSLMODE=disable
dir=$(mktemp -d /run/pgpass/probe.XXXXXX)
trap 'rm -rf -- "$dir"' EXIT
for role
do
  file=/ci-secrets/${role#recipe_}_password
  ( umask 077; { printf '*:*:*:%s:' "$role"; sed -e 's/[\\:]/\\&/g' -- "$file"; } >"$dir/pgpass" )
  target="host=db port=5432 user=$role dbname=recipe"
  query='SELECT 1'
  if [[ $role == recipe_replicator ]]
  then
    target="host=db port=5432 user=$role dbname=replication replication=true"
    query=IDENTIFY_SYSTEM
  fi
  if error=$(PGPASSFILE=$dir/pgpass psql -X -w -At "$target" -c "$query" 2>&1 >/dev/null)
  then
    result=ok
  elif grep -Fq 'password authentication failed' <<<"$error"
  then
    result=password
  elif grep -Fq 'pg_hba.conf rejects connection' <<<"$error"
  then
    result=reject
  else
    result=other
  fi
  printf '%s|%s\n' "$role" "$result"
done
PROBE

probe_roles() {
  # Run the probe in a one-shot container of a pgtools service. The client has an address in the subnet of backend.
  local name args=(run --rm --no-deps -T)
  for name in postgres migrator app backup replicator
  do
    args+=(-v "$RECIPE_SECRETS_DIR/${name}_password:/ci-secrets/${name}_password:ro")
  done
  compose "${args[@]}" --entrypoint bash backup-premigrate -c "$PROBE_SCRIPT" probe "$@"
}

expect_probe() {
  # The first argument is the result that every role must get. The other arguments are the roles.
  local expected=$1 output role result seen=0 count
  shift
  count=$#
  output=$(probe_roles "$@") || fail "$PROBE_LABEL: the probe container failed."
  while IFS='|' read -r role result
  do
    [[ $role =~ ^(postgres|recipe_[a-z]+)$ ]] || continue
    [[ $result == "$expected" ]] || fail "$PROBE_LABEL: the login of $role gave '$result'. Expected '$expected'."
    seen=$((seen + 1))
  done <<<"$output"
  (( seen == count )) || fail "$PROBE_LABEL: the probe reported $seen of $count roles."
}

controldata() {
  # Read pg_controldata of a volume. The container has no network and mounts the volume read-only (ADR-0103 (f)).
  docker run --rm --network none --security-opt no-new-privileges:true -e LC_ALL=C \
    -v "$1:/var/lib/postgresql/data:ro" --entrypoint pg_controldata \
    "recipe-pgtools:${RECIPE_VERSION:?}" /var/lib/postgresql/data
}

controldata_field() {
  # Print the value of one line of pg_controldata. The first argument is the volume. The second is the label.
  local output
  output=$(controldata "$1") || fail "cannot read pg_controldata of the volume $1"
  sed -En "s/^$2: +//p" <<<"$output"
}

db_volume() {
  local name
  name=$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' "$(container db)")
  [[ -n $name ]] || fail "the database container has no data volume"
  printf '%s\n' "$name"
}

stop_services() {
  # ADR-0103 (f) step 1. Delete containers only. The mode kill ends db with SIGKILL and leaves an unclean cluster.
  # A recovery switch follows the step crash of pitr-recovery, which killed db already. Then docker kill fails on a dead container.
  # The kill is enough when db does not run afterwards. A kill that fails on a running db stops the run.
  if [[ $1 == kill ]]
  then
    compose stop proxy app backup walarchive
    docker kill "$(container db)" >/dev/null \
      || [[ $(docker inspect --format '{{.State.Running}}' "$(container db)") == false ]] \
      || fail "switch: cannot kill db."
  else
    compose stop proxy app backup walarchive db
  fi
  compose rm -f proxy app backup walarchive db
}

check_unclean_state() {
  # The CI proof that the else branch of ADR-0103 (f) step 2 runs. The first stop must leave a state other than shut down.
  local state
  state=$(controldata_field "$1" 'Database cluster state')
  if [[ $state == "shut down" ]]
  then
    fail "switch: the first stop left the state 'shut down'. The restart branch did not run."
  fi
  [[ -n $state ]] || fail "switch: pg_controldata shows no state of the old volume."
  printf '%s\n' "$state" >"$CI_STATE/switch.unclean-state"
  log "switch: the first stop left the state '$state'. db starts again."
}

restart_old_db() {
  # The old name stays in RECIPE_PGDATA_VOLUME. db replays its WAL and then runs.
  compose up -d --no-deps --wait --wait-timeout 180 db
}

check_new_volume() {
  # ADR-0103 (f) step 4. Stop without a deletion when the volume is not the one that the identifier names.
  # The optional arguments name the step and the source of the identifier. A switch back uses the change log.
  # The optional fifth argument is the timeline from the change log. Without it, the check compares the identifier only (ADR-0106 (l)).
  local state identifier timeline who=${3:-switch} source=${4:-the line of restore-volume}
  state=$(controldata_field "$1" 'Database cluster state')
  [[ $state == "shut down" ]] || fail "$who: the new volume is not in the state 'shut down'."
  identifier=$(controldata_field "$1" 'Database system identifier')
  [[ $identifier =~ $IDENTIFIER_PATTERN && $identifier == "$2" ]] \
    || fail "$who: the system identifier of the new volume differs from $source."
  if [[ -z ${5:-} ]]
  then
    return 0
  fi
  timeline=$(controldata_field "$1" "$TIMELINE_LABEL")
  [[ $timeline =~ $TIMELINE_PATTERN && $timeline == "$5" ]] \
    || fail "$who: the timeline of the new volume differs from $source."
}

change_log_entry() {
  # Print one entry of the change log (ADR-0105 (b), ADR-0106 (l)). The arguments are the volume, the tag, the identifier, and the timeline.
  printf 'volume: %s\ntag: %s\nDatabase system identifier: %s\n%s: %s\n' "$1" "$2" "$3" "$TIMELINE_LABEL" "$4"
}

require_compose_volume() {
  # ADR-0103 (f) step 3. The first argument is the step name. The second is the volume that Compose must show for pgdata.
  compose config --format json >"$CI_STATE/$1-config.json" || fail "$1: cannot read the Compose configuration."
  [[ $(python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_checks.py" volume-name "$CI_STATE/$1-config.json") == "$2" ]] \
    || fail "$1: Compose does not show the new volume for pgdata."
}

start_db() {
  # ADR-0103 (f) step 5. The first argument is the step name. The second is the volume that db must mount.
  compose up -d --no-deps --wait --wait-timeout 180 db
  [[ $(db_volume) == "$2" ]] || fail "$1: db does not use the new volume."
}

# The two lines that a switch accepts as its source (ADR-0103 (f), ADR-0106 (j)). The caller passes the message of the line.
SOURCE_RESTORE='restore volume completed'
SOURCE_PITR='pitr volume completed'
# The fields of the line "pitr volume completed" (ADR-0106 (d), (f) step 12). The timeline is a number or a string of digits.
PITR_MESSAGE_PATTERN='^\{.*"message":"pitr volume completed".*\}$'
PITR_IDENTIFIER_PATTERN='"system_identifier":"([0-9]{1,20})"'
PITR_TIMELINE_PATTERN='"timeline":"?([0-9]{1,10})"?[,}]'
PITR_DRILL_PATTERN='"drill":(true|false)[,}]'

read_pitr_line() {
  # Read the last line of the output of pitr-volume. The CI step that runs pitr-volume keeps the output in pitr-volume.log.
  # The function sets LINE_IDENTIFIER and LINE_TIMELINE. A line of a drill run stops the run before any container stops.
  # The optional argument is the label of the error messages. The default is switch.
  local last who=${1:-switch}
  last=$(tail -n 1 "$CI_STATE/pitr-volume.log")
  [[ $last =~ $PITR_MESSAGE_PATTERN ]] || fail "$who: the last line of pitr-volume is wrong."
  [[ $last =~ $PITR_IDENTIFIER_PATTERN ]] || fail "$who: the last line of pitr-volume is wrong."
  LINE_IDENTIFIER=${BASH_REMATCH[1]}
  [[ $last =~ $PITR_TIMELINE_PATTERN ]] || fail "$who: the last line of pitr-volume is wrong."
  LINE_TIMELINE=${BASH_REMATCH[1]}
  [[ $last =~ $PITR_DRILL_PATTERN ]] || fail "$who: the last line of pitr-volume is wrong."
  [[ ${BASH_REMATCH[1]} == false ]]     || fail "$who: the line of pitr-volume comes from a drill. A drill volume never becomes production."
}

read_unchecked_old_volume() {
  # ADR-0106 (j) step 2. The old volume needs no state. Set OLD_IDENTIFIER and OLD_TIMELINE to each value that
  # pg_controldata reads. Else the value is the word unreadable. The function never starts db on the volume.
  local output identifier="" timeline=""
  if output=$(controldata "$1" 2>/dev/null)
  then
    identifier=$(sed -En 's/^Database system identifier: +//p' <<<"$output")
    timeline=$(sed -En "s/^$TIMELINE_LABEL: +//p" <<<"$output")
  fi
  [[ $identifier =~ $IDENTIFIER_PATTERN ]] || identifier=unreadable
  [[ $timeline =~ $TIMELINE_PATTERN ]] || timeline=unreadable
  OLD_IDENTIFIER=$identifier
  OLD_TIMELINE=$timeline
}

refuse_drill_volume() {
  # ADR-0106 (j). A volume with the file pitr-drill never becomes production. The container has no network and mounts the volume read-only.
  # The command test exits 0 when the file exists and 1 when it does not. Another status means that the test did not run.
  local status=0
  docker run --rm --network none --security-opt no-new-privileges:true     -v "$1:/var/lib/postgresql/data:ro" --entrypoint test     "recipe-pgtools:${RECIPE_VERSION:?}" -e /var/lib/postgresql/data/pitr-drill || status=$?
  [[ $status -ne 0 ]] || fail "switch: the new volume holds the file pitr-drill. A drill volume never becomes production."
  [[ $status -eq 1 ]] || fail "switch: cannot test the new volume for the file pitr-drill."
}

switch() {
  # ADR-0103 (f) steps 1 to 5. The change log gets the old name, its tag, its identifier, and its timeline for the switch back.
  # The tag is the image tag of the job (ADR-0105 (b)). The argument is the message of the source line.
  # With restore volume completed, the steps stay as they are. With pitr volume completed, ADR-0106 (j) changes steps 2 and 4.
  local source=${1:-} old new expected expected_timeline="" where="the line of restore-volume" state identifier timeline
  case $source in
    "$SOURCE_RESTORE" | "$SOURCE_PITR") ;;
    *) fail "switch: the source line must be '$SOURCE_RESTORE' or '$SOURCE_PITR'." ;;
  esac
  if [[ $source == "$SOURCE_RESTORE" ]]
  then
    [[ -s $CI_STATE/new.volume && -s $CI_STATE/new.system-identifier ]] || fail "switch: run restore first."
    expected=$(<"$CI_STATE/new.system-identifier")
  else
    [[ -s $CI_STATE/new.volume && -s $CI_STATE/pitr-volume.log ]] || fail "switch: run pitr-volume first."
    read_pitr_line
    expected=$LINE_IDENTIFIER
    expected_timeline=$LINE_TIMELINE
    where="the line of pitr-volume"
  fi
  new=$(<"$CI_STATE/new.volume")
  old=$RECIPE_PGDATA_VOLUME
  log "switch: step 1, stop the services and kill db"
  stop_services kill
  log "switch: step 2, read the state of the old volume $old"
  if [[ $source == "$SOURCE_RESTORE" ]]
  then
    check_unclean_state "$old"
    restart_old_db
    stop_services stop
    state=$(controldata_field "$old" 'Database cluster state')
    [[ $state == "shut down" ]] || fail "switch: the old volume is not in the state 'shut down'."
    identifier=$(controldata_field "$old" 'Database system identifier')
    [[ $identifier =~ $IDENTIFIER_PATTERN ]] || fail "switch: the system identifier of the old volume is invalid."
    timeline=$(controldata_field "$old" "$TIMELINE_LABEL")
    [[ $timeline =~ $TIMELINE_PATTERN ]] || fail "switch: the timeline of the old volume is invalid."
  else
    read_unchecked_old_volume "$old"
    identifier=$OLD_IDENTIFIER
    timeline=$OLD_TIMELINE
  fi
  printf '%s
' "$old" >"$CI_STATE/old.volume"
  change_log_entry "$old" "${RECIPE_VERSION:?}" "$identifier" "$timeline" >"$CI_STATE/change.log"
  export RECIPE_PGDATA_VOLUME=$new
  log "switch: step 3, Compose shows the volume $new"
  require_compose_volume switch "$new"
  log "switch: step 4, read the state of the new volume"
  if [[ $source == "$SOURCE_RESTORE" ]]
  then
    check_new_volume "$new" "$expected"
  else
    check_new_volume "$new" "$expected" switch "$where" "$expected_timeline"
    refuse_drill_volume "$new"
  fi
  log "switch: step 5, start db on the new volume"
  start_db switch "$new"
  printf '%s
' "$new" >"$CI_STATE/active.volume"
  log "switch: done"
}

archive_segments() {
  # Print the names of the segment files and the .partial file at the top of the archive directory.
  sudo find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f -regextype posix-extended -regex "$SEGMENT_REGEX" -printf '%f\n' | sort
}

archive_history() {
  # Print the names of the timeline history files at the top of the archive directory. Another name does not match.
  sudo find "$RECIPE_WALARCHIVE_DIR" -maxdepth 1 -type f -regextype posix-extended -regex "$HISTORY_REGEX" -printf '%f\n' | sort
}

archive_bases() {
  # Print the names of the base backup directories in /wal/base. A .tmp-base directory has another name.
  # basebackup-once creates /wal/base at its first run. Before that run, the list is empty.
  if ! sudo test -e "$RECIPE_WALARCHIVE_DIR/base" && ! sudo test -L "$RECIPE_WALARCHIVE_DIR/base"
  then
    return 0
  fi
  sudo find "$RECIPE_WALARCHIVE_DIR/base" -mindepth 1 -maxdepth 1 -type d -regextype posix-extended -regex "$BASE_REGEX" -printf '%f\n' | sort
}

target_name_free() {
  # The stamp is the time in seconds. A second line in the same second would reuse the name of the first target.
  WAL_TARGET="${RECIPE_WALARCHIVE_DIR%/}.moved-$(date -u +%Y%m%d_%H%M%S)"
  ! sudo test -e "$WAL_TARGET"
}

make_target() {
  # ADR-0103 (s) step 2 and ADR-0099 (c). The target is a sibling of the archive directory. mkdir fails on an existing name.
  # A target directory of an earlier line stays, so the stamp must be new.
  retry target_name_free || fail "wal-line: cannot find an unused name for the target directory."
  sudo mkdir -m 0700 -- "$WAL_TARGET" || fail "wal-line: cannot create the target directory."
  # ADR-0099 (a) step 4. The CI WAL disk holds the archive directory, so the sibling target is on the same disk.
  [[ $(sudo stat -c %d "$RECIPE_WALARCHIVE_DIR") == "$(sudo stat -c %d "$WAL_TARGET")" ]] \
    || fail "wal-line: the target directory is on another filesystem than the archive directory."
}

expected_old_identifier() {
  # ADR-0103 (s) step 3 and ADR-0106 (k). Print the content of the file old-system-identifier for the change log.
  # The first argument is the label of the error messages. Both lines come from the last entry only.
  # The content is the identifier line, then the timeline line. A legacy entry has no timeline line (ADR-0106 (l)).
  # A numeric identifier with the word unreadable as timeline adds no timeline line (ADR-0106 (j), (l)).
  # The restore switch reads the old volume after a clean stop, so it never writes unreadable. Its path stays as it was.
  # An identifier of the word unreadable comes from a recovery. A recovery keeps the identifier.
  # Then the line of pitr-volume gives the identifier only (ADR-0106 (j), (p)).
  # Its timeline belongs to the new cluster. The old cluster has no readable timeline, so the file holds no timeline line (ADR-0106 (k), (l)).
  local entry line timeline_line timeline
  entry=$(awk '/^volume: / { entry = "" } { entry = entry $0 "\n" } END { printf "%s", entry }' "$CI_STATE/change.log")
  line=$(sed -En 's/^(Database system identifier: [0-9]{1,20})$/\1/p' <<<"$entry" | tail -n 1)
  timeline_line=$(awk -v label="$TIMELINE_LABEL:" 'index($0, label) == 1 { found = $0 } END { if (found != "") print found }' <<<"$entry")
  if [[ -z $line ]]
  then
    grep -qxF 'Database system identifier: unreadable' <<<"$entry" \
      || fail "$1: the change log holds no identifier line."
    read_pitr_line "$1"
    line="Database system identifier: $LINE_IDENTIFIER"
    timeline_line=
  fi
  timeline=${timeline_line#"$TIMELINE_LABEL: "}
  if [[ -n $timeline_line && $timeline != unreadable ]]
  then
    [[ $timeline =~ $TIMELINE_PATTERN ]] || fail "$1: the timeline in the change log is not valid."
    line=$line$'\n'$timeline_line
  fi
  printf '%s\n' "$line"
}

write_old_identifier() {
  # ADR-0103 (s) step 3. Write the expected content into the target directory.
  local content
  # expected_old_identifier writes the error line. The exit keeps its status.
  content=$(expected_old_identifier wal-line) || exit "$?"
  printf '%s\n' "$content" | sudo tee -- "$WAL_TARGET/old-system-identifier" >/dev/null
}

require_old_identity_file() {
  # The file in the archive names the old cluster. A difference stops the run before any move.
  local content old_id file_id
  # The identifier is the first line of the expected content of old-system-identifier, so both checks use the last entry.
  # expected_old_identifier writes the error line. The exit keeps its status.
  content=$(expected_old_identifier wal-line) || exit "$?"
  old_id=${content%%$'\n'*}
  old_id=${old_id#Database system identifier: }
  file_id=$(sudo cat "$RECIPE_WALARCHIVE_DIR/.system-identifier") \
    || fail "wal-line: the archive has no file .system-identifier."
  [[ $file_id == "$old_id" ]] \
    || fail "wal-line: the file .system-identifier does not hold the identifier of the old cluster."
}

move_segments() {
  # ADR-0103 (s) step 4 and ADR-0099 (a) steps 5 to 7. Move the segments, the .partial file, and .system-identifier.
  # ADR-0106 (k). The move also takes each timeline history file.
  local listing history name paths=()
  listing=$(archive_segments) || fail "wal-line: cannot list the segment files."
  history=$(archive_history) || fail "wal-line: cannot list the history files."
  while IFS= read -r name
  do
    [[ -z $name ]] || paths+=("$RECIPE_WALARCHIVE_DIR/$name")
  done <<<"$listing"$'\n'"$history"
  paths+=("$RECIPE_WALARCHIVE_DIR/.system-identifier")
  sudo mv -t "$WAL_TARGET" -- "${paths[@]}" || fail "wal-line: cannot move the segment files."
  [[ -z $(archive_segments) ]] || fail "wal-line: a segment file stays in the archive directory."
  [[ -z $(archive_history) ]] || fail "wal-line: a history file stays in the archive directory."
  if sudo test -e "$RECIPE_WALARCHIVE_DIR/.system-identifier"
  then
    fail "wal-line: the file .system-identifier stays in the archive directory."
  fi
}

move_bases() {
  # ADR-0103 (s) step 5. Move each base backup of the old line into the directory base of the target.
  local listing name paths=()
  listing=$(archive_bases) || fail "wal-line: cannot list /wal/base."
  { [[ -z $listing ]] || printf '%s\n' "$listing"; } >"$CI_STATE/wal-old$WAL_SUFFIX.bases"
  sudo mkdir -m 0700 -- "$WAL_TARGET/base" || fail "wal-line: cannot create the directory base in the target."
  while IFS= read -r name
  do
    [[ -z $name ]] || paths+=("$RECIPE_WALARCHIVE_DIR/base/$name")
  done <<<"$listing"
  if (( ${#paths[@]} > 0 ))
  then
    sudo mv -t "$WAL_TARGET/base" -- "${paths[@]}" || fail "wal-line: cannot move the base backups."
  fi
  [[ -z $(archive_bases) ]] || fail "wal-line: a base backup of the old line stays in /wal/base."
}

record_target() {
  # The step check proves later that the old volume and every moved file stay.
  printf '%s\n' "$WAL_TARGET" >"$CI_STATE/wal-target$WAL_SUFFIX.dir"
  sudo find "$WAL_TARGET" -mindepth 1 | sort >"$CI_STATE/wal-target$WAL_SUFFIX.manifest"
}

stream_ready() {
  # ADR-0099 (a1). The result is exactly one row with the state streaming.
  local row
  row=$(sql -c "SELECT state, flush_lsn FROM pg_stat_replication WHERE usename = 'recipe_replicator' AND application_name = 'walarchive'") \
    || return 1
  [[ $row =~ $STREAM_ROW_PATTERN ]]
}

flush_reached() {
  # ADR-0099 (a2) and ADR-0077 (c). The first argument is the WAL position.
  local reached
  reached=$(sql -c "SELECT EXISTS (SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '$1'::pg_lsn)") \
    || return 1
  [[ $reached == t ]]
}

take_base_backup() {
  # The first argument is the log file. The second is the step name. The function sets LAST_BASE.
  compose exec -T walarchive basebackup-once | tee "$CI_STATE/$1" || fail "$2: basebackup-once failed."
  LAST_BASE=$(sed -En 's/.*"message":"base backup completed".*"base":"(base_[0-9]{8}_[0-9]{6})".*/\1/p' "$CI_STATE/$1" | tail -n 1)
  [[ $LAST_BASE =~ $BASE_NAME_PATTERN ]] || fail "$2: basebackup-once did not report a base backup."
}

wal_check() {
  # ADR-0099 (a3) and ADR-0107. The first argument is the base backup. The second is the step name.
  compose exec -T walarchive pg_verifybackup --quiet --skip-checksums --ignore=backup_end_time --wal-directory=/wal "/wal/base/$1" \
    || fail "$2: the WAL check failed."
}

readyz_ok() {
  [[ $(compose exec -T proxy wget -q -T 3 -O - http://app:9000/readyz) == ok ]]
}

check_readyz() {
  retry readyz_ok || fail "${1:-check}: /readyz did not answer ok."
}

check_pitr() {
  # The jobs restore-switch and pitr-recovery run this check in their step check.
  # pitr-check keeps its gate PITR_CI=1 (ADR-0077). It recovers from the newest base backup of the new line.
  compose run --rm --no-deps -T -e PITR_CI=1 -e PITR_MARKER_SEQ -e PITR_MARKER_HASH pitr-check \
    || fail "check: pitr-check failed."
}

require_new_identity_file() {
  # The first argument is the identifier of the cluster that the new line streams.
  local file_id
  file_id=$(sudo cat "$RECIPE_WALARCHIVE_DIR/.system-identifier" 2>/dev/null) || file_id=""
  [[ $file_id == "$1" ]] \
    || fail "wal-line: the file .system-identifier of the new line is missing or wrong."
}

start_wal_line() {
  # ADR-0103 (s) steps 1 to 7. The first argument is the identifier of the cluster that the new line streams.
  [[ -z $(compose ps -aq walarchive) ]] || fail "$WAL_LABEL: the walarchive container exists. The switch must delete it."
  log "$WAL_LABEL: step 2, create the target directory"
  make_target
  log "$WAL_LABEL: step 3, copy the old system identifier into the target directory"
  write_old_identifier
  log "$WAL_LABEL: step 4, move the segment files, the .partial file, and the file .system-identifier"
  require_old_identity_file
  move_segments
  log "$WAL_LABEL: step 5, move the base backups of the old line"
  move_bases
  record_target
  log "$WAL_LABEL: step 6, start walarchive on the new line"
  compose up -d --no-deps walarchive
  log "$WAL_LABEL: step 7, stream check, base backup, segment switch, flush check, and WAL check"
  new_line_checks "$1"
}

new_line_checks() {
  # ADR-0103 (s) step 7 and ADR-0099 (a) steps 9 to 13. The first argument is the identifier of the new line.
  local position
  retry stream_ready || fail "wal-line: the stream check did not find one streaming row."
  take_base_backup "wal-line-base$WAL_SUFFIX.log" wal-line
  position=$(sql -c 'SELECT pg_switch_wal()')
  [[ $position =~ $LSN_PATTERN ]] || fail "wal-line: the segment switch returned no position."
  retry flush_reached "$position" || fail "wal-line: the flush check did not return t."
  wal_check "$LAST_BASE" wal-line
  require_new_identity_file "$1"
  printf '%s\n' "$LAST_BASE" >"$CI_STATE/wal-new$WAL_SUFFIX.base"
}

load_active_volume() {
  # A step that follows switch uses the volume that switch made active. Every step is a new process.
  local name
  [[ -s $CI_STATE/active.volume ]] || return 0
  name=$(<"$CI_STATE/active.volume")
  [[ $name =~ ^[A-Za-z0-9_.-]+$ ]] || fail "the active volume name is invalid"
  export RECIPE_PGDATA_VOLUME=$name
}

require_no_labelled_volume() {
  # Run this after delete_ci_volumes. No volume with the label of this job may stay. That includes every data volume.
  local left
  left=$(docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME") || return 1
  [[ -z $left ]] && return 0
  printf 'CI: cleanup: a volume with the label recipe.ci=%s stays: %s\n' "$COMPOSE_PROJECT_NAME" "${left//$'\n'/ }" >&2
  return 1
}

cleanup() {
  # Delete only this job's project and volumes. Compose never deletes the external pgdata volume.
  # delete_ci_volumes removes only the volumes that carry the label of this job.
  # The step crash of pitr-recovery kills db. The command down handles a dead container.
  # Both job scripts use the profiles restore and pitr, so one function serves both.
  local status=0 disk
  compose --profile restore --profile pitr down --remove-orphans || status=$?
  delete_ci_volumes || status=$?
  require_no_labelled_volume || status=$?
  for disk in backups walarchive-disk
  do
    if mountpoint -q "$CI_ROOT/$disk"
    then
      sudo umount "$CI_ROOT/$disk" || status=$?
    fi
  done
  return "$status"
}
