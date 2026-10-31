# Sourced by pitr-volume-test. It holds the cases of steps 1 to 3 of ADR-0106 (f): the inputs, the user,
# the marker file, the lock, and the file .system-identifier. A refused gate writes one ERROR line
# before any stub of a PostgreSQL tool runs.
# It needs lib/helpers.sh (the expect functions) and lib/pitr-volume-stubs.sh (the case helpers).

VALID_TIME=2026-05-01T10:00:00Z

# pitr_refused_checks <name> <wal listing> <target listing>: the checks of a run that a gate refused.
pitr_refused_checks() {
  local name=$1
  expect_no "$name: exit code is not 0" test "$RC" -eq 0
  expect_eq "$name: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "$name: the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
  expect_eq "$name: no PostgreSQL tool ran" "$(wc -c <"$CASE/stub/log")" 0
  expect_eq "$name: the WAL archive is unchanged" "$(pitr_listing "$CASE/wal")" "$2"
  expect_eq "$name: the target directory is unchanged" "$(pitr_listing "$CASE/target")" "$3"
  expect_no "$name: no success line" grep -q 'completed' "$CASE/out"
}

# pitr_refused_case <name> <text in the ERROR line> [VAR=value ...] -- [argument ...]: the case is ready.
pitr_refused_case() {
  local name=$1 text=$2 wal target
  shift 2
  wal=$(pitr_listing "$CASE/wal")
  target=$(pitr_listing "$CASE/target")
  pitr_run "$@"
  pitr_refused_checks "$name" "$wal" "$target"
  expect_ok "$name: the ERROR line holds [$text]" error_has "$text"
}

# pitr_refused_no_lock <name>: a refusal of step 1 or of the marker check takes no lock.
pitr_refused_no_lock() {
  expect_eq "$1: no lock call" "$(wc -c <"$CASE/stub/flock.log")" 0
}

# pitr_usage_case <label> [argument ...]: a run with bad arguments stops with the usage line.
pitr_usage_case() {
  local label=$1
  shift
  new_pitr_case "arg-$label"
  pitr_refused_case "argument ($label)" "usage" -- "$@"
  pitr_refused_no_lock "argument ($label)"
}

# Step 1: the arguments are none, --target-time <time>, and --drill (which needs --target-time).
case_pitr_arguments() {
  pitr_usage_case unknown-option --bogus
  pitr_usage_case positional extra
  pitr_usage_case empty-argument ""
  pitr_usage_case target-time-without-value --target-time
  pitr_usage_case target-time-twice --target-time "$VALID_TIME" --target-time "$VALID_TIME"
  pitr_usage_case drill-twice --target-time "$VALID_TIME" --drill --drill
  pitr_usage_case unknown-after-good --target-time "$VALID_TIME" --force
  pitr_usage_case equals-form "--target-time=$VALID_TIME"
  new_pitr_case arg-empty-value
  pitr_refused_case "argument (empty target time)" "grammar" -- --target-time ""
  new_pitr_case arg-drill-alone
  pitr_refused_case "drill without a target time" "--drill needs --target-time" -- --drill
  pitr_refused_no_lock "drill without a target time"
}

# Step 1: the target time has the grammar of ADR-0106 (a) and names a real UTC time.
case_pitr_target_time() {
  local time i=0
  for time in 2026-05-01T10:00:00 2026-05-01T10:00:00+00:00 2026-05-01T10:00:00z "2026-05-01 10:00:00Z" \
    2026-05-01T10:00Z 2026-5-1T10:00:00Z 20260501T100000Z "2026-05-01T10:00:00Z " $'2026-05-01T10:00:00Z\n' \
    2026-05-01T10:00:00.5Z x ../2026-05-01T10:00:00Z; do
    i=$((i + 1))
    new_pitr_case "time-grammar-$i"
    pitr_refused_case "target time outside the grammar #$i" "grammar" -- --target-time "$time"
    pitr_refused_no_lock "target time outside the grammar #$i"
  done
  i=0
  for time in 2026-02-30T10:00:00Z 2026-02-29T10:00:00Z 2026-04-31T00:00:00Z 2026-13-01T10:00:00Z \
    2026-00-10T10:00:00Z 2026-05-00T10:00:00Z 2026-05-01T24:00:00Z 2026-05-01T10:60:00Z 2026-05-01T10:00:60Z; do
    i=$((i + 1))
    new_pitr_case "time-date-$i"
    pitr_refused_case "target time $time is no real date" "not a real date" -- --target-time "$time"
    pitr_refused_no_lock "target time $time"
  done
  new_pitr_case time-drill-bad
  pitr_refused_case "drill with a bad target time" "grammar" -- --target-time 2026-05-01T10:00:00 --drill
}

