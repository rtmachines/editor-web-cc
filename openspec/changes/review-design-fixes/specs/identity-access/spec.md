## ADDED Requirements

### Requirement: Security event after an early end of the request
The rule covers `Login`, `ChangeOwnPassword`, `UnlockLogin`, and the Basic check of an export route. After the first transaction of one of them commits, the system SHALL write its security event also when the request ends early. After an early end, the system SHALL NOT create the session of `Login` or write the new password of `ChangeOwnPassword`. A failed event write SHALL give one `WARN` line. A closed connection and `http.request_timeout` end a request early (R20, R35, A33).

#### Scenario: Client closes the connection during the password check
- **GIVEN** the active user `mhoffman`
- **WHEN** a client sends `POST /login` with the login `mhoffman` and a wrong password
- **AND** the client closes the connection before the password check ends
- **THEN** the system writes one login attempt row for `mhoffman`
- **AND** the system writes a security event of kind `login_failed`

#### Scenario: Client closes the connection before the session exists
- **GIVEN** the active user `mhoffman`
- **WHEN** a client sends `POST /login` with the login `mhoffman` and the right password
- **AND** the client closes the connection before the password check ends
- **THEN** the system creates no session for `mhoffman`

#### Scenario: Request limit after the commit of an unlock
- **GIVEN** the Administrator `kadmin` with an open session, and the login `mhoffman` with 3 failed attempts
- **WHEN** `kadmin` sends `POST /users/mhoffman/unlock`
- **AND** the limit `http.request_timeout` ends the request after the commit of `LoginUnlocked`
- **THEN** the system writes a security event of kind `unlock`

#### Scenario: Basic check with a closed connection
- **GIVEN** the active `ExportClient` account `xfer01`
- **WHEN** a transfer client sends `GET /export/v1/machines/M1/recipes/R-100/approved` with Basic credentials of `xfer01` and a wrong password
- **AND** the client closes the connection before the password check ends
- **THEN** the system writes a security event of kind `login_failed`

## MODIFIED Requirements

### Requirement: Session cookie
The session cookie SHALL hold only a random 128-bit token and SHALL have HttpOnly, SameSite=Strict, Path=/, and no Domain. With `session.secure` true, the cookie SHALL be `__Host-recipe_session` with Secure. With `session.secure` false in a build with the `dev` feature, it SHALL be `recipe_session` without Secure. A build without the `dev` feature SHALL refuse to start when `session.secure` is false (R21, R38).

#### Scenario: Cookie after login
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and the right password
- **THEN** the response sets the cookie `__Host-recipe_session`
- **AND** the cookie has the attributes HttpOnly, Secure, SameSite=Strict, and Path=/

#### Scenario: Insecure cookie setting
- **GIVEN** a build without the `dev` feature and the setting `session.secure` set to false
- **WHEN** the server starts
- **THEN** the server refuses to start

#### Scenario: Cookie after login in a dev build without Secure
- **GIVEN** a build with the `dev` feature, the setting `session.secure` set to false, and the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and the right password
- **THEN** the response sets the cookie `recipe_session` with HttpOnly, SameSite=Strict, and Path=/
- **AND** the cookie has no attribute Secure
- **AND** the response sets no cookie `__Host-recipe_session`

### Requirement: Pre-session CSRF cookie
`GET /login` SHALL set the pre-session cookie with a 128-bit nonce and its HMAC-SHA256 under the first key of `csrf_signing_key`. The cookie SHALL be `__Host-recipe_csrf` with `HttpOnly`, `Secure`, `SameSite=Strict`, and `Path=/`. When `session.secure` is false in a build with the `dev` feature, it SHALL be `recipe_csrf` without `Secure`. The check SHALL accept a signature only under a key of the key file, and `csrf_pre` SHALL equal the nonce (R38).

#### Scenario: Login form with an open session
- **GIVEN** the Operator `mhoffman` with an open session and a valid cookie `__Host-recipe_csrf`
- **WHEN** a user sends `POST /login` with the nonce of that cookie in `csrf_pre` and no session token
- **THEN** the server accepts the pre-session token
- **AND** the system writes no security event of kind `csrf_rejected`

#### Scenario: Forged pre-session cookie
- **GIVEN** a browser with a cookie `__Host-recipe_csrf` whose signature no key of the key file made
- **WHEN** a user sends `POST /login` with the nonce of that cookie in `csrf_pre`
- **THEN** the server rejects the request with `E-WEB-007`
- **AND** the system writes a security event of kind `csrf_rejected`

#### Scenario: Pre-session field of another cookie
- **GIVEN** a browser with a valid cookie `__Host-recipe_csrf`
- **WHEN** a user sends `POST /login` with a field `csrf_pre` that differs from the nonce of the cookie
- **THEN** the server rejects the request with `E-WEB-007`

#### Scenario: Pre-session cookie in a dev build without Secure
- **GIVEN** a build with the `dev` feature and `session.secure` set to false
- **WHEN** a user sends `GET /login`
- **THEN** the response sets the cookie `recipe_csrf` with `HttpOnly`, `SameSite=Strict`, and `Path=/`
- **AND** the cookie has no attribute `Secure`
