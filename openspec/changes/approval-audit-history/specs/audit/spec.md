## ADDED Requirements

### Requirement: Audit list query
`ListAuditEntries` SHALL check its permission first. It SHALL return at most `page.audit` entries, newest first, with the id below the cursor `before`. It SHALL say whether an older entry exists. It SHALL read no state of an entry (R20, R28, ADR-0051).

#### Scenario: Cursor of the second page
- **GIVEN** 150 audit entries with the ids 1 to 150
- **WHEN** an Administrator runs `ListAuditEntries` with `before` 51
- **THEN** the result holds the entries 50 to 1, newest first
- **AND** the result says that no older entry exists

### Requirement: Scope of the audit queries
For a user with only `AuditViewRecipes`, `ListAuditEntries` SHALL return only the entries with `aggregate_type` set to `recipe`. A filter value of `entity_type` or `action` outside that scope SHALL give an empty list. `GetAuditEntry` of an entry outside that scope SHALL return `NotFound`. The audit list and the entry SHALL include the entries of a deleted recipe (A20, R28, ADR-0051).

#### Scenario: ProcessEngineer filters by the action of a machine
- **GIVEN** the entries `MachineRenamed` of `M12` and `RecipeRenamed` of `R-100`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with the action `MachineRenamed`
- **THEN** the result is empty

#### Scenario: ProcessEngineer reads an entry of a machine
- **GIVEN** the entry `MachineRenamed` of `M12`
- **WHEN** a ProcessEngineer runs `GetAuditEntry` on that entry
- **THEN** the system returns `NotFound`

#### Scenario: ProcessEngineer lists the entries of a deleted recipe
- **GIVEN** the deleted recipe `R-300` with the entry `RecipeDeleted`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with the action `RecipeDeleted`
- **THEN** the result holds the entry of `R-300`

### Requirement: Filters of the audit list
`ListAuditEntries` SHALL filter by `aggregate` (an aggregate id), `entity_type`, `entity_id` (only with `entity_type`), `actor` (a login), `action`, `from`, and `to`. `from` and `to` SHALL be dates in UTC, and both days SHALL be inside the range. An `actor` that names no user SHALL give the empty list, so the answer does not show which logins exist (R20, R38, A42, ADR-0051).

#### Scenario: Range of one day
- **GIVEN** the entries of 2026-10-15 23:59 UTC and of 2026-10-16 00:00 UTC
- **WHEN** an Administrator runs `ListAuditEntries` with `from` and `to` set to 2026-10-16
- **THEN** the result holds only the entry of 2026-10-16 00:00 UTC

#### Scenario: Entries of one parameter
- **GIVEN** the entries `ParameterValueSet` and `LimitsReplaced` of the parameter `SPEED_TABLE` and an entry of `HOLD`
- **WHEN** an Administrator runs `ListAuditEntries` with `entity_type` `parameter` and the id of `SPEED_TABLE`
- **THEN** the result holds the two entries of `SPEED_TABLE`

#### Scenario: Actor that names no user
- **GIVEN** no user with the login `ghost`
- **WHEN** a ProcessEngineer runs `ListAuditEntries` with `actor` set to `ghost`
- **THEN** the result is empty

### Requirement: Audit entry with its states
`GetAuditEntry` SHALL return the entry with `before` and `after`. The states SHALL hold no password hash, no session token, and no cookie value (R20, `reliability.md` 1.1, ADR-0051).

#### Scenario: Entry of a new user
- **GIVEN** the entry `UserCreated` of the user `jsmith`
- **WHEN** an Administrator runs `GetAuditEntry` on that entry
- **THEN** the state `after` holds the login `jsmith`
- **AND** the state holds no field `password_hash`

### Requirement: Recipe history query
`GetRecipeHistory` SHALL check `RecipeView` first. It SHALL return the entries with the `aggregate_id` of the recipe, newest first, at most `page.history` entries, without `before` and `after`. For a deleted recipe, it SHALL return the entries only to a user with `RecipeRestore` and `E-RCP-012` to every other user (R24, A14, A20, ADR-0051).

#### Scenario: History of a recipe
- **GIVEN** the recipe `R-100` with the entries `RecipeCreated`, `VersionCreated`, and `VersionApproved`
- **WHEN** an Operator runs `GetRecipeHistory` on `R-100`
- **THEN** the result holds the three entries, newest first, without states

### Requirement: Security event list query
`ListSecurityEvents` SHALL check `AuditView` first. It SHALL return at most `page.security_events` events, newest first, with the cursor `before`. It SHALL filter by `kind`, `login`, `client_ip`, `from`, and `to`. `login` SHALL match the events of the user with that login. It SHALL also match the events whose `login_hmac` equals the HMAC of the login under the current key of the key ring. The result SHALL hold no `login_hmac` (R35, A33, ADR-0051).

#### Scenario: Events of an unknown login
- **GIVEN** two `login_failed` events of the typed login `ghost`, which matches no account
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `ghost`
- **THEN** the result holds the two events
- **AND** no field of the result holds the `login_hmac`

#### Scenario: Event under an older key
- **GIVEN** a `login_failed` event of the typed login `ghost` with the HMAC under an older key of the key ring
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `ghost`
- **THEN** the result is empty

#### Scenario: Events of a known user
- **GIVEN** a `forbidden` event of the user `mhoffman`
- **WHEN** an Administrator runs `ListSecurityEvents` with `login` set to `mhoffman`
- **THEN** the result holds the event
