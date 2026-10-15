# web-ui Specification

## Purpose
The web UI renders HTML fragments for the recipe editor and protects drafts from overwrites. It maps errors to HTTP responses and selects the UI language.

## Requirements

### Requirement: Row version in every form
The system SHALL render the current `row_version` of a version into the hidden field `row_version` of every form on the version page. Every mutating request on that version SHALL send `row_version` (R39, I12).

#### Scenario: Hidden field on the version page
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** every form of the page holds the hidden field `row_version` with the value 7

### Requirement: New row version after a success
On success, the response SHALL set the header `HX-Trigger` to `{"rowVersion": <new>}`. The script `row-version.js` SHALL set every hidden `row_version` field of the page to the new value (R39, I12).

#### Scenario: Save with the current row version
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **WHEN** a ProcessEngineer sends a change of a parameter value with `row_version` 7
- **THEN** the response sets the header `HX-Trigger` to `{"rowVersion": 8}`
- **AND** every hidden `row_version` field of the page holds the value 8

### Requirement: Conflict fragment
On a mismatch of `row_version`, the server SHALL return `409 E-WEB-001` with the fragment `conflict.html`. The fragment SHALL show the current `row_version`, the login of the last editor, and the time of the last change. The fragment SHALL show one button, Reload version, that sends a GET request to the version page (R36, R39, I12).

#### Scenario: Stale save keeps the input
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 8 and the last editor `anowak`
- **WHEN** a ProcessEngineer sends a change of a parameter value with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`
- **AND** the fragment shows the `row_version` 8 and the login `anowak`
- **AND** the unsaved input stays in the form until the reload

### Requirement: Parameter change sets a new row version
A change of a parameter value SHALL change the `row_version` of the version, not only the parameter row. Two users who change two different parameters of one draft SHALL get a conflict on the second save (R39, I12).

#### Scenario: Two parameters of one draft
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **AND** a ProcessEngineer saved a parameter value, so the `row_version` became 8
- **WHEN** a second ProcessEngineer sends a change of another parameter with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`

### Requirement: Second request of a form
Every approve form and every save form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second request of such a form while the first request is in flight (R39, I12).

#### Scenario: Second request while a request is in flight
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer sends the same save form again
- **THEN** the browser drops the second request

### Requirement: Idempotent approval
When `ApproveVersion` returns `AlreadyApproved`, the server SHALL render the version page with status `200` (R39, I12).

#### Scenario: Repeated approval
- **GIVEN** the approved version 3 of the recipe `R-100` with `row_version` 9
- **WHEN** a ProcessEngineer sends an approval of version 3 with `row_version` 8
- **THEN** the server renders the version page with status `200`

### Requirement: HTMX error headers
For an HTMX request with a status of 400 or higher, the response SHALL set `HX-Retarget` to `#error-panel` and `HX-Reswap` to `innerHTML show:#error-panel:top`, except for `E-WEB-004`. The same response SHALL set `HX-Push-Url` to `false`, so a refused form keeps the URL of its page (R39, ADR-0037).

#### Scenario: Concurrent edit response headers
- **GIVEN** a ProcessEngineer sends an HTMX save with a stale `row_version` of the draft version 3
- **WHEN** the server returns `409 E-WEB-001`
- **THEN** the response sets `HX-Retarget` to `#error-panel`
- **AND** the response sets `HX-Reswap` to `innerHTML show:#error-panel:top`

#### Scenario: Refused machine form keeps the URL
- **GIVEN** the ProcessEngineer `anowak` with an open session on the page `/machines/new`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `X1`
- **THEN** the server returns `422 E-WEB-008`
- **AND** the response sets `HX-Push-Url` to `false`

### Requirement: Session end response
The server SHALL return `401 E-WEB-004` when a session expires or does not exist. For an HTMX request, the response SHALL set `HX-Redirect` to `/login`. For another request, the server SHALL return `303` to `/login`. Only `E-WEB-004` SHALL use `HX-Redirect` (R39).

#### Scenario: Ended session in an HTMX request
- **GIVEN** a session that expires
- **WHEN** a ProcessEngineer sends an HTMX request for `GET /recipes`
- **THEN** the server returns `401 E-WEB-004`
- **AND** the response sets `HX-Redirect` to `/login`

### Requirement: Validation error response
A malformed field, a missing field, or a request with a wrong content type SHALL return `422` with the fragment `field-errors.html`. The fragment SHALL show one line for each field (R36, R39).

#### Scenario: Missing field
- **GIVEN** a form that omits a required field
- **WHEN** a ProcessEngineer sends the form
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the fragment shows one line that names the field

### Requirement: Client error statuses
The server SHALL return `409` for a state conflict and `403` for a missing permission. It SHALL return `404` for a missing resource in the URL and `413` for a request above the body limit. These responses SHALL use the fragment `error.html` (R39).

#### Scenario: Missing permission
- **GIVEN** a user with the role Operator
- **WHEN** an Operator sends a request for an operation that needs `ParameterEdit`
- **THEN** the server returns `403 E-WEB-002` with the fragment `error.html`
- **AND** the fragment names the operation

### Requirement: Login throttle response
When the login throttle is active, the server SHALL return `429 E-IDN-005` with the fragment `error.html` and the header `Retry-After` (R39).

