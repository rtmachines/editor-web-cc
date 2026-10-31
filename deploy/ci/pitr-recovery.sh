#!/usr/bin/env bash
# Job pitr-recovery (pitr-recovery design.md D8, ADR-0106 (p)).
# This file holds the steps prepare, markers, drill, refusals, crash, recover, switch, wal-line, check, refuse, and cleanup.
set -euo pipefail
umask 077
source "$(dirname "${BASH_SOURCE[0]}")/common.sh"
source "$(dirname "${BASH_SOURCE[0]}")/switch-steps.sh"

prepare() {
  # ADR-0106 (p) step 1. The login step creates the administrator. The probe password changes the password once.
  log "pitr-recovery prepare: start the stack"
  bash "$GITHUB_WORKSPACE/deploy/ci/start.sh"
  log "pitr-recovery prepare: log in and change the password"
  bash "$GITHUB_WORKSPACE/deploy/ci/login.sh"
  http_probe password
  log "pitr-recovery prepare: take the first base backup"
  take_base_backup prepare-base.log "pitr-recovery prepare"
  log "pitr-recovery prepare: done"
}

# The grammars of the marker values (ADR-0077 (b)) and of the target time (ADR-0106 (a)).
MARKER_SEQ_PATTERN='^[1-9][0-9]{0,18}$'
MARKER_HASH_PATTERN='^[0-9a-f]{64}$'
MARKER_ACTION=OwnPasswordChanged
TARGET_TIME_PATTERN='^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z$'
# The next full second of the clock of db, in UTC. The text has the form of the target time.
TARGET_SQL="SELECT to_char(date_trunc('second', clock_timestamp() AT TIME ZONE 'UTC') + interval '1 second', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')"
# The name of a drill volume and two fields of the completed line of pitr-volume (ADR-0106 (f) step 12).
DRILL_VOLUME_PATTERN='^recipe-editor_pgdata_drill_[0-9]{8}_[0-9]{6}$'
LINE_SEQ_PATTERN='"chain_seq":([0-9]+)[,}]'
LINE_ACTION_PATTERN='"action":"([A-Za-z]+)"'

change_own_password() {
  # The first argument is the current password. The second is the next one.
  # http_probe reads the current password from admin_password and the next one from new_password.
  # After the step, new_password holds the last password and admin_password holds the one before it.
  printf '%s\n' "$1" >"$CI_STATE/admin_password"
  printf '%s\n' "$2" >"$CI_STATE/new_password"
  http_probe password
}

commit_marker() {
  # ADR-0077 (a) and (b). The first argument is the number of the marker. The newest audit row is the marker.
  # The second argument is the step name of the error lines. The default is markers.
  local current next row seq hash action step=${2:-markers}
  current=$(<"$CI_STATE/new_password")
  next=$(openssl rand -hex 32)
  [[ $next =~ $MARKER_HASH_PATTERN ]] || fail "$step: openssl gave no new password."
  change_own_password "$current" "$next" || fail "$step: the command ChangeOwnPassword of marker $1 failed."
  row=$(sql -c "SELECT chain_seq, encode(entry_hash, 'hex'), action FROM public.audit_log ORDER BY chain_seq DESC LIMIT 1") \
    || fail "$step: cannot read the audit marker $1."
  IFS='|' read -r seq hash action <<<"$row"
  [[ $seq =~ $MARKER_SEQ_PATTERN && $hash =~ $MARKER_HASH_PATTERN ]] \
    || fail "$step: the audit marker identity of marker $1 is invalid."
  [[ $action == "$MARKER_ACTION" ]] || fail "$step: the newest audit entry of marker $1 is not $MARKER_ACTION."
  printf '%s\n' "$seq" >"$CI_STATE/marker$1.seq"
  printf '%s\n' "$hash" >"$CI_STATE/marker$1.hash"
}

clock_passed() {
  # True when the clock of db is one second after the target time.
  local reached
  reached=$(sql -c "SELECT clock_timestamp() >= '$1'::timestamptz + interval '1 second'") || return 1
  [[ $reached == t ]]
}

require_target_between_markers() {
  # The commit of marker 1 lies before the target time. The commit of marker 2 lies at or after it.
  # The arguments are the target time and the sequence numbers of the two markers.
  local inside
  inside=$(sql -c "SELECT (SELECT occurred_at FROM public.audit_log WHERE chain_seq = $2) < '$1'::timestamptz AND (SELECT occurred_at FROM public.audit_log WHERE chain_seq = $3) >= '$1'::timestamptz AND $2::bigint < $3::bigint") \
    || fail "markers: cannot compare the target time with the commits."
  [[ $inside == t ]] || fail "markers: the target time does not lie between the two commits."
}

