# Sourced by backup-test. It holds the cases of backup-health and the helpers of these cases.

health_run() {
  run_script "$@" -- backup-health
}

# make_other_fs_pgdata: a pgdata directory with PG_VERSION on a filesystem other than ROOT.
# It sets OTHER_PGDATA. It returns 1 when the directory OTHER_FS_DIR is not usable.
make_other_fs_pgdata() {
  local dir
  OTHER_PGDATA=""
  [ -d "$OTHER_FS_DIR" ] || return 1
  dir=$(mktemp -d "$OTHER_FS_DIR/pgdata.XXXXXX" 2>/dev/null) || return 1
  SHM_DIRS+=("$dir")
  printf '16\n' >"$dir/PG_VERSION" || return 1
  [ "$(stat -c %d "$dir")" != "$(stat -c %d "$ROOT")" ] || return 1
  OTHER_PGDATA=$dir
}

# health_case <name>: a fresh case whose pgdata is on the other filesystem.
health_case() {
  new_case "$1"
  PGDATA_TEST=$OTHER_PGDATA
}

case_health_age() {
  health_case health-age
  make_set "" 0
  health_run
  expect_eq "health: young daily set is healthy" "$RC" 0
  touch -d "27 hours ago" "$B/$LAST_BASE.dump.sha256"
  health_run
  expect_eq "health: set older than 26 hours fails" "$RC" 1
  touch -d "25 hours ago" "$B/$LAST_BASE.dump.sha256"
  health_run
  expect_eq "health: set younger than 26 hours is healthy" "$RC" 0
}

case_health_suffixes() {
  health_case health-premigrate
  make_set _premigrate 0
  health_run
  expect_eq "health: premigrate set counts" "$RC" 0
  health_case health-now
  make_set _now 0
  health_run
  expect_eq "health: now set counts" "$RC" 0
  expect_ok "health: now set is named" grep -q "recipe_[0-9]*_[0-9]*_now" "$CASE/out"
}

# A set needs all four files. The cases remove one file at a time.
case_health_incomplete() {
  local part
  for part in dump counts globals manifest; do
    health_case "health-without-$part"
    make_set "" 0
    case $part in
      dump) rm -- "$B/$LAST_BASE.dump" ;;
      counts) rm -- "$B/$LAST_BASE.counts" ;;
      globals) rm -- "$B/globals_${LAST_BASE#recipe_}.sql" ;;
      manifest) rm -- "$B/$LAST_BASE.dump.sha256" ;;
    esac
    health_run
    expect_eq "health: set without $part fails" "$RC" 1
    expect_eq "health: set without $part gives one ERROR line" "$(error_count "$CASE/out")" 1
  done
  health_case health-no-set
  health_run
  expect_eq "health: no set fails" "$RC" 1
}

# The temp files of a run and a foreign name are no set (ADR-0074 (b), (c)). They are young here.
case_health_temp_and_foreign() {
  health_case health-temp
  make_temp_set 0
  make_foreign_set 0
  health_run
  expect_eq "health: temp set and foreign name are no set" "$RC" 1
  expect_eq "health: temp set gives one ERROR line" "$(error_count "$CASE/out")" 1
  make_set "" 0
  health_run
  expect_eq "health: a real set beside the temp set is healthy" "$RC" 0
  expect_ok "health: the healthy set is the real one" grep -q "\"set\":\"$LAST_BASE\"" "$CASE/out"
}

case_health_environment() {
  health_case health-no-marker
  make_set "" 0
  rm -- "$B/.backup-marker"
  health_run
  expect_eq "health: missing marker fails" "$RC" 1
  health_case health-no-pgdata
  make_set "" 0
  PGDATA_TEST=$CASE/missing-directory
  health_run
  expect_eq "health: missing pgdata directory fails" "$RC" 1
}

case_health_second_fs() {
  if ! make_other_fs_pgdata; then
    skip "health cases that need a second filesystem (no usable $OTHER_FS_DIR)"
    return
  fi
  case_health_age
  case_health_suffixes
  case_health_incomplete
  case_health_temp_and_foreign
  case_health_environment
  PGDATA_TEST=
}

case_health_same_fs() {
  new_case health-same-fs
  PGDATA_TEST=$CASE/pgdata
  make_set "" 0
  health_run
  expect_eq "health: same filesystem as pgdata fails" "$RC" 1
  expect_eq "health: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "health: same filesystem is the named fault" grep -q 'filesystem of pgdata' "$CASE/out"
  PGDATA_TEST=
}

# A directory without PG_VERSION is no pgdata mount. The check stops before it compares devices.
case_health_no_pg_version() {
  new_case health-no-pg-version
  rm -- "$CASE/pgdata/PG_VERSION"
  PGDATA_TEST=$CASE/pgdata
  make_set "" 0
  health_run
  expect_eq "health (no PG_VERSION): exit code" "$RC" 1
  expect_eq "health (no PG_VERSION): one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "health (no PG_VERSION): PG_VERSION is the named fault" grep -q 'PG_VERSION' "$CASE/out"
  PGDATA_TEST=
}
