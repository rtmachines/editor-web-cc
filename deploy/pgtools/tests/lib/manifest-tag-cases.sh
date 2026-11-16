# Sourced by backup-test after ROOT exists. It tests ADR-0174 (j), (k), (n), (o), (p), and (r).
# shellcheck shell=bash

manifest_tag_fixture() {
  new_case "manifest-tag-$1"
  perl -e 'print unpack("H*", pack("C*", 0..31)), "\n"' >"$CASE/key-first"
  perl -e 'print unpack("H*", pack("C*", 32..63)), "\n"' >"$CASE/key-second"
  cat "$CASE/key-first" "$CASE/key-second" >"$CASE/keys"
  printf '%064d  recipe_20261115_0200.dump\n' 0 >"$CASE/manifest"
  printf '%s  globals_20261115_0200.sql\n' "$(printf '1%.0s' {1..64})" >>"$CASE/manifest"
  printf '%s  recipe_20261115_0200.counts\n' "$(printf '2%.0s' {1..64})" >>"$CASE/manifest"
  printf 'hmac-sha256-v1 e60535bd89336ac1 %s\n' \
    e047af28abfb9ccdd1867c6813ea85e66e50bd453b35726b79a1cccee8e6dd4d >"$CASE/tag"
}

# Run the mounted source, because the image gets the helper in task 2.5.
manifest_tag_run() {
  perl "$HERE/../lib/manifest-tag.pl" "$@" >"$CASE/out" 2>"$CASE/err"
  RC=$?
  manifest_tag_no_key_output
}

# Check the encoded and decoded fixture keys without putting either key in shell variables or output.
manifest_tag_no_key_output() {
  expect_ok "manifest tag: no key in stdout or stderr" perl -e '
    local $/;
    my @keys;
    for my $path (splice @ARGV, 0, 2) {
      open my $file, "<:raw", $path or exit 1;
      my $hex = <$file>;
      $hex =~ s/\n\z//;
      push @keys, $hex, pack("H*", $hex);
    }
    for my $path (@ARGV) {
      open my $file, "<:raw", $path or exit 1;
      my $output = <$file> // "";
      for my $key (@keys) { exit 1 if index($output, $key) >= 0; }
    }
  ' "$CASE/key-first" "$CASE/key-second" "$CASE/out" "$CASE/err"
}

manifest_tag_fault() {
  expect_no "$1: nonzero exit code" test "$RC" -eq 0
  printf '%s\n' "$2" >"$CASE/expected-error"
  expect_ok "$1: fixed fault line" cmp -s "$CASE/err" "$CASE/expected-error"
  expect_no "$1: no stdout" test -s "$CASE/out"
}

case_manifest_tag_vector() {
  manifest_tag_fixture vector
  expect_eq "manifest tag vector: manifest bytes" "$(wc -c <"$CASE/manifest")" 278
  manifest_tag_run sign "$CASE/key-first" "$CASE/manifest"
  expect_eq "manifest tag vector: exit code" "$RC" 0
  expect_ok "manifest tag vector: exact tag line and key id" cmp -s "$CASE/out" "$CASE/tag"
  expect_no "manifest tag vector: no stderr" test -s "$CASE/err"
}

case_manifest_tag_round_trip() {
  manifest_tag_fixture round-trip
  printf 'another manifest\000with a binary byte\n' >"$CASE/manifest"
  manifest_tag_run sign "$CASE/key-first" "$CASE/manifest"
  expect_eq "manifest tag round trip: sign exit code" "$RC" 0
  cp "$CASE/out" "$CASE/tag"
  manifest_tag_run check "$CASE/key-first" "$CASE/manifest" "$CASE/tag"
  expect_eq "manifest tag round trip: check exit code" "$RC" 0
  expect_no "manifest tag round trip: no stdout" test -s "$CASE/out"
  expect_no "manifest tag round trip: no stderr" test -s "$CASE/err"
}

