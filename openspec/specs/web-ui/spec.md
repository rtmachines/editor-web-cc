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
For an HTMX request with a status of 400 or higher, the response SHALL set `HX-Retarget` to `#error-panel` and `HX-Reswap` to `innerHTML show:#error-panel:top`, except for `E-WEB-004` (R39).

#### Scenario: Concurrent edit response headers
- **GIVEN** a ProcessEngineer sends an HTMX save with a stale `row_version` of the draft version 3
- **WHEN** the server returns `409 E-WEB-001`
- **THEN** the response sets `HX-Retarget` to `#error-panel`
- **AND** the response sets `HX-Reswap` to `innerHTML show:#error-panel:top`

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
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id` (R36).

#### Scenario: Validation error fragment
- **GIVEN** a ProcessEngineer with the UI language `en`
- **WHEN** the ProcessEngineer sends a request that gets a validation error
- **THEN** the error fragment shows the code `E-WEB-008`
- **AND** the fragment shows one English sentence that tells the user what to do
- **AND** the fragment shows the `request_id`
- **AND** the structured log of the request holds the same code

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
`GET /static/*` SHALL serve the vendored HTMX 2 file, the style sheet `app.css`, the scripts `row-version.js` and `tree.js`, and the 8 font files of `design-system.md` section 5. Each file SHALL be unchanged and SHALL have its media type. The route SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

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
Every full page SHALL show, in this order, a skip link to the main content, a top bar, and `main` with the id `content`. `main` SHALL start with `#error-panel` and the status region `#toast`, then one `h1`. The top bar SHALL show the application name. Every full page SHALL link the style sheet `/static/app.css` (R39, A44).

#### Scenario: Layout of the profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /me`
- **THEN** the page holds a skip link to `#content` before the top bar
- **AND** `main` with the id `content` holds `#error-panel`, `#toast`, and one `h1` in this order
- **AND** the page links the style sheet `/static/app.css`

#### Scenario: Layout of a full error page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request` and gets `404 E-WEB-005`
- **THEN** the page shows the top bar with the application name and no navigation link
- **AND** the page shows the error in `#error-panel`, one `h1`, and a link to `/recipes`
- **AND** the title of the page holds the code `E-WEB-005` and the application name

### Requirement: Navigation by permission
With a session, the top bar SHALL show the link to a page only when the role holds the permission of that page. The use case of that page SHALL still check the permission. The current link SHALL have `aria-current="page"` and SHALL show a bottom border and the weight 700. A session with `must_change_password` SHALL show no navigation link (R28, R39).

#### Scenario: Navigation of an Operator
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the top bar shows the links to `/recipes` and `/me`
- **AND** the top bar shows no link to `/users`
- **AND** the link to `/recipes` has `aria-current="page"`

#### Scenario: Navigation of an Administrator
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /me`
- **THEN** the top bar shows the links to `/recipes`, `/users`, and `/me`

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
A version state and a user state SHALL show a color, an icon, and a word. Each error, warning, information, and success message SHALL also show a color, an icon, and a word. No element SHALL show a state by its color alone (R39, A44).

#### Scenario: State of a draft version
- **GIVEN** the draft version 3 of the recipe `R-100` and the UI language `pl`
- **WHEN** a page shows the state of version 3
- **THEN** the page shows the icon `status-draft` and the word `Szkic` next to it

#### Scenario: State of an inactive user
- **GIVEN** the Administrator `akowal` with an open session and the inactive user `anowak`
- **WHEN** `akowal` sends `GET /users` with the UI language `en`
- **THEN** the row of `anowak` shows the icon `user-inactive` and the word `Inactive`

#### Scenario: Error fragment as one alert
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends an HTMX request for `GET /users` and gets `403 E-WEB-002`
- **THEN** the fragment is one element with `role="alert"`
- **AND** that element holds the icon `alert-error`, the code `E-WEB-002`, one sentence, and the reference

### Requirement: Field error lines
Each line of `field-errors.html` SHALL show the label of the field from the bundles, not the name of the form field. In an HTMX fragment and on the login page, the line SHALL link to the input of the field. The full error page SHALL show the label without a link (R36, A44).

#### Scenario: Too long current password
- **GIVEN** the Operator `mhoffman` with an open session and the UI language `en`
- **WHEN** `mhoffman` sends an HTMX `POST /me/password` with a current password of 129 characters
- **THEN** the server returns `422 E-WEB-008` with the fragment `field-errors.html`
- **AND** the line shows the label `Current password` with a link to `#current_password`

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
The style sheet SHALL define each token of the token tables of `design-system.md` with the value of the table. A rule outside `:root` SHALL name no color value except `transparent`, `currentColor`, and the system colors of the forced-colors mode (R39, A44).

#### Scenario: Token of the action color
- **GIVEN** the row `--color-action` with the value `#0B4C86` in `design-system.md` section 3
- **WHEN** the token test reads `:root` of `/static/app.css`
- **THEN** `:root` sets `--color-action` to `#0B4C86`

#### Scenario: Color value outside the tokens
- **GIVEN** a rule of `/static/app.css` outside `:root`
- **WHEN** the token test reads the rule
- **THEN** the rule holds no hex color, no color function, and no named color outside the permitted list

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
