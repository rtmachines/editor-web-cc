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
export RECIPE_WALARCHIVE_DIR=$CI_ROOT/walarchive
export RECIPE_SECRETS_DIR=$CI_ROOT/secrets

compose() {
  # Compose must stat the files in the root-owned secret directory.
  sudo --preserve-env=COMPOSE_PROJECT_NAME,RECIPE_VERSION,RECIPE_BACKUP_DIR,RECIPE_WALARCHIVE_DIR,RECIPE_SECRETS_DIR,COMPOSE_SUBNET,COMPOSE_IP_RANGE,PROXY_ADDRESS,SITE_ADDRESS,PITR_MARKER_SEQ,PITR_MARKER_HASH \
    docker compose --project-name "$COMPOSE_PROJECT_NAME" \
    -f "$GITHUB_WORKSPACE/deploy/compose.yaml" \
    -f "$GITHUB_WORKSPACE/deploy/compose.build.yaml" \
    -f "$GITHUB_WORKSPACE/deploy/compose.dev.yaml" "$@"
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
