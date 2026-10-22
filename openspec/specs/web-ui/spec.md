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

#### Scenario: Reload link without user data in an hx attribute
- **GIVEN** a conflict on the draft version 3 of the recipe `R-100`
- **WHEN** the server renders `conflict.html`
- **THEN** the button Reload version is a link with `href="/recipes/R-100/versions/3"` and `hx-boost="true"`
- **AND** no `hx-*` attribute of the fragment holds `R-100`

#### Scenario: Time and editor of the last change
- **GIVEN** the draft version 3 of the recipe `R-100`, which `anowak` changed last on 2026-10-14 at 10:15:03 UTC
- **WHEN** a ProcessEngineer `bnowicki` sends a change with an old `row_version`
- **THEN** the fragment shows the login `anowak` and the time `2026-10-14 10:15 UTC` in the format of `design-system.md` section 5
- **AND** the fragment says that the reload discards the unsaved input

### Requirement: Parameter change sets a new row version
A change of a parameter value SHALL change the `row_version` of the version, not only the parameter row. Two users who change two different parameters of one draft SHALL get a conflict on the second save (R39, I12).

#### Scenario: Two parameters of one draft
- **GIVEN** the draft version 3 of the recipe `R-100` with `row_version` 7
- **AND** a ProcessEngineer saved a parameter value, so the `row_version` became 8
- **WHEN** a second ProcessEngineer sends a change of another parameter with `row_version` 7
- **THEN** the server returns `409 E-WEB-001` with the fragment `conflict.html`

### Requirement: Second request of a form
Every approve form, every save form, and every retire, discard, delete, and restore form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second request of such a form while the first request is in flight. The approve, retire, discard, delete, and restore forms SHALL also have an indicator with the text of `form-in-progress`. They SHALL NOT have the attribute `hx-disabled-elt`, so a refused request keeps the focus on the button (R39, I12, ADR-0049).

#### Scenario: Second request while a request is in flight
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer sends the same save form again
- **THEN** the browser drops the second request

#### Scenario: Retire form
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the form has `hx-sync="closest form:drop"` and no `hx-disabled-elt`

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
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The links SHALL come in the order recipes, machines, users, audit, own profile. The link to `/audit` SHALL show when the row of `ListAuditEntries` allows the role. The use case of that page SHALL still check the permission. A session with `must_change_password` SHALL show no navigation link (R28, R39, A20).

#### Scenario: Navigation of an Operator
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, and `/me`
- **AND** the top bar shows no link to `/users` and no link to `/audit`
- **AND** the link to `/recipes` has `aria-current="page"`

#### Scenario: Navigation of an Administrator
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /me`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/users`, `/audit`, and `/me` in this order

#### Scenario: Navigation of a ProcessEngineer
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes`, `/machines`, `/audit`, and `/me` in this order

#### Scenario: Navigation during a password change
- **GIVEN** the user `mhoffman` with `must_change_password` set to true and an open session
- **WHEN** `mhoffman` sends `GET /me/password`
- **THEN** the top bar shows no navigation link
- **AND** the top bar shows the logout button

### Requirement: Current navigation link
The current link of the top bar SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700 (R39).

#### Scenario: Current link on a machine page
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /machines`
- **THEN** the link to `/machines` has `aria-current="page"`

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
- **WHEN** the text test strips the askama tags and the markup
- **THEN** no letter remains
- **AND** the attributes `aria-label`, `title`, `placeholder`, and `alt` hold no letter outside an askama expression

#### Scenario: Status word in both languages
- **GIVEN** the bundles `pl` and `en`
- **WHEN** the server loads them
- **THEN** the key `status-draft` gives `Szkic` in `pl` and `Draft` in `en`

### Requirement: Recipe list page
`GET /recipes` SHALL run the query `ListRecipes`, which checks `RecipeView` first. The server SHALL render the recipe list page with the page layout. The page SHALL show `page.recipes` rows per page with the pager. A list without a recipe SHALL show the empty state sentence of the bundles (R24, R28, R39).

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

#### Scenario: Second page of the recipe list
- **GIVEN** the ProcessEngineer `anowak` with an open session and 101 recipes
- **WHEN** `anowak` sends `GET /recipes?page=2`
- **THEN** the page shows one table row and a link to the first page

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
A search text of spaces only SHALL count as no search text. A page after the last page of a list SHALL show the text of `list-page-after-end`. The lists are the machine list, the usage list, the recipe list, and a version list. The page SHALL also show a link to the first page (R24, R39).

#### Scenario: Search text of spaces only
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `GET /machines` with `q` set to three spaces
- **THEN** the server returns `200` with a table row of `M7`

#### Scenario: Page after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and one live machine
- **WHEN** `anowak` sends `GET /machines?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/machines?page=1`
- **AND** the page shows no text of the empty state

#### Scenario: Recipe list after the end
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100`
- **WHEN** `anowak` sends `GET /recipes?page=2`
- **THEN** the page shows the text of `list-page-after-end` and a link to `/recipes?page=1`

### Requirement: Content answer of a page
An HTMX request with `HX-Target: content` to a plant page or a recipe page SHALL get the content of `main` and the `title` element. The answer SHALL also update the field `next` of the language switch out of band. The `h1` of that answer SHALL have `tabindex="-1"` and `autofocus`. A request without `HX-Request` SHALL get the full page (R39, A44).

#### Scenario: Machine page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends an HTMX request for `GET /machines/M7` with `HX-Target: content`
- **THEN** the answer holds the `title` element, `#error-panel`, `#toast`, and the `h1` with `autofocus`
- **AND** the answer holds no top bar and no skip link
- **AND** the answer holds the field `next` with the value `/machines/M7` and `hx-swap-oob="true"`

#### Scenario: Version page after a create
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of the recipe `R-100`
- **WHEN** `anowak` sends an HTMX request for `GET /recipes/R-100/versions/3` with `HX-Target: content`
- **THEN** the answer holds the `title` element and the `h1` with `autofocus`
- **AND** the answer holds no top bar

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

### Requirement: Filters of the recipe list
`GET /recipes` SHALL keep a recipe when `q` matches a part of its reference number or name with no difference of case. `machine` SHALL keep the recipes of one live machine. `status` SHALL keep a recipe with a version in that state. The rows SHALL come in the byte order of the reference number (R16, R24, R39).

