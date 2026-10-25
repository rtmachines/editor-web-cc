# Sourced by walarchive-test. The static rules, the gates, and the run of pitr-check
# (ADR-0068 (l) to (o) as amended by ADR-0077 (a) to (e)). The run uses stubs of pg_ctl and psql.
# The real recovery needs the real stack and runs in the CI job "compose" (design D11 step 7).

PITR_SEQ=4711
PITR_HASH=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef

static_pitr_rules() {
  local f=$BIN/pitr-check o
  for o in 'cp $WAL_DIR/%f %p || cp $WAL_DIR/%f.partial %p' 'hot_standby=off' 'recovery.signal' \
    'shared_buffers=64MB' 'maintenance_work_mem=64MB' 'max_connections=10' \
    'max_parallel_workers_per_gather=0' 'max_parallel_maintenance_workers=0' 'listen_addresses=' \
    'WAL_DIR=${WAL_DIR:-/wal}' 'trap cleanup EXIT' 'pg_verifybackup --quiet' 'cp -a' 'flock -n' 'pg_ctl' 'PITR_CI' '.pitr-scratch' \
    'PITR_MARKER_SEQ' 'PITR_MARKER_HASH' "decode('" 'audit_canonical' 'pg_is_in_recovery'; do
    expect_ok "static pitr-check: $o" grep -qF -- "$o" "$f"
  done
  expect_no "static pitr-check: no restore_command with a slot or a target" \
    grep -qE 'recovery_target|primary_slot_name' "$f"
  expect_no "static pitr-check: no fsync setting (ADR-0068 (o))" grep -qF 'fsync=' "$f"
  expect_no "static pitr-check: no connection to db, no secret, no write to the database" \
    grep -qE 'pg_basebackup|PGPASSFILE=|pgpass|INSERT INTO|app_password|replicator_password|--host="\$PGHOST"' "$f"
}

# pitr_run [VAR=value ...]: run pitr-check with PITR_CI=1 and a good marker. A later VAR wins.
pitr_run() {
  wal_run PITR_CI=1 "PITR_MARKER_SEQ=$PITR_SEQ" "PITR_MARKER_HASH=$PITR_HASH" "$@" -- pitr-check
}

# pitr_gate_case <name> <message> [VAR=value ...]: the run refuses at its own gate. The case holds a
# good base backup, so a missing gate lets the run go on and the message or the snapshot differs.
pitr_gate_case() {
  local name=$1 message=$2 before
  shift 2
  wal_new_case "pitr-$name"
  make_base 10 7
  before=$(wal_snapshot)
  wal_run "$@" -- pitr-check
  wal_expect_refused "pitr-check gate $name" "$before"
  expect_message "pitr-check gate $name" "$message"
}

# pitr_env_cases: the gates of the environment. A value is missing, empty, or bad.
pitr_env_cases() {
  local ci='set PITR_CI=1' seq='PITR_MARKER_SEQ is missing or bad' hash='PITR_MARKER_HASH is missing or bad'
  local good_seq="PITR_MARKER_SEQ=$PITR_SEQ" good_hash="PITR_MARKER_HASH=$PITR_HASH"
  pitr_gate_case no-ci "$ci" "$good_seq" "$good_hash"
  pitr_gate_case ci-zero "$ci" PITR_CI=0 "$good_seq" "$good_hash"
  pitr_gate_case no-seq "$seq" PITR_CI=1 "$good_hash"
  local v i=0
  # empty, zero, a leading zero, a non-digit, a sign, 20 digits, and a space
  for v in '' 0 0123 12a -5 12345678901234567890 '1 2'; do
    i=$((i + 1))
    pitr_gate_case "bad-seq-$i" "$seq" PITR_CI=1 "PITR_MARKER_SEQ=$v" "$good_hash"
  done
  pitr_gate_case no-hash "$hash" PITR_CI=1 "$good_seq"
  i=0
  # empty, 63 and 65 characters, an upper-case letter, a non-hex letter, and a space
  for v in '' "${PITR_HASH%?}" "${PITR_HASH}a" "A${PITR_HASH#?}" "g${PITR_HASH#?}" \
    "${PITR_HASH:0:32} ${PITR_HASH:33}"; do
    i=$((i + 1))
    pitr_gate_case "bad-hash-$i" "$hash" PITR_CI=1 "$good_seq" "PITR_MARKER_HASH=$v"
  done
}

