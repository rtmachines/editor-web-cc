#!/usr/bin/env bash
# D11 step 6. The TCP rejection must come from pg_hba.conf.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

http_probe redirect
[[ $(docker inspect --format '{{.Config.User}}' "$(container app)") == 65532:65532 ]] \
  || fail "app does not run as uid 65532"
if compose exec -T backup env PGCONNECT_TIMEOUT=5 PGSSLMODE=disable \
  psql -X -w -h db -U postgres -d recipe -c 'SELECT 1' >"$CI_STATE/tcp.log" 2>&1
then
  fail "postgres connected over TCP"
fi
grep -Fq 'pg_hba.conf rejects connection' "$CI_STATE/tcp.log" \
  || fail "the TCP failure did not come from the reject rule"
