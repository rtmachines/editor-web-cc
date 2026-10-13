# identity-access Specification

## Purpose
The identity-access capability describes the users, roles, sessions, login throttle, password rules, and permission matrix of the recipe editor. It states how the system checks who runs a command and what each role may do.

## Requirements

### Requirement: User login and password hash storage
The system SHALL store each user login and its Argon2id password hash in the same database. A login SHALL match `[a-z0-9._-]{3,64}` and SHALL be unique without regard to case. The system SHALL hash a password with Argon2id, `m = 19 MiB`, `t = 2`, and `p = 1` (R21, R38).

#### Scenario: Duplicate login
- **GIVEN** the user `jkowalski` exists
- **WHEN** an Administrator runs `CreateUser` with the login `jkowalski`
- **THEN** the system rejects the command with `E-IDN-001`
- **AND** the system creates no user

### Requirement: One-time password on user creation
The system SHALL create a user without a password in the request. The server SHALL generate a one-time password, show it once, and set `must_change_password` to true (R21).

#### Scenario: Create a user
- **GIVEN** no user `mhoffman` exists
- **WHEN** an Administrator runs `CreateUser` with the login `mhoffman` and the role `Operator`
- **THEN** the system creates the user `mhoffman` with the role `Operator`
- **AND** the system sets `must_change_password` to true
- **AND** the audit log holds the action `UserCreated`

### Requirement: One-time password on password reset
`ResetPassword` SHALL generate a one-time password on the server and show it once. It SHALL set `must_change_password` to true (R21, A9).

#### Scenario: Reset the password of a user
- **GIVEN** the active user `mhoffman`
- **WHEN** an Administrator runs `ResetPassword` on `mhoffman`
- **THEN** the system shows a new one-time password once
- **AND** the system sets `must_change_password` of `mhoffman` to true
- **AND** the audit log holds the action `PasswordReset`

### Requirement: Three roles
The system SHALL assign each user exactly one role: Administrator, ProcessEngineer, or Operator (R22).

#### Scenario: Role of a new user
- **GIVEN** no user `lnowak` exists
- **WHEN** an Administrator runs `CreateUser` with the login `lnowak` and the role `ProcessEngineer`
- **THEN** the system creates the user `lnowak` with the role `ProcessEngineer`

### Requirement: Permission check first
The system SHALL check the permission of each command and each query as its first statement. The HTTP layer SHALL only authenticate the user and SHALL NOT decide what a role may do (R28).

#### Scenario: Operator runs a command that changes data
- **GIVEN** the user `mhoffman` with the role `Operator`
- **WHEN** an Operator runs `CreateRecipe` for the machine `M12`
- **THEN** the system rejects the command with `E-WEB-002`
- **AND** the system creates no recipe

### Requirement: Forbidden result event
The system SHALL return `E-WEB-002` for a `Forbidden` result. The system SHALL write one security event of kind `forbidden` for that result (R28, A33).

#### Scenario: Forbidden security event
- **GIVEN** the user `mhoffman` with the role `Operator`
- **WHEN** an Operator runs `RenameMachine` on `M12`
- **THEN** the system rejects the command with `E-WEB-002`
- **AND** the system writes a security event of kind `forbidden` for the user `mhoffman`

### Requirement: Operator permissions
The system SHALL give an Operator only the permissions `RecipeView` and `VersionExport` (R24, A4).

#### Scenario: Operator exports an approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` for the approved version of `R-100`
- **THEN** the system returns the export document of version 3

#### Scenario: Operator exports a retired version
- **GIVEN** the retired version 2 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` for version 2 of `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

#### Scenario: Operator renames a recipe
- **GIVEN** the recipe `R-100`
- **WHEN** an Operator runs `RenameRecipe` on `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

### Requirement: ProcessEngineer permissions
The system SHALL give a ProcessEngineer the permissions listed for that role in `api.md` section 3.3 and no other permission (R25, A9).

#### Scenario: ProcessEngineer creates a recipe
- **GIVEN** the live machine `M12`
- **WHEN** a ProcessEngineer runs `CreateRecipe` for the machine `M12` with the reference number `R-100` and the name `Mixer`
- **THEN** the system creates the recipe `R-100`

#### Scenario: ProcessEngineer retires a version
- **GIVEN** the recipe `R-100` with the approved version 2
- **WHEN** a ProcessEngineer runs `RetireVersion` on version 2 of `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

