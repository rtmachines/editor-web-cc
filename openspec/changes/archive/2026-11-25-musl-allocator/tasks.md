# Tasks

Marks: `[opus]` is an ADR text (Opus). `[mechanical]` is a Haiku task. `[deploy]` is a script,
Docker, Compose, or CI task (Codex). `[orchestrator]` is a task of the orchestrator, because it asks
the owner or reads `gh` output. The other tasks are Sonnet tasks.

Run every command from the root of the worktree `/home/rtm/editor-web-cc-musl-allocator`.
Each task takes at most 2 hours.

## Order and shared files

- The proposal did group 1. The plan review and update pass 1 changed its text.
- Task 2.1 comes before task 3.1, so that the real run of the new check fails first.
- Task 2.2 changes `.github/workflows/ci.yml` and `deploy/ci/check.sh`. It runs after task 2.1.
- Task 2.3 changes only `deny.toml`. It can run in parallel with task 2.2.
- Task 2.4 changes `ci.yml`, `deploy/ci/rust-tests.sh`, and `check.sh`. It runs after task 2.2,
  because both change `ci.yml` and `check.sh`.
- Task 2.5 changes the same three files and one new test file. It runs after task 2.4.
- Task 3.1 changes the root `Cargo.toml`, `Cargo.lock`, and `crates/recipe-server/`. It runs after
  tasks 2.1 to 2.5.
- Task 3.2 adds one file to `crates/recipe-server/tests/`. It runs after task 3.1, because the test
  links `mimalloc`.
- Tasks 4.1 to 4.3 change disjoint files. They can run in parallel after task 3.2.
- Group 5 runs after groups 2 to 4. Group 6 needs the approval of the owner. Group 7 comes last.

## 1. ADR and architecture documents (done in the proposal and in update pass 1)

- [x] 1.1 (docs/adr) [opus] Write `docs/adr/0224-mimalloc-on-the-musl-build.md`. It holds the
  evidence of three CI runs, the options A to F, and the reasons against Option B. It also holds the
  items (a) to (q) and the measured numbers. Its status is "Proposed" until task 6.2.
  - Check: `python -I tools/ste-lint/ste-lint.py docs/adr/0224-mimalloc-on-the-musl-build.md`
    reports 0 hard violations.
  - Check: `grep -c '^- (q) ' docs/adr/0224-mimalloc-on-the-musl-build.md` gives 1.
    `grep -c 'mimalloc 3.3.2' docs/adr/0224-mimalloc-on-the-musl-build.md` gives 1.
  - Check:
    `grep -c 'technical reasons against Option B' docs/adr/0224-mimalloc-on-the-musl-build.md` gives
    1.
- [x] 1.2 (docs/adr) Add the note "Amended by ADR-0224" to the status lines of ADR-0064 and ADR-0070
  and to their index rows. Add the index row of ADR-0224 and its entry in the decision map row
  "Deployment, migrations, backup, panic policy, health".
  - Check: `grep -c '0224' docs/adr/README.md` gives 4.
  - Check:
    `grep -c 'ADR-0224' docs/adr/0064-images-and-their-distribution.md docs/adr/0070-container-limits-networks-healthchecks-and-stack-test.md`
    gives 1 for each file.
