# check-result.sh: write the result of a scheduled backup check (ADR-0151 (a) to (g)).
# The caller holds the run lock and calls this function after cleanup.
# shellcheck shell=bash

# write_check_result <directory> <name> <result>: replace one result through a temp file.
write_check_result() {
  local directory=$1 name=$2 result=$3 finished_at
  finished_at=$(date -u +%Y-%m-%dT%H:%M:%SZ) || return 1
  printf '{"result":"%s","finished_at":"%s"}\n' "$result" "$finished_at" \
    >"$directory/.tmp-$name" || {
    rm -f -- "$directory/.tmp-$name" || true
    return 1
  }
  chmod 0644 -- "$directory/.tmp-$name" || {
    rm -f -- "$directory/.tmp-$name" || true
    return 1
  }
  mv -T -f -- "$directory/.tmp-$name" "$directory/$name" || {
    rm -f -- "$directory/.tmp-$name" || true
    return 1
  }
}
