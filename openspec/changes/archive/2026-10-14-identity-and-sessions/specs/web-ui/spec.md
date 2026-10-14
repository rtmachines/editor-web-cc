## ADDED Requirements

### Requirement: Security headers on every response
Every response SHALL carry the headers `Content-Security-Policy`, `X-Content-Type-Options`, `Referrer-Policy`, `Strict-Transport-Security`, and `X-Request-Id` with the values of `api.md` section 9. Every response except `GET /static/*` SHALL carry `Cache-Control: no-store` (R38, R39).

#### Scenario: Headers of the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the response sets `Content-Security-Policy` to the value of `api.md` section 9
- **AND** the response sets `Cache-Control` to `no-store`
- **AND** the response sets `X-Request-Id` to the `request_id` of the request

#### Scenario: Headers of an error response
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /users`
- **THEN** the response carries the same security headers as a success response

### Requirement: Static files
`GET /static/*` SHALL serve the vendored HTMX 2 file without a change and SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

#### Scenario: HTMX file without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/htmx.min.js`
- **THEN** the server returns `200` with the file
- **AND** the response sets `Cache-Control` to `public, max-age=86400`

### Requirement: HTMX configuration of every page
Every full page SHALL hold the `htmx-config` meta tag with the values of `api.md` section 9. Its `<body>` SHALL have `hx-history="false"`. With a session, the `<body>` SHALL also have `hx-headers` with the header `X-CSRF-Token` (R38, R39).

#### Scenario: Profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator sends `GET /me`
- **THEN** the page holds the `htmx-config` meta tag with `"allowEval":false` and `"selfRequestsOnly":true`
- **AND** the `<body>` holds `hx-history="false"` and the CSRF token of the session in `hx-headers`

### Requirement: No inline script or style
A page SHALL load scripts only from `/static/`. A page SHALL hold no inline script, no inline style, and no inline event handler (R38).

#### Scenario: User list page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users`
- **THEN** the page holds no `<script>` element without a `src` attribute
- **AND** the page holds no `style` attribute and no attribute that starts with `on`

### Requirement: Escaped user text
A page SHALL show every text from a request or from the database as escaped text. User text SHALL NOT appear inside an `hx-*` attribute or a `<script>` element (R38).

#### Scenario: Script in a search field
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users` with `q` set to `<script>alert(1)</script>`
- **THEN** the response holds no `<script>alert(1)</script>` element
- **AND** the response holds the escaped text or the error `E-WEB-008`

### Requirement: Login redirect
A successful `POST /login` SHALL set the session cookie and return `303` to `/recipes`. When the user has `must_change_password`, it SHALL return `303` to `/me/password` (R21, R39).

#### Scenario: Login with a one-time password
- **GIVEN** the user `mhoffman` with `must_change_password` set to true
- **WHEN** a user sends `POST /login` with the login `mhoffman` and the right password
- **THEN** the server returns `303` to `/me/password`
- **AND** the response sets the cookie `__Host-recipe_session`

### Requirement: Password change response
A successful HTMX `POST /me/password` SHALL return `200` with the header `HX-Refresh: true` and the new session cookie. A request without `HX-Request` SHALL get `303` to `/recipes` with the new session cookie (R21, R39).

#### Scenario: Password change through HTMX
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with the right current password and a valid new password
- **THEN** the server returns `200` with `HX-Refresh: true`
- **AND** the response sets a new cookie `__Host-recipe_session`

## MODIFIED Requirements

### Requirement: Language switch redirect
`POST /lang` SHALL return `303` to the field `next` when `next` is a same-origin relative path. The path SHALL start with `/`, and its second character SHALL be neither `/` nor `\`. The path SHALL hold no `\` and no control or white-space character. Otherwise the server SHALL return `303` to `/recipes` and SHALL NOT use the `Referer` header (R38, A17).

#### Scenario: Redirect to an external address
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `https://other.example/page`
- **THEN** the response sets the cookie `recipe_lang` to `en`
- **AND** the response returns `303` to `/recipes`

#### Scenario: Redirect to a same-origin path
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `pl` and the field `next` set to `/recipes/R-100/versions/3`
- **THEN** the response returns `303` to `/recipes/R-100/versions/3`

#### Scenario: Backslash after the first slash
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `/\evil.example`
- **THEN** the response returns `303` to `/recipes`

#### Scenario: Tab after the first slash
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en` and the field `next` set to `/`, a tab character, and `/evil.example`
- **THEN** the response returns `303` to `/recipes`