- [x] 1.3 (docs/architecture) Name the allocator in `reliability.md` §7.1 and §8 and in the row of
  `recipe-server` of `overview.md` §5 (ADR-0224 (b), (e)). Add the clippy step of ADR-0224 (p) to
  §8.
  - Check: the `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.
  - Check: `grep -c 'mimalloc. (musl target only, ADR-0224)' docs/architecture/overview.md` gives 1.
    `grep -c 'ADR-0224 (p)' docs/architecture/reliability.md` gives 1.
  - Check:
    `grep -c 'is the global allocator with its default options (ADR-0224)' docs/architecture/reliability.md`
    gives 1.
- [x] 1.4 (docs/architecture) Name both unwind proofs in `reliability.md` §2.4 rule 1 and in the
  level "Stack" of §6 (ADR-0224 (o), (q)).
  - Check: `grep -c 'ADR-0224 (q)' docs/architecture/reliability.md` gives 2.
  - Check:
    `grep -c 'No CI job proves the unwind of the shipped binary' docs/architecture/reliability.md`
    gives 1.

## 2. CI checks first

- [x] 2.1 (deploy) [deploy] Write `deploy/ci/allocator_rule.py` and
  `deploy/ci/test_allocator_rule.py` (ADR-0224 (c), (g), `design.md` D2).
  - Red test: write the fixture tests first. A musl tree without `mimalloc` fails. A gnu tree with
    `mimalloc` fails.
  - Red test: copy the streams and the exit status of each result of ADR-0224 (g) into the fixtures.
    - The tree goes to stdout with exit status 0.
    - "nothing to print" goes to stderr with an empty stdout and exit status 0. It passes on gnu and
      fails on musl.
    - "did not match any packages" goes to stderr with exit status 101. It passes on gnu and fails
      on musl.
    - Any other error of `cargo tree` fails on both targets.
  - Red test: a `main.rs` fixture without `#[cfg(target_env = "musl")]` above `#[global_allocator]`
    fails. A fixture with two `#[global_allocator]` fails.
  - Red test: a fixture with `static GLOBAL: std::alloc::System = std::alloc::System;` under both
    attributes fails and names the type. The musl tree of that fixture still names `mimalloc`.
  - Red test: a fixture whose static has the type `mimalloc::MiMalloc` and another value fails.
  - Red test: a fixture with a second `cfg` attribute on the static fails, for example
    `#[cfg(debug_assertions)]`. A fixture with the attributes inside a comment fails.
  - Red test: a `tests/mimalloc_unwind.rs` fixture without its `#[global_allocator]` static fails
    and names the file. A fixture without `#![cfg(target_env = "musl")]` fails.
  - Red test: a Compose fixture with `MIMALLOC_PURGE_DELAY=0` fails and names the variable and the
    file. A script fixture with `-e MIMALLOC_PURGE_DELAY=0` fails.
  - Red test: a script fixture with only a comment line that names `MIMALLOC_PURGE_DELAY` passes.
    The scan of `deploy/` skips the script and its test only. It also accepts `mimalloc_unwind` as
    the value of `--test`.
  - The script runs `cargo tree --locked -p recipe-server -e normal -i mimalloc --target <target>`
    for `x86_64-unknown-linux-musl` and `x86_64-unknown-linux-gnu`.
  - Check: `bash deploy/ci/check.sh` exits with 0, so the fixture tests pass.
  - Check: `python3 -I -B deploy/ci/allocator_rule.py` on the real workspace fails and names the
    missing allocator of the musl build. Task 3.1 makes it pass.
- [x] 2.2 (.github/workflows) [deploy] Add the step "Check the allocator of the musl build" to the
  job `deny` after the step of `dependency_rule.py`. The step runs
  `python3 -I -B deploy/ci/allocator_rule.py`.
  - Red test: first add a pin to `deploy/ci/check.sh`. Copy the pattern of `check_secure_refusal`
    and `test_secure_refusal_mutations`.
    - The pin requires exactly one step `run: python3 -I -B deploy/ci/allocator_rule.py` in the job
      `deny`.
    - The step has no `if`, `needs`, or `continue-on-error`.
    - Its negative fixtures delete the step, move it to another job, and add `if: ${{ false }}`.
      `bash deploy/ci/check.sh` fails before the step exists.
  - Check: `bash deploy/ci/check.sh` exits with 0 after the step exists.
  - Check: before task 3.1,
    `cargo tree --locked -p recipe-server -e normal -i mimalloc --target x86_64-unknown-linux-musl`
    exits with 101.
    - Its stderr names "did not match any packages", also without `rustup target add`. Task 3.1
      checks the positive result.
