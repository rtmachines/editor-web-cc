#!/usr/bin/env bash
# Task 4.4 and ADR-0097 (h). Run only on the disposable GitHub runner.
# Upgrade step 1: this change requires the manual superuser step of ADR-0097.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/upgrade-common.sh"

previous_commit() {
  local ref base head
  ref=$(upgrade_check previous-ref "${UPGRADE_FROM:-}" "${GITHUB_EVENT_NAME:-}" "${UPGRADE_BEFORE:-}") || return 1
  if [[ -z $ref ]]
  then
    ref=$(git merge-base origin/main HEAD) \
      || fail "cannot find the merge base with origin/main. Stop and report."
  fi
  base=$(git rev-parse --verify --end-of-options "$ref^{commit}" 2>/dev/null) \
    || fail "cannot resolve the previous ref to a commit. Stop and report."
  head=$(git rev-parse --verify HEAD) \
    || fail "cannot resolve HEAD to a commit. Stop and report."
  upgrade_check previous-commit "$base" "$head" || return 1
  printf '%s\n' "$base"
}

build_previous() {
  local base
  base=$(previous_commit) || return 1
  # The shared umask is 077. The postgres user of the db container must read the mounted init script.
  (umask 022 && git worktree add --detach "$UPGRADE_PREVIOUS_DIR" "$base") \
    || fail "cannot create the previous worktree. Stop and report."
  printf 'Previous release: %s\n' "$base"
  if ! docker buildx build --load --tag recipe-server:ci-previous \
      --file "$UPGRADE_PREVIOUS_DIR/deploy/Dockerfile" "$UPGRADE_PREVIOUS_DIR" \
    || ! docker buildx build --load --tag recipe-pgtools:ci-previous \
      --build-context db="$UPGRADE_PREVIOUS_DIR/deploy/db" \
      --file "$UPGRADE_PREVIOUS_DIR/deploy/pgtools/Dockerfile" "$UPGRADE_PREVIOUS_DIR/deploy/pgtools"
  then
    fail "cannot build the previous images. Stop and report."
  fi
}

previous_pgdata_is_external() {
  # Release v0.1.0 declares pgdata without external. Its stack then creates the volume itself.
  grep -A1 -E '^  pgdata:' "$UPGRADE_PREVIOUS_DIR/deploy/compose.yaml" \
    | grep -Eq '^[[:space:]]+external:[[:space:]]*true'
}

start_previous() {
  export RECIPE_VERSION=ci-previous
  UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR
  check_images
  # An external pgdata needs the volume before up. The name stays the same for both releases.
  if previous_pgdata_is_external
  then
    create_pgdata_volume
  fi
  compose up -d --wait --wait-timeout 180 db
  [[ $(db_volume) == "$RECIPE_PGDATA_VOLUME" ]] || fail "the previous database does not use the volume of the upgrade"
  compose up -d backup-premigrate
  wait_job backup-premigrate
  compose up -d --no-deps migrate
  wait_job migrate
  # Bootstrap before app starts. No second app container runs beside a serving app.
  compose run --rm --no-deps -T \
    -v "$RECIPE_SECRETS_DIR/admin_password:/run/secrets/admin_password:ro" \
    app create-admin --login ci-admin --password-file /run/secrets/admin_password
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_smoke.py" seed | sql
  compose up -d --no-deps --wait --wait-timeout 180 app backup walarchive proxy
  check_image app recipe-server:ci-previous
  check_image backup recipe-pgtools:ci-previous
  container app >"$CI_STATE/old-app.id"
  backup_now old-now
  # The restore test of upgrade step 3 runs in upgrade(), in a new container of the previous image.
}

probe_previous_manifest() {
  # ADR-0174 (aa). Probe without a key or a mount from either release.
  PREVIOUS_MANIFEST_PROBE=$(docker run --rm --network none --entrypoint sh recipe-pgtools:ci-previous \
    -c 'if test -f /etc/pgtools/manifest-tag.pl; then printf "check\n"; else printf "no-check\n"; fi') \
    || fail "cannot probe the previous manifest check"
  upgrade_check manifest-plan "$PREVIOUS_MANIFEST_PROBE" >/dev/null \
    || fail "the previous manifest probe has no valid plan"
  if [[ $PREVIOUS_MANIFEST_PROBE == check ]]
  then
    # The CI-only service stays keyless when start_previous uses the previous tree.
    printf 'services:\n  upgrade-rollback:\n    secrets:\n      - backup_manifest_key\n' \
      >"$CI_STATE/rollback-key.compose.yaml" || fail "cannot write the rollback key mount"
  fi
  printf 'CI: previous manifest probe: %s.\n' "$PREVIOUS_MANIFEST_PROBE"
}

previous_compose() {
  # Use the plan only with the new tree. Never merge its rollback file into the previous tree.
  local plan expected file
  local -a files options=()
  [[ $UPGRADE_COMPOSE_DIR == "$GITHUB_WORKSPACE" ]] || fail "the previous image plan requires the new Compose tree"
  plan=$(upgrade_check manifest-plan "$PREVIOUS_MANIFEST_PROBE") || fail "cannot select the previous image plan"
  read -r -a files <<<"$plan" || fail "cannot read the previous image plan"
  expected=${files[0]}
  for file in "${files[@]:1}"
  do
    # compose() already loads compose.yaml before the CI file. Keep that merge order.
    [[ $file == compose.yaml ]] && continue
    options+=(-f "$UPGRADE_COMPOSE_DIR/deploy/$file")
  done
  if [[ $expected == key ]]
  then
    options+=(-f "$CI_STATE/rollback-key.compose.yaml")
  fi
  RECIPE_VERSION=ci-previous compose "${options[@]}" "$@"
}

