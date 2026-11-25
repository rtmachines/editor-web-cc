# ADR-0224: `mimalloc` as the global allocator of the musl build

Status: Accepted
Date: 2026-11-25
Deciders: project owner

## Context

ADR-0064 (d) builds the shipped image of `recipe-server` for the target `x86_64-unknown-linux-musl`.
ADR-0064 named one risk of that target: the allocator of musl can be slower.

- No crate sets a `#[global_allocator]` for the server. The musl build thus uses the allocator of
  musl libc, mallocng.
- The only `#[global_allocator]` of the workspace is `dhat::Alloc` in
  `crates/recipe-web/src/test_support/heap.rs`. It exists only in a test build with
  `--cfg heap_measure` (ADR-0223 (s)).
- The context of ADR-0064 says that only the crate `ring` builds C code. The comment of line 16 in
  `deploy/Dockerfile` and `deploy/ci/Dockerfile.cc-debian13` says the same.
- `app` has `container.app_cpus` of 1 and `runtime.worker_threads` of 2 (`reliability.md` §7.8,
  §7.11).

The paused change `value-body-memory-load` added a value leg to the job `load`. Round 1 sends four value bodies at the same time, and the server refuses each form (ADR-0223 (v)). Round 2 sends four concurrent valid saves of one large parameter form. `http.value_body_permits` (4) admits all four.

- In round 2 on the musl image, no save answered within the client deadline of 30 s. One transaction
  stayed idle for more than `db.idle_in_transaction_timeout`.
- On the `cc-debian13` image with glibc, the four saves of run 45563565325 ended within 5.26 s.
- The two images use the same scripts, the same Compose limits, and the same worker count. Only the
  C library and its allocator differ.

Three CI runs on the throwaway branch `diag/musl-round2` measured the allocator. The section
"Measured numbers" gives the values.

- Run 45563565325 is the baseline with mallocng.
- Run 45574974451 has `mimalloc` as the global allocator of the musl build only. The `cc-debian13`
  leg is the control.
- Run 45581066118 has `mimalloc` and the environment variable `MIMALLOC_PURGE_DELAY=0` in `app`.
- Each musl configuration ran once. The repeatability of each musl value is thus unknown. The glibc
  leg ran in all three runs.
- Only the allocator of the server changed between the two musl runs. Commit b86126b of the second
  run also adds the fields `user_usec` and `system_usec` to each reading of `cpu.stat`.
- So the allocator causes the slowdown of the musl build.
- The mechanism inside mallocng is not measured. Lock contention in `malloc` and `free` is a likely
  cause. The evidence does not record the version of musl.

The measured commit b86126b and `main` 66cb8e9 differ in the value leg and in the probe. Outside the
test files, `deploy/ci/`, `.github/`, `docs/`, and `openspec/`, only three files differ (`git diff
--name-only 66cb8e9 b86126b`).

- The three files are `Cargo.lock`, `crates/recipe-server/Cargo.toml`, and
  `crates/recipe-server/src/main.rs`.
- They hold the probe: the dependency `mimalloc` for `target_env = "musl"` and the static `GLOBAL`.
  No migration, no Compose file, and no Dockerfile differs.

## Decision drivers

- Four concurrent valid saves that the permits admit answer within `http.request_timeout` on the shipped image (R30).
- The shipped image stays the static image of ADR-0064 Option A (R34).
- The project code stays safe Rust under `#![forbid(unsafe_code)]` (R32).
- `memory.peak` of `app` stays below `soak.memory_ceiling` (R30, R31).

## Considered options

### Option A: `mimalloc` as the global allocator of the musl build, with default options

- Pros: measured. At four concurrent saves, the CPU time of one save falls from at least 7.85 s to
  0.73 s, and all four saves answer `303`.
- Pros: the image, the target, and the base image of ADR-0064 stay.
- Cons: a second crate builds C code with `musl-gcc`. Round 1 `memory.peak` rises from 286 150 656
  to 370 139 136 bytes in one run.

