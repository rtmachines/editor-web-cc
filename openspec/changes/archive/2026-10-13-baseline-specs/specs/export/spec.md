# export

## Purpose

The export capability describes the JSON export document of an approved or a retired version. It covers the two export routes, the content hash, and the audit entry of each export.

## ADDED Requirements

### Requirement: Export document as the only machine output
The system SHALL provide the JSON export document as its only machine-readable output. The system SHALL NOT communicate with a PLC (R03, A2).

#### Scenario: Export of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100` on the machine `M12`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the export document as an HTTP response
- **AND** the system starts no connection to a PLC

### Requirement: Export of the approved version
The system SHALL export the approved version of a recipe through `GET /export/v1/recipes/{ref}/approved`. The system SHALL reject the request with `E-EXP-001` when the recipe has no approved version (R03, A4).

#### Scenario: Recipe with only a draft
- **GIVEN** the recipe `R-100` with a draft version 4 and no approved version
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-001`

#### Scenario: Deleted recipe
- **GIVEN** the deleted recipe `R-100` with no approved version
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-001`

### Requirement: Export of an approved or a retired version
The system SHALL export an approved or a retired version through `GET /export/v1/recipes/{ref}/versions/{n}`. The system SHALL reject a draft or a discarded version with `E-EXP-003` (R03, A2).

#### Scenario: Retired version
- **GIVEN** the version 2 of the recipe `R-100`, which is retired after version 3 was approved
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the system returns the export document with `version.status` set to `Retired`

#### Scenario: Draft version
- **GIVEN** the draft version 4 of the recipe `R-100`
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/4`
- **THEN** the system rejects the request with `E-EXP-003`

### Requirement: Deleted recipe on the versions route
The system SHALL reject an export of a deleted recipe through `GET /export/v1/recipes/{ref}/versions/{n}` with `E-RCP-012` (R03, A2).

#### Scenario: Version of a deleted recipe
- **GIVEN** the deleted recipe `R-100` with the retired version 2
- **WHEN** an Administrator runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the system rejects the request with `E-RCP-012`

### Requirement: Schema version of the document
The system SHALL write `schema_version` with the value `"1"` in every export document. A breaking change SHALL use the schema version `"2"` and the route `/export/v2/` (R03, A2).

#### Scenario: Schema version of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `schema_version` with the value `"1"`

### Requirement: Loadable flag
The system SHALL set `loadable` to true only when the version is Approved at export time (R03, A2).

#### Scenario: Approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `true`

#### Scenario: Retired version
- **GIVEN** the version 2 of the recipe `R-100`, which is retired
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the document holds `loadable` with the value `false`

### Requirement: Content hash format
The system SHALL write `content_sha256` as 64 lower-case hex characters. The value SHALL be the SHA-256 of the RFC 8785 canonical form of the hashed body (R03, A2).

#### Scenario: Hash of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100` with the stored hash `H1`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `content_sha256` with the value `H1`
- **AND** the value has 64 lower-case hex characters

### Requirement: Contents of the hashed body
The hashed body SHALL hold identifiers and values only. Names, status, approval data, and export data SHALL stay outside the hash (R03, A2).

#### Scenario: Names outside the hash
- **GIVEN** the approved version 3 of the recipe `R-100` with the name `Bottle 0.5 l` on the machine `M12`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the hashed body holds `machine.code` with the value `M12` and `recipe.reference_number` with the value `R-100`
- **AND** the hashed body holds no `recipe.name`

### Requirement: Hash after a rename or a retirement
The system SHALL NOT change `content_sha256` when a user renames the machine or the recipe, or retires the version (R03, A2).

#### Scenario: Rename of the recipe
- **GIVEN** the approved version 3 of the recipe `R-100` with the stored hash `H1`
- **AND** an Administrator renamed the recipe `R-100` to `Bottle 0.5 litre`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `content_sha256` with the value `H1`
- **AND** the document holds `recipe.name` with the value `Bottle 0.5 litre`

#### Scenario: Retirement of the version
- **GIVEN** the version 3 of the recipe `R-100` with the stored hash `H1`, which is retired
- **WHEN** an Administrator runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/3`
- **THEN** the document holds `content_sha256` with the value `H1`

### Requirement: Stored hash check
The system SHALL rebuild the hashed body of each export and compare its hash with the stored `content_sha256`. A difference SHALL reject the export with `E-EXP-002` and return no document (R03, A2).

#### Scenario: Hash mismatch
- **GIVEN** the approved version 3 of the recipe `R-100` whose rebuilt hashed body has a different hash than the stored value
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-002`
- **AND** the system returns no document

### Requirement: Numeric leaves as strings
The system SHALL write each numeric, duration, and instant leaf of a value as a JSON string of canonical decimal text. The system SHALL write each limit the same way (R03, A2).

#### Scenario: Integer parameter
- **GIVEN** the parameter `SPEED` in the unit `U1` with the value 120 and the limit 10 to 250
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value of `SPEED` as the string `"120"`
- **AND** the limit holds `min` as `"10"` and `max` as `"250"`

### Requirement: BOOL leaves as booleans
The system SHALL write each `BOOL` leaf of a value as a JSON boolean (R03, A2).

#### Scenario: BOOL parameter
- **GIVEN** a `BOOL` parameter with the value `true` on the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value of the parameter as the JSON boolean `true`

### Requirement: Time leaves as base-unit counts
The system SHALL write each time leaf as a JSON string that holds its integer count of the base unit of its type (R03, A43).

#### Scenario: TIME parameter
- **GIVEN** a `TIME` parameter with the value `T#1s` on the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value of the parameter as the string `"1000"`, the count of milliseconds

