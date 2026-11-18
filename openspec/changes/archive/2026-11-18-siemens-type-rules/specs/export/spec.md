## ADDED Requirements

### Requirement: Pattern of the S7 address in the schema v2
The v2 schema file SHALL accept a string `parameters[].s7_address` only in the grammar of `S7Address`. The data block number SHALL be `1..=65535` without a leading zero. The offset SHALL be `0..=65535` with at most five digits. The bit SHALL be `0..=7` and SHALL come only after `DBX`. The v1 schema file SHALL not change (R03, R38, A31, ADR-0189).

#### Scenario: Address of each width
- **GIVEN** a v2 document with the addresses `DB1.DBX0.7`, `DB2.DBB10`, `DB65535.DBW65534`, and `DB12.DBD4`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

#### Scenario: Malformed address
- **GIVEN** a v2 document with one of the addresses `DB1.DBX0.8`, `DB0.DBW0`, `DB65536.DBB0`, `DB1.DBW65536`, `DB1.DBB0.1`, `DB1.DBX0`, or `db1.dbw0`
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check fails

#### Scenario: Malformed address against the v1 file
- **GIVEN** a v1 document with the address `DB1.DBX0.8`
- **WHEN** a consumer checks the document against the v1 schema file
- **THEN** the check passes

#### Scenario: Parameter without an address
- **GIVEN** a v2 document of a version on `S7_1500` with a parameter whose `s7_address` is null
- **WHEN** a consumer checks the document against the v2 schema file
- **THEN** the check passes

### Requirement: Loadable flag and the standard-access rule
The export check SHALL run the rules of the array bounds and of the `BOOL` array with three dimensions on every parameter with an address. A break SHALL set `loadable` to false. The version page check of an `Approved` version SHALL give the same result (R03, R31, A1, ADR-0178, ADR-0187).

#### Scenario: Approved version with an array bound above 32767
- **GIVEN** the approved version 3 of `R-100` on `M3` with the platform `S7_300`
- **AND** the parameter `BUF` of type `ARRAY[32760..32770] OF BYTE` at `DB1.DBB0`, stored before ADR-0178
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `false`

#### Scenario: Approved version with a BOOL array with three dimensions
- **GIVEN** the approved version 3 of `R-100` on `M3` with the parameter `CUBE` of type `ARRAY[1..2,1..2,1..2] OF BOOL` at `DB1.DBB0`, stored before ADR-0187
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `false`

#### Scenario: Approved version with a WCHAR above U+D7FF on S7
- **GIVEN** the approved version 3 of `R-100` on `M5` with the platform `S7_1500` and the parameter `MARK` of type `WCHAR` with the value `"\uE000"`, stored before ADR-0180
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `loadable` with the value `false`

### Requirement: Export of a version stored before the Siemens rules
An `Approved` version that the earlier release stored SHALL export with its stored `content_sha256`. The type rules of ADR-0177, ADR-0179, and ADR-0186 SHALL keep `loadable` true. The rules of ADR-0178, ADR-0180, and ADR-0187 SHALL set `loadable` to false. A restore of the database SHALL give the same document (R03, R19, R31, A1, ADR-0119, ADR-0120).

#### Scenario: Stored type over the nesting limit
- **GIVEN** the approved version 3 of `R-100` on `M5` with the platform `S7_1500`, approved before ADR-0179 with the stored hash `H`
- **AND** the parameter `TOP` of type `ST_Top@1` with nine levels in the S7 count
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `content_sha256` with the value `H`
- **AND** the document holds `loadable` with the value `true`

#### Scenario: Stored WSTRING character from U+D000
- **GIVEN** the approved version 3 of `R-100` on `M6` with the platform `S7_1200`, approved before ADR-0180 with the stored hash `H`
- **AND** the parameter `NOTE` of type `WSTRING(10)` with the value `"A\uD000"`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `content_sha256` with the value `H`
- **AND** the document holds `loadable` with the value `false`

#### Scenario: Export after a restore
- **GIVEN** a backup of the database with the approved version 3 of `R-100`, approved before ADR-0178 with the stored hash `H`
- **WHEN** the restore procedure of `docs/ops/runbook.md` loads the backup into an empty database of this release
- **AND** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds `content_sha256` with the value `H`

### Requirement: Latin-1 bytes of STRING and CHAR values
The export documentation SHALL state the bytes of a `STRING` or `CHAR` value. A transfer client writes each character as one ISO 8859-1 byte with the scalar value of the character. The export document SHALL keep the JSON text of the value (R03, R11, A2, ADR-0181).

#### Scenario: STRING value with a Latin-1 letter
- **GIVEN** the approved version 3 of `R-100` with the parameter `LABEL` of type `STRING(4)` and the value `"é"`
- **WHEN** an Operator runs `ExportVersion` through `GET /export/v1/recipes/R-100/approved`
- **THEN** the document holds the value `"é"` for `LABEL`
- **AND** the export documentation gives the byte `16#E9` for `é`