check_previous_manifest_key() {
  local plan expected actual mounted
  plan=$(upgrade_check manifest-plan "$PREVIOUS_MANIFEST_PROBE") || fail "cannot read the expected manifest key state"
  read -r expected _ <<<"$plan"
  # Without its profile, the Compose output omits restore-volume.
  previous_compose --profile restore config --format json >"$CI_STATE/previous-manifest-config.json" \
    || fail "cannot read the previous image Compose configuration"
  actual=$(upgrade_check manifest-key "$CI_STATE/previous-manifest-config.json") \
    || fail "cannot check the previous image manifest key state"
  [[ $actual == "$expected" ]] || fail "the previous image configuration has the wrong manifest key state"
  mounted=$(previous_compose run --rm --no-deps -T --entrypoint sh backup \
    -c 'if test -f /run/secrets/backup_manifest_key; then printf "key\n"; else printf "no-key\n"; fi') \
    || fail "cannot check the previous container manifest key state"
  [[ $mounted == "$expected" ]] || fail "the previous container has the wrong manifest key state"
  printf 'CI: previous manifest key state: %s.\n' "$mounted"
}

replace_guard() {
  # Upgrade steps 13 and 14. Extract the statement from the new init script.
  upgrade_check guard "$GITHUB_WORKSPACE/deploy/db/initdb/01-roles.sh" >"$CI_STATE/guard.sql"
  # Upgrade step 15. Compute the hash in the same superuser session.
  cat >>"$CI_STATE/guard.sql" <<'SQL'
SET search_path = pg_catalog;
SELECT encode(sha256(convert_to(pg_get_functiondef('audit_guard.audit_objects_dropped()'::regprocedure), 'UTF8')), 'hex');
SQL
  local actual expected
  actual=$(compose exec -T -u postgres db psql -X -qAt -v ON_ERROR_STOP=1 -d recipe <"$CI_STATE/guard.sql")
  # Upgrade steps 16 and 17. EOF closes psql before the comparison.
  expected=$(grep -F 'FUNCTION|audit_guard audit_objects_dropped()|' \
    "$GITHUB_WORKSPACE/deploy/pgtools/restore-code.expected" | cut -d'|' -f4) \
    || fail "cannot read the expected guard SHA-256"
  [[ $expected =~ ^[0-9a-f]{64}$ && $actual == "$expected" ]] \
    || fail "the guard SHA-256 differs from the new restore-code.expected"
  printf 'The guard SHA-256 matches the new image.\n'
}

check_smoke() {
  # Upgrade steps 21 and 22. The direct bridge route gives a known runner address.
  docker cp "$(container proxy):/data/caddy/pki/authorities/local/root.crt" "$CI_STATE/root.crt"
  UPGRADE_CLIENT_IP=$(ip -json route get "$PROXY_ADDRESS" | python3 -I -c 'import json, sys; print(json.load(sys.stdin)[0]["prefsrc"])')
  export UPGRADE_CLIENT_IP
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_smoke.py" check
}

db_volume() {
  local name
  name=$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/var/lib/postgresql/data"}}{{.Name}}{{end}}{{end}}' "$(container db)")
  [[ -n $name ]] || fail "the database container has no data volume"
  printf '%s\n' "$name"
}

db_system_identifier() {
  local id
  id=$(sql -c 'SELECT system_identifier FROM pg_control_system()')
  [[ $id =~ ^[0-9]+$ ]] || fail "cannot read the system identifier of the database"
  printf '%s\n' "$id"
}

# Upgrade step 2 and ADR-0105 (j). The new stack must use an external pgdata volume with the data volume of the running db.
# Each message is one of the three cases of the name check.
VOLUME_EXTERNAL_MESSAGE="the pgdata volume of the new stack is not external. Stop and report."
VOLUME_VARIABLE_MESSAGE="the pgdata volume name of the new stack differs from RECIPE_PGDATA_VOLUME. Stop and report."
VOLUME_NAME_MESSAGE="the pgdata volume of the new stack differs from the data volume of the running db. Stop and report."

check_volume_name() {
  # Every command here has its own fail line. A mutation runs this function in a condition, where set -e is off.
  # The awk command is the one of the runbook, section Upgrade. It prints the fields name and external of pgdata.
  local config fields name actual
  config=$(compose config) || fail "cannot read the Compose configuration of the new stack"
  fields=$(awk '/^volumes:/{v=1;next} /^[^ ]/{v=0} v&&/^  pgdata:/{p=1;next} v&&/^  [^ ]/{p=0} p&&/^    (name|external):/{print $1, $2}' <<<"$config") \
    || fail "cannot read the pgdata fields of the new stack"
  # Case 1. The output holds no line external: true.
  grep -Fxq 'external: true' <<<"$fields" || fail "$VOLUME_EXTERNAL_MESSAGE"
  # Case 2. The name differs from RECIPE_PGDATA_VOLUME.
  name=$(awk '$1 == "name:" {print $2}' <<<"$fields") || fail "cannot read the pgdata volume name of the new stack"
  [[ $name == "$RECIPE_PGDATA_VOLUME" ]] || fail "$VOLUME_VARIABLE_MESSAGE"
  # Case 3. The name differs from the data volume of the running db.
  actual=$(db_volume) || fail "cannot read the data volume of the running db"
  [[ $name == "$actual" ]] || fail "$VOLUME_NAME_MESSAGE"
}

