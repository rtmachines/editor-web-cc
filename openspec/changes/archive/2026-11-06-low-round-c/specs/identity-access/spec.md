## ADDED Requirements

### Requirement: Unlock with a lost unlock event
The write of the security event `unlock` can fail after the commit of `UnlockLogin`. Then the system SHALL keep the unlock and SHALL answer with success. The system SHALL write one `WARN` line with the code of the storage error and the `request_id` (R20, A22, A33).

#### Scenario: Security event table rejects the unlock event
- **GIVEN** the Administrator `kadmin` with an open session, and the login `mhoffman` with 3 failed attempts
- **AND** a write of a security event that fails
- **WHEN** `kadmin` sends `POST /users/mhoffman/unlock`
- **THEN** the server returns the success answer of the unlock
- **AND** the login `mhoffman` has no login attempt row
- **AND** the audit log holds one action `LoginUnlocked`
- **AND** the event log holds one `WARN` line with the `request_id` of the request

#### Scenario: Failed commit of the unlock
- **GIVEN** the Administrator `kadmin` with an open session, and the login `mhoffman` with 3 failed attempts
- **AND** a commit that fails
- **WHEN** `kadmin` sends `POST /users/mhoffman/unlock`
- **THEN** the server returns an error answer
- **AND** the login `mhoffman` keeps its 3 login attempt rows

### Requirement: Token of the language switch with a session
`POST /lang` SHALL accept the CSRF token of the session or a valid pre-session token `csrf_pre`, with or without a session. The Origin check SHALL apply to `POST /lang` as to every other mutating route (R38).

#### Scenario: Language switch with a session and the pre-session token
- **GIVEN** the Operator `mhoffman` with an open session and a valid cookie `__Host-recipe_csrf`
- **WHEN** `mhoffman` sends `POST /lang` with `lang` `en`, the nonce of that cookie in `csrf_pre`, and no session token
- **THEN** the server returns `303` and sets the cookie `recipe_lang` to `en`

#### Scenario: Language switch with a session and a wrong token
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** `mhoffman` sends `POST /lang` with a `csrf_token` that differs from the token of the session and no `csrf_pre`
- **THEN** the server rejects the request with `403 E-WEB-007`
