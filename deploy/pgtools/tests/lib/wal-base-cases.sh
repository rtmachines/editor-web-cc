# Sourced by walarchive-test. The cases of basebackup-once (ADR-0068 (f) to (k)).
# The stubs create the base backup and run no database.

BASE_RE='^base_[0-9]{8}_[0-9]{6}$'

static_base_rules() {
  local f=$BIN/basebackup-once o
  for o in '--wal-method=fetch' '-Fp' '--checkpoint=fast' '--manifest-checksums=SHA256' \
    'pg_verifybackup --quiet "$tmp_dir" >&2' 'pg_archivecleanup' 'LC_ALL=C' 'base_[0-9]{8}_[0-9]{6}' '.walarchive-marker'; do
    expect_ok "static basebackup-once: $o" grep -qF -- "$o" "$f"
  done
}

# new_base_case <name> <count>: <count> old base backups. The i-th is (10 * i) days old and starts at
# segment (2 * i). The archive holds the segments 1 to 30 and the partial segment 31.
new_base_case() {
  local i
  wal_new_case "$1"
  for ((i = 1; i <= $2; i++)); do
    make_base $((i * 10)) $((i * 2))
  done
  make_segments 1 30
}

# Shared checks of a run that published nothing and deleted nothing.
base_expect_unchanged() {
  expect_eq "$1: exit code" "$RC" "1"
  expect_eq "$1: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_ok "$1: JSON lines" json_lines_ok "$CASE/out"
  expect_eq "$1: the archive is unchanged" "$(wal_listing)" "$2"
  expect_eq "$1: the base backups are unchanged" "$(base_tree)" "$3"
  expect_eq "$1: no archivecleanup" "$(count_events 'pg_archivecleanup')" "0"
  expect_eq "$1: pgpass directory removed" "$(find "$CASE/pgpass" -mindepth 1 | wc -l | tr -d ' ')" "0"
  expect_ok "$1: no password in output" no_secret
}

case_wal_base_happy() {
  wal_new_case base-happy
  wal_run STUB_START_SEG=14 -- basebackup-once
  local dir=$W/base/base_$WTS log=$CASE/stub/log
  expect_eq "base: exit code" "$RC" "0"
  expect_eq "base: no ERROR line" "$(error_count "$CASE/out")" "0"
  expect_ok "base: JSON lines" json_lines_ok "$CASE/out"
  expect_ok "base: published directory" test -f "$dir/backup_manifest"
  expect_eq "base: directory mode" "$(stat -c %a "$dir" 2>/dev/null)" "700"
  expect_eq "base: only the new base backup" "$(base_dirs)" "base_$WTS "
  expect_eq "base: no temp name" "$(find "$W/base" -name '.tmp-*' | wc -l | tr -d ' ')" "0"
  expect_eq "base: order of the tools" "$(event_sequence)" \
    "EVENT pg_basebackup start,EVENT pg_basebackup done,EVENT pg_verifybackup .tmp-base_$WTS,EVENT pg_archivecleanup $(seg_name 14),"
  expect_ok "base: pg_basebackup options" grep -qF -- \
    ' --wal-method=fetch -Fp --checkpoint=fast --manifest-checksums=SHA256' "$log"
  expect_ok "base: no password prompt" grep -qF -- ' --no-password' "$log"
  expect_ok "base: archive directory for the cleanup" grep -qF -- "ARGV pg_archivecleanup $W " "$log"
  expect_ok "base: host and user" grep -qF -- 'ENV PGHOST=db PGPORT=5432 PGUSER=recipe_replicator' "$log"
  expect_eq "base: pgpass line" "$(cat "$CASE/stub/pgpass.seen" 2>/dev/null)" "$EXPECTED_PGPASS"
  expect_eq "base: pgpass mode" "$(cat "$CASE/stub/pgpass.mode" 2>/dev/null)" "600"
  expect_eq "base: pgpass directory mode" "$(cat "$CASE/stub/pgpass.dirmode" 2>/dev/null)" "700"
  expect_ok "base: marker stays" test -f "$W/.walarchive-marker"
  expect_ok "base: lock file stays" test -f "$W/.lock"
  expect_eq "base: top directory mode" "$(stat -c %a "$W")" "700"
  expect_ok "base: no password in output" no_secret
}