wait_marker_flush() {
  # ADR-0077 (c) and ADR-0106 (p) steps 3 and 5. The argument is the step name. The function sets FLUSHED_POSITION.
  # No segment switch runs, so the segment of the position stays a .partial file and never exists as a complete file.
  local segment
  FLUSHED_POSITION=$(sql -c 'SELECT pg_current_wal_lsn()') || fail "$1: cannot read the WAL position."
  [[ $FLUSHED_POSITION =~ $LSN_PATTERN ]] || fail "$1: the WAL position is invalid."
  retry flush_reached "$FLUSHED_POSITION" || fail "$1: the flush check did not return t."
  segment=$(sql -c "SELECT pg_walfile_name('$FLUSHED_POSITION'::pg_lsn - 1)") || fail "$1: cannot read the WAL segment name."
  [[ $segment =~ ^[0-9A-F]{24}$ ]] || fail "$1: the WAL segment name is invalid."
  sudo test -f "$RECIPE_WALARCHIVE_DIR/$segment.partial" || fail "$1: the marker segment has no .partial file."
  if sudo test -e "$RECIPE_WALARCHIVE_DIR/$segment"
  then
    fail "$1: the marker segment is complete, not partial."
  fi
}

# The job heartbeat writes the row heartbeat.last_at one interval after app starts (reliability.md 7.11).
# The wait for the first row lasts the interval plus a margin. The tests lower the limit.
HEARTBEAT_INTERVAL_SECONDS=300
HEARTBEAT_MARGIN_SECONDS=60
HEARTBEAT_WAIT_SECONDS=${CI_HEARTBEAT_WAIT_SECONDS:-$((HEARTBEAT_INTERVAL_SECONDS + HEARTBEAT_MARGIN_SECONDS))}
HEARTBEAT_ROW_SQL="SELECT count(*) FROM public.app_setting WHERE key = 'heartbeat.last_at'"

heartbeat_row_exists() {
  # True when app_setting holds the row heartbeat.last_at. The recovered cluster needs it (pitr-volume).
  local rows
  rows=$(sql -c "$HEARTBEAT_ROW_SQL") || return 1
  [[ $rows =~ ^[1-9][0-9]*$ ]]
}

wait_heartbeat_row() {
  # One attempt takes one second, so the number of attempts is the limit in seconds.
  log "markers: wait for the row heartbeat.last_at, at most $HEARTBEAT_WAIT_SECONDS s"
  WAIT_ATTEMPTS=$HEARTBEAT_WAIT_SECONDS retry heartbeat_row_exists \
    || fail "markers: the job heartbeat wrote no row heartbeat.last_at in $HEARTBEAT_WAIT_SECONDS s."
  log "markers: the row heartbeat.last_at exists"
}

markers() {
  # ADR-0106 (p) steps 2 and 3. The target time is the next full second after the commit of marker 1.
  # The clock of db gives the second. The answer of the HTTP request proves that the commit is done.
  # Marker 2 commits one second after the target time, so the target time lies between the two commits.
  # The recovered cluster must hold the row heartbeat.last_at, so marker 1 waits for it.
  local target
  [[ -s $CI_STATE/new_password && -s $CI_STATE/cookies.json ]] || fail "markers: run prepare first."
  wait_heartbeat_row
  log "markers: commit marker 1 with ChangeOwnPassword"
  commit_marker 1
  target=$(sql -c "$TARGET_SQL") || fail "markers: cannot read the clock of db."
  [[ $target =~ $TARGET_TIME_PATTERN ]] || fail "markers: the target time is invalid."
  log "markers: the target time is $target. Wait until the clock of db is one second after it"
  retry clock_passed "$target" || fail "markers: the clock of db did not pass the target time."
  printf '%s\n' "$target" >"$CI_STATE/target.time"
  log "markers: commit marker 2 with ChangeOwnPassword"
  commit_marker 2
  require_target_between_markers "$target" "$(<"$CI_STATE/marker1.seq")" "$(<"$CI_STATE/marker2.seq")"
  log "markers: wait for the flush of marker 2"
  wait_marker_flush markers
  log "markers: done"
}

create_labelled_volume() {
  # The first argument is the step name. The second is the volume name. The label lets delete_ci_volumes find the volume.
  docker volume create --label "recipe.ci=$COMPOSE_PROJECT_NAME" "$2" >/dev/null \
    || fail "$1: cannot create the volume $2."
}

create_scratch_volume() {
  # The first argument is the step name. The function sets SCRATCH_VOLUME and saves the name for delete_drill_volume.
  SCRATCH_VOLUME=recipe-editor_pgdata_drill_$(date -u +%Y%m%d_%H%M%S)
  create_labelled_volume "$1" "$SCRATCH_VOLUME"
  printf '%s\n' "$SCRATCH_VOLUME" >"$CI_STATE/drill.volume"
}

