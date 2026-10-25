# ADR-0072: Export routes outside the request timeout

Status: Accepted
Date: 2026-10-23
Deciders: project owner

## Context

Two decisions disagree on the export routes:

- ADR-0058 (g) wraps the whole request in `http.request_timeout` (30 s). The commit of the use case is inside this limit.
- ADR-0029 and the row "Export generation" of `reliability.md` section 7.4 say that a timeout never hides a committed `VersionExported` entry.
- The worst case of an export is the pool acquire, the session read, `timeout.export` (10 s), the audit insert, and the commit.
- `db.statement_timeout` (10 s) limits the session read, the audit insert, and the commit. `db.pool_acquire_timeout` (5 s) limits the pool acquire.
- The sum is 5 s + 10 s + 10 s + 10 s + 10 s = 45 s. This sum is above `http.request_timeout`.
- The request timeout can therefore end an export after its commit. The client then gets `503 E-WEB-009`, but the audit log holds a `VersionExported` entry.

## Decision drivers

- A timeout never hides a committed `VersionExported` entry (ADR-0029).
- Every request and every query has a timeout (R30).
- Every timeout has one code, `E-WEB-009` (`reliability.md` section 1.1). No new error code.
- Every number comes from `reliability.md` section 7.11.
- The stop of `app` ends within `app.stop_grace_period` (`reliability.md` section 7.3).

## Considered options

### Option A: The export paths outside the request timeout

- Pros: the guarantee of ADR-0029 stays. No setting changes. The limits of the export already bound each step.
- Cons: the request timeout layer reads the path. An export can run longer than `http.request_timeout`.

### Option B: Keep the limit and accept the residual risk

- Pros: no change of the layer. One limit covers every path.
- Cons: the promise of ADR-0029 must become narrower. A client can get `503 E-WEB-009` for an export that the audit log records.

### Option C: A sum rule in section 7.11 with larger values

- Pros: one limit covers every path, and the guarantee of ADR-0029 stays.
- Cons: `http.request_timeout`, `shutdown.drain`, and `app.stop_grace_period` must grow. Every page then waits longer for its timeout, and the stop of `app` takes longer.

## Decision

The project uses **Option A**.

- (a) The request timeout layer of ADR-0058 (g) calls `next.run` without `tokio::time::timeout` when `is_export_path` is true for the request path. The prefix is `/export/v1/` (ADR-0053 (a)). This item amends ADR-0058 (g) and (h).
- (b) An export request keeps its own limits: `timeout.export` (ADR-0029, ADR-0056), `db.statement_timeout`, `db.lock_timeout`, `db.idle_in_transaction_timeout`, and `db.pool_acquire_timeout`.
  - The header read timeout of ADR-0058 (a) and the body deadline of ADR-0058 (i) stay on every path, also on the export paths.
- (c) No timeout ends an export after its commit. A timeout never hides a committed `VersionExported` entry (ADR-0029).
- (d) The bound of an export is the sum of its own limits, which is above `http.request_timeout`. An export that outlasts `shutdown.drain` gets cut at the end of the drain (ADR-0058 (m)).
  - After a cut, the client gets no answer and no false `503`. A cut before the commit rolls back the open transaction and writes no `VersionExported` entry.
  - The sum rule `http.request_timeout <= shutdown.drain` of `reliability.md` section 7.3 stays for every other path.
- (e) Tests with paused time cover the layer:
  - A request on a page path still gets `503 E-WEB-009` after `http.request_timeout`.
  - A request on an export path whose handler runs past `http.request_timeout` gets the answer of its handler.
  - The JSON body of `E-WEB-009` on an export path stays covered by the tests of `timeout.export`.

R30 asks for timeouts on every request and query. An export request has a limit on each of its steps, so it has a finite bound. The header read timeout, the body deadline, and the drain also apply to it.

## Consequences

### Positive

- A client that gets `503 E-WEB-009` from an export finds no `VersionExported` entry for that request (ADR-0029).
- No setting of section 7.11 changes. A page keeps its timeout of 30 s.
- The stop of `app` keeps its bound, because the drain cuts a long export.

### Negative

- The request timeout layer reads the request path. A wrong prefix check removes the limit from a path or puts it on an export.
- An export can take up to 45 s, which is longer than the timeout of a page.
- An export that starts just before `SIGTERM` can get cut at the end of the drain. The client then gets no answer.

### Risks and mitigations

- Risk: a new slow route under `/export/v1/` loses the request timeout without a decision. Mitigation: the layer uses `is_export_path` of ADR-0053 (a), so the prefix has one owner. A new route under the prefix needs a new decision that names its limits.
- Risk: the layer applies the limit to an export by mistake. Mitigation: the tests of item (e) cover both kinds of path.

## Related

- ADR-0029 (scope of `timeout.export`), ADR-0053 (prefix of the export routes), ADR-0056 (permits of the export), ADR-0058 (request timeout and drain)
- `docs/architecture/reliability.md` sections 7.3 and 7.4
- Baseline requirement: operations "Request timeout"
- `overview.md` section 7: R30
