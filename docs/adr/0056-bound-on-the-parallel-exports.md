# ADR-0056: Bound on the parallel exports

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

`ExportVersion` reads a version, rebuilds the hashed body, compares its hash with the stored hash, and writes the document (ADR-0019, ADR-0029). The rebuild, the hash compare, and the writer run in one `spawn_blocking` task.

`timeout.export` limits the wait of the use case, not the blocking task. Tokio cannot stop a running blocking task. After the limit, the task keeps the rows, the body, and the document in memory until it ends.

Every role with `VersionExport` can start an export, also the Operator. No setting limits the number of export tasks. The pool of `db.pool_size` connections limits only the exports inside `timeout.export`. The `app` container has 512 MiB and 1 CPU (`container.app_memory`, `container.app_cpus`). A version can hold 262 144 leaves (A26).

The diff of two versions does the same kind of work. ADR-0050 (g) limits it with `diff.permits`.

## Decision drivers

- The `app` container stays under its memory limit when many users export at the same time (R30, A26).
- The limit holds also for a task that runs after `timeout.export`.
- No new error code. A timeout keeps `503 E-WEB-009`.
- A timeout writes no `VersionExported` entry (ADR-0029).
- The export uses the same pattern as the diff of ADR-0050 (g).

## Considered options

### Option A: A permit for each export

- Pros: at most `export.permits` export tasks hold rows and a document in memory. The code of the diff gate shows the pattern.
- Cons: an export waits when other exports hold every permit. A wait past `timeout.export` gives `503 E-WEB-009`.

### Option B: Accept the risk and add a load row to step P4.2

- Pros: no code.
- Cons: parallel exports can exceed the memory limit of the container. The load test measures the risk but does not remove it.

### Option C: Stop the running rebuild after the limit

- Pros: no export task runs after the limit.
- Cons: Tokio cannot stop a running blocking task. The option is not possible.

## Decision

The project uses **Option A**.

- (a) At most `export.permits` exports run the read, the rebuild, the hash compare, and the writer at the same time.
- (b) `ExportVersion` waits for a permit inside `timeout.export`, before it opens its transaction. When the limit passes during the wait, the use case returns `503 E-WEB-009` and writes nothing. The decision adds no error code. This point amends ADR-0029, which names the scope of `timeout.export`.
- (c) The blocking task of the rebuild holds the permit and releases it when it ends. Tokio cannot stop a running blocking task, so a task that runs after the limit keeps its permit.
  - A queued task that has not started gets an abort when its future drops, as the diff of ADR-0050 (g) does.
- (d) The permit ends with the blocking task. The response body is outside the permit.
- (e) `reliability.md` section 7.11 gets the row `export.permits` with the value 2. That is the value of `diff.permits` for one CPU. The row "Export generation" of section 7.4 names the wait for the permit.
- (f) `AppSettings` holds the setting. `AppContext::new` builds one gate, and every clone of the context shares it.

## Consequences

### Positive

- At most `export.permits` export tasks hold rows, a body, and a document in memory, also after `timeout.export`.
- An export opens its transaction only with a permit. A burst of exports therefore holds at most `export.permits` connections of the pool for its reads.

### Negative

- An export waits when other exports hold every permit. Under load, a user can get `503 E-WEB-009` for a correct version.
- A task that runs after the limit keeps its permit. A slow export therefore delays the next export.
- The response body of an export stays in memory until the client reads it. The permit does not count that memory.

### Risks and mitigations

- Risk: two parallel exports at the budget of A26 exceed the memory limit of the container. Mitigation: the load test of step P4.2 measures an export at the budget of A26. A lower `export.permits` is a change of the setting only.

## Related

- ADR-0019, ADR-0029, ADR-0050, ADR-0055
- `docs/architecture/reliability.md` sections 7.4 and 7.11
- `overview.md` section 7: R30. Section 9: A26