check_volume_name_mutation() {
  # A wrong existing volume must stop the upgrade before any Compose command starts a container of the new stack.
  local wrong=${RECIPE_PGDATA_VOLUME}_wrong db_before
  db_before=$(container db)
  # The label lets delete_ci_volumes remove this empty volume. The mutation never mounts it.
  docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$wrong" >/dev/null
  if (RECIPE_PGDATA_VOLUME=$wrong; check_volume_name) 2>"$CI_STATE/volume-mutation.log"
  then
    fail "the name check did not stop the upgrade for a wrong existing volume"
  fi
  grep -Fxq "CI: $VOLUME_NAME_MESSAGE" "$CI_STATE/volume-mutation.log" \
    || fail "the name check stopped the upgrade for another reason"
  [[ $(container db) == "$db_before" ]] || fail "the name check mutation replaced the database container"
  [[ $(db_volume) == "$(cat "$CI_STATE/db.volume")" ]] \
    || fail "the name check mutation changed the data volume of the database"
  printf 'CI: the name check stopped the upgrade for the wrong volume %s.\n' "$wrong"
}

check_volume_external_mutation() {
  # Design D13 and ADR-0105 (j) case 1. The name check runs on a copy of the previous deploy directory.
  # The copy has no external: true for pgdata, as the files of 0.1.0 have none. It stays in CI_STATE.
  local copy=$CI_STATE/previous-no-external db_before
  db_before=$(container db)
  rm -rf -- "$copy"
  mkdir -p -- "$copy" && cp -a -- "$UPGRADE_PREVIOUS_DIR/deploy" "$copy/deploy" \
    || fail "cannot copy the previous deploy directory"
  sed -i -E '/^  pgdata:/,/^ {0,2}[^ ]/{/^    external:/d}' "$copy/deploy/compose.yaml" \
    || fail "cannot remove external: true from the copy"
  if (UPGRADE_COMPOSE_DIR=$copy; check_volume_name) 2>"$CI_STATE/external-mutation.log"
  then
    fail "the name check did not stop the upgrade for a pgdata volume that is not external"
  fi
  grep -Fxq "CI: $VOLUME_EXTERNAL_MESSAGE" "$CI_STATE/external-mutation.log" \
    || fail "the name check stopped the upgrade for another reason than a pgdata volume that is not external"
  [[ $(container db) == "$db_before" ]] || fail "the name check mutation replaced the database container"
  [[ $(db_volume) == "$(cat "$CI_STATE/db.volume")" ]] \
    || fail "the name check mutation changed the data volume of the database"
  printf 'CI: the name check stopped the upgrade for a pgdata volume that is not external.\n'
}

# Design D8 and ADR-0104 (b). This is the step of the operator. It runs in a new walarchive container, which reads the id on stdin.
# The new walarchive refuses old segments when the file is missing. A release with the guard has the file already.
# The container uses the image of the previous release. The step uses only bash, coreutils, and grep.
WAL_IDENTITY_STEP=$(cat <<'STEP'
set -euo pipefail
umask 077
export LC_ALL=C
dir=${WAL_DIR:-/wal}
file=$dir/.system-identifier
IFS= read -r id
[[ $id =~ ^[0-9]+$ ]] || { printf 'ERROR: the system identifier of db is not a number.\n' >&2; exit 1; }
if [[ -e $file || -L $file ]]; then
  printf 'The segment check does not run. The file .system-identifier exists.\n'
  exit 0
fi
# A complete segment has 24 upper-case hex characters and no suffix. LC_ALL=C sorts the names by age.
segment=$(ls -1 -- "$dir" | { grep -E '^[0-9A-F]{24}$' || true; } | sort | tail -n 1)
# Without a complete segment, the step reads the newest .partial file (ADR-0105 (h)).
if [[ -z $segment ]]; then
  segment=$(ls -1 -- "$dir" | { grep -E '^[0-9A-F]{24}\.partial$' || true; } | sort | tail -n 1)
fi
[[ -n $segment ]] || { printf 'ERROR: the WAL archive holds no complete segment and no .partial file.\n' >&2; exit 1; }
# The long header of the first page holds xlp_info at offset 2 and xlp_sysid at offset 24 (little-endian).
read -r info < <(od -An -t u2 --endian=little -j 2 -N 2 -- "$dir/$segment")
read -r sysid < <(od -An -t u8 --endian=little -j 24 -N 8 -- "$dir/$segment")
[[ $info =~ ^[0-9]+$ && $sysid =~ ^[0-9]+$ ]] \
  || { printf 'ERROR: cannot read the header of the WAL segment %s.\n' "$segment" >&2; exit 1; }
(( info & 2 )) || { printf 'ERROR: the first page of the WAL segment %s has no long header.\n' "$segment" >&2; exit 1; }
[[ $sysid == "$id" ]] \
  || { printf 'ERROR: the header of the WAL segment %s holds another system identifier.\n' "$segment" >&2; exit 1; }
printf 'The segment check passed. The segment %s holds the system identifier of db.\n' "$segment"
temp=$(mktemp "$dir/.system-identifier.tmp.XXXXXX")
cannot_publish() { rm -f -- "$temp"; printf 'ERROR: cannot publish the file .system-identifier.\n' >&2; exit 1; }
printf '%s\n' "$id" >"$temp"
sync -- "$temp" || cannot_publish
ln -T -- "$temp" "$file" || cannot_publish
rm -f -- "$temp"
sync -- "$dir" || { printf 'ERROR: the file .system-identifier exists, but the sync of the directory failed.\n' >&2; exit 1; }
STEP
)

# ADR-0104 (e). The host waits at most this many seconds for the old walarchive to close one WAL segment.
WAL_SEGMENT_WAIT_SECONDS=60

