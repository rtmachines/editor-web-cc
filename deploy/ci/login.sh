#!/usr/bin/env bash
# D11 step 5. The administrator keeps must_change_password until step 7.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

compose run --rm --no-deps -T \
  -v "$RECIPE_SECRETS_DIR/admin_password:/run/secrets/admin_password:ro" \
  app create-admin --login ci-admin --password-file /run/secrets/admin_password
docker cp "$(container proxy):/data/caddy/pki/authorities/local/root.crt" "$CI_STATE/root.crt"
http_probe login
http_probe health