# Six old base backups and a new one. The newest 4 stay. The cleanup gets the lowest start segment of the kept set.
case_wal_base_retention() {
  new_base_case base-retention 6
  local kept=() deleted=() dir n
  wal_run STUB_START_SEG=14 -- basebackup-once
  expect_eq "retention: exit code" "$RC" "0"
  expect_eq "retention: no ERROR line" "$(error_count "$CASE/out")" "0"
  # The base backups of 10, 20, and 30 days and the new one stay.
  for dir in "$W"/base/base_*; do kept+=("${dir##*/}"); done
  expect_eq "retention: four stay" "${#kept[@]}" "4"
  expect_ok "retention: new one stays" base_exists "base_$WTS"
  for n in 40 50 60; do
    deleted+=("$(base_name_of_age "$n")")
  done
  for dir in "${deleted[@]}"; do
    expect_no "retention: $dir deleted" test -e "$W/base/$dir"
  done
  # The kept base backups start at the segments 2 (10 days), 4 (20 days), 6 (30 days), and 14 (new).
  # The oldest by name starts at 6. The lowest start segment of the kept set is 2 (ADR-0077 (f)).
  expect_eq "retention: cleanup gets the lowest start segment of the kept set (segment 2)" \
    "$(grep '^EVENT pg_archivecleanup' "$CASE/stub/log")" "EVENT pg_archivecleanup 000000010000000000000002"
  expect_ok "retention: segment 1 deleted" test ! -e "$W/$(seg_name 1)"
  expect_ok "retention: segment 2 stays" seg_exists 2
  expect_ok "retention: segment 6 stays" seg_exists 6
  expect_ok "retention: segment 30 stays" seg_exists 30
  expect_ok "retention: partial segment stays" test -f "$W/$(seg_name 31).partial"
  expect_ok "retention: marker stays" test -f "$W/.walarchive-marker"
  expect_ok "retention: lock file stays" test -f "$W/.lock"
  expect_eq "retention: one line for each deleted base backup" \
    "$(grep -c 'old base backup deleted' "$CASE/out" || true)" "3"
}

case_wal_base_few() {
  new_base_case base-few 2
  wal_run STUB_START_SEG=14 -- basebackup-once
  expect_eq "few: exit code" "$RC" "0"
  expect_eq "few: nothing deleted" "$(base_dirs | wc -w | tr -d ' ')" "3"
  expect_eq "few: cleanup gets the lowest start segment (10 days, segment 2)" \
    "$(grep '^EVENT pg_archivecleanup' "$CASE/stub/log")" "EVENT pg_archivecleanup 000000010000000000000002"
  new_base_case base-exactly-four 3
  wal_run -- basebackup-once
  expect_eq "exactly four: nothing deleted" "$(base_dirs | wc -w | tr -d ' ')" "4"
}

# ADR-0077 (f): an old base backup can start higher than the new one, for example after a restore into
# a new cluster. Two kept old base backups start at 30 and 20. The new one starts at 14. The cutoff is
# 14, and no segment from 14 up is deleted. A cutoff from the oldest name would delete segments 14 to 29.
case_wal_base_high_start() {
  wal_new_case base-high-start
  make_base 20 30
  make_base 10 20
  make_segments 1 30
  wal_run STUB_START_SEG=14 -- basebackup-once
  expect_eq "high start: exit code" "$RC" "0"
  expect_eq "high start: cleanup gets the start of the new base backup" \
    "$(grep '^EVENT pg_archivecleanup' "$CASE/stub/log")" "EVENT pg_archivecleanup $(seg_name 14)"
  expect_ok "high start: segment 13 deleted" test ! -e "$W/$(seg_name 13)"
  expect_ok "high start: segment 14 stays" seg_exists 14
  expect_ok "high start: segment 20 stays" seg_exists 20
  expect_ok "high start: segment 29 stays" seg_exists 29
  expect_ok "high start: segment 30 stays" seg_exists 30
  expect_eq "high start: three base backups stay" "$(base_dirs | wc -w | tr -d ' ')" "3"
}

# The timeline is not part of the comparison. A kept base backup of an older timeline with the lowest
# segment sets the cutoff, and the cutoff keeps its timeline text.
case_wal_base_timeline_ignored() {
  wal_new_case base-timeline
  make_base 10 9
  printf 'START WAL LOCATION: 0/3000028 (file 000000020000000000000003)\n' >"$W/base/$LAST_BASE/backup_label"
  make_segments 1 30
  wal_run STUB_START_SEG=14 -- basebackup-once
  expect_eq "timeline: exit code" "$RC" "0"
  expect_eq "timeline: the cutoff is the lowest segment after the timeline" \
    "$(grep '^EVENT pg_archivecleanup' "$CASE/stub/log")" "EVENT pg_archivecleanup 000000020000000000000003"
}

