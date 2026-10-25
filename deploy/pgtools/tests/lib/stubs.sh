# Sourced by backup-test. It writes the stubs of the external tools to the stub directory.

make_stub_date() {
  cat >"$ROOT/stubs/date" <<'EOF'
#!/usr/bin/env bash
if [ "$*" = "-u +%Y%m%d_%H%M" ] && [ -n "${STUB_TS:-}" ]; then
  echo "$STUB_TS"
else
  exec "$REAL_DATE" "$@"
fi
EOF
}

# The stub of the snapshot session. It answers the lines that backup-once writes.
#   STUB_PSQL_FAIL=1       psql fails at once.
#   STUB_NO_TABLES=1       the count query gives no row.
#   STUB_PSQL_DIE_GEXEC=n  the count query fails, so the session ends with status n at COUNTS_DONE.
#   STUB_PSQL_DIE_END=n    the session ends with status n at COUNTS_DONE, without the answer.
#   STUB_PSQL_EXIT_END=n   the session answers every line, also COUNTS_DONE. It ends with status n
#                          only at the end of its input.
# A dying session reads its input up to COUNTS_DONE first. So the run reaches wait_for and
# does not stop at a write to a closed pipe.
make_stub_psql() {
  cat >"$ROOT/stubs/psql" <<'EOF'
#!/usr/bin/env bash
echo "EVENT psql open" >>"$STUB_LOG"
echo "ARGV psql $*" >>"$STUB_LOG"
if [ -n "${STUB_PSQL_FAIL:-}" ]; then
  echo "EVENT psql failed" >>"$STUB_LOG"
  exit 1
fi
path=""
die=${STUB_PSQL_DIE_END:-}
while IFS= read -r line; do
  case $line in
    '\echo :pg_export_snapshot') echo "00000003-0000001B-1" ;;
    '\echo COUNTS_DONE')
      if [ -n "$die" ]; then
        echo "EVENT psql died" >>"$STUB_LOG"
        exit "$die"
      fi
      echo COUNTS_DONE
      ;;
    '\echo '*) echo "${line#'\echo '}" ;;
    '\o '*)
      # The real psql creates the file when it opens the redirect.
      path=${line#'\o '}; path=${path//\'/}
      : >"$path"
      ;;
    '\o') path="" ;;
    *'\gexec')
      # A database with no table gives no row, so the file stays empty.
      if [ -n "${STUB_PSQL_DIE_GEXEC:-}" ]; then
        die=$STUB_PSQL_DIE_GEXEC
      elif [ -n "$path" ] && [ -z "${STUB_NO_TABLES:-}" ]; then
        printf 'public.machine|3\npublic.recipe|5\n' >"$path"
      fi
      ;;
  esac
done
echo "EVENT psql closed" >>"$STUB_LOG"
exit "${STUB_PSQL_EXIT_END:-0}"
EOF
}

make_stub_pg_dump() {
  cat >"$ROOT/stubs/pg_dump" <<'EOF'
#!/usr/bin/env bash
file=""
for a in "$@"; do
  case $a in --file=*) file=${a#--file=} ;; esac
done
echo "EVENT pg_dump start" >>"$STUB_LOG"
echo "ARGV pg_dump $*" >>"$STUB_LOG"
if [ -n "${PGPASSFILE:-}" ] && [ -f "$PGPASSFILE" ]; then
  cat "$PGPASSFILE" >"$STUB_DIR/pgpass.seen"
  stat -c %a "$PGPASSFILE" >"$STUB_DIR/pgpass.mode"
fi
if grep -q '^EVENT psql closed' "$STUB_LOG"; then
  echo "EVENT SESSION_CLOSED_EARLY" >>"$STUB_LOG"
fi
sleep "${STUB_PG_DUMP_SLEEP:-0}"
case ${STUB_CREATE_FINAL:-} in
  dump) printf 'FOREIGN' >"$BACKUP_DIR/recipe_${STUB_TS}.dump" ;;
  counts) printf 'FOREIGN' >"$BACKUP_DIR/recipe_${STUB_TS}.counts" ;;
esac
if [ -n "${STUB_PG_DUMP_FAIL:-}" ]; then
  printf 'partial' >"$file"
  echo "EVENT pg_dump failed" >>"$STUB_LOG"
  exit 1
