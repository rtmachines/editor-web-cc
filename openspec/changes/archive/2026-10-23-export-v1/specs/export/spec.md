## ADDED Requirements

### Requirement: Schema file of the export document
The system SHALL keep a JSON Schema 2020-12 file of the export document v1 as the source of truth of the document. Every export document SHALL pass a check against the file. Each object with fixed fields SHALL reject a field that the file does not name (R03, A2).

#### Scenario: Document of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document passes a check against the schema file that includes the formats

#### Scenario: Extra field
- **GIVEN** the document of the version 3 of the recipe `R-100` with an extra field `signature` at the top level
- **WHEN** a consumer checks the document against the schema file
- **THEN** the check fails

### Requirement: Field order of the export document
The system SHALL write the fields of the document and of each nested object in the order of `api.md` section 6.1. The system SHALL write no other field (R03, A2).

#### Scenario: Order of the fields
- **GIVEN** the approved version 3 of the recipe `R-100` with the parameter `SPEED`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the top level holds `schema_version`, `exported_at`, `exported_by`, `loadable`, `content_sha256`, `machine`, `recipe`, `version`, `types`, and `parameters` in this order
- **AND** the entry of `SPEED` holds `unit_path`, `tag`, `key`, `name_en`, `type_id`, `s7_address`, `limits`, and `value` in this order

### Requirement: Member order of a struct value
The system SHALL write the members of a struct value in the order of `members[]` of the type entry of the struct. This order SHALL NOT change `content_sha256` (R03, A2).

#### Scenario: Struct value of a parameter
- **GIVEN** the parameter `ZONE1` of the struct `ZoneTemp` in the revision 2 with the members `setpoint` and `label` in this order
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the value of `ZONE1` holds `setpoint` before `label`
- **AND** the document holds `content_sha256` with the stored hash of the version

### Requirement: Users and times of the export document
The system SHALL write `exported_by` and `version.approved_by` as the `Login` of the user. The system SHALL write `exported_at` and `version.approved_at` in UTC as RFC 3339 text with whole seconds and the suffix `Z` (R03, A8).

#### Scenario: Approver and exporter
- **GIVEN** the version 3 of the recipe `R-100`, which `anowak` approved at 14:02:10.5 UTC on 2026-10-11
- **WHEN** the Operator `mhoffman` runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `version.approved_by` with the value `anowak` and `version.approved_at` with the value `2026-10-11T14:02:10Z`
- **AND** the document holds `exported_by` with the value `mhoffman`

### Requirement: Only GET on the export routes
The export routes SHALL accept only `GET`. A `HEAD` request on an export route SHALL NOT run `ExportVersion` and SHALL NOT write a `VersionExported` entry (R20, A8).

#### Scenario: HEAD request
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator sends `HEAD /export/v1/recipes/R-100/approved`
- **THEN** the server returns no export document
- **AND** the audit log holds no new `VersionExported` entry

### Requirement: Export in read-only mode
In read-only mode, the system SHALL export an approved or a retired version and SHALL write its `VersionExported` entry (A24).

#### Scenario: Export with a full disk
- **GIVEN** the application in read-only mode and the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the export document
- **AND** the audit log holds one new `VersionExported` entry for the version 3

### Requirement: Failed task of an export
An export whose rebuild task fails SHALL return `E-WEB-011` and SHALL write no `VersionExported` entry (R20, R35).

#### Scenario: Task fails
- **GIVEN** the approved version 3 of the recipe `R-100` and a rebuild task that fails
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-WEB-011`
- **AND** the system writes no `VersionExported` entry

### Requirement: Permits of an export
At most `export.permits` exports SHALL run at the same time. `ExportVersion` SHALL take the permit before its transaction, and its blocking task SHALL hold the permit to its end. An export that does not get a permit within `timeout.export` SHALL return `E-WEB-009` and SHALL write no `VersionExported` entry (R30, A26, ADR-0056).

#### Scenario: Busy exports
- **GIVEN** `export.permits` exports that hold a permit
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved` and waits longer than `timeout.export` for a permit
- **THEN** the system returns `E-WEB-009`
- **AND** the audit log holds no new `VersionExported` entry

#### Scenario: Permit after the end of an export
- **GIVEN** `export.permits` exports that hold a permit and one more export that waits for a permit
- **WHEN** one of the running exports ends within `timeout.export` of the waiting export
- **THEN** the waiting export gets the permit and returns the export document

## MODIFIED Requirements

### Requirement: Audit entry of each export
The system SHALL write one audit entry `VersionExported` for each successful export. The entry SHALL hold the version id, the schema version, and `content_sha256` (A4, A8).

#### Scenario: Successful export
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the audit log holds one entry `VersionExported` for the version
- **AND** the entry holds the id of the version 3, the schema version `"1"`, and the `content_sha256` of the document

### Requirement: Serializer error
The system SHALL reject an export with `E-EXP-004` when the serializer cannot create the document. An export that the system rejects with `E-EXP-004` SHALL write no `VersionExported` entry (R03, R20, A2).

#### Scenario: Serializer fails
- **GIVEN** the approved version 3 of the recipe `R-100` and a serializer that fails
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-004`
- **AND** the system returns no document
- **AND** the system writes no `VersionExported` entry