# Names that are not in the grammar are never deleted, also when they are old.
case_wal_base_foreign_names() {
  local foreign=() name
  new_base_case base-foreign 5
  mkdir "$W/base/base_2026" "$W/base/base_20200101_0000" "$W/base/base_20200101_000000x" \
    "$W/base/xbase_20200101_000000" "$W/base/BASE_20200101_000000" "$W/base/.tmp-base_20200101_000000" \
    "$W/base/base_２0200101_000000" "$W/base/base_20200101_000000.bak"
  : >"$W/base/base_20200102_000000"
  : >"$W/base/notes.txt"
  mkdir "$W/.pitr-scratch"
  foreign=(base_2026 base_20200101_0000 base_20200101_000000x xbase_20200101_000000 BASE_20200101_000000
    .tmp-base_20200101_000000 base_２0200101_000000 base_20200101_000000.bak base_20200102_000000 notes.txt)
  mkdir "$CASE/target"
  ln -s "$CASE/target" "$W/base/base_20200103_000000"
  [ -L "$W/base/base_20200103_000000" ] && foreign+=(base_20200103_000000)
  wal_run STUB_START_SEG=14 -- basebackup-once
  expect_eq "foreign: exit code" "$RC" "0"
  for name in "${foreign[@]}"; do
    expect_ok "foreign: $name stays" test -e "$W/base/$name" -o -L "$W/base/$name"
  done
  expect_ok "foreign: the target of a link stays" test -d "$CASE/target"
  expect_ok "foreign: scratch directory stays" test -d "$W/.pitr-scratch"
  expect_eq "foreign: the retention kept four names of the grammar" "$(grammar_count)" "4"
}

# grammar_count: the number of real directories below base that match the grammar of a base backup.
grammar_count() {
  (
    export LC_ALL=C
    cd "$W/base" || exit 1
    for d in base_*; do
      if [ -d "$d" ] && [ ! -L "$d" ] && [[ $d =~ $BASE_RE ]]; then echo "$d"; fi
    done | wc -l | tr -d ' '
  )
}

case_wal_base_verify_fails() {
  local before_top before_base
  new_base_case base-verify-fails 6
  before_top=$(wal_listing)
  before_base=$(base_tree)
  wal_run STUB_VERIFY_FAIL=1 -- basebackup-once
  base_expect_unchanged "failed pg_verifybackup" "$before_top" "$before_base"
  expect_eq "failed pg_verifybackup: pg_verifybackup ran" "$(count_events 'pg_verifybackup')" "1"
  expect_eq "failed pg_verifybackup: no temp name" "$(find "$W/base" -name '.tmp-*' | wc -l | tr -d ' ')" "0"
  expect_no "failed pg_verifybackup: new base backup not published" base_exists "base_$WTS"
}

case_wal_base_basebackup_fails() {
  local before_top before_base
  new_base_case base-basebackup-fails 6
  before_top=$(wal_listing)
  before_base=$(base_tree)
  wal_run STUB_BASEBACKUP_FAIL=1 -- basebackup-once
  base_expect_unchanged "failed pg_basebackup" "$before_top" "$before_base"
  expect_eq "failed pg_basebackup: no pg_verifybackup" "$(count_events 'pg_verifybackup')" "0"
  expect_eq "failed pg_basebackup: no temp name" "$(find "$W/base" -name '.tmp-*' | wc -l | tr -d ' ')" "0"
}

# A cleanup that fails keeps every segment. The new base backup passed its check and stays.
case_wal_base_cleanup_fails() {
  local before_names
  new_base_case base-cleanup-fails 6
  before_names=$(base_dirs)
  wal_run STUB_CLEANUP_FAIL=1 -- basebackup-once
  expect_eq "failed cleanup: exit code" "$RC" "1"
  expect_eq "failed cleanup: one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_ok "failed cleanup: new base backup stays" base_exists "base_$WTS"
  expect_ok "failed cleanup: segment 1 stays" seg_exists 1
  expect_ok "failed cleanup: JSON lines" json_lines_ok "$CASE/out"
  # ADR-0068 (k): the retention deleted the old base backups before the cleanup ran. The newest four stay.
  expect_eq "failed cleanup: the newest four base backups stay" "$(base_dirs)" \
    "$(printf '%s ' $(printf '%s\n' $before_names | tail -n 3) "base_$WTS")"
}

