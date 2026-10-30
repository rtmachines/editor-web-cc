#!/usr/bin/env bash
# Shared commands for the disposable GitHub runner.
set -euo pipefail
umask 077

: "${GITHUB_WORKSPACE:?}" "${RUNNER_TEMP:?}" "${COMPOSE_PROJECT_NAME:?}"
[[ ${GITHUB_ACTIONS:-} == true ]] || exit 1
[[ $COMPOSE_PROJECT_NAME =~ ^recipe-ci-[0-9]+-[0-9]+$ ]] || exit 1
export LC_ALL=C
CI_ROOT=$RUNNER_TEMP/$COMPOSE_PROJECT_NAME
CI_STATE=$CI_ROOT/state
export CI_STATE
export RECIPE_BACKUP_DIR=$CI_ROOT/backups
export RECIPE_WALARCHIVE_DIR=$CI_ROOT/walarchive-disk/walarchive
export RECIPE_SECRETS_DIR=$CI_ROOT/secrets
# One name for the data volume. Release v0.1.0 gave its own volume the same name, so the upgrade keeps it.
export RECIPE_PGDATA_VOLUME=${COMPOSE_PROJECT_NAME}_pgdata

compose() {
  # Compose must stat the files in the root-owned secret directory.
  sudo --preserve-env=COMPOSE_PROJECT_NAME,RECIPE_VERSION,RECIPE_BACKUP_DIR,RECIPE_WALARCHIVE_DIR,RECIPE_SECRETS_DIR,RECIPE_PGDATA_VOLUME,COMPOSE_SUBNET,COMPOSE_IP_RANGE,PROXY_ADDRESS,SITE_ADDRESS,PITR_MARKER_SEQ,PITR_MARKER_HASH \
    docker compose --project-name "$COMPOSE_PROJECT_NAME" \
    -f "$GITHUB_WORKSPACE/deploy/compose.yaml" \
    -f "$GITHUB_WORKSPACE/deploy/compose.build.yaml" \
    -f "$GITHUB_WORKSPACE/deploy/compose.dev.yaml" "$@"
}

create_pgdata_volume() {
  # Compose never creates pgdata. The label lets delete_ci_volumes find this volume.
  docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$RECIPE_PGDATA_VOLUME" >/dev/null
}

delete_ci_volumes() {
  # Delete only volumes with the label of this job. Never delete by name.
  local ids status=0 volume
  ids=$(docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME") || return 1
  for volume in $ids
  do
    # The label check and run-scoped name restrict deletion to the volumes of this job on a throwaway CI runner.
    # The operator rule in docs/ops/runbook.md still applies: never run docker volume rm on pgdata.
    if [[ $(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$volume") == "$COMPOSE_PROJECT_NAME" ]]
    then
      docker volume rm "$volume" >/dev/null || status=$?
    else
      status=1
    fi
  done
  return "$status"
}

fail() {
  printf 'CI: %s\n' "$1" >&2
  exit 1
}

sql() {
  compose exec -T -u postgres db psql -X -At -v ON_ERROR_STOP=1 -d recipe "$@"
}

container() {
  local id
  id=$(compose ps -aq "$1")
  [[ $id =~ ^[0-9a-f]+$ ]] || fail "expected one container for $1"
  printf '%s\n' "$id"
}

http_probe() {
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/http_probe.py" "$@"
}
