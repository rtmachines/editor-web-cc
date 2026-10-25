#!/usr/bin/env bash
# These commands run on the GitHub runner only.
set -euo pipefail
umask 077

case ${1:?} in
  setup)
    sudo apt-get update -qq
    sudo apt-get install --yes --no-install-recommends musl-tools
    rustup toolchain install
    rustup target add x86_64-unknown-linux-musl
    ;;
  panic)
    # The host command below listed 11 tests. Count lines that end with ': test'.
    # CARGO_INCREMENTAL=0 cargo test -p recipe-web --locked -- --list panic
    cargo test --target x86_64-unknown-linux-musl -p recipe-web panic --locked |
      awk '
        { print }
        /^test result: ok\. [0-9]+ passed;/ && $4 >= 11 { passed = 1 }
        END {
          if (!passed) {
            print "Expected at least 11 passing panic tests." > "/dev/stderr"
            exit 1
          }
        }
      '
    ;;
  secure)
    cargo test -p recipe-server --locked --no-default-features --lib config::serve::tests::serve_secure_false_is_refused_in_a_build_without_the_dev_feature -- --exact |
      awk '
        { print }
        /^test result: ok\. 1 passed;/ { passed = 1 }
        END {
          if (!passed) {
            print "Expected 1 passing secure refusal test." > "/dev/stderr"
            exit 1
          }
        }
      '
    ;;
  *) exit 1 ;;
esac
