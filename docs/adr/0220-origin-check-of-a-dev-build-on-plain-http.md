# ADR-0220: Origin check of a dev build on plain HTTP

Status: Accepted. Amends ADR-0035 (e)
Date: 2026-11-22
Deciders: project owner

## Context

ADR-0035 (e) lets a mutating request pass the Origin check in two cases. `Sec-Fetch-Site` is `same-origin`, or `Origin` equals `https://` and the `Host` header. `api.md` §7.3 states the same rule.

`origin_matches` in `crates/recipe-web/src/csrf.rs` applies this rule. The constant `ORIGIN_PREFIX` holds `https://`, and the function reads no setting. So `session.secure=false` does not change the result.

ADR-0171 lets a `dev` build with `session.secure=false` write its cookies without `Secure`. Its Consequences state that such a build gets working cookies on plain HTTP.

A browser sends the `Sec-Fetch-*` headers only to a potentially trustworthy URL. A loopback URL is potentially trustworthy. A plain HTTP URL with another host is not. The browser then sends `Origin: http://` and the host, and no `Sec-Fetch-Site`.

A Chromium run of the browser regression suite showed this. The page `http://recipes.test:<port>` sent `POST /login` with `Origin: http://recipes.test:<port>` and no `Sec-Fetch-Site`. The server answered `403 E-WEB-007`. On this path, `csrf_layer` writes a `csrf_rejected` event. Other browsers may behave the same way, but no run checked them.

A `dev` build on plain HTTP with a host that is not a loopback address therefore refuses every mutating request. The login fails too. The e2e suite and the dev example do not show this, because they use `127.0.0.1` and `secure = true`.

## Decision drivers

- The Consequences of ADR-0171 must hold on plain HTTP. A `dev` build with `session.secure=false` must accept a login.
- A release build keeps the Origin check of today (R38). ADR-0062 (b) refuses `session.secure=false` without the feature `dev`.
- One setting selects the plain HTTP mode of a `dev` build. ADR-0171 uses `session.secure` for the cookies.
- A `dev` build on a LAN host must accept a login on plain HTTP.

## Considered options

### Option A: Accept `http://` with `session.secure=false`

- Pros: the Origin check follows the same setting as the cookies. A `dev` build on plain HTTP works. A release build does not change.
- Cons: `origin_matches` needs the value from the settings.

### Option B: A new setting for the scheme of the Origin check

- Pros: the Origin check has its own switch.
- Cons: two settings select one mode. A `dev` build with one switch on and one switch off gets no working login.

### Option C: No change. A `dev` build on plain HTTP uses a loopback host only

- Pros: no code change.
- Cons: ADR-0171 Consequences stay false for a host that is not a loopback address. A test on a second device of the LAN cannot log in.

## Decision

The project uses **Option A**.

- (a) With `session.secure=true`, the Origin check stays as ADR-0035 (e) states it. `Origin` must equal `https://` and the `Host` header.
- (b) With `session.secure=false`, `Origin` passes when it equals `https://` and the `Host` header. It also passes when it equals `http://` and the `Host` header.
- (c) Only a build with the feature `dev` accepts `session.secure=false`. ADR-0062 (b) stays in force. A release build therefore accepts only `https://`.
- (d) `Sec-Fetch-Site: same-origin` passes in both cases, as ADR-0035 (e) states. Without that header, every other `Origin` fails, for example `Origin: null`, another host, or another port.
- (e) A refused request keeps its answer `403 E-WEB-007` and its `csrf_rejected` event (`api.md` §7.3).
- (f) `origin_matches` takes the value from `WebSettings` (ADR-0171 (f)). `WebSettings::default()` keeps `secure = true`.
- (g) The guard of the export routes does not change (ADR-0055).
- (h) Two tests in `crates/recipe-web/src/csrf/tests.rs` send `Origin: http://` and the host without `Sec-Fetch-Site`. With `false`, the request passes. With `true`, it gets `403 E-WEB-007`.
- (i) This ADR amends ADR-0035 (e). The Origin rule of `api.md` §7.3 changes.

## Consequences

### Positive

- A `dev` build with `session.secure=false` accepts a login on plain HTTP from a host that is not a loopback address.
- One setting selects the cookies and the Origin check of plain HTTP.
- A release build keeps the Origin check of today.

### Negative

- `origin_matches` reads a setting. The function depends on `WebSettings`.

### Risks and mitigations

- Risk: a `dev` build on plain HTTP sends its cookies and forms as clear text on the network. Mitigation: ADR-0062 (b) keeps `session.secure=false` out of a release build. The CSRF token check still applies (`api.md` §7.3).
- Risk: the check reads a fixed scheme again after a later edit. Mitigation: the two tests of item (h) fail for one of the two values.

## Related

- ADR-0035 (e). This ADR amends item (e).
- ADR-0055 (guard of the export routes), ADR-0062 (b), ADR-0171 (cookies without `Secure` in a `dev` build)
- `api.md` §7.3
- `reliability.md` §7.11
- Change `review-design-fixes-2`, capability `identity-access`
- `overview.md` §7: R38
