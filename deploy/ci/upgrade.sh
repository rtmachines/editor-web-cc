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
  # Upgrade step 3. The old image must accept the exact old _now set.
  check_restore "$(cat "$CI_STATE/old-now.set")" old-restore.log
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

# Upgrade step 2 and ADR-0103 (d). The new stack must use the data volume of the running db.
VOLUME_NAME_MESSAGE="the pgdata volume of the new stack differs from the data volume of the running db. Stop and report."

check_volume_name() {
  # Every command here has its own fail line. A mutation runs this function in a condition, where set -e is off.
  local expected actual
  compose config --format json >"$CI_STATE/volume-config.json" \
    || fail "cannot read the Compose configuration of the new stack"
  expected=$(upgrade_check volume-name "$CI_STATE/volume-config.json") \
    || fail "cannot read the pgdata volume name of the new stack"
  actual=$(db_volume) || fail "cannot read the data volume of the running db"
  [[ $expected == "$actual" ]] || fail "$VOLUME_NAME_MESSAGE"
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
[[ $id =~ ^[0-9]+$ ]] || { printf 'CI: the system identifier of db is not a number.\n' >&2; exit 1; }
if [[ -e $file || -L $file ]]; then
  printf 'CI: the segment check does not run. The file .system-identifier exists.\n'
  exit 0
fi
# A complete segment has 24 upper-case hex characters and no .partial suffix. LC_ALL=C sorts them by age.
segment=
for _ in $(seq 30); do
  segment=$(ls -1 -- "$dir" | { grep -E '^[0-9A-F]{24}$' || true; } | sort | tail -n 1)
  [[ -n $segment ]] && break
  sleep 1
done
[[ -n $segment ]] || { printf 'CI: the WAL archive holds no complete segment.\n' >&2; exit 1; }
# The long header of the first page holds xlp_info at offset 2 and xlp_sysid at offset 24 (little-endian).
read -r info < <(od -An -t u2 --endian=little -j 2 -N 2 -- "$dir/$segment")
read -r sysid < <(od -An -t u8 --endian=little -j 24 -N 8 -- "$dir/$segment")
[[ $info =~ ^[0-9]+$ && $sysid =~ ^[0-9]+$ ]] \
  || { printf 'CI: cannot read the header of the WAL segment %s.\n' "$segment" >&2; exit 1; }
(( info & 2 )) || { printf 'CI: the first page of the WAL segment %s has no long header.\n' "$segment" >&2; exit 1; }
[[ $sysid == "$id" ]] \
  || { printf 'CI: the header of the WAL segment %s holds another system identifier.\n' "$segment" >&2; exit 1; }
printf 'CI: the segment check passed. The segment %s holds the system identifier of db.\n' "$segment"
temp=$(mktemp "$dir/.system-identifier.tmp.XXXXXX")
printf '%s\n' "$id" >"$temp"
ln -T -- "$temp" "$file" || { rm -f -- "$temp"; printf 'CI: cannot publish the file .system-identifier.\n' >&2; exit 1; }
rm -f -- "$temp"
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
  grep -Eq '^[0-9A-F]{24}$' <<<"$names"     || fail "the old walarchive wrote no complete WAL segment within $WAL_SEGMENT_WAIT_SECONDS seconds. Stop and report."
  # The step prints one line. The job checks it against the state of the file before the step.
  if sudo test -e "$file" || sudo test -L "$file"
  then
    mode=skipped
  fi
  # The step runs in a new container with the previous tag. The job exported the new tag before.
  printf '%s
' "$id"     | RECIPE_VERSION=ci-previous compose run --rm --no-deps -T --entrypoint sh walarchive -c 'exec bash -c "$1"' sh "$WAL_IDENTITY_STEP"     | tee "$CI_STATE/wal-identity.log"     || fail "the segment check of the existing installation failed. Stop and report."
  upgrade_check wal-identity "$CI_STATE/wal-identity.log" "$mode"
  [[ $(sudo cat -- "$file") == "$(db_system_identifier)" ]]     || fail "the file .system-identifier does not hold the identifier of db"
  printf 'CI: the file .system-identifier holds the system identifier of db.
'
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
  check_volume_name
  # ADR-0104 (a), (b), (e). The step runs before the one replacement of db. The old walarchive still streams.
  # The new walarchive refuses old segments without the file of this step. The step needs no old container.
  write_wal_identity
  wait_backup_minute
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
  # Upgrade step 23, commands 1 to 3. Wait for a later minute before backup-now.
  compose up -d --no-deps --wait --wait-timeout 180 backup walarchive
  check_image backup recipe-pgtools:ci-upgrade
  check_image walarchive recipe-pgtools:ci-upgrade
  backup_now new-now
  check_restore "$(cat "$CI_STATE/new-now.set")" new-restore.log
}

restore_previous() {
  local set stamp directory=$RECIPE_BACKUP_DIR/upgrade-rollback file
  set=$(cat "$CI_STATE/rollback.set")
  [[ $set =~ ^recipe_[0-9]{8}_[0-9]{4}_premigrate$ ]] || fail "the rollback set name is invalid"
  stamp=${set#recipe_}
  sudo install -d -o 999 -g 999 -m 0700 "$directory"
  sudo install -o 999 -g 999 -m 0600 /dev/null "$directory/.backup-marker"
  for file in "$set.dump" "$set.counts" "globals_$stamp.sql" "$set.dump.sha256"
  do
    sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/$file" "$directory/$file"
  done
  # The old restore-test restores into its private cluster and checks every restore gate.
  compose run --rm --no-deps -T upgrade-rollback | tee "$CI_STATE/rollback-restore.log"
  upgrade_check restore "$CI_STATE/rollback-restore.log" "$set"
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
  upgrade) upgrade; restore_previous ;;
  cleanup) cleanup ;;
  *) fail "usage: upgrade.sh previous|upgrade|cleanup" ;;
esac
