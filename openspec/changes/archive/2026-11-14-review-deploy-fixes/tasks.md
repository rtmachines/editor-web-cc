# Tasks

Each task is one Codex run. Write the red test first, see it fail, and then apply the fix. Do not edit files outside the paths of the task.

## 1. NUL byte in a secret file of the init script (F-24)

- [x] 1.1 (deploy/db/initdb/01-roles.sh) Apply design D1: `check_secret_file` counts the NUL bytes before the command substitution and fails with "holds a NUL byte". Name the NUL byte in the header comment. — red test: add `the_script_stops_on_a_secret_file_with_a_nul_byte` to `crates/recipe-db/tests/storage/grants/init_script.rs`. It writes `pass\0word\n` into `app_password` and asserts with `assert_stopped`. Before the fix the script passes the check and exits with another code. — Check: `cargo test -p recipe-db --test storage init_script` (needs Docker) and `bash -n deploy/db/initdb/01-roles.sh`.

## 2. Rust cache of pull requests (F-26)

- [x] 2.1 (.github/workflows/ci.yml) Apply design D2: add `with:` and `save-if: ${{ github.ref == 'refs/heads/main' }}` to the `rust-cache` steps of `clippy`, `test`, `doc`, and `coverage`. — red test: new `deploy/ci/test_rust_cache.py`. It asserts that each `Swatinem/rust-cache@` step in `.github/workflows/*.yml` has that `save-if` line, and its self-test rejects a step without it. Before the fix it reports the four steps. — Check: `python3 -I -B deploy/ci/test_rust_cache.py` and `bash deploy/ci/check.sh`.

## 3. Pull request check with the release toolchain (F-27)

- [x] 3.1 (.github/workflows/ci.yml, deploy/ci/rust-tests.sh, CONTRIBUTING.md) After task 2.1, apply design D3: add the case `release-toolchain` to `rust-tests.sh` and the job `release-toolchain` to `ci.yml`. Add one sentence to `CONTRIBUTING.md`. — red test: new `deploy/ci/test_release_toolchain.py`. Its static part asserts that the job runs `bash deploy/ci/rust-tests.sh release-toolchain`. It fails when the job has a job-level `if:`, when that step has an `if:` key, or when the job has a `needs:` key. It also fails when the job or that step has a `continue-on-error:` key. `deploy/ci/check.sh` line 418 applies this rule to the job `test`. The `save-if:` key of the cache step does not count. Its self-test feeds one bad job for each of these keys and expects a failure. It feeds a job with only `save-if:` and expects a pass. Its run part copies `rust-tests.sh` into a temporary tree with its own `deploy/Dockerfile`. Fake `rustup` and `cargo` come first on `PATH`. The fakes log their arguments and `RUSTUP_TOOLCHAIN`. There are two good fixtures, `ENV RUSTUP_TOOLCHAIN=1.99.1 \` and `ENV RUSTUP_TOOLCHAIN=1.98.0 \`. For each, the fake `rustup` gets `toolchain install <version> --profile minimal`. The fake `cargo` gets `check --release --locked --package recipe-server` and `RUSTUP_TOOLCHAIN=<version>`. The second version shows that the script holds no fixed value. No entry, two entries, and the bad version `1.99` exit with code 1, and no fake runs. One more case makes the fake `cargo` exit with code 101, and `release-toolchain` then exits with a code other than 0. Before the fix the job and the case are missing. — Check: `python3 -I -B deploy/ci/test_release_toolchain.py`, `bash deploy/ci/check.sh`, and `bash deploy/ci/rust-tests.sh release-toolchain` on a host with `rustup`.

## 4. Escape in the docker stub (F-28)

- [x] 4.1 (deploy/ci/test_pitr_recovery.py, deploy/ci/check.sh) Apply design D4: write `\\$` on line 2106, and add the compile step with `-W error` to `check.sh`. — red test: the new step of `check.sh` comes first and fails on `test_pitr_recovery.py` with the `SyntaxError` about `"\$"`. Also record the SHA-256 of `STEP_DOCKER_STUB` before the fix and assert the same value after it. — Check: `bash deploy/ci/check.sh` and `python3 -I -B -W error deploy/ci/test_pitr_recovery.py`.

## 5. Header of the production Caddyfile (F-29)

- [x] 5.1 (deploy/Caddyfile) Apply design D5: line 2 names the tls line, the `auto_https` line, and the `skip_install_trust` option. — red test: add `caddyfiles_differ_only_in_the_named_lines` to `crates/recipe-server/tests/deploy_values/files/caddy.rs`. It takes `code_lines` of both files and keeps repeated lines such as `}`. It replaces the permitted line of each pair by one marker per pair. `auto_https disable_certs` and `skip_install_trust` form one pair. `tls /run/secrets/tls_cert /run/secrets/tls_key` and `tls internal` form the other pair. It asserts that the two full ordered sequences are equal, and that line 2 of each file names all three items. An extra `}` at the end of `deploy/Caddyfile` makes the test fail. Before the fix line 2 of `deploy/Caddyfile` lacks `skip_install_trust`. — Check: `cargo test -p recipe-server --test deploy_values caddy`.

## 6. CHANGELOG entry

- [x] 6.1 (CHANGELOG.md) [mechanical] After tasks 1.1 to 5.1, add the entries of this change under `[Unreleased]`, as `reliability.md` §8 requires. `### Added` names the job `release-toolchain` and the compile step of `check.sh`. `### Fixed` names the NUL refusal of `01-roles.sh` and the cache save on `main` only. Change A `review-code-fixes` also adds entries. The change that merges second rebases `CHANGELOG.md` and keeps both sets of entries. — red test: none, because the file is a document. — Check: the ste-lint command of task 7.3 reports 0 hard violations.

## 7. Integration checks

- [x] 7.1 (.) Run `bash deploy/ci/check.sh`, `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo test --workspace --all-features --locked`. Check: all four pass. The CI jobs `ci-scripts`, `fmt`, `clippy`, `test`, and `release-toolchain` pass on the pull request.
- [x] 7.2 (.) Run `cargo deny --locked check` and the three coverage commands of `CONTRIBUTING.md`, in this order: `python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json`, `cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json`, and `python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json`. Check: all four pass. The CI jobs `deny` and `coverage` pass on the pull request.
- [x] 7.3 (.) Run `openspec validate review-deploy-fixes --strict`, `openspec validate --specs --strict`, and `bash -O globstar -O nullglob -c 'python -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md openspec/changes/review-deploy-fixes/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'`. Check: both validations pass, and ste-lint reports 0 hard violations.
- [x] 7.4 (.) After the merge, run the workflow `ci.yml` on `main`. Check: the jobs `compose`, `upgrade`, `restore-switch`, and `pitr-recovery` pass, so the init script works on an empty volume.

## Workflow follow-up

- The owner adds the job `release-toolchain` to the branch protection of `main`.
- Change B `review-design-fixes` writes the ADR of F-25 (option A of `design.md` "Decided Questions").
