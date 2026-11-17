## MODIFIED Requirements

### Requirement: Second pick of the approved version
The system SHALL pick the approved version again when the export locks a version that is no longer approved. It SHALL export that version when its lock finds it approved. When the second pick or its lock finds no approved version, the system SHALL reject the request with `404 E-EXP-001`. It SHALL NOT pick a third time (R18, R31, A4). On a route with `{code}`, the second pick SHALL apply the machine filter of the first pick. When the live machine of that code does not hold the recipe, the system SHALL reject the request with `404 E-WEB-005` (R18, A10).

#### Scenario: Approval between the pick and the lock
- **GIVEN** the recipe `R-100` with the approved version 3 and the draft version 4
- **AND** an Operator export through `GET /export/v1/recipes/R-100/approved` that picked version 3
- **WHEN** an Administrator runs `ApproveVersion` on version 4 before the export locks version 3
- **THEN** the system exports version 4 with the status `Approved`
- **AND** the system writes one entry `VersionExported` for version 4

#### Scenario: No approved version at the second pick
- **GIVEN** the recipe `R-100` with the approved version 3 and no other version
- **AND** an Operator export through `GET /export/v1/recipes/R-100/approved` that picked version 3
- **WHEN** an Administrator runs `RetireVersion` on version 3 before the export locks it
- **THEN** the system rejects the request with `404 E-EXP-001`
- **AND** the system writes no entry `VersionExported`

#### Scenario: Second loss of the approved state
- **GIVEN** an Operator export through `GET /export/v1/recipes/R-100/approved` that found version 3 retired under its lock
- **AND** the export picked the approved version 4, and `R-100` has the draft version 5
- **WHEN** an Administrator runs `ApproveVersion` on version 5 before the export locks version 4
- **THEN** the system rejects the request with `404 E-EXP-001`
- **AND** the system writes no entry `VersionExported`

#### Scenario: Machine code reused before the second pick
- **GIVEN** the recipe `R-100` on the machine `M12` with the approved version 3 and the draft version 4
- **AND** the `ExportClient` account `plc-m12` sent `GET /export/v1/machines/M12/recipes/R-100/approved`, and the export picked version 3
- **WHEN** before the export locks version 3, an Administrator approves version 4, archives `M12`, and creates a new machine with the code `M12`
- **THEN** the system rejects the request with `404 E-WEB-005`
- **AND** the system writes no entry `VersionExported`