#### Scenario: ProcessEngineer creates a user
- **GIVEN** the user `jkowalski` with the role `ProcessEngineer`
- **WHEN** a ProcessEngineer runs `CreateUser` with the login `pzielinski`
- **THEN** the system rejects the command with `E-WEB-002`

### Requirement: Administrator permissions
The system SHALL give an Administrator every permission of the `Permission` enum of `api.md` section 3.1 (R26).

#### Scenario: Administrator restores a recipe
- **GIVEN** the deleted recipe `R-100` of the live machine `M12`
- **WHEN** an Administrator runs `RestoreRecipe` on `R-100`
- **THEN** the system clears `deleted_at` of `R-100`
- **AND** the audit log holds the action `RecipeRestored`

### Requirement: Recipe deletion permission
The system SHALL allow an Administrator and a ProcessEngineer to run `DeleteRecipe`. The system SHALL NOT allow an Operator to run `DeleteRecipe` (R23, A13).

#### Scenario: Operator deletes a recipe
- **GIVEN** the recipe `R-100`
- **WHEN** an Operator runs `DeleteRecipe` on `R-100`
- **THEN** the system rejects the command with `E-WEB-002`

### Requirement: Approval permission
The system SHALL allow an Administrator and a ProcessEngineer to run `ApproveVersion` (A5).

#### Scenario: ProcessEngineer approves a draft of another user
- **GIVEN** the draft of `R-100` created by `mhoffman` and not edited by `jkowalski`
- **WHEN** a ProcessEngineer runs `ApproveVersion` on the draft as `jkowalski`
- **THEN** the system sets the status of the draft to `Approved`

#### Scenario: Operator approves a draft
- **GIVEN** the draft of `R-100`
- **WHEN** an Operator runs `ApproveVersion` on the draft
- **THEN** the system rejects the command with `E-WEB-002`

### Requirement: Form and preview query permission
A form or preview query SHALL check the permission of the command that it prepares (R28).

#### Scenario: Operator opens the recipe form
- **GIVEN** the user `mhoffman` with the role `Operator`
- **WHEN** an Operator runs `GetRecipeForm`
- **THEN** the system rejects the query with `E-WEB-002`

### Requirement: Audit log visibility for Administrator
The system SHALL show the full audit log and the security events only to an Administrator (A20).

#### Scenario: Administrator reads security events
- **GIVEN** the security events of the system
- **WHEN** an Administrator runs `ListSecurityEvents`
- **THEN** the system returns the security events

#### Scenario: ProcessEngineer reads security events
- **GIVEN** the security events of the system
- **WHEN** a ProcessEngineer runs `ListSecurityEvents`
- **THEN** the system rejects the query with `E-WEB-002`

### Requirement: Audit log visibility for ProcessEngineer
The system SHALL show a ProcessEngineer only the audit entries with `aggregate_type` set to `recipe` (A20).

#### Scenario: ProcessEngineer reads audit entries
- **GIVEN** the audit entries of the recipe `R-100` and of the machine `M12`
- **WHEN** a ProcessEngineer runs `ListAuditEntries`
- **THEN** the system returns the audit entries of `R-100`
- **AND** the system returns no audit entry of `M12`

### Requirement: Recipe history for every role
Every role SHALL view the recipe history of a recipe through `GetRecipeHistory` (A20).

#### Scenario: Operator reads recipe history
- **GIVEN** the recipe `R-100` with the version 1 and the version 2
- **WHEN** an Operator runs `GetRecipeHistory` on `R-100`
- **THEN** the system returns the history of `R-100`

### Requirement: Last administrator
The system SHALL reject `SetUserRole` and `DeactivateUser` when the command would leave no active Administrator (A9).

#### Scenario: Deactivate the last Administrator
- **GIVEN** the user `akowal` is the only active Administrator
- **WHEN** the Administrator `akowal` runs `DeactivateUser` on `akowal`
- **THEN** the system rejects the command with `E-IDN-002`
- **AND** `akowal` stays active

### Requirement: Serializable isolation for role and activation changes
The system SHALL run `SetUserRole` and `DeactivateUser` in SERIALIZABLE isolation. The system SHALL retry once on SQLSTATE `40001` and SHALL then return `E-DB-001` (R31).

#### Scenario: Retry succeeds
- **GIVEN** the first attempt of `DeactivateUser` on `mhoffman` gets SQLSTATE `40001`
- **WHEN** an Administrator runs `DeactivateUser` on `mhoffman`
- **THEN** the system retries the command once
- **AND** the system sets `is_active` of `mhoffman` to false

