#!/usr/bin/env bash
# D11 step 7. The marker follows the base backup (ADR-0077).
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/memory.sh"

# A later minute makes the new set sort after the initial premigration set.
deadline=$((SECONDS + 65))
while sudo test -f "$RECIPE_BACKUP_DIR/recipe_$(date -u +%Y%m%d_%H%M)_premigrate.dump.sha256"
do
  (( SECONDS < deadline )) || fail "the backup minute did not advance"
  sleep 1
done
memory_exec backup backup-now
memory_exec backup restore-test | tee "$CI_STATE/restore.log"
grep -Eq '"message":"restore test passed".*"set":"recipe_[0-9]{8}_[0-9]{4}_now"' "$CI_STATE/restore.log" \
  || fail "restore-test did not prove the new now set"
# The base backup runs in walarchive, so read that container's cumulative peak too.
memory_exec walarchive basebackup-once
http_probe password
marker=$(sql -c "SELECT chain_seq, encode(entry_hash, 'hex'), action FROM public.audit_log ORDER BY chain_seq DESC LIMIT 1")
IFS='|' read -r PITR_MARKER_SEQ PITR_MARKER_HASH action <<<"$marker"
[[ $PITR_MARKER_SEQ =~ ^[1-9][0-9]{0,18}$ && $PITR_MARKER_HASH =~ ^[0-9a-f]{64}$ ]] \
  || fail "the audit marker identity is invalid"
[[ $action == OwnPasswordChanged ]] || fail "the latest audit row is not OwnPasswordChanged"
position=$(sql -c 'SELECT pg_current_wal_lsn()')
[[ $position =~ ^[0-9A-F]+/[0-9A-F]+$ ]] || fail "the WAL position is invalid"
timeout 60s bash -euo pipefail -c '
  source "$GITHUB_WORKSPACE/deploy/ci/common.sh"
  while true
  do
    flushed=$(sql -c "$1")
    [[ $flushed == t ]] && exit 0
    sleep 1
  done
' bash "SELECT EXISTS (SELECT 1 FROM pg_stat_replication WHERE application_name = 'walarchive' AND usename = 'recipe_replicator' AND flush_lsn >= '$position'::pg_lsn)" \
  || fail "walarchive did not flush the marker within 60 seconds"
segment=$(sql -c "SELECT pg_walfile_name('$position'::pg_lsn - 1)")
[[ $segment =~ ^[0-9A-F]{24}$ ]] || fail "the WAL segment name is invalid"
sudo test -f "$RECIPE_WALARCHIVE_DIR/$segment.partial" \
  || fail "the marker segment has no .partial file"
if sudo test -e "$RECIPE_WALARCHIVE_DIR/$segment"; then
  fail "the marker segment is complete, not partial"
fi
export PITR_MARKER_SEQ PITR_MARKER_HASH
memory_pitr
bash "$GITHUB_WORKSPACE/deploy/ci/modes.sh"
