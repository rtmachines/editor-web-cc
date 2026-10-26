#!/usr/bin/env bash
# D11 step 9. A 100 response proves that app waits for the request body.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

app=$(container app)
address=$(docker inspect --format "{{(index .NetworkSettings.Networks \"${COMPOSE_PROJECT_NAME}_backend\").IPAddress}}" "$app")
python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/stop_probe.py" "$address" &
request_pid=$!
trap 'kill "$request_pid" 2>/dev/null || true' EXIT
deadline=$((SECONDS + 5))
until [[ -f $CI_STATE/inflight-ready ]]
do
  kill -0 "$request_pid" || fail "the request stopped before the drain"
  (( SECONDS < deadline )) || fail "the request did not reach app"
  sleep 0.1
done
compose stop app
wait "$request_pid" || fail "the shutdown request check failed"
trap - EXIT
[[ $(docker inspect --format '{{.State.Status}}|{{.State.ExitCode}}|{{.State.OOMKilled}}' "$app") == 'exited|0|false' ]] \
  || fail "app did not stop with exit code zero"
