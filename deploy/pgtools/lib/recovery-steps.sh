# recovery-steps.sh: the steps that pitr-check and pitr-volume share.
#
# Source: ADR-0106 (h), ADR-0068 (l) and (o), ADR-0077 (e), design D1 of the change pitr-recovery.
# A script sources this file after it sets its own constants. The file only defines names and
# functions, and it runs nothing.
#
# The sourcing script sets these names:
#   SCRATCH_PORT            scratch.port of reliability.md 7.11
#   SCRATCH_SUPERUSER       the superuser name of the scratch cluster
#   DB_NAME                 the database of the connection wait
#   READY_POLL_SECONDS      pitr_check.ready_poll of reliability.md 7.11
#   LOG_TAIL_LINES          pitr_check.log_tail of reliability.md 7.11
#   fail                    the function that writes the one ERROR line and stops
#   scratch_psql            the function that runs one statement on the recovered cluster
#   CHAIN_SQL               the audit chain check, defined in restore-steps.sh. The script sources that file first.
# The function recovery_start_server closes file descriptor 9, which holds the lock of the WAL archive.
# shellcheck shell=bash
# shellcheck disable=SC2034

# recovery_restore_command <WAL directory>: print the restore_command of ADR-0068 (l).
# The fallback replays the transactions of the partial last segment.
recovery_restore_command() {
  printf 'cp %s/%%f %%p || cp %s/%%f.partial %%p' "$1" "$1"
}

# recovery_start_options <WAL directory> <socket directory>: print the argument for pg_ctl -o.
# It holds the settings of the scratch cluster (scratch.* of reliability.md 7.11). The caller adds more.
# pg_ctl runs the string through a shell, so the restore_command stays in double quotes.
# hot_standby=off: the lower max_connections cannot stop the recovery (ADR-0068 (o)).
# The string sets no fsync option, because only restore-test may do so (ADR-0067 (f)).
# No setting goes into a configuration file, so the copy of a cluster keeps its configuration files.
recovery_start_options() {
  local options
  options="-c shared_buffers=64MB -c maintenance_work_mem=64MB -c max_connections=10"
  options+=" -c max_parallel_workers_per_gather=0 -c max_parallel_maintenance_workers=0"
  options+=" -c hot_standby=off -c listen_addresses='' -c port=$SCRATCH_PORT"
  options+=" -c unix_socket_directories=$2 -c archive_mode=off"
  options+=" -c \"restore_command=$(recovery_restore_command "$1")\""
  printf '%s' "$options"
}

# recovery_wait_for_connections <data directory> <socket directory> <deadline in SECONDS>.
# pg_ctl can report standby before hot_standby=off permits connections.
# Use the remaining budget of the caller and stop when the postmaster exits.
recovery_wait_for_connections() {
  local data_dir=$1 socket_dir=$2 deadline=$3
  while [ "$SECONDS" -lt "$deadline" ]; do
    [ -f "$data_dir/postmaster.pid" ] || return 1
    pg_ctl --pgdata="$data_dir" status >/dev/null 2>&1 || return 1
    if pg_isready --quiet --host="$socket_dir" --port="$SCRATCH_PORT" \
      --username="$SCRATCH_SUPERUSER" --dbname="$DB_NAME" --timeout="$READY_POLL_SECONDS"; then
      [ "$SECONDS" -lt "$deadline" ]
      return
    fi
    sleep "$READY_POLL_SECONDS"
  done
  return 1
}

# recovery_log_tail <server log>: write the last lines of the server log to standard error.
recovery_log_tail() {
  tail -n "$LOG_TAIL_LINES" -- "$1" >&2 || true
}

# recovery_start_server <data directory> <server log> <socket directory> <options> <deadline>:
# start the server with the settings in <options> and wait until it accepts connections.
# It returns 1 after the log tail when the budget ends, the start fails, or the postmaster exits.
# The deadline is a value of SECONDS. The caller writes the one ERROR line.
recovery_start_server() {
  local data_dir=$1 log_file=$2 socket_dir=$3 options=$4 deadline=$5
  if [ "$SECONDS" -ge "$deadline" ] \
    || ! pg_ctl --pgdata="$data_dir" --log="$log_file" --wait \
      --timeout="$((deadline - SECONDS))" -o "$options" start 9>&- >&2 \
    || ! recovery_wait_for_connections "$data_dir" "$socket_dir" "$deadline"; then
    recovery_log_tail "$log_file"
    return 1
  fi
}

# recovery_verify_copy <copy>: check a copy of a base backup and remove the end time file from it.
# basebackup-once adds the file backup_end_time to the base backup (ADR-0106 (c)). The manifest does
# not list it, so the check ignores that one file. Any other file outside the manifest still fails.
# The delete follows the check, so the server never starts with the file in its data directory.
# The caller writes the one ERROR line when this function returns 1.
recovery_verify_copy() {
  pg_verifybackup --quiet --ignore=backup_end_time "$1" >&2 || return 1
  rm -f -- "$1/backup_end_time"
}

# recovery_check_marker <chain_seq> <entry_hash as hex>: exactly one row has the marker identity
# (ADR-0077 (e)). The caller passes values that passed the grammar check of ADR-0077 (b).
recovery_check_marker() {
  local found
  found=$(scratch_psql "SELECT count(*) FROM public.audit_log WHERE chain_seq = $1 AND entry_hash = pg_catalog.decode('$2', 'hex')") \
    || fail "cannot read the marker entry in the recovered cluster"
  [ "$found" = "1" ] || fail "the marker entry is missing in the recovered cluster"
}

# recovery_check_chain: the audit chain check of data-model.md 4.9 gives no break and a matching head.
recovery_check_chain() {
  local chain
  chain=$(scratch_psql "$CHAIN_SQL") || fail "cannot run the audit chain check on the recovered cluster"
  [ "$chain" = "0|t|t" ] \
    || fail "the audit chain check failed on the recovered cluster" "\"result\":\"$chain\""
}
