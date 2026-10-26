#!/usr/bin/env bash
# Delete only the disposable resources of this GitHub job.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# The runner owns these volumes. No production project uses this unique name.
status=0
compose down --volumes --remove-orphans || status=$?
volume=${COMPOSE_PROJECT_NAME}-one-filesystem
if docker volume inspect "$volume" >/dev/null 2>&1
then
  if [[ $(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$volume") == "$COMPOSE_PROJECT_NAME" ]]
  then
    # The label check and run-scoped name restrict deletion to this fixture on a throwaway CI runner.
    # The operator rule in docs/ops/runbook.md still applies: never run docker volume rm on pgdata.
    docker volume rm "$volume" >/dev/null || status=$?
  else
    status=1
  fi
fi
for disk in backups walarchive
do
  if mountpoint -q "$CI_ROOT/$disk"
  then
    sudo umount "$CI_ROOT/$disk" || status=$?
  fi
done
exit "$status"