#### Scenario: Throttled login
- **GIVEN** the login throttle is active for the login `anowak` and the client address
- **WHEN** a user sends `POST /login` with the login `anowak`
- **THEN** the server returns `429 E-IDN-005`
- **AND** the response sets the header `Retry-After`

### Requirement: Unavailable response
When a timeout, a deadlock, pool exhaustion, or a database outage occurs, the server SHALL return `503` with the fragment `error.html`. The header `Retry-After` SHALL hold the value of the setting `http.retry_after`, which is 5 seconds (R39).

#### Scenario: Pool exhausted
- **GIVEN** a connection pool with no free connection
- **WHEN** a ProcessEngineer sends a request for `GET /recipes`
- **THEN** the server returns `503` with the fragment `error.html`
- **AND** the response sets `Retry-After` to 5 seconds

### Requirement: Internal error response
A caught panic, a failed `spawn_blocking` task, or an unexpected database error SHALL return `500` with the fragment `error.html`. The response SHALL show the error code and the `request_id` only (R36).

#### Scenario: Unexpected database error
- **GIVEN** a database error with an unexpected SQLSTATE
- **WHEN** a ProcessEngineer sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server returns `500 E-DB-008` with the fragment `error.html`
- **AND** the response shows the `request_id`
- **AND** the response shows no SQL, no error chain, and no file path

### Requirement: Error fragment content
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id`. Each code of the routes of `api.md` sections 2.2 and 2.3 SHALL have its sentence in the bundles `pl` and `en` (R10, R36).

#### Scenario: Validation error fragment
- **GIVEN** a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer sends a request that gets a validation error
- **THEN** the error fragment shows the code `E-WEB-008`
- **AND** the fragment shows one English sentence that tells the user what to do
- **AND** the fragment shows the `request_id`
- **AND** the structured log of the request holds the same code

#### Scenario: Plant error in Polish
- **GIVEN** the ProcessEngineer `anowak` with an open session and the UI language `pl`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7/units/X1`
- **THEN** the server returns `422 E-PLT-017`
- **AND** the fragment shows the sentence of `error-E-PLT-017` from the bundle `pl`

### Requirement: Full error page
A request without the header `HX-Request` SHALL get a full error page. The page SHALL have the content of the error fragment and the page layout (R01, R36, R39).

#### Scenario: Missing recipe without HTMX
- **GIVEN** no recipe `R-999` exists
- **WHEN** a user sends `GET /recipes/R-999` without the header `HX-Request`
- **THEN** the server returns `404 E-WEB-005` with a full error page
- **AND** the page shows the code and the `request_id`

### Requirement: JSON error on export routes
The export routes SHALL return a JSON error body with the fields `code`, `message`, and `request_id` (R36).

#### Scenario: Recipe without an approved version
- **GIVEN** the recipe `R-100` with no approved version
- **WHEN** a user sends `GET /export/v1/recipes/R-100/approved`
- **THEN** the server returns `404` with a JSON error body
- **AND** the field `code` of the body holds `E-EXP-001`

### Requirement: Read-only error response
In read-only mode, a command other than `Login`, `Logout`, and `ExportVersion` SHALL return `503 E-WEB-006` with the fragment `error.html` and the read-only banner (A24).

#### Scenario: Save in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** a ProcessEngineer sends a change of a parameter value
- **THEN** the server returns `503 E-WEB-006` with the fragment `error.html`
- **AND** the page shows the read-only banner

### Requirement: Routes available in read-only mode
In read-only mode, queries, the export routes, `POST /login`, and `POST /logout` SHALL stay available (A24).

#### Scenario: Version page in read-only mode
- **GIVEN** the application in read-only mode
- **WHEN** an Operator sends a request for `GET /recipes/R-100/versions/3`
- **THEN** the server renders the version page with status `200`

### Requirement: Language cookie value
The cookie `recipe_lang` SHALL hold `pl` or `en`, and the server SHALL read it on every request. A missing cookie or another value SHALL select the UI language `pl`. The server SHALL ignore the header `Accept-Language` (R10, A17).

#### Scenario: Unknown cookie value
- **GIVEN** the cookie `recipe_lang` with the value `de` and the header `Accept-Language: en-US`
- **WHEN** a user sends a request for `GET /recipes`
- **THEN** the UI uses the language `pl`

### Requirement: Language cookie attributes
The cookie `recipe_lang` SHALL have a `Max-Age` of 365 days, `Secure`, `SameSite=Strict`, and `Path=/` (R10, A17).

