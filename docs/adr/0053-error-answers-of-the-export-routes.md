# ADR-0053: Error answers of the export routes

Status: Accepted
Date: 2026-10-22
Deciders: project owner

## Context

`api.md` section 5.3 says that the export routes return JSON errors with the fields `code`, `message`, and `request_id`. Other parts of `api.md` give different answers for the same codes:

- Section 5.1 gives `error.html` to `E-EXP-001` and `E-EXP-003`, and to the codes of section 2 that every route can return.
- Section 5.1 gives `303` to `/login` for `E-WEB-004`. Section 7.4 and `reliability.md` section 1.2 give `303` to `/me/password` for `E-IDN-007`.
- No document says which language `message` uses.
- No document says whether an HTMX header changes a JSON answer.

A person starts the export with a plain link in a browser (ADR-0052). The routes use the session cookie only (ADR-0014), and no machine client exists.

## Decision drivers

- One answer format for each export route (R36).
- A person who meets an error reads one sentence in the UI language (`api.md` section 5.2, R10).
- A person with an expired session or a forced password change reaches the form that helps (R39).
- `recipe-web` has no JSON library of its own. Only `recipe-export` writes JSON for `recipe-web` (`overview.md` section 5).

## Considered options

### Option A: JSON for every error, redirects for the session and the password

- Pros: one body format for every code. The two redirects keep the flows of a browser.
- Cons: two codes have no JSON body.

### Option B: JSON for every code, also `E-WEB-004` and `E-IDN-007`

- Pros: no exception.
- Cons: a person with an expired session sees JSON, not the login form.

### Option C: An HTML error page for a browser request

- Pros: the page layout for each error.
- Cons: it breaks `api.md` section 5.3.

For the language, the options are the UI language of the request or English for every request.

## Decision

The project uses **Option A** and the UI language.

- (a) Every answer of the error layer with an error code on a path under `/export/v1/` is a JSON object. Its fields are `code`, `message`, and `request_id` in this order. The answer has `Content-Type: application/json`. This rule wins over the fragments of `api.md` section 5.1 for these paths. The text `404` of point (g) and the `HEAD` answer of ADR-0054 stay outside this rule.
- (b) `message` is the text of the key `error-<code>` in the UI language of the request (cookie `recipe_lang`, `api.md` section 8). It does not hold the sentence with the reference, because `request_id` is a field of its own.
- (c) The answer has no `HX-Retarget`, `HX-Reswap`, `HX-Push-Url`, or `HX-Redirect` header. A request with `HX-Request: true` gets the `403 E-WEB-007` of the navigation guard (ADR-0055).
- (d) `E-WEB-004` answers `303` to `/login` and ends the session cookie, as on the other routes. `E-IDN-007` answers `303` to `/me/password`. An HTMX header does not change these answers on the export paths.
- (e) `Retry-After` stays on a `503` answer and on a `429` answer.
- (f) `recipe-export` writes the JSON body. `recipe-web` puts it into the answer.
- (g) The text `404` of the router for an unknown path or an unlisted method stays as it is. It has no code on every path of the application. A later step can change it for the whole application.

## Consequences

### Positive

- A consumer reads one error format on the export routes.
- The person who clicks the export link reads the message in the UI language, with the code and the `request_id`.

### Negative

- Two codes answer with a redirect and no body.
- The text of `message` depends on the cookie. A program must use `code`, not `message`.

### Risks and mitigations

- Risk: a later error path on an export route returns HTML. Mitigation: a test sends each error of the route table to both export routes and checks the JSON body.

## Related

- ADR-0014, ADR-0019, ADR-0036, ADR-0047, ADR-0052, ADR-0054, ADR-0055
- `docs/architecture/api.md` sections 2.7, 5.1, 5.2, 5.3, 7.4, 8
- `docs/architecture/reliability.md` section 1.2
- `overview.md` section 7: R10, R36, R39. Section 9: A2, A24
