# ADR-0054: Only GET on the export routes

Status: Accepted
Date: 2026-10-22
Deciders: project owner

## Context

axum answers a `HEAD` request with the handler of the `GET` route and drops the body. The route table of `recipe-web` therefore gives `HEAD` the row of `GET`. On an export route, a `HEAD` request would run `ExportVersion`. The use case writes one `VersionExported` entry for each successful export (A8), so a `HEAD` request would write an entry for a file that nobody received.

No document says how an export route answers a `HEAD` request.

## Decision drivers

- An audit entry `VersionExported` records an export that the server delivered (A8, R20).
- No new error code and no new behavior of the router for other routes.

## Considered options

### Option A: Only `GET`. `HEAD` gets the answer of an unlisted method

- Pros: no use case runs and no entry exists. The router answers as for every other unlisted method.
- Cons: the text `404` has no code (ADR-0053 (g)).

### Option B: `405` with `Allow: GET`

- Pros: the correct status of HTTP.
- Cons: a new behavior of the router for two routes only.

### Option C: `HEAD` runs the export

- Pros: the default of axum.
- Cons: an audit entry for a file that nobody received.

## Decision

The project uses **Option A**.

- (a) The two export routes accept only `GET`. A `HEAD` request on an export path gets the answer that the router gives for a method that the route table does not list.
- (b) The use case does not run for a `HEAD` request, and the audit log gets no entry.
- (c) Every other route keeps the rule that `HEAD` uses the row of `GET`.

## Consequences

### Positive

- Each `VersionExported` entry belongs to a `GET` request that received the document.

### Negative

- A client cannot read the headers of an export without a download.

### Risks and mitigations

- Risk: a later change of the route table gives `HEAD` the `GET` row again. Mitigation: a test sends `HEAD` to both export routes and checks that the audit log has no new entry.

## Related

- ADR-0019, ADR-0053
- `docs/architecture/api.md` section 2.7
- `overview.md` section 7: R20. Section 9: A8