#### Scenario: Retry fails
- **GIVEN** every attempt of `DeactivateUser` on `mhoffman` gets SQLSTATE `40001`
- **WHEN** an Administrator runs `DeactivateUser` on `mhoffman`
- **THEN** the system rejects the command with `E-DB-001`

### Requirement: Session end on role change, deactivation, and reset
The system SHALL increment `session_epoch` and delete every session of the user when `SetUserRole`, `DeactivateUser`, or `ResetPassword` runs. The next request of that user SHALL get `E-WEB-004` (R28, A14).

#### Scenario: Deactivated user sends a request
- **GIVEN** the user `mhoffman` with an open session
- **WHEN** an Administrator runs `DeactivateUser` on `mhoffman`
- **THEN** the system deletes every session of `mhoffman`
- **AND** the next query of `mhoffman` gets `E-WEB-004`

#### Scenario: Set the role of a user
- **GIVEN** the user `mhoffman` with the role `Operator` and an open session
- **WHEN** an Administrator runs `SetUserRole` on `mhoffman` with the role `ProcessEngineer`
- **THEN** the system sets the role of `mhoffman` to `ProcessEngineer`
- **AND** the system deletes every session of `mhoffman`
- **AND** the audit log holds the action `UserRoleSet`

### Requirement: Deactivation keeps the user row
The system SHALL deactivate a user by setting `is_active` to false. The system SHALL keep the user row (A14).

#### Scenario: Deactivate an active user
- **GIVEN** the active user `mhoffman`
- **WHEN** an Administrator runs `DeactivateUser` on `mhoffman`
- **THEN** the system sets `is_active` of `mhoffman` to false
- **AND** the user row of `mhoffman` stays in the database

### Requirement: Activation of a user
`ActivateUser` SHALL set `is_active` to true and SHALL clear the deactivation time. The system SHALL reject `ActivateUser` with `E-IDN-009` for a user that is already active (A9).

#### Scenario: Activate an inactive user
- **GIVEN** the inactive user `mhoffman`
- **WHEN** an Administrator runs `ActivateUser` on `mhoffman`
- **THEN** the system sets `is_active` of `mhoffman` to true
- **AND** the audit log holds the action `UserActivated`

#### Scenario: Activate an active user
- **GIVEN** the active user `mhoffman`
- **WHEN** an Administrator runs `ActivateUser` on `mhoffman`
- **THEN** the system rejects the command with `E-IDN-009`

### Requirement: One response for failed authentication
The system SHALL give the same status, body, and response time for an unknown login, an inactive user, and a wrong password. The status SHALL be 401 and the code SHALL be `E-IDN-003` (R36, A42).

#### Scenario: Wrong password
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and a wrong password
- **THEN** the system rejects the command with `E-IDN-003`
- **AND** the response has status 401

#### Scenario: Inactive user
- **GIVEN** the inactive user `mhoffman`
- **WHEN** a user runs `Login` with the login `mhoffman` and the right password
- **THEN** the system rejects the command with `E-IDN-003`
- **AND** the response has status 401

### Requirement: Security event of a failed login
The system SHALL write a security event for each failed login. The kind SHALL be `login_failed` for an unknown login or a wrong password, and `login_rejected_inactive` for an inactive user (A33).

#### Scenario: Inactive user logs in
- **GIVEN** the inactive user `mhoffman`
- **WHEN** a user runs `Login` with the login `mhoffman` and the right password
- **THEN** the system writes a security event of kind `login_rejected_inactive`
- **AND** the system writes no audit entry

### Requirement: Login delay after failures
The system SHALL reject a login attempt with `E-IDN-005` and the `Retry-After` value when it comes within `2^(n-3)` seconds after the last failure. Here `n` is the failure count of the login and client address in 15 minutes, from 3 on. The delay SHALL be capped at 60 seconds. The system SHALL record each attempt before it checks the password, so every failed attempt counts. A throttled login SHALL write a security event of kind `login_throttled` and no audit entry (A22, A33).

