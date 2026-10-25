#!/usr/bin/env bash
# D11 step 11. Never print raw logs or the full container configuration.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

mkdir -p -- "$CI_STATE"
# Print only the numeric measurements or the fixed availability message.
if [[ -f $CI_STATE/memory.log ]]; then
  grep -E '^memory\.peak ([a-z-]+ [0-9]+ of [0-9]+|unavailable)$' "$CI_STATE/memory.log" || true
fi
compose ps -a || true
compose logs --no-color >"$CI_STATE/compose.log" 2>&1 || true
sudo python3 -I -B "$GITHUB_WORKSPACE/deploy/ci/redact.py" "$CI_ROOT"
