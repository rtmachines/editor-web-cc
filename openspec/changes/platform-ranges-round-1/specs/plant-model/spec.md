## MODIFIED Requirements

### Requirement: String length per platform
The length of a `STRING(n)` or a `WSTRING(n)` SHALL be inside the range of the `StringLength` table of `domain-model.md` section 2.1 for the platform of the machine. On `CodesysIec` and `TwinCat3`, the range SHALL stay 1 to 255, also when the controller accepts a longer string. A `STRING` or a `WSTRING` without a length SHALL mean the seed type of that platform (R11, R12, R13).
#### Scenario: STRING length above the S7 range
- **GIVEN** the machine `M12` with the platform `S7_1500`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(255)`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** the system creates no type

#### Scenario: STRING length inside the TwinCat3 range
- **GIVEN** the machine `M7` with the platform `TwinCat3` and no type `STRING(255)`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(255)`
- **THEN** the system creates the type `STRING(255)` for `M7`

#### Scenario: WSTRING length above the TwinCat3 range
- **GIVEN** the machine `M7` with the platform `TwinCat3`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(256)`
- **THEN** the system rejects the command with `E-PLT-013`

#### Scenario: STRING without a length
- **GIVEN** the machine `M3` with the platform `S7_300`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING`
- **THEN** the system uses the seed type `STRING(254)` of `S7_300`
- **AND** the system writes no action `DerivedTypeCreated`

#### Scenario: STRING length above the CodesysIec range
- **GIVEN** the machine `M12` with the platform `CodesysIec`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `STRING(256)`
- **THEN** the system rejects the command with `E-PLT-013`
- **AND** the system creates no type

#### Scenario: WSTRING length inside the CodesysIec range
- **GIVEN** the machine `M12` with the platform `CodesysIec` and no type `WSTRING(255)`
- **WHEN** a ProcessEngineer runs `AddParameter` with the type `WSTRING(255)`
- **THEN** the system creates the type `WSTRING(255)` for `M12`
