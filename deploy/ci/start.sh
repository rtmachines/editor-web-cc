#!/usr/bin/env bash
# D11 step 4. This is the first start, not an upgrade (ADR-0082).
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/memory.sh"

compose config --quiet
timeout 300s bash -c '
  source "$GITHUB_WORKSPACE/deploy/ci/common.sh"
  compose pull --ignore-buildable --quiet
'
deadline=$((SECONDS + 60))
memory_premigrate
memory_watch backup-premigrate '' 60 &
memory_pid=$!
trap 'kill "$memory_pid" 2>/dev/null || true' EXIT
timeout 60s bash -c '
  source "$GITHUB_WORKSPACE/deploy/ci/common.sh"
  compose -f "$CI_STATE/memory.compose.yaml" up -d --no-build
'
wait "$memory_pid"
trap - EXIT
while (( SECONDS < deadline ))
do
  ready=true
  for service in db app proxy backup walarchive
  do
    state=$(docker inspect --format '{{.State.Status}}|{{.State.Health.Status}}' "$(container "$service")")
    [[ $state == 'running|healthy' ]] || ready=false
  done
  for service in backup-premigrate migrate
  do
    state=$(docker inspect --format '{{.State.Status}}|{{.State.ExitCode}}' "$(container "$service")")
    [[ $state == 'exited|0' ]] || ready=false
    [[ $state != exited\|* || $state == 'exited|0' ]] || fail "a one-shot service failed"
  done
  if $ready && [[ $(compose exec -T proxy wget -q -T 3 -O - http://app:9000/readyz) == ok ]]
  then
    exit 0
  fi
  sleep 1
done
fail "the stack did not become ready within 60 seconds"