### Option B: the image of ADR-0064 Option B, `cc-debian13` with glibc

- Pros: measured as the control in three runs. At four concurrent saves, one save needs 0.91 s to
  1.25 s of CPU time, and all four saves answer `303`.
- Pros: no new crate.
- Cons: the cons of ADR-0064 Option B. ADR-0064 (p) needs a new ADR that names the new base image.
- Cons: at four concurrent saves, one save needs more CPU time than with `mimalloc`. Three glibc
  runs gave 0.91 s to 1.25 s against 0.73 s and 0.76 s with `mimalloc`.

### Option C: `http.value_body_permits` of 1 on musl, or a gate of one permit around the parse and the diff

- Pros: four saves in sequence need about 4 × 1.32 s of CPU time, which is inside 30 s.
- Cons: the change of a setting of `reliability.md` §7.11, or a setting for one target only.
- Cons: one save on musl still needs 1.32 s of CPU time, against 0.75 s with Option A.
- Cons: the gate treats the value routes only. Other paths also allocate. With Option A,
  `GetVersion` of the job `load` falls from about 0.9 s to 0.47 s.
- Cons: a gate of 2 permits is not safe. Two saves on musl answered after 14.7 s, near
  `db.idle_in_transaction_timeout` of 15 s.

### Option D: the typed parse and the diff in `spawn_blocking`

- Pros: the Tokio workers stay free for the timers and the other tasks.
- Cons: the work moves to other threads, but its CPU time does not fall. The blocking threads use
  the same allocator.
- Cons: the write transaction stays open during the CPU work. The change needs a new rule in
  `reliability.md` §7.4.

### Option E: more CPU for `app`

- Cons: it hides the limit of the shipped image on the host of ADR-0070 (m).
- Cons: it changes `container.app_cpus` of `reliability.md` §7.11. No run measured it.

### Option F: Option A with `MIMALLOC_PURGE_DELAY=0`

- Pros: round 1 `memory.peak` falls from 370 139 136 to 351 600 640 bytes in one run.
- Cons: the fall of 18 538 496 bytes is inside the spread of glibc. Three glibc runs of round 1
  spread over 47 308 800 bytes.
- Cons: the system CPU time rises: in round 1 from 0.12 s to 0.42 s, at four saves from 0.07 s to
  0.25 s.
- Cons: one more setting outside `reliability.md` §7.11.

## Decision

The owner chose **Option A** on 2026-11-25. The owner gives these technical reasons against Option
B:

- Option A keeps the static image of ADR-0064 Option A (R34). It thus needs no new ADR for a base
  image (ADR-0064 (p)).
- At four concurrent saves, one save needed less CPU time with `mimalloc`. It needed 0.73 s with
  default options and 0.76 s with purge delay 0. Three glibc runs gave 0.91 s to 1.25 s.
- The change adds one dependency on one target. One revert undoes it.

The allocator:

- (a) `crates/recipe-server` depends on `mimalloc` only in the table
  `[target.'cfg(target_env = "musl")'.dependencies]`. The crate uses the default features of
  `mimalloc`.
  - The probe used `mimalloc` 0.1.52 and `libmimalloc-sys` 0.1.49. `Cargo.lock` holds these two
    versions and the checksums of commit b86126b.
  - These default features build the C library mimalloc 3.3.2: the directory `v3` of
    `libmimalloc-sys`, with the feature `v2` off.
  - The job `load` does not run on a pull request. An update that changes the version of the C
    library thus needs one dispatched run of `ci.yml` before the merge.
- (b) `crates/recipe-server/src/main.rs` holds the one static `GLOBAL` of type `mimalloc::MiMalloc`
  with `#[cfg(target_env = "musl")]` and `#[global_allocator]`.
  - The static needs no `unsafe` and no `#[allow]`. It compiles under `#![forbid(unsafe_code)]`.
  - The library target of `recipe-server` does not set it. So the unit tests of the crate and the
    tests of the other crates keep the system allocator.
  - Item (q) gives the one test binary that declares the same static.