- [x] 2.3 (deny.toml) [mechanical] Add the entry
  `{ crate = "mimalloc", wrappers = ["recipe-server"], reason = "ADR-0224: only the binary crate recipe-server sets the global allocator" }`
  to `[bans] deny` (ADR-0224 (e)).
  - Red test: none, because the task writes no code. The entry has no effect before task 3.1.
  - Check: `grep -c 'crate = "mimalloc", wrappers = \["recipe-server"\]' deny.toml` gives 1.
  - Check: `cargo deny --locked check` passes.
  - Task 3.1 runs the negative check of this entry, because the check needs `mimalloc` in
    `[workspace.dependencies]`.
- [x] 2.4 (deploy) [deploy] Add a mode `musl-clippy` to `deploy/ci/rust-tests.sh` and a step that
  runs it to the job `musl-unwind` (ADR-0224 (p)).
  - The mode runs
    `cargo clippy --target x86_64-unknown-linux-musl -p recipe-server --all-targets --all-features --locked -- -D warnings`.
    The job already sets `CC_x86_64_unknown_linux_musl`.
  - Add `musl-clippy` to the usage line of `rust-tests.sh`.
  - Red test: first add a pin to `check.sh` with the pattern of task 2.2. It requires exactly one
    step `run: bash deploy/ci/rust-tests.sh musl-clippy` in the job `musl-unwind`.
    - Its negative fixtures delete the step, move it to another job, and add `if: ${{ false }}`.
      `bash deploy/ci/check.sh` fails before the step exists.
  - Check: `bash deploy/ci/check.sh` exits with 0. `bash -n deploy/ci/rust-tests.sh` exits with 0.
  - Check: `CC_x86_64_unknown_linux_musl=musl-gcc bash deploy/ci/rust-tests.sh musl-clippy` passes,
    when the host has `musl-tools`.
  - When musl clippy reports a finding that exists before this change, stop and report it. Do not
    change Rust code in this task.
- [x] 2.5 (deploy) [deploy] Add a mode `mimalloc-unwind` to `deploy/ci/rust-tests.sh` (ADR-0224 (q),
  `design.md` D7). Add a step that runs it to the job `musl-unwind`, after the step of
  `musl-clippy`.
  - The mode runs
    `cargo test --target x86_64-unknown-linux-musl -p recipe-server --test mimalloc_unwind --locked`
    twice: once as written and once with `--release`. Add `mimalloc-unwind` to the usage line.
  - Each run must print the line `test result: ok. 1 passed;`, as the mode `secure` does. Any other
    count in a run fails and names the expected count and the profile.
  - Red test: first add a fixture test to `deploy/ci/`. Copy the stub `cargo` of
    `deploy/ci/test_release_toolchain.py`.
    - A stub output with `0 passed` fails. A stub with a failed status fails. A stub with `1 passed`
      passes.
    - The stub fails when the dev run prints `1 passed` and the `--release` run prints `0 passed`.
      It also fails in the reverse case. The stub passes only when both runs print `1 passed`.
  - Red test: add a pin to `check.sh` with the pattern of task 2.2. It requires exactly one step
    `run: bash deploy/ci/rust-tests.sh mimalloc-unwind` in the job `musl-unwind`.
    - Its negative fixtures delete the step, move it to another job, and add `if: ${{ false }}`.
  - Check: `bash deploy/ci/check.sh` exits with 0. `bash -n deploy/ci/rust-tests.sh` exits with 0.
  - Check: the real mode fails until task 3.2 adds the test file.
  - After the first run of the pull request, read the time of the job `musl-unwind`. If it comes
    near `timeout-minutes: 30`, set the timeout from that time, as commit 73a7772 did for the heap
    shards.

## 3. The allocator

