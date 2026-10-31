# Sourced by pitr-volume-test. It holds the cases of step 4 of ADR-0106 (f): the selection of the base
# backup (ADR-0106 (c), design D3) and the check of the free space of the target disk.
# A chosen base backup shows in the INFO line "base backup selected", in the field base_backup, and in the
# file origin of the target, because the copy takes that base backup.
# A refused selection or a refused free space writes one ERROR line, copies nothing, and runs no tool.
# It needs lib/helpers.sh, lib/pitr-volume-stubs.sh (add_pitr_base, the df stub), and
# lib/pitr-volume-gate-cases.sh (pitr_refused_case).

# The bases of the selection cases. A is older than B, and B is older than C.
BASE_A=base_20260101_000000
BASE_B=base_20260102_000000
BASE_C=base_20260103_000000

# clear_pitr_bases: remove the default base backup of new_pitr_case.
clear_pitr_bases() { rm -rf -- "${CASE:?}/wal/base"/*; }

# pitr_selected: the base backup that the line "base backup selected" names.
pitr_selected() {
  sed -n 's/.*"message":"base backup selected".*"base_backup":"\([^"]*\)".*/\1/p' "$CASE/out"
}

# pitr_select_case <label> <expected base backup> [VAR=value ...] -- [argument ...]: the run names the base
# backup, copies exactly that one, completes, and leaves the WAL archive as it was. The case may call this
# function several times. Each call empties the target first.
pitr_select_case() {
  local label=$1 expected=$2 wal
  shift 2
  reset_pitr_run
  wal=$(pitr_listing "$CASE/wal")
  pitr_run "$@"
  expect_eq "$label: exit code" "$RC" 0
  expect_eq "$label: no ERROR line" "$(error_count "$CASE/out")" 0
  expect_ok "$label: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_eq "$label: the selected base backup" "$(pitr_selected)" "$expected"
  expect_eq "$label: the target holds the selected base backup" "$(cat "$CASE/target/origin" 2>/dev/null)" "$expected"
  expect_ok "$label: the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
  expect_eq "$label: the WAL archive is unchanged" "$(pitr_listing "$CASE/wal")" "$wal"
}

# add_pitr_decoys: names that never win, even when they sort after every real base backup.
add_pitr_decoys() {
  local dir=$CASE/wal/base
  mkdir -p "$CASE/elsewhere"
  add_pitr_base base_20271231_235959 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
  mv -- "$dir/base_20271231_235959" "$CASE/elsewhere/real"
  ln -s "$CASE/elsewhere/real" "$dir/base_20281231_235959"
  printf 'x\n' >"$dir/base_20291231_235959"
  add_pitr_base .tmp-base_20301231_235959 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base base_20311231_2359 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base base_20321231_235959x 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base xbase_20331231_235959 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base base_20341231_235959.tmp 2026-01-01T00:00:00Z "2026-01-01 00:00:00 UTC"
}

# Without a target time, the newest base backup of the grammar wins (ADR-0106 (c)).
case_pitr_select_newest() {
  new_pitr_case select-newest
  clear_pitr_bases
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "2026-01-02 00:00:00 UTC"
  add_pitr_base "$BASE_C" 2026-01-03T00:10:00Z "2026-01-03 00:00:00 UTC"
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  pitr_select_case "no target: the newest of three" "$BASE_C" --
  new_pitr_case select-newest-single
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  pitr_select_case "no target: a single base backup" "$BASE_A" --
  new_pitr_case select-newest-decoys
  add_pitr_decoys
  pitr_select_case "no target: a link, a file, .tmp-base, and bad names never win" base_20260101_000000 --
  new_pitr_case select-newest-without-times
  add_pitr_base "$BASE_B" - -
  pitr_select_case "no target: the grammar decides, so a base backup without lines wins" "$BASE_B" --
}

# With a target time, the newest base backup that ends at or before the target wins.
case_pitr_select_target() {
  new_pitr_case select-target
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "2026-01-02 00:00:00 UTC"
  pitr_select_case "target after both ends" "$BASE_B" -- --target-time 2026-06-01T00:00:00Z
  pitr_select_case "target between the start and the end of B selects A" "$BASE_A" -- --target-time 2026-01-02T00:05:00Z
  pitr_select_case "target one second before the end of B selects A" "$BASE_A" -- --target-time 2026-01-02T00:09:59Z
  pitr_select_case "target exactly at the end of B selects B" "$BASE_B" -- --target-time 2026-01-02T00:10:00Z
  pitr_select_case "target exactly at the end of A selects A" "$BASE_A" -- --target-time 2026-01-01T00:10:00Z
  pitr_select_case "drill with a target" "$BASE_A" -- --target-time 2026-01-02T00:05:00Z --drill
  new_pitr_case select-target-skips-newer
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "2026-01-02 00:00:00 UTC"
  add_pitr_base "$BASE_C" 2026-01-03T00:10:00Z "2026-01-03 00:00:00 UTC"
  pitr_select_case "target inside the run of C selects B" "$BASE_B" -- --target-time 2026-01-03T00:05:00Z
  pitr_select_case "target inside the run of B selects A" "$BASE_A" -- --target-time 2026-01-02T00:05:00Z
  new_pitr_case select-target-decoys
  add_pitr_decoys
  pitr_select_case "target: a link, a file, .tmp-base, and bad names never win" base_20260101_000000 -- --target-time 2036-01-01T00:00:00Z
}