- (c) The allocator runs with its default options. No Compose file and no script sets or passes an
  environment variable whose name starts with `MIMALLOC_` in any letter case. `reliability.md` §7.11
  gets no new setting.
  - `mimalloc` reads its `MIMALLOC_` variables from the environment of the process at the start. A
    CI check rejects such a variable in `deploy/`.
  - The check skips a comment line, so a comment can name the variable.
  - The check accepts `mimalloc_unwind` only as the value of `--test`. The forms are `--test x`,
    `--test=x`, and `--test` with a quoted `x`. The unwind test of item (q) uses that name.
- (d) The binary of `recipe-server` uses `mimalloc` on each target with `target_env = "musl"`.
  `x86_64-unknown-linux-musl` is the target of the shipped image.
  - A build of the binary for another C library keeps the system allocator. This includes the
    `cc-debian13` image and the host builds.
  - The heap tests of ADR-0223 (s) use `dhat::Alloc` on the gnu host.
  - On musl, a test that starts the built binary of `recipe-server` runs that binary with
    `mimalloc`.

The dependency policy:

- (e) Only `recipe-server` depends on `mimalloc`. `deny.toml` gets one entry in `[bans] deny` with
  the wrapper `recipe-server`. `overview.md` §5 names the crate in the row of `recipe-server`.
- (f) `mimalloc` and `libmimalloc-sys` have the license MIT. `deny.toml` already permits it.
- (g) A CI check reads `cargo tree -i mimalloc` of `recipe-server` for the musl target and for the
  gnu target.
  - On musl, the output names `mimalloc` and `recipe-server` as its dependent. On gnu, the output
    names no `mimalloc`.
  - `cargo tree` has three results. It writes the tree to stdout and exits with 0.
  - Or it writes "nothing to print" to stderr and exits with 0, when the lock holds the crate for
    another target only. Its stdout is then empty.
  - Or it writes "did not match any packages" to stderr and exits with 101, when the lock has no
    `mimalloc`. On musl, only the first result passes.
  - On gnu, the second result and the third result pass. Every other error of `cargo tree` fails the
    check. The check thus reads both streams and the exit status.
  - The same check reads `crates/recipe-server/src/main.rs`. It requires one `#[global_allocator]`
    on one active static.
  - That static has the type `mimalloc::MiMalloc` and the value `mimalloc::MiMalloc`. Its only `cfg`
    attribute is `#[cfg(target_env = "musl")]`.
  - The same check reads `crates/recipe-server/tests/mimalloc_unwind.rs`. It requires exactly one
    `#[global_allocator]` static of type and value `mimalloc::MiMalloc`.
  - The crate attribute `#![cfg(target_env = "musl")]` is the only `cfg` of that file. The static
    needs no own `cfg`.
  - The check of both files also rejects each form that can bypass the pin:
    - A `mod`, an `as` alias, a `use` item, or a glob import that can shadow the crate `mimalloc`.
    - A `#[allow]` or a `#[expect]` on the static.
    - A `cfg_attr` that names `global_allocator`.
  - The same check runs `cargo metadata --filter-platform x86_64-unknown-linux-musl` twice. The runs
    use the default features and `--all-features`.
  - Each run requires the resolved features of `mimalloc` to be exactly `["default"]`. The `default`
    list of its manifest must be empty.
  - Each run requires the resolved features of `libmimalloc-sys` to be `[]`. A feature of
    either crate thus fails the check.
- (h) Line 16 of `deploy/Dockerfile` and of `deploy/ci/Dockerfile.cc-debian13` gets one new text.
  The text is true for both targets, so the five substitutions of ADR-0128 (h) stay.
  - The new text:
    `# The crate ring (C and assembly) needs the C compiler of CC_. On musl, libmimalloc-sys also needs it (ADR-0224).`

