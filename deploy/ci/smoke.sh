#!/usr/bin/env bash
# ADR-0128 (aj) and (ak). Scan smoke artifacts before publication.
set -euo pipefail
umask 077
# shellcheck source=deploy/ci/common.sh
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mask_password() {
  local value=$1
  value=${value//'%'/'%25'}
  value=${value//$'\r'/'%0D'}
  value=${value//$'\n'/'%0A'}
  printf '::add-mask::%s\n' "$value"
}

admin_password=$(<"$CI_STATE/new_password")
mask_password "$admin_password"
approver_password=$(openssl rand -hex 16)
mask_password "$approver_password"

# A fresh directory keeps earlier artifacts outside this scan and upload.
upload=$CI_STATE/smoke-upload
mkdir -- "$upload"
artifacts=$(mktemp -d "$CI_STATE/smoke-artifacts.XXXXXX")
sessions=$(mktemp -d "$CI_STATE/smoke-sessions.XXXXXX")
one_time_file=$(mktemp "$CI_STATE/smoke-one-time.XXXXXX")
trap 'rm -f -- "$one_time_file"' EXIT
mkdir -- "$artifacts/test-results" "$artifacts/playwright-report"

cd "$GITHUB_WORKSPACE/e2e"
status=0
E2E_ADMIN_LOGIN=ci-admin \
E2E_ADMIN_PASSWORD=$admin_password \
E2E_APPROVER_PASSWORD=$approver_password \
E2E_ONE_TIME_FILE=$one_time_file \
E2E_BASE_URL=https://localhost:8443 \
E2E_STATE_DIR=$sessions \
PLAYWRIGHT_HTML_OUTPUT_DIR=$artifacts/playwright-report \
  npx playwright test -c playwright.compose.config.ts --output "$artifacts/test-results" \
  >"$artifacts/playwright.log" 2>&1 || status=$?

# Child output can contain a password in a failure message. Scan it too.
E2E_ONE_TIME_FILE=$one_time_file \
E2E_ADMIN_PASSWORD=$admin_password E2E_APPROVER_PASSWORD=$approver_password \
  python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/secret_scan.py" "$artifacts"
rm -f -- "$one_time_file"
trap - EXIT
unset admin_password approver_password

mv -- "$artifacts/test-results" "$artifacts/playwright-report" "$artifacts/playwright.log" "$upload/"
printf 'CI smoke artifacts passed the scan. Playwright exit status: %s.\n' "$status"
exit "$status"
