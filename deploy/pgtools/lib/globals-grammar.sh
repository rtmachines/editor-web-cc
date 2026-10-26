#!/usr/bin/env bash
# ADR-0095: accept only the five role statement forms of pg_dumpall 16.
# Each expression matches a whole line. Each list has a closed set of words.
GLOBALS_ROLE='[a-z_][a-z0-9_]*'
GLOBALS_ATTRIBUTE='(NO)?(SUPERUSER|INHERIT|CREATEROLE|CREATEDB|LOGIN|REPLICATION|BYPASSRLS)'
GLOBALS_GRANT_OPTION='(ADMIN OPTION|INHERIT (TRUE|FALSE)|SET (TRUE|FALSE))'
GLOBALS_SET="^SET (default_transaction_read_only = off|client_encoding = 'UTF8'|standard_conforming_strings = on);$"
GLOBALS_CREATE="^CREATE ROLE $GLOBALS_ROLE;$"
GLOBALS_ALTER="^ALTER ROLE $GLOBALS_ROLE WITH( $GLOBALS_ATTRIBUTE)+;$"
GLOBALS_SETTING="^ALTER ROLE $GLOBALS_ROLE SET (log_error_verbosity|log_parameter_max_length) TO '[a-z0-9]+';$"
GLOBALS_GRANT="^GRANT (audit_owner|pg_read_all_data|pg_read_all_stats) TO $GLOBALS_ROLE( WITH $GLOBALS_GRANT_OPTION(, $GLOBALS_GRANT_OPTION)*)?( GRANTED BY $GLOBALS_ROLE)?;$"
GLOBALS_RESTRICT='^\\restrict ([A-Za-z0-9]+)$'
GLOBALS_UNRESTRICT='^\\unrestrict ([A-Za-z0-9]+)$'
GLOBALS_COMMENT_OR_BLANK='^([[:blank:]]*|--.*)$'

# globals_error <message> <line>: report a fault without the input text or a shell exit.
globals_error() {
  printf 'ERROR: %s at line %d\n' "$1" "$2" >&2
  return 1
}

# check_globals_file <path>: check the pair, backslashes, control bytes, and role statements.
check_globals_file() {
  local LC_ALL=C line key='' phase=0 number=0 prefix='' nul_line=0
  local pair_fault='the globals file has no valid restrict pair'
  local grammar_fault='the globals file holds a statement outside the role grammar'
  [ -r "$1" ] || { globals_error 'cannot read the globals file' 1; return 1; }
  # Bash skips NUL bytes during a line read. Find the first NUL before that read.
  if IFS= read -r -d '' prefix <"$1"; then
    prefix=${prefix//[!$'\n']/}
    nul_line=$((${#prefix} + 1))
  fi
  while IFS= read -r line || [ -n "$line" ] || [ "$number" -lt "$nul_line" ]; do
    number=$((number + 1))
    if [[ $line =~ $GLOBALS_RESTRICT ]] && [ "$number" -ne "$nul_line" ]; then
      [ "$phase" -eq 0 ] || { globals_error "$pair_fault" "$number"; return 1; }
      key=${BASH_REMATCH[1]}
      phase=1
    elif [[ $line =~ $GLOBALS_UNRESTRICT ]] && [ "$number" -ne "$nul_line" ]; then
      { [ "$phase" -eq 1 ] && [ "${BASH_REMATCH[1]}" = "$key" ]; } \
        || { globals_error "$pair_fault" "$number"; return 1; }
      phase=2
    elif [[ $line == *\\* ]]; then
      globals_error 'the globals file holds a psql meta-command' "$number"
      return 1
    elif [[ ${line//$'\t'/} == *[[:cntrl:]]* ]] || [ "$number" -eq "$nul_line" ]; then
      globals_error 'the globals file holds a control character' "$number"
      return 1
    elif [[ ! $line =~ $GLOBALS_COMMENT_OR_BLANK ]]; then
      [ "$phase" -eq 1 ] || { globals_error "$pair_fault" "$number"; return 1; }
      [[ $line =~ $GLOBALS_SET || $line =~ $GLOBALS_CREATE || $line =~ $GLOBALS_ALTER \
        || $line =~ $GLOBALS_SETTING || $line =~ $GLOBALS_GRANT ]] \
        || { globals_error "$grammar_fault" "$number"; return 1; }
    fi
  done <"$1"
  [ "$phase" -eq 2 ] || { globals_error "$pair_fault" "$((number + 1))"; return 1; }
  return 0
}