#### Scenario: Search by a part of the name
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipes `R-100` "Linia A" and `R-200` "Prasa"
- **WHEN** `anowak` sends `GET /recipes?q=LINIA`
- **THEN** the page shows a row of `R-100` and no row of `R-200`

#### Scenario: Unknown state filter
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes?status=open`
- **THEN** the server returns `422 E-WEB-008`

### Requirement: Deleted recipes in the list
The recipe list SHALL hide a deleted recipe. `show_deleted=1` SHALL run `ListDeletedRecipes` with `RecipeRestore` and SHALL add the deleted recipes with the state Deleted. A user without `RecipeRestore` SHALL get `403 E-WEB-002` (A14, R28).

#### Scenario: Administrator shows the deleted recipes
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300`
- **WHEN** `admin` sends `GET /recipes?show_deleted=1`
- **THEN** the page shows a row of `R-300` with the icon `status-retired` and the word of `status-deleted`

#### Scenario: ProcessEngineer asks for the deleted recipes
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /recipes?show_deleted=1`
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one `security_event` of kind `forbidden`

### Requirement: Pages of a deleted recipe
An Administrator SHALL read the recipe page, the version pages, the diff pages, and the history of a deleted recipe. These pages SHALL show the state Deleted and no form. Every `GET` route with `{ref}` SHALL answer `409 E-RCP-012` to every other role. The approval preview of a deleted recipe SHALL answer `409 E-RCP-012` to every role with `VersionApprove` (A14, R24).

#### Scenario: Operator opens a deleted recipe
- **GIVEN** the Operator `mhoffman` with an open session and the deleted recipe `R-300`
- **WHEN** `mhoffman` sends `GET /recipes/R-300`
- **THEN** the server returns `409 E-RCP-012`

#### Scenario: Administrator opens a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300`
- **WHEN** `admin` sends `GET /recipes/R-300`
- **THEN** the server returns `200` with the state Deleted and no form

#### Scenario: History of a deleted recipe
- **GIVEN** the deleted recipe `R-300`
- **WHEN** the Administrator `admin` and the ProcessEngineer `anowak` each send `GET /recipes/R-300/history`
- **THEN** `admin` gets `200` with the history
- **AND** `anowak` gets `409 E-RCP-012`

#### Scenario: Preview of a deleted recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300` with the draft version 2
- **WHEN** `admin` sends `GET /recipes/R-300/versions/2/approve`
- **THEN** the server returns `409 E-RCP-012`

### Requirement: Recipe form and creation
`GET /recipes/new` SHALL show a form with a select of the live machines, the reference number, the name, and the change note. `POST /recipes` SHALL run `CreateRecipe` and SHALL answer `303` to `/recipes/{ref}/versions/1` (R16, R25).

#### Scenario: New recipe
- **GIVEN** the ProcessEngineer `anowak` with an open session and the live machine `M7`
- **WHEN** `anowak` sends `POST /recipes` with `machine_code=M7`, `reference_number=r-500`, a name, and a change note
- **THEN** the server returns `303` to `/recipes/R-500/versions/1`

#### Scenario: Unknown machine
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `POST /recipes` with `machine_code=M99`
- **THEN** the server returns `404 E-PLT-019` with the fragment `error.html`

### Requirement: Recipe page
`GET /recipes/{ref}` SHALL show the name, the machine, and the version list with `page.versions` rows per page. Each row SHALL show the number, the state badge, and the change note. The page SHALL show the rename form only to a user with `RecipeEdit` and the link New version only to a user with `VersionCreate` (R17, R24, R25).

#### Scenario: Operator reads a recipe
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100` with three versions
- **WHEN** `mhoffman` sends `GET /recipes/R-100`
- **THEN** the page shows three version rows with their state badges
- **AND** the page shows no form and no link to `/recipes/R-100/versions/new`

### Requirement: Rename of a recipe
`POST /recipes/{ref}/rename` SHALL run `RenameRecipe`. An HTMX request SHALL get the new name, the toast, and an empty `#error-panel`. A request without `HX-Request` SHALL get `303` to `/recipes/{ref}` (R16, R25).

#### Scenario: Rename by HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100`
- **WHEN** `anowak` sends an HTMX `POST /recipes/R-100/rename` with a new name
- **THEN** the server returns `200` with the new name and the toast

### Requirement: Version form and creation
`GET /recipes/{ref}/versions/new` SHALL show a select of the version numbers of the recipe, the choice empty draft, and the change note. `POST /recipes/{ref}/versions` SHALL run `CreateVersion`. A `based_on` that names no version SHALL give `422 E-WEB-008` with the field `based_on` (R17, A7).

#### Scenario: Base version that does not exist
- **GIVEN** the ProcessEngineer `anowak` with an open session and the recipe `R-100` with three versions
- **WHEN** `anowak` sends `POST /recipes/R-100/versions` with `based_on=9`
- **THEN** the server returns `422 E-WEB-008` with a line for the field `based_on`

#### Scenario: Snapshot check fails
- **GIVEN** the approved version 2 of `R-100` with a parameter on the archived unit `U3`
- **WHEN** `anowak` sends `POST /recipes/R-100/versions` with `based_on=2`
- **THEN** the server returns `422 E-RCP-020` with one line for each item

### Requirement: Version page
`GET /recipes/{ref}/versions/{n}` SHALL show the header of the version and the parameter table with `page.parameters` rows grouped by unit. The rows SHALL come in the order unit path, ordinal, and tag. Only a live `Draft` and a user with `ParameterEdit` SHALL get the edit forms of parameters. The page SHALL link to the history of the recipe (R17, R24, R25, A28).

#### Scenario: Operator reads a draft
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameter table and no form
- **AND** the page shows no link to `/recipes/R-100/versions/3/approve`
- **AND** the page shows a link to `/recipes/R-100/history`

#### Scenario: Large draft
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft with 4 000 parameters in 40 units
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** the page shows the parameters of one unit and a link to each unit with the count of its parameters
- **AND** the page shows at most 100 parameter rows
- **AND** the link of the shown unit has `aria-current="page"`

### Requirement: Actions of a version page
The version page SHALL show an action form or an action link when the matrix row of its command allows the role. The state of the version SHALL also permit the transition (R24, R25, R28).

#### Scenario: Administrator reads an approved version
- **GIVEN** the Administrator `admin` with an open session and the approved version 2 of `R-100`
- **WHEN** `admin` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows the retire form in a `details` element with the class `danger`
- **AND** the page shows no edit form of a parameter

