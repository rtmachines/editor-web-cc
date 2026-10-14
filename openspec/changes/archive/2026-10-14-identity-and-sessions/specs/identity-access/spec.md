## ADDED Requirements

### Requirement: Deactivation of an inactive user
The system SHALL reject `DeactivateUser` with `E-IDN-010` for a user that is already inactive. The command SHALL change no row and SHALL write no audit entry (A9, A14).

#### Scenario: Deactivate an inactive user
- **GIVEN** the inactive user `mhoffman`, deactivated on 2026-10-01
- **WHEN** an Administrator runs `DeactivateUser` on `mhoffman`
- **THEN** the system rejects the command with `E-IDN-010`
- **AND** the deactivation time of `mhoffman` stays 2026-10-01
- **AND** the audit log holds no new entry

### Requirement: Throttle of the own password change
`ChangeOwnPassword` SHALL apply the login delay and the per-address budget to the login of the user and the client address. A throttled attempt SHALL return `E-IDN-005` with the `Retry-After` value and SHALL write a security event of kind `login_throttled` (A22, A33).

#### Scenario: Password change inside the delay
- **GIVEN** 4 failed attempts for `mhoffman` from the address `10.0.0.5` in 15 minutes
- **AND** the last failure is 1 second ago
- **WHEN** the Operator `mhoffman` runs `ChangeOwnPassword` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`
- **AND** the system checks no password
- **AND** the system writes a security event of kind `login_throttled`

#### Scenario: New password outside the policy
- **GIVEN** the Operator `mhoffman` with no failed attempt
- **WHEN** the Operator `mhoffman` runs `ChangeOwnPassword` with a new password of 11 characters
- **THEN** the system rejects the command with `E-IDN-006`
- **AND** the system inserts no login attempt row

### Requirement: Retry-After of a throttled login
The system SHALL record a throttled attempt inside the login delay as a failure. The `Retry-After` value of the login delay SHALL be the full delay that this failure starts. The `Retry-After` value of the per-address budget SHALL be the time until the oldest failure of the address leaves the 15-minute window. Both values SHALL be whole seconds, rounded up, and at least 1 (A22).

#### Scenario: Login inside the delay
- **GIVEN** 4 failed attempts for `mhoffman` from the address `10.0.0.5` in 15 minutes
- **AND** the last failure is 1 second ago
- **WHEN** a user runs `Login` with the login `mhoffman` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`
- **AND** the `Retry-After` value is 4 seconds

#### Scenario: Login after the Retry-After time
- **GIVEN** a `Login` of `mhoffman` from the address `10.0.0.5` that got `E-IDN-005` with a `Retry-After` value of 4 seconds
- **WHEN** a user runs `Login` with the login `mhoffman` from the address `10.0.0.5` 4 seconds later
- **THEN** the system checks the password

