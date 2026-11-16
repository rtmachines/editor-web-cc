# manifest-tag.sh: wrappers for the manifest HMAC program and location check of ADR-0174 (p).
# The caller passes file paths only. It maps the fixed fault on stderr to its ERROR line.
# Location faults: usage, key-read, key-location, or the fixed sentence for equal device numbers.
# This file only defines functions. The caller sets its shell options and umask.
# shellcheck shell=bash

# manifest_tag_sign <key-file> <manifest>: print the tag line for the first key.
manifest_tag_sign() {
  perl "${BASH_SOURCE[0]%/*}/manifest-tag.pl" sign "$@"
}

# manifest_tag_check <key-file> <manifest> <tag-file>: check either key and preserve the exit code.
manifest_tag_check() {
  perl "${BASH_SOURCE[0]%/*}/manifest-tag.pl" check "$@"
}

# manifest_tag_key_ids <key-file>: print one key id per key line, in file order.
manifest_tag_key_ids() {
  perl "${BASH_SOURCE[0]%/*}/manifest-tag.pl" key-ids "$@"
}

# manifest_tag_message <fault-line>: print <fixed message><TAB><optional line number><LF>.
# Read both fields with IFS=$'\t' read -r message line. Unknown faults never reach the output.
manifest_tag_message() {
  local message line=''
  case ${1:-} in
    key-missing) message='the manifest key file is missing' ;;
    key-read) message='cannot read the manifest key file' ;;
    key-nul) message='the manifest key file holds a NUL byte' ;;
    key-cr) message='the manifest key file holds a carriage return' ;;
    key-lines) message='the manifest key file has too many lines' ;;
    key-line\ *)
      message='the manifest tag program failed'
      if [[ $1 =~ ^key-line\ ([0-9]{1,2})$ ]]; then
        message='the manifest key file has a bad line'
        line=${BASH_REMATCH[1]}
      fi
      ;;
    key-size) message='the manifest key file exceeds the size limit' ;;
    'the manifest key file lies on the backup disk') message='the manifest key file lies on the backup disk' ;;
    key-location) message='cannot check the filesystem of the manifest key file' ;;
    tag-read) message='cannot read the tag file of the set' ;;
    tag-format) message='the tag file of the set has a bad line' ;;
    tag-size) message='the tag file of the set exceeds the size limit' ;;
    unknown-key) message='the tag of the set comes from an unknown key' ;;
    tag-mismatch) message='the tag of the set does not match the manifest' ;;
    manifest-read) message='cannot read the manifest' ;;
    manifest-size) message='the manifest exceeds the size limit' ;;
    *) message='the manifest tag program failed' ;;
  esac
  printf '%s\t%s\n' "$message" "$line"
}

# manifest_tag_check_location <key-file>: compare its filesystem with BACKUP_DIR.
# Return 1 with one stderr line on a fault. The caller maps that line to one ERROR line.
manifest_tag_check_location() {
  local key_device backup_device
  if [ "$#" -ne 1 ] || [ -z "${BACKUP_DIR:-}" ]; then
    printf '%s\n' usage >&2
    return 1
  fi
  if ! key_device=$(stat -c %d -- "$1" 2>/dev/null); then
    printf '%s\n' key-read >&2
    return 1
  fi
  if ! backup_device=$(stat -c %d -- "$BACKUP_DIR" 2>/dev/null); then
    printf '%s\n' key-location >&2
    return 1
  fi
  if [ "$key_device" = "$backup_device" ]; then
    printf '%s\n' 'the manifest key file lies on the backup disk' >&2
    return 1
  fi
  return 0
}
