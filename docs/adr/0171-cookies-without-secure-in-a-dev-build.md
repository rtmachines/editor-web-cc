# ADR-0171: Cookies without Secure in a dev build

Status: Accepted. Amends ADR-0062 (b)
Date: 2026-11-14
Deciders: project owner

## Context

ADR-0062 (b) lets only a build with the feature `dev` accept `session.secure=false`. The item does not say what the value `false` changes.

The code reads the value and records it. `check_secure` in `crates/recipe-server/src/config/serve.rs` refuses `false` without the feature `dev`. `CompareSettings` stores the value as `session.secure` in `app_setting` (ADR-0062 (a)).

No other code reads the value. Each cookie writer in `recipe-web` passes the literal `true` to `.secure(..)`. These writers are `session_cookie`, `clear_session_cookie`, and `lang_cookie` in `cookies.rs`, and the pre-session cookie in `csrf/pre_session.rs`.

A `dev` build with `session.secure=false` therefore records a value that no code applies. Each cookie keeps `Secure`, so a `dev` build on plain HTTP can get no working cookie.

A browser also refuses a cookie with the prefix `__Host-` and without `Secure`. The names `__Host-recipe_session` and `__Host-recipe_csrf` of `api.md` §7.1 and §7.3 then cannot stay.

## Decision drivers

- A recorded setting must equal the value that the code applies (A34, `reliability.md` §7.12).
- A release build keeps `Secure` and the prefix `__Host-` on its cookies (R21, R38).
- One process uses one set of cookie names.
- The owner asked for a code change, not only a text change.

## Considered options

### Option A: Apply `session.secure` to each cookie

- Pros: the recorded value is the applied value. A `dev` build on plain HTTP gets working cookies.
- Cons: two cookies get a second name. The writers and the readers need the value from the settings.

### Option B: No effect of the key

- Pros: no code change.
- Cons: the key records a value that no code applies. The owner asked for a code change.

## Decision

The project uses **Option A**.

- (a) Only a build with the feature `dev` accepts `session.secure=false`. ADR-0062 (b) stays in force. With `false`, no cookie has `Secure`.
- (b) The value sets the name of each cookie as this table shows.

| Cookie | `session.secure = true` | `session.secure = false` |
|---|---|---|
| Session cookie | `__Host-recipe_session` with `Secure` | `recipe_session` without `Secure` |
| Pre-session cookie | `__Host-recipe_csrf` with `Secure` | `recipe_csrf` without `Secure` |
| Language cookie | `recipe_lang` with `Secure` | `recipe_lang` without `Secure` |

- (c) The session cookie and the pre-session cookie lose the prefix `__Host-` with `false`, because a browser refuses that prefix without `Secure`. `recipe_lang` keeps its name.
- (d) Each cookie keeps its other attributes. The session cookie and the pre-session cookie keep `HttpOnly`. Each cookie keeps `SameSite=Strict`, `Path=/`, and no `Domain`.
- (e) The server keeps `Strict-Transport-Security` on each response (`api.md` §7.1). A browser ignores it on plain HTTP.
- (f) `WebSettings` holds the value. `WebSettings::default()` keeps `secure = true`. `Plan::new` in `crates/recipe-server/src/serve/plan.rs` copies the configured value into `WebSettings`.
- (g) Each writer and each reader of the cookies takes the name from `WebSettings`. One process therefore uses one set of names.
- (h) The writers are `session_cookie`, `clear_session_cookie`, and `lang_cookie` in `cookies.rs`, and the pre-session cookie in `csrf/pre_session.rs`. The text `CLEAR_SESSION_FALLBACK` in `cookies.rs` follows the value too.
- (i) The readers are `session.rs` and `pages/request.rs` through `SESSION_COOKIE`, and `csrf.rs` through `CSRF_COOKIE`.
- (j) Round-trip tests run for both values. One test sends a login, then a `GET` with the session cookie, then a logout. One test sends `GET /login`, then `POST /login` with the pre-session cookie.
- (k) This ADR amends ADR-0062 (b). `api.md` §7.1, §7.3, and §8 change. The rows `session.cookie_name` and `session.secure` of `reliability.md` §7.11 change.

## Consequences

### Positive

- The recorded value of `session.secure` is the value that the cookies show.
- A `dev` build with `false` gets working cookies on plain HTTP.
- A release build keeps the names and the attributes of today.

### Negative

- The session cookie and the pre-session cookie have two names. Each writer and each reader needs the value from `WebSettings`.

### Risks and mitigations

- Risk: a browser keeps the old cookies after a change of `session.secure`. Mitigation: the server ignores a cookie with the other name. The user logs in again.
- Risk: a writer or a reader keeps a fixed name. Mitigation: the round-trip tests of item (j) fail for one of the two values.

## Related

- ADR-0062 (a), (b). This ADR amends item (b).
- ADR-0014 (session store), ADR-0035 (pre-session cookie)
- `api.md` §7.1, §7.3, §8
- `reliability.md` §7.11, §7.12
- Change `review-design-fixes`, capabilities `identity-access` and `web-ui`
- `overview.md` §7: R10, R21, R38. §9: A17, A34
