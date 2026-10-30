# wal-identity.sh: the system identifier guard of the WAL archive.
#
# Source: ADR-0103 (s), design D7 of the change restore-volume-switch, ADR-0074 (b), ADR-0099 (d).
# The guard keeps the segments of one cluster in one WAL archive. The file /wal/.system-identifier
# holds the identifier of that cluster. A script sources this file after it sets its own constants.
# The file only defines names and functions, and it runs nothing.
#
# The sourcing script sets these names:
#   WAL_DIR                the WAL archive disk
#   PGHOST PGPORT PGUSER   the replication connection of recipe_replicator
#   fail                   the function that writes the one ERROR line and stops
# It calls wal_identity_guard <pgpass file> after it writes the pgpass file.
# shellcheck shell=bash
# shellcheck disable=SC2034

IDENTITY_NAME=.system-identifier
# The name of a segment file and of its partial file (ADR-0099 (d)). The name has three groups
# of 8 upper-case hex characters: the timeline, the log number, and the segment number.
SEGMENT_GRAMMAR='^[0-9A-F]{24}(\.partial)?$'
IDENTIFIER_GRAMMAR='^[0-9]+$'
IDENTITY_MSG_MALFORMED="the file $IDENTITY_NAME is malformed"
IDENTITY_MSG_UNREADABLE="cannot read the file $IDENTITY_NAME"

guard_server_id=""
guard_file_id=""

# identify_detail <file>: the cleaned first line of the file, from tool_detail of restore-steps.sh.
# The file sits next to this file. A subshell loads it, so its log and fail do not replace the
# ones of the sourcing script. DETAIL_LENGTH is restore_test.detail_length of reliability.md 7.11.
identify_detail() {
  (
    DETAIL_LENGTH=160
    # shellcheck source=restore-steps.sh
    source "${BASH_SOURCE[0]%/*}/restore-steps.sh" 2>/dev/null && tool_detail "$1"
  ) || true
}

# read_server_identifier <pgpass file>: sets guard_server_id from IDENTIFY_SYSTEM.
# The answer line is systemid|timeline|xlogpos|dbname. Only the first field counts.
# libpq matches the database field of the pgpass line against dbname. Without dbname, it uses the
# user name. So the connection names dbname=replication, as pg_receivewal and pg_basebackup do.
# The error of psql goes to a file next to the pgpass file, in the private work directory of the
# run. The ERROR line gets its first line as the field detail (ADR-0105 (s)). The error holds no
# password. The file is gone before this function returns.
read_server_identifier() {
  local answer first err_file=${1%/*}/identify.err detail
  if ! answer=$(PGPASSFILE=$1 psql --no-psqlrc --quiet --no-align --tuples-only --no-password \
    --dbname="host=$PGHOST port=$PGPORT user=$PGUSER dbname=replication replication=true application_name=walarchive_identity" \
    --command=IDENTIFY_SYSTEM 2>"$err_file"); then
    detail=$(identify_detail "$err_file")
    rm -f -- "${err_file:?}"
    fail "cannot read the system identifier of the server" "\"detail\":\"$detail\""
  fi
  rm -f -- "${err_file:?}"
  first=${answer%%|*}
  [[ $first =~ $IDENTIFIER_GRAMMAR ]] || fail "cannot read the system identifier of the server"
  guard_server_id=$first
}

# has_segment: one name at the top of the WAL archive matches the segment grammar.
has_segment() {
  local path
  for path in "$WAL_DIR"/*; do
    if [[ ${path##*/} =~ $SEGMENT_GRAMMAR ]]; then
      return 0
    fi
  done
  return 1
}

# read_identifier_file: sets guard_file_id. The file is one line of ASCII digits.
# The checks use counts only. A CR, a NUL byte, or any other byte makes the counts differ.
read_identifier_file() {
  local path=$WAL_DIR/$IDENTITY_NAME total newlines digits first_line
  [ -f "$path" ] && [ ! -L "$path" ] && [ -r "$path" ] || fail "$IDENTITY_MSG_UNREADABLE"
  total=$(wc -c <"$path")
  newlines=$(tr -cd '\n' <"$path" | wc -c)
  digits=$(tr -cd '0-9' <"$path" | wc -c)
  [ "$digits" -gt 0 ] && [ $((digits + newlines)) -eq "$total" ] || fail "$IDENTITY_MSG_MALFORMED"
  [ "$newlines" -le 1 ] || fail "$IDENTITY_MSG_MALFORMED"
  if [ "$newlines" -eq 1 ]; then
    first_line=$(head -n 1 -- "$path" | wc -c)
    [ "$first_line" -eq "$total" ] || fail "$IDENTITY_MSG_MALFORMED"
  fi
  # A file without a line end makes read return 1. The variable still holds the line.
  IFS= read -r guard_file_id <"$path" || true
  [[ $guard_file_id =~ $IDENTIFIER_GRAMMAR ]] || fail "$IDENTITY_MSG_MALFORMED"
}

# compare_identifiers: the file and the server name the same cluster.
compare_identifiers() {
  [ "$guard_file_id" = "$guard_server_id" ] \
    || fail "the file $IDENTITY_NAME holds the identifier of another system" \
      "\"file_identifier\":\"$guard_file_id\",\"server_identifier\":\"$guard_server_id\""
}

# publish_identifier: write the file under a temp name and link it to its final name (ADR-0074 (b)).
# sync runs on the temp file before the link and on the directory after it (ADR-0105 (t)).
# A failed sync removes the temp name and stops the run.
# ln without -f fails when the final name exists. Then a first start at the same time won,
# so this run reads the file again and compares. That path runs no sync, because the other run owns the file.
publish_identifier() {
  local temp final=$WAL_DIR/$IDENTITY_NAME
  temp=$(mktemp "$WAL_DIR/$IDENTITY_NAME.tmp.XXXXXX") || fail "cannot write the file $IDENTITY_NAME"
  if ! printf '%s\n' "$guard_server_id" >"$temp" || ! sync -- "$temp" 2>/dev/null; then
    rm -f -- "${temp:?}"
    fail "cannot write the file $IDENTITY_NAME"
  fi
  if ln -T -- "$temp" "$final" 2>/dev/null; then
    rm -f -- "${temp:?}"
    sync -- "$WAL_DIR" 2>/dev/null || fail "the file $IDENTITY_NAME exists, but the sync of the directory failed"
    return 0
  fi
  rm -f -- "${temp:?}"
  [ -e "$final" ] || fail "cannot write the file $IDENTITY_NAME"
  read_identifier_file
  compare_identifiers
}

# wal_identity_guard <pgpass file>: stop with one ERROR line unless the WAL archive belongs to the
# cluster of the server. A missing file next to a segment is a stop, because the segments may
# come from another cluster. Without a segment, the guard writes the file.
wal_identity_guard() {
  read_server_identifier "$1"
  if [ -e "$WAL_DIR/$IDENTITY_NAME" ] || [ -L "$WAL_DIR/$IDENTITY_NAME" ]; then
    read_identifier_file
    compare_identifiers
    return 0
  fi
  if has_segment; then
    fail "segments are in the WAL archive and the file $IDENTITY_NAME is missing"
  fi
  publish_identifier
}
