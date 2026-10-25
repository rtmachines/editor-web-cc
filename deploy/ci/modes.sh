#!/usr/bin/env bash
# Check the modes of the new set and the WAL archive.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

[[ $(sudo stat -c %a "$RECIPE_BACKUP_DIR") == 755 ]]
[[ $(sudo stat -c %a "$RECIPE_WALARCHIVE_DIR") == 700 ]]
found=false
for manifest in "$RECIPE_BACKUP_DIR"/recipe_*_now.dump.sha256
do
  [[ -f $manifest ]] || continue
  base=${manifest%.dump.sha256}
  stamp=${base##*/recipe_}
  for file in "$manifest" "$base.dump" "$base.counts" "$RECIPE_BACKUP_DIR/globals_$stamp.sql"
  do
    [[ $(sudo stat -c '%a:%u' "$file") == 600:999 ]] || fail "a backup file has the wrong mode or owner"
  done
  found=true
done
$found || fail "no new backup set exists"
compose exec -T walarchive bash -eu -c '
  found=false
  for path in /wal/base/base_*
  do
    test -d "$path" || continue
    test "$(stat -c %a "$path")" = 700
    found=true
  done
  "$found"
'
