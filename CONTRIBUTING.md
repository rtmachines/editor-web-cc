# Contributing

This file repeats the rules of sections 2 and 8 of [`docs/architecture/reliability.md`](docs/architecture/reliability.md). That document is the source. When this file and `reliability.md` disagree, `reliability.md` applies.

## Language and style

- Code comments, identifiers, commit messages, documentation, and specifications are English (R42).
- English text follows the ASD-STE100 structural rules (R40).
- `tools/ste-lint/ste-lint.py` checks these rules. CI runs it on the files that ADR-0023 and ADR-0135 name. One hard violation fails the job.
- Hard violations are a semicolon in prose, a sentence of more than 25 words, and two verbs from one synonym group in one file.
- A soft phrasal verb, a marketing adjective, a nominalization, and a list item that ends with "and" or "or" are also hard violations.
- Passive voice and compound tenses are advisory. They do not fail the job.
- Each design pattern in the code names the problem that it solves in the doc comment of its type (R43). A review rejects a pattern with no named problem.

## No-panic policy and lints

Every crate has `#![forbid(unsafe_code)]` and `[lints] workspace = true`. The workspace `Cargo.toml` sets these lints:

| Lint | Level |
|---|---|
| `clippy::unwrap_used`, `clippy::expect_used`, `clippy::panic`, `clippy::unreachable` | deny |
| `clippy::todo`, `clippy::unimplemented` | deny |
| `clippy::indexing_slicing`, `clippy::arithmetic_side_effects` | deny |
| `clippy::cast_possible_truncation`, `clippy::cast_sign_loss`, `clippy::cast_possible_wrap`, `clippy::cast_precision_loss` | deny |
| `clippy::exit`, `clippy::print_stdout`, `clippy::print_stderr` | deny |
| `unused_must_use` | deny |
| `clippy::large_futures` | warn |

`[profile.release]` sets `overflow-checks = true` and `panic = "unwind"`.

Rules for code that cannot panic:

- Integer arithmetic uses `checked_*` and returns `Err` on `None`. Use `saturating_*` only for a counter that the UI shows.
- Integer conversion uses `try_from`. Never use `as` between integer types.
- Domain values are `rust_decimal::Decimal`. Decimal arithmetic uses `checked_*`.
- The domain needs no float to integer conversion. If a conversion is necessary, use `Decimal::to_i64()` and handle `None`.
- A division checks the divisor first or uses `checked_div`.
- The resolution of a type tree uses an explicit stack, not recursion. The `TypeTree` validation limits the depth.
- Access uses `get`, `first`, `last`, `entry`, and iterators. Never write `v[i]`, `map[&key]`, or `s[a..b]`.
- An `Option` ends in `let Some(x) = opt else { return Err(..) };` or in `opt.ok_or(..)?`.
- A regular expression is a literal in a `static` of type `LazyLock<Result<Regex, regex::Error>>`. The caller maps `Err` with `map_err`.
- A unit test compiles every regular expression, so a bad pattern fails the build.
- `parking_lot::Mutex` replaces `std::sync::Mutex`.
- Never call `block_on` inside async code. Argon2id hashing and snapshot validation run in `spawn_blocking`.
- A use case maps the `JoinError` of a `spawn_blocking` task to `AppError::TaskFailed { stage }`.
- Time comes from `time::OffsetDateTime::now_utc()`.

Permitted exceptions:

- Test code (`#[cfg(test)]` and `tests/`) can use `unwrap`, `expect`, and indexing.
- `clippy.toml` permits them in `#[test]` functions and in `#[cfg(test)]` code.
- The root file of each integration test crate in `tests/` starts with `#![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]` and a comment that names `reliability.md` section 2.3. The attribute covers every module of that test crate.
- The watchdog function is the only item with `#[allow(clippy::exit)]`. A comment on the attribute names `reliability.md` section 2.3.
- `recipe-server::main` returns `anyhow::Result<()>` and has no `expect`. A failed startup check writes one `ERROR` line, and the process exits with code 1.
- A review rejects every other `#[allow]` of these lints outside tests.

Panic containment (`reliability.md` section 2.4):

- A panic hook writes one `tracing::error!` event and does not end the process.
- The backstop `CatchPanicLayer` sits directly inside the request id layer, so every answer has a request id. A panic in a request gives `500 E-WEB-010`.
- A supervisor loop restarts a background job after a panic.
- A watchdog thread ends a stalled process with exit code 70, and Docker restarts the container.
- Application code never calls `std::panic::catch_unwind`.

## Dependencies

- `overview.md` section 5 gives the dependency rule of the workspace crates.
- `deny.toml` bans six crates outside their permitted users:
  - `recipe-app` outside `recipe-db`, `recipe-web`, and `recipe-server`,
  - `recipe-web` outside `recipe-server`,
  - `recipe-export` outside `recipe-web`,
  - `recipe-db` outside `recipe-server` and its own dev-dependency, which enables the feature `test-support` for the storage tests,
  - `sqlx` outside `recipe-db`,
  - `anyhow` outside `recipe-server` (Assumption A37).
