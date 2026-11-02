## MODIFIED Requirements

### Requirement: Export document as the only machine output
The system SHALL provide the JSON export document and the JSON recipe list of the export routes as its only machine-readable outputs. The system SHALL NOT communicate with a PLC (R03, A2).

#### Scenario: Export of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the export document as an HTTP response
- **AND** the system starts no connection to a PLC

#### Scenario: Recipe list of a transfer client
- **GIVEN** the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** the `ExportClient` account `plc-m12` runs `ListExportableRecipes` through `GET /export/v1/machines/M12/recipes`
- **THEN** the system returns the recipe list as an HTTP response
- **AND** the system starts no connection to a PLC

## ADDED Requirements

### Requirement: HTTP Basic on the transfer routes
The three transfer routes SHALL authenticate a request only with the HTTP Basic credentials of an active `ExportClient` account. A request without valid Basic credentials SHALL get `401 E-WEB-012` (R28, A45).

#### Scenario: Valid credentials
- **GIVEN** the active `ExportClient` account `plc-m12` and the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** the client sends `GET /export/v1/machines/M12/recipes/R-100/approved` with the Basic credentials of `plc-m12`
- **THEN** the system returns the export document v1 of the version 3

#### Scenario: Missing header
- **GIVEN** the machine `M12`
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` without an `Authorization` header
- **THEN** the system returns `401 E-WEB-012` as the JSON error body
- **AND** the system runs no password check

#### Scenario: Other scheme
- **GIVEN** the machine `M12`
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` with `Authorization: Bearer abc`
- **THEN** the system returns `401 E-WEB-012`

### Requirement: Basic check of a person account
The Basic check SHALL refuse an unknown login, an inactive account, a wrong password, and an account of the roles Administrator, ProcessEngineer, and Operator. Each refusal SHALL give the same `401 E-IDN-003` answer (A42, A45).

#### Scenario: Operator credentials
- **GIVEN** the active Operator `mhoffman` with a correct password
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` with the Basic credentials of `mhoffman`
- **THEN** the system returns `401 E-IDN-003`
- **AND** the system writes one `security_event` of kind `login_failed`

#### Scenario: Deactivated client account
- **GIVEN** the deactivated `ExportClient` account `plc-m12`
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` with the old Basic credentials of `plc-m12`
- **THEN** the system returns `401 E-IDN-003`
- **AND** the system writes one `security_event` of kind `login_rejected_inactive`

#### Scenario: Each refusal gives one answer
- **GIVEN** an unknown login `ghost`, the Administrator `anowak`, the ProcessEngineer `lnowak`, the Operator `mhoffman`, and `plc-m12` with a wrong password
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` once with each of the five credentials
- **THEN** each answer has the status `401`, the code `E-IDN-003`, and the same body except `request_id`
- **AND** each request runs the password verifier one time with the parameters of `argon2.m_cost`, `t_cost`, and `p_cost`

#### Scenario: Login outside the grammar
- **GIVEN** no account with the login `Plc M12`
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` with the Basic login `Plc M12`
- **THEN** the system runs the dummy hash and returns `401 E-IDN-003`

### Requirement: State check after the password check
After a correct password, the Basic check SHALL lock the user row. It SHALL check again the active state, the role ExportClient, and the password hash. A change SHALL give `401 E-IDN-003` (R28, A45).

#### Scenario: Password reset during the hash check
- **GIVEN** a Basic check of `plc-m12` with the correct password that runs its hash check
- **WHEN** an Administrator runs `ResetPassword` on `plc-m12` and commits before the check locks the user row
- **THEN** the system returns `401 E-IDN-003`
- **AND** the `login_attempt` row of the request holds `succeeded = false`

#### Scenario: Deactivation during the hash check
- **GIVEN** a Basic check of `plc-m12` with the correct password that runs its hash check
- **WHEN** an Administrator runs `DeactivateUser` on `plc-m12` and commits before the check locks the user row
- **THEN** the system returns `401 E-IDN-003`
- **AND** the system writes no `VersionExported` entry

### Requirement: Parser of the Basic header
The parser SHALL give `401 E-WEB-012` without a password check for each of these faults. They are a duplicate `Authorization` header, a header above `export.basic_header_max`, and bad Base64. They are also text that is not UTF-8, text without `:`, and a control character. It SHALL accept the scheme in any case and split at the first `:` (A45).

