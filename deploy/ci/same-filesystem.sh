#!/usr/bin/env bash
# D11 step 10. Both fixture directories use one plain volume.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

volume=${COMPOSE_PROJECT_NAME}-one-filesystem
docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$volume" >/dev/null
docker run --rm --network none --user 0:0 --security-opt no-new-privileges:true \
  --mount "type=volume,src=$volume,dst=/fixture" --entrypoint bash \
  "recipe-pgtools:$RECIPE_VERSION" -euc '
    umask 077
    mkdir /fixture/backups /fixture/pgdata
    printf "16\n" >/fixture/pgdata/PG_VERSION
    touch /fixture/backups/.backup-marker
    chown -R 999:999 /fixture
  '
if docker run --rm --network none --cap-drop ALL --security-opt no-new-privileges:true \
  --mount "type=volume,src=$volume,dst=/fixture,readonly" \
  -e BACKUP_DIR=/fixture/backups -e PGDATA=/fixture/pgdata \
  "recipe-pgtools:$RECIPE_VERSION" backup-health >"$CI_STATE/same-filesystem.log" 2>&1
then
  fail "backup-health accepted one filesystem"
fi
grep -Fq '"message":"the backup directory is on the filesystem of pgdata"' "$CI_STATE/same-filesystem.log" \
  || fail "backup-health failed for another reason"