case_manifest_tag_changed_byte() {
  manifest_tag_fixture changed-byte
  perl -pi -e 's/\A0/3/' "$CASE/manifest"
  manifest_tag_run check "$CASE/key-first" "$CASE/manifest" "$CASE/tag"
  manifest_tag_fault "manifest tag changed byte" tag-mismatch
}

case_manifest_tag_unknown_key() {
  manifest_tag_fixture unknown-key
  sed -i 's/e60535bd89336ac1/0000000000000000/' "$CASE/tag"
  manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/tag"
  manifest_tag_fault "manifest tag unknown key" unknown-key
}

case_manifest_tag_malformed() {
  local form
  for form in version short-tag upper-case extra-field extra-line no-line-feed; do
    manifest_tag_fixture "malformed-$form"
    case "$form" in
      version) sed -i 's/hmac-sha256-v1/hmac-sha256-v2/' "$CASE/tag" ;;
      short-tag) sed -i 's/d4d$/d4/' "$CASE/tag" ;;
      upper-case) sed -i 's/e047/E047/' "$CASE/tag" ;;
      extra-field) sed -i 's/$/ extra/' "$CASE/tag" ;;
      extra-line) printf '\n' >>"$CASE/tag" ;;
      no-line-feed) truncate -s -1 "$CASE/tag" ;;
    esac
    manifest_tag_run check "$CASE/key-first" "$CASE/manifest" "$CASE/tag"
    manifest_tag_fault "manifest tag malformed $form" tag-format
  done
}

case_manifest_tag_first_key() {
  manifest_tag_fixture first-key
  manifest_tag_run sign "$CASE/keys" "$CASE/manifest"
  expect_eq "manifest tag first key: exit code" "$RC" 0
  expect_ok "manifest tag first key: first line signs" cmp -s "$CASE/out" "$CASE/tag"
}

case_manifest_tag_second_key() {
  manifest_tag_fixture second-key
  cat "$CASE/key-second" "$CASE/key-first" >"$CASE/keys"
  manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/tag"
  expect_eq "manifest tag second key: check exit code" "$RC" 0
  expect_no "manifest tag second key: no stdout" test -s "$CASE/out"
  expect_no "manifest tag second key: no stderr" test -s "$CASE/err"
}

case_manifest_tag_key_ids() {
  local form ending
  for form in one two reversed; do
    for ending in line-feed no-line-feed; do
      manifest_tag_fixture "key-ids-$form-$ending"
      printf '%s\n' e60535bd89336ac1 >"$CASE/expected-ids"
      case "$form" in
        one) cp "$CASE/key-first" "$CASE/keys" ;;
        two) printf '%s\n' fc23825e7b842834 >>"$CASE/expected-ids" ;;
        reversed)
          cat "$CASE/key-second" "$CASE/key-first" >"$CASE/keys"
          printf '%s\n' fc23825e7b842834 e60535bd89336ac1 >"$CASE/expected-ids"
          ;;
      esac
      if [ "$ending" = no-line-feed ]; then truncate -s -1 "$CASE/keys"; fi
      manifest_tag_run key-ids "$CASE/keys"
      expect_eq "manifest key ids $form $ending: exit code" "$RC" 0
      expect_ok "manifest key ids $form $ending: exact lines" cmp -s "$CASE/out" "$CASE/expected-ids"
      expect_no "manifest key ids $form $ending: no stderr" test -s "$CASE/err"
    done
  done
}

case_manifest_tag_key_ids_usage() {
  manifest_tag_fixture key-ids-usage
  manifest_tag_run key-ids
  manifest_tag_fault "manifest key ids: missing key path" usage
  manifest_tag_run key-ids "$CASE/key-first" "$CASE/manifest"
  manifest_tag_fault "manifest key ids: extra argument" usage
  manifest_tag_run key-ids "$CASE/key-first" "$CASE/manifest" "$CASE/tag"
  manifest_tag_fault "manifest key ids: two extra arguments" usage
}

