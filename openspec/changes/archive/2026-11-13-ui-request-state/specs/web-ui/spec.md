## ADDED Requirements

### Requirement: Request state of a form
While an HTMX request of a form is in flight, the form SHALL have `aria-busy="true"`. Each text input and each `textarea` of the form SHALL be `readonly`. Each `select`, check box, and radio button of the form SHALL refuse a change. The focus SHALL stay on its control. After the last end of the request, the form SHALL lose `aria-busy` and each `readonly` that the lock set. The ends are an answer, a network error, an abort, a timeout, and a failed handling of the answer (R31, R39, A44).

#### Scenario: Typing during a save that answers 200
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and a save request in flight
- **WHEN** `anowak` types `22` into the value input
- **THEN** the value input keeps the sent text and the focus
- **AND** after the answer `200` the value input shows the saved value and is not `readonly`

#### Scenario: Typing during a save that answers 422
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, the value `abc`, and a save request in flight
- **WHEN** `anowak` types `33` into the value input
- **THEN** the value input keeps `abc`
- **AND** after the answer `422 E-RCP-005` the value input shows `abc` with its error line and is not `readonly`

#### Scenario: Typing during a save of a translation row
- **GIVEN** the ProcessEngineer `anowak` with the translation row `de` and a save request in flight
- **WHEN** `anowak` types a text into the name input of the row
- **THEN** the name input keeps the sent text
- **AND** after the answer `200` the name input is not `readonly`

#### Scenario: Typing during a translation save that answers 422
- **GIVEN** the ProcessEngineer `anowak` with the translation row `de`, a name that the server refuses, and a save request in flight
- **WHEN** `anowak` types a text into the description input of the row
- **THEN** the description input keeps the sent text
- **AND** after the answer `422` the row shows its error line and no input of the row is `readonly`

#### Scenario: Enter and click start the same lock
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** `anowak` sends the form once with Enter in the value input and once with a click on the save button
- **THEN** in both cases the form has `aria-busy="true"` while the request is in flight

#### Scenario: Unlock after a network error
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and the value `44`
- **WHEN** the save request fails with a network error
- **THEN** the form has no `aria-busy`, the value input is not `readonly`, and it holds `44`

#### Scenario: Unlock after an abort
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and a save request in flight
- **WHEN** the browser aborts the request
- **THEN** the form has no `aria-busy` and the value input is not `readonly`
- **AND** the value input keeps its text and the focus

#### Scenario: Unlock after an answer with a missing target
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** the save answer carries `HX-Retarget` with an id that the page does not hold
- **THEN** the form has no `aria-busy` and the value input is not `readonly`

#### Scenario: Readonly of the page stays
- **GIVEN** the edit form of `LIMIT` with one input that has `readonly` before the send
- **WHEN** a save request of the form ends with a network error
- **THEN** that input is still `readonly`
- **AND** each other text input of the form is not `readonly`

### Requirement: Lock rules of a busy form
A form with several requests in flight SHALL keep its lock until the last request ends. The lock SHALL let through Tab, each key with Ctrl, Alt, or Meta, and the keys F1 to F12. So the shortcuts of the browser work (R31, R39, A44).

#### Scenario: Two requests of one form
- **GIVEN** a form with two requests in flight, for example a boosted filter form and a boosted link inside it
- **WHEN** the first request ends
- **THEN** the form still has `aria-busy="true"` and its text inputs are still `readonly`
- **AND** after the second request ends the form has no `aria-busy` and no input that the lock set is `readonly`

#### Scenario: Shortcuts of the browser during a request
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, a `select` in it, and a save request in flight
- **WHEN** `anowak` presses Ctrl+R or F5 on the `select`
- **THEN** the script does not call `preventDefault`, so the browser keeps its shortcut
- **AND** a press of the Arrow Down key on the `select` stays refused

### Requirement: Message of a lost answer
When an HTMX request ends with a network error or with a timeout, the page SHALL put the message `E-WEB-015` into `#error-panel` and SHALL empty `#toast`. The page SHALL also show the top of the panel. The message SHALL have `role="alert"`, the code, and one sentence in the UI language. It SHALL say that the result is not known and SHALL have no reference. The next HTMX request SHALL delete the message (R31, R39, A44).

#### Scenario: Network error before the send
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`, the UI language `en`, and the value `44`
- **WHEN** the save request fails with a network error before the server gets it
- **THEN** `#error-panel` holds a fragment with `role="alert"` and the code `E-WEB-015`
- **AND** the value input holds `44`

#### Scenario: Answer lost after the server saved
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT` and the value `55`
- **WHEN** the server saves `55` and the browser gets a network error in place of the answer
- **THEN** `#error-panel` holds the message `E-WEB-015` that the result is not known
- **AND** a new load of the page shows `55`

#### Scenario: Timeout of a request
- **GIVEN** the ProcessEngineer `anowak` with the edit form of `LIMIT`
- **WHEN** the save request gets no answer within `htmx.request_timeout`
- **THEN** `#error-panel` holds the message `E-WEB-015`
- **AND** the form has no `aria-busy`

#### Scenario: Lost answer after a success
- **GIVEN** the ProcessEngineer `anowak` on the edit page of `LIMIT` with the success message of a save in `#toast`
- **WHEN** the next save request fails with a network error
- **THEN** `#toast` is empty
- **AND** `#error-panel` holds the message `E-WEB-015`

#### Scenario: Next request deletes the message
- **GIVEN** a page that shows the message `E-WEB-015` after a network error before the send
- **WHEN** the user sends the same form again and the server answers `200`
- **THEN** `#error-panel` holds no message `E-WEB-015`