### Requirement: Form of a new parameter
The link Add a parameter of the version page SHALL load the form of a new parameter into `#parameter-form`. The button Show value fields SHALL run `GetParameterForm` with the sent type and SHALL write nothing. `AddParameter` SHALL answer the row of the new parameter and SHALL clear `#parameter-form` out of band (R08, R25).

#### Scenario: Value fields of a typed array
- **GIVEN** the ProcessEngineer `anowak` with the form of a new parameter of the draft version 3 of `R-100`
- **WHEN** `anowak` sends the form with `type=ARRAY[1..8] OF INT` and `fields=1`
- **THEN** the server returns `200` with the form and eight value rows `[1]` to `[8]` and one row `[*]`
- **AND** the draft keeps its `row_version`

#### Scenario: New parameter saved
- **GIVEN** the ProcessEngineer `anowak` with the form of a new parameter of type `INT` and a value
- **WHEN** `anowak` sends the form
- **THEN** the server returns `200` with the row of the new parameter and the header `HX-Trigger` with the new `row_version`

#### Scenario: Value fields in read-only mode
- **GIVEN** the application in read-only mode and the form of a new parameter
- **WHEN** the ProcessEngineer `anowak` sends the form with `fields=1`
- **THEN** the server returns `503 E-WEB-006`

### Requirement: Form rendered again keeps the input
A form that the server renders again without a write SHALL keep every sent text and the sent `row_version`. Only a `GET` of the page SHALL give a new `row_version`. A save from a stale form SHALL get `409 E-WEB-001` (R31, R39, I12).

#### Scenario: Limit mode switch after another save
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `ZONES` at `row_version` 7, and a second user saved the draft at `row_version` 8
- **WHEN** `anowak` sends the form with `limit_mode=[*]`, then saves the answer form
- **THEN** the switch answer holds the sent texts and `row_version` 7
- **AND** the save gets `409 E-WEB-001`

#### Scenario: Type changed without the value fields
- **GIVEN** the edit form of `SPEED` with `value_type=INT`
- **WHEN** `anowak` sends the form with `type=REAL` and no `fields`
- **THEN** the server returns `200` with the value fields of `REAL` and a notice, and writes nothing

### Requirement: Leaf grid
The value and the limits of a parameter SHALL show in one table with one row per leaf. A row SHALL show the leaf path, the type, the value, and only the limit inputs of the class of its leaf. Each input SHALL name its row header and its column header in `aria-labelledby` (R09, R13, A36, A44).

#### Scenario: Struct in a struct
- **GIVEN** the parameter `LINE_A` of type `ST_Line@1` with the member `zone` of type `ST_Zone@2` in the draft version 3 of `R-100`
- **WHEN** the ProcessEngineer `anowak` opens the page of `LINE_A`
- **THEN** the grid shows the rows `.name`, `.speed`, `.zone.setpoint`, `.zone.hold`, and `.zone.enabled`
- **AND** the row `.zone.enabled` shows a select with TRUE and FALSE and no limit input

#### Scenario: Time leaf as an IEC literal
- **GIVEN** the leaf `.zone.hold` of type `TIME` with the stored count 90000
- **WHEN** the ProcessEngineer `anowak` opens the page of `LINE_A`
- **THEN** the value input of `.zone.hold` holds `T#1m30s`

#### Scenario: Multidimensional array
- **GIVEN** the parameter `GRID` of type `ARRAY[0..1,1..2] OF REAL`
- **WHEN** the ProcessEngineer `anowak` opens the page of `GRID`
- **THEN** the grid shows the rows `[0,1]`, `[0,2]`, `[1,1]`, and `[1,2]` and one row `[*]`

### Requirement: Limit modes of an array level
Each array level of the grid SHALL have one limit for all elements, in a row `[*]`, or a limit per element. The page SHALL show a limit per element when the limits hold an index at that level. A button of the level SHALL switch the mode and SHALL write nothing (R09, A36).

#### Scenario: Switch to a limit per element
- **GIVEN** the parameter `ZONES` of type `ARRAY[1..4] OF ST_Zone@2` with a limit 20 to 250 on `[*].setpoint`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with `limit_mode=[*]`
- **THEN** the server returns `200` with limit inputs on each row `[1].setpoint` to `[4].setpoint`
- **AND** each of these inputs holds the limit 20 to 250
- **AND** the draft keeps its `row_version`

#### Scenario: Switch back with unequal limits
- **GIVEN** the limits 20 to 250 on `[1].setpoint` and 30 to 260 on `[2].setpoint`
- **WHEN** `anowak` switches the level back to one limit for all elements
- **THEN** the row `[*]` holds empty limit inputs and the page shows a notice

### Requirement: Groups of a large array
An array with more than `form.array_chunk` elements SHALL split into groups with a toggle button of the disclosure pattern, unless it stands inside a group. An array inside a group SHALL show all its rows in that group, with no toggle of its own. A group SHALL be closed, except the first group and each group with an error line. An input in a closed group SHALL submit with the form (R13, A26, A44).

#### Scenario: Array of 1 000 elements
- **GIVEN** the parameter `PROFILE` of type `ARRAY[1..1000] OF INT`
- **WHEN** the ProcessEngineer `anowak` opens the page of `PROFILE`
- **THEN** the grid shows ten groups and only the first group is open
- **AND** each toggle button has `aria-expanded` and `aria-controls`

#### Scenario: Array inside a group
- **GIVEN** the parameter `ZONE_TABLE` of type `ARRAY[1..120] OF ST_Zone`, where `ST_Zone` holds the member `profile` of type `ARRAY[1..120] OF INT`
- **WHEN** the ProcessEngineer `anowak` opens the page of `ZONE_TABLE`
- **THEN** the grid shows two groups of the outer array and no other group
- **AND** the first group holds the 120 rows of `profile` of each of its elements
- **AND** no `tbody` of the grid holds another `tbody`

### Requirement: Value fields of a form
A form SHALL send one value field per leaf with the name `value` and the leaf path. A key outside the fields of the route, a malformed leaf path, a `[*]` segment, or a duplicate key SHALL give `422 E-WEB-008`. A missing leaf or a leaf path that the type does not hold SHALL give `422 E-RCP-005`. The server SHALL keep the text of a `STRING` or `CHAR` leaf as sent (R38, A11, A43).