delete_drill_volume() {
  # Delete only the volume that drill created. Its name is in the state file. The label of the job must match.
  local status volume
  [[ -s $CI_STATE/drill.volume ]] || return 0
  volume=$(<"$CI_STATE/drill.volume")
  [[ $volume =~ $DRILL_VOLUME_PATTERN ]] || fail "drill: the drill volume name is invalid."
  delete_labelled_volume "$volume" || {
    status=$?
    [[ $status -ne $VOLUME_LABEL_MISMATCH ]] || fail "drill: the drill volume $volume has no label of this job."
    fail "drill: cannot delete the drill volume $volume."
  }
  rm -f -- "$CI_STATE/drill.volume"
}

drill_exit() {
  # A failed drill still deletes its volume. The status of the failure stays. A failed deletion writes its own line.
  local status=$?
  trap - EXIT
  (delete_drill_volume) || status=1
  exit "$status"
}

check_pitr_line() {
  # ADR-0106 (p) steps 4 and 6. The arguments are the step name, the expected value of the field drill,
  # the sequence number of the marker, and the number of the marker. A drill run writes a drill volume. Another run writes the new volume.
  local last drill kind=new
  last=$(tail -n 1 "$CI_STATE/pitr-volume.log")
  [[ $last =~ $PITR_MESSAGE_PATTERN ]] || fail "$1: the last line of pitr-volume is wrong."
  [[ $last =~ $PITR_DRILL_PATTERN ]] || fail "$1: the last line of pitr-volume is wrong."
  drill=${BASH_REMATCH[1]}
  [[ $drill == "$2" ]] || fail "$1: the completed line of pitr-volume has drill $drill."
  if [[ $2 == true ]]
  then
    kind=drill
  fi
  [[ $last =~ $LINE_SEQ_PATTERN && ${BASH_REMATCH[1]} == "$3" ]] \
    || fail "$1: the newest audit entry of the $kind volume is not marker $4."
  [[ $last =~ $LINE_ACTION_PATTERN && ${BASH_REMATCH[1]} == "$MARKER_ACTION" ]] \
    || fail "$1: the newest audit entry of the $kind volume is not marker $4."
}

expect_switch_refusals() {
  # The switch refuses a drill volume by its file and by the drill field of the line (ADR-0106 (j)). Both checks must stop.
  local output
  if output=$( (refuse_drill_volume "$1") 2>&1 )
  then
    fail "drill: the switch check accepted the drill volume."
  fi
  [[ $output == "CI: switch: the new volume holds the file pitr-drill. A drill volume never becomes production." ]] \
    || fail "drill: the switch check refused for another reason."
  if output=$( (read_pitr_line) 2>&1 )
  then
    fail "drill: the switch check accepted the line of a drill."
  fi
  [[ $output == "CI: switch: the line of pitr-volume comes from a drill. A drill volume never becomes production." ]] \
    || fail "drill: the switch check refused the line for another reason."
}

drill() {
  # ADR-0106 (o) and (p) step 4. db and app keep running on the production volume. The drill volume has its own name.
  local name seq hash target volume
  for name in marker1.seq marker1.hash target.time
  do
    [[ -s $CI_STATE/$name ]] || fail "drill: run markers first."
  done
  seq=$(<"$CI_STATE/marker1.seq")
  hash=$(<"$CI_STATE/marker1.hash")
  target=$(<"$CI_STATE/target.time")
  [[ $seq =~ $MARKER_SEQ_PATTERN ]] || fail "drill: the sequence number of marker 1 is invalid."
  [[ $hash =~ $MARKER_HASH_PATTERN ]] || fail "drill: the hash of marker 1 is invalid."
  [[ $target =~ $TARGET_TIME_PATTERN ]] || fail "drill: the target time is invalid."
  trap drill_exit EXIT
  create_scratch_volume drill
  volume=$SCRATCH_VOLUME
  log "drill: run pitr-volume with the target time $target into the volume $volume"
  export PITR_MARKER_SEQ=$seq PITR_MARKER_HASH=$hash
  compose run --rm --no-deps -T -e PITR_MARKER_SEQ -e PITR_MARKER_HASH -v "$volume:/var/lib/postgresql/data" \
    pitr-volume --target-time "$target" --drill | tee "$CI_STATE/pitr-volume.log" || fail "drill: pitr-volume failed."
  log "drill: check the completed line"
  check_pitr_line drill true "$seq" 1
  log "drill: run the switch checks on the drill volume. Both must refuse"
  expect_switch_refusals "$volume"
  log "drill: delete the drill volume"
  delete_drill_volume
  trap - EXIT
  log "drill: done"
}