### Requirement: Type references
The system SHALL write a type reference as the primitive name for a primitive without a length. For every other type, the system SHALL write the `id` of the entry in `types[]` (R03, A2).

#### Scenario: Primitive and struct references
- **GIVEN** the parameter `SPEED` of the type `INT` and the struct `ZoneTemp` with the member `label` of a `STRING(16)` type
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the parameter `SPEED` holds `type_id` with the value `INT`
- **AND** the member `label` holds `type_id` with the `id` of the string type entry

### Requirement: Fields absent from a type entry
The system SHALL omit `element_type_id` and `dimensions` from every type entry that is not an array. The system SHALL omit `members` from every type entry that is not a struct (R03, A2).

#### Scenario: Struct entry
- **GIVEN** the struct `ZoneTemp` in the revision 2 with the members `setpoint` and `label`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the entry of `ZoneTemp` holds `members` with two entries
- **AND** the entry of `ZoneTemp` holds no `element_type_id` and no `dimensions`

### Requirement: S7 address of a parameter
The system SHALL write `parameters[].s7_address` with the stored address when one exists. The system SHALL write null for every parameter on `CodesysIec` and `TwinCat3` (R03, A31).

#### Scenario: Parameter on TwinCat3
- **GIVEN** the machine `M12` with the platform `TwinCat3` and the parameter `SPEED` without an address
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the parameter `SPEED` holds `s7_address` with the value null

#### Scenario: Parameter on S7_300
- **GIVEN** the machine `M12` with the platform `S7_300` and the parameter `SPEED` with the address `DB1.DBW0`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the parameter `SPEED` holds `s7_address` with the value `DB1.DBW0`

### Requirement: Stable parameter key
The system SHALL write the parameter key in `parameters[].key` (R03, A40).

#### Scenario: Key across versions
- **GIVEN** the parameter `SPEED` with the key `K1` in the version 2 and in the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the parameter `SPEED` holds `key` with the value `K1`

### Requirement: Parameter ordinal not exported
The system SHALL NOT export `parameter.ordinal`. The field sorts the user interface only (R03, A2).

#### Scenario: Parameter fields of the document
- **GIVEN** the approved version 3 of the recipe `R-100` with the parameter `SPEED`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds no field `ordinal` in any entry of `parameters[]`

### Requirement: Order of parameters
The system SHALL write `parameters[]` ordered by `unit_path` and then by `tag`. The system SHALL compare `unit_path` segment by segment as integers, without the `U` prefix. A path SHALL come before a longer path with the same first segments. The system SHALL compare `tag` byte by byte (R03, A2).

#### Scenario: Unit paths with two digits
- **GIVEN** the parameters `TEMP` on the unit `U1.10` and `SPEED` on the unit `U1.2` of the machine `M12`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the entry of `SPEED` comes before the entry of `TEMP` in `parameters[]`

### Requirement: Order of types, limits, and members
The system SHALL write `types[]` ordered by `id`, `parameters[].limits[]` ordered by `leaf_path`, and `types[].members[]` ordered by member ordinal. The system SHALL compare `id` and `leaf_path` as text, byte by byte (R03, A2).

#### Scenario: Limits of one parameter
- **GIVEN** the parameter `ZONE1` with the limits on the leaf paths `.setpoint` and `.label`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the limit of `.label` comes before the limit of `.setpoint` in `limits[]`

### Requirement: Descriptions and translations not exported
The system SHALL NOT export parameter descriptions or parameter translations (R03, A2).

#### Scenario: Parameter with a translation
- **GIVEN** the parameter `SPEED` with the English name `Line speed` and a Polish translation
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the entry of `SPEED` holds `name_en` with the value `Line speed`
- **AND** the document holds no translation and no description

### Requirement: File name of the export
The system SHALL name the file `recipe_{ref}_v{n}.json` for an approved version. The system SHALL name the file `RETIRED_recipe_{ref}_v{n}.json` for a retired version (R03, A2).

#### Scenario: File name of a retired version
- **GIVEN** the version 2 of the recipe `R-100`, which is retired
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the response holds `Content-Disposition` with the file name `RETIRED_recipe_R-100_v2.json`

### Requirement: No caching of the export
The system SHALL set `Cache-Control` to `no-store` on every export response (R03, A2).

#### Scenario: Response of an approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the response holds `Cache-Control` with the value `no-store`

### Requirement: Audit entry of each export
The system SHALL write one audit entry `VersionExported` for each successful export (A4, A8).

#### Scenario: Successful export
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the audit log holds one entry `VersionExported` for the version

### Requirement: Export as a read operation
The system SHALL treat an export as a read operation. An export SHALL NOT change any recipe, version, or parameter row (R03, A4).

#### Scenario: Export keeps the version unchanged
- **GIVEN** the approved version 3 of the recipe `R-100` with the `row_version` 7
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the version 3 keeps its parameters and the `row_version` 7

### Requirement: Serializer error
The system SHALL reject an export with `E-EXP-004` when the serializer cannot create the document (R03, A2).

#### Scenario: Serializer fails
- **GIVEN** the approved version 3 of the recipe `R-100` and a serializer that fails
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-004`
- **AND** the system returns no document

### Requirement: Export timeout
The system SHALL reject an export that takes longer than 10 seconds with `E-WEB-009` (R03, A2).

#### Scenario: Export takes too long
- **GIVEN** the approved version 3 of the recipe `R-100` and an export that takes longer than 10 seconds
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-WEB-009`
- **AND** the system returns no document
