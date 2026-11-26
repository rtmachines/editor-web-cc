#!/usr/bin/env bash
# Run the login flood and the value load steps (ADR-0128, ADR-0222).
# Without an argument, run the flood and check the timings and flood readings.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/memory.sh"

# GitHub release metadata pins the Linux amd64 asset of the checker fixtures.
readonly OHA_VERSION=v1.16.0
readonly OHA_SHA256=620bb9e16fb53eabc9a3fc45f88bdb41fefa3fee5c05e75892011ce320391716

install_oha() {
  mkdir -p "$CI_STATE/load-bin"
  curl --fail --silent --show-error --location --output "$CI_STATE/load-bin/oha" \
    "https://github.com/hatoo/oha/releases/download/$OHA_VERSION/oha-linux-amd64"
  printf '%s  %s\n' "$OHA_SHA256" "$CI_STATE/load-bin/oha" \
    | sha256sum --check --status || fail "the oha download failed its SHA-256 check"
  chmod 700 "$CI_STATE/load-bin/oha"
  export PATH="$CI_STATE/load-bin:$PATH"
}

locate_app() {
  APP_ID=$(container app)
  APP_ID=$(docker inspect --format '{{.Id}}' "$APP_ID" 2>/dev/null) || APP_ID=
  [[ $APP_ID =~ ^[0-9a-f]{64}$ ]] \
    || fail "expected a 64-character lowercase hexadecimal app container ID"
  APP_CGROUP=$(sudo -n find /sys/fs/cgroup -type d \
    \( -name "$APP_ID" -o -name "docker-$APP_ID.scope" \) -print -quit 2>/dev/null) || APP_CGROUP=
  APP_CGROUP=${APP_CGROUP:-/sys/fs/cgroup/unavailable}
  APP_LIMIT=$(sudo -n cat "$APP_CGROUP/memory.max" 2>/dev/null) || APP_LIMIT=unavailable
}

read_memory() {
  local phase=$1 reading current oom maximum killed restarts
  locate_app
  # memory_cgroup records the shared fields. The Python checker owns every bound.
  reading=$(memory_cgroup app "$APP_CGROUP" "$APP_LIMIT") || true
  current=$(sudo -n cat "$APP_CGROUP/memory.current" 2>/dev/null) || current=unavailable
  [[ $current =~ ^[0-9]+$ ]] || current=unavailable
  oom=$(sudo -n awk '$1 == "oom" { print $2 }' "$APP_CGROUP/memory.events" 2>/dev/null) || oom=unavailable
  maximum=$(sudo -n awk '$1 == "max" { print $2 }' "$APP_CGROUP/memory.events" 2>/dev/null) || maximum=unavailable
  killed=$(docker inspect --format '{{.State.OOMKilled}}' "$APP_ID" 2>/dev/null) || killed=unavailable
  restarts=$(docker inspect --format '{{.RestartCount}}' "$APP_ID" 2>/dev/null) || restarts=unavailable
  python3 -I -B - "$CI_STATE/load-memory-$phase.json" "$reading" \
    "$oom" "$maximum" "$killed" "$restarts" "$current" "$APP_ID" <<'PY'
import json
import pathlib
import sys

fields = dict(part.split("=", 1) for part in sys.argv[2].splitlines()[0].split()[2:])
extra = ("oom", "max", "OOMKilled", "RestartCount", "current", "container_id")
fields.update(zip(extra, sys.argv[3:]))
pathlib.Path(sys.argv[1]).write_text(json.dumps(fields) + "\n", encoding="utf-8")
PY
}

