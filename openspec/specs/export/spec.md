# export Specification

## Purpose
The export capability describes the JSON export document of an approved or a retired version. It covers the two export routes, the content hash, and the audit entry of each export.

## Requirements

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
The system SHALL set `loadable` to true only for a version that is Approved at export time and has no part that breaks a current rule. A part is a stored value, a limit, or an S7 address. The check SHALL cover every parameter of the version, also for a unit document. A part that fails the earlier rule set of ADR-0120 SHALL give `500 E-DB-008`. The check SHALL not change the hashed body or `content_sha256` (R03, R14, R31, A2).

#### Scenario: Approved version
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `true`

#### Scenario: Retired version
- **GIVEN** the version 2 of the recipe `R-100`, which is retired
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/2`
- **THEN** the document holds `loadable` with the value `false`

#### Scenario: Approved version with a DATE after the range
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in the parameter `DUE`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the document with status 200
- **AND** the document holds `loadable` with the value `false`
- **AND** `content_sha256` equals the stored hash of version 3

#### Scenario: Approved version with too many digits in a limit
- **GIVEN** the approved version 3 of the recipe `R-100` with the stored limit `max` `100.00001` on the `REAL` parameter `RATE`
- **WHEN** a ProcessEngineer runs `ExportVersion` through `GET /export/v1/recipes/R-100/versions/3`
- **THEN** the document holds `loadable` with the value `false`
- **AND** the file name is `recipe_R-100_v3.json`

#### Scenario: Approved version with an address past the block end
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `DINT` at `DB1.DBD65534`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `false`
- **AND** `content_sha256` equals the stored hash of version 3

#### Scenario: Approved version with a data block above 16000 on S7_300
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_300` with the stored parameter `SPEED` of type `INT` at `DB16001.DBW0`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `false`

#### Scenario: Data block above 16000 on S7_1500
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_1500` with the stored parameter `SPEED` of type `INT` at `DB16001.DBW0` and no other fault
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `true`

#### Scenario: Unit document with an affected parameter outside the unit
- **GIVEN** the approved version 3 of the recipe `R-100` with the stored `REAL` value `16777217` in the parameter `SPEED` in `U1`
- **WHEN** the `ExportClient` account `plc-m12` sends `GET /export/v2/machines/M12/recipes/R-100/approved/units/U1.2`
- **THEN** the document holds `loadable` with the value `false`
- **AND** `parameters[]` holds no `SPEED`

#### Scenario: Approved version with a DATE after the earlier range
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_1500` with the stored `DATE` value `72900` in `DUE`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns `500 E-DB-008`

#### Scenario: Changed hash and an affected value
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_1500` with the stored `DATE` value `72700` in `DUE`
- **AND** a rebuilt hash that differs from the stored hash
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns `500 E-EXP-002`

#### Scenario: Approved version with an LDT after the current range on TwinCat3
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `TwinCat3` with the stored `LDT` value `10413792000000000000` in the parameter `STAMP`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the document with status 200
- **AND** the document holds `loadable` with the value `false`
- **AND** `content_sha256` equals the stored hash of version 3

#### Scenario: Data block above 59999 on S7_1500
- **GIVEN** the approved version 3 of the recipe `R-100` on the platform `S7_1500` with the stored parameter `SPEED` of type `INT` at `DB60000.DBW0`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system returns the document with status 200
- **AND** the document holds `loadable` with the value `false`

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
The system SHALL write one audit entry `VersionExported` for each successful export. The entry SHALL hold the version id, the schema version, and `content_sha256` (A4, A8).

#### Scenario: Successful export
- **GIVEN** the approved version 3 of the recipe `R-100`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the audit log holds one entry `VersionExported` for the version
- **AND** the entry holds the id of the version 3, the schema version `"1"`, and the `content_sha256` of the document

### Requirement: Export as a read operation
The system SHALL treat an export as a read operation. An export SHALL NOT change any recipe, version, or parameter row (R03, A4).

#### Scenario: Export keeps the version unchanged
- **GIVEN** the approved version 3 of the recipe `R-100` with the `row_version` 7
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the version 3 keeps its parameters and the `row_version` 7

### Requirement: Serializer error
The system SHALL reject an export with `E-EXP-004` when the serializer cannot create the document. An export that the system rejects with `E-EXP-004` SHALL write no `VersionExported` entry (R03, R20, A2).

#### Scenario: Serializer fails
- **GIVEN** the approved version 3 of the recipe `R-100` and a serializer that fails
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the system rejects the request with `E-EXP-004`
- **AND** the system returns no document
- **AND** the system writes no `VersionExported` entry

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

### Requirement: Schema file of the export document
The system SHALL keep a JSON Schema 2020-12 file of the export document v1 as the source of truth of the document. Every export document SHALL pass a check against the file. Each object whose field names the file lists SHALL reject a field that the file does not name (R03, A2).

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