# Step 1: PITR_EXPECTED_LSN has the grammar of a PostgreSQL LSN in upper-case hex.
case_pitr_expected_lsn() {
  local lsn i=0
  for lsn in 0 /3000000 0/ 0/3000000G 0/30000000A 123456789/0 0/3000000a 0x0/1 "0/3000000 " $'0/3000000\n' \
    0/3000000/1 -1/0 '0/1;x' '0/$(id)'; do
    i=$((i + 1))
    new_pitr_case "lsn-$i"
    pitr_refused_case "PITR_EXPECTED_LSN outside the grammar #$i" "PITR_EXPECTED_LSN" "PITR_EXPECTED_LSN=$lsn" --
    pitr_refused_no_lock "PITR_EXPECTED_LSN outside the grammar #$i"
  done
}

# Step 1: the target has to be an empty directory. A live data directory is never empty.
case_pitr_target_not_empty() {
  local variant
  for variant in file directory hidden; do
    new_pitr_case "target-$variant"
    case $variant in
      file) printf '16\n' >"$CASE/target/PG_VERSION" ;;
      directory) mkdir "$CASE/target/base" ;;
      hidden) : >"$CASE/target/.hidden" ;;
    esac
    pitr_refused_case "target with a $variant" "the target directory is not empty" --
    pitr_refused_no_lock "target with a $variant"
  done
  new_pitr_case target-missing
  rmdir "$CASE/target"
  mkdir "$CASE/target-other"
  pitr_run --
  expect_no "target missing: exit code is not 0" test "$RC" -eq 0
  expect_eq "target missing: one ERROR line" "$(error_count "$CASE/out")" 1
  expect_ok "target missing: the ERROR line says so" error_has "the target directory is missing"
  expect_eq "target missing: no PostgreSQL tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# Step 1: the run needs uid 999 and the user name postgres (ADR-0105 (r)).
case_pitr_user() {
  local uid
  for uid in 0 1000 998 9990; do
    new_pitr_case "uid-$uid"
    pitr_refused_case "uid $uid" "must run as uid 999" "STUB_ID_UID=$uid" --
    pitr_refused_no_lock "uid $uid"
  done
  new_pitr_case user-root
  pitr_refused_case "user name root" "must run as the user postgres" STUB_ID_NAME=root --
  pitr_refused_no_lock "user name root"
  new_pitr_case user-postgres2
  pitr_refused_case "user name postgres2" "must run as the user postgres" STUB_ID_NAME=postgres2 --
  new_pitr_case user-unreadable
  pitr_refused_case "unreadable user" "cannot read the user id" STUB_ID_FAIL=1 --
  new_pitr_case user-injection
  pitr_refused_case "user name with JSON characters" "must run as the user postgres" 'STUB_ID_NAME=a"b\c' --
}

# Step 1: WAL_DIR is a plain absolute path.
case_pitr_wal_dir() {
  local dir i=0
  for dir in wal /wal/../etc /wal/. relative/path "/wal dir"; do
    i=$((i + 1))
    new_pitr_case "waldir-$i"
    pitr_refused_case "WAL_DIR [$dir]" "WAL_DIR" "WAL_DIR=$dir" --
    pitr_refused_no_lock "WAL_DIR [$dir]"
  done
}

# Step 2: the marker file of the WAL archive disk must exist and be a regular file.
case_pitr_marker() {
  new_pitr_case marker-missing
  rm -- "$CASE/wal/.walarchive-marker"
  pitr_refused_case "missing marker" "marker" --
  pitr_refused_no_lock "missing marker"
  new_pitr_case marker-link
  rm -- "$CASE/wal/.walarchive-marker"
  : >"$CASE/wal/real-marker"
  ln -s real-marker "$CASE/wal/.walarchive-marker"
  pitr_refused_case "marker is a symbolic link" "marker" --
  new_pitr_case marker-directory
  rm -- "$CASE/wal/.walarchive-marker"
  mkdir "$CASE/wal/.walarchive-marker"
  pitr_refused_case "marker is a directory" "marker" --
}

