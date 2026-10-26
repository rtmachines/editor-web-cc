#!/usr/bin/env bash
# Shared commands for the upgrade test on a disposable GitHub runner.
set -euo pipefail
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[[ ${CI:-} == true && ${RUNNER_OS:-} == Linux ]] || fail "the upgrade test requires Linux CI"
[[ $COMPOSE_PROJECT_NAME == "recipe-ci-${GITHUB_RUN_ID:?}-${GITHUB_RUN_ATTEMPT:?}" ]] \
  || fail "the upgrade project does not belong to this job"
export UPGRADE_PREVIOUS_DIR=$CI_ROOT/previous
UPGRADE_COMPOSE_DIR=$GITHUB_WORKSPACE

compose() {
  sudo --preserve-env=COMPOSE_PROJECT_NAME,RECIPE_VERSION,RECIPE_BACKUP_DIR,RECIPE_WALARCHIVE_DIR,RECIPE_SECRETS_DIR,COMPOSE_SUBNET,COMPOSE_IP_RANGE,PROXY_ADDRESS,SITE_ADDRESS,UPGRADE_PREVIOUS_DIR \
    docker compose -p "$COMPOSE_PROJECT_NAME" \
    -f "$UPGRADE_COMPOSE_DIR/deploy/compose.yaml" \
    -f "$UPGRADE_COMPOSE_DIR/deploy/compose.dev.yaml" \
    -f "$GITHUB_WORKSPACE/deploy/ci/upgrade.compose.yaml" "$@"
}

upgrade_check() {
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/upgrade_checks.py" "$@"
}

check_images() {
  compose config --format json >"$CI_STATE/upgrade-config.json"
  upgrade_check images "$CI_STATE/upgrade-config.json" "$RECIPE_VERSION"
}

wait_job() {
  local service=$1 state deadline=$((SECONDS + 180))
  while (( SECONDS < deadline ))
  do
    state=$(docker inspect --format '{{.State.Status}}|{{.State.ExitCode}}' "$(container "$service")")
    if [[ $state == exited\|* ]]
    then
      compose ps -a "$service"
      compose logs --no-log-prefix "$service" >"$CI_STATE/$service.log"
      [[ $state == 'exited|0' ]] || fail "$service did not exit with code 0"
      return 0
    fi
    sleep 1
  done
  fail "$service did not finish within 180 seconds"
}

check_image() {
  local actual expected
  actual=$(docker inspect --format '{{.Image}}' "$(container "$1")")
  expected=$(docker image inspect --format '{{.Id}}' "$2")
  [[ $actual == "$expected" ]] || fail "$1 does not use the expected image"
}

wait_backup_minute() {
  local deadline=$((SECONDS + 65)) stamp
  while true
  do
    stamp=$(date -u +%Y%m%d_%H%M)
    if ! sudo test -f "$RECIPE_BACKUP_DIR/recipe_${stamp}_premigrate.dump.sha256" \
      && ! sudo test -f "$RECIPE_BACKUP_DIR/recipe_${stamp}_now.dump.sha256"
    then
      return 0
    fi
    (( SECONDS < deadline )) || fail "the backup minute did not advance"
    sleep 1
  done
}

check_restore() {
  local expected=$1 log=$2
  compose exec -T backup restore-test | tee "$CI_STATE/$log"
  upgrade_check restore "$CI_STATE/$log" "$expected"
}

backup_now() {
  wait_backup_minute
  compose exec -T backup backup-now | tee "$CI_STATE/$1.log"
  upgrade_check backup "$CI_STATE/$1.log" _now >"$CI_STATE/$1.set"
}

restore_tag_on_failure() {
  local status=$?
  if (( status != 0 ))
  then
    # Upgrade step 12. Select the previous tag without replacing the running app.
    export RECIPE_VERSION=ci-previous
    UPGRADE_COMPOSE_DIR=$UPGRADE_PREVIOUS_DIR
    check_images
    printf 'CI: the upgrade stopped before the guard step. The previous image tag is selected.\n' >&2
  fi
  return "$status"
}
