# ADR-0101: Start a PostgreSQL container for the storage tests

Status: Accepted
Date: 2026-10-26
Deciders: project owner

## Context

`reliability.md` section 6 and the code disagree on the database of the storage tests:

- The row "Integration, storage" of `reliability.md` section 6 names two tools. These are `testcontainers` with the pinned `postgres` image, or `DATABASE_URL` of a Compose database in CI.
- No code reads `DATABASE_URL` for the tests. A search of `crates/`, `deploy/`, and `.github/` finds no read of `DATABASE_URL`.
- The function `boot` in `crates/recipe-db/tests/storage/support/container.rs` always starts a container. It uses the image `postgres` with the tag `16.15-bookworm`.
- When Docker is absent, `boot` fails with the message "The storage tests need Docker".
- CI runs the tests in the job `test` on a runner `ubuntu-latest` with Docker. The command is `cargo test --workspace --all-features --locked`.
- No earlier ADR names `testcontainers` or `DATABASE_URL` as a test tool. No accepted ADR needs an amendment.

`reliability.md` section 3.4 lists `DATABASE_URL` among the values that the log never holds. This ADR does not change that rule.

## Decision drivers

- Two sources never disagree. A normative document states what the code does.
- Each run of the tests gets its own database. No test writes to a shared database.
- The decision adds no code before the release.

## Considered options

### Option A: Use `testcontainers` only

- Pros: it matches the code. Each run starts one isolated database from the pinned image. No state remains after the run.
- Cons: the tests need Docker on the host.

### Option B: Add support for `DATABASE_URL`

- Pros: a host without Docker can run the tests against a Compose database.
- Cons: new code and new tests before the release. A test could write to a shared database. The fixtures need a clean database.

### Option C: Keep the disagreement as a known gap

- Pros: no change now.
- Cons: a normative document names a tool that the code does not support.

## Decision

The project uses **Option A**.

- (a) The storage tests of `recipe-db` always start their own PostgreSQL container with `testcontainers` and the pinned `postgres` image.
- (b) The tests do not read `DATABASE_URL`. A Compose database is not a test target.
- (c) A host without Docker cannot run the storage tests. The tests fail with a message that tells the user to start Docker.
- (d) The follow-up edits of this ADR are:
  - `docs/architecture/reliability.md` section 6, row "Integration, storage": the Tool cell names `testcontainers` with the pinned `postgres` image and cites ADR-0101.
  - `docs/adr/README.md`: a new index row and the note "Also 0101" in the decision map row "Deployment, migrations, backup, panic policy, health".
  - No code, no setting, and no number of `reliability.md` section 7.11 changes.

## Consequences

### Positive

- `reliability.md`, the code, and CI agree on the database of the storage tests.
- Each run uses a new database. Tests never share state with a Compose stack.
- No code changes.

### Negative

- The storage tests need Docker. Developers without Docker cannot run them.
- Each run pays the start time of a container.

### Risks and mitigations

- Risk: a reader looks for a way to point the tests at a Compose database. Mitigation: item (b) and the Tool cell of `reliability.md` section 6 state that no such way exists.
- Risk: a later change adds `DATABASE_URL` support without a decision. Mitigation: a new ADR must supersede this one.

## Related

- `docs/architecture/reliability.md` section 3.4 (redaction, unchanged) and section 6 (test levels).
- `crates/recipe-db/tests/storage/support/container.rs` (`boot`, `container_request`).
- `.github/workflows/ci.yml` (job `test`).