recreate_app() {
  local deadline=$((SECONDS + 60))
  compose up -d --no-deps --no-build --force-recreate app
  while (( SECONDS < deadline ))
  do
    if [[ $(compose exec -T proxy wget -q -T 3 -O - http://app:9000/readyz) == ok ]]; then
      return 0
    fi
    sleep 1
  done
  fail "app did not become ready within 60 seconds"
}

flood_preflight() {
  locate_app
  sudo -n cat "$APP_CGROUP/cpu.max" >"$CI_STATE/load-cpu.max"
  printf '%s\n' "$APP_LIMIT" >"$CI_STATE/load-memory.max"
  sql -q -c 'SELECT count(*) FROM login_attempt WHERE NOT succeeded;' >"$CI_STATE/load-failed-logins"
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_flood.py" preflight \
    "$CI_STATE/load-cpu.max" "$CI_STATE/load-memory.max" "$CI_STATE/load-failed-logins"
}

flood_body() {
  curl --fail --silent --show-error --cacert "$CI_STATE/root.crt" \
    --cookie-jar "$CI_STATE/load-cookies" --output "$CI_STATE/load-login.html" https://localhost:8443/login
  python3 -I -B - "$GITHUB_WORKSPACE/deploy/ci" "$CI_STATE" <<'PY'
import http.cookiejar
import pathlib
import sys

sys.path.insert(0, sys.argv[1])
from http_probe import form_token

try:
    state = pathlib.Path(sys.argv[2])
    token = form_token((state / "load-login.html").read_text(), "/login", "csrf_pre")
    jar = http.cookiejar.MozillaCookieJar(str(state / "load-cookies"))
    # curl writes the expiry 0 for a session cookie. Python before 3.15 reads 0 as expired.
    jar.load(ignore_discard=True, ignore_expires=True)
    cookie, = [item for item in jar if item.name == "__Host-recipe_csrf"]
    (state / "load-csrf-pre").write_text(token, encoding="ascii")
    (state / "load-cookie-header").write_text(f"{cookie.name}={cookie.value}", encoding="ascii")
except Exception:
    print("CI flood could not read the pre-session token and cookie.", file=sys.stderr)
    sys.exit(1)
PY
  openssl rand -hex 16 >"$CI_STATE/load-password"
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_flood.py" body \
    "$CI_STATE/load-csrf-pre" "$CI_STATE/load-password" "$CI_STATE/load-bodies"
}

flood() {
  local started ended cookie status=0 oha_status=0
  install_oha
  recreate_app
  flood_preflight
  flood_body
  cookie=$(cat "$CI_STATE/load-cookie-header")
  started=$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)
  oha --ipv4 -n 200 -c 200 -m POST -Z "$CI_STATE/load-bodies" --cacert "$CI_STATE/root.crt" \
    -H 'Origin: https://localhost:8443' -H "Cookie: $cookie" \
    -H 'Content-Type: application/x-www-form-urlencoded' --no-tui --output-format json \
    https://localhost:8443/login >"$CI_STATE/load-oha.json" 2>"$CI_STATE/load-oha.stderr" || oha_status=$?
  ended=$(date -u +%Y-%m-%dT%H:%M:%S.%NZ)
  docker logs --since "$started" --until "$ended" "$APP_ID" >"$CI_STATE/load-app.log" 2>&1 || status=$?
  read_memory flood
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_flood.py" check \
    "$CI_STATE/load-oha.json" "$CI_STATE/load-app.log" --output "$CI_STATE/load-flood.json" || status=$?
  if (( oha_status != 0 )); then
    printf 'CI: oha failed with exit status %s\n' "$oha_status" >&2
    return "$oha_status"
  fi
  return "$status"
}

check_memory() {
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_memory.py" \
    "$CI_STATE/load-memory-timings.json" "$CI_STATE/load-memory-flood.json" \
    --output "$CI_STATE/load-memory.json" "$@"
}

check_value_memory() {
  local phase
  local -a readings=()
  for phase in value-seed value-precheck value-baseline-1 value-round-1 \
    value-baseline-2 value-round-2
  do
    readings+=(--value "$phase=$CI_STATE/load-memory-$phase.json")
  done
  check_memory "${readings[@]}"
}

seed_value() {
  date +%s >"$CI_STATE/load-value-leg-start"
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_seed.py" value
  read_memory value-seed
}

value_client() {
  local command=$1
  shift
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/load_value.py" \
    "$command" --ci-state "$CI_STATE" "$@"
}

read_value_versions() {
  local phase=$1 versions status
  if versions=$(sql -q 2>/dev/null <<'SQL'
SELECT pg_catalog.json_build_object(
    'row_versions', pg_catalog.json_object_agg(r.reference_number, v.row_version)
)
FROM public.recipe_version v
JOIN public.recipe r ON r.id = v.recipe_id
WHERE r.reference_number IN ('LOAD-V1', 'LOAD-V2', 'LOAD-V3', 'LOAD-V4', 'LOAD-V5')
  AND v.number = 1 AND v.status = 'draft' AND v.deleted_at IS NULL
HAVING count(*) = 5;
SQL
  ); then
    [[ -n $versions ]] || fail "expected five draft versions for value phase $phase"
    printf '%s\n' "$versions" >"$CI_STATE/load-value-versions-$phase.json"
  else
    status=$?
    printf 'CI: could not read draft versions for value phase %s\n' "$phase" >&2
    return "$status"
  fi
}

baseline_anon() {
  python3 -I -B - "$CI_STATE/load-memory-value-baseline-$1.json" <<'PY'
import json
import pathlib
import sys

try:
    reading = json.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
    anon = reading["anon"]
    if not isinstance(anon, str) or not anon.isascii() or not anon.isdecimal():
        raise ValueError
except (OSError, ValueError, KeyError, TypeError):
    sys.exit("CI: the value baseline has no valid anon reading")
print(anon)
PY
}

value_round() {
  local number=$1 anon client_status=0 reading_status=0 versions_status=0
  recreate_app
  read_memory "value-baseline-$number"
  anon=$(baseline_anon "$number")
  value_client round --round "$number" --cgroup "$APP_CGROUP" --reference-anon "$anon" \
    --url https://localhost:8443 --cacert "$CI_STATE/root.crt" || client_status=$?
  # Read memory and draft versions even after a client failure. Return the first failure.
  (read_memory "value-round-$number") || reading_status=$?
  (read_value_versions "$number") || versions_status=$?
  if (( client_status != 0 )); then
    return "$client_status"
  fi
  if (( reading_status != 0 )); then
    return "$reading_status"
  fi
  return "$versions_status"
}

value_precheck() {
  local number
  for number in 1 2
  do
    value_client single --round "$number" --reference LOAD-V5 \
      --url https://localhost:8443 --cacert "$CI_STATE/root.crt" || return "$?"
  done
  read_value_versions precheck || return "$?"
  value_client check-precheck
}

run_value() {
  local precheck_status=0 reading_status=0
  (value_precheck) || precheck_status=$?
  # Keep the pre-check reading after a failure. Return the first failure.
  (read_memory value-precheck) || reading_status=$?
  if (( precheck_status != 0 )); then
    return "$precheck_status"
  fi
  if (( reading_status != 0 )); then
    return "$reading_status"
  fi
  value_round 1
  value_client check-round --round 1
  value_round 2
}

case ${1:-all} in
  memory-timings) read_memory timings ;;
  value-seed) seed_value ;;
  flood) flood ;;
  value) run_value ;;
  memory-check) check_value_memory ;;
  value-check)
    value_client check --job-start "$RUNNER_TEMP/load-job-start" --summary "$GITHUB_STEP_SUMMARY"
    ;;
  all)
    read_memory timings
    status=0
    # Keep errexit active inside flood, then check memory even after a flood failure.
    bash "$GITHUB_WORKSPACE/deploy/ci/load.sh" flood || status=$?
    memory_status=0
    check_memory || memory_status=$?
    if (( status == 0 )); then status=$memory_status; fi
    exit "$status"
    ;;
  *) fail "usage: load.sh [memory-timings|value-seed|flood|value|memory-check|value-check|all]" ;;
esac