case_manifest_tag_output_write() {
  local mode args
  for mode in sign key-ids; do
    manifest_tag_fixture "output-write-$mode"
    args=("$CASE/keys")
    if [ "$mode" = sign ]; then args+=("$CASE/manifest"); fi
    : >"$CASE/out"
    perl "$HERE/../lib/manifest-tag.pl" "$mode" "${args[@]}" >/dev/full 2>"$CASE/err"
    RC=$?
    manifest_tag_fault "manifest tag $mode: full output device" output-write
    manifest_tag_no_key_output
  done
}

case_manifest_tag_key_ids_wrapper() {
  manifest_tag_fixture key-ids-wrapper
  printf '%s\n' e60535bd89336ac1 fc23825e7b842834 >"$CASE/expected-ids"
  bash -c 'source "$1" && manifest_tag_key_ids "$2"' bash \
    "$HERE/../lib/manifest-tag.sh" "$CASE/keys" >"$CASE/out" 2>"$CASE/err"
  expect_eq "manifest key ids wrapper: exit code" "$?" 0
  expect_ok "manifest key ids wrapper: exact lines" cmp -s "$CASE/out" "$CASE/expected-ids"
  expect_no "manifest key ids wrapper: no stderr" test -s "$CASE/err"
  manifest_tag_no_key_output
  bash -c 'source "$1" && manifest_tag_key_ids "$2"' bash \
    "$HERE/../lib/manifest-tag.sh" "$CASE/missing-key" >"$CASE/out" 2>"$CASE/err"
  RC=$?
  manifest_tag_fault "manifest key ids wrapper fault" key-missing
  manifest_tag_no_key_output
}

case_manifest_tag_wrappers() {
  manifest_tag_fixture wrappers
  bash -c 'source "$1" && manifest_tag_sign "$2" "$3"' bash \
    "$HERE/../lib/manifest-tag.sh" "$CASE/key-first" "$CASE/manifest" \
    >"$CASE/out" 2>"$CASE/err"
  expect_eq "manifest tag wrapper: sign exit code" "$?" 0
  expect_ok "manifest tag wrapper: exact tag line" cmp -s "$CASE/out" "$CASE/tag"
  expect_no "manifest tag wrapper: no sign stderr" test -s "$CASE/err"
  manifest_tag_no_key_output
  bash -c 'source "$1" && manifest_tag_check "$2" "$3" "$4"' bash \
    "$HERE/../lib/manifest-tag.sh" "$CASE/keys" "$CASE/manifest" "$CASE/tag" \
    >"$CASE/out" 2>"$CASE/err"
  expect_eq "manifest tag wrapper: check exit code" "$?" 0
  expect_no "manifest tag wrapper: no stdout" test -s "$CASE/out"
  expect_no "manifest tag wrapper: no stderr" test -s "$CASE/err"
  manifest_tag_no_key_output
  sed -i 's/e60535bd89336ac1/0000000000000000/' "$CASE/tag"
  bash -c 'source "$1" && manifest_tag_check "$2" "$3" "$4"' bash \
    "$HERE/../lib/manifest-tag.sh" "$CASE/keys" "$CASE/manifest" "$CASE/tag" \
    >"$CASE/out" 2>"$CASE/err"
  RC=$?
  manifest_tag_fault "manifest tag wrapper fault" unknown-key
  manifest_tag_no_key_output
}

