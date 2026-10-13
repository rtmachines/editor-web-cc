# Spec Delta: export

## MODIFIED Requirements

### Requirement: Export timeout
The system SHALL reject an export that takes longer than 10 seconds with `E-WEB-009` (R03, A2). The limit SHALL cover the read, the locks, the rebuild of the hashed body, and the hash compare. An export that the system rejects with `E-WEB-009` SHALL write no `VersionExported` entry (R20, ADR-0029).

#### Scenario: Export takes too long
- **GIVEN** the approved version 3 of the recipe `R-100` and an export that takes longer than 10 seconds
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-WEB-009`
- **AND** the system returns no document
- **AND** the system writes no `VersionExported` entry

#### Scenario: Slow commit after the checks
- **GIVEN** the approved version 3 of the recipe `R-100`
- **AND** an export whose read, rebuild, and hash compare end within 10 seconds
- **AND** a commit of the audit entry that ends after the limit
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the document
- **AND** the audit log holds one `VersionExported` entry for the version 3