#### Scenario: Attempt inside the delay
- **GIVEN** 4 failed attempts for `jkowalski` from the address `10.0.0.5` in 15 minutes
- **AND** the last failure is 1 second ago
- **WHEN** a user runs `Login` with the login `jkowalski` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`
- **AND** the result carries the `Retry-After` value
- **AND** the system writes a security event of kind `login_throttled`

### Requirement: Per-address login budget
The system SHALL reject every login from a client address with `E-IDN-005` after 100 failed attempts from that address in 15 minutes. After the 100th failed attempt, the system SHALL record no new attempt from that address (A22).

#### Scenario: Budget reached
- **GIVEN** 100 failed attempts from the address `10.0.0.5` in 15 minutes
- **WHEN** a user runs `Login` with the login `mhoffman` from the address `10.0.0.5`
- **THEN** the system rejects the command with `E-IDN-005`

### Requirement: No permanent lockout
The system SHALL NOT lock a login permanently. When an Administrator runs `UnlockLogin`, the system SHALL clear the throttle counters of that login (A22).

#### Scenario: Administrator unlocks a login
- **GIVEN** 5 failed attempts for `jkowalski` in 15 minutes
- **WHEN** an Administrator runs `UnlockLogin` on `jkowalski`
- **THEN** the system deletes the login attempts of `jkowalski`
- **AND** the system writes a security event of kind `unlock`
- **AND** the audit log holds the action `LoginUnlocked`

### Requirement: Successful login creates a session
The system SHALL create a new session with a new token and a new session id at each successful login. The system SHALL delete the session that the login request carries (R21, A33).

#### Scenario: Login with an existing session
- **GIVEN** the user `jkowalski` with the role `Operator` and the open session `S1`
- **WHEN** a user runs `Login` with the login `jkowalski`, the right password, and the session `S1`
- **THEN** the system creates a new session with a new session token
- **AND** the system deletes the session `S1`
- **AND** the audit log holds the action `UserLoggedIn`

### Requirement: Logout ends the session
The system SHALL delete the session of the request when a user runs `Logout` (A33).

#### Scenario: Operator logs out
- **GIVEN** the Operator `mhoffman` with the open session `S2`
- **WHEN** an Operator runs `Logout`
- **THEN** the system deletes the session `S2`
- **AND** the audit log holds the action `UserLoggedOut`

### Requirement: Login in read-only mode
The system SHALL run `Login` in read-only mode. The system SHALL still write the login attempt row and the security event of a failed or throttled login (A24).

#### Scenario: Failed login in read-only mode
- **GIVEN** the system is in read-only mode
- **WHEN** a user runs `Login` with the login `jkowalski` and a wrong password
- **THEN** the system rejects the command with `E-IDN-003`
- **AND** the system writes a login attempt row with `succeeded` set to false
- **AND** the system writes a security event of kind `login_failed`

### Requirement: Argon2 permit timeout
The system SHALL run each Argon2id hash under one of 4 permits. The system SHALL stop waiting for a permit after 2 seconds, and a timeout SHALL return `E-WEB-009` (R30).

#### Scenario: No permit within 2 seconds
- **GIVEN** all 4 Argon2id permits are in use
- **WHEN** a user runs `Login` with the login `jkowalski`
- **THEN** the system rejects the command with `E-WEB-009`

### Requirement: Session timeouts
The system SHALL end a session after 30 minutes without a request. The system SHALL end every session 12 hours after its login. An ended session SHALL get `E-WEB-004` (A21).

#### Scenario: Idle session
- **GIVEN** the session of the Operator `mhoffman` with the last request 31 minutes ago
- **WHEN** an Operator runs `ListRecipes`
- **THEN** the system rejects the query with `E-WEB-004`

### Requirement: Session cookie
The system SHALL set the session cookie `__Host-recipe_session` with the attributes HttpOnly, Secure, SameSite=Strict, and Path=/. The system SHALL NOT set a Domain attribute. The cookie SHALL hold only a random 128-bit token. The server SHALL refuse to start when `session.secure` is false, except in a build with the `dev` feature (R21).

#### Scenario: Cookie after login
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and the right password
- **THEN** the response sets the cookie `__Host-recipe_session`
- **AND** the cookie has the attributes HttpOnly, Secure, SameSite=Strict, and Path=/

#### Scenario: Insecure cookie setting
- **GIVEN** a build without the `dev` feature and the setting `session.secure` set to false
- **WHEN** the server starts
- **THEN** the server refuses to start

### Requirement: Password policy
The system SHALL accept a password of 12 to 128 characters that is not in the breached-password list. The system SHALL reject any other password with `E-IDN-006` (A23).

#### Scenario: Password of 11 characters
- **GIVEN** the Operator `mhoffman`
- **WHEN** an Operator runs `ChangeOwnPassword` with a new password of 11 characters
- **THEN** the system rejects the command with `E-IDN-006`
- **AND** the password hash of `mhoffman` stays unchanged

### Requirement: Wrong current password
The system SHALL check the current password before it changes the password. A wrong current password SHALL return `E-IDN-008` and SHALL insert one login attempt row (A22).

#### Scenario: Wrong current password
- **GIVEN** the Operator `mhoffman`
- **WHEN** an Operator runs `ChangeOwnPassword` with a wrong current password
- **THEN** the system rejects the command with `E-IDN-008`
- **AND** the system inserts one login attempt row for `mhoffman` with `succeeded` set to false

### Requirement: Own password change ends other sessions
On a successful `ChangeOwnPassword`, the system SHALL delete every session of the user and SHALL create one new session for the current request (R21).

#### Scenario: Password changed with two sessions
- **GIVEN** the Operator `mhoffman` with the open sessions `S3` and `S4`
- **WHEN** an Operator runs `ChangeOwnPassword` from the session `S3` with the right current password and a valid new password
- **THEN** the system deletes the sessions `S3` and `S4`
- **AND** the system creates one new session for the current request
- **AND** the audit log holds the action `OwnPasswordChanged`

### Requirement: Password change gate
A user with `must_change_password` set to true SHALL reach only `GET /me/password`, `POST /me/password`, `POST /logout`, and `POST /lang`. Every other route SHALL redirect with `303` to `/me/password` with the code `E-IDN-007` (R21).

#### Scenario: Recipe list before password change
- **GIVEN** the Operator `mhoffman` with `must_change_password` set to true
- **WHEN** an Operator runs `ListRecipes`
- **THEN** the system redirects with `303` to `/me/password`

### Requirement: CSRF token bound to the session
The system SHALL bind the CSRF token to the session. A form SHALL send the token in the field `csrf_token`, and an HTMX request SHALL send it in the header `X-CSRF-Token`. `POST /login` and `POST /lang` without a session SHALL use the pre-session token `csrf_pre`. The export routes SHALL use `GET` and SHALL need no token (R38).

#### Scenario: Login creates a CSRF token
- **GIVEN** the active user `jkowalski`
- **WHEN** a user runs `Login` with the login `jkowalski` and the right password
- **THEN** the system creates a new CSRF token bound to the new session

#### Scenario: Login without the pre-session token
- **GIVEN** a browser without a session
- **WHEN** a user sends `POST /login` without the field `csrf_pre`
- **THEN** the system rejects the request with `E-WEB-007`

### Requirement: CSRF rejection
A mutating request without a valid token SHALL return `E-WEB-007`. The system SHALL write a security event of kind `csrf_rejected` for that request (A33).

#### Scenario: Missing token
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator runs `CreateRecipe` without a CSRF token
- **THEN** the system rejects the command with `E-WEB-007`
- **AND** the system writes a security event of kind `csrf_rejected`

### Requirement: Origin check for mutating requests
The system SHALL reject a mutating request that has no `Origin` header matching the site and no `Sec-Fetch-Site` value of `same-origin` (R38).

#### Scenario: Request from another site
- **GIVEN** the Operator `mhoffman` with an open session
- **WHEN** an Operator runs `CreateRecipe` with an `Origin` header of another site
- **THEN** the system rejects the request

### Requirement: Client address from trusted proxies
The server SHALL read `X-Forwarded-For` and `X-Forwarded-Proto` only from an address in the trusted-proxy list. For every other request, the server SHALL use the socket address as the client address (R38, A22).

#### Scenario: Forwarded header from an untrusted address
- **GIVEN** a request from the address `10.0.0.9`, which is not in the trusted-proxy list
- **WHEN** the request carries the header `X-Forwarded-For: 10.0.0.5`
- **THEN** the server uses `10.0.0.9` as the client address

### Requirement: Request body limits
The server SHALL accept an HTML form body of at most 256 KiB. The four routes that carry a parameter value SHALL accept at most 1 MiB. The route `PUT .../translations/{lang}` SHALL accept at most 64 KiB. A larger request SHALL get `E-WEB-003` (R30, R38).

#### Scenario: Form above the limit
- **GIVEN** the recipe `R-100` and a ProcessEngineer with an open session
- **WHEN** the ProcessEngineer sends a `RenameRecipe` form with a body of 300 KiB
- **THEN** the server rejects the request with `E-WEB-003`
- **AND** the name of `R-100` does not change