#### Scenario: Cookie set by a language change
- **GIVEN** a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` with a `Max-Age` of 365 days
- **AND** the cookie has the attributes `Secure`, `SameSite=Strict`, and `Path=/`

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

### Requirement: Language switch without session or audit
`POST /lang` SHALL work before login and in read-only mode. It SHALL write no audit entry, because it writes no database row (A17, A24).

#### Scenario: Language change in read-only mode
- **GIVEN** the application in read-only mode and a user with no session
- **WHEN** a user sends `POST /lang` with the language `en`
- **THEN** the response sets the cookie `recipe_lang` to `en` and returns `303`
- **AND** the system writes no audit entry

### Requirement: Translation fallback
A parameter name or description SHALL show the translation for the UI language when a translation exists. Otherwise the page SHALL show the English base text with the marker `(en)` (R10, A17).

#### Scenario: Missing Polish translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and no `pl` translation
- **WHEN** a user sends `GET /recipes/R-100/versions/3` with the cookie `recipe_lang` set to `pl`
- **THEN** the page shows `Line speed (en)`

### Requirement: Messages from the fluent bundles
Error messages, status words, and permission names SHALL come from the `fluent` bundles `pl` and `en`. An error code SHALL NOT change with the UI language. The values that a message shows as words SHALL also come from the bundles. These values are a field reason, a missing entity, a password reason, a task stage, and an operation name. Only data SHALL stay data, for example a login, a code, or a request id (R10, R36, A17).

#### Scenario: Error message in English
- **GIVEN** the cookie `recipe_lang` set to `en`
- **WHEN** an Operator sends a request that needs `ParameterEdit`
- **THEN** the error fragment shows the English message of `E-WEB-002`
- **AND** the error fragment shows the code `E-WEB-002`

#### Scenario: Missing user in Polish
- **GIVEN** the Administrator `akowal` with an open session and the UI language `pl`
- **WHEN** `akowal` sends an HTMX request for `GET /users/nobody`
- **THEN** the server returns `404 E-WEB-005`
- **AND** the fragment shows `Użytkownik nobody` and no English word

### Requirement: English UI text style
English UI strings and English error message templates SHALL follow the structural ASD-STE100 rules of `reliability.md` section 8 (R40).

#### Scenario: English error template
- **GIVEN** the English message template of `E-WEB-005`
- **WHEN** a user sends a request for `GET /recipes/R-999`
- **THEN** the error fragment shows the English template with the recipe reference
- **AND** the template has at most 25 words and no semicolon

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
`GET /static/*` SHALL serve the vendored HTMX 2 file, the style sheets `app.css` and `components.css`, the scripts `row-version.js` and `tree.js`, and the 8 font files of `design-system.md` section 5. Each file SHALL be unchanged and SHALL have its media type. The route SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

#### Scenario: HTMX file without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/htmx.min.js`
- **THEN** the server returns `200` with the file
- **AND** the response sets `Cache-Control` to `public, max-age=86400`

#### Scenario: Style sheet without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/app.css`
- **THEN** the server returns `200` with the media type `text/css; charset=utf-8` and `X-Content-Type-Options: nosniff`
- **AND** the response sets `Cache-Control` to `public, max-age=86400`

#### Scenario: Component style sheet without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/components.css`
- **THEN** the server returns `200` with the media type `text/css; charset=utf-8` and `X-Content-Type-Options: nosniff`

#### Scenario: Font file
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/fonts/atkinson-hyperlegible-mono-latin-400-normal.woff2`
- **THEN** the server returns `200` with the media type `font/woff2`
- **AND** the SHA-256 of the body equals the hash that `recipe-web` records for the file

#### Scenario: Unknown static file
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/fonts/other.woff2`
- **THEN** the server returns `404` with `Cache-Control: no-store`

### Requirement: HTMX configuration of every page
Every full page SHALL hold the `htmx-config` meta tag with the values of `api.md` section 9. Its `<body>` SHALL have `hx-history="false"`. With a session, the `<body>` of every full page except the full error page SHALL also have `hx-headers` with the header `X-CSRF-Token` (R38, R39).

#### Scenario: Profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator sends `GET /me`
- **THEN** the page holds the `htmx-config` meta tag with `"allowEval":false` and `"selfRequestsOnly":true`
- **AND** the `<body>` holds `hx-history="false"` and the CSRF token of the session in `hx-headers`

#### Scenario: Full error page with a session
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request`
- **THEN** the page holds the `htmx-config` meta tag and `hx-history="false"`
- **AND** the `<body>` holds no `hx-headers`

### Requirement: No inline script or style
A page SHALL load only the scripts `htmx.min.js`, `row-version.js`, and `tree.js` from `/static/`. `row-version.js` SHALL have at most 20 lines, and `tree.js` SHALL have at most 50 lines. A page SHALL hold no inline script, no `<style>` element, no `style` attribute, and no inline event handler (R38).

#### Scenario: User list page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users`
- **THEN** the page holds no `<script>` element without a `src` attribute
- **AND** the page holds no `style` attribute and no attribute that starts with `on`

#### Scenario: Scripts of a page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the page loads `/static/htmx.min.js`, `/static/row-version.js`, and `/static/tree.js` and no other script
- **AND** the page holds no `<style>` element

#### Scenario: Size of the scripts
- **GIVEN** the files `static/row-version.js` and `static/tree.js`
- **WHEN** the script test counts their lines
- **THEN** `row-version.js` has at most 20 lines and `tree.js` has at most 50 lines

### Requirement: Escaped user text
A page SHALL show every text from a request or from the database as escaped text. User text SHALL NOT appear inside an `hx-*` attribute or a `<script>` element. A template SHALL use no unescaped output except `|safe` on a literal (R38).

#### Scenario: Script in a search field
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users` with `q` set to `<script>alert(1)</script>`
- **THEN** the response holds no `<script>alert(1)</script>` element
- **AND** the response holds the escaped text or the error `E-WEB-008`

#### Scenario: Quote in the field next of the language switch
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me?x='onfocus=alert(1)'`
- **THEN** the field `next` of the language switch holds `/me?x=&#39;onfocus=alert(1)&#39;`
- **AND** each `<svg` of the response equals the fixed tag of its icon in `design-system.md` section 8

#### Scenario: Unescaped output in a template
- **GIVEN** every template and every Rust source of `recipe-web`
- **WHEN** the template test reads them
- **THEN** no template uses `{% filter safe %}`, an `escape` filter with an argument, or `|safe` on a variable
- **AND** no Rust source sets `escape` or `ext` on a template or wraps a value in `Safe`, `HtmlSafe`, or `MaybeSafe`

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

### Requirement: Page layout
Every full page SHALL show, in this order, a skip link to the main content, a top bar, and `main` with the id `content`. `main` SHALL start with `#error-panel` and the status region `#toast`, then one `h1`. The top bar SHALL show the application name. Every full page SHALL link the style sheets `/static/app.css` and `/static/components.css` in this order (R39, A44).

#### Scenario: Layout of the profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me`
- **THEN** the page holds a skip link to `#content` before the top bar
- **AND** `main` with the id `content` holds `#error-panel`, `#toast`, and one `h1` in this order
- **AND** the page links the style sheet `/static/app.css` and then the style sheet `/static/components.css`

#### Scenario: Layout of a full error page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request` and gets `404 E-WEB-005`
- **THEN** the page shows the top bar with the application name and no navigation link
- **AND** the page shows the error in `#error-panel`, one `h1`, and a link to `/recipes`
- **AND** the title of the page holds the code `E-WEB-005` and the application name

### Requirement: Navigation by permission
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The links SHALL come in the order recipes, machines, users, own profile. The use case of that page SHALL still check the permission. The current link SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700. A session with `must_change_password` SHALL show no navigation link (R28, R39).

#### Scenario: Navigation of an Operator
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, and `/me`
- **AND** the top bar shows no link to `/users`
- **AND** the link to `/recipes` has `aria-current="page"`

#### Scenario: Navigation of an Administrator
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /me`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/users`, and `/me` in this order

#### Scenario: Current link on a machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines`
- **THEN** the link to `/machines` has `aria-current="page"`

#### Scenario: Navigation during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the top bar shows no navigation link
- **AND** the top bar shows the logout button

#### Scenario: Look of the current link
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `[aria-current]` in the top bar
- **THEN** the rule sets a bottom border of 3 px and the weight 700

### Requirement: Language switch
Every full page that answers a `GET` request SHALL hold a form that sends `POST /lang`. The form SHALL send the CSRF token of the session, or the field `csrf_pre` on the login page. The field `next` SHALL hold the path and the query of the request. A full page that answers another method SHALL hold no switch. The login page SHALL always hold the switch with `next` set to `/login`, also as the answer of a failed `POST /login` (R10, A17).

#### Scenario: Language switch on the user list
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users?q=ab`
- **THEN** the page holds a form that sends `POST /lang` with the field `csrf_token` of the session
- **AND** the field `next` of the form holds `/users?q=ab`

#### Scenario: Language switch on the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds a form that sends `POST /lang` with the field `csrf_pre`
- **AND** the field `csrf_pre` holds the nonce of the cookie `__Host-recipe_csrf` of the same response

#### Scenario: Language switch after a failed login
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` with a wrong password and without the header `HX-Request`
- **THEN** the login page holds a form that sends `POST /lang` with the field `csrf_pre`
- **AND** the field `next` of the form holds `/login`

#### Scenario: No switch on the page of a one-time password
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `POST /users/anowak/reset-password` without the header `HX-Request`
- **THEN** the page shows the one-time password
- **AND** the page holds no form that sends `POST /lang`

### Requirement: Language labels
The button of each language SHALL show the name of the language in that language, with the attribute `lang` of that language. The button of the current language SHALL have `aria-current="true"` and the look of the current navigation link (R10, A17, A44).

#### Scenario: Labels on an English page
- **GIVEN** the cookie `recipe_lang` set to `en` and the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me`
- **THEN** the button `Polski` has `lang="pl"` and no `aria-current` attribute
- **AND** the button `English` has `lang="en"` and `aria-current="true"`

### Requirement: State shown by color, icon, and word
A version state, a user state, and the state "Archived" of a machine or a unit SHALL show a color, an icon, and a word. Each error, warning, information, and success message SHALL also show a color, an icon, and a word. No element SHALL show a state by its color alone (R39, A44).

#### Scenario: State of a draft version
- **GIVEN** the draft version 3 of the recipe `R-100` and the UI language `pl`
- **WHEN** a page shows the state of version 3
- **THEN** the page shows the icon `status-draft` and the word `Szkic` next to it

#### Scenario: State of an inactive user
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends `GET /users` with the UI language `en`
- **THEN** the row of `anowak` shows the icon `user-inactive` and the word `Inactive`

#### Scenario: State of an archived unit
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the archived machine `M12` with the archived unit `U2`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the node of `U2` shows the icon `status-retired` and the word `Archived`

#### Scenario: Error fragment as one alert
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX request for `GET /users` and gets `403 E-WEB-002`
- **THEN** the fragment is one element with `role="alert"`
- **AND** that element holds the icon `alert-error`, the code `E-WEB-002`, one sentence, and the reference

### Requirement: Field error lines
Each line of `field-errors.html` SHALL show the label of the field from the bundles, not the name of the form field. In an HTMX fragment and on the login page, the line SHALL link to the input of the field. A field of a row of a list SHALL show the row number in its label and SHALL link to the input of that row. The full error page SHALL show the label without a link (R36, A44).

#### Scenario: Too long current password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a current password of 129 characters
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the line shows the label `Current password` with a link to `#current_password`

#### Scenario: Malformed member name
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the name `ST_Zone` and a first member with the name `9speed`
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the line shows the label `Name of member 1` with a link to `#members-0-name`

### Requirement: Contrast of the color tokens
Each row of the contrast table of `design-system.md` section 4 SHALL reach its minimum WCAG 2.2 ratio with the values of `/static/app.css`. A text pair SHALL reach 4.5:1. A pair of a UI element or an icon SHALL reach 3:1 (A44).

#### Scenario: Muted text on the canvas
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the contrast test computes the ratio of `--color-text-muted` on `--color-canvas`
- **THEN** the ratio is at least 4.5:1

#### Scenario: Edge of a draft badge
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the contrast test computes the ratio of `--color-draft-edge` on `--color-canvas`
- **THEN** the ratio is at least 3:1

### Requirement: Design tokens equal the reference
The style sheet `app.css` SHALL define each token of the token tables of `design-system.md` with the value of the table. Only `app.css` SHALL hold `:root`. A rule outside `:root` of either style sheet SHALL name no color value except `transparent`, `currentColor`, and the system colors of the forced-colors mode (R39, A44).

#### Scenario: Token of the action color
- **GIVEN** the row `--color-action` with the value `#0B4C86` in `design-system.md` section 3
- **WHEN** the token test reads `:root` of `/static/app.css`
- **THEN** `:root` sets `--color-action` to `#0B4C86`

#### Scenario: Color value outside the tokens
- **GIVEN** a rule of `/static/app.css` outside `:root`
- **WHEN** the token test reads the rule
- **THEN** the rule holds no hex color, no color function, and no named color outside the permitted list

#### Scenario: Color value in the component style sheet
- **GIVEN** a rule of `/static/components.css`
- **WHEN** the color test reads the rule
- **THEN** the rule holds no `:root`, no hex color, no color function, and no named color outside the permitted list

### Requirement: Touch targets and focus indicator
Every button, input, select, navigation item, tree node, and link outside a sentence SHALL have a minimum height and width of 44 CSS px. Adjacent targets SHALL keep a gap of at least 8 CSS px. Every focusable element SHALL show the focus ring of `design-system.md` section 7 on keyboard focus (A44).

#### Scenario: Size of a stand-alone link
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `.target-link`
- **THEN** the rule sets `display` to `inline-flex` and `min-height` and `min-width` to `var(--target)`
- **AND** `:root` sets `--target` to `44px`

#### Scenario: Gap between adjacent targets
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `.actions`
- **THEN** the rule sets `gap` to `var(--target-gap)`

#### Scenario: Focus ring
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of `:focus-visible`
- **THEN** the rule draws the ring with `--color-focus` and `--color-focus-ring`
- **AND** the rule sets a transparent outline for the forced-colors mode

### Requirement: Reflow of data
A data value in the mono face SHALL break inside the word when it does not fit its line. A page SHALL NOT scroll sideways at a width of 320 CSS px, except inside a table frame (A44).

#### Scenario: Long login in the top bar
- **GIVEN** the style sheet `/static/app.css`
- **WHEN** the style test reads the rule of the class `.data`
- **THEN** the rule sets `overflow-wrap` to `anywhere`

### Requirement: Texts only from the bundles
A template SHALL hold no literal word. Every UI text SHALL come from the `fluent` bundles `pl` and `en`. Each message key that a template uses SHALL exist in both bundles (R10, R40, A17).

#### Scenario: Template without a literal word
- **GIVEN** every template of `recipe-web`
- **WHEN** the text test removes the askama tags and the markup
- **THEN** no letter remains
- **AND** the attributes `aria-label`, `title`, `placeholder`, and `alt` hold no letter outside an askama expression

#### Scenario: Status word in both languages
- **GIVEN** the bundles `pl` and `en`
- **WHEN** the server loads them
- **THEN** the key `status-draft` gives `Szkic` in `pl` and `Draft` in `en`

### Requirement: Recipe list page
`GET /recipes` SHALL run the query `ListRecipes`, which checks `RecipeView` first. The server SHALL render the recipe list page with the page layout. A list without a recipe SHALL show the empty state sentence of the bundles (R24, R28, R39).

#### Scenario: Recipe list of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the server returns `200` with the heading `Recipes`
- **AND** the page shows the empty state sentence

#### Scenario: Recipe list without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /recipes`
- **THEN** the server returns `303` to `/login`

#### Scenario: Recipe list during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the server returns `303` to `/me/password`

#### Scenario: Recipe list in read-only mode
- **GIVEN** the application in read-only mode and the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes`
- **THEN** the server returns `200` with the recipe list page

### Requirement: Status region
Every full page SHALL hold the status region `#toast` with `role="status"` and `aria-live="polite"`. No element inside it SHALL have a role of its own. A success message SHALL go into `#toast` with an icon and one sentence, and the same answer SHALL empty `#error-panel` (R39, A44).

#### Scenario: Status region of the login page
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds the empty element `#toast` with `role="status"` and `aria-live="polite"`

#### Scenario: Success after an error
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends an HTMX `POST /users/anowak/activate` and gets `200`
- **THEN** the answer puts the icon `alert-success` and one sentence into `#toast`
- **AND** the answer empties `#error-panel`

### Requirement: Machine list page
`GET /machines` SHALL show the machine list in a table with the code, the name, and the PLC platform. The page SHALL hold a search form with `q` and `show_archived` and links to the previous and the next page. An archived machine SHALL show the state "Archived" and a link to `/machines/by-id/{uuid}`. A list without a row SHALL show the empty state (R05, R24, R39, A44).

#### Scenario: Machine list of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines`
- **THEN** the server returns `200` with a table row of `M7` and a link to `/machines/M7`
- **AND** the page holds no link to `/machines/new`

#### Scenario: Archived machine in the list
- **GIVEN** the ProcessEngineer `anowak` with an open session and the archived machine `M12`
- **WHEN** `anowak` sends `GET /machines?show_archived=1`
- **THEN** the row of `M12` shows the icon `status-retired` and the word of `status-archived`
- **AND** the row links to `/machines/by-id/` and the identifier of the archived `M12`

#### Scenario: Malformed page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends an HTMX request for `GET /machines?page=0`
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`

#### Scenario: Search text in the pager links
- **GIVEN** the ProcessEngineer `anowak` with an open session and 101 live machines with the name `a&b #+%Ł`
- **WHEN** `anowak` sends `GET /machines` with `q` set to `a&b #+%Ł`
- **THEN** the link to the next page holds the percent-encoded search text

### Requirement: Search text and pages of a list
A search text of spaces only SHALL count as no search text. A page after the last page of the machine list or the usage list SHALL show the text of `list-page-after-end`. It SHALL also show a link to the first page (R24, R39).

#### Scenario: Search text of spaces only
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines` with `q` set to three spaces
- **THEN** the server returns `200` with a table row of `M7`

#### Scenario: Page after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and one live machine
- **WHEN** `anowak` sends `GET /machines?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/machines?page=1`
- **AND** the page shows no text of the empty state

### Requirement: Content answer of a page
An HTMX request with `HX-Target: content` to a plant page SHALL get the content of `main` and the `title` element. The answer SHALL also update the field `next` of the language switch out of band. The `h1` of that answer SHALL have `tabindex="-1"` and `autofocus`. A request without `HX-Request` SHALL get the full page (R39, A44).

#### Scenario: Machine page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7` with `HX-Target: content`
- **THEN** the answer holds the `title` element, `#error-panel`, `#toast`, and the `h1` with `autofocus`
- **AND** the answer holds no top bar and no skip link
- **AND** the answer holds the field `next` with the value `/machines/M7` and `hx-swap-oob="true"`

### Requirement: Machine form and creation
`GET /machines/new` SHALL show a form with the code, the name, a list of the five PLC platforms, and the name of the main unit. The list SHALL start with an empty option, so the user chooses the platform. A valid `POST /machines` SHALL answer `303` to `/machines/{code}`. A rejected HTMX `POST /machines` SHALL put the error into `#error-panel` and SHALL keep the form with the input of the user (R05, R06, R15, R39).

#### Scenario: Create an S7-300 machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and no live machine `M7`
- **WHEN** `anowak` sends `POST /machines` with the code `M7`, the name `Press line`, the platform `S7_300`, and the main unit name `Infeed`
- **THEN** the server returns `303` to `/machines/M7`

#### Scenario: Code of a live machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines` with the code `M7`
- **THEN** the server returns `409 E-PLT-001` with `HX-Retarget: #error-panel`

#### Scenario: No platform chosen
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines/new`
- **THEN** the first option of the platform list has an empty value and is selected
- **AND** an HTMX `POST /machines` with an empty `plc_platform` gets `422 E-WEB-008` with a line for the platform field

### Requirement: Machine page
`GET /machines/{code}` SHALL show the code, the name, the PLC platform, the unit tree, and a link to the type list of the live machine. A user with `MachineManage` SHALL also get the rename form and the archive form inside a `details` element. The page of an archived machine SHALL show the state "Archived", no form, and no link to a route under `/machines/{code}` (R05, R06, R15, A32).

#### Scenario: Page of a live machine for an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the live machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the page shows `M7`, its name, its platform, and its unit tree
- **AND** the page holds no form that sends a `POST` request to `/machines/M7`

#### Scenario: Page of an archived machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and the archived machine `M12`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M12`
- **THEN** the page shows the large badge with the word of `status-archived`
- **AND** the page holds no form and no link to `/machines/M12` or to a path under it

#### Scenario: Live machine by identifier
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines/by-id/` with the identifier of `M7`
- **THEN** the page shows no badge of `status-archived`
- **AND** the page holds the rename form of `M7`

#### Scenario: Malformed machine code
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /machines/X7`
- **THEN** the server returns `404 E-WEB-005`

### Requirement: Rename and archive of a machine
The rename input SHALL hold the current name. A valid HTMX rename SHALL answer `200` with the new name, a success message in `#toast`, and an empty `#error-panel`. A valid archive SHALL answer `303` to `/machines?archived={code}`. The list page SHALL then show the success message of the archive in `#toast` (R05, R39, A14).

#### Scenario: Rename form with the current name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7` with the name `Press line`
- **WHEN** `anowak` sends `GET /machines/M7`
- **THEN** the input `machine_name` of the rename form has the value `Press line`

#### Scenario: Rename through HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/rename` with the name `Press line 2`
- **THEN** the server returns `200` with the name `Press line 2`
- **AND** the answer puts the icon `alert-success` into `#toast` and empties `#error-panel`

#### Scenario: Archive with a message
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the live machine `M7` without a draft
- **WHEN** `anowak` sends `POST /machines/M7/archive`
- **THEN** the server returns `303` to `/machines?archived=M7`
- **AND** the page `GET /machines?archived=M7` shows the sentence `Machine M7 is archived.` in `#toast`

#### Scenario: Archive with a live draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with a recipe that has a live draft
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/archive`
- **THEN** the server returns `409 E-PLT-011` with `HX-Retarget: #error-panel`

### Requirement: Unit tree with the disclosure pattern
The unit tree SHALL be nested lists inside `.tree` with no `role="tree"` and no `role="treeitem"`. Each node with children SHALL be a `button` with `aria-expanded` and `aria-controls`, next to a link to the unit page. The server SHALL render the whole tree, and the nodes of depth 1 and 2 SHALL be expanded. The targets of a row SHALL keep a gap of 8 CSS px. Two rows SHALL keep a gap of 16 CSS px (R06, R07, R39, A30, A44).

#### Scenario: Tree of five levels and 64 units
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7` with 64 live units down to `U1.2.4.5.6`
- **WHEN** `mhoffman` sends an HTMX request for `GET /machines/M7/units`
- **THEN** the fragment holds 64 links to a unit page
- **AND** the groups of the nodes of depth 3 and 4 have the attribute `hidden`
- **AND** each `aria-controls` names a group with the prefix `tree-group-` and a UUID

#### Scenario: Links to add a sub-unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5.6`
- **WHEN** `anowak` sends `GET /machines/M7`
- **THEN** the node `U1.2.4.5` holds a link to `/machines/M7/units/new?parent_path=U1.2.4.5`
- **AND** the node `U1.2.4.5.6` holds no such link

#### Scenario: Tree of an Operator
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7`
- **THEN** the tree holds no link to `/machines/M7/units/new`

#### Scenario: Gap in a row of the tree
- **GIVEN** the style sheet `/static/components.css`
- **WHEN** the style test reads the rule of a tree row
- **THEN** the rule sets `display` to `flex` and `gap` to `var(--target-gap)`

### Requirement: Addition of a unit
The link "Add a sub-unit" SHALL load the unit form into `#unit-form` with an `h2` that names the parent and `autofocus` on its first input. A valid HTMX `POST /machines/{code}/units` SHALL answer `200` with the whole tree, the ancestors of the new unit expanded, and `autofocus` on its link. The answer SHALL put a success message into `#toast` and SHALL empty `#error-panel` (R06, R07, R39, A30).

#### Scenario: Unit form in the machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7/units/new?parent_path=U1.2` with `HX-Target: unit-form`
- **THEN** the answer is the form with an `h2` that holds `U1.2`
- **AND** the first input of the form has the attribute `autofocus`

#### Scenario: New unit on level 5
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U1.2.4.5`, the segment `6`, and a name
- **THEN** the server returns `200` with the tree
- **AND** the link of `U1.2.4.5.6` has the attribute `autofocus` and its ancestors are expanded

#### Scenario: Sixth level
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the unit `U1.2.4.5.6`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units` with the parent `U1.2.4.5.6` and the segment `7`
- **THEN** the server returns `422 E-PLT-002` with `HX-Retarget: #error-panel`
- **AND** the system creates no unit

#### Scenario: All numbers in use
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with 64 units under `U1`
- **WHEN** `anowak` sends `GET /machines/M7/units/new?parent_path=U1`
- **THEN** the form shows the sentence of `unit-no-free-segment` and no create button

### Requirement: Places of the unit form
The machine page and the full page of `GET /machines/{code}/units` SHALL both hold `#unit-tree` and an empty `#unit-form`. So the link "Add a sub-unit" works on both pages (R06, R39).

#### Scenario: Add link on the full tree page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/units` without `HX-Request`
- **THEN** the page holds `#unit-tree` with the tree and an empty `#unit-form`
- **AND** the node `U1.2` holds a link to `/machines/M7/units/new?parent_path=U1.2`

### Requirement: Unit page
`GET /machines/{code}/units/{path}` SHALL show the path, the name, the parent, and the live children of the unit. A user with `UnitManage` SHALL also get the rename form and the archive form inside a `details` element with the count of the sub-units. A valid archive SHALL answer `303` to `/machines/{code}?archived={path}`, and the machine page SHALL then show the success message (R06, R07, A14, A30).

#### Scenario: Path with six segments
- **GIVEN** the Operator `mhoffman` with an open session and the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/units/U1.2.4.5.6.7`
- **THEN** the server returns `422 E-PLT-002`

#### Scenario: Archive with a message
- **GIVEN** the ProcessEngineer `anowak` with an open session, the UI language `en`, and the unused unit `U1.2` of the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/units/U1.2/archive`
- **THEN** the server returns `303` to `/machines/M7?archived=U1.2`
- **AND** the page `GET /machines/M7?archived=U1.2` shows the sentence `Unit U1.2 and its sub-units are archived.` in `#toast`

#### Scenario: Archive of the last main unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` with the one main unit `U1`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/units/U1/archive`
- **THEN** the server returns `409 E-PLT-008` with `HX-Retarget: #error-panel`

### Requirement: UDT form with member rows
`GET /machines/{code}/types/new` SHALL accept only the field `from`. The form SHALL hold the name of the struct and one `fieldset` of member rows. Each row SHALL hold a name input and a type input with a `datalist` of the types of the machine. The button "Add a member" SHALL send the form with `POST` and `add=1`. The server SHALL then create no type and SHALL show the sent rows and one more empty row (R13, R39).

#### Scenario: Add a member row
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/types` with the name `ST_Zone`, the row `temp_max` with `REAL`, and `add=1`
- **THEN** the server returns `200` with the name `ST_Zone`, the row `temp_max` with `REAL`, and one empty row
- **AND** the first input of the empty row has the id `members-1-name` and the attribute `autofocus`
- **AND** the machine `M7` holds no struct `ST_Zone`

#### Scenario: Form of a new revision
- **GIVEN** the ProcessEngineer `anowak` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new?from=ST_Zone`
- **THEN** the form holds the members of `ST_Zone@2`
- **AND** the form sends `POST /machines/M7/types/ST_Zone/revisions`

#### Scenario: CSRF token in the query
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new?csrf_token=abc`
- **THEN** the server returns `422 E-WEB-008`

### Requirement: UDT definition through the form
A valid `POST /machines/{code}/types` SHALL answer `303` to the type page. A valid `POST /machines/{code}/types/{name}/revisions` SHALL answer `303` to the type page of the new revision. The server SHALL skip a member row with an empty name and an empty type. A rejected HTMX request SHALL put the error into `#error-panel` and SHALL keep the form (R13, R19, R39).

#### Scenario: Struct with a cycle
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7` that holds two structs that contain each other
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with a member of one of the two structs
- **THEN** the server returns `422 E-PLT-005` with `HX-Retarget: #error-panel`
- **AND** the system creates no type

#### Scenario: Duplicate member name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with two members of the name `speed`
- **THEN** the server returns `422 E-PLT-021` with `HX-Retarget: #error-panel`

#### Scenario: Empty row
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `POST /machines/M7/types` with the name `ST_Zone`, the row `temp_max` with `REAL`, and one empty row
- **THEN** the server returns `303` to `/machines/M7/types/ST_Zone`
- **AND** the struct `ST_Zone@1` has one member

#### Scenario: Reserved name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the name `new` and one valid member row
- **THEN** the server returns `422 E-WEB-008` with `HX-Retarget: #error-panel`
- **AND** the system creates no type

### Requirement: Type pages
`GET /machines/{code}/types` SHALL show the struct revisions in a table. `GET /machines/{code}/types/{name}` SHALL show the type tree with the disclosure pattern, the list of revisions, and a link to the usage page. A user with `UdtManage` SHALL also get a link to the form of a new revision. `GET .../usage` SHALL show each usage row with the state of its version (R13, R19, R39, A44).

#### Scenario: Type tree page
- **GIVEN** the Operator `mhoffman` with an open session and the struct `ST_Zone@2` of the machine `M7`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone`
- **THEN** the page shows the type tree of `ST_Zone@2` and the links to the revisions 2 and 1
- **AND** the page holds no link to `/machines/M7/types/new?from=ST_Zone`

#### Scenario: Usage page with a row
- **GIVEN** the Operator `mhoffman` with an open session and the draft of the recipe `R-100` with the parameter `LINE` of the type `ST_Line@1`, which holds `ST_Zone@2`
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=2`
- **THEN** the table shows a row with `R-100`, the icon `status-draft`, and the tag `LINE`

#### Scenario: Usage page without a row
- **GIVEN** the Operator `mhoffman` with an open session and the struct `ST_Zone@1` that no parameter uses
- **WHEN** `mhoffman` sends `GET /machines/M7/types/ST_Zone/usage?rev=1`
- **THEN** the page shows the empty state sentence

### Requirement: Plant forms without user data in an hx attribute
Every HTMX request of a machine, unit, or type page SHALL come from a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute that a template of these pages writes SHALL hold a literal. No URL of these pages SHALL hold the CSRF token (R38).

#### Scenario: Rename form of a unit
- **GIVEN** the ProcessEngineer `anowak` with an open session and the unit `U1.2` of the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/units/U1.2`
- **THEN** the rename form has `action="/machines/M7/units/U1.2/rename"` and `hx-boost="true"`
- **AND** no `hx-*` attribute that the page template writes holds `M7` or `U1.2`

#### Scenario: Forms that hold the token
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends `GET /machines/M7/types/new`
- **THEN** each form that holds `csrf_token` sends `POST`
- **AND** no `href`, `action`, or field `next` of the page holds `csrf_token`