#### Scenario: Budget of an address
- **GIVEN** 100 failed attempts from the address `10.0.0.5` in 15 minutes
- **AND** the oldest of these failures is 14 minutes and 30 seconds old
- **WHEN** a user runs `Login` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`
- **AND** the `Retry-After` value is 30 seconds

### Requirement: Identity from the user row on every request
On every request with a session, the system SHALL read `role`, `is_active`, `session_epoch`, and `must_change_password` from the user row. A session of an inactive user or with another epoch SHALL get `E-WEB-004` (R28, A21).

#### Scenario: Role changed outside a command
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** the role of `mhoffman` changes to `Administrator` without a change of `session_epoch`
- **AND** `mhoffman` runs `ListUsers`
- **THEN** the system checks the permission with the role `Administrator`
- **AND** the system returns the user list

#### Scenario: Epoch changed
- **GIVEN** the Operator `mhoffman` with an open session of epoch 0
- **WHEN** the `session_epoch` of `mhoffman` becomes 1
- **AND** `mhoffman` runs `GetOwnProfile`
- **THEN** the system rejects the query with `E-WEB-004`

### Requirement: Hashed session token
The system SHALL store only the SHA-256 of the session token as the session id. The database SHALL hold no session token (R21).

#### Scenario: Session row after a login
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and the right password
- **THEN** the session row holds the SHA-256 of the token in the cookie
- **AND** no column of the database holds the token

### Requirement: Pre-session CSRF cookie
`GET /login` SHALL set the cookie `__Host-recipe_csrf` with a 128-bit nonce and its HMAC-SHA256 under the first key of `csrf_signing_key`. The cookie SHALL have `HttpOnly`, `Secure`, `SameSite=Strict`, and `Path=/`. The pre-session check SHALL accept a signature only under a key of the key file, and the field `csrf_pre` SHALL equal the nonce (R38).

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

### Requirement: Credentials checked again at the commit
`Login` and `ChangeOwnPassword` SHALL check again under the lock of the user row that the user is active. They SHALL also check that `session_epoch` and the password hash did not change after the password check. A change SHALL roll back the command. `Login` SHALL then return `E-IDN-003`, and `ChangeOwnPassword` SHALL return `E-WEB-004` (R21, R28).

#### Scenario: Password reset during a login
- **GIVEN** the active user `mhoffman` and a `Login` of `mhoffman` with the right password after its password check
- **WHEN** an Administrator runs `ResetPassword` on `mhoffman` before that `Login` commits
- **THEN** the `Login` returns `E-IDN-003`
- **AND** the system creates no session for `mhoffman`

#### Scenario: Password reset during an own password change
- **GIVEN** the Operator `mhoffman` with an open session and a `ChangeOwnPassword` after its password check
- **WHEN** an Administrator runs `ResetPassword` on `mhoffman` before that change commits
- **THEN** the `ChangeOwnPassword` returns `E-WEB-004`
- **AND** the password hash of `mhoffman` is the hash of the one-time password

### Requirement: Concurrent demotion of administrators
When two Administrators demote or deactivate each other at the same time and no third active Administrator exists, the system SHALL complete exactly one command. The other command SHALL return `E-IDN-002` (A9, R31).

#### Scenario: Two administrators demote each other
- **GIVEN** the active Administrators `akowal` and `bnowak` and no other active Administrator
- **WHEN** `akowal` runs `SetUserRole` on `bnowak` with the role `Operator`
- **AND** `bnowak` runs `SetUserRole` on `akowal` with the role `Operator` at the same time
- **THEN** exactly one of the two commands completes
- **AND** the other command returns `E-IDN-002`
- **AND** one active Administrator remains

### Requirement: Login form fields
The system SHALL accept a `login` field of 1 to 64 characters with no control character and a `password` field of 1 to 128 characters. Another `login` or `password` field SHALL return `E-WEB-008` before the password check, and the system SHALL insert no login attempt row (R38).

#### Scenario: Login field of 65 characters
- **GIVEN** a browser with a valid pre-session token
- **WHEN** a user sends `POST /login` with a `login` field of 65 characters
- **THEN** the server rejects the request with `E-WEB-008`
- **AND** the system inserts no login attempt row

#### Scenario: Login field with a NUL character
- **GIVEN** a browser with a valid pre-session token
- **WHEN** a user sends `POST /login` with a `login` field that holds the character `U+0000`
- **THEN** the server rejects the request with `E-WEB-008`
- **AND** the system inserts no login attempt row

### Requirement: Strength of a one-time password
A one-time password of `CreateUser` and `ResetPassword` SHALL hold 20 characters from a random source. It SHALL meet the password policy (A23).

#### Scenario: One-time password of a new user
- **GIVEN** no user `mhoffman` exists
- **WHEN** an Administrator runs `CreateUser` with the login `mhoffman` and the role `Operator`
- **THEN** the shown one-time password has 20 characters
- **AND** the one-time password is not in the breached-password list

### Requirement: No secret in an audit entry or an error
No audit entry, error message, or error page SHALL hold a secret. The secrets are a password, a one-time password, a hash, a session token, a session id, a CSRF token, and a cookie value (R20, R36).

#### Scenario: Audit entry of a password reset
- **GIVEN** the active user `mhoffman`
- **WHEN** an Administrator runs `ResetPassword` on `mhoffman`
- **THEN** the entry `PasswordReset` holds no `password_hash`
- **AND** the entry holds no one-time password

### Requirement: User list
`ListUsers` SHALL return the users ordered by login in pages of 100 rows. The field `q` SHALL keep only the users whose login holds the text of `q` (A9).

#### Scenario: Filter by part of a login
- **GIVEN** the users `akowal`, `jkowalski`, and `mhoffman`
- **WHEN** an Administrator runs `ListUsers` with `q` set to `kowal`
- **THEN** the list holds `akowal` and `jkowalski` in this order
- **AND** the list does not hold `mhoffman`

### Requirement: Throttle state of a user
`GetUser` SHALL show the number of failed attempts of the login in the last 15 minutes over every client address. It SHALL also show the time of the last failure (A22).

#### Scenario: User with failed attempts
- **GIVEN** 4 failed attempts for `jkowalski` from two client addresses in 15 minutes
- **WHEN** an Administrator runs `GetUser` on `jkowalski`
- **THEN** the user page shows 4 failed attempts
- **AND** the user page shows the time of the last failure

## MODIFIED Requirements

### Requirement: Per-address login budget
The system SHALL reject every login from a client address with `E-IDN-005` after 100 failed attempts from that address in 15 minutes. After the 100th failed attempt, the system SHALL record no new attempt from that address. Parallel attempts with other logins from that address SHALL add at most 3 more attempt rows, one fewer than the Argon2 permits (A22).

#### Scenario: Budget reached
- **GIVEN** 100 failed attempts from the address `10.0.0.5` in 15 minutes
- **WHEN** a user runs `Login` with the login `mhoffman` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`