- The proc macro `prost-derive` under `testcontainers` is the one third-party exception to the `anyhow` ban.
- Review checks the other forbidden dependencies of `overview.md` section 5, for example `axum` and `tokio` in `recipe-domain`.
- `cargo deny check` runs in CI. It fails on a denied license, a yanked crate, a security advisory, or a banned dependency edge.
- `cargo audit` runs nightly against the `Cargo.lock`. `npm audit` runs nightly against `e2e/package-lock.json`.
- `Cargo.lock` is in the repository. Dependency updates happen once per month in one pull request. A security advisory gets an update at once.
- The minimum supported Rust version is the current stable at release time. `rust-version` in the workspace `Cargo.toml` records it. The project uses edition 2024.

## Code conventions

- `rustfmt` uses the workspace `rustfmt.toml`.
- No `unsafe`, no `static mut`, and no global mutable state. Shared state goes through axum `State` as `Arc<AppState>`.
- Interior mutability exists only in the places that `reliability.md` section 8 lists.
- Every public type and function in a library crate has a doc comment.
- Commit messages follow Conventional Commits in English, for example `feat:`, `fix:`, `docs:`, `test:`, `refactor:`, and `chore:`.

## Local checks

Run these commands before you open a pull request. CI runs the same checks on each pull request.

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
bash deploy/ci/rust-tests.sh secure
bash deploy/ci/rust-tests.sh release-toolchain
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --locked
cargo deny --locked check
python3 -I -B tools/test_coverage_gate.py
python3 -I -B tools/test_coverage_report.py
python3 -I -B tools/test_coverage_inventory.py
python3 -I -B tools/test_coverage_summary.py
python3 -I -B tools/test_coverage_cli.py
python3 -I -B tools/coverage-gate.py --write-inventory target/coverage-inventory.json
cargo llvm-cov --workspace --all-features --locked --json --output-path target/llvm-cov-report.json
python3 -I -B tools/coverage-gate.py --report target/llvm-cov-report.json --inventory target/coverage-inventory.json
bash -O globstar -O nullglob -c 'python3 -I tools/ste-lint/ste-lint.py docs/{architecture,adr,ops,user}/**/*.md openspec/specs/**/*.md README.md CONTRIBUTING.md CHANGELOG.md'
```

The `secure` mode tests the refusal of `session.secure=false` in a build without the `dev` feature (ADR-0062 (b)).

The `release-toolchain` mode needs `rustup` and network access. It installs the toolchain of `ENV RUSTUP_TOOLCHAIN=` in `deploy/Dockerfile`.

The `openspec` job checks the specifications with the local command `openspec validate --all --strict`.

The coverage target is 80 % line coverage per crate (`reliability.md` section 6). `tools/coverage-gate.py` applies it to each crate. The gate counts production lines only. It finds test modules from their `#[cfg(test)]` declarations (ADR-0127).

The gate fails on an inline test module in `src/`. Move such a module to its own file, for example `foo/tests.rs`.

The gate fails on a production file with a test file name: `tests.rs`, `*_tests.rs`, or `*-tests.rs`. It also fails on a production file in a `tests/`, `examples/`, or `benches/` directory under `src/`.

The gate fails on a required production file with no line in the report. A declaration-only file needs no report entry. A crate without a production file shows "no code".

A crate that passes the percentage shows `incomplete` in its row when a required file is missing or has zero lines. The summary prints each other-configuration file as `<path> (<configuration>)`. It lists the build scripts under `build-time code outside the metric, not counted:`.

Run `python3 -I tools/coverage-gate.py --source-only` to scan the source without a report.

Run the five files `tools/test_coverage_*.py` with `python3 -I -B` to test the gate.

The gate exits with one of three codes:

| Exit | Meaning |
|---|---|
| 0 | Every check passes. Accounting is complete and each crate has at least 80 % lines. |
| 1 | Coverage deficit: a crate below 80 %. Incomplete accounting: a required file without lines, an untracked file without a report entry, or a violated source rule. |
| 2 | Input or policy error: a bad report, path, counter, policy, inventory, or argument, or a bad inventory file entry. A report entry for an untracked file, a directory outside a Git work tree, or a `#[path]` attribute in `src/` also gives this code. |

Exit 2 wins over exit 1. The gate prints no percentage after an input or policy error.

`tools/coverage-policy.json` holds exact-path entries for target-gated files and files of another configuration (ADR-0162). The rules of an entry:

- An entry names one exact path. A wildcard entry fails with exit 2.
- Each entry has a path, a source hash, a configuration, a reason, and evidence.
- A duplicate, stale, unused, or incomplete entry fails with exit 2.
- A missing report entry never creates a policy entry.
- A declaration-only file needs no entry.