- [x] 3.1 (recipe-server) Add `mimalloc` and the static `GLOBAL` (ADR-0224 (a), (b), `design.md`
  D1).
  - Red test: `python3 -I -B deploy/ci/allocator_rule.py` of task 2.1 fails.
  - Add `mimalloc = "0.1.52"` to the root `[workspace.dependencies]`. Add
    `mimalloc = { workspace = true }` to `[target.'cfg(target_env = "musl")'.dependencies]` of
    `crates/recipe-server/Cargo.toml`.
  - Run `cargo metadata --format-version 1 > /dev/null` once without `--locked`, so that cargo adds
    the two packages to `Cargo.lock`.
  - If the lock holds other versions, run `cargo update -p mimalloc --precise 0.1.52` and
    `cargo update -p libmimalloc-sys --precise 0.1.49`.
  - Add the static to `crates/recipe-server/src/main.rs`. Its doc comment names the problem that it
    solves: the allocator of musl slows concurrent saves (R43).
  - The static has no `unsafe` and no `#[allow]`. Do not add it to the library target.
  - Check: while the base is `main` 66cb8e9, `git diff b86126b -- Cargo.lock` prints nothing. So the
    versions, the checksums, and the edge of `recipe-server` equal the measured commit.
  - On another base, compare only the blocks of `mimalloc` and `libmimalloc-sys` and the edge of
    `recipe-server` with b86126b.
  - Check: `git diff Cargo.lock | grep '^+name = '` gives exactly `mimalloc` and `libmimalloc-sys`.
  - Check: `python3 -I -B deploy/ci/allocator_rule.py` reports only the missing file
    `crates/recipe-server/tests/mimalloc_unwind.rs`. Task 3.2 adds that file.
  - Check:
    `cargo tree --locked -p recipe-server -e normal -i mimalloc --target x86_64-unknown-linux-musl`
    names `recipe-server` as the only dependent.
  - Check: `cargo deny --locked check` passes.
  - Check: in a scratch copy of the workspace without `target/`, give `recipe-web` the dependency
    `mimalloc = { workspace = true }`. `cargo deny check bans` then fails and names `mimalloc`.
    Delete the copy.
  - Check: `CC_x86_64_unknown_linux_musl=musl-gcc bash deploy/ci/rust-tests.sh musl-clippy` of task
    2.4 passes, when the host has `musl-tools`. Else the CI job `Panic unwind on musl` runs it.
  - Check: the local checks of the apply guidance in `openspec/config.yaml` pass.
- [x] 3.2 (recipe-server) Write the unwind test `crates/recipe-server/tests/mimalloc_unwind.rs`
  (ADR-0224 (q), `design.md` D7).
  - Red test: before the file exists, the mode `mimalloc-unwind` of task 2.5 fails on a host with
    `musl-tools`. Else the CI job fails. Both runs of the mode (dev and `--release`) must pass after
    the file exists.
  - The file starts with `#![cfg(target_env = "musl")]`. It declares
    `#[global_allocator] static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;`.
  - Its doc comment names the problem that it solves: no other test proves the unwind with the
    allocator of the shipped image (R43).
  - The test has one `#[test]` or `#[tokio::test]` function, so the mode counts `1 passed`.
  - A route allocates a buffer, holds a value whose `Drop` sets an `AtomicBool`, and panics.
    `CatchPanicLayer` wraps the route.
  - The test asserts `500` for that route, the set flag, and `200` for a second request to a normal
    route.
  - Use only the current dependencies and dev-dependencies of `recipe-server`. Add no dependency, no
    feature, and no entry to `deny.toml`.
  - Check: `git diff -- Cargo.toml crates/recipe-server/Cargo.toml deny.toml` adds only the
    `mimalloc` lines of tasks 2.2 and 3.1 and their comments. `git diff b86126b -- Cargo.lock`
    prints nothing.
  - Check: `cargo deny --locked check` passes. `python3 -I -B deploy/ci/allocator_rule.py` passes,
    because the test file holds the required static and the crate `cfg` of its check 3.
  - Check: `cargo test --workspace --all-features --locked` passes on the gnu host. The file
    compiles to an empty crate there.
  - Check: `CC_x86_64_unknown_linux_musl=musl-gcc bash deploy/ci/rust-tests.sh mimalloc-unwind`
    passes, when the host has `musl-tools`. Else the CI job `Panic unwind on musl` runs it.
  - When the test does not compile on musl or an assertion fails, stop and report it to the owner.
    Do not change the allocator or the profile.