#### Scenario: Message in Polish
- **GIVEN** the UI language `pl`
- **WHEN** the server renders a full page
- **THEN** the template `#transport-error` holds the code `E-WEB-015` and the sentence of `error-E-WEB-015` from the bundle `pl`

## MODIFIED Requirements

### Requirement: Error fragment content
Every error fragment SHALL show the error code `E-<CTX>-<nnn>`, one sentence in the UI language that tells the user what to do, and the `request_id`. The fragment of `E-WEB-015` is the one exception among the fragments: it has no `request_id`, because no answer arrived. Each code of the routes of `api.md` sections 2.2 and 2.3 SHALL have its sentence in the bundles `pl` and `en`. The code `E-WEB-015` SHALL also have its sentence in both bundles (R10, R36, R39).

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

#### Scenario: Fragment of a lost answer
- **GIVEN** the UI language `en`
- **WHEN** the server renders the template `#transport-error` of a full page
- **THEN** the fragment shows `E-WEB-015` and the sentence of `error-E-WEB-015` from the bundle `en`
- **AND** the fragment has no element `.alert-ref`

### Requirement: Static files
`GET /static/*` SHALL serve the vendored HTMX 2 file, the style sheets `app.css` and `components.css`. It SHALL also serve the scripts `row-version.js`, `tree.js`, and `request-state.js`, and the 8 font files of `design-system.md` section 5. Each file SHALL be unchanged and SHALL have its media type. The route SHALL need no session. The response SHALL set `Cache-Control: public, max-age` with the value of `static.cache_max_age` (R39).

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

#### Scenario: Request state script without a session
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /static/request-state.js`
- **THEN** the server returns `200` with a JavaScript media type and `X-Content-Type-Options: nosniff`

### Requirement: HTMX configuration of every page
Every full page SHALL hold the `htmx-config` meta tag with the values of `api.md` section 9. The values SHALL include `timeout` with the value of `htmx.request_timeout` in milliseconds. Its `<body>` SHALL have `hx-history="false"`. With a session, the `<body>` of every full page except the full error page SHALL also have `hx-headers` with the header `X-CSRF-Token`. Every full page SHALL hold the template `#transport-error` outside `main` (R38, R39).

#### Scenario: Profile page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator sends `GET /me`
- **THEN** the page holds the `htmx-config` meta tag with `"allowEval":false`, `"selfRequestsOnly":true`, and `"timeout":45000`
- **AND** the `<body>` holds `hx-history="false"` and the CSRF token of the session in `hx-headers`

#### Scenario: Full error page with a session
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users/nobody` without the header `HX-Request`
- **THEN** the page holds the `htmx-config` meta tag and `hx-history="false"`
- **AND** the `<body>` holds no `hx-headers`

#### Scenario: Template of the lost answer
- **GIVEN** a browser without a session
- **WHEN** a user sends `GET /login`
- **THEN** the page holds `<template id="transport-error">` after `main`

### Requirement: No inline script or style
A page SHALL load only the scripts `htmx.min.js`, `row-version.js`, `tree.js`, and `request-state.js` from `/static/`. `row-version.js` SHALL have at most 20 lines, `tree.js` SHALL have at most 50 lines, and `request-state.js` SHALL have at most 40 lines. A page SHALL hold no inline script, no `<style>` element, no `style` attribute, and no inline event handler (R38).

#### Scenario: User list page
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `GET /users`
- **THEN** the page holds no `<script>` element without a `src` attribute
- **AND** the page holds no `style` attribute and no attribute that starts with `on`

#### Scenario: Scripts of a page
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `GET /recipes`
- **THEN** the page loads `/static/htmx.min.js`, `/static/row-version.js`, `/static/tree.js`, and `/static/request-state.js` and no other script
- **AND** the page holds no `<style>` element

#### Scenario: Size of the scripts
- **GIVEN** the files `static/row-version.js`, `static/tree.js`, and `static/request-state.js`
- **WHEN** the script test counts their lines
- **THEN** `row-version.js` has at most 20 lines, `tree.js` has at most 50 lines, and `request-state.js` has at most 40 lines

### Requirement: Second request of a form
Every approve form, every save form, and every retire, discard, delete, and restore form SHALL have the attribute `hx-sync="closest form:drop"`. The browser SHALL drop a second request of such a form while the first request is in flight. The approve, retire, discard, delete, and restore forms SHALL also have an indicator with the text of `form-in-progress`. Three save forms of a draft SHALL have that indicator too. They are the edit form and the new form of a parameter, and the save form of a translation row. No form SHALL have the attribute `hx-disabled-elt`, so a refused request keeps the focus on the button (R39, I12, ADR-0049, ADR-0158).

#### Scenario: Second request while a request is in flight
- **GIVEN** a save form of the draft version 3 with a request in flight
- **WHEN** a ProcessEngineer sends the same save form again
- **THEN** the browser drops the second request

#### Scenario: Retire form
- **GIVEN** the page of the approved version 2 of `R-100` for the Administrator `admin`
- **WHEN** the test reads the retire form
- **THEN** the form has `hx-sync="closest form:drop"` and no `hx-disabled-elt`

#### Scenario: Indicator of a save form
- **GIVEN** the ProcessEngineer `anowak` with an open session and the draft version 3 of `R-100` with the parameter `LIMIT`
- **WHEN** `anowak` sends `GET` for the edit page of `LIMIT`
- **THEN** the edit form and the save form of each translation row hold an element with the class `htmx-indicator` and the text of `form-in-progress`
- **AND** no form of the page has `hx-disabled-elt`