write_wal_identity() {
  local id file=$RECIPE_WALARCHIVE_DIR/.system-identifier attempt names mode=ran
  id=$(db_system_identifier)
  # Close the current segment while the old walarchive streams. The old stream then leaves a complete segment.
  sql -c 'SELECT pg_switch_wal()' >/dev/null
  # The host user cannot read the directory of the archive. Wait for one complete segment, not a .partial file.
  for ((attempt = 0; attempt < WAL_SEGMENT_WAIT_SECONDS; attempt++))
  do
    names=$(sudo ls -1 -- "$RECIPE_WALARCHIVE_DIR") || fail "cannot list the WAL archive directory"
    if grep -Eq '^[0-9A-F]{24}$' <<<"$names"
    then
      break
    fi
    sleep 1
  done
  grep -Eq '^[0-9A-F]{24}$' <<<"$names" \
    || fail "the old walarchive wrote no complete WAL segment within $WAL_SEGMENT_WAIT_SECONDS seconds. Stop and report."
  # The step prints one line. The job checks it against the state of the file before the step.
  if sudo test -e "$file" || sudo test -L "$file"
  then
    mode=skipped
  fi
  # The step runs in a new container with the previous tag. The job exported the new tag before.
  printf '%s\n' "$id" \
    | previous_compose run --rm --no-deps -T --entrypoint sh walarchive -c 'exec bash -c "$1"' sh "$WAL_IDENTITY_STEP" \
    | tee "$CI_STATE/wal-identity.log" \
    || fail "the segment check of the existing installation failed. Stop and report."
  upgrade_check wal-identity "$CI_STATE/wal-identity.log" "$mode"
  [[ $(sudo cat -- "$file") == "$(db_system_identifier)" ]] \
    || fail "the file .system-identifier does not hold the identifier of db"
  printf 'CI: the file .system-identifier holds the system identifier of db.\n'
}

check_backup_without_key() {
  check_image backup recipe-pgtools:ci-previous
  compose exec -T backup sh -c 'test ! -e /run/secrets/backup_manifest_key && test ! -L /run/secrets/backup_manifest_key' \
    || fail "the previous backup container has a manifest key"
}