# The ERROR lines that pitr-volume writes (ADR-0106 (a) and (g) step 2). A target after the newest commit ends the recovery with FATAL.
TARGET_ERROR="the temporary server did not finish the recovery"
GAP_ERROR="the recovery ended before the end of the WAL archive"
# The word @JOB@ becomes the job that writes the line. The step refuse expects a line of walarchive-entry.
ERROR_LINE_PATTERN='^\{"timestamp":"[^"]+","level":"ERROR","job":"@JOB@","message":"([^"]*)"\}$'
# The ERROR line of the guard of walarchive-entry for a segment of another timeline (ADR-0106 (m)).
TIMELINE_ERROR="segments of another timeline are in the WAL archive"
# The name of the running step and the job that writes its ERROR line. The step refuse sets both.
# The shared functions use them in their messages. The step refusals runs with the defaults.
ERROR_STEP=refusals
ERROR_JOB=pitr-volume
# One hour after the clock of db lies after every commit that the archive can hold when the run starts.
REFUSAL_TARGET_SQL="SELECT to_char(date_trunc('second', clock_timestamp() AT TIME ZONE 'UTC') + interval '1 hour', 'YYYY-MM-DD\"T\"HH24:MI:SS\"Z\"')"
# The copy of /wal lives in this directory of the CI state. The state file names it for the cleanup.
WAL_COPY_DIR=$CI_STATE/wal-copy
WAL_COPY_STATE=$CI_STATE/refusals.wal-copy
# The gap run needs this many complete segments after the start segment: one to delete and the newest one to keep.
# The step switches the segment this many times. The message of wait_gap_segments names the number as a word.
GAP_SEGMENT_COUNT=2

production_facts() {
  # The facts that a run of pitr-volume must not change: the data volume of db, the container of db, and the volumes of the job.
  printf 'data volume: %s\n' "$(db_volume)"
  docker inspect --format 'container {{.Id}} {{.State.StartedAt}}' "$(container db)"
  docker volume ls -q --filter "label=recipe.ci=$COMPOSE_PROJECT_NAME" | sort
}

delete_wal_copy() {
  # Delete only the directory that refusals or refuse created. The state file names it.
  [[ -s $WAL_COPY_STATE ]] || return 0
  [[ $(<"$WAL_COPY_STATE") == "$WAL_COPY_DIR" ]] || fail "$ERROR_STEP: the state file names another WAL copy."
  sudo rm -rf -- "$WAL_COPY_DIR" || fail "$ERROR_STEP: cannot delete the WAL copy."
  rm -f -- "$WAL_COPY_STATE"
}

refusals_exit() {
  # A failed step still deletes its scratch volume and its WAL copy. The status of the failure stays.
  local status=$?
  trap - EXIT
  (delete_drill_volume) || status=1
  (delete_wal_copy) || status=1
  exit "$status"
}

expect_pitr_error() {
  # The first argument names the refusal. The second is the message of the ERROR line. The others follow "compose run".
  # The job ERROR_JOB must fail and its last line must be the one ERROR line with that message.
  local label=$1 message=$2 last log_file=$CI_STATE/$ERROR_STEP.log pattern=${ERROR_LINE_PATTERN/@JOB@/$ERROR_JOB}
  shift 2
  if compose run --rm --no-deps -T "$@" | tee "$log_file"
  then
    fail "$ERROR_STEP: $ERROR_JOB accepted $label."
  fi
  last=$(tail -n 1 "$log_file")
  [[ $(grep -c '"level":"ERROR"' "$log_file" || true) == 1 && $last =~ $pattern \
    && ${BASH_REMATCH[1]} == "$message" ]] || fail "$ERROR_STEP: the ERROR line of $ERROR_JOB is wrong for $label."
}

copy_wal_archive() {
  # Copy /wal into the CI state. The state file names the copy, so delete_wal_copy can delete it.
  # The real /wal is only the source of the copy. Nothing writes to it.
  if sudo test -e "$WAL_COPY_DIR"
  then
    fail "$ERROR_STEP: the directory of the WAL copy exists."
  fi
  printf '%s\n' "$WAL_COPY_DIR" >"$WAL_COPY_STATE"
  sudo install -d -o 999 -g 999 -m 0700 "$WAL_COPY_DIR" || fail "$ERROR_STEP: cannot create the WAL copy."
  sudo cp -a -- "$RECIPE_WALARCHIVE_DIR/." "$WAL_COPY_DIR/" || fail "$ERROR_STEP: cannot copy /wal."
}

base_start_segment() {
  # The argument is an archive directory. Print the start segment of its newest base backup.
  local base start
  base=$(sudo find "$1/base" -mindepth 1 -maxdepth 1 -type d -regextype posix-extended -regex "$BASE_REGEX" -printf '%f\n' | sort | tail -n 1) \
    || fail "refusals: cannot list the base backups of $1."
  [[ $base =~ $BASE_NAME_PATTERN ]] || fail "refusals: $1 holds no base backup."
  start=$(sudo sed -En 's/^START WAL LOCATION: .* \(file ([0-9A-F]{24})\)$/\1/p' "$1/base/$base/backup_label") \
    || fail "refusals: cannot read the backup label of $base."
  [[ $start =~ ^[0-9A-F]{24}$ ]] || fail "refusals: the start segment of the base backup is invalid."
  printf '%s\n' "$start"
}

