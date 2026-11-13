# Coverage qualification

## Status on 2026-11-13

- This file is historical evidence for change B. [follow-up.md](follow-up.md) lists the scope of B. Change A does not use this file.
- The task numbers below refer to the old plan. The current `tasks.md` has no branch qualification task.
- The fixtures in `/tmp/cov-qual.F3S85K` and the isolated install in `/tmp/cllvm-0.6.24.PGxJzL` no longer exist.
- `nightly-2025-01-01` (rustc 1.85) cannot build the workspace. The workspace has `rust-version` 1.99 and uses let chains.
- Also, 31 dependencies need a rustc above 1.85.
- Thus no qualified pair exists for the workspace. The fixture result below applies to the fixtures only.
- The owner chose cargo-llvm-cov 0.9.1 with a dated nightly 1.99 or later for the branch leg of change B.

## Task 1.1: branch toolchain qualification

Status: FIXTURE RESULT ONLY, NOT QUALIFIED FOR THE WORKSPACE. The fixture result is in "Update 2026-11-11: older nightly qualified". See the status block above.
The sections from "Required tool" to the first "Decision needed from the owner" record the first state. That state is historical.
Its blocker, a missing cargo-llvm-cov 0.6.24, no longer applies. The isolated install exists. No toolchain pin exists.
Date: 2026-11-11. Repository revision: `4bd52e856d6e5f2abcb9be637307e904b9a7e900`.

### Required tool

`tasks.md` task 1.1 and design D3 require cargo-llvm-cov 0.6.24.
The CI workflow pins the same version (`.github/workflows/ci.yml`, `tool: cargo-llvm-cov@0.6.24`).
Task 1.1 requires the exact tool versions and the qualification result. It does not forbid an install.
The restriction on installs and upgrades came from the original analysis request of the user and from the parent delegation.
This session obeyed that restriction.

### Installed tools

| Item | Value |
|---|---|
| cargo-llvm-cov | 0.9.1 (`/home/rtm/.cargo/bin/cargo-llvm-cov`, the only copy on `PATH`) |
| Stable rustc | 1.99.1 (58cf8f9c3 2026-10-13), LLVM 23.1.1, host `x86_64-unknown-linux-gnu` |
| Cargo | 1.99.1 (5f94df478 2026-08-27) |
| Nightly rustc | 1.101.0-nightly (c980177d8 2026-11-06), LLVM 23.1.5 |
| Other toolchains | `1.95.0`, `1.99` (not inspected further) |

`cargo llvm-cov --help` of 0.9.1 lists `--branch` as unstable.

### Blocker

The installed cargo-llvm-cov is 0.9.1. The required version is 0.6.24.
A run with 0.9.1 qualifies no pair for 0.6.24 and would give no valid pin.
The result of such a run would not satisfy design D3.

### Commands run

```bash
rustc -vV
cargo --version
cargo llvm-cov --version        # cargo-llvm-cov 0.9.1
rustup toolchain list           # stable (active), nightly, 1.95.0, 1.99
rustc +nightly -vV
which -a cargo-llvm-cov
cargo llvm-cov --help
```

The session created no fixture. No Cargo build, test, or coverage run happened.
No JSON shape, counter, time, or disk measurement exists.

### Not checked

- Whether the nightly toolchain has the `llvm-tools` component. The CLI denied the inspection command because of a missing permission. The denial does not show that the component is missing.
- Branch outcomes of the one-arm, two-arm, and branchless fixtures.

### Decision needed from the owner

One of these actions unblocks task 1.1:

1. Install cargo-llvm-cov 0.6.24 into an isolated temporary root and keep the installed 0.9.1 unchanged. For example, run `cargo install cargo-llvm-cov --version 0.6.24 --locked --root "$(mktemp -d)"`. Then call the binary in `<root>/bin` by its full path. Do not install it globally. Check the `llvm-tools` component of the nightly toolchain, and add `llvm-tools-preview` only if it is missing. This session did not run these commands.
2. Amend ADR-0162 and design D3 to qualify cargo-llvm-cov 0.9.1, together with the filename-filter tests of the gate.