check_previous_backup_schedule() {
  local id=$1 environment mount schedule
  check_backup_without_key
  environment=$(docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$id") \
    || fail "cannot inspect the previous backup environment"
  grep -Fxq 'BACKUP_CRONTAB=/etc/pgtools/crontab.previous' <<<"$environment" \
    || fail "the previous backup has the wrong crontab path"
  mount=$(docker inspect --format '{{range .Mounts}}{{if eq .Destination "/etc/pgtools/crontab.previous"}}{{.Source}}|{{.Destination}}|{{.RW}}{{end}}{{end}}' "$id") \
    || fail "cannot inspect the previous backup crontab mount"
  [[ $mount == "$GITHUB_WORKSPACE/deploy/crontab.previous|/etc/pgtools/crontab.previous|false" ]] \
    || fail "the previous backup crontab is not the read-only file of the new tree"
  schedule=$(compose exec -T backup cat /etc/pgtools/crontab.previous) \
    || fail "cannot read the previous backup crontab"
  printf '%s\n' "$schedule"
  grep -Fxq '0 2 * * * backup-once' <<<"$schedule" || fail "the previous backup crontab has no daily dump"
  if grep -Ev '^[[:space:]]*(#|$)' <<<"$schedule" | grep -Fq restore-test
  then
    fail "the previous backup crontab runs restore-test"
  fi
  printf 'CI: the previous backup uses the read-only crontab.previous and has no manifest key.\n'
}

check_early_upgrade_failure() {
  local before status=0
  before=$(container backup)
  (
    unset UPGRADE_STEP3_STARTED
    trap restore_tag_on_failure EXIT
    printf 'CI: force an upgrade failure before step 3.\n' >&2
    false
  ) >"$CI_STATE/early-upgrade-failure.log" 2>&1 || status=$?
  (( status != 0 )) || fail "the failure before step 3 returned success"
  [[ $(container backup) == "$before" ]] || fail "the failure before step 3 replaced the backup container"
  printf 'CI: the failure before step 3 kept the backup container.\n'
}

check_failed_upgrade() {
  local before after status=0
  before=$(container backup)
  sudo find "$RECIPE_BACKUP_DIR" -maxdepth 1 -name 'recipe_*.dump.sha256' -print | LC_ALL=C sort \
    >"$CI_STATE/failed-upgrade-before.sets" || fail "cannot list the sets before the forced upgrade failure"
  (
    trap restore_tag_on_failure EXIT
    UPGRADE_STEP3_STARTED=true
    printf 'CI: force command 1 to fail without the manifest key.\n' >&2
    compose -f "$GITHUB_WORKSPACE/deploy/compose.previous.yaml" up -d backup-premigrate \
      || fail "cannot start the keyless backup-premigrate"
    wait_job backup-premigrate || fail "the keyless backup-premigrate failed"
  ) >"$CI_STATE/failed-upgrade.log" 2>&1 || status=$?
  cat "$CI_STATE/failed-upgrade.log"
  (( status != 0 )) || fail "the keyless backup-premigrate returned success"
  cat "$CI_STATE/backup-premigrate.log"
  grep -Fq '"message":"the manifest key file is missing"' "$CI_STATE/backup-premigrate.log" \
    || fail "the forced upgrade failure did not stop at the missing manifest key"
  sudo find "$RECIPE_BACKUP_DIR" -maxdepth 1 -name 'recipe_*.dump.sha256' -print | LC_ALL=C sort \
    >"$CI_STATE/failed-upgrade-after.sets" || fail "cannot list the sets after the forced upgrade failure"
  cmp "$CI_STATE/failed-upgrade-before.sets" "$CI_STATE/failed-upgrade-after.sets" \
    || fail "the forced upgrade failure published a new set"
  after=$(container backup)
  if [[ $PREVIOUS_MANIFEST_PROBE == no-check ]]
  then
    [[ $after != "$before" ]] || fail "the failed upgrade kept the old backup container"
    check_previous_backup_schedule "$after"
  else
    [[ $after == "$before" ]] || fail "the failed upgrade replaced a backup with the manifest check"
  fi
  printf 'CI: the forced upgrade failure returned %s with no new set.\n' "$status"
  check_early_upgrade_failure
}

reset_failed_upgrade_backup() {
  [[ $PREVIOUS_MANIFEST_PROBE == no-check ]] || return 0
  (
    export RECIPE_VERSION=ci-previous
    UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR
    compose up -d --no-deps --force-recreate backup || fail "cannot reset the previous backup after the case"
  ) || fail "cannot restore the backup state of start_previous"
  local environment
  environment=$(docker inspect --format '{{range .Config.Env}}{{println .}}{{end}}' "$(container backup)") \
    || fail "cannot inspect the backup environment after the case"
  grep -Fxq 'BACKUP_CRONTAB=/dev/null' <<<"$environment" \
    || fail "the backup schedule is not disabled after the case"
  check_backup_without_key
  printf 'CI: after the case: recipe-pgtools:ci-previous, BACKUP_CRONTAB=/dev/null, no manifest key.\n'
}

prepare_overlay_copy() {
  local scratch=$1
  mkdir -p -- "$scratch" && cp -a -- "$GITHUB_WORKSPACE/deploy" "$scratch/deploy" \
    || fail "cannot copy deploy for the overlay case"
  cp -- "$scratch/deploy/.env.example" "$scratch/deploy/.env" \
    || fail "cannot create the environment file of the overlay case"
  sed -i 's/^RECIPE_VERSION=.*/RECIPE_VERSION=ci-previous/' "$scratch/deploy/.env" \
    || fail "cannot select the previous tag in the overlay copy"
  printf 'COMPOSE_FILE=compose.yaml:compose.previous.yaml\n' >>"$scratch/deploy/.env" \
    || fail "cannot select the previous overlay in the copy"
}

check_overlay_key_state() {
  local directory=$1 expected=$2 actual
  (cd "$directory" && env -u RECIPE_VERSION docker compose --profile restore config --format json) \
    >"$CI_STATE/overlay-config.json" || fail "cannot read the overlay configuration"
  actual=$(upgrade_check manifest-key "$CI_STATE/overlay-config.json") \
    || fail "cannot read the overlay manifest key state"
  [[ $actual == "$expected" ]] || fail "the overlay configuration has the wrong manifest key state"
}

check_overlay_step_two() {
  local scratch=$CI_STATE/overlay-1 before
  before=$(container backup) || fail "cannot read the backup ID before the overlay exit"
  prepare_overlay_copy "$scratch"
  export COMPOSE_FILE=compose.yaml:compose.previous.yaml
  check_overlay_key_state "$scratch/deploy" no-key
  leave_previous_overlay "$scratch/deploy" ci-upgrade || fail "the step 2 overlay exit failed"
  check_overlay_key_state "$scratch/deploy" key
  [[ $(container backup) == "$before" ]] || fail "the step 2 overlay exit replaced backup"
  printf 'CI: overlay case 1: step 2 selected the key and kept the backup container.\n'
}

check_backup_with_key() {
  local number=$1 state deadline=$((SECONDS + 180))
  check_image backup recipe-pgtools:ci-upgrade
  compose exec -T backup test -f /run/secrets/backup_manifest_key \
    || fail "the new backup container has no manifest key"
  while (( SECONDS < deadline ))
  do
    state=$(docker inspect --format '{{.State.Health.Status}}' "$(container backup)") \
      || fail "cannot read the health of backup"
    if [[ $state == healthy ]]
    then
      printf 'CI: overlay case %s: backup has the manifest key and is healthy.\n' "$number"
      return 0
    fi
    sleep 1
  done
  fail "backup is not healthy after the overlay exit"
}

check_overlay_step_six() {
  local scratch=$CI_STATE/overlay-3
  if [[ $PREVIOUS_MANIFEST_PROBE == no-check ]]
  then
    RECIPE_VERSION=ci-previous compose -f "$GITHUB_WORKSPACE/deploy/compose.previous.yaml" \
      up -d --no-deps --force-recreate backup || fail "cannot create the rolled-back backup"
  else
    previous_compose up -d --no-deps --force-recreate backup || fail "cannot create the checked previous backup"
  fi
  prepare_overlay_copy "$scratch"
  export COMPOSE_FILE=compose.yaml:compose.previous.yaml
  check_overlay_key_state "$scratch/deploy" no-key
  leave_previous_overlay "$scratch/deploy" ci-upgrade || fail "the step 6 overlay exit failed"
  check_overlay_key_state "$scratch/deploy" key
  compose up -d --no-deps --force-recreate backup || fail "cannot recreate backup after the step 6 overlay exit"
  check_backup_with_key 3
}

overlay_temp_names() {
  find "$1" -maxdepth 1 -name '.env.*' -printf '%f\n' | LC_ALL=C sort
}

run_overlay_exit() {
  # The pasted function must also work without errexit or the CI source initialization.
  local directory=$1 stub=$2 output=$3 script
  script="$(declare -f leave_previous_overlay)"$'\n'
  script+='export COMPOSE_FILE=compose.yaml:compose.previous.yaml
leave_previous_overlay "$1" ci-upgrade
status=$?
printf "%s|%s\n" "$status" "${COMPOSE_FILE-unset}"'
  PATH="$stub:$PATH" bash -c "$script" bash "$directory" >"$output" \
    || fail "the shell of the overlay case did not finish"
}

prepare_overlay_file_case() {
  local number=$1 scratch=$2 directory=$2/deploy stub=$2/stubs group
  local -a groups
  prepare_overlay_copy "$scratch"
  mkdir -- "$stub" || fail "cannot create the overlay stubs"
  chmod 0644 "$directory/.env" || fail "cannot set the mode of the overlay environment file"
  if [[ $number == 5 ]]
  then
    group=$(id -G) || fail "cannot read the groups of the job user"
    read -r -a groups <<<"$group" || fail "cannot parse the groups of the job user"
    group=${groups[1]:-}
    if [[ -n $group ]]
    then
      chgrp "$group" "$directory/.env" || fail "cannot set the supplementary group of the overlay environment file"
    else
      printf 'CI: overlay case 5: the job user has no supplementary group.\n'
    fi
  fi
  if [[ $number == 6 ]]
  then
    sed -i '/^COMPOSE_FILE=/d; s/^RECIPE_VERSION=.*/RECIPE_VERSION=ci-upgrade/' "$directory/.env" \
      || fail "cannot prepare the overlay copy without COMPOSE_FILE"
  fi
  cp -- "$directory/.env" "$scratch/reference.env" || fail "cannot save the overlay reference"
  stat -c '%a %u %g' "$directory/.env" >"$scratch/reference.stat" || fail "cannot save the overlay metadata"
  grep -Ev '^(COMPOSE_FILE|RECIPE_VERSION)=' "$directory/.env" >"$scratch/reference.lines" \
    || fail "cannot save the other environment lines"
  overlay_temp_names "$directory" >"$scratch/before.names" || fail "cannot list the overlay temp names"
}

write_overlay_fault_stub() {
  local number=$1 directory=$2/deploy stub=$2/stubs
  case $number in
    4) chmod a-w "$directory/.env" "$directory" || fail "cannot make the overlay copy read-only" ;;
    7)
      cat >"$stub/perl" <<'STUB' || fail "cannot write the environment write stub"
