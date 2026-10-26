#!/usr/bin/env bash
# Read cgroup v2 memory metrics on the disposable GitHub runner.
set -euo pipefail
umask 077

memory_log() {
  printf '%s\n' "$1" | tee -a "$CI_STATE/memory.log" || true
}

memory_cgroup() {
  local service=$1 path=$2 limit=$3 peak anon file oom_kill
  peak=$(sudo -n cat "$path/memory.peak" 2>/dev/null) || peak=
  anon=$(sudo -n awk '$1 == "anon" { print $2 }' "$path/memory.stat" 2>/dev/null) || anon=
  file=$(sudo -n awk '$1 == "file" { print $2 }' "$path/memory.stat" 2>/dev/null) || file=
  oom_kill=$(sudo -n awk '$1 == "oom_kill" { print $2 }' "$path/memory.events" 2>/dev/null) || oom_kill=
  [[ $peak =~ ^[0-9]+$ ]] || peak=unavailable
  [[ $limit =~ ^[0-9]+$ ]] || limit=unavailable
  [[ $anon =~ ^[0-9]+$ ]] || anon=unavailable
  [[ $file =~ ^[0-9]+$ ]] || file=unavailable
  [[ $oom_kill =~ ^[0-9]+$ ]] || oom_kill=unavailable
  memory_log "memory.peak $service peak=$peak limit=$limit anon=$anon file=$file oom_kill=$oom_kill"
  # The peak includes file cache. Only an OOM kill fails this check.
  if [[ $oom_kill =~ ^[0-9]+$ && $oom_kill =~ [1-9] ]]; then
    memory_log "memory.oom_kill $service oom_kill=$oom_kill: OOM kill detected"
    return 1
  fi
  return 0
}

memory_peak() {
  local service=$1 reference=$2 id path= limit=
  id=$(docker inspect --format '{{.Id}}' "$reference" 2>/dev/null) || id=
  if [[ $id =~ ^[0-9a-f]{64}$ ]]; then
    path=$(sudo -n find /sys/fs/cgroup -type d \
      \( -name "$id" -o -name "docker-$id.scope" \) -print -quit 2>/dev/null) || path=
    limit=$(docker inspect --format '{{.HostConfig.Memory}}' "$id" 2>/dev/null) || limit=
  fi
  memory_cgroup "$service" "${path:-/sys/fs/cgroup/unavailable}" "$limit"
}

memory_service() {
  local id
  id=$(compose ps -aq "$1" 2>/dev/null) || id=
  memory_peak "$1" "$id"
}

memory_exec() {
  local service=$1 status=0
  shift
  compose exec -T "$service" "$@" || status=$?
  memory_service "$service" || status=$?
  return "$status"
}

memory_hold() {
  local status=0 deadline
  "$@" || status=$?
  # Keep the cgroup alive until the host reads its peak, even when the command fails.
  touch /tmp/ci-memory-done
  deadline=$((SECONDS + 30))
  while [[ ! -f /tmp/ci-memory-release ]] && (( SECONDS < deadline ))
  do
    sleep 0.1
  done
  exit "$status"
}

memory_watch() {
  local service=$1 reference=$2 deadline=$((SECONDS + $3)) id state status=0
  while (( SECONDS < deadline ))
  do
    if [[ -n $reference ]]; then
      id=$(docker inspect --format '{{.Id}}' "$reference" 2>/dev/null) || id=
    else
      id=$(compose ps -aq "$service" 2>/dev/null) || id=
    fi
    if [[ $id =~ ^[0-9a-f]+$ ]]; then
      if docker exec "$id" test -f /tmp/ci-memory-done 2>/dev/null; then
        memory_peak "$service" "$id" || status=$?
        docker exec "$id" touch /tmp/ci-memory-release 2>/dev/null || true
        return "$status"
      fi
      state=$(docker inspect --format '{{.State.Status}}' "$id" 2>/dev/null) || state=
      if [[ $state == exited || $state == dead ]]; then
        memory_peak "$service" "$id" || status=$?
        return "$status"
      fi
    fi
    sleep 0.2
  done
  memory_log 'memory.peak unavailable'
}

memory_premigrate() {
  # The startup dependency runs a premigration dump. Wrap it without changing its exit code.
  cat >"$CI_STATE/memory.compose.yaml" <<YAML
services:
  backup-premigrate:
    command: ["bash", "/ci-memory.sh", "hold", "backup-once", "_premigrate"]
    volumes:
      - "$GITHUB_WORKSPACE/deploy/ci/memory.sh:/ci-memory.sh:ro"
YAML
}

memory_pitr() {
  local name=${COMPOSE_PROJECT_NAME}-pitr-memory watcher status=0
  memory_watch pitr-check "$name" 2400 &
  watcher=$!
  # Keep a named container until the measurement finishes, then remove only that container.
  compose run --name "$name" --no-deps -T \
    -v "$GITHUB_WORKSPACE/deploy/ci/memory.sh:/ci-memory.sh:ro" \
    -e PITR_CI=1 -e PITR_MARKER_SEQ -e PITR_MARKER_HASH \
    pitr-check bash /ci-memory.sh hold pitr-check || status=$?
  if (( status != 0 )) && kill -0 "$watcher" 2>/dev/null; then
    kill "$watcher" 2>/dev/null || true
    memory_peak pitr-check "$name" || status=$?
  fi
  wait "$watcher" || status=$?
  docker rm "$name" >/dev/null
  return "$status"
}

if [[ ${BASH_SOURCE[0]} == "$0" ]]; then
  fail() {
    printf 'CI: %s\n' "$1" >&2
    exit 1
  }
  [[ ${1:-} == hold ]] || fail "usage: memory.sh hold command"
  shift
  memory_hold "$@"
fi
