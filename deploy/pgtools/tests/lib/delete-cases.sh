# Sourced by backup-test. It holds the cases of backup-once in which a delete of cleanup fails.

# delete_fails_checks <label> <temp files that stay>: the rm stub refuses each temp file.
delete_fails_checks() {
  local left refused
  left=$(cd "$B" && find . -maxdepth 1 -name '.tmp-*' -printf '%f\n' | LC_ALL=C sort | tr '\n' ',')
  refused=$(grep '"message":"cannot delete a file of this run"' "$CASE/out" \
    | sed -e 's/.*"file":"\([^"]*\)".*/\1/' | LC_ALL=C sort | tr '\n' ',')
  expect_eq "$1: exit code" "$RC" 1
  expect_eq "$1: ERROR lines name the files that stay" "$refused" "$left"
  expect_eq "$1: only the refused temp files stay" "$left" "$2"
  expect_eq "$1: no published file stays" \
    "$(find "$B" -maxdepth 1 -name "*_$TS*" ! -name '.tmp-*' | wc -l)" 0
  expect_eq "$1: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# After a failed dump, cleanup must try every delete.
case_cleanup_delete_fails() {
  new_case cleanup-delete-fails
  run_script "PATH=$ROOT/rmstub:$ROOT/stubs:$PATH" STUB_PG_DUMP_FAIL=1 -- backup-once ""
  delete_fails_checks "cleanup delete" ".tmp-recipe_$TS.counts,.tmp-recipe_$TS.dump,"
  expect_ok "cleanup delete: pg_dump ERROR line" grep -q '"message":"pg_dump failed"' "$CASE/out"
  expect_eq "cleanup delete: three ERROR lines in total" "$(error_count "$CASE/out")" 3
  expect_no "cleanup delete: no fallback line" grep -q 'backup run failed unexpectedly' "$CASE/out"
}

# The rm of the temp name in publish fails and set -e stops the run. The fallback line must stay.
case_publish_delete_fails() {
  new_case publish-delete-fails
  run_script "PATH=$ROOT/rmstub:$ROOT/stubs:$PATH" -- backup-once ""
  delete_fails_checks "publish delete" ".tmp-globals_$TS.sql,.tmp-recipe_$TS.counts,.tmp-recipe_$TS.dump,"
  expect_eq "publish delete: one fallback ERROR line with the exit code" \
    "$(grep -c '"message":"backup run failed unexpectedly","exit_code":1' "$CASE/out" || true)" 1
  expect_eq "publish delete: four ERROR lines in total" "$(error_count "$CASE/out")" 4
}

# The run stops in the first publish, after the dump has a final name and before the manifest.
# The rm stub refuses the temp names and the final dump. The second loop of cleanup must try
# the final dump, write its ERROR line, and go on. The fallback line names the cause.
case_published_delete_fails() {
  new_case published-delete-fails
  run_script "PATH=$ROOT/rmstub:$ROOT/stubs:$PATH" STUB_RM_REFUSE_FINAL=1 -- backup-once ""
  local dump=recipe_$TS.dump listed
  listed=$(cd "$B" && find . -maxdepth 1 -name "*_$TS*" ! -name '.tmp-*' -printf '%f\n' \
    | LC_ALL=C sort | tr '\n' ',')
  expect_eq "published delete: exit code" "$RC" 1
  expect_eq "published delete: one fallback ERROR line with the exit code" \
    "$(grep -c '"message":"backup run failed unexpectedly","exit_code":1' "$CASE/out" || true)" 1
  expect_eq "published delete: one ERROR line for the final dump" \
    "$(grep -c "\"message\":\"cannot delete a file of this run\",\"file\":\"$dump\"" "$CASE/out" || true)" 1
  expect_eq "published delete: five ERROR lines in total" "$(error_count "$CASE/out")" 5
  expect_eq "published delete: only the refused dump is left as a final name" "$listed" "$dump,"
  expect_ok "published delete: the refused dump is not empty" test -s "$B/$dump"
  expect_eq "published delete: only the refused temp files stay" \
    "$(cd "$B" && find . -maxdepth 1 -name '.tmp-*' -printf '%f\n' | LC_ALL=C sort | tr '\n' ',')" \
    ".tmp-globals_$TS.sql,.tmp-recipe_$TS.counts,.tmp-recipe_$TS.dump,"
  expect_no "published delete: no manifest" test -e "$B/recipe_$TS.dump.sha256"
  expect_eq "published delete: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}

# The run publishes the dump, the globals file, and the counts file. The second sync fails, so the
# manifest never comes. The rm stub refuses the dump (first) and the counts file (last). The second
# loop of cleanup must not stop at the first refusal (VW2-7). It must delete the globals file and
# write one ERROR line for each refused file.
case_published_delete_continues() {
  new_case published-delete-continues
  run_script "PATH=$ROOT/rmstub:$ROOT/stubs:$PATH" STUB_RM_ALLOW_TEMP=1 STUB_RM_REFUSE_FINAL=1 \
    STUB_RM_REFUSE_COUNTS=1 STUB_SYNC_FAIL_AT=2 -- backup-once ""
  local dump=recipe_$TS.dump counts=recipe_$TS.counts listed
  listed=$(cd "$B" && find . -maxdepth 1 -name "*_$TS*" -printf '%f\n' | LC_ALL=C sort | tr '\n' ',')
  expect_eq "published continues: exit code" "$RC" 1
  expect_eq "published continues: sync ran twice" "$(grep -c '^SYNC ' "$CASE/stub/log")" 2
  expect_eq "published continues: only the refused files stay" "$listed" "$counts,$dump,"
  expect_no "published continues: the later file is deleted" test -e "$B/globals_$TS.sql"
  expect_eq "published continues: one ERROR line for the dump" \
    "$(grep -c "\"message\":\"cannot delete a file of this run\",\"file\":\"$dump\"" "$CASE/out" || true)" 1
  expect_eq "published continues: one ERROR line for the counts file" \
    "$(grep -c "\"message\":\"cannot delete a file of this run\",\"file\":\"$counts\"" "$CASE/out" || true)" 1
  expect_eq "published continues: one fallback ERROR line with the exit code" \
    "$(grep -c '"message":"backup run failed unexpectedly","exit_code":1' "$CASE/out" || true)" 1
  expect_eq "published continues: three ERROR lines in total" "$(error_count "$CASE/out")" 3
  expect_no "published continues: no manifest" test -e "$B/recipe_$TS.dump.sha256"
  expect_eq "published continues: pgpass directory is empty" "$(find "$CASE/pgpass" -mindepth 1 | wc -l)" 0
}