# Step 2: the lock file is opened read-only and taken with flock -n. A held lock stops the run.
case_pitr_lock() {
  new_pitr_case lock-missing
  rm -- "$CASE/wal/.lock"
  pitr_refused_case "missing lock file" "lock" --
  new_pitr_case lock-link
  rm -- "$CASE/wal/.lock"
  : >"$CASE/wal/real-lock"
  ln -s real-lock "$CASE/wal/.lock"
  pitr_refused_case "lock file is a symbolic link" "lock" --
  new_pitr_case lock-held
  # The test shell holds the lock on its own file description. The script opens another one.
  exec 8<"$CASE/wal/.lock"
  "$REAL_FLOCK" -n 8
  pitr_refused_case "held lock" "another run holds the lock" --
  exec 8<&-
  expect_ok "held lock: the script tried flock -n" grep -qx 'ARGV flock -n 9' "$CASE/stub/flock.log"
  expect_ok "held lock: the script opened the lock file read-only" grep -qx 'FDMODE 0' "$CASE/stub/flock.log"
  expect_eq "held lock: the lock file stays empty" "$(wc -c <"$CASE/wal/.lock")" 0
  new_pitr_case lock-released
  exec 8<"$CASE/wal/.lock"
  "$REAL_FLOCK" -n 8
  exec 8<&-
  pitr_run --
  expect_eq "released lock: exit code" "$RC" 0
  expect_ok "released lock: the run completes" grep -q 'pitr volume completed' "$CASE/out"
}

# Step 3: the file .system-identifier is one line of digits (the checks of wal-identity.sh).
case_pitr_identifier() {
  local variant
  new_pitr_case ident-missing
  rm -- "$CASE/wal/.system-identifier"
  pitr_refused_case "missing .system-identifier" "cannot read the file .system-identifier" --
  new_pitr_case ident-link
  rm -- "$CASE/wal/.system-identifier"
  printf '7300000000000000001\n' >"$CASE/wal/real-identifier"
  ln -s real-identifier "$CASE/wal/.system-identifier"
  pitr_refused_case ".system-identifier is a symbolic link" "cannot read the file .system-identifier" --
  for variant in empty letters two-lines blank-line carriage-return trailing-space leading-space nul; do
    new_pitr_case "ident-$variant"
    case $variant in
      empty) : >"$CASE/wal/.system-identifier" ;;
      letters) printf '73000abc\n' >"$CASE/wal/.system-identifier" ;;
      two-lines) printf '7300000000000000001\n7300000000000000002\n' >"$CASE/wal/.system-identifier" ;;
      blank-line) printf '\n' >"$CASE/wal/.system-identifier" ;;
      carriage-return) printf '7300000000000000001\r\n' >"$CASE/wal/.system-identifier" ;;
      trailing-space) printf '7300000000000000001 \n' >"$CASE/wal/.system-identifier" ;;
      leading-space) printf ' 7300000000000000001\n' >"$CASE/wal/.system-identifier" ;;
      nul) printf '7300\0000000000000001\n' >"$CASE/wal/.system-identifier" ;;
    esac
    pitr_refused_case "malformed .system-identifier ($variant)" "is malformed" --
  done
}

