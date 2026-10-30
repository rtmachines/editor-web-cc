#!/usr/bin/env bash
# Shared client rules of the database cluster (ADR-0103 (l) and (m), design D1).
#
# Source: data-model.md section 5.1, ADR-0032, and ADR-0069 (r) to (t).
# The user sources this file in bash. The file only defines functions and runs no command.
# It sets no shell option, so the option set of the caller stays as it is.
# The database image mounts it at /etc/recipe-db/cluster-access.sh, outside
# /docker-entrypoint-initdb.d, so the entrypoint never runs it.
# The image recipe-pgtools copies it to /etc/pgtools/cluster-access.sh.

# Check that a value is an IPv4 address with a prefix length, for example 172.28.0.0/16.
# The function writes no message. It returns 1 for a bad value. The caller reports the fault.
check_compose_subnet() {
  local value=${1-} octet
  local byte='(0|[1-9][0-9]{0,2})'
  local prefix='([1-9]|[12][0-9]|3[0-2])'
  [[ $value =~ ^${byte}\.${byte}\.${byte}\.${byte}/${prefix}$ ]] || return 1
  for octet in "${BASH_REMATCH[@]:1:4}"; do
    (( octet <= 255 )) || return 1
  done
}

# Write the seven client rules of data-model.md section 5.1 to a file.
# Arguments: the Compose subnet, and the path of the pg_hba.conf file.
#
# The first matching line wins. Only a process of the system user postgres logs in through the
# socket (method peer). The four login roles connect from the Compose subnet only. The probe line
# serves the healthcheck of db: no role pg_isready exists, so the probe ends at the password
# request (ADR-0070 (q)).
# The function writes a temporary file and replaces the target in one rename.
# The rename runs only after every write succeeds, so a failed write never publishes a part of the file.
# The writes join with && because a group returns only the status of its last command.
# The subshell gives the temporary file mode 0600 and leaves the umask of the caller alone.
write_hba_conf() {
  local subnet=${1:?write_hba_conf needs the Compose subnet}
  local hba_file=${2:?write_hba_conf needs the path of the file}
  (
    umask 077
    {
      printf '# Written by 01-roles.sh (data-model.md section 5.1, ADR-0069).\n' &&
        printf '# TYPE  DATABASE     USER               ADDRESS          METHOD\n' &&
        printf 'local   all          postgres                            peer\n' &&
        printf 'host    recipe       recipe_migrator    %-16s scram-sha-256\n' "$subnet" &&
        printf 'host    recipe       recipe_app         %-16s scram-sha-256\n' "$subnet" &&
        printf 'host    recipe       recipe_backup      %-16s scram-sha-256\n' "$subnet" &&
        printf 'host    replication  recipe_replicator  %-16s scram-sha-256\n' "$subnet" &&
        printf 'host    recipe       pg_isready         127.0.0.1/32     scram-sha-256\n' &&
        printf 'host    all          all                all              reject\n'
    } > "${hba_file}.new" &&
      mv -- "${hba_file}.new" "$hba_file"
  )
}

# Print the two statements of ADR-0032: a normal session of a login role reaches no default
# database. The caller pipes the output to psql.
print_default_db_revokes() {
  printf 'REVOKE CONNECT, TEMPORARY ON DATABASE postgres FROM PUBLIC;\n'
  printf 'REVOKE CONNECT, TEMPORARY ON DATABASE template1 FROM PUBLIC;\n'
}