# The label of each kept base backup is read before any delete. A bad label of any kept base backup
# (the oldest or the newest kept one) stops all deletes (ADR-0077 (f)).
case_wal_base_bad_label() {
  local age
  for age in 30 10; do
    bad_label_case "$age"
  done
}

bad_label_case() {
  local before_top before_base kept
  new_base_case "base-bad-label-$1" 6
  kept=$(base_name_of_age "$1")
  [ -d "$W/base/$kept" ] || bad "bad label ($1 days): the base backup $kept does not exist"
  printf 'garbage\n' >"$W/base/$kept/backup_label"
  touch -d "$1 days ago" "$W/base/$kept/backup_label" "$W/base/$kept"
  before_top=$(wal_listing)
  before_base=$(base_tree)
  wal_run -- basebackup-once
  expect_eq "bad label ($1 days): exit code" "$RC" "1"
  expect_eq "bad label ($1 days): one ERROR line" "$(error_count "$CASE/out")" "1"
  expect_eq "bad label ($1 days): the archive is unchanged" "$(wal_listing)" "$before_top"
  expect_eq "bad label ($1 days): no old base backup deleted" "$(base_dirs | wc -w | tr -d ' ')" "7"
  expect_eq "bad label ($1 days): no archivecleanup" "$(count_events 'pg_archivecleanup')" "0"
  expect_ok "bad label ($1 days): new base backup stays" base_exists "base_$WTS"
  expect_eq "bad label ($1 days): old trees are unchanged" \
    "$(base_tree | grep -v "base_$WTS")" "$before_base"
}

# A secret without a final newline still gives a pgpass line that ends with a newline.
case_wal_base_secret_no_newline() {
  wal_new_case base-secret-no-newline
  printf '%s' "$SECRET" >"$CASE/secret"
  wal_run -- basebackup-once
  expect_eq "base: exit code of a secret without a newline" "$RC" "0"
  expect_eq "base: pgpass line of a secret without a newline" \
    "$(cat "$CASE/stub/pgpass.seen" 2>/dev/null)" "$EXPECTED_PGPASS"
  expect_eq "base: the pgpass line ends with a newline" "$(cat "$CASE/stub/pgpass.last" 2>/dev/null)" "0a"
}

case_wal_base_name_exists() {
  local before_top before_base name
  for name in "base_" ".tmp-base_"; do
    new_base_case "base-name-exists${name%%_}" 5
    mkdir "$W/base/$name$WTS"
    before_top=$(wal_listing)
    before_base=$(base_tree)
    wal_run -- basebackup-once
    base_expect_unchanged "existing name $name" "$before_top" "$before_base"
    expect_eq "existing name $name: no tool started" "$(count_events '')" "0"
  done
}

case_wal_base_args_and_secret() {
  local before s
  wal_new_case base-args
  before=$(wal_snapshot)
  wal_run -- basebackup-once extra
  wal_expect_refused "extra argument" "$before"
  for s in empty cr two-lines; do
    wal_new_case "base-secret-$s"
    case $s in
      empty) : >"$CASE/secret" ;;
      cr) printf 'abc\r\n' >"$CASE/secret" ;;
      two-lines) printf 'abc\ndef\n' >"$CASE/secret" ;;
    esac
    before=$(wal_snapshot)
    wal_run -- basebackup-once
    wal_expect_refused "base secret $s" "$before"
  done
}

# A second run stops at the lock. The fake flock of some hosts grants every lock, so the case needs a
# real flock. The image has one.
case_wal_base_parallel() {
  if ! flock_is_real; then
    skip "flock on this host grants a second lock (fake flock)"
    return 0
  fi
  new_base_case base-parallel 1
  WAL_BG=1 wal_run STUB_BASEBACKUP_SLEEP=3 -- basebackup-once
  wait_for_file "$CASE/stub/pgpass.seen" 100
  sleep 0.3
  OUTTAG=.second wal_run -- basebackup-once
  expect_eq "parallel: second run exit code" "$RC" "1"
  expect_eq "parallel: second run ERROR line" "$(error_count "$CASE/out.second")" "1"
  wait_bg
  expect_eq "parallel: first run exit code" "$RC" "0"
  expect_eq "parallel: one pg_basebackup" "$(count_events 'pg_basebackup start')" "1"
}
