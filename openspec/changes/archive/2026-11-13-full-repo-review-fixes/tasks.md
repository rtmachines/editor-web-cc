# Tasks

## 1. Checked arithmetic in recipe-server (R-01)

- [x] 1.1 (recipe-server) Add a test to `limited_read/tests.rs`. Give a path a count of `usize::MAX`. Then `mark_if_running` and `finish` of a second call must leave the count at `usize::MAX`. See the test fail. Then apply design D1 in `limited_read.rs`. Check: the new test and every `limited_read` test pass.
- [x] 1.2 (recipe-server) Apply design D2 in `credential_cipher.rs`. No red test: the change keeps the bytes, and `sealed_form_has_version_nonce_ciphertext_and_tag` holds them. Check: every test of `credential_cipher/tests.rs` passes.
- [x] 1.3 (recipe-server) [mechanical] Apply design D3 in `create_admin.rs` and `check_results.rs`, with a doc comment on each constant. No red test: the change keeps the limits, and the size-limit tests hold them. Check: those tests pass, and the grep command below prints no line.

  `grep -rnE 'saturating_(add|sub)' crates/recipe-server/src --exclude=tests.rs --exclude='*_tests.rs' --exclude-dir=tests --exclude-dir=test_support`

## 2. Drop contract of SecretField (R-02)

- [x] 2.1 (recipe-web) Apply design D4: add the compile-time assertion to `connection_request.rs` and remove the test `a_secret_field_clears_its_buffer_on_drop`. Red step: set the field to `String` and the constructor to `Self(text)`. `cargo build -p recipe-web` must fail only with `E0277` on `String: ZeroizeOnDrop`. Restore both lines. Check: `cargo build -p recipe-web` and every `connection_request_tests` test pass.

## 3. Bound of stalled calls (R-03)

- [x] 3.1 (recipe-app) Add the test of design D5 to `jobs/alerts/tests/disks.rs`. In it, a disk double counts the calls in flight, and one run of `alerts` gives a peak of 1. No red test: the test holds a premise of ADR-0161 that is true on `main`. Check: the test passes, and it fails when `low_disk` joins its reads (try it once, then restore).
- [x] 3.2 (docs/adr) Write ADR-0161 from design D5 in the shape of ADR-0133, with ADR-0151 (o), (p) in Related. Set the Status line of ADR-0133 and of its index row. Add the index row of ADR-0161. Add ADR-0161 to the two decision map rows of design D5. Check: `ste-lint.py` reports 0 hard violations on ADR-0161, ADR-0133, and `docs/adr/README.md`.
- [x] 3.3 (docs/architecture) [mechanical] Add the bound with a citation of ADR-0161 to `reliability.md` §7.8, after the line of ADR-0133 (d), (e). Add it also to the item of the maps of stalled paths in §8. Check: `ste-lint.py docs/architecture/reliability.md` reports 0 hard violations.
- [x] 3.4 (recipe-server) [mechanical] Cite ADR-0161 in the doc comments of `StalledPaths` in `limited_read.rs` and of `Alerts::low_disk` in `recipe-app/src/jobs/alerts.rs`. No red test: only comments change. Check: `RUSTDOCFLAGS='-D warnings' cargo doc -p recipe-server -p recipe-app --no-deps` passes.

## 4. Function and file length (R-04)

- [x] 4.1 (recipe-app) Apply design D6: move the lock and `row_version` step of `SetPlcConnection::execute` into the free function `lock_for_write`. No red test: the change keeps behavior, and the tests of `plant::set_plc_connection` hold it. Check: those tests pass, and `execute` has 50 lines or fewer, counted as in design D6.
- [x] 4.2 (recipe-app) [mechanical] Split `plant/set_plc_connection/tests.rs` by design D7. No red test: the split keeps every test. Check: the count of `cargo test -p recipe-app --lib plant::set_plc_connection -- --list` does not change, and each new file has 800 lines or fewer.
- [x] 4.3 (recipe-db) [mechanical] Split `tests/storage/plc_connection.rs` by design D7. No red test: the split keeps every test. Check: the count of `cargo test -p recipe-db --test storage -- --list` does not change, and each new file has 800 lines or fewer.
- [x] 4.4 (recipe-db) [mechanical] Split `tests/storage/constraints/plc_connection.rs` by design D7. No red test: the split keeps every test. Check: the count of `cargo test -p recipe-db --test storage -- --list` does not change, and each new file has 800 lines or fewer.
- [x] 4.5 (recipe-server) [mechanical] Split `tests/deploy_values/files.rs` by the file families of design D7, with `pub use` lines in `files.rs`. No red test: the split keeps every check. Check: the count of `cargo test -p recipe-server --test deploy_values -- --list` does not change. `git diff --color-moved=zebra` shows only moved lines. Each new file has 800 lines or fewer.
- [x] 4.6 (recipe-server) [mechanical] After task 4.5, split `tests/deploy_values/file_mutations.rs` by design D7. No red test: the split keeps every test. Check: the count of `cargo test -p recipe-server --test deploy_values -- --list` does not change, and each new file has 800 lines or fewer.