The CI job `Coverage` uploads the report and the inventory as the artifact `coverage-report-<commit SHA>` and keeps it 14 days.

### Browser regression suite

The Playwright suite in `e2e/` runs against `examples/dev_server.rs` (ADR-0048). It needs Node 22. CI runs it on each pull request.

```bash
cd e2e
npm ci
npx playwright install chromium
E2E_PASSWORD=<12 to 128 characters> npm test
```

The variable `RECIPE_DEV_PORT` changes the port of the development server. The default port of the suite is 18080.

### Jobs outside pull requests

The jobs `compose`, `upgrade`, `restore-switch`, and `pitr-recovery` of `.github/workflows/ci.yml` run only on a push to `main` and on a manual dispatch (ADR-0094). A pull request does not start them. The input `upgrade_from` of a dispatch names the previous release ref of the upgrade test. Without the input, `deploy/ci/upgrade.sh` selects the previous commit. The upgrade test runs only on a Linux CI runner.

The job `load` of `.github/workflows/ci.yml` runs only on a push to `main` and on a manual dispatch (ADR-0128 (a)). A pull request does not start it. A red `load` job on `main` blocks the next release (ADR-0128 (d)).

The job `ci-scripts` runs on each pull request without Docker. It runs `deploy/ci/check.sh`, which runs the Python unit tests and the structure checks of `deploy/ci` (ADR-0128 (e)).

The job `release-toolchain` checks `recipe-server` on each pull request with the toolchain version from `deploy/Dockerfile`.

The CI job `heap` runs on each pull request. It runs the ignored measurement tests of `recipe-web` in parallel shards with `--cfg heap_measure`.
The shard `rest` runs every ignored test that no other shard selects (ADR-0223).

## Pull requests

A pull request needs:

1. green CI,
2. one review,
3. an entry in `CHANGELOG.md` when behavior changes.

## Release

This section describes the release steps of the maintainer. Commit `7f102d1` (release 0.7.1) is the model for the file changes.

1. Update the release files as commit `7f102d1` does.
   - `CHANGELOG.md`: Move the entries of `## [Unreleased]` under `## [X.Y.Z] - <date>`. Keep `## [Unreleased]` empty.
   - `CHANGELOG.md`: Point `[Unreleased]` to `vX.Y.Z...HEAD`. Add the link ref `[X.Y.Z]`.
   - `CHANGELOG.md`: Update the section "Known gaps" for `X.Y.Z`.
   - `Cargo.toml`: Set the workspace `version` to `X.Y.Z`.
   - `Cargo.lock`: Update the version of the workspace crates.
   - `README.md`: Update the release line and the list "Known gaps of X.Y.Z".
   - `deploy/.env.example`: Set `RECIPE_VERSION` to `X.Y.Z`.
2. Run the checks of section "Local checks" and merge the pull request after one review.
3. Check that the job `load` is green on `main` for the merge commit that gets the tag (ADR-0128 (d)).
4. Tag that commit `vX.Y.Z` and push the tag.
5. Build both images from the tag, as ADR-0064 (i) describes. Save each image with `docker save`, as ADR-0064 (j) requires.
6. Compute the SHA-256 of each file. Read the image ID of each local image with `docker image inspect --format '{{.Id}}'`.
7. Put the `CHANGELOG.md` section of `X.Y.Z` into the body of the GitHub release of `vX.Y.Z`. Add the SHA-256 values and the image IDs (ADR-0198).

Run the release build in the directory `deploy/` and export `RECIPE_VERSION=X.Y.Z` first. The release build adds `--env-file .env.example`, but the header of `deploy/compose.build.yaml` gives the form without it. Compose needs more required values than `RECIPE_VERSION`. Without the env file, Compose stops with `required variable ... is missing a value`.

```bash
docker compose --env-file .env.example -f compose.yaml -f compose.build.yaml build
```

The command builds both images.

To check the configuration, run `docker compose --env-file .env.example -f compose.yaml -f compose.build.yaml config --images` in `deploy/`. The output must list `recipe-server:X.Y.Z` and `recipe-pgtools:X.Y.Z`. This command reads the configuration only. It does not show that the build made the images. Use the image IDs of step 6 in step 7. The script `deploy/ci/upgrade.sh` builds its previous-release images with `docker buildx build`.

The value of `RECIPE_VERSION` is `X.Y.Z` without `v`, while the Git tag is `vX.Y.Z`. The image names are `recipe-server:X.Y.Z` and `recipe-pgtools:X.Y.Z`.

No document gives the `docker save` command or the names of the image files. For step 6, use the commands of steps 1 and 4 of section "Load the images" in `docs/ops/runbook.md`.

`CHANGELOG.md` does not hold the SHA-256 values or the image IDs (ADR-0064 (k)).
