## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: HTMX error headers
For an HTMX request with a status of 400 or higher, the response SHALL set `HX-Retarget` to `#error-panel` and `HX-Reswap` to `innerHTML show:#error-panel:top`, except for `E-WEB-004` (R39).

#### Scenario: Concurrent edit response headers
- **GIVEN** a ProcessEngineer sends an HTMX save with a stale `row_version` of the draft version 3
- **WHEN** the server returns `409 E-WEB-001`
- **THEN** the response sets `HX-Retarget` to `#error-panel`
- **AND** the response sets `HX-Reswap` to `innerHTML show:#error-panel:top`

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
