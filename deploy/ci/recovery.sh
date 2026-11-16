#!/usr/bin/env bash
# D11 step 7. The marker follows the base backup (ADR-0077).
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/memory.sh"

delete_set_copy() {
  local file status=0
  sudo rm -f -- "${copy_files[@]}" || status=1
  for file in "${copy_files[@]}"
  do
    if sudo test -e "$file" || sudo test -L "$file"; then
      status=1
    fi
  done
  if (( status != 0 )); then
    printf 'CI: cannot delete the set copy\n' >&2
  fi
  return "$status"
}

check_changed_set() (
  local set=$1 stamp copy_set suffix file status=0 result
  stamp=$(date -u -d "${set:7:8} ${set:16:2}:${set:18:2} UTC + 1 minute" +%Y%m%d_%H%M)_now
  copy_set=recipe_$stamp
  local copy_files=("$RECIPE_BACKUP_DIR/$copy_set.dump.sha256" "$RECIPE_BACKUP_DIR/$copy_set.dump.sha256.hmac"
    "$RECIPE_BACKUP_DIR/$copy_set.dump" "$RECIPE_BACKUP_DIR/globals_$stamp.sql" "$RECIPE_BACKUP_DIR/$copy_set.counts")
  for file in "${copy_files[@]}"
  do
    if sudo test -e "$file" || sudo test -L "$file"; then
      fail "a file of the set copy already exists"
    fi
  done
  trap 'status=$?; trap - EXIT; delete_set_copy || status=1; exit "$status"' EXIT
  trap 'exit 143' TERM
  trap 'exit 130' INT
  trap 'exit 129' HUP
  for suffix in dump counts dump.sha256 dump.sha256.hmac
  do
    sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/$set.$suffix" "$RECIPE_BACKUP_DIR/$copy_set.$suffix"
  done
  sudo install -o 999 -g 999 -m 0600 "$RECIPE_BACKUP_DIR/globals_${set#recipe_}.sql" "$RECIPE_BACKUP_DIR/globals_$stamp.sql"
  # A custom dump starts with PGDMP. Replace its first byte and keep its size.
  printf '\000' | sudo dd of="$RECIPE_BACKUP_DIR/$copy_set.dump" bs=1 count=1 conv=notrunc status=none
  sudo bash -euo pipefail -c '
    cd "$1"
    sha256sum -- "$2.dump" "globals_${2#recipe_}.sql" "$2.counts" >"$2.dump.sha256"
  ' bash "$RECIPE_BACKUP_DIR" "$copy_set"
  memory_exec backup restore-test | tee "$CI_STATE/restore-tag-mismatch.log" || status=$?
  (( status != 0 )) || fail "restore-test accepted the changed set"
  [[ $(grep -c '"level":"ERROR"' "$CI_STATE/restore-tag-mismatch.log") == 1 ]] \
    || fail "restore-test did not write one tag ERROR line"
  grep -Eq '"level":"ERROR","job":"restore-test","message":"the tag of the set does not match the manifest","set":"'"$copy_set"'"}$' \
    "$CI_STATE/restore-tag-mismatch.log" || fail "restore-test did not refuse the changed set at the tag check"
  result=$(sudo cat "$RECIPE_BACKUP_DIR/restore-test.result")
  printf '%s\n' "$result"
  printf '%s\n' "$result" | grep -Eq '^\{"result":"failed","finished_at":"[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z"\}$' \
    || fail "restore-test did not write the result failed"
)

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
set=$(sed -nE 's/.*"message":"restore test passed".*"set":"(recipe_[0-9]{8}_[0-9]{4}_now)".*/\1/p' "$CI_STATE/restore.log")
[[ $set =~ ^recipe_[0-9]{8}_[0-9]{4}_now$ ]] || fail "restore-test did not name one now set"
sudo test -f "$RECIPE_BACKUP_DIR/$set.dump.sha256.hmac" || fail "the now set has no tag file"
printf 'CI: the tag file %s.dump.sha256.hmac exists\nCI: real pg_restore passed for %s\n' "$set" "$set"
check_changed_set "$set"
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