complete_segments_after() {
  # The first argument is an archive directory. The second is a segment name.
  # Print the complete segments of the timeline of that segment after it, oldest first. A .partial file does not count.
  sudo find "$1" -maxdepth 1 -type f -regextype posix-extended -regex "$COMPLETE_SEGMENT_REGEX" -printf '%f\n' | sort \
    | awk -v start="$2" 'length($0) == 24 && substr($0, 1, 8) == substr(start, 1, 8) && substr($0, 9, 16) > substr(start, 9, 16)'
}

switch_wal_segments() {
  # Each switch ends one segment. A switch with no new WAL does nothing, so a write comes before each switch.
  local switch
  for ((switch = 0; switch < GAP_SEGMENT_COUNT; switch++))
  do
    sql -c "SELECT pg_logical_emit_message(true, 'ci-refusals', 'wal-gap')" >/dev/null \
      || fail "refusals: cannot write WAL before a segment switch."
    sql -c 'SELECT pg_switch_wal()' >/dev/null || fail "refusals: cannot switch the WAL segment."
  done
}

archive_holds_gap_segments() {
  # True when /wal holds at least GAP_SEGMENT_COUNT complete segments after the segment in the argument.
  local segments
  segments=$(complete_segments_after "$RECIPE_WALARCHIVE_DIR" "$1") || return 1
  [[ $(printf '%s\n' "$segments" | grep -c . || true) -ge $GAP_SEGMENT_COUNT ]]
}

wait_gap_segments() {
  # One attempt takes one second, so the number of attempts is the limit in seconds (WAIT_ATTEMPTS).
  local start
  start=$(base_start_segment "$RECIPE_WALARCHIVE_DIR")
  log "refusals: wait for $GAP_SEGMENT_COUNT complete segments after $start, at most $WAIT_ATTEMPTS s"
  retry archive_holds_gap_segments "$start" \
    || fail "refusals: the archive holds fewer than two complete segments after the start segment."
}

make_wal_gap_copy() {
  # ADR-0106 (p) step 4. Copy /wal into the CI state. Then delete one complete segment between the start segment and the newest one.
  # The victim is the oldest complete segment after the start segment. The newest complete segment and every .partial file stay.
  local start segments victim
  copy_wal_archive
  start=$(base_start_segment "$WAL_COPY_DIR")
  segments=$(complete_segments_after "$WAL_COPY_DIR" "$start") || fail "refusals: cannot list the segments of the copy of /wal."
  victim=$(printf '%s\n' "$segments" | awk 'NR == 1 { first = $0 } END { if (NR >= 2) print first }') \
    || fail "refusals: cannot choose a segment of the copy of /wal."
  [[ $victim =~ ^[0-9A-F]{24}$ ]] \
    || fail "refusals: the copy of /wal has no complete segment between the start segment and the newest one."
  sudo rm -- "$WAL_COPY_DIR/$victim" || fail "refusals: cannot delete a segment of the WAL copy."
  log "refusals: the copy of /wal lacks the segment $victim"
}

refusals() {
  # ADR-0106 (p) step 4, the second and the third run. Both run on a new scratch volume. No run touches the production volume.
  local before after target
  [[ -s $CI_STATE/marker2.seq ]] || fail "refusals: run markers first."
  before=$(production_facts)
  trap refusals_exit EXIT
  target=$(sql -c "$REFUSAL_TARGET_SQL") || fail "refusals: cannot read the clock of db."
  [[ $target =~ $TARGET_TIME_PATTERN ]] || fail "refusals: the target time is invalid."
  log "refusals: run pitr-volume with the target time $target after the newest commit"
  create_scratch_volume refusals
  expect_pitr_error "the target time after the newest commit" "$TARGET_ERROR" \
    -v "$SCRATCH_VOLUME:/var/lib/postgresql/data" pitr-volume --target-time "$target"
  delete_drill_volume
  log "refusals: end $GAP_SEGMENT_COUNT segments, so /wal holds complete segments after the base backup"
  switch_wal_segments
  wait_gap_segments
  log "refusals: copy /wal and delete one complete segment between the base backup and the newest segment"
  make_wal_gap_copy
  log "refusals: run pitr-volume with the copy of /wal as the archive"
  create_scratch_volume refusals
  # This run sets no expected position. The check of the newest segment alone must refuse the copy (ADR-0106 (g) check 2).
  expect_pitr_error "the copy of /wal with a missing segment" "$GAP_ERROR" \
    -v "$SCRATCH_VOLUME:/var/lib/postgresql/data" -v "$WAL_COPY_DIR:/wal:ro" pitr-volume
  delete_drill_volume
  delete_wal_copy
  trap - EXIT
  after=$(production_facts)
  [[ $before == "$after" ]] || fail "refusals: a production fact changed during the step."
  log "refusals: done"
}

