# ADR-0040: POST routes for translations and for the removal of a parameter

Status: Accepted
Date: 2026-10-15
Deciders: project owner

## Context

`api.md` section 2.6 has three routes with the methods `PUT` and `DELETE`:

- `PUT .../parameters/{key}/translations/{lang}` (`UpsertTranslation`),
- `DELETE .../parameters/{key}/translations/{lang}` (`RemoveTranslation`),
- `DELETE .../parameters/{key}` (`RemoveParameter`).

The path of each route holds `{ref}`, `{n}`, `{key}`, and `{lang}`. These are user data. ADR-0037 (f) permits user data in a URL only in a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute holds a literal. An HTML form sends only `GET` and `POST`.

Facts of HTMX 2.0.11 and axum 0.8.9:

- A boosted form sends the method of its `method` attribute. With `method="put"`, HTMX sends `PUT`.
- Without JavaScript, a browser sends `GET` for a form with an unknown method. The form holds `csrf_token`, so the token would go into the URL. ADR-0037 (h) forbids this.
- `hx-put=""` sends the request to the URL of the page, not to the form `action`.
- axum 0.8.9 and tower-http 0.7.1 have no method override.

## Decision drivers

- No URL holds the CSRF token (`api.md` section 7.3, ADR-0037 (h)).
- Each `hx-*` attribute holds a literal (ADR-0037 (f)).
- A form works the same way with and without JavaScript.
- No new dependency and no custom layer that changes the method of a request.

## Considered options

### Option A: POST routes

- Pros: a boosted form and a plain form send the same request. The project already uses `POST /recipes/{ref}/delete` and `POST /machines/{code}/archive`.
- Cons: `api.md` sections 1, 2.6, 7.6, and 9 change.

### Option B: A hidden method field and a layer that changes the method

- Pros: the paths and methods of `api.md` stay.
- Cons: a custom layer must run before the CSRF check and the router. The method of a request then depends on its body.

### Option C: A boosted form with `method="put"` or `method="delete"`

- Pros: no change of `api.md`.
- Cons: without JavaScript, the browser sends `GET` with the CSRF token in the URL.

## Decision

The project uses **Option A**.

- (a) `POST .../parameters/{key}/translations/{lang}` runs `UpsertTranslation`. Its request, its success answer, and its errors are those of the `PUT` row of `api.md` section 2.6.
- (b) `POST .../parameters/{key}/translations/{lang}/delete` runs `RemoveTranslation`. Its request, its success answer, and its errors are those of the first `DELETE` row of `api.md` section 2.6.
- (c) `POST .../parameters/{key}/delete` runs `RemoveParameter`. Its request, its success answer, and its errors are those of the second `DELETE` row of `api.md` section 2.6.
- (d) No route of the application uses `PUT` or `DELETE`. `api.md` section 1 lists `POST` as the only method of a mutating request.
- (e) The route of point (a) gets the body limit `http.body_limit.translation` (`api.md` section 7.6).
- (f) The removal of a parameter and the removal of a translation stand in the danger disclosure of `design-system.md` section 10.5. The `summary` names the action. The danger button names the tag of the parameter or the language tag.
- (g) The `htmx-config` meta tag keeps `methodsThatUseUrlParams: ["get"]` (`api.md` section 9). No route needs it now. The value keeps the body of any other method out of the URL.

## Consequences

### Positive

- Every form of the application sends `GET` or `POST`, with or without JavaScript.
- The CSRF layer, the router, and the route table need no method other than `GET` and `POST`.

### Negative

- The URL of a removal ends with `/delete`, not with the method `DELETE`.

### Risks and mitigations

- Risk: a client sends `PUT` or `DELETE` to the old paths. Mitigation: the router answers `405`, and no client outside the application exists.

## Related

- ADR-0001, ADR-0036, ADR-0037
- `api.md` sections 1, 2.6, 7.3, 7.6, 9
- `reliability.md` section 4.2
- `design-system.md` section 10.5