fi
printf 'DUMP-%s\n' "$$" >"$file"
echo "EVENT pg_dump done" >>"$STUB_LOG"
EOF
}

make_stub_pg_dumpall() {
  cat >"$ROOT/stubs/pg_dumpall" <<'EOF'
#!/usr/bin/env bash
file=""
for a in "$@"; do
  case $a in --file=*) file=${a#--file=} ;; esac
done
echo "EVENT pg_dumpall start" >>"$STUB_LOG"
echo "ARGV pg_dumpall $*" >>"$STUB_LOG"
if [ -n "${STUB_DUMPALL_FAIL:-}" ]; then
  printf 'partial' >"$file"
  echo "EVENT pg_dumpall failed" >>"$STUB_LOG"
  exit 1
fi
printf 'CREATE ROLE recipe_app;\n' >"$file"
echo "EVENT pg_dumpall done" >>"$STUB_LOG"
EOF
}

make_stubs() {
  mkdir -p "$ROOT/stubs"
  make_stub_date
  make_stub_psql
  make_stub_pg_dump
  make_stub_pg_dumpall
  chmod 0755 "$ROOT/stubs/date" "$ROOT/stubs/psql" "$ROOT/stubs/pg_dump" "$ROOT/stubs/pg_dumpall"
}

# make_extra_stubs: supercronic and sync record each call. The stub of rm fails on a temp file.
#   STUB_RM_REFUSE_FINAL=1  the stub of rm also fails on a published dump (*/recipe_*.dump).
#   STUB_RM_REFUSE_COUNTS=1 the stub of rm also fails on a published counts file (*/recipe_*.counts).
#   STUB_RM_ALLOW_TEMP=1    the stub of rm deletes a temp file, so the run can publish its files.
# Without a variable, the stub of rm fails on a temp file only.
# The stub of sync fails at its n-th call when STUB_SYNC_FAIL_AT=n.
make_extra_stubs() {
  local dir=$ROOT/stubs
  mkdir -p "$ROOT/rmstub"
  cat >"$dir/supercronic" <<'EOF'
#!/usr/bin/env bash
echo "EVENT supercronic" >>"$STUB_LOG"
echo "ARGV supercronic $*" >>"$STUB_LOG"
EOF
  cat >"$dir/sync" <<'EOF'
#!/usr/bin/env bash
# It records which files of the new set and whether an old manifest exist at the time of the call.
# With STUB_SYNC_FAIL_AT=n, the n-th call fails.
t=$BACKUP_DIR/.tmp-recipe_${STUB_TS}
f=$BACKUP_DIR/recipe_${STUB_TS}
tmp_dump=no
dump=no
tmp_manifest=no
manifest=no
old=no
[ -e "$t.dump" ] && tmp_dump=yes
[ -e "$f.dump" ] && dump=yes
[ -e "$t.dump.sha256" ] && tmp_manifest=yes
[ -e "$f.dump.sha256" ] && manifest=yes
[ -n "${STUB_OLD_MANIFEST:-}" ] && [ -e "$STUB_OLD_MANIFEST" ] && old=yes
echo "SYNC tmp_dump=$tmp_dump dump=$dump tmp_manifest=$tmp_manifest manifest=$manifest old=$old" >>"$STUB_LOG"
if [ "$(grep -c '^SYNC ' "$STUB_LOG")" = "${STUB_SYNC_FAIL_AT:-0}" ]; then
  exit 1
fi
exit 0
EOF
  cat >"$ROOT/rmstub/rm" <<'EOF'
#!/usr/bin/env bash
for a in "$@"; do
  case $a in */.tmp-*) [ -n "${STUB_RM_ALLOW_TEMP:-}" ] || exit 1 ;; esac
  case $a in */recipe_*.dump) [ -z "${STUB_RM_REFUSE_FINAL:-}" ] || exit 1 ;; esac
  case $a in */recipe_*.counts) [ -z "${STUB_RM_REFUSE_COUNTS:-}" ] || exit 1 ;; esac
done
exec "$REAL_RM" "$@"
EOF
  chmod 0755 "$dir/supercronic" "$dir/sync" "$ROOT/rmstub/rm"
}