#!/usr/bin/env bash
input=${!#}
head -c "$(( $(wc -c <"$input") / 2 ))" -- "$input"
printf 'stub: the environment write failed.\n' >&2
exit 1
STUB
      ;;
    8) printf '#!/usr/bin/env bash\nprintf "stub: the rename failed.\\n" >&2\nexit 1\n' >"$stub/mv" \
      || fail "cannot write the rename stub" ;;
  esac
  if [[ $number == 7 || $number == 8 ]]
  then
    chmod 0755 "$stub/"* || fail "cannot set the mode of the overlay stub"
  fi
}

check_overlay_file_case() {
  local number=$1 scratch=$CI_STATE/overlay-$1 directory=$CI_STATE/overlay-$1/deploy
  prepare_overlay_file_case "$number" "$scratch"
  write_overlay_fault_stub "$number" "$scratch"
  run_overlay_exit "$directory" "$scratch/stubs" "$scratch/result"
  if [[ $number == 4 ]]
  then
    chmod u+w "$directory" "$directory/.env" || fail "cannot make the overlay copy writable again"
  fi
  overlay_temp_names "$directory" >"$scratch/after.names" || fail "cannot list the remaining overlay temp names"
  cmp "$scratch/before.names" "$scratch/after.names" || fail "the overlay exit left a temp file"
  check_overlay_file_result "$number" "$scratch"
  printf 'CI: overlay case %s: the file, shell state, metadata, and temp names passed.\n' "$number"
}

check_overlay_file_result() {
  local number=$1 scratch=$2 directory=$2/deploy result
  result=$(cat "$scratch/result") || fail "cannot read the overlay result"
  if [[ $number == 5 || $number == 6 ]]
  then
    [[ $result == '0|unset' ]] || fail "the overlay exit did not return success and clear COMPOSE_FILE"
    stat -c '%a %u %g' "$directory/.env" >"$scratch/after.stat" || fail "cannot read the final overlay metadata"
    cmp "$scratch/reference.stat" "$scratch/after.stat" || fail "the overlay exit changed the environment metadata"
    grep -Ev '^(COMPOSE_FILE|RECIPE_VERSION)=' "$directory/.env" >"$scratch/after.lines" \
      || fail "cannot read the final environment lines"
    cmp "$scratch/reference.lines" "$scratch/after.lines" || fail "the overlay exit changed another environment line"
    [[ $(grep -c '^RECIPE_VERSION=' "$directory/.env") == 1 ]] \
      && grep -Fxq 'RECIPE_VERSION=ci-upgrade' "$directory/.env" || fail "the overlay exit wrote the wrong tag"
    if grep -q '^COMPOSE_FILE=' "$directory/.env"; then fail "the overlay exit kept COMPOSE_FILE"; fi
  else
    [[ $result =~ ^[1-9][0-9]*\|compose.yaml:compose.previous.yaml$ ]] \
      || fail "the failed overlay exit returned success or cleared COMPOSE_FILE"
    cmp "$scratch/reference.env" "$directory/.env" || fail "the failed overlay exit changed the environment file"
  fi
}

check_overlay_file_cases() {
  local number
  for number in 4 5 6 7 8
  do
    check_overlay_file_case "$number"
  done
}

check_upgraded_backup() {
  # Upgrade step 23, commands 1 to 3. Wait for a later minute before backup-now.
  compose up -d --no-deps --wait --wait-timeout 180 backup walarchive
  check_image backup recipe-pgtools:ci-upgrade
  check_backup_with_key 2
  check_image walarchive recipe-pgtools:ci-upgrade
  backup_now new-now
  check_restore "$(cat "$CI_STATE/new-now.set")" new-restore.log
}

