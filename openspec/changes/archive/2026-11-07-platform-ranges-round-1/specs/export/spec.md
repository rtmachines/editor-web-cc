## MODIFIED Requirements

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
