# Sourced by restore-test-test. It holds the cases of the checks of the code of the dump
# (ADR-0067 (h) to (l), ADR-0076): the name list of step 2 and the comparisons of steps 4 and 5.

# unknown_entry_case <name> <entry of pg_restore --list> <text in the ERROR line>
# An unknown entry stops the test before the cluster exists (step 2).
unknown_entry_case() {
  failed_restore_case "$1" "STUB_LIST_EXTRA=$2"
  expect_eq "$1: only the list ran" "$(event_sequence)" "EVENT list,"
  expect_ok "$1: the ERROR line names the entry" error_has "$3"
  expect_ok "$1: the ERROR line says that the file lacks the entry" \
    error_has "restore-code.expected does not hold"
}

case_restore_unknown_entries() {
  unknown_entry_case unknown-function \
    "240; 1255 16499 FUNCTION public planted() recipe_migrator" "public planted()"
  unknown_entry_case unknown-trigger \
    "241; 2620 16498 TRIGGER public machine planted recipe_migrator" "public machine planted"
  unknown_entry_case unknown-event-trigger \
    "242; 3466 16497 EVENT TRIGGER - planted postgres" "- planted"
  unknown_entry_case function-in-another-schema \
    "243; 1255 16496 FUNCTION other set_updated_at() recipe_migrator" "other set_updated_at()"
  failed_restore_case list-fails STUB_LIST_FAIL=1
  expect_ok "list fails: the ERROR line names the list" error_has "list"
}

# The ERROR line of a comparison has the fields missing and extra. A changed line counts in both.
# A changed function, CHECK constraint, or generated column stops the test after the restore of
# pre-data and before the data load. The sequence ends with the comparison and the stop.
case_restore_changed_pre_data_code() {
  local kind
  for kind in FUNCTION CHECK GENERATED; do
    failed_restore_case "changed-$kind" "STUB_CODE_CHANGE_KIND=$kind"
    expect_eq "changed $kind: stops before the data load" "$(event_sequence)" \
      "${SEQ_TO_PRE_DATA}EVENT pg_ctl stop,"
    expect_ok "changed $kind: the ERROR line names the section" error_has "pre-data"
    expect_ok "changed $kind: one line missing" error_has '"missing":1,"extra":1'
  done
  failed_restore_case changed-function-first STUB_CODE_CHANGE_KIND=FUNCTION
  expect_ok "changed function: the ERROR line names the function" \
    error_has 'FUNCTION|audit_guard audit_objects_dropped()'
  expect_no "changed function: the ERROR line holds no hash" \
    bash -c "grep '\"level\":\"ERROR\"' '$CASE/out' | grep -Eq '[0-9a-f]{64}'"
}

# A CHECK constraint that pg_dump moves to post-data gives a missing line (ADR-0067 (k)).
case_restore_missing_and_extra_pre_data_code() {
  failed_restore_case check-moved STUB_CODE_DROP_KIND=CHECK
  expect_eq "check moved: stops before the data load" "$(event_sequence)" \
    "${SEQ_TO_PRE_DATA}EVENT pg_ctl stop,"
  expect_ok "check moved: one line missing" error_has '"missing":1,"extra":0'
  failed_restore_case planted-function \
    'STUB_CODE_EXTRA=FUNCTION|public planted()|recipe_migrator|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
  expect_eq "planted function: stops before the data load" "$(event_sequence)" \
    "${SEQ_TO_PRE_DATA}EVENT pg_ctl stop,"
  expect_ok "planted function: one line extra" error_has '"missing":0,"extra":1'
}

# A trigger and an event trigger are in post-data. The first comparison does not see them, so the
# data load runs first. The comparison after post-data stops the test before the row counts.
case_restore_changed_post_data_code() {
  local kind
  for kind in TRIGGER "EVENT TRIGGER"; do
    failed_restore_case "changed-$kind" "STUB_CODE_CHANGE_KIND=$kind"
    expect_eq "changed $kind: stops before the row counts" "$(event_sequence)" \
      "${SEQ_TO_POST_DATA}EVENT pg_ctl stop,"
    expect_ok "changed $kind: the ERROR line names the section" error_has "post-data"
    expect_ok "changed $kind: one line missing" error_has '"missing":1,"extra":1'
  done
  failed_restore_case changed-trigger-name STUB_CODE_CHANGE_KIND=TRIGGER
  expect_ok "changed trigger: the ERROR line names the trigger" \
    error_has 'TRIGGER|public machine machine_set_updated_at'
  failed_restore_case planted-trigger \
    'STUB_CODE_EXTRA=TRIGGER|public machine planted|recipe_migrator|aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
  expect_eq "planted trigger: stops before the row counts" "$(event_sequence)" \
    "${SEQ_TO_POST_DATA}EVENT pg_ctl stop,"
  failed_restore_case dropped-trigger STUB_CODE_DROP_KIND=TRIGGER
  expect_ok "dropped trigger: one line missing" error_has '"missing":1,"extra":0'
}