# A complete segment file. The segments of the old timeline in the target directory of the move have this form.
COMPLETE_SEGMENT_REGEX='.*/[0-9A-F]{24}'
# The step wal-line names the target directory of the move: the archive directory, then .moved- and the stamp of the time.
MOVED_STAMP_PATTERN='^[0-9]{8}_[0-9]{6}$'

old_timeline_segment() {
  # The argument is the target directory of the move. Print the newest complete segment there. It has the timeline of the old line.
  sudo find "$1" -maxdepth 1 -type f -regextype posix-extended -regex "$COMPLETE_SEGMENT_REGEX" -printf '%f\n' | sort | tail -n 1
}

read_moved_directory() {
  # The function sets MOVED_DIRECTORY from the state file of wal-line. The name must have the form that wal-line gives.
  local stamp
  [[ -s $CI_STATE/wal-target$WAL_SUFFIX.dir && -s $CI_STATE/active.volume && -f $CI_STATE/wal-target$WAL_SUFFIX.manifest ]] \
    || fail "refuse: run wal-line first."
  MOVED_DIRECTORY=$(<"$CI_STATE/wal-target$WAL_SUFFIX.dir")
  stamp=${MOVED_DIRECTORY#"${RECIPE_WALARCHIVE_DIR%/}.moved-"}
  [[ $stamp != "$MOVED_DIRECTORY" && $stamp =~ $MOVED_STAMP_PATTERN ]] \
    || fail "refuse: the target directory of the move is invalid."
}

refuse() {
  # ADR-0106 (m) and (p) step 9. A copy of /wal gets one segment of the old timeline from the target directory of the move.
  # walarchive-entry runs on the copy and must stop with the ERROR line of the guard. The real /wal and the target directory stay as they are.
  # If the guard accepted the copy, the container would run on and the limit of the CI step would end it. The step cleanup removes it.
  local before after segment
  ERROR_STEP=refuse
  ERROR_JOB=walarchive-entry
  read_moved_directory
  before=$(production_facts)
  trap refusals_exit EXIT
  log "refuse: copy /wal"
  copy_wal_archive
  segment=$(old_timeline_segment "$MOVED_DIRECTORY") || fail "refuse: cannot list the target directory of the move."
  [[ $segment =~ ^[0-9A-F]{24}$ ]] || fail "refuse: the target directory of the move holds no segment of the old timeline."
  if sudo test -e "$WAL_COPY_DIR/$segment"
  then
    fail "refuse: the copy of /wal holds the segment $segment already."
  fi
  sudo install -o 999 -g 999 -m 0600 -- "$MOVED_DIRECTORY/$segment" "$WAL_COPY_DIR/$segment" \
    || fail "refuse: cannot copy the segment $segment."
  log "refuse: the copy of /wal holds the segment $segment of the old timeline"
  log "refuse: run walarchive-entry with the copy of /wal"
  expect_pitr_error "the copy of /wal with a segment of the old timeline" "$TIMELINE_ERROR" -v "$WAL_COPY_DIR:/wal" walarchive
  after=$(sudo find "$MOVED_DIRECTORY" -mindepth 1 | sort) || fail "refuse: cannot list the target directory of the move."
  [[ $after == "$(<"$CI_STATE/wal-target$WAL_SUFFIX.manifest")" ]] \
    || fail "refuse: the target directory of the move changed during the step."
  delete_wal_copy
  trap - EXIT
  after=$(production_facts)
  [[ $before == "$after" ]] || fail "refuse: a production fact changed during the step."
  log "refuse: done"
}

# The pattern of the epoch seconds that crash records. The recovery time of the step check starts there.
EPOCH_PATTERN='^[0-9]{10,}$'

db_killed() {
  # True when the container in the first argument is not running and its exit code is 137, the code of SIGKILL.
  [[ $(docker inspect --format '{{.State.Running}} {{.State.ExitCode}}' "$1") == "false 137" ]]
}

crash() {
  # ADR-0106 (p) step 5 and (i) steps 1 to 4. db ends with SIGKILL and the old volume becomes unreadable on purpose.
  local old db_id started
  [[ -s $CI_STATE/marker2.seq && -s $CI_STATE/marker2.hash ]] || fail "crash: run markers first."
  old=$RECIPE_PGDATA_VOLUME
  [[ $(db_volume) == "$old" ]] || fail "crash: db does not use the volume $old."
  has_job_label "$old" || fail "crash: the old volume $old has no label of this job."
  db_id=$(container db)
  # ADR-0106 (p) measures the recovery time from step 5. The clock starts before the stop. The file comes last.
  started=$(date +%s)
  log "crash: the recovery time starts at $started. Stop app and backup"
  compose stop app backup || fail "crash: cannot stop app and backup."
  log "crash: read the WAL position and wait for its flush"
  wait_marker_flush crash
  printf '%s\n' "$FLUSHED_POSITION" >"$CI_STATE/expected.lsn"
  log "crash: the expected WAL position is $FLUSHED_POSITION. Kill db with SIGKILL"
  compose kill -s SIGKILL db || fail "crash: cannot kill db with SIGKILL."
  retry db_killed "$db_id" || fail "crash: db did not end with SIGKILL."
  compose stop walarchive || fail "crash: cannot stop walarchive."
  # The container mounts only the old volume and deletes only this one file. db is dead, so no process holds the cluster.
  log "crash: delete global/pg_control of the old volume $old"
  docker run --rm --network none --security-opt no-new-privileges:true -v "$old:/var/lib/postgresql/data" \
    --entrypoint rm "recipe-pgtools:${RECIPE_VERSION:?}" /var/lib/postgresql/data/global/pg_control \
    || fail "crash: cannot delete global/pg_control of the old volume."
  printf '%s\n' "$started" >"$CI_STATE/recovery.start"
}

recover() {
  # ADR-0106 (p) step 6 and (i) steps 5 and 6. pitr-volume runs to the end of the archive with the marker 2.
  # The step switch reads new.volume and pitr-volume.log. A failed run leaves the labelled volume to the step cleanup.
  local name seq hash lsn start volume
  for name in marker2.seq marker2.hash
  do
    [[ -s $CI_STATE/$name ]] || fail "recover: run markers first."
  done
  for name in expected.lsn recovery.start
  do
    [[ -s $CI_STATE/$name ]] || fail "recover: run crash first."
  done
  seq=$(<"$CI_STATE/marker2.seq")
  hash=$(<"$CI_STATE/marker2.hash")
  lsn=$(<"$CI_STATE/expected.lsn")
  start=$(<"$CI_STATE/recovery.start")
  [[ $seq =~ $MARKER_SEQ_PATTERN ]] || fail "recover: the sequence number of marker 2 is invalid."
  [[ $hash =~ $MARKER_HASH_PATTERN ]] || fail "recover: the hash of marker 2 is invalid."
  [[ $lsn =~ $LSN_PATTERN ]] || fail "recover: the expected WAL position is invalid."
  [[ $start =~ $EPOCH_PATTERN ]] || fail "recover: the start time of the recovery is invalid."
  # ADR-0103 (e) gives the name. The label lets delete_ci_volumes find the volume.
  volume=recipe-editor_pgdata_$(date -u +%Y%m%d_%H%M%S)
  create_labelled_volume recover "$volume"
  log "recover: run pitr-volume to the end of the archive into the volume $volume"
  export PITR_MARKER_SEQ=$seq PITR_MARKER_HASH=$hash
  # Sudo drops PITR_EXPECTED_LSN, so the option carries the value. The position is no secret.
  compose run --rm --no-deps -T -e PITR_MARKER_SEQ -e PITR_MARKER_HASH -e "PITR_EXPECTED_LSN=$lsn" \
    -v "$volume:/var/lib/postgresql/data" pitr-volume | tee "$CI_STATE/pitr-volume.log" || fail "recover: pitr-volume failed."
  log "recover: check the completed line"
  check_pitr_line recover false "$seq" 2
  printf '%s\n' "$volume" >"$CI_STATE/new.volume"
  log "recover: done. The new volume is $volume"
}

prove_role_logins() {
  # ADR-0106 (p) step 6 and (i) step 9. The recovered cluster keeps the role passwords of the recovery point.
  # The step sets the passwords from the secret files. Then each login role must log in with its password.
  # Psql reads the secret files inside db, so no password reaches an argument, a log line, or an error line.
  log "switch: set the passwords from the secret files"
  set_role_passwords || fail "switch: cannot set the role passwords."
  log "switch: password login of each login role works"
  PROBE_LABEL=switch expect_probe ok "${LOGIN_ROLES[@]}"
}

switch_recovered() {
  # ADR-0106 (p) step 6. The shared switch runs with the changes of (j). The passwords follow it and come before the new WAL line.
  switch "pitr volume completed"
  prove_role_logins
}

wal_line() {
  # ADR-0106 (k) and ADR-0103 (s). The new WAL line starts after the switch and before app. Nothing is deleted.
  # A recovery keeps the system identifier, so the completed line of pitr-volume names the identifier of the new line.
  # The step check reads pitr-volume.log again, so no step deletes it.
  local name
  for name in change.log old.volume active.volume pitr-volume.log
  do
    [[ -s $CI_STATE/$name ]] || fail "wal-line: run switch first."
  done
  read_pitr_line wal-line
  log "wal-line: step 1, the switch deleted the walarchive container"
  start_wal_line "$LINE_IDENTIFIER"
  log "wal-line: done"
}

# The recovery time objective is 1 hour (reliability.md 7.7, ADR-0020). Section 7.11 has no row for it (design.md D8).
RECOVERY_TIME_LIMIT_SECONDS=3600
# The epoch seconds at the start of the running step of check. lap_start and lap_end set it.
LAP_START=0

lap_start() {
  LAP_START=$(date +%s)
}

lap_end() {
  # The argument is the number of the step. The next step starts at the end of this one.
  local now
  now=$(date +%s)
  log "check: step $1 took $((now - LAP_START)) s"
  LAP_START=$now
}

check_recovery_time() {
  # ADR-0106 (p). The recovery time runs from the start of crash to the start of app. The argument is the start in epoch seconds.
  local elapsed
  elapsed=$(($(date +%s) - $1))
  log "check: the recovery time is $elapsed s. The limit is $RECOVERY_TIME_LIMIT_SECONDS s."
  (( elapsed <= RECOVERY_TIME_LIMIT_SECONDS )) \
    || fail "check: the recovery time of $elapsed s is above the limit of $RECOVERY_TIME_LIMIT_SECONDS s."
}

refuse_marker_1_password() {
  # admin_password holds the password of marker 1 and new_password holds the password of marker 2 (change_own_password).
  # http_probe relogin reads new_password, so the file holds the old password for one try. Then it gets its value back.
  local current old status=0
  current=$(<"$CI_STATE/new_password")
  old=$(<"$CI_STATE/admin_password")
  [[ -n $old && $old != "$current" ]] || fail "check: the password of marker 1 is missing or equals the password of marker 2."
  rm -f -- "$CI_STATE/cookies.json"
  printf '%s\n' "$old" >"$CI_STATE/new_password"
  http_probe relogin >/dev/null 2>&1 || status=$?
  printf '%s\n' "$current" >"$CI_STATE/new_password"
  [[ $status -ne 0 ]] || fail "check: the login with the password of marker 1 passed."
}

prove_now_set() {
  # ADR-0106 (p) step 8. backup-now and restore-test prove the first set of the new cluster.
  wait_backup_minute
  compose exec -T backup backup-now || fail "check: backup-now failed."
  compose exec -T backup restore-test | tee "$CI_STATE/restore-check.log" || fail "check: restore-test failed."
  grep -Eq '"message":"restore test passed".*"set":"recipe_[0-9]{8}_[0-9]{4}_now"' "$CI_STATE/restore-check.log" \
    || fail "check: restore-test did not prove the new now set."
}

check() {
  # ADR-0106 (i) steps 11 and 12 and (p) steps 6 to 8. The step starts app, so the recovery time ends here.
  # The log holds the time of each step.
  local name start
  for name in wal-new.base active.volume
  do
    [[ -s $CI_STATE/$name ]] || fail "check: run wal-line first."
  done
  [[ -s $CI_STATE/recovery.start ]] || fail "check: run crash first."
  for name in admin_password new_password cookies.json
  do
    [[ -s $CI_STATE/$name ]] || fail "check: run markers first."
  done
  start=$(<"$CI_STATE/recovery.start")
  [[ $start =~ $EPOCH_PATTERN ]] || fail "check: the start time of the recovery is invalid."
  lap_start
  log "check: step 1, truncate session and login_attempt, then start app, proxy, and backup"
  sql -c "TRUNCATE public.session, public.login_attempt" >/dev/null || fail "check: cannot truncate session and login_attempt."
  compose up -d --no-deps --wait --wait-timeout 180 app || fail "check: app did not start."
  check_recovery_time "$start"
  compose up -d --no-deps --wait --wait-timeout 180 proxy backup || fail "check: proxy and backup did not start."
  lap_end 1
  log "check: step 2, /readyz, the login with the password of marker 1 fails, the login with the password of marker 2 passes"
  check_readyz check
  refuse_marker_1_password
  rm -f -- "$CI_STATE/cookies.json"
  http_probe relogin || fail "check: the login with the password of marker 2 failed."
  lap_end 2
  log "check: step 3, backup-now and restore-test"
  prove_now_set
  lap_end 3
  log "check: step 4, basebackup-once, a new marker, the flush check, and pitr-check on the new WAL line"
  take_base_backup check-base.log check
  commit_marker 3 check
  wait_marker_flush check
  PITR_MARKER_SEQ=$(<"$CI_STATE/marker3.seq")
  PITR_MARKER_HASH=$(<"$CI_STATE/marker3.hash")
  export PITR_MARKER_SEQ PITR_MARKER_HASH
  check_pitr
  lap_end 4
  log "check: done"
}

load_active_volume
case ${1:-} in
  prepare) prepare ;;
  markers) markers ;;
  drill) drill ;;
  refusals) refusals ;;
  crash) crash ;;
  recover) recover ;;
  switch) switch_recovered ;;
  wal-line) wal_line ;;
  check) check ;;
  refuse) refuse ;;
  cleanup) cleanup ;;
  *) fail "usage: pitr-recovery.sh prepare|markers|drill|refusals|crash|recover|switch|wal-line|check|refuse|cleanup" ;;
esac