#### Scenario: Partial value
- **GIVEN** the parameter `SPEED_TABLE` of type `ARRAY[1..8] OF INT`
- **WHEN** the ProcessEngineer `anowak` sends the edit form without the field `value[5]`
- **THEN** the server returns `422 E-RCP-005` with a line for the leaf `[5]`

#### Scenario: Spaces in a STRING leaf
- **GIVEN** the leaf `.name` of type `STRING(20)`
- **WHEN** `anowak` sends `value.name` with the text `  Linia A  `
- **THEN** the stored value of `.name` is `  Linia A  `

#### Scenario: Flood of pairs
- **GIVEN** the parameter `SPEED` of type `INT`
- **WHEN** `anowak` sends the edit form with 100 000 extra pairs
- **THEN** the server returns `422 E-WEB-008` with one line and reads no more pairs

### Requirement: One edit form of a parameter
The parameter page of a live `Draft` SHALL hold one edit form with the definition fields, the S7 address, and the leaf grid. The form SHALL send the value and the limits together to `/definition`. The page SHALL hold no second form with a definition field of that parameter (R09, R25, I12).

#### Scenario: Value and limits in one save
- **GIVEN** the parameter `SPEED` of type `INT` with the value 1500 and the limit 0 to 3000
- **WHEN** the ProcessEngineer `anowak` sends the edit form with the value 3500 and the limit 3000 to 4000
- **THEN** the server returns `200` with the row of `SPEED` and the header `HX-Trigger`
- **AND** the answer swaps the form out of band with the saved texts and no error line

#### Scenario: Enter saves
- **GIVEN** the edit form of `SPEED`
- **WHEN** the browser sends the form for Enter in a value input
- **THEN** the request holds no field `fields` and no field `limit_mode`

### Requirement: S7 address field per platform
The parameter forms SHALL show the S7 address as a required field on `S7_300` and as an optional field on `S7_1200` and `S7_1500`. They SHALL show no S7 address field on `CodesysIec` and `TwinCat3` (R14, A31).

#### Scenario: TwinCAT machine
- **GIVEN** the ProcessEngineer `anowak` with an open session and a draft of a recipe of a live machine with the platform `TwinCat3`
- **WHEN** `anowak` opens the form of a new parameter
- **THEN** the form holds no field `s7_address`

#### Scenario: S7-300 machine
- **GIVEN** a draft of a recipe of the machine `M7` with the platform `S7_300`
- **WHEN** `anowak` opens the form of a new parameter
- **THEN** the label of the field `s7_address` holds the word for required

### Requirement: Translation rows
The parameter page SHALL show one translation row per tag of `i18n.translation_langs` and per other tag of the parameter. Each row SHALL be one form that sends `POST` to `.../translations/{lang}`. A deletion SHALL send `POST` to `.../translations/{lang}/delete` from a danger disclosure (R10, A17).

#### Scenario: Polish translation saved
- **GIVEN** the parameter `LINE` of the draft version 3 of `R-100` and `i18n.translation_langs` with `pl`
- **WHEN** the ProcessEngineer `anowak` sends the row `pl` with a name and a description
- **THEN** the server returns `200` with the translation fragment and the header `HX-Trigger`

#### Scenario: Tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl`
- **WHEN** `anowak` sends `POST .../translations/de`
- **THEN** the server returns `422 E-WEB-008`

#### Scenario: Delete a tag outside the list
- **GIVEN** `i18n.translation_langs` with `pl` and the parameter `LINE` with a translation `de`
- **WHEN** `anowak` sends `POST .../translations/de/delete`
- **THEN** the server returns `200` and `LINE` has no translation `de`

### Requirement: Deletion of a parameter
`POST .../parameters/{key}/delete` SHALL run `RemoveParameter`. The form SHALL stand in a danger disclosure whose button names the tag. The answer SHALL be the parameter table of the version (R23, R25).

#### Scenario: Delete a parameter
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/versions/3/parameters/<key>/delete` with the current `row_version`
- **THEN** the server returns `200` with the parameter table without `LINE`

### Requirement: Rebind form of an outdated draft
The page of a draft with outdated parameters SHALL list them with the state Outdated. It SHALL show one rebind form per outdated struct revision. The form SHALL hold one field per new member leaf of each affected parameter (R19, A29).

#### Scenario: New member without a value
- **GIVEN** the draft version 3 of `R-100` with the parameter `ZONE_1` of type `ST_Zone@2` and the newest revision `ST_Zone@3` with the new member `ramp`
- **WHEN** the ProcessEngineer `anowak` sends the rebind form without the field of `.ramp`
- **THEN** the server returns `422 E-RCP-028` with one line for `ZONE_1`

### Requirement: Bound of the rebind forms
`GetVersion` SHALL compute the new member leaves of a rebind only for a user with `ParameterEdit` on a live `Draft`. The sum of the new member fields of all rebind forms of one version page SHALL be at most `form.rebind_leaves`. Over the bound, each rebind form SHALL show the notice `rebind-too-large` and SHALL hold no field of a new member and no submit button (A26).

#### Scenario: Operator reads an outdated draft
- **GIVEN** the Operator `mhoffman` with an open session and the draft version 3 of `R-100` with the outdated parameter `ZONE_1`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/3`
- **THEN** the page lists `ZONE_1` with the state Outdated
- **AND** the page shows no rebind form and no field of a new member

#### Scenario: Rebind over the bound
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100`
- **AND** the outdated parameters of the draft need 20 000 new member leaves for their rebind
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/3`
- **THEN** each rebind form shows the notice `rebind-too-large` with the bound 16 384
- **AND** the page holds no field of a new member and no rebind submit button

### Requirement: Every field error at once
A form answer SHALL list every bad field and every fault of the value and the limits at once. The answer SHALL put the summary into `#error-panel` and SHALL swap the form out of band with an error line under each bad input. The summary SHALL show at most 20 lines and the count of the others. This SHALL apply to the recipe forms and the plant forms (R36, R39, A44).

#### Scenario: Two bad leaves
- **GIVEN** the parameter `SPEED_TABLE` with the limit 0 to 3000 on `[*]`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with 3500 in `[3]` and 4000 in `[6]`
- **THEN** the server returns `422 E-RCP-004` with two lines in `#error-panel`
- **AND** the inputs of `[3]` and `[6]` have `aria-invalid="true"` and an error line