## 4. Comments and repeated rules

- [x] 4.1 (deploy) [deploy] Replace line 16 of `deploy/Dockerfile` and of
  `deploy/ci/Dockerfile.cc-debian13` with the text of ADR-0224 (h). Both lines must be equal.
  - Red test: none, because the task changes a comment. The substitution check of `check.sh` fails
    when the two lines differ.
  - Check: `bash deploy/ci/check.sh` exits with 0.
  - Check:
    `grep -c 'libmimalloc-sys also needs it (ADR-0224)' deploy/Dockerfile deploy/ci/Dockerfile.cc-debian13`
    gives 1 for each file.
- [x] 4.2 (CONTRIBUTING.md) [mechanical] Repeat the new rules of `reliability.md` §2 and §8 in
  `CONTRIBUTING.md`.
  - Red test: run the four `grep -c` commands of the checks below first. Each gives 0.
  - In the section "Dependencies", change "bans six crates" to "bans seven crates". Change the full
    stop of the item of `anyhow` to a comma.
  - Add the last item "`mimalloc` outside `recipe-server` (ADR-0224)."
  - In the section "Code conventions", find the rule "No `unsafe`, no `static mut`, and no global
    mutable state". After it, add the sentence on the static `#[global_allocator]` of
    `reliability.md` §8.
  - Add one sentence with a link to ADR-0224 (a): an update that changes the C version of `mimalloc`
    needs one dispatched run before the merge. The sentence links the rule and does not repeat its
    details.
  - Add one sentence: the CI job `musl-unwind` runs `musl-clippy` and `mimalloc-unwind` of
    `deploy/ci/rust-tests.sh`. Both steps need `musl-tools`, so they run in CI only.
  - Add the line `python3 -I -B deploy/ci/allocator_rule.py` to the block of the section "Local
    checks", after the line of `cargo deny`.
  - Check: `grep -c 'bans seven crates' CONTRIBUTING.md` gives 1.
    `grep -c 'mimalloc. outside .recipe-server. (ADR-0224)' CONTRIBUTING.md` gives 1.
  - Check: `grep -c 'deploy/ci/allocator_rule.py' CONTRIBUTING.md` gives 1.
    `grep -c 'mimalloc-unwind' CONTRIBUTING.md` gives 1.
  - Check: `grep -c 'ADR-0224 (a)' CONTRIBUTING.md` gives 1.
  - Check: the `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.
- [x] 4.3 (CHANGELOG.md) [mechanical] Add one entry under the next version: the musl build of
  `recipe-server` uses `mimalloc` as its global allocator (ADR-0224).
  - Red test: `grep -c 'ADR-0224' CHANGELOG.md` gives 0 before the edit.
  - Check: `grep -c 'ADR-0224' CHANGELOG.md` gives 1.
  - Check: the `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.

## 5. Local checks

- [x] 5.1 (workspace) [mechanical] Run each command of the block of the section "Local checks" of
  `CONTRIBUTING.md`, in its order.
  - Red test: none, because the task writes no code.
  - Check: each command of that block exits with 0. The block includes `rust-tests.sh secure`,
    `rust-tests.sh release-toolchain`, `cargo doc`, the coverage commands, `allocator_rule.py`, and
    `ste-lint.py`.
  - Check: `bash deploy/ci/check.sh` and `python3 -I -B deploy/ci/dependency_rule.py` exit with 0.
  - Check: `openspec validate musl-allocator --strict` reports the change as valid.
  - Report the output of each command. When a command fails, stop and report it.

