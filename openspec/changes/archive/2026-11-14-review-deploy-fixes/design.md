# Design

## Context

See `proposal.md` - Why. The rules come from these sources. ADR-0031 gives the one-line rule of a secret file. ADR-0064 (d) pins the toolchain of the image. ADR-0094 runs the job `compose` only on `main` and on a dispatch. ADR-0128 (e) runs `deploy/ci/check.sh` in the job `ci-scripts` on each pull request. ADR-0065 gives the two Caddyfiles.

Facts at `8db04c7`:

- `check_secret_file` of `01-roles.sh` (lines 39-49) reads the file with `content=$(cat "$path"; printf x)`. Bash drops a NUL byte in a command substitution. The check then tests empty, carriage return, and a second line only. `check_secret_file` of `deploy/pgtools/bin/backup-once` (lines 133-149) counts the NUL bytes with `tr -cd '\000'` and fails.
- `crates/recipe-db/tests/storage/grants/init_script.rs` runs `01-roles.sh` in the harness container. Each test writes one bad secret file with `printf '%b'` and asserts exit code 1, the prefix `01-roles.sh:`, and an empty `pg_hba.conf`.
- `ci.yml` has five `Swatinem/rust-cache` steps, and `playwright.yml` has one. The steps at lines 648 (`musl-unwind`) and 45 of `playwright.yml` set `save-if: ${{ github.ref == 'refs/heads/main' }}`. The steps at lines 702, 718, 738, and 784 (`clippy`, `test`, `doc`, `coverage`) have no `with:` block.
- Each job-level `if:` of `ci.yml` has a 4-space indent. The `save-if:` key of a cache step also holds the text `if:`.
- `rust-toolchain.toml` names the channel `stable`. `deploy/Dockerfile` line 17 is `ENV RUSTUP_TOOLCHAIN=1.99.1 \`. A trailing ` \` continues the `ENV` instruction on line 18. `deploy/Dockerfile` line 17 and `deploy/ci/Dockerfile.cc-debian13` set `RUSTUP_TOOLCHAIN=1.99.1`. `Cargo.toml` sets `rust-version = "1.99"`. Every pull request job uses `stable`. The job `compose` builds the image, but not on a pull request.
- `deploy/ci/test_pitr_recovery.py` line 2106 holds `\$` in the plain string `STEP_DOCKER_STUB` (lines 2007-2113). Python 3.14 with `-W error` refuses the file with `SyntaxError: "\$" is an invalid escape sequence`. No other Python file of `deploy/ci/` gives this warning. The same string also holds `\n` escapes on lines 2052, 2056, and 2060, which Python turns into newlines.
- The code lines of `deploy/Caddyfile` and `deploy/Caddyfile.dev` differ in two substituted lines. `diff` of the code lines shows line 4 and line 32. Production has `auto_https disable_certs` and `tls /run/secrets/tls_cert /run/secrets/tls_key`. Development has `skip_install_trust` and `tls internal`. The header of `Caddyfile.dev` names three differences. The header of `Caddyfile` (line 2) names two.

## Goals / Non-Goals

**Goals:**

- Each fix has a test or a check that fails before the fix and passes after it.
- No new rule. Each fix applies a rule that ADR-0031, ADR-0064 (d), ADR-0065, or ADR-0128 (e) already states. The NUL case of D1 is a fourth stop case. Change B adds it to ADR-0031 with an "Amended by" note.

**Non-Goals:**

- F-25. The owner chose option A of "Decided Questions". Change B `review-design-fixes` writes the ADR.
- A change of the secret readers of `recipe-server` or of `deploy/pgtools/`.
- A change of `rust-toolchain.toml`, of the toolchain version, or of the trigger of the job `compose`.

## Decisions

### D1. NUL byte in a secret file of `01-roles.sh` (F-24)

- `check_secret_file` counts the NUL bytes with `tr -cd '\000' <"$path" | wc -c`, as `backup-once` does. The count runs after the read check and before the command substitution.
- A count above 0 calls `fail "$path holds a NUL byte"`. The other checks and messages stay.
- The header comment "A secret file holds exactly one line, without a carriage return" also names the NUL byte.
- The new test `the_script_stops_on_a_secret_file_with_a_nul_byte` in `init_script.rs` writes `pass\0word\n` into one secret file with `printf '%b'`. It uses `assert_stopped`.
- ADR-0031 lists three stop cases: empty, carriage return, and a second line. The NUL byte is a fourth case. ADR-0031 also says that both readers use the same password.
- `recipe-server migrate` refuses only the three cases of ADR-0031 (`crates/recipe-server/src/config/mod.rs` line 221). After D1 the two readers of ADR-0031 refuse different files.
- Alternative: rewrite the whole check with counts only, as `backup-once` does. Rejected. The change is larger, and the other three checks work.

### D2. Rust cache saves only on `main` (F-26)

- The four steps get `with:` and `save-if: ${{ github.ref == 'refs/heads/main' }}`, the exact text of line 651. They keep the default key. A pull request then restores the cache of `main` and saves nothing.
- The new test `deploy/ci/test_rust_cache.py` reads each file of `.github/workflows/`. Each step that uses `Swatinem/rust-cache@` must have that `save-if` line in its `with:` block. A self-test feeds a step without the line and expects a failure, as `test_coverage_workflow.py` does. `check.sh` runs the file, because it runs each `deploy/ci/test_*.py`.
- Alternative: a `shared-key` for all jobs. Rejected. The jobs build different profiles and targets.

### D3. Pull request job with the release toolchain (F-27)

- `deploy/Dockerfile` is the one source of the version (ADR-0064 (d)). The job reads the value there and holds no copy.
- `deploy/ci/rust-tests.sh` gets the case `release-toolchain`. It reads the lines `ENV RUSTUP_TOOLCHAIN=<version>` of `deploy/Dockerfile`. The match accepts a trailing ` \` after the value, as on line 17. It fails unless it finds exactly one value of the form `<n>.<n>.<n>`. The failure comes before the first `rustup` or `cargo` command.
- Then it runs `rustup toolchain install "$version" --profile minimal` and `RUSTUP_TOOLCHAIN="$version" cargo check --release --locked --package recipe-server`. The command follows the `cargo build` line of the image, on the target of the runner.
- The check makes no binary. A fault that only code generation or the linker finds stays with the job `compose` on `main`.
- The new job `release-toolchain` (name "Release toolchain check") in `ci.yml` has no `if:`, so it runs on each pull request. It has `runs-on: ubuntu-latest`, `timeout-minutes: 30`, `persist-credentials: false`, and one `rust-cache` step with `key: release-toolchain` and the `save-if` of D2. It runs `bash deploy/ci/rust-tests.sh release-toolchain`.
- The new test `deploy/ci/test_release_toolchain.py` checks the job and the case.
  - The static part finds the job and its `run` line. It fails when the job has a job-level `if:`, when the step that runs `bash deploy/ci/rust-tests.sh release-toolchain` has an `if:` key, or when the job has a `needs:` key. It also fails when the job or that step has a `continue-on-error:` key. `deploy/ci/check.sh` line 418 applies the same rule to the job `test`. Only a job-level `if:` with a 4-space indent counts, so the `save-if:` key of the cache step does not.
  - A self-test feeds one bad job for each of these keys and expects a failure. It feeds a job with only `save-if:` and expects a pass.
  - The run part copies `rust-tests.sh` into a temporary tree with its own `deploy/Dockerfile`. Fake `rustup` and `cargo` come first on `PATH`.
  - The fakes log their arguments and the value of `RUSTUP_TOOLCHAIN`. There are two good fixtures, `ENV RUSTUP_TOOLCHAIN=1.99.1 \` and `ENV RUSTUP_TOOLCHAIN=1.98.0 \`. The bad cases are no entry, two entries, and the bad version `1.99`.
  - For each good fixture, the fake `rustup` gets `toolchain install <version> --profile minimal`. The fake `cargo` gets the `check` arguments and `RUSTUP_TOOLCHAIN=<version>`. The second version shows that the script holds no fixed value.
  - The three bad cases exit with code 1, and no fake runs.
  - One more case makes the fake `cargo` exit with code 101. The case `release-toolchain` then exits with a code other than 0.
- `CONTRIBUTING.md` gets one sentence about the job, next to the sentence about `ci-scripts`.
- Alternatives:
  - Pin `rust-toolchain.toml` to `1.99.1`. Rejected. `reliability.md` §8 and ADR-0064 keep the channel `stable`. A change needs a new ADR.
  - Run the job `compose` on each pull request. Rejected. ADR-0094 decides against it.
  - Build the musl target in the job. Rejected. A check on the runner target finds the same syntax and API faults in less time. The job `musl-unwind` already builds for musl.

### D4. Escape in the docker stub (F-28)

- Line 2106 gets `\\$` in place of `\$`. Python then holds the same two characters in the string, so the stub does not change.
- An `r` prefix on the whole string is wrong. It changes the `\n` escapes of lines 2052, 2056, and 2060.
- `check.sh` gets one step after the `bash -n` loop. It compiles each `deploy/ci/*.py` with `python3 -I -B -W error` and the built-in `compile`. It writes no `.pyc` file. A `SyntaxWarning` then stops `check.sh`.

### D5. Header of `deploy/Caddyfile` (F-29)

- Line 2 names the three differences: the tls line, the `auto_https` line, and the `skip_install_trust` option. It keeps the sentence form of `Caddyfile.dev` line 2.
- The new test `caddyfiles_differ_only_in_the_named_lines` in `crates/recipe-server/tests/deploy_values/files/caddy.rs` uses `code_lines`. It takes `code_lines` of both files and keeps repeated lines such as `}`. It replaces the permitted line of each pair by one marker per pair. `auto_https disable_certs` and `skip_install_trust` form one pair. `tls /run/secrets/tls_cert /run/secrets/tls_key` and `tls internal` form the other pair. Then it asserts that the two full ordered sequences are equal. It also asserts that line 2 of each file names `tls`, `auto_https`, and `skip_install_trust`. An extra `}` at the end of `deploy/Caddyfile` makes the test fail.

## Risks / Trade-offs

- [The new job adds a check of about 5 minutes to each pull request] → The job uses its own cache key. It checks one package only.
- [`tr` is missing in the database image] → The `postgres` image is Debian based and has `coreutils`. The test of D1 runs in that image. It fails when `tr` is missing.
- [A pull request after a change of `Cargo.lock` finds no matching cache] → The jobs build from zero once. A push to `main` saves the new cache.
- [The job `release-toolchain` is not a required check] → The owner adds it to the branch protection of `main` after the merge. ADR-0128 (e) did the same for `ci-scripts`.

## Migration Plan

No migration. The change touches CI, a test fixture, a comment, and the init script. The init script runs only on an empty volume, so a running cluster does not see the change.

## Decided Questions

F-25 is out of the tasks. The four login role secrets on `db` serve three procedures:

1. The first init: `01-roles.sh` reads them on an empty volume.
2. The restore: `runbook.md` "Restore into production" step 6 sets each password from `/run/secrets/` inside `db` (ADR-0103 (k), ADR-0033). `deploy/ci/switch-steps.sh` does the same.
3. A password change: `runbook.md` "Change a database password" step 8 reads `/run/secrets/<name>` inside `db`.

Compose cannot remove a secret from a running container. Each option below needs a decision of the owner:

- Option A: keep the mounts. A new ADR records the reason and accepts the risk. Only a document changes. A process that controls `db` already owns every role, but it also gets the clear text for a login outside `db`.
- Option B: `compose.yaml` gives `db` only `postgres_password`. An overlay file, for example `deploy/compose.roles.yaml`, adds the four secrets. The operator starts `db` with the overlay for the three procedures and then starts `db` again without it. This needs a new ADR and changes of `runbook.md`, of `reliability.md` §7.11 (the rows `secrets.*_password_file`), of the CI scripts, and of the `deploy_values` tests. A forgotten overlay on the first init stops `01-roles.sh` with "cannot read".
- Option C: a one-shot service sets the passwords over the network as `postgres`. The first init then creates the roles without a password. This needs a new ADR and a `pg_hba.conf` line for `postgres` from the subnet (ADR-0069). It widens the access of the superuser.

The owner picks one option. A later change carries it out.

Decision of the owner (2026-11-13): option A. Change B `review-design-fixes` writes the ADR. This change has no task for F-25.

Decision of the owner (2026-11-13): the job `release-toolchain` needs no ADR. D3 applies ADR-0064 (d) and adds no rule.

1. The NUL case of D1 (F-24). ADR-0031 lists three stop cases, and D1 adds a fourth case to `01-roles.sh` only. No normative document names a NUL byte in a secret file. The owner picks one option:
   - Option A: D1 applies the rule of ADR-0031 that both readers use the same password. No ADR is necessary.
   - Option B: a new ADR, or an "Amended by" note on ADR-0031 in change B, adds the fourth case. The note also says whether `recipe-server migrate` refuses a NUL byte.
   - Task 1.1 stays in the plan.

Decision of the owner (2026-11-14): option B. Change B `review-design-fixes` writes the "Amended by" note on ADR-0031. Task 1.1 can start.

Decision of the owner (2026-11-14): D3 keeps `cargo check --release --locked --package recipe-server`. The job checks and does not build. D3 and task 3.1 test the static rule and the run part as above. D5 and task 5.1 compare the two ordered sequences of code lines with one marker per permitted pair.