#### Scenario: Two bad member rows of a UDT
- **GIVEN** the ProcessEngineer `anowak` with an open session and the machine `M7`
- **WHEN** `anowak` sends an HTMX `POST /machines/M7/types` with the bad member names `9a` and `9b`
- **THEN** the server returns `422 E-WEB-008` with two lines
- **AND** each member name input has an error line under it

### Requirement: Refused form page and toast
A request without HTMX with field errors of a form and a session SHALL get a full page in the layout of the session. The page SHALL hold the navigation, the CSRF token in `hx-headers`, and the login. It SHALL hold the summary with a link to each input, one `h1`, and the form with the sent texts and the error lines. Every other error and a request without a session SHALL get the full error page. Every HTMX error answer SHALL end with an empty `#toast` out of band (R36, R39, A44).

#### Scenario: Refused form without HTMX
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `POST /recipes` without `HX-Request`, with an empty machine code, and with the name `Kept name`
- **THEN** the server returns `422` with the navigation, the login, and `hx-headers` with the CSRF token
- **AND** `#error-panel` holds a line that links to `#machine_code`
- **AND** the page holds one `h1` and the recipe form with the name `Kept name` and the error lines
- **AND** the application writes nothing

#### Scenario: Other error without HTMX
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends a form route without `HX-Request` and gets `403 E-WEB-002`
- **THEN** the answer is the full error page with no form

#### Scenario: Error answer after a success toast
- **GIVEN** the ProcessEngineer `anowak` with a success message in `#toast` of the version page
- **WHEN** `anowak` sends an HTMX save that gets `422`, `409`, or `404`
- **THEN** the answer ends with an empty `#toast` element with `hx-swap-oob="innerHTML"`
- **AND** `#toast` keeps `role="status"` and `aria-live="polite"`

### Requirement: Permission before the fields of a form route
A route of a form SHALL run the form query of its command before it reads its fields. The form query SHALL check the permission of the command that it serves. A user without that permission SHALL get `403 E-WEB-002` and one `forbidden` event that names the command, and SHALL get no form. This SHALL also apply to a field that the form does not know (R25, R28).

#### Scenario: Operator with a bad field on a form route
- **GIVEN** the Operator `mhoffman` with an open session and the recipe `R-100`
- **WHEN** `mhoffman` sends an HTMX `POST /recipes/R-100/rename` with an empty name and the unknown field `extra`
- **THEN** the server returns `403 E-WEB-002` and not `422`
- **AND** `security_event` holds one `forbidden` event that names `RenameRecipe`
- **AND** the body holds no form and no input

#### Scenario: Operator on a parameter route
- **GIVEN** the Operator `mhoffman` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `mhoffman` sends `POST .../parameters/<key>/translations/pl` with the unknown field `extra`
- **THEN** the server returns `403 E-WEB-002` with no translation form
- **AND** `security_event` holds one `forbidden` event that names `UpsertTranslation`

### Requirement: Body limit per route
The four routes that carry a parameter value SHALL accept a body up to `http.body_limit.value`. The route that writes a translation SHALL accept up to `http.body_limit.translation`. Every other route SHALL accept up to `http.body_limit.form`. A larger body SHALL give `413 E-WEB-003` (R30, R38).

#### Scenario: Value of 16 384 leaves
- **GIVEN** the parameter `PROFILE` of type `ARRAY[1..16384] OF INT`
- **WHEN** the ProcessEngineer `anowak` sends the edit form with all leaves, a body of about 500 KiB
- **THEN** the server does not answer `413`

#### Scenario: Large rename
- **GIVEN** the recipe `R-100`
- **WHEN** `anowak` sends `POST /recipes/R-100/rename` with a body of 300 KiB
- **THEN** the server returns `413 E-WEB-003`

### Requirement: Recipe pages without user data in an hx attribute
Every HTMX request of a recipe, version, or parameter page SHALL come from a form `action` or a link `href` with `hx-boost="true"`. Each `hx-*` attribute of these pages SHALL hold a literal. No URL SHALL hold the CSRF token. Every form that holds `csrf_token` SHALL send `POST` (R38).

#### Scenario: Parameter page
- **GIVEN** the ProcessEngineer `anowak` with an open session and the parameter `LINE` of the draft version 3 of `R-100`
- **WHEN** `anowak` opens the page of `LINE`
- **THEN** no `hx-*` attribute of the page holds `R-100` or the key of `LINE`
- **AND** each form that holds `csrf_token` sends `POST`

### Requirement: Messages of the recipe codes
Every error code that a recipe, version, or parameter route returns SHALL have a message in the bundles `pl` and `en`. Each word of a message, such as a state, a budget, a platform, or an expected form, SHALL come from the bundles (R10, R36, R40).

