# ADR-0122: Bounds of the parameter table check and the version page check

Status: Accepted
Date: 2026-11-04
Deciders: project owner

## Context

ADR-0121 makes `ListParameters` read the value of every row of one page. One page can hold up to `page.parameters` rows times `budget.value_bytes` of values (ADR-0121 (f)). The read check of ADR-0118 (b) parses each value and checks it with two rule sets.

ADR-0119 (l) and (m) make the version page of an `Approved` version run the check of the export. That check reads every parameter of the version. A version can hold 262 144 leaves (A26).

The time of both checks grows with the data. A check on an async worker has no await point, so `http.request_timeout` cannot stop it. `reliability.md` section 7.4 gives a limit to the other blocking work, but no ADR gives a limit to these two checks.

ADR-0056 bounds the parallel exports with `export.permits`, but it names only `ExportVersion`. The `app` container has 512 MiB and 1 CPU (`container.app_memory`, `container.app_cpus`).

ADR-0042 (b) states that `GetVersion` reads in one `RepeatableRead` transaction. A wait for a permit inside that transaction breaks the rule of `reliability.md` section 7.4.

## Decision drivers

- No request hangs, and no check blocks an async worker (R30).
- The `app` container stays under its memory limit when many users open `Approved` versions at the same time (R30, A26).
- No new setting and no new error code. The bounds use the keys of `reliability.md` section 7.11.
- Inside a transaction, no use case waits for I/O or for a lock that it does not hold (`reliability.md` section 7.4).
- The check of the version page uses the bounds of the export check that it copies (ADR-0119 (k), (l)).

## Considered options

### Option A: The bounds of the existing blocking work

- Pros: each check runs in `spawn_blocking` with a limit of section 7.11. The version page check shares the gate of the export, so the memory bound of ADR-0056 covers it.
- Cons: many version pages of `Approved` versions delay the exports, and many exports delay those pages. The parameter table takes no permit, so no gate counts the memory of parallel table reads.

### Option B: No bound

- Pros: no code.
- Cons: a long check blocks an async worker, and no limit stops it. No gate counts the memory of parallel checks of `Approved` versions.

### Option C: A new setting for each check

- Pros: each check gets its own limit and its own gate. A version page does not wait for an export.
- Cons: new keys in section 7.11 and a new gate in `AppContext`. No measurement gives the values yet.

## Decision

The project uses **Option A**. This is the decision of the project owner of 2026-11-04. It amends ADR-0056 (a) and ADR-0042 (b).

- (a) `ListParameters` ends its read transaction first. Then it runs the value texts and the read check of ADR-0118 (b) in one `spawn_blocking` task through `Blocking::validate`.
- (b) `timeout.validation` limits that task, and the table takes no permit. A timeout gives `503 E-WEB-009` with the stage `Validation`. A failed task gives `500 E-WEB-011` with the stage `Validation`.
- (c) `GetVersion` reads the header of the version in its own `RepeatableRead` transaction and ends it. A draft and a `Retired` version take no permit and run no check.
- (d) For an `Approved` version, `GetVersion` then waits for a permit of `export.permits` outside any transaction, as `ExportVersion` does (ADR-0056 (b)).
- (e) With the permit, `GetVersion` opens a second `RepeatableRead` transaction. It reads every parameter of the version and ends the transaction. This amends ADR-0042 (b) for an `Approved` version.
- (f) The check of ADR-0119 (l) runs in `spawn_blocking`. The task holds the permit to its end, as ADR-0056 (c) states for the export.
- (g) `timeout.export` limits the wait, the second read, and the task together. A timeout gives `503 E-WEB-009` with the stage `Export`. A failed task gives `500 E-WEB-011` with the stage `Export`.
- (h) At most `export.permits` tasks of `ExportVersion` and of the version page check run at the same time. This widens ADR-0056 (a). The value of `export.permits` does not change.
- (i) The parameters of an `Approved` version do not change (ADR-0009). A UDT revision does not change (ADR-0006). So the second transaction checks the parameters that the page shows.
- (j) The decision adds no setting and no error code. `reliability.md` section 7.4 gets one row for each check. Section 7.11 names the second user of `export.permits`.

## Consequences

### Positive

- No read check runs on an async worker. Each check has a limit of section 7.11.
- At most `export.permits` tasks hold the rows of a whole `Approved` version in memory, also after `timeout.export`.
- No transaction waits for a permit.

### Negative

- Many version pages of `Approved` versions delay the exports, and many exports delay those pages. Under load, a user can get `503 E-WEB-009` on a correct page.
- The version page of an `Approved` version uses two transactions. The state on the page comes from the first transaction. When a user approves a new version between the two, the page can show the old state.
- No gate counts the memory of parallel table reads. Each read can hold up to `page.parameters` times `budget.value_bytes` of values.

### Risks and mitigations

- Risk: parallel table reads of large composite values exceed the memory limit of the container. Mitigation: the load test of step P4.2 measures the table at the budget of A26. A gate for the table is a new ADR.
- Risk: a check that runs after its limit keeps its thread or its permit until it ends. Mitigation: the permit of item (f) counts that task, as ADR-0056 (c) states for the export.

## Related

- ADR-0042 (isolation of the draft queries), ADR-0056 (permits of the export)
- ADR-0118 (read check), ADR-0119 (check of the version page), ADR-0121 (values of the table rows)
- ADR-0006, ADR-0009
- `api.md` sections 2.5 and 2.6, `reliability.md` sections 7.4 and 7.11
- `overview.md` section 7: R30. Section 9: A26
