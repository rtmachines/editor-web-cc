# ADR-0057: Panic containment layers

Status: Accepted. Amended by ADR-0092 (item (h): a sixth place, the `JoinSet` of the webhook calls)
Date: 2026-10-23
Deciders: project owner

## Context

`reliability.md` section 2.4 rule 3 says that `tower_http::catch_panic::CatchPanicLayer` is the outermost layer of the router. The same rule asks for a `500 E-WEB-010` answer with the `request_id`.

Step P4.1 wires the layer, and the plan found these gaps:

- The panic handler of `tower-http` 0.7.1 gets only the panic payload. It gets no request, so it cannot read the `request_id` or the path.
- An outermost layer catches the panic after the unwind left every inner layer. The answer then has no `X-Request-Id`, no security header, and no request id in the body.
- An outermost layer cannot give the JSON body of `api.md` section 5.3 on a path under `/export/v1/`. ADR-0053 (a) requires that body for every error answer of the export routes.
- A layer directly inside `render_errors` gives the full answer. It cannot catch a panic of `request_id`, `TraceLayer`, the header layers, or `render_errors`.
- One caught panic gives two `ERROR` lines: the event of the panic hook (rule 2) and the line of `E-WEB-010`. Section 1.1 says that an error is logged exactly once. ADR-0020 says that the layer logs at `ERROR`.
- Rule 6 counts three places that catch a panic. A second layer instance is one more place.

## Decision drivers

- A panic in one request does not stop the service (R30, A41).
- The answer of a caught panic holds the code and the `request_id`, so the operator finds the log line (R36).
- The export routes answer with JSON (ADR-0053 (a)).
- No global mutable state (`reliability.md` section 8).
- No new error code.

## Considered options

### Option A: One layer inside `render_errors` and a backstop inside `request_id`

- Pros: the inner layer gives the page, the HTMX fragment, or the JSON body with the `request_id`. The backstop catches a panic of the outer layers.
- Cons: two layer instances. The answer of the backstop has no fragment and no JSON body.

### Option B: One outermost layer, as rule 3 says today

- Pros: one layer catches every panic of the router.
- Cons: the answer has no `request_id`, no security header, and no JSON body on the export routes.

### Option C: One layer inside `render_errors` only

- Pros: one layer and the full answer.
- Cons: a panic of an outer layer ends the connection task, and the client gets no answer.

### Option D: A thread-local value that hands the `request_id` from the panic hook to an outermost layer

- Pros: one outermost layer with the `request_id`.
- Cons: a thread-local value is global mutable state. The answer still has no JSON body on the export routes.

## Decision

The project uses **Option A**.

- (a) The inner `CatchPanicLayer::custom` sits directly inside `render_errors`. Its handler ignores the payload and returns a `500` answer with a marker in the response extensions. `render_errors` turns the marker into `WebError::Internal` (`E-WEB-010`) with the `request_id` of the request. The layer does not use `DefaultResponseForPanic`, because that handler writes its own `ERROR` line.
- (b) `render_errors` reads the `request_id` from the request. It writes the full page, the HTMX fragment with the headers of `api.md` section 5.3, or the JSON body on `/export/v1/`. The outer layers add the security headers and `X-Request-Id`.
- (c) The backstop is a second `CatchPanicLayer::custom` directly inside the layer `request_id`. It catches a panic of `TraceLayer`, the header layers, and `render_errors`.
- (d) The backstop answers a plain `500` with `Content-Type: text/plain`. The body holds the code `E-WEB-010` and the English text of its row in `reliability.md` section 1.2 without the reference, because the handler has no `request_id` (g). The layer `request_id` adds `X-Request-Id`. The answer has no `HX-*` header and no JSON body.
- (e) The answer of the backstop is the one exception to `api.md` sections 5.1 to 5.3 and to ADR-0053 (a). It is also the one exception to `.claude/rules/web.md` and to the web-ui requirement "Internal error response". It is also the one exception to ADR-0036 (p), ADR-0037 (z), ADR-0047 (b), and ADR-0049 (k).
- (f) A caught panic gives two `ERROR` lines. The panic hook writes the panic event. The error mapping writes the line of `E-WEB-010`. These are two events, so the rule "an error is logged exactly once" of section 1.1 stays true.
- (g) The handler of the backstop writes the line of `E-WEB-010` itself, because `render_errors` does not run for its answer. The handler gets only the panic payload, so this line holds no `request_id`. The panic hook line of rule 2 holds it while the span of `TraceLayer` is active.
- (h) Rule 6 of section 2.4 counts the inner layer and the backstop as two places. With the supervisor of the jobs and the join of a `spawn_blocking` task, the count is four. ADR-0058 adds a fifth place.
- (i) The open transaction of the request drops with the unwind, and PostgreSQL rolls it back (R29). The next request on the same connection gets a normal answer.
- (j) The header layers are inside the backstop, so its answer has only `X-Request-Id` of the headers of `api.md` section 9. This answer is the one exception to that section. Caddy adds HSTS, CSP, `nosniff`, and `Referrer-Policy` to it as defaults (ADR-0065 (f)).

## Consequences

### Positive

- A panic in a handler gives the same answer as every other `E-WEB-010`, with the `request_id`, in the UI language.
- A panic on an export route gives the JSON body of ADR-0053 (a).
- A panic in an outer layer still gives an answer with `X-Request-Id`.

### Negative

- The router has two `CatchPanicLayer` instances.
- The answer of the backstop is English text without a fragment and without JSON.
- The answer of the backstop gets HSTS, CSP, `nosniff`, and `Referrer-Policy` only from the defaults of Caddy.
- One panic gives two `ERROR` lines.

### Risks and mitigations

- Risk: a later layer goes between `render_errors` and the inner layer, and the answer loses its shape. Mitigation: the HTTP tests send a panic to a page, to an HTMX request, and to an export route. They also check the backstop answer and a normal answer after the panic (`reliability.md` section 6).

## Related

- ADR-0020 (failure policy) and ADR-0053 (a). This ADR amends both.
- ADR-0036 (p), ADR-0037 (z), ADR-0047 (b), and ADR-0049 (k) (the `HX-*` headers and the empty `#toast` of an HTMX error answer). This ADR amends the four items.
- `docs/architecture/reliability.md` section 1.1, section 2.4 rules 3 and 6, and section 3.2 (a line of the backstop without a `request_id`)
- `docs/architecture/api.md` sections 5.1, 5.2, 5.3, and 9
- ADR-0065 (f) (the default headers of Caddy)
- `.claude/rules/web.md` (the exception line) and `.claude/rules/rust.md` (the count of the places that catch a panic)
- Baseline requirements: web-ui "Internal error response", operations "Panic in a request"
- `overview.md` section 7: R29, R30, R36. Section 9: A41
- ADR-0058 (the fifth place)