case_manifest_tag_key_faults() {
  local form mode fault
  for form in missing directory empty short long upper-case carriage-return nul three-lines two-line-feeds bad-second \
    nul-after-bad-line cr-after-bad-line nul-third-line cr-third-line; do
    manifest_tag_fixture "key-$form"
    cp "$CASE/key-first" "$CASE/bad-key"
    fault='key-line 1'
    case "$form" in
      missing) rm -- "$CASE/bad-key"; fault=key-missing ;;
      directory) rm -- "$CASE/bad-key"; mkdir "$CASE/bad-key"; fault=key-read ;;
      empty) : >"$CASE/bad-key" ;;
      short) perl -pi -e 's/.\n\z/\n/' "$CASE/bad-key" ;;
      long) perl -pi -e 's/\n\z/0\n/' "$CASE/bad-key" ;;
      upper-case) perl -pi -e 's/a/A/' "$CASE/bad-key" ;;
      carriage-return) perl -pi -e 's/\n\z/\r\n/' "$CASE/bad-key"; fault=key-cr ;;
      nul) perl -pi -e 's/0/\0/' "$CASE/bad-key"; fault=key-nul ;;
      three-lines) cat "$CASE/keys" "$CASE/key-first" >"$CASE/bad-key"; fault=key-lines ;;
      two-line-feeds) printf '\n' >>"$CASE/bad-key"; fault='key-line 2' ;;
      bad-second) printf 'invalid\n' >>"$CASE/bad-key"; fault='key-line 2' ;;
      nul-after-bad-line) printf 'invalid\n\000\n' >"$CASE/bad-key"; fault=key-nul ;;
      cr-after-bad-line) printf 'invalid\n\r\n' >"$CASE/bad-key"; fault=key-cr ;;
      nul-third-line) cat "$CASE/keys" >"$CASE/bad-key"; printf '\000' >>"$CASE/bad-key"; fault=key-nul ;;
      cr-third-line) cat "$CASE/keys" >"$CASE/bad-key"; printf '\r' >>"$CASE/bad-key"; fault=key-cr ;;
    esac
    for mode in sign check key-ids; do
      if [ "$mode" = sign ]; then
        manifest_tag_run sign "$CASE/bad-key" "$CASE/manifest"
      elif [ "$mode" = check ]; then
        manifest_tag_run check "$CASE/bad-key" "$CASE/manifest" "$CASE/tag"
      else
        manifest_tag_run key-ids "$CASE/bad-key"
      fi
      manifest_tag_fault "manifest key $form ($mode)" "$fault"
    done
  done
}

case_manifest_tag_key_line_ends() {
  local count ending
  for count in one two; do
    for ending in line-feed no-line-feed; do
      manifest_tag_fixture "key-$count-$ending"
      if [ "$count" = one ]; then cp "$CASE/key-first" "$CASE/keys"; fi
      if [ "$ending" = no-line-feed ]; then truncate -s -1 "$CASE/keys"; fi
      manifest_tag_run sign "$CASE/keys" "$CASE/manifest"
      expect_eq "manifest key $count $ending: sign exit code" "$RC" 0
      expect_ok "manifest key $count $ending: exact tag" cmp -s "$CASE/out" "$CASE/tag"
      expect_no "manifest key $count $ending: no sign stderr" test -s "$CASE/err"
      manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/tag"
      expect_eq "manifest key $count $ending: check exit code" "$RC" 0
      expect_no "manifest key $count $ending: no check stdout" test -s "$CASE/out"
      expect_no "manifest key $count $ending: no check stderr" test -s "$CASE/err"
    done
  done
}

case_manifest_tag_manifest_limit() {
  manifest_tag_fixture manifest-limit
  perl -e 'print "m" x 65536' >"$CASE/manifest"
  manifest_tag_run sign "$CASE/keys" "$CASE/manifest"
  expect_eq "manifest limit: sign exit code" "$RC" 0
  expect_no "manifest limit: no sign stderr" test -s "$CASE/err"
  cp "$CASE/out" "$CASE/tag"
  manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/tag"
  expect_eq "manifest limit: check exit code" "$RC" 0
  expect_no "manifest limit: no check stdout" test -s "$CASE/out"
  expect_no "manifest limit: no check stderr" test -s "$CASE/err"
  printf 'm' >>"$CASE/manifest"
  manifest_tag_run sign "$CASE/keys" "$CASE/manifest"
  manifest_tag_fault "manifest limit plus one (sign)" manifest-size
  manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/tag"
  manifest_tag_fault "manifest limit plus one (check)" manifest-size
}

