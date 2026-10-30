#!/usr/bin/env bash
# Delete only the disposable resources of this GitHub job.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"

# The runner owns these volumes. No production project uses this unique name.
status=0
compose down --volumes --remove-orphans || status=$?
# delete_ci_volumes removes only the volumes that carry the label of this job.
delete_ci_volumes || status=$?
for disk in backups walarchive-disk
do
  if mountpoint -q "$CI_ROOT/$disk"
  then
    sudo umount "$CI_ROOT/$disk" || status=$?
  fi
done
exit "$status"