Without one of these actions, the branch task stops here, as design D3 requires.

## Update 2026-11-11: isolated install done, execution denied

The owner approved an isolated install of cargo-llvm-cov 0.6.24.
The command `cargo install cargo-llvm-cov --version 0.6.24 --locked --root /tmp/cllvm-0.6.24.PGxJzL` finished with exit 0.
The install used 50 compiled dependencies and 4.4 MB of disk.
Retained root: `/tmp/cllvm-0.6.24.PGxJzL`. The global 0.9.1 copy stays unchanged.
Nightly `1.101.0-nightly (c980177d8 2026-11-06)`, LLVM 23.1.5, has the `llvm-tools` component. The session needs no extra component install.
The permission layer denied `/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov --version`. The session is not interactive.
No fixture ran. Task 1.1 stays unchecked and the blocker above remains until the owner allows that binary to run.

## Update 2026-11-11: fixtures ran, tool failure with the nightly

The owner allowed the binary `/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov`. Its `llvm-cov --version` prints `cargo-llvm-cov 0.6.24`.
The fixtures are in `/tmp/cov-qual.F3S85K`. Each fixture selects its toolchain with a local `rust-toolchain.toml`.
Nightly is `1.101.0-nightly (c980177d8 2026-11-06)`, LLVM 23.1.5. Stable is 1.99.1.

### Fixtures

| Directory | Files | Test |
|---|---|---|
| `one-arm/` | `src/lib.rs` with `pick(flag)` (one `if`/`else`), `src/plain.rs` with `add` (no branch) | `tests/arms.rs` calls `pick(true)` only |
| `two-arm/` | Same two source files | `tests/arms.rs` calls `pick(true)` and `pick(false)` |
| `stable-probe/` | One `lib.rs` with `pick` and one unit test, stable toolchain | Control run without `--branch` |

The `plain.rs` file is the branchless case. A separate branchless measurement could not run, because no report exists (see below).

### Commands and results

Each command ran in its fixture directory. `<dir>` is `/tmp/cov-qual.F3S85K`.

```bash
cargo +nightly generate-lockfile --offline --manifest-path <dir>/one-arm/Cargo.toml     # exit 0
cargo +nightly generate-lockfile --offline --manifest-path <dir>/two-arm/Cargo.toml     # exit 0
/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov llvm-cov --branch --locked --json \
  --output-path <dir>/one-arm-report.json                                                # exit 1
/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov llvm-cov --branch --locked --json \
  --output-path <dir>/two-arm-report.json                                                # exit 1
/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov llvm-cov --json \
  --output-path <dir>/stable-probe-report.json                                           # exit 0, stable
```

| Run | Toolchain | Tests | Result |
|---|---|---|---|
| one-arm, `--branch` | nightly | 1 passed | exit 1: `failed to collect object files: not found object files (searched directories: .../target/llvm-cov-target/debug)` |
| two-arm, `--branch` | nightly | 1 passed | Same error, exit 1 |
| stable-probe, no `--branch` | stable | 1 passed | Report written. Test binary is in `target/llvm-cov-target/debug/deps/` |

The nightly runs print `warning: --branch option is unstable`. No JSON report exists for the nightly runs.
Therefore no JSON shape, no branch counter, and no `not_applicable` or `unmeasured` observation exists. Do not infer any of them.
The files `one-arm-report.json` and `two-arm-report.json` do not exist.
The only report is `/tmp/cov-qual.F3S85K/stable-probe-report.json`. It has no branch data and does not qualify any pin.

### Cause