# No valid key or tag fills its byte limit. At the limit, the reader must reach the grammar check.
case_manifest_tag_key_and_tag_limits() {
  local kind limit fault
  for kind in key tag; do
    manifest_tag_fixture "$kind-limit"
    if [ "$kind" = key ]; then limit=65536; else limit=4096; fi
    perl -e 'print "a" x $ARGV[0]' "$limit" >"$CASE/large-file"
    if [ "$kind" = key ]; then
      manifest_tag_run sign "$CASE/large-file" "$CASE/manifest"
    else
      manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/large-file"
    fi
    if [ "$kind" = key ]; then fault='key-line 1'; else fault=tag-format; fi
    manifest_tag_fault "$kind limit reaches grammar" "$fault"
    if [ "$kind" = key ]; then
      manifest_tag_run key-ids "$CASE/large-file"
      manifest_tag_fault "key limit reaches grammar (key-ids)" 'key-line 1'
    fi
    printf 'a' >>"$CASE/large-file"
    if [ "$kind" = key ]; then
      manifest_tag_run sign "$CASE/large-file" "$CASE/manifest"
    else
      manifest_tag_run check "$CASE/keys" "$CASE/manifest" "$CASE/large-file"
    fi
    manifest_tag_fault "$kind limit plus one" "$kind-size"
    if [ "$kind" = key ]; then
      manifest_tag_run key-ids "$CASE/large-file"
      manifest_tag_fault "key limit plus one (key-ids)" key-size
    fi
  done
}

manifest_tag_location_run() {
  bash -c 'set -euo pipefail; umask 077; source "$1"; BACKUP_DIR=$2; manifest_tag_check_location "$3"' bash \
    "$HERE/../lib/manifest-tag.sh" "$B" "$1" >"$CASE/out" 2>"$CASE/err"
  RC=$?
  manifest_tag_no_key_output
}

case_manifest_tag_key_location() {
  manifest_tag_fixture key-location
  manifest_tag_location_run "$CASE/key-first"
  manifest_tag_fault "manifest key on backup filesystem" 'the manifest key file lies on the backup disk'
  manifest_tag_location_run "$CASE/missing-key"
  manifest_tag_fault "manifest key location: missing key" key-read
  local dir
  if [ ! -d "$OTHER_FS_DIR" ] || ! dir=$(mktemp -d "$OTHER_FS_DIR/manifest-key.XXXXXX" 2>/dev/null); then
    skip "manifest key location needs a second filesystem (no usable $OTHER_FS_DIR)"
    return
  fi
  SHM_DIRS+=("$dir")
  if [ "$(stat -c %d -- "$dir")" = "$(stat -c %d -- "$B")" ]; then
    skip "manifest key location needs a second filesystem ($OTHER_FS_DIR is on the backup filesystem)"
    return
  fi
  cp "$CASE/key-first" "$dir/key"
  manifest_tag_location_run "$dir/key"
  expect_eq "manifest key on another filesystem: exit code" "$RC" 0
  expect_no "manifest key on another filesystem: no stdout" test -s "$CASE/out"
  expect_no "manifest key on another filesystem: no stderr" test -s "$CASE/err"
  B=$CASE/missing-backups
  manifest_tag_location_run "$dir/key"
  manifest_tag_fault "manifest key location: missing backup directory" key-location
}

case_manifest_tag_no_key_arguments() {
  manifest_tag_fixture no-key-arguments
  grep -rE -- '-hmac|-macopt|hexkey' "$HERE/../bin" "$HERE/../lib" >"$CASE/out" 2>"$CASE/err"
  expect_eq "manifest tag: no secret command arguments" "$?" 1
  expect_no "manifest tag: no secret argument matches" test -s "$CASE/out"
  expect_no "manifest tag: no secret argument scan errors" test -s "$CASE/err"
}


