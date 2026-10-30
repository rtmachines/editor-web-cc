# restore-steps.sh: the steps that restore-test and restore-volume share.
#
# Source: ADR-0067 steps 1, 2, and 4 to 7, ADR-0103 (j), design D3 of the change restore-volume-switch.
# A script sources this file after it sets its own constants. The file only defines grammars and
# functions, and it runs nothing. The file holds no value of reliability.md section 7.11.
#
# The sourcing script sets these names:
#   JOB, DETAIL_LENGTH                  the job name of the log line and the length of a tool message
#   BACKUP_DIR, CODE_DIR, DB_NAME       the backup disk, the code files, and the restored database
#   work_dir, socket_dir, superuser     the private work directory and the temporary server
#   error_logged                        the flag that fail and log ERROR set
# The sourcing script may set SCRATCH_PORT. An empty or unset value gives the default port of the
# socket name, and the library then passes no port to a tool.
#   set_stamp, base, manifest_name, dump_path, globals_path, globals_copy, counts_path, table_count
#                                       the state that select_set, check_manifest, and check_counts fill
# shellcheck shell=bash
# shellcheck disable=SC2034

SET_GRAMMAR='^recipe_([0-9]{8}_[0-9]{4})(_premigrate|_now)?\.dump\.sha256$'
# An entry of pg_restore --list is "<dump id>; <class oid> <object oid> <kind> <name> <owner>".
# The kinds of ADR-0076 (b) have a name list. The kinds of ADR-0078 (b) stop the test.
CHECKED_PREFIX='^[0-9]+; [0-9]+ [0-9]+ (FUNCTION|TRIGGER|EVENT TRIGGER) '
ENTRY_GRAMMAR='^[0-9]+; [0-9]+ [0-9]+ (FUNCTION|TRIGGER|EVENT TRIGGER) (.+) [^ ]+$'
REFUSED_GRAMMAR='^[0-9]+; [0-9]+ [0-9]+ (PROCEDURE|AGGREGATE|MATERIALIZED VIEW|OPERATOR) '
COUNTS_GRAMMAR='^[^|]+\|[0-9]+$'
# A role name or a database name has this grammar. Both enter SQL text, so the name is checked first.
ROLE_GRAMMAR='^[a-z_][a-z0-9_]*$'
# The kinds of restore-code.expected that pre-data holds, and every kind (ADR-0076 (c)).
PRE_DATA_KINDS='^(FUNCTION|CHECK|GENERATED)\|'
ALL_KINDS='^.'

# log <level> <message> [<extra JSON fields>]. Every value is a fixed text or a checked name.
log() {
  if [ "$1" = ERROR ]; then
    error_logged=1
  fi
  printf '{"timestamp":"%s","level":"%s","job":"%s","message":"%s"%s}\n' \
    "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" "$JOB" "$2" "${3:+,$3}"
}

# fail <message> [<extra JSON fields>]: write the one ERROR line and stop.
fail() {
  log ERROR "$@"
  exit 1
}

# clean <text>: the text with each character outside a short safe set replaced by a question mark.
# A name from a dump is external input, so it never enters a log line unchecked.
clean() {
  local text=${1:0:$DETAIL_LENGTH}
  printf '%s' "$text" | tr -c 'A-Za-z0-9_ ().,:|=-' '?'
}

# tool_detail <file>: the first line of the message of a tool, cleaned.
tool_detail() {
  local line=""
  line=$(sed -n '1p' -- "$1") || line=""
  clean "$line"
}

# random_secret <file>: 24 random bytes as hex text in a new file. The value never enters argv.
random_secret() {
  od -An -N24 -tx1 /dev/urandom | tr -d ' \n' >"$1"
}

# set_paths <stamp>: the names and paths of the files of the set with that stamp. The stamp has the
# timestamp and the suffix. The caller checked the stamp against its grammar.
set_paths() {
  set_stamp=$1
  base=recipe_$set_stamp
  manifest_name=$base.dump.sha256
  dump_path=$BACKUP_DIR/$base.dump
  globals_path=$BACKUP_DIR/globals_$set_stamp.sql
  counts_path=$BACKUP_DIR/$base.counts
}