#### Scenario: Parallel attempts at the budget
- **GIVEN** 99 failed attempts from the address `10.0.0.5` in 15 minutes
- **WHEN** 10 users run `Login` at the same time with 10 other logins from the address `10.0.0.5`
- **THEN** the window holds at most 103 failed attempts from that address

### Requirement: Client address from trusted proxies
The server SHALL read `X-Forwarded-For` and `X-Forwarded-Proto` only from an address in the trusted-proxy list. For every other request, the server SHALL use the socket address as the client address. The server SHALL convert an IPv4-mapped IPv6 address to its IPv4 form before it compares or stores the address (R38, A22).

#### Scenario: Forwarded header from an untrusted address
- **GIVEN** a request from the address `10.0.0.9`, which is not in the trusted-proxy list
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.9` as the client address

#### Scenario: Proxy on an IPv6 socket
- **GIVEN** the trusted-proxy list `172.20.0.0/16` and a request from the socket address `::ffff:172.20.0.2`
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.5` as the client address

#### Scenario: Forwarded entry that is not an address
- **GIVEN** a request from a trusted proxy
- **WHEN** the request carries the header `X-Forwarded-For: not-an-address`
- **THEN** the server rejects the request with `E-WEB-008`

#### Scenario: Trusted proxy without a forwarded header
- **GIVEN** a request from a trusted proxy
- **WHEN** the request carries no header `X-Forwarded-For`
- **THEN** the server rejects the request with `E-WEB-010`
- **AND** the system inserts no login attempt row

### Requirement: Origin check for mutating requests
A mutating request is a request with any method except `GET`, `HEAD`, and `OPTIONS`. The system SHALL reject a mutating request that has no `Origin` header matching the site and no `Sec-Fetch-Site` value of `same-origin`. An `Origin` header matches the site when it equals `https://` followed by the `Host` header. A rejected request SHALL get `E-WEB-007` and SHALL write a security event of kind `csrf_rejected` (R38).

#### Scenario: Request from another site
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator runs `CreateRecipe` with an `Origin` header of another site
- **THEN** the system rejects the request

#### Scenario: Request without an origin header
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` without `Origin` and without `Sec-Fetch-Site`
- **THEN** the server rejects the request with `E-WEB-007`
- **AND** the system writes a security event of kind `csrf_rejected`
- **AND** `mhoffman` stays active

#### Scenario: Origin with another scheme
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.plant.local` and `Origin: http://recipes.plant.local`
- **THEN** the server rejects the request with `E-WEB-007`

#### Scenario: Origin of the same host
- **GIVEN** the Administrator `akowal` with an open session and a valid CSRF token
- **WHEN** `akowal` sends `POST /users/mhoffman/deactivate` with `Host: recipes.plant.local` and `Origin: https://recipes.plant.local`
- **THEN** the server accepts the request