#### Scenario: Value of the wrong shape in Polish
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` sends the text `abc` for a leaf of type `INT`
- **THEN** the line of `E-RCP-005` holds only Polish words and the data of the request

### Requirement: Approval preview page
The approval preview SHALL show the diff against the current Approved version with the limits first. With the approve form, it SHALL show the outdated parameters and the version that the approval retires. The checkbox `allow_outdated` SHALL show only with an outdated parameter. Without a blocker, the page SHALL hold one approve form with `csrf_token`, `row_version`, and `hx-sync="closest form:drop"` (R27, A29, A34, ADR-0049).

#### Scenario: Preview of a changed limit
- **GIVEN** the approved version 1 of `R-100` and its draft version 2, in which `SPEED_TABLE` has the limit row `[*]` with `min` 10 in place of 0
- **WHEN** the Administrator `admin` sends `GET /recipes/R-100/versions/2/approve`
- **THEN** the limits section shows one row of `SPEED_TABLE` with the path `[*]`
- **AND** the old limit text of the row holds the minimum `0`, and the new limit text holds the minimum `10`
- **AND** the row shows the word of `diff-state-limit` and the icon `diff-limit`
- **AND** the page names version 1 as the version that the approval retires

#### Scenario: Outdated parameter
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the page lists `ZONES` and shows the checkbox `allow_outdated`

#### Scenario: No outdated parameter
- **GIVEN** the draft version 2 of `R-100` without an outdated parameter
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the page shows no checkbox `allow_outdated`

### Requirement: Blockers on the approval preview
The approval preview SHALL run the checks of the approval without a write. Separation of duties can be on, and the user can be the creator or an editor of the draft. Then the page SHALL show the notice of `E-RCP-015`. For each fault of `check_for_approval`, the page SHALL show one line with the code and a link to the parameter page. With a blocker, the page SHALL show no approve form (R27, A34, A36, ADR-0049).

#### Scenario: Editor of the draft opens the preview
- **GIVEN** the draft version 2 of `R-100`, which the ProcessEngineer `anowak` edited
- **WHEN** `anowak` opens the approval preview of version 2
- **THEN** the page shows the code `E-RCP-015` and no approve form

#### Scenario: Missing limit
- **GIVEN** the draft version 2 of `R-100` with a numeric leaf of `HOLD` without a limit
- **WHEN** the ProcessEngineer `jsmith` opens the approval preview of version 2
- **THEN** the page shows a line of `E-RCP-016` that links to the parameter page of `HOLD`
- **AND** the page shows no approve form

### Requirement: Bound of the blocker list
The blocker notice of the approval preview SHALL show at most 20 lines. When more lines exist, the notice SHALL show one sentence with the count of the other lines (R30, ADR-0041, ADR-0049).

#### Scenario: More than 20 blocker lines
- **GIVEN** a draft with 25 numeric leaves without a limit
- **WHEN** the Administrator `admin` opens the approval preview of the draft
- **THEN** the page shows 20 lines of `E-RCP-016`
- **AND** the page shows one sentence with the count 5

### Requirement: Layout of the approval preview
The blockers and the notices of the approval preview SHALL stand right after the header. The blockers SHALL stand in one notice with the heading of `approve-blockers-heading` and without `role="alert"`. Without a blocker, the first item of the summary SHALL link to the heading of the approve form. In a section, only the first row of a parameter SHALL link its tag (R39, A44, ADR-0049).

#### Scenario: Blockers before the diff
- **GIVEN** the draft version 2 of `R-100`, which the ProcessEngineer `anowak` edited
- **WHEN** `anowak` opens the approval preview of version 2
- **THEN** the notice of the blockers stands before the summary of the diff
- **AND** no element of the page has `role="alert"`

#### Scenario: Link to the approve form
- **GIVEN** the draft version 2 of `R-100` without a blocker
- **WHEN** the Administrator `admin` opens the approval preview of version 2
- **THEN** the first item of the summary shows the text of `approve-go-to-form` and links to the heading of the approve form

### Requirement: Answer of the approval
`POST /recipes/{ref}/versions/{n}/approve` SHALL run `ApproveVersion`. A success SHALL answer `200` with the version page and a toast that names the approved version and the retired version. An HTMX answer SHALL set `HX-Push-Url` to the path of the version page. When the answer holds a toast, its `h1` SHALL have `aria-describedby="toast"`. An `AlreadyApproved` result SHALL answer the same page with the toast `version-already-approved` (R27, R39, I12, ADR-0049).

#### Scenario: Approval by HTMX
- **GIVEN** the draft version 2 of `R-100`, which `anowak` edited, and the approved version 1
- **WHEN** the Administrator `admin` sends an HTMX approval of version 2 with its `row_version`
- **THEN** the server returns `200` with the version page of version 2 and a toast that names versions 2 and 1
- **AND** the answer sets `HX-Push-Url` to `/recipes/R-100/versions/2`
- **AND** the `h1` of the answer has `aria-describedby="toast"`
- **AND** the audit log holds one entry `VersionApproved` of version 2

#### Scenario: Double click
- **GIVEN** an approval of version 2 of `R-100` that succeeded with `row_version` 8
- **WHEN** `admin` sends the same approval with `row_version` 8 again
- **THEN** the server returns `200` with the toast `version-already-approved`
- **AND** the audit log still holds one entry `VersionApproved` of version 2

### Requirement: Errors of the approval
The approval SHALL answer `409 E-RCP-015` to the creator and to each editor of the draft while separation of duties is on. It SHALL answer `409 E-WEB-001` with `conflict.html` for a stale `row_version` in the order of the next requirement. It SHALL answer `422 E-RCP-024` for an outdated parameter without `allow_outdated`. An Operator SHALL get `403 E-WEB-002`, and the system SHALL write one security event of kind `forbidden` (R27, R28, A29, A34).

#### Scenario: Creator approves the own draft
- **GIVEN** the draft version 2 of `R-100`, which `anowak` created
- **WHEN** `anowak` sends the approval of version 2
- **THEN** the server returns `409 E-RCP-015` and writes no audit entry

#### Scenario: Two sessions approve one draft
- **GIVEN** the approval preview of version 2 in two sessions of two Administrators
- **WHEN** the first approves and the second sends the approval with the same `row_version`
- **THEN** the second gets `200` with the toast `version-already-approved`

#### Scenario: Draft changed after the preview
- **GIVEN** the approval preview of version 2 of `R-100` with `row_version` 7 in the session of `admin`
- **WHEN** `anowak` saves a value of version 2 in a second session and `admin` then sends the approval with `row_version` 7
- **THEN** `admin` gets `409 E-WEB-001` with `conflict.html`
- **AND** version 2 stays a draft

#### Scenario: Outdated parameter without the flag
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`
- **WHEN** `admin` sends the approval of version 2 without `allow_outdated`
- **THEN** the server returns `422 E-RCP-024` in `#error-panel`
- **AND** the summary line shows the label of `field-allow-outdated` and links to the checkbox `allow_outdated`

#### Scenario: Operator sends an approval
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `POST /recipes/R-100/versions/2/approve` with a valid CSRF token
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one security event of kind `forbidden` that names `ApproveVersion`

### Requirement: Order of the approval errors
The approval SHALL check the draft once before its transaction, and before the compare of the `row_version`. A fault of that first check SHALL give its own code, also with a stale `row_version`. Under the locks, the approval SHALL compare the `row_version` before the second check, so a stale `row_version` there SHALL give `409 E-WEB-001` (R27, R28, A29).

#### Scenario: Stale row version and an outdated parameter
- **GIVEN** the draft version 2 of `R-100` with the outdated parameter `ZONES`, and a `row_version` that a later save made stale
- **WHEN** `admin` sends the approval of version 2 with the stale `row_version` and without `allow_outdated`
- **THEN** the server returns `422 E-RCP-024` with the line in the summary and no approve form
- **AND** the summary line has no link, and version 2 stays a draft

