# Design

## Context

See `proposal.md`, section "Why". ADR-0224 gives the evidence, the considered options, and the
decision. This file links to it and does not repeat it.

- ADR-0064 (d) gives the musl target of the shipped image. ADR-0128 (h) gives the five substitutions
  between `deploy/Dockerfile` and `deploy/ci/Dockerfile.cc-debian13`.
- `reliability.md` §2.1 forbids `unsafe` code in every crate. §8 forbids global mutable state.
- `overview.md` §5 lists the dependencies of each crate. `deny.toml` bans the forbidden edges, and
  `deploy/ci/dependency_rule.py` checks the direct edges.
- The job `Dependency policy` of `.github/workflows/ci.yml` runs `cargo deny --locked check` and
  `dependency_rule.py`.
- `deploy/ci/check.sh` pins the step `rust-tests.sh secure` of the job `test` with negative
  fixtures. The new steps of this change copy that pattern.
- The value leg of the job `load` is not on `main`. It is part of the paused change
  `value-body-memory-load`.

## Goals / Non-Goals

**Goals:**

- The shipped musl binary uses `mimalloc`. Every other build of the binary keeps the system
  allocator.
- A panic unwinds in a musl test binary with `mimalloc`, and CI proves it.
- CI fails when a rule breaks or when a new step leaves `ci.yml`.
- The documents name the new dependency, its scope, and the budgets that it changes.

**Non-Goals:**

- No new setting in `reliability.md` §7.11 and no allocator option.
- No new bound of the job `load`.
- No soak job and no proof of the unwind of the shipped binary.

## Decisions

### D1. The allocator and its scope

ADR-0224 (a) to (d). The table `[target.'cfg(target_env = "musl")'.dependencies]` of
`crates/recipe-server/Cargo.toml` names `mimalloc`. The root `[workspace.dependencies]` holds its
version, as for `rustix`.

The static goes into `main.rs` and not into the library target. So the unit tests of `recipe-server`
and the tests of every other crate keep the system allocator. The heap tests of ADR-0223 thus keep
their counts (ADR-0224 (l)).

Two test binaries on musl use `mimalloc`. The unwind test of D7 declares the static itself. A test
that starts the built binary of `recipe-server` on musl runs that binary with `mimalloc` (ADR-0224
(d)).

`Cargo.lock` holds the versions and the checksums of the measured commit b86126b. So the build uses
the measured C library mimalloc 3.3.2 (ADR-0224 (a)).

### D2. The proof of the link

ADR-0224 (c), (g). A new script `deploy/ci/allocator_rule.py` holds four checks.

1. It runs `cargo tree --locked -p recipe-server -e normal -i mimalloc` for the musl target and for
   the gnu target. It reads stdout, stderr, and the exit status of the three results of ADR-0224
   (g).
2. It runs `cargo metadata` for the musl target twice, with the default features and with
   `--all-features`. It requires the resolved features of `mimalloc` and `libmimalloc-sys` that
   ADR-0224 (g) gives.
3. It reads `crates/recipe-server/src/main.rs`. It requires one `#[global_allocator]` on one active
   static of type `mimalloc::MiMalloc` with the value `mimalloc::MiMalloc`. The only `cfg` attribute
   of the static is `#[cfg(target_env = "musl")]`.
   It also reads `crates/recipe-server/tests/mimalloc_unwind.rs`. That file must hold exactly one
   `#[global_allocator]` static of type and value `mimalloc::MiMalloc`. The crate attribute
   `#![cfg(target_env = "musl")]` must be the only `cfg` of the file. The static needs no own `cfg`.
   It also rejects the shadowing, lint attribute, and `cfg_attr` forms of ADR-0224 (g).
4. It reads the files of `deploy/` and rejects each line outside a comment that names a variable
   whose name starts with `MIMALLOC_` in any letter case. The script and its test are not in that
   scan, because they hold the name as a pattern. The scan accepts `mimalloc_unwind` only as the
   value of `--test` (ADR-0224 (c)).

Check 3 reads both files. Without the pin on the test file, a deleted static leaves the unwind test
of D7 green with the musl allocator. The proof of ADR-0224 (q) is then gone.

The fixture tests in `deploy/ci/test_allocator_rule.py` run in `check.sh`, as every `test_*.py`
does. The job `Dependency policy` runs the script on the real workspace. `check.sh` requires exactly
one step of the script in that job.

Alternative: one more function in `dependency_rule.py`. That script compares the direct edges with
the column "Forbidden" of `overview.md` §5. The allocator rule is a required edge of one target, so
a new script keeps each script to one rule.

Alternative: a Rust test as the link proof. A test binary does not link the static of `main.rs`, so
a Rust test cannot read the allocator of the shipped binary.

### D3. The ban of other edges

ADR-0224 (e). `deny.toml` gets one entry in `[bans] deny`: the crate `mimalloc` with the wrapper
`recipe-server` and the reason ADR-0224. `[graph]` names no target, so the rule covers the musl edge
too.

### D4. The comment of both Dockerfiles

ADR-0224 (h). The same new text goes into line 16 of both files. The substitution check of
`check.sh` then passes without a sixth substitution, so ADR-0128 (h) stays.

### D5. Lints of the musl build

ADR-0224 (p). No other job of a pull request compiles `recipe-server` for musl. `compose` and `load`
run only on `workflow_dispatch` and on a push to `main`.