#### Scenario: Colon in the password
- **GIVEN** the `ExportClient` account `plc-m12` with the password `a:b-c:d-e:f-12`
- **WHEN** a client sends the Basic credentials `plc-m12:a:b-c:d-e:f-12`
- **THEN** the check accepts the login `plc-m12` and the password `a:b-c:d-e:f-12`

#### Scenario: Password of 129 characters
- **GIVEN** the machine `M12`
- **WHEN** a client sends Basic credentials with a password of 129 characters
- **THEN** the system returns `401 E-WEB-012`
- **AND** the system runs no password check

#### Scenario: Scheme in lower case
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a client sends `Authorization: basic` with the correct credentials of `plc-m12`
- **THEN** the system returns the answer of the route

#### Scenario: Two Authorization headers
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a client sends two `Authorization` headers with the correct credentials of `plc-m12`
- **THEN** the system returns `401 E-WEB-012`

### Requirement: One scheme for each export route
The three transfer routes SHALL NOT read the session cookie. The two export routes of a user session SHALL NOT read the `Authorization` header (R28, A45).

#### Scenario: Session cookie on a transfer route
- **GIVEN** a valid session of the Operator `mhoffman`
- **WHEN** the browser sends `GET /export/v1/machines/M12/recipes` with the session cookie and without an `Authorization` header
- **THEN** the system returns `401 E-WEB-012`

#### Scenario: Basic credentials on a session route
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a client sends `GET /export/v1/recipes/R-100/approved` with the Basic credentials of `plc-m12` and without a session cookie
- **THEN** the system returns `303` to `/login`
- **AND** the system writes no `VersionExported` entry

### Requirement: No password dialog of a browser
A `401` answer of a transfer route SHALL carry `WWW-Authenticate: RecipeExport realm="export"`. No answer SHALL carry a challenge of the scheme `Basic` (R39, A45).

#### Scenario: Header of a refused request
- **GIVEN** the machine `M12`
- **WHEN** a client sends `GET /export/v1/machines/M12/recipes` without an `Authorization` header
- **THEN** the answer carries `WWW-Authenticate: RecipeExport realm="export"`
- **AND** the answer carries no challenge of the scheme `Basic`

### Requirement: Login throttle on the Basic check
The Basic check SHALL use the login delay and the per-address budget of `Login`. A throttled check SHALL give `429 E-IDN-005` with `Retry-After` and SHALL run no password check (A22, A33).

#### Scenario: Wrong passwords from one address
- **GIVEN** 3 failed Basic checks of the login `plc-m12` from the address `10.0.0.5` within 15 minutes
- **WHEN** a client at `10.0.0.5` sends one more request with the Basic credentials of `plc-m12` within the delay
- **THEN** the system returns `429 E-IDN-005` with `Retry-After`
- **AND** the system writes one `security_event` of kind `login_throttled`

#### Scenario: Success after a failure
- **GIVEN** 1 failed Basic check of the login `plc-m12` from the address `10.0.0.5`
- **WHEN** a client at `10.0.0.5` sends a request with the correct Basic credentials of `plc-m12`
- **THEN** the system returns the answer of the route
- **AND** the `login_attempt` row of the request holds `succeeded = true`

### Requirement: Bound on parallel Basic checks
At most `export.basic_permits` Basic checks SHALL wait for or hold an Argon2 permit at the same time. A check that gets no permit within `argon2.acquire_timeout` SHALL give `503 E-WEB-009` with `Retry-After` (R30, A45).

#### Scenario: Busy Basic check
- **GIVEN** one Basic check that holds the Basic permit for longer than `argon2.acquire_timeout`
- **WHEN** a second client sends a request with Basic credentials
- **THEN** the system returns `503 E-WEB-009` with `Retry-After`
- **AND** a login through `POST /login` in the same time gets an Argon2 permit

### Requirement: Navigation guard before the Basic check
On a transfer route, the navigation guard SHALL run before the Basic check. A refused request SHALL get `403 E-WEB-007` and SHALL run no password check (R20, A33).

#### Scenario: Cross-site request with credentials
- **GIVEN** the active `ExportClient` account `plc-m12`
- **WHEN** a request with `Sec-Fetch-Site: cross-site` and the Basic credentials of `plc-m12` reaches `GET /export/v1/machines/M12/recipes`
- **THEN** the system returns `403 E-WEB-007`
- **AND** the system writes no `login_attempt` row