ADR-0064 and its fallback:

- (i) This ADR amends the context of ADR-0064: `ring` and, on musl only, `libmimalloc-sys` build C
  code. It also amends the risk "the allocator of musl fails the soak" of ADR-0064. The allocator of
  the musl image is now `mimalloc`.
- (j) ADR-0064 (n), (o), and (p) keep their text, as ADR-0128 (ac) amends (o). This ADR adds no
  trigger of the fallback to Option B.
  - For a memory failure, the decision reads both legs before it applies the fallback.
    The glibc runs did not consistently give a lower round 1 peak than `mimalloc`.

Budgets and gates:

- (k) ADR-0211 (p) and `reliability.md` §7.8 count buffered bytes: 4 × 48 MiB of value bodies and 4
  × 19 MiB of Argon2 memory. The allocator does not change these counts, so they stay valid.
  - The memory that the allocator keeps on top of them is not in these counts. `memory.peak` shows
    it.
- (l) The heap tests of ADR-0223 (s) and (t) count the bytes that the code requests through
  `dhat::Alloc`. They run on the gnu host and never link `mimalloc`. Their pins stay valid.
  - ADR-0223 (r) keeps its text. Its sentence is a hedge on resident memory, and `memory.peak`
    measures that memory on the shipped image.
- (m) The bounds of the job `load` of ADR-0128 (y) to (ac) keep their values: no OOM kill, no
  restart, and a peak below `container.app_memory`.
  - The readings with mallocng no longer describe the shipped image. One dispatched run of `ci.yml`
    reads the peaks of the timings and of the flood with `mimalloc` again.
  - Round 2 with four saves on musl in task 3.1 of `value-body-memory-load` is the acceptance proof
    of this ADR.
  - That proof needs four `303` answers within 30 s, each with a larger `row_version`. Another
    result stops that change, and the owner decides on a new ADR.
  - Task 6.2 sets this ADR to "Accepted" before that proof runs. If the acceptance proof fails, a
    new ADR supersedes this ADR.
- (n) Round 1 of the value leg comes near `soak.memory_ceiling`. The margin falls from 116 502 528
  bytes with mallocng to 32 514 048 bytes with `mimalloc` in one run.
  - The change `value-body-memory-load` measures the value leg again from its task 3.1. Its next
    change decides the bound of round 1.
- (o) The CI job `musl-unwind` gives two proofs of the unwind on the musl target.
  - The panic tests of `recipe-web` unwind with the allocator of musl (ADR-0064 (m), ADR-0070 (z)).
  - The test of item (q) unwinds with `mimalloc`. It runs in the dev profile and in the release
    profile.

  `deploy/ci/check.sh` pins `panic = "unwind"` in `[profile.release]` of the root `Cargo.toml`.
  Cargo ignores `panic` for test targets, so the release run of the test cannot prove that line.

  No CI job proves the unwind of the shipped binary of `recipe-server`, because every proof runs in
  a test binary. Section "Risks and mitigations" names this gap.

Lints of the musl build:

- (p) Clippy of CI runs on the gnu host. So no other job of a pull request compiles the code under
  `#[cfg(target_env = "musl")]`.
  - The CI job `musl-unwind` already installs `musl-tools` and the musl target. It gets one more
    step: clippy on `recipe-server` for `x86_64-unknown-linux-musl`, with all targets, all features,
    and `-D warnings`.
  - The step runs the build script of `libmimalloc-sys`, so it compiles the C code with `musl-gcc`.
    Clippy does not link the release binary.
  - The image builds of the jobs `compose` and `load` link the release binary. These jobs run on
    `workflow_dispatch` and on a push to `main`.

The unwind proof with `mimalloc`:

- (q) `crates/recipe-server/tests/mimalloc_unwind.rs` is an integration test that compiles only for
  `target_env = "musl"`. It declares `mimalloc::MiMalloc` as its `#[global_allocator]`.
  - The check of item (g) pins that static. Without the pin, a deleted static leaves the test green
    with the musl allocator.
  - It wraps a service that allocates and then panics in `tower_http::catch_panic::CatchPanicLayer`. It asserts the answer `500`, the run of a `Drop`, and a second normal answer.
  - It uses only the dependencies and the dev-dependencies of `recipe-server`. So it adds no
    dependency edge and no second wrapper to `deny.toml`.
  - A mode of `deploy/ci/rust-tests.sh` runs the test in the job `musl-unwind` twice: once in the
    dev profile and once with `--release`. The mode fails when the count of passed tests of a run
    differs from the count of the file.
  - `deploy/ci/check.sh` requires exactly one step of the mode in the job `musl-unwind`.

## Measured numbers

CPU time of one save is the difference of `usage_usec` of `cpu.stat` of `app` after and before the
round. The table divides that difference by 1 000 000 and by the count of concurrent saves.

| Build and run | 1 save | 2 saves | 4 saves | Answers at 4 saves | Answer times at 4 saves |
|---|---|---|---|---|---|
| musl, mallocng, 45563565325 | 1.32 s | 7.34 s | at least 7.85 s | 3 × `503 E-WEB-009`, 1 × `500 E-DB-008` | 31.23 s to 31.42 s |
| musl, `mimalloc`, 45574974451 | 0.75 s | 0.81 s | 0.73 s | 4 × `303` | 2.81 s to 3.00 s |
| musl, `mimalloc`, purge delay 0, 45581066118 | 0.91 s | 0.76 s | 0.76 s | 4 × `303` | 2.82 s to 3.16 s |
| glibc, `cc-debian13`, 45563565325 | 0.80 s | 0.97 s | 1.25 s | 4 × `303` | 4.34 s to 5.26 s |
| glibc, `cc-debian13`, 45574974451 | 0.81 s | 0.84 s | 1.07 s | 4 × `303` | 3.96 s to 4.54 s |
| glibc, `cc-debian13`, 45581066118 | 0.69 s | 0.78 s | 0.91 s | 4 × `303` | 3.11 s to 3.81 s |

With `mimalloc` and default options, the system CPU time of each reading is 0.02 s to 0.12 s.

`memory.peak` of `app` in bytes. Each glibc cell holds the runs 45563565325, 45574974451, and
45581066118 in that order.

| Reading | musl, mallocng | musl, `mimalloc` | musl, `mimalloc`, purge delay 0 | glibc, three runs |
|---|---|---|---|---|
| Timings of the job `load` | 72 499 200 | 125 546 496 | 98 557 952 | 167 686 144, 167 690 240, 187 973 632 |
| Login flood | 109 608 960 | 139 706 368 | 139 112 448 | 110 088 192, 110 071 808, 109 129 728 |
| Round 1, four refused forms | 286 150 656 | 370 139 136 | 351 600 640 | 355 192 832, 402 501 632, 372 678 656 |
| Four concurrent saves | 251 289 600 | 250 650 624 | 269 725 696 | 268 722 176, 275 013 632, 265 003 008 |

- `soak.memory_ceiling` is 384 MiB, which is 402 653 184 bytes. `container.app_memory` is 512 MiB.
- The login flood gave 28.8 logins per second in run 45563565325 and 30.0 in run 45485053543 with
  mallocng.
- It gave 43.1 with `mimalloc` and 35.4 with purge delay 0. Four glibc runs gave 28.3 to 46.5. The
  difference is thus inside the spread.
- `GetVersion` of the job `load` took 0.90 s in run 45485053543 and 0.91 s in run 45563565325 with
  mallocng. It took 0.47 s with `mimalloc`. Four glibc runs gave 0.42 s to 0.59 s.
