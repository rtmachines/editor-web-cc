#!/usr/bin/env bash
# Test-only init script of the storage harness (ADR-0069 (u), data-model.md section 5.1).
#
# Production never mounts this file. The tests of recipe-db and recipe-server connect from the
# Docker host through a published port, so the source address is not in COMPOSE_SUBNET.
# The postgres image runs the init scripts in name order, so this script runs after 01-roles.sh.
# It puts the host lines of the harness at the top of pg_hba.conf. The lines of 01-roles.sh stay
# below them in the same order. The first matching line wins.
#
# Environment:
#   PGDATA  the data directory (set by the image)
set -euo pipefail
umask 077

hba_file="${PGDATA:?PGDATA is not set}/pg_hba.conf"
if [ ! -s "$hba_file" ]; then
  echo "02-test-hosts.sh: $hba_file is missing or empty" >&2
  exit 1
fi
{
  printf '# The host lines of the storage harness (02-test-hosts.sh).\n'
  printf 'host    all          all                all              scram-sha-256\n'
  printf 'host    replication  recipe_replicator  all              scram-sha-256\n'
  cat -- "$hba_file"
} > "${hba_file}.new"
mv -- "${hba_file}.new" "$hba_file"