# pitr_archive_cases: the gates of the archive disk: the path, the marker, and the base backups.
pitr_archive_cases() {
  local dir='WAL_DIR is not a plain absolute path'
  pitr_gate_case wal-dir-space "$dir" PITR_CI=1 "PITR_MARKER_SEQ=$PITR_SEQ" "PITR_MARKER_HASH=$PITR_HASH" 'WAL_DIR=/tmp/a b'
  pitr_gate_case wal-dir-dotdot "$dir" PITR_CI=1 "PITR_MARKER_SEQ=$PITR_SEQ" "PITR_MARKER_HASH=$PITR_HASH" 'WAL_DIR=/tmp/../etc'
  pitr_gate_case wal-dir-relative "$dir" PITR_CI=1 "PITR_MARKER_SEQ=$PITR_SEQ" "PITR_MARKER_HASH=$PITR_HASH" 'WAL_DIR=wal'
  pitr_gate_case wal-dir-quote "$dir" PITR_CI=1 "PITR_MARKER_SEQ=$PITR_SEQ" "PITR_MARKER_HASH=$PITR_HASH" "WAL_DIR=/tmp/a'b"
  local before
  wal_new_case pitr-no-marker
  make_base 10 7
  rm -f -- "$W/.walarchive-marker"
  before=$(wal_snapshot)
  pitr_run
  wal_expect_refused "pitr-check gate no-marker" "$before"
  expect_message "pitr-check gate no-marker" "the marker file of the WAL archive disk is missing"
  wal_new_case pitr-no-base
  before=$(wal_snapshot)
  pitr_run
  wal_expect_refused "pitr-check gate no-base" "$before"
  expect_message "pitr-check gate no-base" "no base backup in the WAL archive"
  wal_new_case pitr-foreign-base
  mkdir "$W/base/base_20200101_0000" "$W/base/.tmp-base_20200101_000000" "$W/base/xbase_20200101_000000"
  : >"$W/base/base_20200102_000000"
  before=$(wal_snapshot)
  pitr_run
  wal_expect_refused "pitr-check gate foreign names only" "$before"
  expect_message "pitr-check gate foreign names only" "no base backup in the WAL archive"
}

# The lock of the WAL archive is held by another process. The run copies nothing.
case_wal_pitr_busy_lock() {
  local holder before
  if ! flock_is_real; then
    skip "flock on this host grants a second lock (fake flock)"
    return 0
  fi
  wal_new_case pitr-busy-lock
  make_base 10 7
  ( exec 8>>"$W/.lock"; flock -n 8 || exit 1; : >"$CASE/held"; exec sleep 30 ) &
  holder=$!
  if ! wait_for_file "$CASE/held" 100; then
    kill "$holder" 2>/dev/null
    bad "pitr-check busy lock: the holder did not start"
    return 0
  fi
  before=$(wal_snapshot)
  pitr_run
  kill "$holder" 2>/dev/null
  wait "$holder" 2>/dev/null || true
  wal_expect_refused "pitr-check gate busy lock" "$before"
  expect_message "pitr-check gate busy lock" "another run holds the lock of the WAL archive"
}