The nightly writes test binaries to `target/llvm-cov-target/debug/build/covfix/<hash>/out/<name>`.
Stable 1.99.1 writes them to `target/llvm-cov-target/debug/deps/<name>`.
The tool 0.6.24 finds no object files in the nightly layout, although the profile data (`*.profraw`, `.profdata`) exists.
The runs show the layout difference. The statement that it causes the search failure is an inference from the error text and the control run.
The session tried no fixture-local Cargo setting that selects the old layout. The CLI marked `.cargo/config.toml` as a sensitive file and denied the write. 0.6.24 rejects `--config` (`error: invalid option '--config'`).
This fixture attempt installed no toolchain and changed no setting. The earlier section records the isolated install of cargo-llvm-cov 0.6.24.
`rustup toolchain list` shows no other nightly: stable, nightly, 1.95.0, 1.99.

### Time and disk

Wall-clock time was not recorded. The CLI denied `date`, and `time` was not allowed.
The Cargo output reports `Finished` in 0.25 s (one-arm) and 0.21 s (two-arm) for the compile step.
Disk use of `/tmp/cov-qual.F3S85K` after all runs: 39 MB (`du -sh`).

### Result of task 1.1

Only one pair ran: the installed nightly `1.101.0-nightly (c980177d8 2026-11-06)` with cargo-llvm-cov 0.6.24. That pair produced no branch report.
The session did not qualify any other dated nightly. Task 1.1 stays unchecked.
The session wrote no pin. Regions do not replace branches.

### Decision needed from the owner

1. Authorise one older dated nightly with the old layout, installed under a temporary `RUSTUP_HOME`, and repeat the three measurements.
2. Or authorise a fixture-local `.cargo/config.toml` that disables the new layout, if the nightly still has that option. The nightly may have removed it.
3. Or amend ADR-0162 and design D3 to qualify a newer cargo-llvm-cov, together with the filename-filter tests of the gate.

## Update 2026-11-11: older nightly qualified

The owner authorised one older dated nightly in an isolated `RUSTUP_HOME`. The candidate was `nightly-2025-01-01`. No second nightly was tried.
Root: `/tmp/cov-qual.F3S85K`. Raw logs: `logs/` (install, global versions) and `logs2/` (fixtures, repo checks). Drivers: `driver/step1` to `step7`.

### Install

```bash
RUSTUP_HOME=/tmp/cov-qual.F3S85K/rustup-home-nightly-2025-01-01 \
  rustup toolchain install nightly-2025-01-01 --profile minimal --component llvm-tools-preview --no-self-update
```

Exit 0 in 409 s. Components: cargo, llvm-tools, rust-std, rustc.
Compiler: `rustc 1.85.0-nightly (d117b7f21 2024-12-31)`, LLVM 19.1.6, host `x86_64-unknown-linux-gnu`.
Isolated `RUSTUP_HOME` size: 1002520 KiB. Each fixture directory: about 14 MiB.
A later `rustup toolchain list` in that home hung and was killed (exit -15, 219 s). The result does not need it.

### Global state

The files `logs/global_before_*` and `logs/global_after_*` hold `rustup toolchain list`, `rustup --version`, `cargo --version`, `rustc -vV`, `cargo llvm-cov --version`, and `rustup component list --installed`.
Each pair is byte-identical. Global cargo-llvm-cov stays 0.9.1. Global toolchains stay: stable (default), nightly, 1.95.0, 1.99. `~/.rustup/settings.toml` still names `stable` as default.

### Fixtures and command

New fixtures: `one-arm-n25/`, `two-arm-n25/`, `branchless-n25/`. Each has its own `Cargo.lock` (`cargo generate-lockfile --offline`, same SHA-256 prefix `99a22a41b94a352a`).
`one-arm-n25` and `two-arm-n25` have `src/lib.rs` (`pick` with one `if`/`else`) and `src/plain.rs` (`add`). `branchless-n25` has only `plain.rs`.
The tests call `pick(true)` only (one-arm), `pick(true)` and `pick(false)` (two-arm), and `add` only (branchless).
Environment: `RUSTUP_HOME=<isolated>`, `RUSTUP_TOOLCHAIN=nightly-2025-01-01`. Argv, in each fixture directory:

```bash
/tmp/cllvm-0.6.24.PGxJzL/bin/cargo-llvm-cov llvm-cov --branch --locked --json --output-path <dir>/<fixture>-report.json
```

All three runs: exit 0, 0.41 s, 0.28 s, 0.20 s (`time.monotonic`). The test binaries are in `target/llvm-cov-target/debug/deps/`, so the old layout works.
The tool prints `warning: --branch option is unstable`. Reports: `<dir>/*-n25-report.json`.

### JSON shape (observed)

Top keys: `data`, `type` (`llvm.coverage.json.export`), `version` (`2.0.1`), `cargo_llvm_cov` (`{version: "0.6.24", manifest_path}`).
`data[0]` has `files`, `functions`, `totals`. `totals` and each file `summary` have `branches`, `mcdc`, `functions`, `instantiations`, `lines`, `regions`.
A `branches` object has `count`, `covered`, `notcovered`, `percent`. Each file also has a `branches` array. A row is `[line1, col1, line2, col2, true_count, false_count, 0, 0, 4]`.

### Branch results

| Fixture | Total `branches` (count / covered / notcovered) | `lib.rs` array | `plain.rs` branches |
|---|---|---|---|
| one-arm | 2 / 1 / 1 (50 %) | `[4,8,4,12,1,0,0,0,4]`, `[4,8,4,12,0,0,0,0,4]` | 0 / 0 / 0, array `[]` |
| two-arm | 2 / 2 / 0 (100 %) | `[4,8,4,12,1,1,0,0,4]`, `[4,8,4,12,0,0,0,0,4]` | 0 / 0 / 0, array `[]` |
| branchless | 0 / 0 / 0 | no `lib.rs` | 0 / 0 / 0, array `[]` |

The one-arm run reports 1 of 2 outcomes. The two-arm run reports 2 of 2. This is the required distinction.
Lines: one-arm 7/8, two-arm 8/8, branchless 3/3. Regions: one-arm 4/5, two-arm 5/5.
The second array row at the same location has zero counts. The `functions` list has a second `pick` instance with count 0 (the unit-test binary). The summary count is 2, not 4. Use the summary, or merge rows by location. Do not sum rows.

### Branchless and unmeasured states

A branchless file reports `branches` count 0 with an empty array. The branchless `plain.rs` inside the one-arm fixture reports the same data.
A control run of `one-arm-n25` without `--branch` (`one-arm-n25-nobranch-report.json`, exit 0) also reports `branches` count 0 and empty arrays, in the same keys.
Therefore the report alone cannot separate `not_applicable` from `unmeasured`. The gate must take the instrumentation state from run metadata (the `--branch` argv and toolchain), not from the JSON.
Design D3 and task 5.1 must use that rule.

### Pair result

The pair `nightly-2025-01-01` (rustc 1.85.0-nightly d117b7f21, LLVM 19.1.6) with cargo-llvm-cov 0.6.24 gives correct covered and uncovered branch outcomes on the fixtures.
The earlier nightly 1.101 failed with 0.6.24 on the new build layout. No pin file or workflow change was made.
The fixtures give no evidence about the workspace code. The pair needs a run on the real workspace in the later tasks.

### Repository checks (original project toolchain, stable 1.99.1)

| Command | Result |
|---|---|
| `cargo fmt --all --check` | exit 0 |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | exit 0 |
| `openspec validate coverage-measurement-integrity --strict` | exit 0, valid |
| `cargo test --workspace --all-features --locked` | exit 101 |

The test run had 353 passed and 979 failed in the `recipe-db` storage suite. Other suites passed (2137, 77, and 4 passed, 0 failed).
Every sampled failure is `The storage tests need Docker ... failed to create a container`. `docker ps` returns `permission denied ... /var/run/docker.sock`.
The cause is the missing Docker permission of this session. This session changed no source.
The Docker-backed tests did not run, so this required check is not green.

### Open

1. Run `cargo test --workspace --all-features --locked` in a session that can use Docker.
2. Then check the box of task 1.1.