### Requirement: Danger actions of a version
The page of an Approved version SHALL show the retire action to a role with `VersionRetire`. The page of a live `Draft` SHALL show the discard action to a role with `VersionDiscard`. Each action SHALL be one `details` element with the class `danger`. Its form SHALL hold `csrf_token`, `row_version`, and `hx-sync="closest form:drop"`. The button SHALL name the version (R17, R39, A28, ADR-0049).

#### Scenario: ProcessEngineer reads an approved version
- **GIVEN** the ProcessEngineer `anowak` with an open session and the approved version 2 of `R-100`
- **WHEN** `anowak` sends `GET /recipes/R-100/versions/2`
- **THEN** the page shows no retire form

#### Scenario: Discard of a draft
- **GIVEN** the draft version 3 of `R-100`
- **WHEN** the ProcessEngineer `anowak` sends the discard form of version 3
- **THEN** the server returns `303` to `/recipes/R-100`
- **AND** version 3 has the state Discarded

#### Scenario: Retire with a stale row version
- **GIVEN** the approved version 2 of `R-100` with `row_version` 9
- **WHEN** `admin` sends the retire form of version 2 with `row_version` 8
- **THEN** the server returns `409 E-WEB-001` with `conflict.html`

### Requirement: Notice of a danger action
The notice of a danger action SHALL have the id `danger-notice` and no role. The danger button SHALL name the notice in `aria-describedby`. A page SHALL hold at most one danger action (R39, A44, ADR-0049).

#### Scenario: Notice of the retire action
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the page holds one element with the id `danger-notice`
- **AND** the retire button has `aria-describedby="danger-notice"`

### Requirement: Delete action of a recipe
The recipe page SHALL show the delete action to a role with `RecipeDelete` in one `details` element with the class `danger`. Its form SHALL hold `csrf_token` and `hx-sync="closest form:drop"` and no `row_version`. A success SHALL answer `303` to `/recipes` (R23, A13, ADR-0049).

#### Scenario: ProcessEngineer and a retired version
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** the ProcessEngineer `anowak` sends the delete form of `R-100`
- **THEN** the server returns `409 E-RCP-026`

#### Scenario: Administrator and an approved version
- **GIVEN** the recipe `R-100` with the approved version 2
- **WHEN** the Administrator `admin` sends the delete form of `R-100`
- **THEN** the server returns `409 E-RCP-010`

#### Scenario: Administrator and only retired versions
- **GIVEN** the recipe `R-100` with the retired version 1 and no approved version
- **WHEN** `admin` sends the delete form of `R-100`
- **THEN** the server returns `303` to `/recipes`

### Requirement: Restore from the list of deleted recipes
The list `/recipes?show_deleted=1` SHALL show a restore form in each deleted row. The form SHALL hold `csrf_token` and `hx-sync="closest form:drop"`. Its secondary button SHALL name the reference number. A success SHALL answer `303` to `/recipes/{ref}` (R26, A14, ADR-0049).

#### Scenario: Administrator restores a recipe
- **GIVEN** the Administrator `admin` with an open session and the deleted recipe `R-300` on a live machine
- **WHEN** `admin` sends the restore form of `R-300`
- **THEN** the server returns `303` to `/recipes/R-300`

#### Scenario: Restore on an archived machine
- **GIVEN** the deleted recipe `R-300` on the archived machine `M12`
- **WHEN** `admin` sends the restore form of `R-300`
- **THEN** the server returns `409 E-PLT-010`

### Requirement: Diff page
`GET /recipes/{ref}/versions/{n}/diff/{m}` SHALL show the changes from version `m` to version `n`. The sections SHALL be limits, values, definitions, translations, added parameters, and parameters only in the old version. A summary SHALL give the counts and link to each section. The table of the limits section SHALL have a border of `--color-error-edge`. Each change SHALL show the color, the icon, and the word of its state (R27, R39, A43, ADR-0050).

#### Scenario: Changed value inside a struct
- **GIVEN** the versions 1 and 2 of `R-100`, in which `LINE_A` has `.zone.setpoint` 85.5 in version 1 and 90 in version 2
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/versions/2/diff/1`
- **THEN** the values section shows a row of `LINE_A` with the path `.zone.setpoint`, the old text `85.5`, and the new text `90`

#### Scenario: Unknown kind
- **GIVEN** the versions 1 and 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2/diff/1?kind=other`
- **THEN** the server returns `422 E-WEB-008`

#### Scenario: Same version
- **GIVEN** the version 2 of `R-100`
- **WHEN** `mhoffman` sends `GET /recipes/R-100/versions/2/diff/2`
- **THEN** the page shows the empty state of the diff

### Requirement: Pages of a large diff
A page of a diff SHALL show at most `page.diff` parameters over all its sections. A section SHALL show at most `diff.leaf_rows` rows of one parameter. It SHALL show the count of the other rows with links to the parameter pages of both versions. A page after the last page of a section SHALL show the text of `list-page-after-end` (R30, A26, ADR-0050).

#### Scenario: Array of 2 000 changed elements
- **GIVEN** two versions of `R-100` that differ in 2 000 elements of one array parameter
- **WHEN** a user opens the values section of the diff
- **THEN** the section shows 1 000 rows of that parameter and the count 1 000 of the other rows

### Requirement: Review state of a paged preview
Each link of the approval preview to another page SHALL carry the query key `rv` with the `row_version` of its first page. The approve form SHALL send that value as `row_version`. A later page can read another `row_version`. Then the page SHALL show a notice and a link to the first page, and no approve form (R31, I12, ADR-0050).

#### Scenario: Draft changes during the review
- **GIVEN** the approval preview of version 2 of `R-100` with `rv=7` on its second page link
- **WHEN** a draft command commits and the Administrator `admin` opens the second page
- **THEN** the page shows the notice of a changed draft and a link to the first page
- **AND** the page shows no approve form

### Requirement: Recipe history page
`GET /recipes/{ref}/history` SHALL show the audit entries of the recipe, newest first, at most `page.history` rows, with the cursor pager. A row SHALL show the time, the actor, the action, and the label of the entity, and no state. For a role with `AuditView` or `AuditViewRecipes`, each row SHALL link to `/audit/{id}` (R20, R24, A20, ADR-0051).

#### Scenario: Operator reads the history
- **GIVEN** the recipe `R-100` with the approval of version 2
- **WHEN** the Operator `mhoffman` sends `GET /recipes/R-100/history`
- **THEN** the page shows a row with the word of the action `VersionApproved`
- **AND** the page shows no link to `/audit/`