# A base backup without backup_end_time counts with its START TIME in UTC (ADR-0106 (c)).
case_pitr_select_start_time() {
  new_pitr_case select-start-time
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "2026-01-02 00:00:00 UTC"
  pitr_select_case "no end time: the start time is at the target" "$BASE_B" -- --target-time 2026-01-02T00:00:00Z
  pitr_select_case "no end time: the start time is after the target" "$BASE_A" -- --target-time 2026-01-01T23:59:59Z
  pitr_select_case "no end time: the start time is before the target" "$BASE_B" -- --target-time 2026-01-02T00:00:01Z
  new_pitr_case select-end-time-first
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "2026-01-02 00:00:00 UTC"
  pitr_select_case "an end time wins over the start time" "$BASE_A" -- --target-time 2026-01-02T00:05:00Z
}

PITR_NEVER_COUNT=0

# pitr_never_wins_case <label> <backup_end_time content> <START TIME text>: B is newer than A and has the
# given lines. With a target far after both, B would win if its lines counted.
pitr_never_wins_case() {
  PITR_NEVER_COUNT=$((PITR_NEVER_COUNT + 1))
  new_pitr_case "never-$PITR_NEVER_COUNT"
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" "$2" "$3"
  pitr_select_case "never wins: $1" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
}

# Bad times, bad files, and bad labels never win. The older base backup A wins in each case.
case_pitr_select_never_wins() {
  local label="2026-01-02 00:00:00 UTC" zone
  # A zone other than UTC (no end time file).
  for zone in CET PST "UTC+1" utc GMT "" ; do
    pitr_never_wins_case "zone [$zone]" - "2026-01-02 00:00:00${zone:+ $zone}"
  done
  pitr_never_wins_case "start time of another form" - "2026-01-02T00:00:00 UTC"
  pitr_never_wins_case "start time without seconds" - "2026-01-02 00:00 UTC"
  pitr_never_wins_case "start time that is no date" - "2026-02-30 00:00:00 UTC"
  # A malformed end time never wins and never falls back to the start time.
  pitr_never_wins_case "end time without Z" 2026-01-02T00:10:00 "$label"
  pitr_never_wins_case "end time with a zone offset" 2026-01-02T00:10:00+00:00 "$label"
  pitr_never_wins_case "end time in lower case" 2026-01-02t00:10:00z "$label"
  pitr_never_wins_case "end time that is no date" 2026-02-30T00:10:00Z "$label"
  pitr_never_wins_case "end time with text" garbage "$label"
  pitr_never_wins_case "end time with a leading space" " 2026-01-02T00:10:00Z" "$label"
  pitr_never_wins_case "end time with a trailing space" "2026-01-02T00:10:00Z " "$label"
  pitr_never_wins_case "end time in the START TIME form" "2026-01-02 00:10:00 UTC" "$label"
  pitr_never_wins_case "empty end time" "" "$label"
  new_pitr_case never-two-lines
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" 2026-01-02T00:10:00Z "$label"
  printf '2026-01-02T00:11:00Z\n' >>"$CASE/wal/base/$BASE_B/backup_end_time"
  pitr_select_case "never wins: end time of two lines" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  new_pitr_case never-no-newline
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "$label"
  printf '2026-01-02T00:10:00Z' >"$CASE/wal/base/$BASE_B/backup_end_time"
  pitr_select_case "never wins: end time without a newline" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  # No time at all.
  pitr_never_wins_case "neither line" - -
  new_pitr_case never-no-label-file
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - -
  rm -- "$CASE/wal/base/$BASE_B/backup_label"
  pitr_select_case "never wins: no end time and no backup_label" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  new_pitr_case never-two-start-lines
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "$label"
  printf 'START TIME: 2026-01-02 00:00:00 UTC\n' >>"$CASE/wal/base/$BASE_B/backup_label"
  pitr_select_case "never wins: two lines START TIME" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  # The files are links or directories.
  new_pitr_case never-end-time-link
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "$label"
  printf '2026-01-02T00:10:00Z\n' >"$CASE/real-end-time"
  ln -s "$CASE/real-end-time" "$CASE/wal/base/$BASE_B/backup_end_time"
  pitr_select_case "never wins: backup_end_time is a link" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  new_pitr_case never-end-time-directory
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "$label"
  mkdir "$CASE/wal/base/$BASE_B/backup_end_time"
  pitr_select_case "never wins: backup_end_time is a directory" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
  new_pitr_case never-label-link
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "$label"
  mv -- "$CASE/wal/base/$BASE_B/backup_label" "$CASE/real-label"
  ln -s "$CASE/real-label" "$CASE/wal/base/$BASE_B/backup_label"
  pitr_select_case "never wins: backup_label is a link" "$BASE_A" -- --target-time 2036-01-01T00:00:00Z
}