case_manifest_tag_key_unreadable() {
  if [ "$(id -u)" -eq 0 ]; then
    skip "manifest key unreadable case needs a non-root user"
    return
  fi
  manifest_tag_fixture key-unreadable
  cp "$CASE/key-first" "$CASE/unreadable-key"
  chmod 0000 "$CASE/unreadable-key"
  local mode
  for mode in sign check key-ids; do
    case "$mode" in
      sign) manifest_tag_run sign "$CASE/unreadable-key" "$CASE/manifest" ;;
      check) manifest_tag_run check "$CASE/unreadable-key" "$CASE/manifest" "$CASE/tag" ;;
      key-ids) manifest_tag_run key-ids "$CASE/unreadable-key" ;;
    esac
    manifest_tag_fault "manifest key unreadable ($mode)" key-read
  done
  chmod 0600 "$CASE/unreadable-key"
}

# Check the complete output and the two fields from one read. Never print the supplied fault.
manifest_tag_message_case() {
  local message line
  bash -c 'set -euo pipefail; source "$1"; manifest_tag_message "$2"' bash \
    "$HERE/../lib/manifest-tag.sh" "$1" >"$CASE/out" 2>"$CASE/err"
  expect_eq "manifest tag mapper: exit code" "$?" 0
  printf '%s\t%s\n' "$2" "$3" >"$CASE/expected-message"
  expect_ok "manifest tag mapper: exact output" cmp -s "$CASE/out" "$CASE/expected-message"
  expect_no "manifest tag mapper: no stderr" test -s "$CASE/err"
  IFS=$'\t' read -r message line <"$CASE/out"
  expect_ok "manifest tag mapper: message field" test "$message" = "$2"
  expect_ok "manifest tag mapper: line field" test "$line" = "$3"
  manifest_tag_no_key_output
}

case_manifest_tag_messages() {
  manifest_tag_fixture messages
  local fault message line
  while IFS='|' read -r fault message line; do
    manifest_tag_message_case "$fault" "$message" "$line"
  done <<'MESSAGES'
key-missing|the manifest key file is missing|
key-read|cannot read the manifest key file|
key-nul|the manifest key file holds a NUL byte|
key-cr|the manifest key file holds a carriage return|
key-lines|the manifest key file has too many lines|
key-line 1|the manifest key file has a bad line|1
key-line 2|the manifest key file has a bad line|2
key-line 10|the manifest key file has a bad line|10
key-line 99|the manifest key file has a bad line|99
key-size|the manifest key file exceeds the size limit|
the manifest key file lies on the backup disk|the manifest key file lies on the backup disk|
key-location|cannot check the filesystem of the manifest key file|
tag-read|cannot read the tag file of the set|
tag-format|the tag file of the set has a bad line|
tag-size|the tag file of the set exceeds the size limit|
unknown-key|the tag of the set comes from an unknown key|
tag-mismatch|the tag of the set does not match the manifest|
manifest-read|cannot read the manifest|
manifest-size|the manifest exceeds the size limit|
usage|the manifest tag program failed|
output-write|the manifest tag program failed|
untrusted-fault-do-not-echo|the manifest tag program failed|
key-format|the manifest tag program failed|
key-line 123|the manifest tag program failed|
key-line -1|the manifest tag program failed|
key-line x|the manifest tag program failed|
key-line 1 extra|the manifest tag program failed|
key-line |the manifest tag program failed|
|the manifest tag program failed|
MESSAGES
  manifest_tag_message_case $'key-line 1\nuntrusted-fault-do-not-echo' 'the manifest tag program failed' ''
  expect_no "manifest tag mapper: no unknown input" grep -qF untrusted-fault-do-not-echo "$CASE/out"
}

case_manifest_tag_vector
case_manifest_tag_round_trip
case_manifest_tag_changed_byte
case_manifest_tag_unknown_key
case_manifest_tag_malformed
case_manifest_tag_first_key
case_manifest_tag_second_key
case_manifest_tag_key_ids
case_manifest_tag_key_ids_usage
case_manifest_tag_output_write
case_manifest_tag_key_ids_wrapper
case_manifest_tag_wrappers
case_manifest_tag_key_faults
case_manifest_tag_key_line_ends
case_manifest_tag_manifest_limit
case_manifest_tag_key_and_tag_limits
case_manifest_tag_key_location
case_manifest_tag_no_key_arguments
case_manifest_tag_key_unreadable
case_manifest_tag_messages