upgrade() {
  # Upgrade steps 5 and 6. CI uses an explicit tag instead of a release .env file.
  export RECIPE_VERSION=ci-upgrade
  UPGRADE_COMPOSE_DIR=$GITHUB_WORKSPACE
  check_images
  # Upgrade step 7 does not apply. CI has no plant device before Caddy.
  # Upgrade steps 8 and 9. Keep the old app running while the rollback set completes.
  trap restore_tag_on_failure EXIT
  # The new mount of cluster-access.sh replaces db on the same volume. Record the identity of the cluster first.
  db_volume >"$CI_STATE/db.volume"
  db_system_identifier >"$CI_STATE/db.system-id"
  # Upgrade step 2. Check the volume name before the first Compose command that starts a container of the new stack.
  check_volume_name_mutation
  check_volume_external_mutation
  check_volume_name
  probe_previous_manifest
  check_previous_manifest_key
  # ADR-0104 (a), (b), (e). The step runs before the one replacement of db. The old walarchive still streams.
  # The new walarchive refuses old segments without the file of this step. The step needs no old container.
  write_wal_identity
  # Upgrade step 3 (ADR-0105 (g)). A new container of the previous image checks the old _now set. The job exported the new tag before.
  previous_compose run --rm --no-deps -T backup restore-test | tee "$CI_STATE/old-restore.log"
  upgrade_check restore "$CI_STATE/old-restore.log" "$(cat "$CI_STATE/old-now.set")"
  wait_backup_minute
  check_failed_upgrade
  check_overlay_step_two
  check_overlay_file_cases
  reset_failed_upgrade_backup
  UPGRADE_STEP3_STARTED=true
  compose up -d backup-premigrate
  wait_job backup-premigrate
  upgrade_check backup "$CI_STATE/backup-premigrate.log" _premigrate >"$CI_STATE/rollback.set"
  check_image backup-premigrate recipe-pgtools:ci-upgrade
  # Upgrade steps 10 to 12. A failure stops this script before the guard step.
  compose up -d --no-deps migrate
  wait_job migrate
  check_image migrate recipe-server:ci-upgrade
  trap - EXIT
  [[ $(container app) == "$(cat "$CI_STATE/old-app.id")" ]] || fail "the old app changed before the guard step"
  [[ $(db_volume) == "$(cat "$CI_STATE/db.volume")" ]] || fail "the upgrade changed the data volume of the database"
  [[ $(db_system_identifier) == "$(cat "$CI_STATE/db.system-id")" ]] || fail "the upgrade changed the system identifier of the database"
  replace_guard
  # Upgrade steps 18 and 19. Compose replaces the single app container.
  compose up -d --no-deps --wait --wait-timeout 180 app
  check_image app recipe-server:ci-upgrade
  compose exec -T app /usr/local/bin/recipe-server healthcheck --path /readyz
  # Upgrade step 20 does not apply without a plant device.
  check_smoke
  check_upgraded_backup
}