# No match: one ERROR line, no copy, no tool.
case_pitr_select_none() {
  new_pitr_case none-empty
  clear_pitr_bases
  pitr_refused_case "no base backup at all" "no base backup" --
  new_pitr_case none-missing-directory
  rmdir -- "$CASE/wal/base" 2>/dev/null || rm -rf -- "$CASE/wal/base"
  pitr_refused_case "no directory wal/base" "no base backup" --
  new_pitr_case none-decoys-only
  clear_pitr_bases
  add_pitr_decoys
  pitr_refused_case "only a link, a file, .tmp-base, and bad names" "no base backup" --
  new_pitr_case none-before-target
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC"
  add_pitr_base "$BASE_B" - "2026-01-02 00:00:00 UTC"
  pitr_refused_case "target before every end time" "no base backup" -- --target-time 2026-01-01T00:09:59Z
  new_pitr_case none-without-times
  clear_pitr_bases
  add_pitr_base "$BASE_A" - -
  pitr_refused_case "target and a base backup without lines" "no base backup" -- --target-time 2036-01-01T00:00:00Z
  new_pitr_case none-other-zone
  clear_pitr_bases
  add_pitr_base "$BASE_A" - "2026-01-01 00:00:00 CET"
  pitr_refused_case "target and a start time in another zone" "no base backup" -- --target-time 2036-01-01T00:00:00Z
  expect_no "no match: no selected line" grep -q 'base backup selected' "$CASE/out"
}

# pitr_space_need <base backup name>: the bytes that the copy needs. The base backup size is the
# du -sb size. The segment files below hold 1000 and 500 bytes. The other files do not count.
pitr_space_need() {
  local base
  base=$(du -sb -- "$CASE/wal/base/$1" | cut -f1)
  echo $((base + 1500))
}

# new_pitr_space_case <name>: one base backup and the files at the top of the WAL archive.
new_pitr_space_case() {
  new_pitr_case "$1"
  clear_pitr_bases
  add_pitr_base "$BASE_A" 2026-01-01T00:10:00Z "2026-01-01 00:00:00 UTC" 5000
  head -c 1000 /dev/zero >"$CASE/wal/000000010000000000000001"
  head -c 500 /dev/zero >"$CASE/wal/000000010000000000000002.partial"
  head -c 7000 /dev/zero >"$CASE/wal/00000002.history"
  head -c 7000 /dev/zero >"$CASE/wal/000000010000000000000003.backup"
  head -c 7000 /dev/zero >"$CASE/wal/base/.not-a-base"
}

# The free bytes minus the copy must stay at or above disk.warn_percent (15) of the disk size.
case_pitr_free_space() {
  local need limit
  new_pitr_space_case space-boundary
  need=$(pitr_space_need "$BASE_A")
  limit=150000 # 15 percent of a disk of 1000000 bytes
  pitr_select_case "free space exactly at the percent passes" "$BASE_A" \
    STUB_DF_AVAIL=$((need + limit)) STUB_DF_SIZE=1000000 --
  expect_ok "free space exactly at the percent: the line holds the sizes" \
    grep -qF "\"base_bytes\":$((need - 1500)),\"segment_bytes\":1500" "$CASE/out"
  pitr_select_case "free space far above the percent passes" "$BASE_A" \
    STUB_DF_AVAIL=$((need + 900000)) STUB_DF_SIZE=1000000 --
  new_pitr_space_case space-one-below
  pitr_refused_case "free space one byte below the percent" "disk.warn_percent" \
    STUB_DF_AVAIL=$((need + limit - 1)) STUB_DF_SIZE=1000000 --
  new_pitr_space_case space-below-copy
  pitr_refused_case "free space below the size of the copy" "disk.warn_percent" \
    STUB_DF_AVAIL=$((need - 1)) STUB_DF_SIZE=1000000 --
  new_pitr_space_case space-full
  pitr_refused_case "a full disk" "disk.warn_percent" STUB_DF_AVAIL=0 STUB_DF_SIZE=1000000 --
  new_pitr_space_case space-segments-count
  pitr_refused_case "the segment files count (the base backup alone would pass)" "disk.warn_percent" \
    STUB_DF_AVAIL=$((need - 1500 + limit + 1499)) STUB_DF_SIZE=1000000 --
  new_pitr_space_case space-drill
  pitr_refused_case "a drill checks the free space too" "disk.warn_percent" \
    STUB_DF_AVAIL=$((need + limit - 1)) STUB_DF_SIZE=1000000 -- --target-time 2026-06-01T00:00:00Z --drill
  new_pitr_space_case space-df-unreadable
  pitr_refused_case "df prints no numbers" "free space" STUB_DF_TEXT="n/a n/a" --
  new_pitr_space_case space-df-extra
  pitr_refused_case "df prints a third field" "free space" STUB_DF_TEXT="1000000 2000000 3" --
  new_pitr_space_case space-df-zero-size
  pitr_refused_case "df prints a disk of size 0" "free space" STUB_DF_AVAIL=0 STUB_DF_SIZE=0 --
  new_pitr_space_case space-df-fails
  pitr_refused_case "df fails" "free space" STUB_DF_FAIL=1 --
}
