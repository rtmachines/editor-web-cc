# Web rules

Source: `docs/architecture/api.md` §1 to §9, ADR-0001, ADR-0008, ADR-0015. Load the skill `htmx:htmx` before you write templates.

## Rendering (api.md §1, ADR-0001)

- `recipe-web` renders HTML on the server with axum, askama, and HTMX 2. There is no JavaScript build chain.
- A full page request gets `text/html`. An HTMX request (`HX-Request: true`) gets a fragment.
- Only the export routes under `/export/v1/` return JSON.
- A path value parses into its value object. A bad path value never gives a 500. §1 gives the code of each bad value.
- Every mutating route accepts only `application/x-www-form-urlencoded`. Request structs use `#[serde(deny_unknown_fields)]`.
- UI text comes from the `fluent` bundles `pl` (default) and `en`. The cookie `recipe_lang` holds the choice (api.md §8).

## HTMX configuration (api.md §9)

The `<meta name="htmx-config">` tag sets these values:

- `allowEval: false`, `allowScriptTags: false`, `selfRequestsOnly: true`, `historyCacheSize: 0`.
- `methodsThatUseUrlParams: ["get"]`. Then `hx-delete` sends a form body like `hx-post`.
- `includeIndicatorStyles: false`. The rules for `.htmx-indicator` are in `static/app.css`.
- `responseHandling: [{"code":"204","swap":false},{"code":"[23]..","swap":true},{"code":"[45]..","swap":true,"error":true},{"code":"...","swap":false}]`.
- Without that `responseHandling`, HTMX 2 does not swap `conflict.html`, `field-errors.html`, or `error.html`.
- `<body>` has `hx-history="false"` and `hx-headers` with the header `X-CSRF-Token`.
- Every approve form and save form has `hx-sync="closest form:drop"`.

## CSP and scripts (api.md §9)

Every response sends this `Content-Security-Policy`:

```text
default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' data:; object-src 'none'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'
```

- Only three scripts exist: `static/htmx.min.js` (vendored HTMX 2, unchanged), `static/row-version.js` (at most 20 lines), `static/tree.js` (at most 50 lines).
- Do not add a script, an inline script, an inline style, or an inline event handler.
- User data never appears in an `hx-*` attribute or in a `<script>` tag.
- Never apply the askama filter `|safe` to a value that is not a literal.

## Requests, concurrency, errors (api.md §4, §5, §7)

- Every `POST`, `PUT`, and `DELETE` carries the CSRF token: the field `csrf_token` or the header `X-CSRF-Token`.
- `POST /login` and `POST /lang` without a session use the pre-session token `csrf_pre`. A bad token gives `403 E-WEB-007`.
- A mutating request needs a matching `Origin` header or `Sec-Fetch-Site: same-origin`. A rejection writes a `csrf_rejected` security event.
- Every form on a draft page has the hidden field `row_version`. Every mutating request on a draft sends it.
- A success sets `HX-Trigger: {"rowVersion": <new>}`. A mismatch gives `409 E-WEB-001` with `conflict.html`.
- Every error fragment shows its code `E-<CTX>-<nnn>`, one sentence in the UI language, and the `request_id`.
- An HTMX error response sets `HX-Retarget: #error-panel` and `HX-Reswap: innerHTML`.
- Only `E-WEB-004` (no session or expired session) uses `HX-Redirect: /login` (api.md §5.1).
- A session with `must_change_password` gets `303` to `/me/password` on every other route (`E-IDN-007`, api.md §7.4).
- A response never shows `Debug` output, a stack trace, an SQL fragment, or a file path.
- The HTTP test suite sends an XSS payload corpus through every text field.
