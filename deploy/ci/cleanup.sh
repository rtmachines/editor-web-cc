#!/usr/bin/env bash
# Delete only the disposable resources of this GitHub job.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# The runner owns these volumes. No production project uses this unique name.
compose down --volumes --remove-orphans || exit 1
volume=${COMPOSE_PROJECT_NAME}-one-filesystem
if docker volume inspect "$volume" >/dev/null 2>&1
then
  [[ $(docker volume inspect --format '{{index .Labels "recipe.ci"}}' "$volume") == "$COMPOSE_PROJECT_NAME" ]]
  docker volume rm "$volume" >/dev/null
fi
for disk in backups walarchive
do
  if mountpoint -q "$CI_ROOT/$disk"
  then
    sudo umount "$CI_ROOT/$disk"
  fi
done
