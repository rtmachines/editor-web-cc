# ADR-0055: Navigation guard of the export routes

Status: Accepted
Date: 2026-10-22
Deciders: project owner

## Context

The two export routes of `api.md` section 2.7 are the only `GET` routes that run a command. Each successful export writes one `VersionExported` entry (A4, A8). The CSRF layer skips every `GET` request (`api.md` section 7.3).

`SameSite=Strict` stops the session cookie only on a cross-site request. A page on a sibling host under the same registrable domain is same-site. The browser then sends the cookie with an image, a `no-cors` request, or a navigation. Such a page can run an export in the name of the user. The attacker cannot read the document, because no CORS layer exists. The harm is a false entry in the audit log.

A boosted export link is a second risk. HTMX swaps a `200` answer into `#content` as HTML. A stored text with `<` then becomes live markup in the page, and that markup inherits the CSRF header of `<body>`. ADR-0052 prevents the boost in the markup of the link only.

## Decision drivers

- Each `VersionExported` entry records an export that the user started on the site (A4, A8, R20).
- No export document reaches a page as HTML (R39, ADR-0052).
- No new error code and no change for the other routes.
- A bookmark and a typed address keep their export (`Sec-Fetch-Site: none`).

## Considered options

### Option A: A guard on the export paths with the headers `Sec-Fetch-Site` and `HX-Request`

- Pros: a same-site page and an HTMX request run no export and write no entry. The answer reuses `E-WEB-007` and the event `csrf_rejected`.
- Cons: a browser that sends no `Sec-Fetch-Site` header passes the guard.

### Option B: A CSRF token in the URL of the export link

- Pros: every browser sends it.
- Cons: the token goes into the browser history and the access logs. A bookmark then fails.

### Option C: Accept the risk and record it

- Pros: no code.
- Cons: the audit log can hold an entry that the user did not start.

## Decision

The project uses **Option A**.

- (a) One guard runs before the use case on every path under `/export/v1/`.
- (b) The guard rejects a request with a `Sec-Fetch-Site` header other than `same-origin` or `none`. A request without a `Sec-Fetch-Site` header passes.
- (c) The guard rejects a request with the header `HX-Request: true`.
- (d) A rejected request gets `403 E-WEB-007` as the JSON body of ADR-0053 (a). The guard writes one security event of kind `csrf_rejected` through `SecurityEventSink`. The use case does not run, and the audit log gets no `VersionExported` entry.
- (e) The guard is a layer of the two export routes only. It runs after the session layer and after the gates of the forced password change and of the read-only mode. A request without a valid session therefore still gets `303` to `/login`, and a session with `must_change_password` gets `303` to `/me/password` (ADR-0053 (d)).

## Consequences

### Positive

- A same-site page cannot write a `VersionExported` entry in the name of the user.
- An HTMX request never receives an export document, also when a later change boosts the link.

### Negative

- The sentence of `E-WEB-007` speaks about a form. The user reads it after a rejected export too.
- A browser that sends no `Sec-Fetch-Site` header keeps the same-site risk.

### Risks and mitigations

- Risk: a later change of the layers moves the guard after the use case. Mitigation: a test sends each rejected case to both export routes and checks that the audit log has no new entry.

## Related

- ADR-0014, ADR-0035, ADR-0052, ADR-0053, ADR-0054
- `docs/architecture/api.md` sections 2.7, 5.3, 7.3
- `docs/architecture/reliability.md` section 1.2
- `overview.md` section 7: R20, R39. Section 9: A4, A8