# select_set: the newest manifest by the anchored grammar of ADR-0074 (c). Only a regular file counts.
select_set() {
  local manifest name
  for manifest in "$BACKUP_DIR"/recipe_*.dump.sha256; do
    name=${manifest##*/}
    [[ $name =~ $SET_GRAMMAR ]] || continue
    if [ -f "$manifest" ] && [ ! -L "$manifest" ]; then
      set_stamp=${BASH_REMATCH[1]}${BASH_REMATCH[2]-}
    fi
  done
  [ -n "$set_stamp" ] || fail "no complete backup set found"
  set_paths "$set_stamp"
}

# check_manifest: the manifest lists exactly the three files of the set, and each hash is right.
check_manifest() {
  local listed expected file globals_hash copy_hash
  for file in "$dump_path" "$globals_path" "$counts_path"; do
    if [ ! -f "$file" ] || [ -L "$file" ]; then
      fail "the newest set lacks a regular file" "\"set\":\"$base\""
    fi
  done
  listed=$(sed -E -e 's/^[0-9a-f]{64} [ *]//' -- "$BACKUP_DIR/$manifest_name" | sort) \
    || fail "cannot read the manifest of the newest set"
  expected=$(printf '%s\n' "${dump_path##*/}" "${globals_path##*/}" "${counts_path##*/}" | sort)
  [ "$listed" = "$expected" ] || fail "the manifest does not list exactly the files of the set"
  globals_copy=$work_dir/${globals_path##*/}
  cat -- "$globals_path" >"$globals_copy" || fail "cannot copy the globals file"
  chmod 0600 -- "$globals_copy" || fail "cannot set the globals copy permissions"
  globals_hash=$(awk -v name="${globals_path##*/}" \
    'substr($0, 67) == name { print substr($0, 1, 64) }' "$BACKUP_DIR/$manifest_name") \
    || fail "cannot read the manifest of the newest set"
  if ! (cd "$BACKUP_DIR" && sha256sum -c --strict -- "$manifest_name") \
    >"$work_dir/manifest.out" 2>&1; then
    cat -- "$work_dir/manifest.out" >&2
    fail "the manifest of the newest set does not verify" "\"set\":\"$base\""
  fi
  copy_hash=$(sha256sum -- "$globals_copy") || fail "cannot hash the globals copy"
  [ "${copy_hash%% *}" = "$globals_hash" ] || fail "the globals copy does not match the manifest"
}

# check_counts: the counts file is external input. Each line has the grammar. The loop counts the
# tables, so the last line counts also without its newline.
check_counts() {
  local line
  table_count=0
  while IFS= read -r line || [ -n "$line" ]; do
    [[ $line =~ $COUNTS_GRAMMAR ]] || fail "the counts file has a bad line"
    table_count=$((table_count + 1))
  done <"$counts_path"
}

# check_globals: the globals file is external input, and psql runs its SQL as the scratch
# superuser. This gate refuses each psql meta-command: every line with a backslash at any place.
# It accepts one pair \restrict KEY and \unrestrict KEY of pg_dumpall (pg_dumpall.c, REL_16_STABLE).
# The pair has the same key. \restrict is the first line that is no comment and no blank line.
# \unrestrict is the last such line, and the SQL stands between them.
# The backslash branch precedes the grammar, so each meta-command keeps its error message.
# The control-byte check precedes comments, also between the pair. Only the role grammar permits SQL.
check_globals() {
  local report
  # shellcheck source=../lib/globals-grammar.sh
  source "${RESTORE_GLOBALS_DIR:-/etc/pgtools}/globals-grammar.sh" \
    || fail "cannot load the globals grammar"
  if ! report=$(check_globals_file "$globals_copy" 2>&1); then
    fail "$(clean "${report#ERROR: }")"
  fi
}

# check_names: step 2. Each function, trigger, and event trigger of the dump is in the file.
# A line of a checked kind that does not match the grammar stops the test (fail closed).
check_names() {
  local line entry kind
  awk -F'|' '$1 == "FUNCTION" || $1 == "TRIGGER" || $1 == "EVENT TRIGGER" { print $1 "|" $2 }' \
    "$CODE_DIR/restore-code.expected" | sort -u >"$work_dir/names.expected"
  pg_restore --list "$dump_path" >"$work_dir/listing" 2>"$work_dir/list.err" \
    || fail "pg_restore cannot list the dump" "\"detail\":\"$(tool_detail "$work_dir/list.err")\""
  while IFS= read -r line; do
    if [[ $line =~ $REFUSED_GRAMMAR ]]; then
      fail "the dump holds an entry of a kind that the restore test refuses" \
        "\"kind\":\"${BASH_REMATCH[1]}\""
    fi
    [[ $line =~ $CHECKED_PREFIX ]] || continue
    kind=${BASH_REMATCH[1]}
    [[ $line =~ $ENTRY_GRAMMAR ]] \
      || fail "an entry of pg_restore --list does not match the grammar" "\"kind\":\"$kind\""
    entry="${BASH_REMATCH[1]}|${BASH_REMATCH[2]}"
    if ! grep -qxF -- "$entry" "$work_dir/names.expected"; then
      fail "the dump holds an entry that restore-code.expected does not hold" \
        "\"entry\":\"$(clean "$entry")\""
    fi
  done <"$work_dir/listing"
}

# scratch_psql <arguments>: psql as the system user over the private socket of the temporary server.
scratch_psql() {
  psql --no-psqlrc --quiet --no-align --tuples-only --no-password -v ON_ERROR_STOP=1 \
    --host="$socket_dir" ${SCRATCH_PORT:+"--port=$SCRATCH_PORT"} --username="$superuser" "$@"
}

# restore_globals: the globals file gives one known error. The system user exists already.
restore_globals() {
  local line
  scratch_psql --dbname=postgres --file="$globals_copy" -v ON_ERROR_STOP=0 \
    >/dev/null 2>"$work_dir/globals.err" || fail "psql cannot read the globals file"
  while IFS= read -r line; do
    case $line in
      *"ERROR:  role \"$superuser\" already exists") ;;
      *ERROR:*) fail "the globals file gave an error that is not known" \
        "\"detail\":\"$(clean "${line#*ERROR:}")\"" ;;
    esac
  done <"$work_dir/globals.err"
}

# check_database: the restore created the database that the script names.
check_database() {
  local found
  found=$(scratch_psql --dbname=postgres --command="SELECT count(*) FROM pg_catalog.pg_database WHERE datname = '$DB_NAME'") \
    || fail "cannot read the list of databases"
  [ "$found" = 1 ] || fail "the dump did not create the database of the expected name"
}

# restore_section <section>: one pg_restore call. The first call creates the database of the dump.
restore_section() {
  local target=$DB_NAME create=()
  if [ "$1" = pre-data ]; then
    target=postgres
    create=(--create)
  fi
  pg_restore --exit-on-error --no-password --host="$socket_dir" ${SCRATCH_PORT:+"--port=$SCRATCH_PORT"} \
    --username="$superuser" --section="$1" ${create[@]+"${create[@]}"} --dbname="$target" \
    "$dump_path" >/dev/null 2>"$work_dir/restore.err" \
    || fail "pg_restore failed" "\"section\":\"$1\",\"detail\":\"$(tool_detail "$work_dir/restore.err")\""
}

# sorted_kinds <regex> <file> <out>: the lines of the file that match, in byte order.
sorted_kinds() {
  { grep -E -- "$1" "$2" || true; } | sort >"$3"
}

# compare_code <stage> <regex of the kinds>: restore-code.sql against restore-code.expected.
compare_code() {
  local missing extra first
  scratch_psql --dbname="$DB_NAME" --file="$CODE_DIR/restore-code.sql" >"$work_dir/code.actual" \
    2>"$work_dir/code.err" || fail "restore-code.sql failed" "\"detail\":\"$(tool_detail "$work_dir/code.err")\""
  sorted_kinds "$2" "$CODE_DIR/restore-code.expected" "$work_dir/code.want"
  sorted_kinds "$2" "$work_dir/code.actual" "$work_dir/code.have"
  comm -23 "$work_dir/code.want" "$work_dir/code.have" >"$work_dir/code.missing"
  comm -13 "$work_dir/code.want" "$work_dir/code.have" >"$work_dir/code.extra"
  missing=$(($(wc -l <"$work_dir/code.missing")))
  extra=$(($(wc -l <"$work_dir/code.extra")))
  if [ "$missing" -ne 0 ] || [ "$extra" -ne 0 ]; then
    first=$(cat -- "$work_dir/code.missing" "$work_dir/code.extra" | sed -n '1p' | cut -d'|' -f1,2)
    fail "the code after the restore of $1 differs from restore-code.expected" \
      "\"missing\":$missing,\"extra\":$extra,\"first\":\"$(clean "$first")\""
  fi
}

COUNT_QUERY=""
IFS= read -r -d '' COUNT_QUERY <<'SQL' || true
SELECT format('SELECT %L, count(*) FROM %I.%I', n.nspname || '.' || c.relname, n.nspname, c.relname)
FROM pg_catalog.pg_class AS c
JOIN pg_catalog.pg_namespace AS n ON n.oid = c.relnamespace
WHERE c.relkind = 'r'
  AND n.nspname <> 'information_schema'
  AND n.nspname !~ '^pg_'
  AND NOT (n.nspname = 'public' AND c.relname IN ('session', 'login_attempt'))
ORDER BY n.nspname, c.relname \gexec
SQL

# fail_first_table <message> <file>: the ERROR line names the table of the first line of the file.
fail_first_table() {
  local line
  line=$(sed -n '1p' -- "$2")
  fail "$1" "\"table\":\"$(clean "${line%%|*}")\""
}

# compare_counts: step 6. The tables and the row counts equal the counts file in both directions.
compare_counts() {
  scratch_psql --dbname="$DB_NAME" <<<"$COUNT_QUERY" >"$work_dir/counts.actual" \
    || fail "the count query failed on the restored database"
  sort -- "$counts_path" >"$work_dir/counts.want"
  sort -- "$work_dir/counts.actual" >"$work_dir/counts.have"
  cut -d'|' -f1 "$work_dir/counts.want" | sort -u >"$work_dir/tables.want"
  cut -d'|' -f1 "$work_dir/counts.have" | sort -u >"$work_dir/tables.have"
  comm -23 "$work_dir/tables.want" "$work_dir/tables.have" >"$work_dir/tables.missing"
  comm -13 "$work_dir/tables.want" "$work_dir/tables.have" >"$work_dir/tables.extra"
  comm -23 "$work_dir/counts.want" "$work_dir/counts.have" >"$work_dir/counts.differ"
  if [ -s "$work_dir/tables.missing" ]; then
    fail_first_table "a table of the counts file is missing in the restore" "$work_dir/tables.missing"
  fi
  if [ -s "$work_dir/tables.extra" ]; then
    fail_first_table "a restored table is not in the counts file" "$work_dir/tables.extra"
  fi
  if [ -s "$work_dir/counts.differ" ]; then
    fail_first_table "the row count of a table differs from the counts file" "$work_dir/counts.differ"
  fi
}

# The audit chain check of data-model.md section 4.9 over the whole table: breaks, head, last row.
# The text equals the text of pitr-check.
CHAIN_SQL=""
IFS= read -r -d '' CHAIN_SQL <<'SQL' || true
WITH batch AS (
    SELECT a.chain_seq, a.prev_hash, a.entry_hash,
           pg_catalog.sha256(COALESCE(a.prev_hash, '\x'::bytea)
               || pg_catalog.convert_to(public.audit_canonical(a), 'UTF8')) AS recomputed
    FROM public.audit_log AS a
), chain AS (
    SELECT b.*,
           lag(b.chain_seq, 1, 0::bigint) OVER w AS previous_seq,
           lag(b.entry_hash, 1, NULL::bytea) OVER w AS previous_hash
    FROM batch AS b
    WINDOW w AS (ORDER BY b.chain_seq)
)
SELECT (SELECT count(*) FROM chain
        WHERE chain_seq IS DISTINCT FROM previous_seq + 1
           OR prev_hash IS DISTINCT FROM previous_hash
           OR entry_hash IS DISTINCT FROM recomputed) AS breaks,
       h.last_seq = COALESCE((SELECT max(chain_seq) FROM public.audit_log), 0) AS head_matches,
       h.last_hash IS NOT DISTINCT FROM
           (SELECT entry_hash FROM public.audit_log ORDER BY chain_seq DESC LIMIT 1) AS hash_matches
FROM public.audit_chain_head AS h
WHERE h.singleton
SQL

# check_chain: step 7. A database with no table has no audit_log table and nothing to check.
check_chain() {
  local exists result
  exists=$(scratch_psql --dbname="$DB_NAME" --command="SELECT to_regclass('public.audit_log') IS NOT NULL") \
    || fail "cannot look for the audit_log table"
  if [ "$exists" != t ]; then
    [ "$table_count" -eq 0 ] || fail "the restored database has tables but no audit_log table"
    log INFO "the database has no table, the audit chain check is skipped"
    return 0
  fi
  result=$(scratch_psql --dbname="$DB_NAME" --command="$CHAIN_SQL") \
    || fail "the audit chain check could not run"
  [ "$result" = "0|t|t" ] || fail "the audit chain check failed" "\"result\":\"$(clean "$result")\""
}
