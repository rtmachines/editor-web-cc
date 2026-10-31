#!/usr/bin/env bash
# D11 step 8. Test the installed scripts with the tools of their image.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

bash "$GITHUB_WORKSPACE/deploy/ci/check.sh"
for suite in backup-test restore-test-test restore-volume-test walarchive-test pitr-volume-test
do
  docker run --rm --network none --cap-drop ALL --security-opt no-new-privileges:true \
    --memory 512m --memory-swap 512m --cpus 0.5 \
    -e CI=true -e PGTOOLS_BIN=/usr/local/bin -e PGTOOLS_CRONTAB_DIR=/etc/pgtools --entrypoint bash \
    --mount "type=bind,src=$GITHUB_WORKSPACE/deploy/pgtools,dst=/pgtools,readonly" \
    --mount "type=bind,src=$GITHUB_WORKSPACE/deploy/db,dst=/db,readonly" \
    "recipe-pgtools:$RECIPE_VERSION" "/pgtools/tests/$suite"
done