#### Scenario: ProcessEngineer reads the history
- **GIVEN** the recipe `R-100` with the approval of version 2
- **WHEN** the ProcessEngineer `anowak` sends `GET /recipes/R-100/history`
- **THEN** each row links to its page `/audit/{id}`

### Requirement: Audit list page
`GET /audit` SHALL show a filter form that sends `GET` without a token and without `before`, and the list of entries with the cursor pager. The selects of `entity_type` and `action` SHALL offer only the values of the scope of the role. For a role with `AuditView`, the page SHALL link to `/security-events` (R20, R28, A20, ADR-0051).

#### Scenario: Operator asks for the audit list
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /audit`
- **THEN** the server returns `403 E-WEB-002`
- **AND** the system writes one security event of kind `forbidden`

#### Scenario: Filter of a ProcessEngineer
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /audit`
- **THEN** the select of `entity_type` offers only `recipe`, `recipe_version`, and `parameter`
- **AND** the page shows no link to `/security-events`

#### Scenario: Last page
- **GIVEN** the Administrator `admin` and an audit log with 30 entries
- **WHEN** `admin` sends `GET /audit`
- **THEN** the page shows 30 rows, the link "Newest entries", and the text of `audit-no-older`

### Requirement: Filter values of the audit list page
The filter form of `/audit` SHALL be a boosted form with literal `hx-*` attributes. A malformed filter value SHALL answer `422 E-WEB-008` in `#error-panel` with the label of the field, and the form SHALL stay. A well-formed `actor` that names no user SHALL give the empty list. A `to` of 9999-12-31 SHALL set no upper bound (R38, A42, ADR-0051).

#### Scenario: Malformed date in the filter
- **GIVEN** the Administrator `admin` on the audit list
- **WHEN** `admin` sends the filter form by HTMX with `from` set to `2026-13-01`
- **THEN** the server returns `422 E-WEB-008` in `#error-panel` with the label of `field-from`
- **AND** the filter form stays on the page

#### Scenario: Actor that names no user
- **GIVEN** the Administrator `admin` and no user with the login `ghost`
- **WHEN** `admin` sends `GET /audit?actor=ghost`
- **THEN** the server returns `200` with the text of `audit-empty`

#### Scenario: Last day of the calendar
- **GIVEN** the Administrator `admin` and an audit log with 30 entries
- **WHEN** `admin` sends `GET /audit?to=9999-12-31`
- **THEN** the page shows the 30 rows

### Requirement: Audit entry page
`GET /audit/{id}` SHALL show the fields of the entry and `before` and `after` as formatted JSON of the stored record in a `pre` element. Each text SHALL be escaped. The page SHALL show no `entry_hash` and no `prev_hash`, no password hash, no session token, and no cookie value. It SHALL show `chain_seq`, `client_ip`, and `session_ref` only to a role with `AuditView` (R20, A20, A43, `reliability.md` 1.1, ADR-0051).

#### Scenario: Entry with markup in a state
- **GIVEN** an entry `RecipeRenamed` whose `after` holds the name `<script>alert(1)</script>`
- **WHEN** the Administrator `admin` opens the page of the entry
- **THEN** the page holds the text `&#60;script&#62;` inside the `pre` element and no `<script>` element of the name

#### Scenario: ProcessEngineer reads a recipe entry
- **GIVEN** the entry `RecipeRenamed` of `R-100`
- **WHEN** the ProcessEngineer `anowak` opens the page of the entry
- **THEN** the page shows no chain position, no client address, and no session reference

#### Scenario: Entry outside the recipe scope
- **GIVEN** the entry `MachineRenamed` of the machine `M12`
- **WHEN** the ProcessEngineer `anowak` sends `GET /audit/{id}` of that entry
- **THEN** the server returns `404 E-WEB-005`

### Requirement: Security event page
`GET /security-events` SHALL show the filter form and the security events, newest first, with the cursor pager. A row SHALL show the time, the kind, the count, the client address, the user agent, and the request id. It SHALL also show the login or the words of `security-login-unknown`, and `detail` as formatted JSON. The page SHALL never show `login_hmac` (R35, A33, ADR-0051).

#### Scenario: ProcessEngineer asks for the security events
- **GIVEN** the ProcessEngineer `anowak` with an open session
- **WHEN** `anowak` sends `GET /security-events`
- **THEN** the server returns `403 E-WEB-002`

#### Scenario: Event of an unknown login
- **GIVEN** a security event `login_failed` of the typed login `ghost`, which matches no account
- **WHEN** the Administrator `admin` sends `GET /security-events?login=ghost`
- **THEN** the page shows the row with the words of `security-login-unknown`
- **AND** the page holds no text of the `login_hmac` of the row

### Requirement: Read-only mode for the lifecycle actions
In read-only mode, the approve, retire, discard, delete, and restore routes SHALL answer `503 E-WEB-006` and write nothing. The preview, the diff, the history, the audit, and the security event pages SHALL stay available (A24).

#### Scenario: Approval in read-only mode
- **GIVEN** the read-only mode
- **WHEN** the Administrator `admin` sends the approval of version 2 of `R-100`
- **THEN** the server returns `503 E-WEB-006`
- **AND** version 2 stays a draft

### Requirement: Texts of the approval and audit pages
Every error code that a route of this change returns SHALL have a message in the bundles `pl` and `en`. Each word of these pages SHALL come from the bundles. Examples are an action, an entity type, a kind of security event, and a state of the diff (R10, R36, R40).

#### Scenario: Action word in Polish
- **GIVEN** the ProcessEngineer `anowak` with the UI language `pl`
- **WHEN** `anowak` opens the history of `R-100`
- **THEN** each action word of the page comes from the bundle `pl`

### Requirement: Approval and audit pages without user data in an hx attribute
Each `hx-*` attribute of the approval preview, the diff, the history, the audit, and the security event pages SHALL hold a literal. No URL SHALL hold the CSRF token. Every form that holds `csrf_token` SHALL send `POST` (R38, ADR-0037).

#### Scenario: Approval preview
- **GIVEN** the approval preview of version 2 of `R-100`
- **WHEN** the test reads every `hx-*` attribute of the page
- **THEN** no attribute holds `R-100`, a version number, or a parameter tag