### Requirement: Recipe list of a machine
`GET /export/v1/machines/{code}/recipes` SHALL run `ListExportableRecipes` with `VersionExport`. The JSON list SHALL hold each recipe of the machine with an `Approved` version, in the byte order of the reference number (R18, A10).

#### Scenario: Machine with an approved and a draft recipe
- **GIVEN** the machine `M12` with the recipe `R-100` with the approved version 3 and the recipe `R-101` with only a draft
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v1/machines/M12/recipes`
- **THEN** the list holds one row for `R-100` with `version_number` 3, `name`, `approved_at`, and the stored `content_sha256`
- **AND** the list holds no row for `R-101`
- **AND** the audit log holds no new entry

#### Scenario: Archived machine
- **GIVEN** the archived machine `M14` with the recipe `R-140` with the approved version 2, and no live machine `M14`
- **WHEN** the `ExportClient` account `plc-m14` sends `GET /export/v1/machines/M14/recipes`
- **THEN** the list holds one row for `R-140` with `version_number` 2
- **AND** the object `machine` holds `archived` with the value `true`

#### Scenario: Recipe of an archived machine
- **GIVEN** the archived machine `M14` with the recipe `R-140` with the approved version 2, and no live machine `M14`
- **WHEN** the `ExportClient` account `plc-m14` sends `GET /export/v1/machines/M14/recipes/R-140/approved`
- **THEN** the system returns the export document v1 of the version 2
- **AND** the audit log holds one `VersionExported` entry with the actor `plc-m14`

#### Scenario: Unknown machine
- **GIVEN** no live and no archived machine `M99`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v1/machines/M99/recipes`
- **THEN** the system returns `404 E-WEB-005`

### Requirement: Time bound of the recipe list
`ListExportableRecipes` SHALL run inside `timeout.export`. A list that passes the limit SHALL give `503 E-WEB-009` (R30, A19).

#### Scenario: Slow list
- **GIVEN** the machine `M12` and a read of the list that takes longer than `timeout.export`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v1/machines/M12/recipes`
- **THEN** the system returns `503 E-WEB-009` with `Retry-After`

### Requirement: Approved recipe of a machine
`GET /export/v1/machines/{code}/recipes/{ref}/approved` SHALL run `ExportVersion` with `VersionExport` and SHALL return the export document v1. A recipe of another machine SHALL give `404 E-WEB-005` (R18, A10).

#### Scenario: Recipe of another machine
- **GIVEN** the recipe `R-100` on the machine `M12` and the live machine `M13`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v1/machines/M13/recipes/R-100/approved`
- **THEN** the system returns `404 E-WEB-005`
- **AND** the system writes no `VersionExported` entry

#### Scenario: Recipe without an approved version
- **GIVEN** the recipe `R-101` on the machine `M12` with only a draft
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v1/machines/M12/recipes/R-101/approved`
- **THEN** the system returns `404 E-EXP-001`

### Requirement: Unit scope document
`GET /export/v2/machines/{code}/recipes/{ref}/approved/units/{path}` SHALL return the export document v2. It SHALL hold each parameter of the approved version in the unit `{path}` or in a descendant unit. It SHALL hold the types that these parameters use (R07, A10).

#### Scenario: Unit with a descendant unit
- **GIVEN** the approved version 3 of the recipe `R-100` with the parameters `SPEED` in `U1`, `ZONE1` in `U1.2`, and `ZONE2` in `U1.2.4`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1.2`
- **THEN** the document holds `schema_version` `"2"` and `scope` with `unit_path` `U1.2`
- **AND** `parameters[]` holds `ZONE1` and `ZONE2` and no `SPEED`
- **AND** `types[]` holds only the types that `ZONE1` and `ZONE2` use