`release-toolchain` checks `recipe-server` on the gnu host. The job `musl-unwind` already has
`musl-gcc` and the musl target, so it gets the clippy step. Clippy runs the build script of
`libmimalloc-sys` but does not link the release binary. The image builds of `compose` and `load` in
task 6.1 prove that link.

The job `musl-unwind` has a timeout of 30 min. Its new steps compile `recipe-server` and its
dev-dependencies for musl. The first run of the pull request gives the time, and the timeout follows
from it.

Alternative: a new job. It needs the same setup again and gives no other proof.

### D6. Budgets and gates

ADR-0224 (k) to (o).

- The byte counts of ADR-0211 (p) and `reliability.md` §7.8 stay valid.
- The heap pins of ADR-0223 stay valid.
- The bounds of ADR-0128 (y) to (ac) keep their values. Task 6.1 reads the peaks of the timings and
  of the flood again with `mimalloc`. It measures the head commit of the branch before the merge.
- Round 2 with four saves on musl in task 3.1 of `value-body-memory-load` is the acceptance proof of
  ADR-0224 (m). This change cannot run that round, because the value leg is not on `main`.
- That proof needs four `303` answers within 30 s, each with a larger `row_version`. Another result
  stops `value-body-memory-load`, and the owner decides on a new ADR.

### D7. The unwind proof with `mimalloc`

ADR-0224 (o), (q). `crates/recipe-server/tests/mimalloc_unwind.rs` compiles only for
`target_env = "musl"`. It declares `mimalloc::MiMalloc` as its `#[global_allocator]`.

- The test wraps a service that allocates, holds a value with a `Drop`, and panics in `tower_http::catch_panic::CatchPanicLayer`.
- It asserts the answer `500`, the run of the `Drop`, and a second normal answer `200`.
- `mimalloc` is a normal dependency of `recipe-server` on musl. `tower-http` with `catch-panic` is already a dev-dependency. `axum`, `tower`, and `tokio` are already dependencies.
- So the test adds no dependency edge, no feature, and no second wrapper in `deny.toml`.
  `cargo tree` on the branch `diag/musl-round2` shows these edges for the musl target.
- A new mode `mimalloc-unwind` of `deploy/ci/rust-tests.sh` runs the test twice in the job
  `musl-unwind`: once in the dev profile and once with `--release`. It requires the passed count of
  the file in each run, as the mode `secure` does.
- `check.sh` requires exactly one step of the mode in the job `musl-unwind`.

The test runs in the dev profile and in the release profile. It proves the unwind with the allocator
of the shipped image. The gap is that the test binary is not the shipped binary (ADR-0224 (o)).

Alternative: a musl test of `recipe-web` with `mimalloc`. It needs a dev-dependency of `recipe-web`
and a second wrapper in `deny.toml`.

## Risks / Trade-offs

- [Round 1 of the value leg comes near `soak.memory_ceiling`] → ADR-0224 (n). The next change of
  `value-body-memory-load` decides its bound.
- [Each musl number comes from one run] → Task 6.1 reads the job `load` again.
  `value-body-memory-load` repeats the value leg.
- [No soak exists, so no run measured the long-run memory of `mimalloc`] → None in this change.
  ADR-0224 names the risk for the owner.
- [No CI job proves the unwind of the shipped binary] → D7 proves the unwind with `mimalloc` in a
  musl test binary. It runs in the dev profile and in the release profile.
- [The musl test of D7 never compiled] → Task 3.2 stops and reports a compile failure or a failed
  assertion.
- [`cargo tree` in the job `Dependency policy` needs the index of crates.io] → The job already runs
  `cargo deny --locked check`, which reads the same index.
- [A dependency update moves the C library of `mimalloc`] → ADR-0224 (a) requires one dispatched run
  of `ci.yml` before the merge.

## Migration Plan

1. Merge the change. The next image build of `recipe-server` links `mimalloc`.
2. A rollback is a revert of the code commit and a new image. No data, no migration, and no setting
   change.
3. The plant gets the new image with the next release by the procedure of `reliability.md` §7.10.

## Owner decisions

The owner decided these points on 2026-11-25.

- O1. Yes. A CI job proves the unwind with `mimalloc` (D7, ADR-0224 (q)). ADR-0070 (z) gets the note
  "Amended by ADR-0224". `reliability.md` §2.4 rule 1 names both proofs.
- O2. Yes. ADR-0224 states the technical reasons against Option B in its section "Decision".
- O3. Keep the fallback to the glibc image of ADR-0064 (o) and (p), with the failures of ADR-0128
  (ac) for the flood. ADR-0224 (j) adds no trigger. For a memory failure, the decision reads both
  legs first.

Decisions of the owner on the open points of update pass 1 (2026-11-25):

- N1. Decided 2026-11-25. The test of D7 runs in the dev profile and with `--release`. Each run must
  print `1 passed`. The gap is that the test binary is not the shipped binary.
- N2. Decided 2026-11-25. The order stays. Task 6.2 sets ADR-0224 to "Accepted". If the acceptance
  proof of ADR-0224 (m) fails, a new ADR supersedes ADR-0224.
- N3. Decided 2026-11-25. ADR-0224 records the missing soak as a known gap without a mitigation. It
  is a separate topic, because no production server exists yet.
- N4. Decided 2026-11-25. ADR-0224 records the unmeasured host settings as a known gap without a
  mitigation. It is a separate topic, because no production server exists yet.
- N5. Decided 2026-11-25. Task 4.2 adds one sentence to `CONTRIBUTING.md` with a link to ADR-0224
  (a).