check_rollback_runbook() {
  local scratch=$CI_STATE/rollback-runbook set stamp status=0
  set=$(cat "$CI_STATE/rollback.set") || fail "cannot read the rollback set"
  [[ $set =~ ^recipe_[0-9]{8}_[0-9]{4}_premigrate$ ]] || fail "the rollback set name is invalid"
  stamp=${set#recipe_}
  prepare_overlay_copy "$scratch"
  # Delete backup to release the restore lock.
  compose rm -s -f backup || fail "cannot delete backup before the runbook check"
  (
    cd "$scratch/deploy" || fail "cannot enter the rollback runbook copy"
    # Compose must stat the root-owned secrets. Keep the runbook command unchanged inside this shell.
    sudo --preserve-env=COMPOSE_PROJECT_NAME,RECIPE_BACKUP_DIR,RECIPE_WALARCHIVE_DIR,RECIPE_SECRETS_DIR,RECIPE_PGDATA_VOLUME,COMPOSE_SUBNET,COMPOSE_IP_RANGE,PROXY_ADDRESS,SITE_ADDRESS \
      bash -euo pipefail -s -- "$stamp" <<'CHECK'
umask 077
stamp=$1
env -u COMPOSE_FILE RECIPE_VERSION=ci-upgrade docker compose -f compose.yaml run --rm restore-volume --check-only "$stamp"
CHECK
  ) | tee "$CI_STATE/rollback-runbook.log" || status=$?
  # Restore the CI overlay and its disabled schedule, including after a failed check.
  compose up -d --no-deps --wait --wait-timeout 180 backup || fail "cannot restart backup after the runbook check"
  (( status == 0 )) || fail "the runbook command refused the rollback set"
  grep -Fq '"level":"INFO","job":"restore-volume","message":"the set passed the check","set":"'"$set"'"}' \
    "$CI_STATE/rollback-runbook.log" || fail "the runbook check has no pass line"
  check_backup_with_key 4
}

restore_previous() {
  local set stamp directory=$RECIPE_BACKUP_DIR/upgrade-rollback file
  if [[ -n ${1:-} ]]; then
    set=recipe_$1
  else
    set=$(cat "$CI_STATE/rollback.set")
  fi
  [[ $set =~ ^recipe_[0-9]{8}_[0-9]{4}_premigrate$ ]] || fail "the rollback set name is invalid"
  stamp=${set#recipe_}
  # ADR-0174 (aj), command 1. Use the new tree and tag, the default /backups, and its existing lock.
  [[ $UPGRADE_COMPOSE_DIR == "$GITHUB_WORKSPACE" ]] || fail "the rollback check requires the new Compose tree"
  compose run --rm --no-deps -T restore-volume --check-only "$stamp" | tee "$CI_STATE/rollback-check.log" \
    || fail "the new image refused the rollback set"
  sudo install -d -o 999 -g 999 -m 0700 "$directory" || fail "cannot create the rollback directory"
  sudo install -o 999 -g 999 -m 0600 /dev/null "$directory/.backup-marker" \
    || fail "cannot create the rollback marker"
  for file in "$set.dump" "$set.counts" "globals_$stamp.sql" "$set.dump.sha256" "$set.dump.sha256.hmac"
  do
    sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/$file" "$directory/$file" \
      || fail "cannot copy a file of the rollback set"
    sudo cmp -- "$RECIPE_BACKUP_DIR/$file" "$directory/$file" \
      || fail "a copied rollback file differs from its original"
  done
  # The old restore-test restores into its private cluster and checks every restore gate.
  previous_compose run --rm --no-deps -T upgrade-rollback | tee "$CI_STATE/rollback-restore.log" \
    || fail "the previous image refused the rollback set"
  upgrade_check restore "$CI_STATE/rollback-restore.log" "$set"
  check_rollback_manifest "$set" "$stamp"
}

delete_rollback_set_copy() {
  local file
  sudo rm -f -- "${copy_files[@]}" || { printf 'CI: cannot delete the rollback set copy.\n' >&2; return 1; }
  for file in "${copy_files[@]}"
  do
    if sudo test -e "$file" || sudo test -L "$file"; then
      printf 'CI: a file of the rollback set copy remains.\n' >&2
      return 1
    fi
  done
}

check_changed_rollback_set() {
  (
    local set stamp copy_set suffix file status=0
    set=$(cat "$CI_STATE/rollback.set")
    [[ $set =~ ^recipe_[0-9]{8}_[0-9]{4}_premigrate$ ]] || fail "the rollback set name is invalid"
    stamp=$(date -u -d "${set:7:8} ${set:16:2}:${set:18:2} UTC + 1 minute" +%Y%m%d_%H%M)_premigrate
    copy_set=recipe_$stamp
    local copy_files=("$RECIPE_BACKUP_DIR/$copy_set.dump.sha256" "$RECIPE_BACKUP_DIR/$copy_set.dump.sha256.hmac"
      "$RECIPE_BACKUP_DIR/$copy_set.dump" "$RECIPE_BACKUP_DIR/globals_$stamp.sql" "$RECIPE_BACKUP_DIR/$copy_set.counts")
    for file in "${copy_files[@]}"
    do
      if sudo test -e "$file" || sudo test -L "$file"; then
        fail "a file of the rollback set copy already exists"
      fi
    done
    trap 'status=$?; trap - EXIT; delete_rollback_set_copy || status=1; exit "$status"' EXIT
    trap 'exit 143' TERM
    trap 'exit 130' INT
    trap 'exit 129' HUP
    for suffix in dump counts dump.sha256 dump.sha256.hmac
    do
      sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/$set.$suffix" "$RECIPE_BACKUP_DIR/$copy_set.$suffix" \
        || fail "cannot copy a file of the changed rollback set"
    done
    sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/globals_${set#recipe_}.sql" "$RECIPE_BACKUP_DIR/globals_$stamp.sql" \
      || fail "cannot copy the globals of the changed rollback set"
    # A custom dump starts with PGDMP. Replace its first byte and keep its size and old tag.
    printf '\000' | sudo dd of="$RECIPE_BACKUP_DIR/$copy_set.dump" bs=1 count=1 conv=notrunc status=none \
      || fail "cannot change the rollback dump byte"
    sudo bash -euo pipefail -c '
      cd "$1"
      sha256sum -- "$2.dump" "globals_${2#recipe_}.sql" "$2.counts" >"$2.dump.sha256"
    ' bash "$RECIPE_BACKUP_DIR" "$copy_set" || fail "cannot compute the changed rollback manifest"
    (restore_previous "$stamp") | tee "$CI_STATE/rollback-tag-mismatch.log" || status=$?
    (( status != 0 )) || fail "the rollback accepted the changed set"
    [[ ! -e $RECIPE_BACKUP_DIR/upgrade-rollback && ! -L $RECIPE_BACKUP_DIR/upgrade-rollback ]] \
      || fail "the refused rollback created its copy directory"
    [[ $(grep -c '"level":"ERROR"' "$CI_STATE/rollback-tag-mismatch.log") == 1 ]] \
      || fail "the rollback check did not write one tag ERROR line"
    grep -Eq '"level":"ERROR","job":"restore-volume","message":"the tag of the set does not match the manifest","set":"'"$copy_set"'"}$' \
      "$CI_STATE/rollback-tag-mismatch.log" || fail "the rollback did not stop at the tag check"
    printf 'CI: the new image refused the changed rollback set %s before the copy.\n' "$copy_set"
  )
}

check_rollback_manifest() {
  # restore-test created the lock in the copied set directory. A capable image also proves its tag gate here.
  [[ $PREVIOUS_MANIFEST_PROBE == check ]] || return 0
  previous_compose run --rm --no-deps -T upgrade-rollback restore-volume --check-only "$2" \
    | tee -a "$CI_STATE/rollback-restore.log" || fail "the rollback manifest check failed"
  grep -Fq '"level":"INFO","job":"restore-volume","message":"the set passed the check","set":"'"$1"'"}' \
    "$CI_STATE/rollback-restore.log" || fail "the rollback log has no manifest check pass"
}

cleanup() {
  # Delete only this job's project and volumes, including failed one-shot containers.
  local status=0
  compose --profile upgrade-rollback down -v --remove-orphans || status=$?
  delete_ci_volumes || status=$?
  # A previous release without external pgdata owns an unlabelled volume. It stays on the disposable runner.
  for disk in backups walarchive-disk
  do
    if mountpoint -q "$CI_ROOT/$disk"
    then
      sudo umount "$CI_ROOT/$disk" || status=$?
    fi
  done
  return "$status"
}

case ${1:-} in
  previous) build_previous; start_previous ;;
  upgrade) upgrade; check_rollback_runbook; check_changed_rollback_set; restore_previous; check_overlay_step_six ;;
  cleanup) cleanup ;;
  *) fail "usage: upgrade.sh previous|upgrade|cleanup" ;;
esac
