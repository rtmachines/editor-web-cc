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
  git worktree add --detach "$UPGRADE_PREVIOUS_DIR" "$base" \
    || fail "cannot create the previous worktree. Stop and report."
  printf 'Previous release: %s\n' "$base"
  if ! docker buildx build --load --tag recipe-server:ci-previous \
      --file "$UPGRADE_PREVIOUS_DIR/deploy/Dockerfile" "$UPGRADE_PREVIOUS_DIR" \
    || ! docker buildx build --load --tag recipe-pgtools:ci-previous \
      --file "$UPGRADE_PREVIOUS_DIR/deploy/pgtools/Dockerfile" "$UPGRADE_PREVIOUS_DIR/deploy/pgtools"
  then
    fail "cannot build the previous images. Stop and report."
  fi
}

start_previous() {
  export RECIPE_VERSION=ci-previous
  UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR
  check_images
  compose up -d --wait --wait-timeout 180 db
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
  container db >"$CI_STATE/old-db.id"
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
    "$GITHUB_WORKSPACE/deploy/pgtools/restore-code.expected" | cut -d'|' -f4)
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

upgrade() {
  # Upgrade steps 5 and 6. CI uses an explicit tag instead of a release .env file.
  export RECIPE_VERSION=ci-upgrade
  UPGRADE_COMPOSE_DIR=$GITHUB_WORKSPACE
  check_images
  # Upgrade step 7 does not apply. CI has no plant device before Caddy.
  # Upgrade steps 8 and 9. Keep the old app running while the rollback set completes.
  trap restore_tag_on_failure EXIT
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
  [[ $(container db) == "$(cat "$CI_STATE/old-db.id")" ]] || fail "the upgrade replaced the database container"
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
  compose --profile upgrade-rollback down -v --remove-orphans
  for disk in backups walarchive
  do
    if mountpoint -q "$CI_ROOT/$disk"
    then
      sudo umount "$CI_ROOT/$disk"
    fi
  done
}

case ${1:-} in
  previous) build_previous; start_previous ;;
  upgrade) upgrade; restore_previous ;;
  cleanup) cleanup ;;
  *) fail "usage: upgrade.sh previous|upgrade|cleanup" ;;
esac
