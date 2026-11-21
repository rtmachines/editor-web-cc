## ADDED Requirements

### Requirement: Language switch on full pages
Every full page that answers a `GET` request SHALL hold a form that sends `POST /lang`. The form SHALL send the CSRF token of the session, or the field `csrf_pre` on the login page. The field `next` SHALL hold the path and the query of the request. A full page that answers another method SHALL hold no switch. The login page that answers `GET /login` SHALL hold the switch with `next` set to `/login`. The login page that answers a failed `POST /login` SHALL hold no switch (R10, A17).

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

#### Scenario: No switch after a failed login
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` with a wrong password and without the header `HX-Request`
- **THEN** the login page shows the error and the login form with the field `csrf_pre`
- **AND** the field `csrf_pre` holds the nonce of the cookie `__Host-recipe_csrf` of the same response
- **AND** the page holds no form that sends `POST /lang`

#### Scenario: No switch on the page of a one-time password
- **GIVEN** the Administrator `akowal` with an open session
- **WHEN** `akowal` sends `POST /users/anowak/reset-password` without the header `HX-Request`
- **THEN** the page shows the one-time password
- **AND** the page holds no form that sends `POST /lang`

## REMOVED Requirements

### Requirement: Language switch
**Reason**: The answer of a failed `POST /login` holds no switch. The validator refuses a renamed scenario inside a MODIFIED block.
**Migration**: The requirement "Language switch on full pages" replaces this requirement. It renames the scenario of a failed login to "No switch after a failed login".