# The library files must load. A missing library gives one ERROR line before any tool.
case_pitr_library() {
  local name
  for name in recovery-steps.sh wal-identity.sh; do
    new_pitr_case "lib-$name"
    mkdir "$CASE/lib"
    cp -- "$ROOT/etc"/*.sh "$CASE/lib/"
    rm -- "$CASE/lib/$name"
    pitr_refused_case "missing $name" "cannot load $name" "WALARCHIVE_LIB_DIR=$CASE/lib" --
  done
  # restore-steps.sh loads before the log functions of the script exist. The load failure is one line on stderr.
  new_pitr_case lib-restore-steps.sh
  mkdir "$CASE/lib"
  cp -- "$ROOT/etc"/*.sh "$CASE/lib/"
  rm -- "$CASE/lib/restore-steps.sh"
  pitr_run "WALARCHIVE_LIB_DIR=$CASE/lib" --
  expect_eq "missing restore-steps.sh: exit code" "$RC" 1
  expect_ok "missing restore-steps.sh: the message is on stderr"     grep -qx 'pitr-volume: cannot load restore-steps.sh' "$CASE/err"
  expect_eq "missing restore-steps.sh: no PostgreSQL tool ran" "$(wc -c <"$CASE/stub/log")" 0
}

# A run with valid inputs passes the gates of steps 1 to 3 and runs on to the last line. It writes no ERROR
# line, keeps the lock to the exit, leaves the WAL archive as it was, and copies the selected base backup.
case_pitr_gates_pass() {
  local variant args envs wal
  for variant in plain target-time drill expected-lsn all; do
    new_pitr_case "pass-$variant"
    args=()
    envs=()
    case $variant in
      target-time) args=(--target-time "$VALID_TIME") ;;
      drill) args=(--target-time "$VALID_TIME" --drill) ;;
      expected-lsn) envs=(PITR_EXPECTED_LSN=16/B374D848 "STUB_HISTORY_TEXT=$(pitr_history 16/B374D848)") ;;
      all) args=(--drill --target-time 2028-02-29T23:59:59Z); envs=(PITR_EXPECTED_LSN=0/3000000) ;;
    esac
    wal=$(pitr_listing "$CASE/wal")
    pitr_run ${envs[@]+"${envs[@]}"} -- ${args[@]+"${args[@]}"}
    expect_eq "pass ($variant): exit code" "$RC" 0
    expect_eq "pass ($variant): no ERROR line" "$(error_count "$CASE/out")" 0
    expect_ok "pass ($variant): the output is JSON of the job pitr-volume" pitr_json_ok "$CASE/out"
    expect_eq "pass ($variant): the selected base backup" "$(pitr_selected)" base_20260101_000000
    expect_eq "pass ($variant): the target holds the selected base backup" "$(cat "$CASE/target/origin")" base_20260101_000000
    expect_eq "pass ($variant): the WAL archive is unchanged" "$(pitr_listing "$CASE/wal")" "$wal"
    expect_ok "pass ($variant): the lock was taken with flock -n" grep -qx 'ARGV flock -n 9' "$CASE/stub/flock.log"
    expect_ok "pass ($variant): the lock file was open read-only" grep -qx 'FDMODE 0' "$CASE/stub/flock.log"
    expect_ok "pass ($variant): the run writes the last line" grep -q 'pitr volume completed' "$CASE/out"
    case $variant in
      drill | all) expect_ok "pass ($variant): the file pitr-drill exists" test -f "$CASE/target/pitr-drill" ;;
      *) expect_no "pass ($variant): no file pitr-drill" test -e "$CASE/target/pitr-drill" ;;
    esac
  done
  new_pitr_case pass-fields
  pitr_run -- --target-time "$VALID_TIME" --drill
  expect_ok "pass: the started line holds the drill field" grep -q '"message":"pitr volume started".*"drill":true' "$CASE/out"
  expect_ok "pass: the started line holds the target time" grep -qF "\"target_time\":\"$VALID_TIME\"" "$CASE/out"
}

# The script is a bash script of the shape of restore-volume.
case_pitr_static_rules() {
  local script=$BIN/pitr-volume
  expect_ok "static: pitr-volume exists" test -f "$script"
  expect_ok "static: pitr-volume is executable" test -x "$script"
  expect_ok "static: bash -n pitr-volume" bash -n "$script"
  expect_no "static: xtrace" grep -Eq 'set[[:space:]]+-[A-Za-z]*x|xtrace' "$script"
  expect_no "static: carriage return" grep -q $'\r' "$script"
  expect_no "static: password in argv or environment" \
    grep -Eq 'PGPASSWORD=|--password[^-]|[[:space:]]-W([[:space:]]|$)' "$script"
  expect_no "static: rm -rf" grep -Eq 'rm[[:space:]]+-[A-Za-z]*r[A-Za-z]*f|rm[[:space:]]+-[A-Za-z]*f[A-Za-z]*r' "$script"
  expect_no "static: eval" grep -Eq '(^|[[:space:];&(])eval[[:space:]]' "$script"
  expect_ok "static: strict mode" grep -qx 'set -euo pipefail' "$script"
  expect_ok "static: the lock is opened read-only on descriptor 9" grep -Eq 'exec 9<"[^"]*"' "$script"
  expect_no "static: the lock file is opened for writing" grep -Eq 'exec 9>' "$script"
  expect_ok "static: the lock uses flock -n" grep -Eq 'flock -n 9' "$script"
  expect_ok "static: the script sources recovery-steps.sh" grep -q 'recovery-steps.sh' "$script"
  expect_ok "static: the script sources wal-identity.sh" grep -q 'wal-identity.sh' "$script"
  expect_ok "static: the script sources restore-steps.sh" grep -q 'restore-steps.sh' "$script"
  case_pitr_roles_defined_once
  case_pitr_chain_sql_defined_once
  case_pitr_clean_json_text
}

# The script keeps its own text cleaner, because the empty check of read_text_value needs deletion. The
# library restore-steps.sh defines clean, which replaces each character with a question mark. The local
# function has another name, so it does not shadow the library one.
case_pitr_clean_json_text() {
  local script=$BIN/pitr-volume
  expect_eq "clean_json_text: pitr-volume defines no clean" "$(grep -c '^clean()' "$script" || true)" 0
  expect_eq "clean_json_text: pitr-volume defines clean_json_text once" "$(grep -c '^clean_json_text()' "$script" || true)" 1
  expect_no "clean_json_text: no call of the bare name clean" \
    grep -Eq '\$\(clean[[:space:]]|[|;&][[:space:]]*clean[[:space:]]' "$script"
  expect_ok "clean_json_text: read_text_value calls clean_json_text" \
    grep -Fq '"$(clean_json_text "$raw")"' "$script"
  expect_ok "clean_json_text: DETAIL_LENGTH is set for the library clean" \
    grep -qE '^DETAIL_LENGTH=160 # restore_test\.detail_length of reliability\.md section 7\.11' "$script"
}

# The comparison of the roles (ADR-0103 (m)) has one definition, in lib/restore-steps.sh. Both scripts call
# it, and neither script holds a second copy that can drift.
case_pitr_roles_defined_once() {
  local lib=$HERE/../lib/restore-steps.sh
  expect_ok "roles once: restore-steps.sh exists" test -f "$lib"
  expect_eq "roles once: restore-steps.sh defines compare_roles once" \
    "$(grep -c '^compare_roles()' "$lib")" 1
  expect_no "roles once: restore-volume defines no roles comparison" \
    grep -Eq '^(compare_roles|check_roles)\(\)' "$BIN/restore-volume"
  expect_no "roles once: pitr-volume defines no roles comparison" \
    grep -Eq '^(compare_roles|check_roles)\(\)' "$BIN/pitr-volume"
  expect_ok "roles once: restore-volume calls compare_roles" grep -Eq '^[[:space:]]+compare_roles[[:space:]]' "$BIN/restore-volume"
  expect_ok "roles once: pitr-volume calls compare_roles" grep -Eq '^compare_roles[[:space:]]' "$BIN/pitr-volume"
}

# The audit chain SQL (data-model.md 4.9) has one definition, in lib/restore-steps.sh. The library
# recovery-steps.sh and the scripts use that value, so no second copy can drift (ADR-0106 (h)).
case_pitr_chain_sql_defined_once() {
  local lib=$HERE/../lib defs
  defs=$(grep -l '^CHAIN_SQL=' "$lib"/*.sh "$BIN"/* | sed 's|.*/||' | sort | tr '
' ' ')
  expect_eq "chain sql once: only restore-steps.sh defines CHAIN_SQL" "$defs" "restore-steps.sh "
  expect_eq "chain sql once: the text of the chain check exists once"     "$(grep -c 'public.audit_canonical(a)' "$lib/restore-steps.sh" "$lib/recovery-steps.sh" "$BIN/pitr-check" "$BIN/pitr-volume" | tr '
' ' ')"     "$lib/restore-steps.sh:1 $lib/recovery-steps.sh:0 $BIN/pitr-check:0 $BIN/pitr-volume:0 "
  expect_ok "chain sql once: pitr-check sources restore-steps.sh" grep -q '^source .*restore-steps\.sh' "$BIN/pitr-check"
  expect_ok "chain sql once: recovery_check_chain uses CHAIN_SQL"     grep -Fq 'scratch_psql "$CHAIN_SQL"' "$lib/recovery-steps.sh"
}