# A disabled trigger gives the same pg_get_triggerdef text. The line holds tgenabled, so the state
# changes the hash. The comparison after post-data stops the test (ADR-0078 (a)).
case_restore_disabled_trigger() {
  local state
  for state in D R A; do
    failed_restore_case "trigger-state-$state" "STUB_TRIGGER_STATE=$state"
    expect_eq "trigger state $state: stops before the row counts" "$(event_sequence)" \
      "${SEQ_TO_POST_DATA}EVENT pg_ctl stop,"
    expect_ok "trigger state $state: the ERROR line names the section" error_has "post-data"
    expect_ok "trigger state $state: one line missing" error_has '"missing":1,"extra":1'
    expect_ok "trigger state $state: the ERROR line names the trigger" \
      error_has 'TRIGGER|public machine machine_set_updated_at'
  done
  new_restore_case trigger-state-enabled
  restore_run STUB_TRIGGER_STATE=O
  expect_eq "trigger state O: exit code" "$RC" 0
}

# refused_kind_case <name> <entry of pg_restore --list> <kind>
# A kind that the release does not use stops the test before the cluster exists (ADR-0078 (b)).
refused_kind_case() {
  failed_restore_case "$1" "STUB_LIST_EXTRA=$2"
  expect_eq "$1: only the list ran" "$(event_sequence)" "EVENT list,"
  expect_ok "$1: the ERROR line says that the kind is refused" error_has "a kind that the restore test refuses"
  expect_ok "$1: the ERROR line names the kind" error_has "\"kind\":\"$3\""
}

case_restore_refused_kinds() {
  refused_kind_case refused-procedure \
    "250; 1255 16500 PROCEDURE public planted() recipe_migrator" PROCEDURE
  refused_kind_case refused-aggregate \
    "251; 1255 16501 AGGREGATE public planted(integer) recipe_migrator" AGGREGATE
  refused_kind_case refused-materialized-view \
    "252; 1259 16502 MATERIALIZED VIEW public planted recipe_migrator" "MATERIALIZED VIEW"
  refused_kind_case refused-operator \
    "253; 2617 16503 OPERATOR public + (integer, integer) recipe_migrator" OPERATOR
  refused_kind_case refused-materialized-view-data \
    "254; 0 16502 MATERIALIZED VIEW DATA public planted recipe_migrator" "MATERIALIZED VIEW"
  new_restore_case not-a-refused-kind
  restore_run "STUB_LIST_EXTRA=255; 1259 16504 TABLE public operator_log recipe_migrator"
  expect_eq "a table with a refused word in its name passes: exit code" "$RC" 0
}

# An entry of a checked kind that does not match the grammar must not pass unseen (fail closed).
case_restore_name_lines_outside_the_grammar() {
  local variant=0 line
  for line in "256; 1255 16505 FUNCTION public planted() " "257; 2620 16506 TRIGGER " \
    "258; 3466 16507 EVENT TRIGGER "; do
    variant=$((variant + 1))
    failed_restore_case "name-grammar-$variant" "STUB_LIST_EXTRA=$line"
    expect_eq "name grammar $variant: only the list ran" "$(event_sequence)" "EVENT list,"
    expect_ok "name grammar $variant: the ERROR line says so" \
      error_has "an entry of pg_restore --list does not match the grammar"
  done
}

case_restore_unchanged_code_passes() {
  new_restore_case code-unchanged
  restore_run
  expect_eq "unchanged code: exit code" "$RC" 0
  expect_eq "unchanged code: both comparisons ran" "$(event_count 'code')" 2
  expect_eq "unchanged code: the section data is restored after pre-data" \
    "$(grep -E '^EVENT pg_restore' "$CASE/stub/log" | tr '\n' ',')" \
    "EVENT pg_restore pre-data,EVENT pg_restore data,EVENT pg_restore post-data,"
}
