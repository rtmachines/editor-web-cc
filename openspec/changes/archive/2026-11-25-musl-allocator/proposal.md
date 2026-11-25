# Proposal

## Why

On the shipped musl image, four concurrent valid saves of one large parameter form did not answer within 30 s. `http.value_body_permits` (4) admits all four saves. On the glibc control image of run 45563565325, the same saves ended within 5.26 s.

Three CI runs of the throwaway branch `diag/musl-round2` found the cause: the allocator of musl
libc. The musl build then got `mimalloc` as its global allocator. At four concurrent saves, one save
needed 0.73 s of CPU time, and all four saves answered `303`. ADR-0224 gives the numbers.

The change satisfies R30 (no hang under the admitted load), R32 (the project code stays safe Rust),
and R34 (the static image of ADR-0064 stays). It uses Assumption A41 for the unwind proof with
`mimalloc`.

The paused change `value-body-memory-load` resumes at its task 3.1 after this change merges.

## What Changes

- `recipe-server` uses `mimalloc` as its global allocator only for `target_env = "musl"`, with its
  default options.
- Only `recipe-server` depends on `mimalloc`. `deny.toml` bans every other edge.
- A CI check proves that the musl build links `mimalloc` and the gnu build does not. It rejects
  a `MIMALLOC_` variable in `deploy/`.
- The check requires a static of type `mimalloc::MiMalloc` in `main.rs` and in the unwind test.
- The job `musl-unwind` runs clippy on the musl build of `recipe-server`.
- The job `musl-unwind` runs a musl integration test of `recipe-server` with `mimalloc`. The
  test proves that a panic unwinds through `CatchPanicLayer`.
- `deploy/ci/check.sh` requires each new CI step.
- The comment of line 16 in both Dockerfiles names the second crate with C code.
- ADR-0224 records the decision and amends the context and the risk of the allocator of ADR-0064. It amends ADR-0070 (z).
- `reliability.md` §2.4 rule 1 and the level "Stack" of §6 name both unwind proofs.
- `reliability.md` §7.1, §8 and `overview.md` §5 name the allocator.
- One dispatched CI run reads the peaks of the job `load` again.

## Capabilities

### New Capabilities

None.

### Modified Capabilities

- `operations`: two new requirements, "Allocator of the musl build" and "Unwind proof with the
  allocator of the musl build".

## Impact

- Code: `crates/recipe-server/Cargo.toml`, `crates/recipe-server/src/main.rs`,
  `crates/recipe-server/tests/mimalloc_unwind.rs`, the root `Cargo.toml`, `Cargo.lock`.
- Policy: `deny.toml`, `deploy/ci/`, `.github/workflows/ci.yml`.
- Images: `deploy/Dockerfile` and `deploy/ci/Dockerfile.cc-debian13`, comment only.
- Docs: ADR-0224, ADR-0064, ADR-0070, `docs/adr/README.md`, `reliability.md`, `overview.md`,
  `CONTRIBUTING.md`, `CHANGELOG.md`.
- Memory: the musl peaks of `app` rise. Round 1 of the value leg comes near `soak.memory_ceiling`.

## Out of scope

- The bound of round 1 near `soak.memory_ceiling`. The next change of `value-body-memory-load`
  decides it.
- The answer `500 E-DB-008` when an open write transaction stays idle for too long under load.
- An allocator option, for example `MIMALLOC_PURGE_DELAY`.
- A CI proof of the unwind of the shipped binary. The test runs a test binary in the
  dev profile and with `--release`.
- A soak job. No soak exists, and ADR-0224 names the risk.
- The value leg of the job `load`, which `value-body-memory-load` adds.