## 6. CI measurement

- [x] 6.1 (deploy) [orchestrator] Ask the owner to approve the commit, the push of the branch, and
  one `workflow_dispatch` of `ci.yml` on it. Do nothing before the approval.
  - The run measures the head commit of the branch, not `main`. Check:
    `gh run view <run> --json headSha --jq .headSha` gives that commit.
  - Read the artifacts `load-musl` and `load-cc-debian13`. Write the peaks of the timings and of the
    flood and the logins per second of each leg.
  - Compare them with the section "Measured numbers" of ADR-0224. When a bound of the job `load`
    fails, stop and ask the owner.
  - For a memory failure, read both legs before a proposal of the fallback of ADR-0224 (j).
  - Check:
    `gh run view <run> --json jobs --jq '.jobs[] | select(.name == "Load (musl)") | .conclusion'`
    gives `success`. This job builds the musl image, so it proves the link of the release binary
    (ADR-0224 (p)).
  - Check: `gh run view <run> --json conclusion --jq .conclusion` gives `success`.
  - Check: the log of the step of `mimalloc-unwind` in the job `Panic unwind on musl` holds the line
    `test result: ok. 1 passed` twice, once for the dev run and once for the `--release` run. A
    failure there applies the stop rule of task 3.2.
    - This includes the jobs `Dependency policy`, `Panic unwind on musl`, `Heap (<shard>)`,
      `Compose stack`, and `Load (cc-debian13)`.
  - Check: `gh run download <run> -n load-musl` gives `load-memory.json`. Its `.timings.peak`,
    `.flood.peak`, and `.peak` are below 536 870 912.
  - Check: in that file, `.timings.oom_kill`, `.flood.oom_kill`, `.timings.RestartCount`, and
    `.flood.RestartCount` are 0.
- [x] 6.2 (docs/adr) [mechanical] Add the run of task 6.1 and its peaks to the section "Measured
  numbers" of ADR-0224. Set its status and its index row to "Accepted".
  - Check: `grep -c '^Status: Accepted' docs/adr/0224-mimalloc-on-the-musl-build.md` gives 1.
  - Check: `grep -c '<run id of task 6.1>' docs/adr/0224-mimalloc-on-the-musl-build.md` gives at
    least 1. Each peak of task 6.1 is in the file.
  - Check: `grep -c 'as the global allocator of the musl build | Accepted' docs/adr/README.md` gives
    1.
  - Check: the `ste-lint.py` command of `CLAUDE.md` reports 0 hard violations.

## 7. Hand-off

- [x] 7.1 (openspec/changes) [orchestrator] After the merge of this change, tell the owner that
  `value-body-memory-load` can resume at its task 3.1.
  - The merge is the condition of this task. Check: `gh pr view <pr> --json state --jq .state` gives
    `MERGED`.
  - The message names the merge commit. It says that the paused change rebases on the new `main`.
  - It says that round 2 with four saves on musl in task 3.1 of that change is the acceptance proof
    of ADR-0224 (m).
  - That proof needs four `303` answers within 30 s, each with a larger `row_version`. Another
    result stops that change, and the owner decides on a new ADR.
  - It says that the musl readings of the value leg with mallocng and with purge delay 0 do not
    describe the shipped image. Run 45574974451 does.
  - Check: the message names the merge commit, task 3.1, ADR-0224 (m), and ADR-0224 (n).

## Workflow follow-up

- Review the plan with `plan-eng-review` before the owner approves this file.
- Review the code with `code-review` after group 5.
- Archive the change with `openspec-archive-change` after the merge checks pass.
- The owner decides when the branch `diag/musl-round2` goes away. It stays until this change merges.