# pitr_run_case <name> [VAR=value ...]: a full run with stubs. Two base backups and foreign names.
# The base backup of 10 days starts at segment 7. The base backup of 20 days starts at segment 9.
pitr_run_case() {
  local name=$1
  shift
  wal_new_case "pitr-run-$name"
  make_base 20 9
  make_base 10 7
  make_segments 1 10
  mkdir "$W/base/base_29990101_000000.bak"
  : >"$W/base/base_29990102_000000"
  pitr_make_link
  PITR_BASE_BEFORE=$(base_tree)
  PITR_TOP_BEFORE=$(wal_listing)
  pitr_run STUB_PSQL_OK=1 PGHOST=evil PGPASSWORD=zzz PGPASSFILE=/nowhere "$@"
}

# pitr_link_state: name, type, size, mode, and mtime of every entry below the target of the link.
pitr_link_state() {
  (cd "$CASE/link-target" && find . -printf '%p %y %s %m %T@\n' | LC_ALL=C sort)
}

# pitr_make_link: a link with a name of the grammar. It lies after every real name, so the run would
# select it. Its target is a copy of a base backup with a label for segment 99 and mode 0755.
# ADR-0077 (d): a link is not a base backup. The run must not copy it, change its mode, or write to it.
# A host without symlinks leaves PITR_LINK_BEFORE empty, and the case reports a skip.
pitr_make_link() {
  PITR_LINK_BEFORE=""
  cp -a -- "$W/base/$LAST_BASE" "$CASE/link-target"
  printf 'START WAL LOCATION: 0/63000028 (file %s)\n' "$(seg_name 99)" >"$CASE/link-target/backup_label"
  chmod 0755 "$CASE/link-target"
  ln -s "$CASE/link-target" "$W/base/base_29990103_000000" 2>/dev/null || true
  if [ ! -L "$W/base/base_29990103_000000" ]; then
    # Some hosts copy the target at ln -s. The copy would be a real base backup, so remove it.
    rm -r -f -- "${W:?}/base/base_29990103_000000"
    return 0
  fi
  PITR_LINK_BEFORE=$(pitr_link_state)
}

# The run did not select the link: the label of the copy is segment 7, and the target did not change.
pitr_expect_link_ignored() {
  if [ -z "$PITR_LINK_BEFORE" ]; then
    skip "pitr run: a link with a name of the grammar (this host makes no symlink)"
    return 0
  fi
  expect_ok "pitr run: the link is not selected (label of segment 99 absent)" \
    test "$(grep -cF "$(seg_name 99)" "$CASE/stub/label.seen" || true)" = 0
  expect_eq "pitr run: the target of the link is unchanged" "$(pitr_link_state)" "$PITR_LINK_BEFORE"
  expect_ok "pitr run: the link stays" test -L "$W/base/base_29990103_000000"
}

# The scratch directory of a run is gone. The directory .pitr-scratch stays. Nothing else changed.
pitr_expect_clean() {
  expect_ok "$1: the directory .pitr-scratch stays" test -d "$W/.pitr-scratch"
  expect_eq "$1: the scratch directory is empty" "$(find "$W/.pitr-scratch" -mindepth 1 | wc -l | tr -d ' ')" "0"
  expect_eq "$1: the base backups are unchanged" "$(base_tree)" "$PITR_BASE_BEFORE"
  expect_eq "$1: the archive is unchanged" "$(wal_listing | grep -v '\.pitr-scratch')" "$PITR_TOP_BEFORE"
  expect_ok "$1: JSON lines" json_lines_ok "$CASE/out"
}