#### Scenario: Unit path that is a text prefix only
- **GIVEN** the approved version 3 of the recipe `R-100` with the parameter `FEED` in `U12` and the parameter `SPEED` in `U1`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1`
- **THEN** `parameters[]` holds `SPEED` and no `FEED`

### Requirement: Scope hash of a unit document
The v2 document SHALL hold `content_sha256` with the stored hash of the whole version. It SHALL hold `scope_sha256`: the lower-case hex SHA-256 of the RFC 8785 form of the scope hashed body of `api.md` section 6 (R31, A2).

#### Scenario: Consumer checks the scope hash
- **GIVEN** the v2 document of the unit `U1.2` of the version 3 of the recipe `R-100`
- **WHEN** a consumer builds the scope hashed body from the document and computes its SHA-256
- **THEN** the result equals `scope_sha256`
- **AND** `content_sha256` equals the stored hash of the version 3

#### Scenario: Two units of one version
- **GIVEN** the approved version 3 of the recipe `R-100` with parameters in `U1.2` and in `U1.3`
- **WHEN** the client gets the v2 documents of `U1.2` and of `U1.3`
- **THEN** the two documents hold the same `content_sha256`
- **AND** the two documents hold different values of `scope_sha256`

### Requirement: Whole version check before the scope hash
A unit export SHALL first compare the hash of the rebuilt whole version with the stored `content_sha256`. A difference SHALL give `500 E-EXP-002`, and the system SHALL return no document and write no `VersionExported` entry (R31, A2).

#### Scenario: Changed parameter outside the unit
- **GIVEN** the approved version 3 of the recipe `R-100` whose parameter `SPEED` in `U1` no longer matches the stored hash
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1.2`
- **THEN** the system returns `500 E-EXP-002`
- **AND** the system writes no `VersionExported` entry

### Requirement: Empty unit scope
A unit export of a unit without a parameter in the scope SHALL give `404 E-EXP-005`. A valid path that names no unit of the machine SHALL give `404 E-WEB-005`. On a live machine, an archived unit SHALL count as no unit (R31, A6).

#### Scenario: Unit without parameters
- **GIVEN** the live unit `U1.3` of the machine `M12` and the approved version 3 of the recipe `R-100` without a parameter in `U1.3` or below it
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1.3`
- **THEN** the system returns `404 E-EXP-005`
- **AND** the system writes no `VersionExported` entry

#### Scenario: Unit that does not exist
- **GIVEN** no unit `U7` on the machine `M12`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U7`
- **THEN** the system returns `404 E-WEB-005`

#### Scenario: Malformed unit path
- **GIVEN** the machine `M12`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/X1`
- **THEN** the system returns `422 E-PLT-017`

### Requirement: Schema file of the export document v2
The system SHALL keep a JSON Schema 2020-12 file of the export document v2. Every v2 document SHALL pass a check against the file, and the file SHALL reject a field that it does not name (R03, A2).

#### Scenario: Unit document
- **GIVEN** the v2 document of the unit `U1.2` of the version 3 of the recipe `R-100`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

#### Scenario: Unit document against the v1 file
- **GIVEN** the v2 document of the unit `U1.2` of the version 3 of the recipe `R-100`
- **WHEN** a consumer checks the document against the v1 schema file
- **THEN** the check fails

### Requirement: Audit entry of a client export
Each export through a transfer route SHALL write one `VersionExported` entry with the `ExportClient` account as the actor. A unit export SHALL add the unit path and `scope_sha256` to the entry (R20, A8).

#### Scenario: Unit export
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** the `ExportClient` account `plc-m12` gets the v2 document of the unit `U1.2`
- **THEN** the audit log holds one `VersionExported` entry with the actor `plc-m12`
- **AND** the entry holds the schema version `"2"`, `content_sha256`, the unit path `U1.2`, and `scope_sha256`
- **AND** the document holds `exported_by` with the value `plc-m12`

### Requirement: File name of a unit document
The answer of a unit export SHALL set `Content-Disposition: attachment; filename="recipe_{ref}_v{n}_{path}.json"` and `Cache-Control: no-store` (R03, A2).

#### Scenario: Unit export
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** the `ExportClient` account `plc-m12` gets the v2 document of the unit `U1.2`
- **THEN** the answer has the file name `recipe_R-100_v3_U1.2.json`
- **AND** the answer has `Cache-Control: no-store`

### Requirement: Transfer routes in read-only mode
In read-only mode, the three transfer routes SHALL stay available. The Basic check SHALL keep its `login_attempt` rows, and an export SHALL write its `VersionExported` entry (A24).

#### Scenario: Unit export with a full disk
- **GIVEN** the application in read-only mode and the approved version 3 of the recipe `R-100`
- **WHEN** the `ExportClient` account `plc-m12` gets the v2 document of the unit `U1.2`
- **THEN** the system returns the document
- **AND** the audit log holds one new `VersionExported` entry