- `ApproveVersion` took 2.40 s in run 45485053543 and 2.43 s in run 45563565325 with mallocng. It
  took 1.25 s with `mimalloc`. Four glibc runs gave 1.16 s to 1.50 s.
- Run 45485053543 of the paused change also gave the fourth glibc values.

Run 45657355507 measured commit 13ba427 of this change. It ran once.

- The job `Load (musl)` passed. `memory.peak` of `app` was 126 636 032 bytes for the timings and
  142 409 728 bytes for the login flood. No OOM kill and no restart occurred.
- The musl leg gave 42.3 logins per second. `GetVersion` took 0.47 s, `ApproveVersion` 1.23 s, and
  `CreateVersion` 1.01 s.
- The job `Load (cc-debian13)` passed. Its peaks were 188 039 168 bytes for the timings and
  109 502 464 bytes for the login flood.
- The glibc leg gave 40.2 logins per second. `GetVersion` took 0.42 s, `ApproveVersion` 1.14 s, and
  `CreateVersion` 1.00 s.
- The job `Panic unwind on musl` passed in 8 minutes. The test of item (q) passed in the dev profile
  and in the release profile.

## Consequences

### Positive

- Four concurrent saves on the shipped image answer `303`, with 0.73 s of CPU time for each save in
  one run.
- The image, the target, and the base image of ADR-0064 stay.
- The project code stays safe Rust. The `unsafe` code of the allocator is in the dependency, as the
  C code of `ring` is.

### Negative

- A second crate builds C code. The musl build compiles it with `musl-gcc`.
- The peaks of `app` on musl rise. Round 1 rises by 83 988 480 bytes in one run.
- The project watches one more dependency for advisories with `cargo deny` and `cargo audit`.
- The gnu builds and the musl build use different allocators. Only the test of item (q) runs with
  the allocator of the shipped image, in the dev profile and in the release profile.

### Risks and mitigations

- Risk: round 1 of the value leg reaches `soak.memory_ceiling`. Mitigation: item (n). The next
  change of `value-body-memory-load` decides the bound.
- Risk: each musl value comes from one run. Mitigation: item (m) reads the job `load` again, and the
  change `value-body-memory-load` repeats the value leg.
- Risk: no soak exists. The section "Known gaps" of `docs/ops/runbook.md` says that the repository
  holds no soak script and no nightly job.
  - So no run measured the long-run memory of `mimalloc` on a host that runs 24/7. Mitigation: none
    in this ADR. The owner decides.
- Risk: no CI job proves the unwind of the shipped binary with `mimalloc`. Mitigation: item (q)
  proves the unwind with `mimalloc` in a musl test binary, in the dev profile and in the release
  profile. The test binary is not the shipped binary.
- Risk: the plant host can have other settings of `vm.overcommit_memory` and of transparent huge
  pages than the GitHub runner. `mimalloc` reserves large virtual areas.
  - No run measured these settings, so `memory.peak` on the plant host can differ from CI.
    Mitigation: none in this ADR.
  - This is a known gap. It is a separate topic, because no production server exists yet.
  - The same applies to the missing soak. This is a known gap without a mitigation.
- Risk: an update of `mimalloc` or `libmimalloc-sys` changes the measured C library. Mitigation:
  item (a) requires a dispatched run before the merge.
- Risk: an environment variable `MIMALLOC_` changes the allocator. Mitigation: item (c).

## Related

- ADR-0064 (d), (m), (n), (o), (p), amended by this ADR in its context and its risk of the allocator
- ADR-0070 (z): the CI job `musl-unwind`, amended by item (o) of this ADR
- ADR-0128 (h), (y) to (ac): the job `load` and the substitutions of the `cc-debian13` file
- ADR-0211 (p): the buffered value bodies
- ADR-0223 (r), (s), (t): the heap tests
- `reliability.md` §2.1, §2.4, §6, §7.1, §7.8, §7.11, §8
- `overview.md` §5