case_wal_pitr_passes() {
  pitr_run_case pass
  local log=$CASE/stub/log
  expect_eq "pitr run: exit code" "$RC" "0"
  expect_eq "pitr run: no ERROR line" "$(error_count "$CASE/out")" "0"
  expect_message "pitr run" "pitr check passed"
  expect_ok "pitr run: the result line holds the chain_seq" grep -qF "\"chain_seq\":$PITR_SEQ" "$CASE/out"
  expect_no "pitr run: the result line holds no request_id" grep -q request_id "$CASE/out"
  expect_eq "pitr run: order of the tools" "$(event_sequence)" \
    "EVENT pg_verifybackup data,EVENT pg_ctl start,EVENT psql,EVENT psql,EVENT psql,EVENT pg_ctl stop,"
  expect_ok "pitr run: pg_verifybackup is quiet" grep -qF 'ARGV pg_verifybackup --quiet ' "$log"
  expect_ok "pitr run: recovery.signal" grep -qF 'recovery.signal present' "$log"
  expect_ok "pitr run: the copy of the newest base backup (10 days, segment 7)" \
    grep -qF "(file $(seg_name 7))" "$CASE/stub/label.seen"
  expect_eq "pitr run: mode of the copy" "$(cat "$CASE/stub/data.mode")" "700"
  pitr_expect_link_ignored
  pitr_expect_settings
  pitr_expect_checks
  pitr_expect_clean "pitr run"
}

# The settings of the scratch cluster: the restore command of ADR-0068 (l) and no fsync.
pitr_expect_settings() {
  local conf=$CASE/stub/auto.conf
  expect_ok "pitr run: restore_command with the archive path" \
    grep -qxF "restore_command='cp $W/%f %p || cp $W/%f.partial %p'" "$conf"
  expect_ok "pitr run: hot_standby=off" grep -qxF 'hot_standby=off' "$conf"
  expect_ok "pitr run: scratch port" grep -qxF 'port=5433' "$conf"
  expect_ok "pitr run: no network" grep -qxF "listen_addresses=''" "$conf"
  expect_no "pitr run: no fsync setting" grep -q 'fsync' "$conf"
}

# The checks of the recovered cluster: the private socket, the marker identity, no libpq variable.
pitr_expect_checks() {
  local log=$CASE/stub/log
  expect_eq "pitr run: three psql calls on the private socket" \
    "$(grep -c "^ARGV psql .*--host=$W/.pitr-scratch/run\.[A-Za-z0-9]*/sock .*--port=5433 " "$log" || true)" "3"
  expect_ok "pitr run: the check reads chain_seq" grep -qF "chain_seq = $PITR_SEQ" "$log"
  expect_ok "pitr run: the check compares entry_hash with decode" \
    grep -qF "entry_hash = pg_catalog.decode('$PITR_HASH', 'hex')" "$log"
  expect_eq "pitr run: no libpq variable reaches a tool" \
    "$(grep '^ENV PGHOST=' "$log" | sort -u)" "ENV PGHOST= PGPORT= PGUSER= PGPASSWORD= PGPASSFILE="
}

# Each later fault stops with one ERROR line of its own and leaves nothing behind.
case_wal_pitr_faults() {
  local entry='the marker entry is missing in the recovered cluster'
  pitr_fault_case verify STUB_VERIFY_FAIL=1 'pg_verifybackup failed'
  pitr_fault_case start STUB_CTL_FAIL=1 'the scratch cluster did not finish the recovery'
  pitr_fault_case recovery STUB_PSQL_RECOVERY=f 'the scratch cluster is still in recovery'
  pitr_fault_case psql-fails STUB_PSQL_OK= 'cannot read the recovery state of the scratch cluster'
  pitr_fault_case missing-entry STUB_PSQL_COUNT=0 "$entry"
  pitr_fault_case two-entries STUB_PSQL_COUNT=2 "$entry"
  pitr_fault_case chain 'STUB_PSQL_CHAIN=1|t|t' 'the audit chain check failed on the recovered cluster'
}

# pitr_fault_case <name> <VAR=value> <message>
pitr_fault_case() {
  local name=$1 env=$2 message=$3
  pitr_run_case "fault-$name" "$env"
  expect_eq "pitr fault $name: exit code" "$RC" "1"
  expect_eq "pitr fault $name: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_message "pitr fault $name" "$message"
  pitr_expect_clean "pitr fault $name"
}

case_wal_pitr_gates() {
  pitr_env_cases
  pitr_archive_cases
  case_wal_pitr_busy_lock
}

case_wal_pitr_runs() {
  case_wal_pitr_passes
  case_wal_pitr_faults
}