## 5. Text fixes (R-05, R-06)

- [x] 5.1 (recipe-app) [mechanical] Rewrite the doc comment of `lock_derived_types` in `ports/types.rs` by design D8. No red test: only a comment changes. Check: `RUSTDOCFLAGS='-D warnings' cargo doc -p recipe-app --no-deps` passes.
- [x] 5.2 (docs/ops) [mechanical] Rewrite the read-only sentence at `runbook.md:2397` by design D8. Check: `ste-lint.py docs/ops/runbook.md` reports 0 hard violations.
- [x] 5.3 (CHANGELOG.md) [mechanical] Add the line of design D8 under `Changed` in `[Unreleased]`. Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.

## 6. Integration checks

- [x] 6.1 (.) Run the full gate: `cargo fmt --all --check`, `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`, and `cargo test --workspace --all-features --locked`. Check: all three pass.
- [x] 6.2 (.) Run `openspec validate full-repo-review-fixes --strict`, `openspec validate --specs --strict`, and the `ste-lint.py` command of `CLAUDE.md`. Find each `.rs` file in `crates/` above 800 lines. Check: both validations pass, ste-lint reports 0 hard violations, and the file search finds no file.

## 7. Review fixes

- [x] 7.1 (recipe-server) Revise D1 by design D1. Change the test of `limited_read/tests.rs` with the count `usize::MAX`: `mark_if_running` must give `Err(LimitFault::MarkOverflow)`, and the count stays `usize::MAX`. See the test fail. Then add the variant, return it from `mark_if_running`, and give it from `read_with_limit`. Check: every `limited_read` test and clippy of `recipe-server` pass.
- [x] 7.2 (recipe-app) Change the test of design D5 in `jobs/alerts/tests/disks.rs`: the disk double counts the calls in flight on each path, and the test asserts a peak of 1 on each path. Build the job with the helpers of `support.rs`. Rewrite the doc comment of `Alerts::low_disk`. No red test: the test holds a premise that is true on `main`. Check: the test passes, and it fails when one run reads one path twice at the same time (try it once, then restore).
- [x] 7.3 (docs/adr) Rewrite ADR-0161 (b), (f), (g), and the risks, and add item (i), by design D1 and D5. Cite ADR-0161 (i) in the row "Integer arithmetic" of `reliability.md` §2.2. Check: `ste-lint.py` reports 0 hard violations on both files.
- [x] 7.4 (recipe-server) [mechanical] Rename `STDIN_READ_LIMIT` to `STDIN_READ_LIMIT_BYTES` in `create_admin.rs`. Check: the tests of `create_admin` pass.
- [x] 7.5 (CHANGELOG.md) [mechanical] In the line of ADR-0161, change "The code does not change." to "The behavior does not change.". Check: `ste-lint.py CHANGELOG.md` reports 0 hard violations.
- [x] 7.6 (.) Run the checks of tasks 6.1 and 6.2 again. Check: all pass.

## Workflow follow-up

- Review the code with `code-review` (Opus) and Codex. Fix each CRITICAL and HIGH finding.
- Run `superpowers:verification-before-completion` and the local checks of `CONTRIBUTING.md`.
- Archive the change with `openspec-archive-change`.
- Add `docs/plan/review/2026-11-11-full-repo-review.md` to the commit of this change, only after a request of the user.
